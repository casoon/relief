//! Antworten aufzeichnen und wiedergeben.
//!
//! Ein Kalibrierlauf mit Key kostet Geld; seine Antworten werden als JSON
//! gespeichert ([`Recording`]) und lassen sich ohne Key und ohne Netz erneut
//! auswerten ([`Replay`]), etwa mit geänderter Trefferregel. Tests nutzen
//! denselben Weg mit ausgedachten Antworten.
//!
//! Schlüssel einer Anfrage ist Seitenadresse plus Fokus-Knoten des
//! Ausschnitts ([`key`]).

use std::cell::RefCell;
use std::collections::BTreeMap;

use relief_ai_contract::{
    FilteredInput, ModelId, ModelProvider, ModelReply, ModelRequest, ProviderError, Tier, Usage,
};
use serde::{Deserialize, Serialize};

/// Eine gespeicherte Antwort.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordedReply {
    /// [`ModelId`] als Text, z. B. `api:anthropic@claude-haiku-4-5-20251001`.
    pub model: String,
    pub text: String,
    pub usage: Option<Usage>,
}

/// Gespeicherte Antworten nach [`key`].
pub type Replies = BTreeMap<String, Result<RecordedReply, String>>;

/// `<Seitenadresse> <Fokus>`, z. B. `file:///REPO/spike/fixtures/shop-broken.html t0:15`.
pub fn key(input: &FilteredInput) -> String {
    format!(
        "{} {}",
        input.page().url.as_deref().unwrap_or_default(),
        input.focus().unwrap_or_default()
    )
}

fn parse_model_id(text: &str) -> ModelId {
    let (tier, rest) = text.split_once(':').unwrap_or(("api", text));
    let (name, version) = rest.split_once('@').unwrap_or((rest, ""));
    ModelId {
        tier: match tier {
            "os" => Tier::Os,
            "local" => Tier::Local,
            "none" => Tier::None,
            _ => Tier::Api,
        },
        name: name.into(),
        version: version.into(),
    }
}

/// Gibt gespeicherte Antworten wieder; ohne Eintrag: keine Antwort.
#[derive(Debug, Clone, Default)]
pub struct Replay {
    pub replies: Replies,
}

impl ModelProvider for Replay {
    fn tier(&self) -> Tier {
        Tier::Api
    }

    fn complete(&self, request: &ModelRequest) -> Result<Option<ModelReply>, ProviderError> {
        match self.replies.get(&key(request.input())) {
            None => Ok(None),
            Some(Err(e)) => Err(ProviderError(e.clone())),
            Some(Ok(r)) => Ok(Some(ModelReply {
                model: parse_model_id(&r.model),
                text: r.text.clone(),
                usage: r.usage,
            })),
        }
    }
}

/// Reicht an einen Anbieter durch und merkt sich jede Antwort.
pub struct Recording<'a> {
    inner: &'a dyn ModelProvider,
    replies: RefCell<Replies>,
}

impl<'a> Recording<'a> {
    pub fn new(inner: &'a dyn ModelProvider) -> Self {
        Self {
            inner,
            replies: RefCell::default(),
        }
    }

    pub fn into_replies(self) -> Replies {
        self.replies.into_inner()
    }
}

impl ModelProvider for Recording<'_> {
    fn tier(&self) -> Tier {
        self.inner.tier()
    }

    fn complete(&self, request: &ModelRequest) -> Result<Option<ModelReply>, ProviderError> {
        let result = self.inner.complete(request);
        let recorded = match &result {
            Ok(None) => None,
            Ok(Some(r)) => Some(Ok(RecordedReply {
                model: r.model.to_string(),
                text: r.text.clone(),
                usage: r.usage,
            })),
            Err(e) => Some(Err(e.0.clone())),
        };
        if let Some(recorded) = recorded {
            self.replies
                .borrow_mut()
                .insert(key(request.input()), recorded);
        }
        result
    }
}
