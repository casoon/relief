//! Änderungssignal für den CDP-Host (Backlog 08, Punkte 1 und 2).
//!
//! Chrome sendet `Accessibility.nodesUpdated` nicht (geprüft 2026-09-24 mit
//! rohem CDP, auch nach `getFullAXTree`/`getPartialAXTree`; nur
//! `loadComplete` kommt). Inkrementelle AX-Deltas gibt es über CDP also nicht.
//! Ersatz: DOM-Mutationsereignisse sagen, *dass* sich etwas geändert hat.
//!
//! - „Seite ruht“: [`QUIET`] ohne DOM-Mutation, [`NET_QUIET`] ohne
//!   Netzwerkereignis und keine Anfrage ausstehend; höchstens
//!   [`MAX_SETTLE`]. Nachladende Seiten (bahn.de) mutieren erst, wenn die
//!   Antwort da ist, und stoßen Anfragen per Zeitgeber an — DOM-Ruhe allein
//!   endet vorher.
//! - Ausstehend zählen nur Anfragen, die ein Ende haben: WebSockets
//!   (eigene Ereignisse, kein `requestWillBeSent`), `EventSource`, Beacons
//!   (`Ping`) und Medienströme nicht; Anfragen, die länger als
//!   [`LONG_REQUEST`] offen sind (Long-Polling, hängende Tracker), auch nicht.
//! - Neu aufnehmen nur, wenn seit der letzten Aufnahme etwas mutiert ist.
//!
//! Ersetzt die Seite ihr Dokument (`DOM.documentUpdated`, z. B. amazon.de
//! direkt nach dem Laden), meldet der DOM-Agent danach nichts mehr, bis das
//! Dokument erneut angefordert wird — [`Live::take_dirty`] tut das.
//!
//! Nicht erfasst: Änderungen ohne DOM-Mutation (per Skript gesetzte
//! `value`-Eigenschaft, Fokuswechsel). Nach eigenen Aktionen nimmt der Host
//! deshalb immer neu auf.

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use anyhow::Result;
use chromiumoxide::cdp::browser_protocol::dom::{
    EventAttributeModified, EventAttributeRemoved, EventCharacterDataModified,
    EventChildNodeInserted, EventChildNodeRemoved, EventDocumentUpdated, GetDocumentParams,
};
use chromiumoxide::cdp::browser_protocol::network::{
    EventLoadingFailed, EventLoadingFinished, EventRequestWillBeSent, RequestId, ResourceType,
};
use chromiumoxide::Page;
use futures::{Stream, StreamExt};
use tokio::sync::mpsc;

/// So lange ohne DOM-Mutation gilt die Seite als ruhig.
pub const QUIET: Duration = Duration::from_millis(150);
/// So lange ohne Netzwerkereignis (Anfrage begonnen oder beendet), falls
/// eines kam; wie `networkIdle` in Chrome. Seiten laden per Zeitgeber nach,
/// ohne dass vorher DOM oder Netz etwas melden (bahn.de: 330–490 ms Pause;
/// mit 300 ms instabil, mit 500 ms stabil).
pub const NET_QUIET: Duration = Duration::from_millis(500);
/// Obergrenze fürs Warten auf Ruhe (Animationen, Ticker, Karussells).
pub const MAX_SETTLE: Duration = Duration::from_millis(3000);
/// Länger offene Anfragen gelten als Dauerverbindung und halten die Ruhe
/// nicht auf.
pub const LONG_REQUEST: Duration = Duration::from_millis(1000);

pub struct Live {
    page: Page,
    rx: mpsc::UnboundedReceiver<Signal>,
    tasks: Vec<tokio::task::JoinHandle<()>>,
    /// Seit der letzten Aufnahme ist etwas mutiert.
    dirty: bool,
    document_replaced: bool,
    /// Seit der letzten Abfrage hat die Seite ihr Dokument ersetzt: Für das
    /// Modell ist das ein neuer Baum (neue Tree-ID).
    new_document: bool,
    /// Ausstehende Anfragen mit Startzeit.
    pending: HashMap<RequestId, Instant>,
    /// Beendete Anfragen, deren Start noch nicht eingetroffen ist: Jede
    /// Ereignisart kommt über ein eigenes Abo, die Reihenfolge zwischen
    /// ihnen ist nicht garantiert.
    finished: HashSet<RequestId>,
    /// Letztes Netzwerkereignis.
    last_network: Option<Instant>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Signal {
    Mutation,
    Document,
    /// Netzwerksignale mit Eingangszeit, weil sie erst beim nächsten Warten
    /// verarbeitet werden.
    RequestStarted(RequestId, Instant),
    RequestDone(RequestId, Instant),
}

#[derive(Debug, Default, Clone)]
pub struct Settle {
    pub mutations: usize,
    /// Abgeschlossene Anfragen während des Wartens.
    pub requests: usize,
    pub waited_ms: u128,
    pub hit_max: bool,
}

impl Live {
    pub async fn start(page: &Page) -> Result<Self> {
        let (tx, rx) = mpsc::unbounded_channel();
        // Den Network-Agenten schaltet chromiumoxide beim Anlegen der Seite ein.
        let tasks = vec![
            forward(
                page.event_listener::<EventChildNodeInserted>().await?,
                tx.clone(),
                |_| Some(Signal::Mutation),
            ),
            forward(
                page.event_listener::<EventChildNodeRemoved>().await?,
                tx.clone(),
                |_| Some(Signal::Mutation),
            ),
            forward(
                page.event_listener::<EventAttributeModified>().await?,
                tx.clone(),
                |_| Some(Signal::Mutation),
            ),
            forward(
                page.event_listener::<EventAttributeRemoved>().await?,
                tx.clone(),
                |_| Some(Signal::Mutation),
            ),
            forward(
                page.event_listener::<EventCharacterDataModified>().await?,
                tx.clone(),
                |_| Some(Signal::Mutation),
            ),
            forward(
                page.event_listener::<EventDocumentUpdated>().await?,
                tx.clone(),
                |_| Some(Signal::Document),
            ),
            forward(
                page.event_listener::<EventRequestWillBeSent>().await?,
                tx.clone(),
                |e| {
                    let open_ended = matches!(
                        e.r#type,
                        Some(
                            ResourceType::EventSource
                                | ResourceType::WebSocket
                                | ResourceType::Ping
                                | ResourceType::Media
                        )
                    );
                    (!open_ended)
                        .then(|| Signal::RequestStarted(e.request_id.clone(), Instant::now()))
                },
            ),
            forward(
                page.event_listener::<EventLoadingFinished>().await?,
                tx.clone(),
                |e| Some(Signal::RequestDone(e.request_id.clone(), Instant::now())),
            ),
            forward(
                page.event_listener::<EventLoadingFailed>().await?,
                tx,
                |e| Some(Signal::RequestDone(e.request_id.clone(), Instant::now())),
            ),
        ];
        request_document(page).await?;
        Ok(Live {
            page: page.clone(),
            rx,
            tasks,
            dirty: false,
            document_replaced: false,
            new_document: false,
            pending: HashMap::new(),
            finished: HashSet::new(),
            last_network: None,
        })
    }

    /// Den DOM-Agenten an das aktuelle Dokument hängen (nach dem Laden bzw.
    /// einer Navigation). Die Ereignis-Abos gelten für die Seite und bleiben,
    /// ebenso die ausstehenden Anfragen: Wer vor der Navigation startet,
    /// zählt die Anfragen des Ladens mit.
    pub async fn attach(&mut self) -> Result<()> {
        request_document(&self.page).await
    }

    /// Wartet auf „Seite ruht“.
    pub async fn settle(&mut self) -> Settle {
        let started = Instant::now();
        let mut s = Settle::default();
        loop {
            let left = MAX_SETTLE.saturating_sub(started.elapsed());
            if left.is_zero() {
                s.hit_max = true;
                break;
            }
            let wait = QUIET.max(self.network_rest(started)).min(left);
            match tokio::time::timeout(wait, self.rx.recv()).await {
                Ok(Some(signal)) => {
                    match signal {
                        Signal::Mutation | Signal::Document => s.mutations += 1,
                        Signal::RequestDone(..) => s.requests += 1,
                        Signal::RequestStarted(..) => {}
                    }
                    self.note(signal);
                }
                Ok(None) => break,
                Err(_) if !self.loading() && self.network_rest(started).is_zero() => break,
                Err(_) => {}
            }
        }
        self.dirty |= s.mutations > 0;
        s.waited_ms = started.elapsed().as_millis();
        s
    }

    /// Wie lange noch bis zur Netzwerkruhe. Zählt nur Netzwerkereignisse
    /// seit Beginn des Wartens: Was vorher endete (das Dokument selbst beim
    /// Laden), hat seine DOM-Wirkung schon angestoßen; ausstehende Anfragen
    /// zählen unabhängig davon.
    fn network_rest(&self, since: Instant) -> Duration {
        self.last_network
            .filter(|t| *t >= since)
            .map(|t| NET_QUIET.saturating_sub(t.elapsed()))
            .unwrap_or_default()
    }

    /// Steht eine Anfrage aus, die kürzer als [`LONG_REQUEST`] läuft?
    fn loading(&self) -> bool {
        self.pending.values().any(|t| t.elapsed() < LONG_REQUEST)
    }

    fn note(&mut self, signal: Signal) {
        match signal {
            Signal::Mutation => self.dirty = true,
            Signal::Document => {
                self.dirty = true;
                self.document_replaced = true;
                self.new_document = true;
            }
            Signal::RequestStarted(id, at) => {
                self.last_network = Some(at);
                if !self.finished.remove(&id) {
                    self.pending.insert(id, at);
                }
            }
            Signal::RequestDone(id, at) => {
                self.last_network = Some(at);
                if self.pending.remove(&id).is_none() {
                    self.finished.insert(id);
                }
            }
        }
    }

    /// Hat die Seite seit der letzten Abfrage ihr Dokument ersetzt
    /// (`DOM.documentUpdated`)? Nach [`Live::take_dirty`] aufrufen, das die
    /// wartenden Signale einsammelt.
    pub fn take_new_document(&mut self) -> bool {
        std::mem::take(&mut self.new_document)
    }

    /// Hat sich seit der letzten Aufnahme etwas geändert? Ein ersetztes
    /// Dokument wird dabei neu angefordert, damit wieder Mutationen kommen.
    pub async fn take_dirty(&mut self) -> Result<bool> {
        while let Ok(signal) = self.rx.try_recv() {
            self.note(signal);
        }
        if std::mem::take(&mut self.document_replaced) {
            request_document(&self.page).await?;
        }
        Ok(std::mem::take(&mut self.dirty))
    }
}

/// Der DOM-Agent meldet Mutationen nur für Knoten, die er schon übertragen
/// hat — deshalb das ganze Dokument anfordern.
async fn request_document(page: &Page) -> Result<()> {
    page.execute(GetDocumentParams::builder().depth(-1).pierce(true).build())
        .await?;
    Ok(())
}

fn forward<T: Send + 'static>(
    mut events: impl Stream<Item = T> + Unpin + Send + 'static,
    tx: mpsc::UnboundedSender<Signal>,
    signal: impl Fn(T) -> Option<Signal> + Send + 'static,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        while let Some(event) = events.next().await {
            let Some(signal) = signal(event) else {
                continue;
            };
            if tx.send(signal).is_err() {
                break;
            }
        }
    })
}

impl Drop for Live {
    fn drop(&mut self) {
        self.tasks.iter().for_each(|t| t.abort());
    }
}
