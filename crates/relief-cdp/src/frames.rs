//! Frames in einem anderen Renderer-Prozess (Site Isolation, Paket 70).
//!
//! Chrome gibt einem iframe fremder Site einen eigenen Prozess. Die Sitzung
//! der Seite erreicht ihn nicht: `getFullAXTree { frameId }` antwortet nicht,
//! `DOM.getDocument` (`pierce`) liefert am `iframe` nur die `frameId`, kein
//! Dokument. Ein solcher Frame ist ein eigenes CDP-Ziel mit derselben ID wie
//! der Frame; der Host hängt sich über eine zweite Verbindung zum Browser
//! flach an (`Target.attachToTarget`, `flatten`) und schickt Befehle mit der
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

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Mutex;

use anyhow::{anyhow, Result};
use chromiumoxide::cdp::browser_protocol::dom::{
    BackendNodeId, DescribeNodeParams, EventDocumentUpdated, GetDocumentParams, Node,
};
use chromiumoxide::cdp::browser_protocol::network::EnableParams;
use chromiumoxide::cdp::browser_protocol::target::{AttachToTargetParams, SessionId, TargetId};
use chromiumoxide::cdp::js_protocol::runtime::EvaluateParams;
use chromiumoxide::types::{CallId, CdpJsonEventMessage, Command, Message, Method, MethodId};
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

/// Zweite Verbindung zum Browser mit den angehängten Frames einer Seite.
pub struct Frames {
    tx: mpsc::UnboundedSender<(MethodId, Option<SessionId>, Value, Reply)>,
    task: tokio::task::JoinHandle<()>,
    /// Angehängte Frames nach Frame-ID (= Ziel-ID).
    attached: Mutex<HashMap<String, Frame>>,
}

impl Frames {
    /// Verbindet sich mit dem Browser (`webSocketDebuggerUrl`). Dazu die
    /// Ereignisse der angehängten Frames für `live.rs`.
    pub async fn connect(
        ws_url: &str,
    ) -> Result<(Self, mpsc::UnboundedReceiver<CdpJsonEventMessage>)> {
        let mut conn = Connection::<CdpJsonEventMessage>::connect(ws_url).await?;
        let (tx, mut rx) = mpsc::unbounded_channel::<(MethodId, Option<SessionId>, Value, Reply)>();
        let (events, frame_events) = mpsc::unbounded_channel();
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
                            // Wie `live.rs` für die Seite: Nach einem
                            // ersetzten Dokument meldet der DOM-Agent erst
                            // wieder, wenn es angefordert ist. Die Antwort
                            // wartet auf niemanden.
                            if event.method == EventDocumentUpdated::IDENTIFIER {
                                let cmd = GetDocumentParams::builder().depth(-1).pierce(true).build();
                                let params = serde_json::to_value(&cmd)
                                    .expect("GetDocumentParams ist serialisierbar");
                                let session = event.session_id.clone().map(SessionId::from);
                                conn.submit_command(cmd.identifier(), session, params).ok();
                            }
                            events.send(event).ok();
                        }
                        Some(Err(_)) => {}
                        None => break,
                    }
                }
            }
        });
        let frames = Frames {
            tx,
            task,
            attached: Mutex::new(HashMap::new()),
        };
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

    /// Frame `frame_id` in einem anderen Prozess anhängen; schon angehängte
    /// kommen aus dem Speicher. Fehler, wenn es kein eigenes Ziel ist (Frame
    /// im Prozess der Seite).
    pub async fn attach(&self, frame_id: &str) -> Result<Frame> {
        if let Some(frame) = self.known(frame_id) {
            return Ok(frame);
        }
        let params = AttachToTargetParams::builder()
            .target_id(TargetId::from(frame_id.to_string()))
            .flatten(true)
            .build()
            .map_err(|e| anyhow!(e))?;
        let session = self.execute(None, params).await?.session_id;
        // Änderungssignal: Anfragen und Mutationen des Frames an `live.rs`.
        self.execute(Some(&session), EnableParams::default())
            .await?;
        self.execute(
            Some(&session),
            GetDocumentParams::builder().depth(-1).pierce(true).build(),
        )
        .await?;
        let mut attached = self.attached.lock().expect("Frames");
        let frame = Frame {
            index: attached.len() as i64 + 1,
            session,
        };
        attached.insert(frame_id.to_string(), frame.clone());
        Ok(frame)
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
        let frame = self.attach(frame_id).await?;
        let mut document = self
            .execute(
                Some(&frame.session),
                GetDocumentParams::builder().depth(-1).pierce(true).build(),
            )
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
