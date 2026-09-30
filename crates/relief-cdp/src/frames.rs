//! Frames in einem anderen Renderer-Prozess (Site Isolation, Paket 70).
//!
//! Chrome gibt einem iframe fremder Site einen eigenen Prozess. Die Sitzung
//! der Seite erreicht ihn nicht: `getFullAXTree { frameId }` antwortet nicht,
//! `DOM.getDocument` (`pierce`) liefert am `iframe` nur die `frameId`, kein
//! Dokument. Ein solcher Frame ist ein eigenes CDP-Ziel mit derselben ID wie
//! der Frame; der Host hängt sich über eine zweite Verbindung zum Browser
//! flach an (`Target.setAutoAttach`, `flatten`, siehe unten) und schickt Befehle mit der
//! Sitzungs-ID des Ziels. chromiumoxide kann Befehle nur an die Sitzung der
//! Seite schicken, deshalb die eigene Verbindung (`chromiumoxide::Connection`,
//! keine neue Abhängigkeit).
//!
//! **Backend-IDs** vergibt jeder Renderer-Prozess selbst, sie können sich
//! zwischen Seite und Frame überschneiden. Im Modell steht deshalb
//! [`encode`]: die Nummer des Frames (ab 1, je Seite in der Reihenfolge des
//! ersten Anhängens) in den oberen 32 Bit, die Backend-ID des Frames in den
//! unteren; `0` oben ist die Seite selbst, deren IDs bleiben unverändert.
//! [`Frames::resolve`] macht daraus wieder Sitzung und Backend-ID.
//!
//! **Änderungssignal (Paket 85):** Beim Anhängen schaltet der Host in der
//! Sitzung des Frames den Network-Agenten ein und fordert das Dokument an
//! (der DOM-Agent meldet Mutationen nur für übertragene Knoten). Die
//! Ereignisse aller Frame-Sitzungen gehen an `live.rs`; ersetzt ein Frame
//! sein Dokument, fordert die Verbindung es selbst neu an.
//!
//! **Anhängen beim Entstehen (Paket 105):** Die Anfrage für das Dokument
//! eines solchen Frames beginnt in der Sitzung der Seite
//! (`requestWillBeSent`), endet aber nur in der Sitzung des Frames
//! (`loadingFinished`). Hinge der Host erst bei der ersten Aufnahme an,
//! bliebe sie bis [`crate::live::LONG_REQUEST`] offen. Die zweite
//! Verbindung hängt sich deshalb vor dem Laden an die Seite und schaltet
//! dort `Target.setAutoAttach` ein (nur `iframe`-Ziele, angehalten bis zum
//! Start); je neuem Frame schaltet sie Network- und DOM-Agenten ein,
//! dasselbe Anhängen für Frames in diesem Frame, und lässt ihn dann
//! laufen. Die Nummer bleibt die des ersten Anhängens, auch wenn ein Frame
//! nach einer Navigation ein neues Ziel bekommt.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use chromiumoxide::cdp::browser_protocol::dom::{
    BackendNodeId, DescribeNodeParams, EventDocumentUpdated, GetDocumentParams, Node,
};
use chromiumoxide::cdp::browser_protocol::network::EnableParams;
use chromiumoxide::cdp::browser_protocol::target::{
    AttachToTargetParams, EventAttachedToTarget, FilterEntry, SessionId, SetAutoAttachParams,
    TargetFilter, TargetId,
};
use chromiumoxide::cdp::js_protocol::runtime::{EvaluateParams, RunIfWaitingForDebuggerParams};
use chromiumoxide::types::{CallId, CdpJsonEventMessage, Command, Message, MethodId};
use chromiumoxide::{Connection, Page};
use futures::StreamExt;
use serde_json::Value;
use tokio::sync::{mpsc, oneshot};

/// Elemente, die einen Frame besitzen (`HTMLFrameOwnerElement`).
const FRAME_OWNERS: &[&str] = &["iframe", "frame", "object", "embed", "fencedframe"];

/// Nummer eines Frames in den oberen Bits der Backend-ID im Modell.
const SHIFT: u32 = 32;

/// Backend-ID `backend` im Frame `index` als ID im Modell.
pub fn encode(index: i64, backend: i64) -> i64 {
    (index << SHIFT) | backend
}

/// ID im Modell → (Nummer des Frames, Backend-ID in dessen Prozess).
fn decode(id: i64) -> (i64, i64) {
    (id >> SHIFT, id & ((1 << SHIFT) - 1))
}

/// Ein angehängter Frame in einem anderen Prozess.
#[derive(Clone, Debug)]
pub struct Frame {
    /// Nummer für [`encode`], ab 1.
    pub index: i64,
    session: SessionId,
}

/// Wohin ein Befehl geht: an die Seite oder an einen Frame.
pub enum Doc<'a> {
    Page(&'a Page),
    Frame(&'a Frames, Frame),
}

impl Doc<'_> {
    pub async fn execute<C: Command>(&self, cmd: C) -> Result<C::Response> {
        match self {
            Doc::Page(page) => Ok(page.execute(cmd).await?.result),
            Doc::Frame(frames, frame) => frames.execute(Some(&frame.session), cmd).await,
        }
    }

    /// Nummer für [`encode`]: `0` für die Seite.
    pub fn index(&self) -> i64 {
        match self {
            Doc::Page(_) => 0,
            Doc::Frame(_, frame) => frame.index,
        }
    }
}

type Reply = oneshot::Sender<Result<Value>>;

/// Angehängte Frames nach Frame-ID (= Ziel-ID).
type Attached = Arc<Mutex<HashMap<String, Frame>>>;

/// Zweite Verbindung zum Browser mit den angehängten Frames einer Seite.
pub struct Frames {
    tx: mpsc::UnboundedSender<(MethodId, Option<SessionId>, Value, Reply)>,
    task: tokio::task::JoinHandle<()>,
    attached: Attached,
}

impl Frames {
    /// Verbindet sich mit dem Browser (`webSocketDebuggerUrl`) und hängt
    /// ab jetzt jeden Frame der Seite `page` in einem anderen Prozess beim
    /// Entstehen an. Dazu die Ereignisse der angehängten Frames für
    /// `live.rs`.
    pub async fn connect(
        ws_url: &str,
        page: &TargetId,
    ) -> Result<(Self, mpsc::UnboundedReceiver<CdpJsonEventMessage>)> {
        let mut conn = Connection::<CdpJsonEventMessage>::connect(ws_url).await?;
        let (tx, mut rx) = mpsc::unbounded_channel::<(MethodId, Option<SessionId>, Value, Reply)>();
        let (events, frame_events) = mpsc::unbounded_channel();
        let attached = Attached::default();
        let registry = attached.clone();
        let task = tokio::spawn(async move {
            let mut waiting: HashMap<CallId, Reply> = HashMap::new();
            loop {
                tokio::select! {
                    request = rx.recv() => {
                        let Some((method, session, params, reply)) = request else { break };
                        match conn.submit_command(method, session, params) {
                            Ok(id) => { waiting.insert(id, reply); }
                            Err(e) => { reply.send(Err(e.into())).ok(); }
                        }
                    }
                    message = conn.next() => match message {
                        Some(Ok(Message::Response(response))) => {
                            let Some(reply) = waiting.remove(&response.id) else { continue };
                            let result = match (response.result, response.error) {
                                (_, Some(error)) => Err(anyhow!("{}", error.message)),
                                (result, None) => Ok(result.unwrap_or(Value::Null)),
                            };
                            reply.send(result).ok();
                        }
                        Some(Ok(Message::Event(event))) => {
                            // Die Antworten der folgenden Befehle warten auf
                            // niemanden; die Reihenfolge je Sitzung hält der
                            // Browser ein. Auch das Anhängen an die Seite
                            // selbst meldet `attachedToTarget`; nur Frames
                            // zählen.
                            if event.method == EventAttachedToTarget::IDENTIFIER {
                                if let Some(e) = serde_json::from_value::<EventAttachedToTarget>(event.params.clone())
                                    .ok()
                                    .filter(|e| e.target_info.r#type == "iframe")
                                {
                                    register(&registry, &e);
                                    // Änderungssignal (Anfragen, Mutationen),
                                    // Frames in diesem Frame, dann laufen lassen.
                                    submit(&mut conn, &e.session_id, EnableParams::default());
                                    submit(&mut conn, &e.session_id, full_document());
                                    submit(&mut conn, &e.session_id, auto_attach());
                                    submit(&mut conn, &e.session_id, RunIfWaitingForDebuggerParams::default());
                                }
                            }
                            // Wie `live.rs` für die Seite: Nach einem
                            // ersetzten Dokument meldet der DOM-Agent erst
                            // wieder, wenn es angefordert ist.
                            if event.method == EventDocumentUpdated::IDENTIFIER {
                                if let Some(session) = event.session_id.clone() {
                                    submit(&mut conn, &SessionId::from(session), full_document());
                                }
                            }
                            events.send(event).ok();
                        }
                        Some(Err(_)) => {}
                        None => break,
                    }
                }
            }
        });
        let frames = Frames { tx, task, attached };
        let params = AttachToTargetParams::builder()
            .target_id(page.clone())
            .flatten(true)
            .build()
            .map_err(|e| anyhow!(e))?;
        let session = frames.execute(None, params).await?.session_id;
        frames.execute(Some(&session), auto_attach()).await?;
        Ok((frames, frame_events))
    }

    async fn execute<C: Command>(
        &self,
        session: Option<&SessionId>,
        cmd: C,
    ) -> Result<C::Response> {
        let (reply, answer) = oneshot::channel();
        self.tx
            .send((
                cmd.identifier(),
                session.cloned(),
                serde_json::to_value(&cmd)?,
                reply,
            ))
            .map_err(|_| anyhow!("Verbindung zu den Frames geschlossen"))?;
        Ok(C::response_from_value(answer.await??)?)
    }

    /// Angehängter Frame `frame_id` in einem anderen Prozess. Fehler, wenn es
    /// keiner ist (Frame im Prozess der Seite).
    pub fn attached(&self, frame_id: &str) -> Result<Frame> {
        self.known(frame_id)
            .ok_or_else(|| anyhow!("kein angehängter Frame in einem anderen Prozess"))
    }

    /// Schon angehängter Frame mit dieser Frame-ID.
    fn known(&self, frame_id: &str) -> Option<Frame> {
        self.attached.lock().expect("Frames").get(frame_id).cloned()
    }

    /// ID im Modell → Ziel des Befehls und Backend-ID dort.
    pub fn resolve<'a>(&'a self, page: &'a Page, id: i64) -> Result<(Doc<'a>, i64)> {
        let (index, backend) = decode(id);
        if index == 0 {
            return Ok((Doc::Page(page), backend));
        }
        let frame = self
            .attached
            .lock()
            .expect("Frames")
            .values()
            .find(|f| f.index == index)
            .cloned()
            .ok_or_else(|| anyhow!("Frame {index} nicht angehängt"))?;
        Ok((Doc::Frame(self, frame), backend))
    }

    /// Dokument der Seite (`DOM.getDocument`, Tiefe -1, `pierce`) mit den
    /// Dokumenten der Frames in anderen Prozessen als `content_document`
    /// ihres `iframe`; deren Backend-IDs über [`encode`]. Dazu die
    /// eingehängten Frames in Dokumentreihenfolge (äußere vor inneren).
    pub async fn document(&self, page: &Page) -> Result<(Node, Vec<Frame>)> {
        let mut root = Doc::Page(page)
            .execute(GetDocumentParams::builder().depth(-1).pierce(true).build())
            .await?
            .root;
        let mut grafted = Vec::new();
        self.graft(&mut root, &mut grafted).await;
        Ok((root, grafted))
    }

    fn graft<'a>(
        &'a self,
        node: &'a mut Node,
        grafted: &'a mut Vec<Frame>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            // `frameId` tragen Frame-Besitzer, aber auch Dokument und
            // Wurzelelement eines Frames; nur Besitzer ohne Dokument zeigen
            // auf einen anderen Prozess.
            if FRAME_OWNERS.contains(&node.local_name.as_str()) && node.content_document.is_none() {
                if let Some(frame_id) = node.frame_id.clone() {
                    // Kein eigenes Ziel: Frame ohne Dokument (etwa noch
                    // nicht geladen) — dann bleibt das iframe leer.
                    if let Ok((frame, document)) = self.frame_document(frame_id.as_ref()).await {
                        grafted.push(frame);
                        node.content_document = Some(Box::new(document));
                    }
                }
            }
            for child in node.children.iter_mut().flatten() {
                self.graft(child, grafted).await;
            }
            for shadow in node.shadow_roots.iter_mut().flatten() {
                self.graft(shadow, grafted).await;
            }
            if let Some(document) = node.content_document.as_deref_mut() {
                self.graft(document, grafted).await;
            }
        })
    }

    async fn frame_document(&self, frame_id: &str) -> Result<(Frame, Node)> {
        let frame = self.attached(frame_id)?;
        let mut document = self
            .execute(Some(&frame.session), full_document())
            .await?
            .root;
        renumber(&mut document, frame.index);
        Ok((frame, document))
    }

    /// Fokussiertes Element als ID im Modell. Liegt der Fokus in einem Frame
    /// eines anderen Prozesses, meldet die Seite dessen `iframe`; dann im
    /// Frame weiterfragen.
    pub async fn active_element(&self, page: &Page) -> Result<i64> {
        let mut doc = Doc::Page(page);
        loop {
            let node = describe(&doc, "document.activeElement").await?;
            let backend = encode(doc.index(), *node.backend_node_id.inner());
            match node.frame_id.and_then(|id| self.known(id.as_ref())) {
                Some(frame) => doc = Doc::Frame(self, frame),
                None => return Ok(backend),
            }
        }
    }
}

/// Neu angehängten Frame eintragen. Ein Frame, der schon eine Nummer hat
/// (neues Ziel nach einer Navigation), behält sie.
fn register(attached: &Attached, e: &EventAttachedToTarget) {
    let mut attached = attached.lock().expect("Frames");
    let frame_id = e.target_info.target_id.as_ref().to_string();
    let index = match attached.get(&frame_id) {
        Some(frame) => frame.index,
        None => attached.len() as i64 + 1,
    };
    attached.insert(
        frame_id,
        Frame {
            index,
            session: e.session_id.clone(),
        },
    );
}

/// Frames beim Entstehen anhängen, angehalten bis
/// `Runtime.runIfWaitingForDebugger`; nur `iframe`-Ziele, keine Worker.
fn auto_attach() -> SetAutoAttachParams {
    let only_iframes = TargetFilter::new(vec![
        FilterEntry::builder().r#type("iframe").build(),
        FilterEntry::builder().exclude(true).build(),
    ]);
    SetAutoAttachParams::builder()
        .auto_attach(true)
        .wait_for_debugger_on_start(true)
        .flatten(true)
        .filter(only_iframes)
        .build()
        .expect("SetAutoAttachParams vollständig")
}

/// Ganzes Dokument (der DOM-Agent meldet Mutationen nur für übertragene
/// Knoten).
fn full_document() -> GetDocumentParams {
    GetDocumentParams::builder().depth(-1).pierce(true).build()
}

/// Befehl ohne Warten auf die Antwort (aus der Schleife der Verbindung).
fn submit<C: Command>(conn: &mut Connection<CdpJsonEventMessage>, session: &SessionId, cmd: C) {
    let params = serde_json::to_value(&cmd).expect("Befehl ist serialisierbar");
    conn.submit_command(cmd.identifier(), Some(session.clone()), params)
        .ok();
}

impl Drop for Frames {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// Element, zu dem `expression` im Dokument auswertet.
pub async fn describe(doc: &Doc<'_>, expression: &str) -> Result<Node> {
    let eval = EvaluateParams::builder()
        .expression(expression)
        .build()
        .map_err(|e| anyhow!(e))?;
    let object_id = doc
        .execute(eval)
        .await?
        .result
        .object_id
        .ok_or_else(|| anyhow!("{expression}: kein Element"))?;
    Ok(doc
        .execute(DescribeNodeParams::builder().object_id(object_id).build())
        .await?
        .node)
}

/// Backend-IDs eines Frame-Dokuments über [`encode`].
fn renumber(node: &mut Node, index: i64) {
    let id = |b: &BackendNodeId| BackendNodeId::new(encode(index, *b.inner()));
    node.backend_node_id = id(&node.backend_node_id);
    for n in node.distributed_nodes.iter_mut().flatten() {
        n.backend_node_id = id(&n.backend_node_id);
    }
    for child in node
        .children
        .iter_mut()
        .flatten()
        .chain(node.shadow_roots.iter_mut().flatten())
        .chain(node.pseudo_elements.iter_mut().flatten())
        .chain(node.content_document.as_deref_mut())
        .chain(node.template_content.as_deref_mut())
    {
        renumber(child, index);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seite_bleibt_frame_oben() {
        assert_eq!(encode(0, 42), 42);
        assert_eq!(decode(42), (0, 42));
        let id = encode(3, 42);
        assert_ne!(id, 42);
        assert_eq!(decode(id), (3, 42));
    }
}
