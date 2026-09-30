//! Ausführung eines validierten `ActionPlan` über CDP.
//!
//! Anders als im Fork (`AXActionData`) läuft das hier über DOM und JavaScript
//! am Element mit der Backend-ID. Deshalb erreicht der Spike z. B. keine
//! ARIA-Widgets, die nur auf echte Tastatur-/Mausereignisse reagieren
//! (→ `plan/spezifikation/09`, Phase 0a).

use anyhow::{anyhow, Result};
use chromiumoxide::cdp::browser_protocol::dom::{BackendNodeId, FocusParams, ResolveNodeParams};
use chromiumoxide::cdp::browser_protocol::input::{DispatchKeyEventParams, DispatchKeyEventType};
use chromiumoxide::cdp::js_protocol::runtime::{CallArgument, CallFunctionOnParams};
use chromiumoxide::Page;
use relief_interaction::{ActionKind, ActionPlan, ScrollDirection};

const ACTIVATE: &str =
    "function() { this.scrollIntoView({block: 'center'}); this.focus(); this.click(); }";

/// Setzt den Wert über den nativen Setter, damit Frameworks (React u. a.)
/// die Änderung bemerken.
const SET_VALUE: &str = "function(v) {
  this.scrollIntoView({block: 'center'});
  this.focus();
  const proto = Object.getPrototypeOf(this);
  const desc = Object.getOwnPropertyDescriptor(proto, 'value');
  if (desc && desc.set) { desc.set.call(this, v); } else { this.value = v; }
  this.dispatchEvent(new Event('input', {bubbles: true}));
  this.dispatchEvent(new Event('change', {bubbles: true}));
}";

/// Überschriften und Bereiche sind nicht fokussierbar: für die Dauer des
/// Fokus `tabindex="-1"` setzen (wie Sprunglinks es tun), danach wieder
/// entfernen. So beginnt auch die Tab-Reihenfolge an dieser Stelle.
const NAVIGATE: &str = "function() {
  if (!this.hasAttribute('tabindex')) {
    this.setAttribute('tabindex', '-1');
    this.addEventListener('blur', () => this.removeAttribute('tabindex'), {once: true});
  }
  this.focus({preventScroll: true});
  this.scrollIntoView({block: 'start'});
}";

/// Scrollt das Dokument und liefert `[vorher, nachher, größte Position]`.
const SCROLL: &str = "(dir) => {
  const el = document.scrollingElement || document.documentElement;
  const before = el.scrollTop;
  const page = Math.round(window.innerHeight * 0.8);
  const top = {down: before + page, up: before - page, top: 0, bottom: el.scrollHeight}[dir];
  window.scrollTo({top, behavior: 'instant'});
  return [before, el.scrollTop, el.scrollHeight - el.clientHeight];
}";

const SELECT: &str = "function(label) {
  if (this.tagName !== 'SELECT') { throw new Error('kein natives select'); }
  const opt = Array.from(this.options).find(o => o.label.trim() === label || o.text.trim() === label);
  if (!opt) { throw new Error('Option fehlt'); }
  this.focus();
  this.value = opt.value;
  this.dispatchEvent(new Event('input', {bubbles: true}));
  this.dispatchEvent(new Event('change', {bubbles: true}));
}";

pub async fn execute(page: &Page, plan: &ActionPlan) -> Result<()> {
    let backend = BackendNodeId::new(plan.dom_node_id);
    match &plan.kind {
        ActionKind::Focus => {
            page.execute(FocusParams::builder().backend_node_id(backend).build())
                .await?;
            call(
                page,
                backend,
                "function() { this.scrollIntoView({block: 'center'}); }",
                None,
            )
            .await
        }
        ActionKind::Activate => call(page, backend, ACTIVATE, None).await,
        ActionKind::SetValue(v) => call(page, backend, SET_VALUE, Some(v)).await,
        ActionKind::Select(label) => call(page, backend, SELECT, Some(label)).await,
        ActionKind::NavigateTo => call(page, backend, NAVIGATE, None).await,
        // Echte Pfeiltasten am fokussierten Element: wirken auf native
        // Zahlen-/Bereichsfelder und auf ARIA-Widgets, die nur auf Tasten
        // hören (wie `Increment`/`Decrement` im Fork).
        ActionKind::Increment | ActionKind::Decrement => {
            page.execute(FocusParams::builder().backend_node_id(backend).build())
                .await?;
            if plan.kind == ActionKind::Increment {
                press_key(page, "ArrowUp", 38).await
            } else {
                press_key(page, "ArrowDown", 40).await
            }
        }
    }
}

/// Seite scrollen: `(vorher, nachher, größte Position)` in Pixeln.
pub async fn scroll(page: &Page, direction: ScrollDirection) -> Result<(f64, f64, f64)> {
    let dir = match direction {
        ScrollDirection::Down => "down",
        ScrollDirection::Up => "up",
        ScrollDirection::Top => "top",
        ScrollDirection::Bottom => "bottom",
    };
    let result = page.evaluate(format!("({SCROLL})('{dir}')")).await?;
    let [before, after, max]: [f64; 3] = result.into_value()?;
    Ok((before, after, max))
}

/// Escape an das fokussierte Element, wie eine echte Taste.
pub async fn press_escape(page: &Page) -> Result<()> {
    press_key(page, "Escape", 27).await
}

/// Tab wie eine echte Taste (Formular-Zusicherung `tabfolge`).
pub async fn press_tab(page: &Page) -> Result<()> {
    press_key(page, "Tab", 9).await
}

/// Taste ohne Text (Name = Code) an das fokussierte Element.
async fn press_key(page: &Page, key: &str, code: i64) -> Result<()> {
    for kind in [
        DispatchKeyEventType::RawKeyDown,
        DispatchKeyEventType::KeyUp,
    ] {
        let event = DispatchKeyEventParams::builder()
            .r#type(kind)
            .key(key)
            .code(key)
            .windows_virtual_key_code(code)
            .native_virtual_key_code(code)
            .build()
            .map_err(|e| anyhow!(e))?;
        page.execute(event).await?;
    }
    Ok(())
}

async fn call(
    page: &Page,
    backend: BackendNodeId,
    function: &str,
    arg: Option<&str>,
) -> Result<()> {
    let resolved = page
        .execute(
            ResolveNodeParams::builder()
                .backend_node_id(backend)
                .build(),
        )
        .await?;
    let object_id = resolved
        .result
        .object
        .object_id
        .clone()
        .ok_or_else(|| anyhow!("Element nicht auflösbar"))?;
    let mut params = CallFunctionOnParams::builder()
        .function_declaration(function)
        .object_id(object_id)
        .user_gesture(true);
    if let Some(arg) = arg {
        params = params.argument(
            CallArgument::builder()
                .value(serde_json::json!(arg))
                .build(),
        );
    }
    let result = page
        .execute(params.build().map_err(|e| anyhow!(e))?)
        .await?;
    if let Some(ex) = &result.result.exception_details {
        return Err(anyhow!("Fehler im Element: {}", ex.text));
    }
    Ok(())
}
