//! Variante A: `cxx`-Bridge für den Aufruf aus C++ im selben Prozess.
//!
//! Nur die Rust-Seite; die C++-Seite erzeugt Chromiums GN-Template
//! `rust_static_library` mit `cxx_bindings` (→ `plan/spezifikation/02`).
//! Dieselben flachen Strukturen spiegelt der Mojo-Entwurf
//! (`mojom/relief_runtime.mojom`, Variante B); im Utility-Prozess kopiert
//! der C++-Empfänger die Mojo-Strukturen in diese hier.
//!
//! Regeln an der Grenze:
//! - `cxx` kennt in geteilten Strukturen kein `Option`; optionale Werte haben
//!   ein `has_*`-Feld, optionale Enums eine Variante `Unset`.
//! - Kein `Result` über die Grenze: `cxx` übersetzt es in eine C++-Ausnahme,
//!   Chromium baut ohne Ausnahmen (bestätigt im Fork-Build). Ergebnisse tragen
//!   `ok` und einen Fehlertext.
//! - Chromium baut Rust mit `panic=abort` (→ 02); nichts hier darf an
//!   Eingaben aus C++ in Panik geraten. Ungültige Enum-Werte (in C++ möglich)
//!   werden zu Fehlern.
//! - Aus Chromium kommen nur `Known`-Aussagen; die Gegenrichtung
//!   ([`delta_to_ffi`]) verliert deshalb Certainty, Quelle und Evidence
//!   abgeleiteter Aussagen und die nur von CDP gemeldeten `ignored_reasons`.

use std::collections::BTreeMap;

use relief_model::{
    Action, BoundsChange, Certainty, Editable, Fact, GraphVersion, HasPopup, Invalid, NameFrom,
    NodeId, NodeRef, Orientation, Rect, Relations, Role, SemanticNode, States, Toggle, TreeData,
    TreeDelta, TreeId, TreeUpdate,
};

use relief_interaction::{ScrollDirection, TaskLine};

use crate::command::{Key, Reply, Step};
use crate::runtime::{ActionRequest, Rejection, Runtime};

#[cxx::bridge(namespace = "relief::bridge")]
pub mod ffi {
    /// `Option<bool>`.
    #[derive(Debug)]
    enum OptBool {
        Unset,
        False,
        True,
    }

    #[derive(Debug)]
    enum Toggle {
        Unset,
        False,
        True,
        Mixed,
    }

    #[derive(Debug)]
    enum Invalid {
        Unset,
        True,
        Spelling,
        Grammar,
    }

    #[derive(Debug)]
    enum Editable {
        Unset,
        Plaintext,
        Richtext,
    }

    #[derive(Debug)]
    enum HasPopup {
        Unset,
        True,
        Menu,
        Listbox,
        Tree,
        Grid,
        Dialog,
    }

    #[derive(Debug)]
    enum Orientation {
        Unset,
        Horizontal,
        Vertical,
    }

    #[derive(Debug)]
    enum NameFrom {
        Unset,
        Attribute,
        RelatedElement,
        Contents,
        Placeholder,
        Title,
    }

    /// Auswahl aus `ax::mojom::Action`.
    #[derive(Debug)]
    enum Action {
        DoDefault,
        Focus,
        Blur,
        SetValue,
        Increment,
        Decrement,
        Expand,
        Collapse,
        ScrollToMakeVisible,
        ShowContextMenu,
        SetSequentialFocusNavigationStartingPoint,
    }

    #[derive(Debug)]
    enum Certainty {
        Known,
        Inferred,
        Uncertain,
    }

    #[derive(Debug)]
    enum Rejection {
        None,
        Stale,
        UnknownNode,
        Ignored,
        Disabled,
        Unsupported,
        ValueMismatch,
        /// Die Anfrage selbst ist ungültig (z. B. unbekannter Enum-Wert).
        Invalid,
    }

    #[derive(Debug, Clone, Copy, Default)]
    struct Rect {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    }

    #[derive(Debug, Clone, Default)]
    struct Attribute {
        key: String,
        value: String,
    }

    /// Ein Knoten, vollständig (wie `SemanticNode`).
    #[derive(Debug, Clone)]
    struct Node {
        id: i32,
        /// Relief-Rollenname (ARIA, sonst Chromium-intern in lowerCamelCase).
        role: String,
        has_name: bool,
        name: String,
        name_from: NameFrom,
        has_description: bool,
        description: String,
        has_value: bool,
        value: String,
        focusable: bool,
        disabled: bool,
        readonly: bool,
        required: bool,
        modal: bool,
        multiline: bool,
        multiselectable: bool,
        settable: bool,
        busy: bool,
        expanded: OptBool,
        selected: OptBool,
        checked: Toggle,
        pressed: Toggle,
        invalid: Invalid,
        editable: Editable,
        has_popup: HasPopup,
        orientation: Orientation,
        actions: Vec<Action>,
        has_bounds: bool,
        bounds: Rect,
        ignored: bool,
        has_level: bool,
        level: u32,
        has_url: bool,
        url: String,
        has_parent: bool,
        parent: i32,
        children: Vec<i32>,
        has_child_tree: bool,
        child_tree: String,
        labelled_by: Vec<i32>,
        described_by: Vec<i32>,
        controls: Vec<i32>,
        details: Vec<i32>,
        error_message: Vec<i32>,
        flow_to: Vec<i32>,
        has_active_descendant: bool,
        active_descendant: i32,
        has_dom_node_id: bool,
        dom_node_id: i64,
        extra: Vec<Attribute>,
    }

    #[derive(Debug, Clone, Default)]
    struct TreeData {
        has_parent: bool,
        parent_tree: String,
        parent_node: i32,
        has_url: bool,
        url: String,
        has_title: bool,
        title: String,
        has_focus: bool,
        focus: i32,
    }

    #[derive(Debug, Clone, Default)]
    struct BoundsChange {
        node: i32,
        has_bounds: bool,
        bounds: Rect,
    }

    #[derive(Debug, Clone, Default)]
    struct TreeUpdate {
        tree: String,
        has_data: bool,
        data: TreeData,
        has_root: bool,
        root: i32,
        removed: Vec<i32>,
        created: Vec<Node>,
        changed: Vec<Node>,
        bounds: Vec<BoundsChange>,
    }

    /// `TreeDelta`.
    #[derive(Debug, Clone, Default)]
    struct Delta {
        base: u64,
        has_root: bool,
        root: String,
        removed_trees: Vec<String>,
        trees: Vec<TreeUpdate>,
    }

    #[derive(Debug, Clone, Default)]
    struct ApplyResult {
        ok: bool,
        /// Version nach dem Anwenden (bei Fehler: unverändert).
        version: u64,
        error: String,
    }

    #[derive(Debug, Clone)]
    struct ActionRequest {
        tree: String,
        node: i32,
        action: Action,
        has_value: bool,
        value: String,
        version: u64,
    }

    /// Geprüfte Aktion; bei `ok == false` steht der Grund in `rejection`.
    #[derive(Debug, Clone)]
    struct ActionPlan {
        ok: bool,
        rejection: Rejection,
        tree: String,
        node: i32,
        action: Action,
        has_value: bool,
        value: String,
        version: u64,
    }

    /// Ergebnis einer Suche; `found == false`: kein Treffer.
    #[derive(Debug, Clone)]
    struct Found {
        found: bool,
        tree: String,
        node: i32,
        version: u64,
    }

    #[derive(Debug, Clone)]
    struct Answer {
        found: bool,
        version: u64,
        role: String,
        has_name: bool,
        name: String,
        name_certainty: Certainty,
    }

    /// Befehle in Sprache (→ `command`).
    #[derive(Debug)]
    enum ReplyKind {
        Answer,
        Perform,
        Escape,
        Scroll,
    }

    #[derive(Debug)]
    enum ScrollDirection {
        Down,
        Up,
        Top,
        Bottom,
    }

    /// Taste an das fokussierte Element (Ersatzweg ohne `AXActionData`).
    #[derive(Debug)]
    enum Key {
        None,
        Escape,
        ArrowUp,
        ArrowDown,
    }

    /// Ein Schritt: `key != None` ist eine Taste, sonst eine Aktion über
    /// `AXActionData` an (`tree`, `node`).
    #[derive(Debug, Clone)]
    struct Step {
        key: Key,
        tree: String,
        node: i32,
        action: Action,
        has_value: bool,
        value: String,
    }

    /// Antwort auf eine Eingabe: `text` bei `Answer`, `steps` bei `Perform`,
    /// `scroll` bei `Scroll`.
    #[derive(Debug, Clone)]
    struct Reply {
        kind: ReplyKind,
        text: String,
        steps: Vec<Step>,
        scroll: ScrollDirection,
    }

    /// Sprungmarke mit Position (Seitenkoordinaten, CSS-Pixel).
    #[derive(Debug, Clone)]
    struct MarkBox {
        label: String,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        /// Name nicht gesichert (erschlossen, unsicher, fehlt).
        uncertain: bool,
    }

    /// Autocomplete eines Felds im Formular eines Ziels (Paket 75).
    #[derive(Debug, Clone)]
    struct FieldFact {
        node: i32,
        autocomplete: String,
    }

    /// Angaben des Renderers zum Formular eines Ziels (Paket 75): Knoten im
    /// Baum `tree`.
    #[derive(Debug, Clone)]
    struct FormFacts {
        tree: String,
        node: i32,
        has_action: bool,
        action: String,
        fields: Vec<FieldFact>,
    }

    /// Antwort einer Methode der Domäne `Relief.*` (Paket 45): `json` ist
    /// bei `ok` das Ergebnis, sonst die Fehlermeldung.
    #[derive(Debug, Clone)]
    struct DevToolsReply {
        ok: bool,
        json: String,
    }

    #[derive(Debug)]
    enum TaskKind {
        Url,
        Do,
        /// Formular-Zusicherung; braucht DOM-Fakten, nur im CDP-Host.
        Assert,
        Expect,
    }

    /// Zeile einer Aufgabendatei (`spike/tasks/*.txt`).
    #[derive(Debug, Clone)]
    struct Task {
        kind: TaskKind,
        text: String,
    }

    extern "Rust" {
        type Runtime;

        fn new_runtime() -> Box<Runtime>;
        /// Delta anwenden; läuft nicht im AX-Callback des UI-Threads
        /// (→ `plan/spezifikation/02`).
        fn apply_delta(runtime: &mut Runtime, delta: &Delta) -> ApplyResult;
        fn describe_node(runtime: &Runtime, tree: &str, node: i32) -> Answer;
        fn plan_action(runtime: &Runtime, request: &ActionRequest) -> ActionPlan;
        /// Erster Knoten mit genau diesem Namen, der `action` meldet
        /// (`Runtime::find`).
        fn find_node(runtime: &Runtime, name: &str, action: Action) -> Found;
        /// Knoten über alle Bäume.
        fn node_count(runtime: &Runtime) -> u64;

        /// Eingabe in Sprache gegen den aktuellen Graphen (`Runtime::command`).
        fn run_command(runtime: &mut Runtime, input: &str) -> Reply;
        /// Ziel der offenen Rückfrage (`found` false: keine offen).
        fn confirmation_target(runtime: &Runtime) -> Found;
        /// Wofür vor `input` Angaben beim Renderer anzufragen sind
        /// (`Runtime::facts_target`; `found` false: nichts).
        fn facts_target(runtime: &Runtime, input: &str) -> Found;
        /// Angaben des Renderers zum Formular ins Modell
        /// (`Runtime::apply_form_facts`).
        fn apply_form_facts(runtime: &mut Runtime, facts: &FormFacts);
        /// Offene Rückfrage mit dem jetzigen Modell neu stellen
        /// (`Runtime::reconfirm`).
        fn reconfirm(runtime: &mut Runtime) -> Reply;
        /// Bedienung aus der Semantic View (`Runtime::view_act`).
        fn view_act(runtime: &mut Runtime, key: &str, kind: &str, value: &str) -> Reply;
        /// Methode der CDP-Domäne `Relief.*`, Parameter als JSON
        /// (`devtools::command`).
        fn devtools_command(runtime: &Runtime, method: &str, params: &str) -> DevToolsReply;
        /// Eingabe für Protokoll und Log der Befehlsleiste, Wert verdeckt
        /// (`relief_interaction::redact_input`).
        fn redact_input(input: &str) -> String;
        /// Antwort auf die ausgeführten Schritte bzw. Escape, gegen den
        /// jetzigen Graphen (`Runtime::finish`).
        fn finish_command(runtime: &mut Runtime) -> String;
        /// Security-Log seit dem letzten Abholen, ein JSON-Objekt je Eintrag
        /// (`Runtime::take_security_log`).
        fn take_security_log(runtime: &mut Runtime) -> Vec<String>;
        /// Seitenbeschreibung („was ist hier“).
        fn describe_page(runtime: &Runtime) -> String;
        /// Antwort nach dem Scrollen: Position vorher, nachher, größte
        /// Position (gleiche Einheit).
        fn scrolled_text(direction: ScrollDirection, before: f64, after: f64, max: f64) -> String;
        /// Inspector: Bereiche, Überschriften, Bedienelemente als JSON.
        fn inspector_json(runtime: &Runtime) -> String;
        /// Inspector „im Dokument zeigen“: Schritte zum Eintrag `key`
        /// (Fokus bzw. Hinbewegen), danach `finish_command`.
        fn show_node(runtime: &mut Runtime, key: &str) -> Reply;
        /// Sprungmarken des aktuellen Stands (merkt sie für „marke …“).
        fn show_marks(runtime: &mut Runtime) -> Vec<MarkBox>;
        fn parse_task_file(text: &str) -> Vec<Task>;
        /// Teilstring ohne Groß-/Kleinschreibung (auch Umlaute).
        fn expectation_met(answer: &str, expected: &str) -> bool;
    }
}

fn new_runtime() -> Box<Runtime> {
    Box::new(Runtime::new())
}

fn apply_delta(runtime: &mut Runtime, delta: &ffi::Delta) -> ffi::ApplyResult {
    let result = delta_from_ffi(delta).and_then(|d| runtime.apply(&d).map_err(|e| e.to_string()));
    match result {
        Ok(version) => ffi::ApplyResult {
            ok: true,
            version: version.0,
            error: String::new(),
        },
        Err(error) => ffi::ApplyResult {
            ok: false,
            version: runtime.graph().version.0,
            error,
        },
    }
}

fn describe_node(runtime: &Runtime, tree: &str, node: i32) -> ffi::Answer {
    let at = NodeRef::new(TreeId(tree.to_string()), NodeId(node));
    match runtime.describe(&at) {
        Some(answer) => ffi::Answer {
            found: true,
            version: answer.version.0,
            role: answer.role.as_str().to_string(),
            has_name: answer.name.value.is_some(),
            name: answer.name.value.unwrap_or_default(),
            name_certainty: match answer.name.certainty {
                Certainty::Known => ffi::Certainty::Known,
                Certainty::Inferred => ffi::Certainty::Inferred,
                Certainty::Uncertain => ffi::Certainty::Uncertain,
            },
        },
        None => ffi::Answer {
            found: false,
            version: runtime.graph().version.0,
            role: String::new(),
            has_name: false,
            name: String::new(),
            name_certainty: ffi::Certainty::Uncertain,
        },
    }
}

fn plan_action(runtime: &Runtime, request: &ffi::ActionRequest) -> ffi::ActionPlan {
    let mut out = ffi::ActionPlan {
        ok: false,
        rejection: ffi::Rejection::Invalid,
        tree: request.tree.clone(),
        node: request.node,
        action: request.action,
        has_value: request.has_value,
        value: request.value.clone(),
        version: runtime.graph().version.0,
    };
    let Some(action) = action_from_ffi(request.action) else {
        return out;
    };
    let wish = ActionRequest {
        target: NodeRef::new(TreeId(request.tree.clone()), NodeId(request.node)),
        action,
        value: request.has_value.then(|| request.value.clone()),
        version: GraphVersion(request.version),
    };
    match runtime.plan(&wish) {
        Ok(_) => {
            out.ok = true;
            out.rejection = ffi::Rejection::None;
        }
        Err(rejection) => {
            out.rejection = match rejection {
                Rejection::Stale { .. } => ffi::Rejection::Stale,
                Rejection::UnknownNode => ffi::Rejection::UnknownNode,
                Rejection::Ignored => ffi::Rejection::Ignored,
                Rejection::Disabled => ffi::Rejection::Disabled,
                Rejection::Unsupported => ffi::Rejection::Unsupported,
                Rejection::ValueMismatch => ffi::Rejection::ValueMismatch,
            }
        }
    }
    out
}

fn find_node(runtime: &Runtime, name: &str, action: ffi::Action) -> ffi::Found {
    let version = runtime.graph().version.0;
    match action_from_ffi(action).and_then(|a| runtime.find(name, a)) {
        Some(at) => ffi::Found {
            found: true,
            tree: at.tree.0,
            node: at.node.0,
            version,
        },
        None => ffi::Found {
            found: false,
            tree: String::new(),
            node: 0,
            version,
        },
    }
}

fn node_count(runtime: &Runtime) -> u64 {
    runtime.graph().len() as u64
}

fn show_marks(runtime: &mut Runtime) -> Vec<ffi::MarkBox> {
    runtime
        .show_marks()
        .into_iter()
        .map(|m| ffi::MarkBox {
            uncertain: m.uncertain(),
            label: m.label,
            x: m.bounds.x,
            y: m.bounds.y,
            width: m.bounds.width,
            height: m.bounds.height,
        })
        .collect()
}

fn inspector_json(runtime: &Runtime) -> String {
    crate::inspector::inspector_json(runtime)
}

fn show_node(runtime: &mut Runtime, key: &str) -> ffi::Reply {
    reply_to_ffi(runtime.show(key))
}

fn run_command(runtime: &mut Runtime, input: &str) -> ffi::Reply {
    reply_to_ffi(runtime.command(input))
}

fn confirmation_target(runtime: &Runtime) -> ffi::Found {
    found(runtime, runtime.confirmation_target())
}

fn found(runtime: &Runtime, at: Option<NodeRef>) -> ffi::Found {
    match at {
        Some(at) => ffi::Found {
            found: true,
            tree: at.tree.0,
            node: at.node.0,
            version: runtime.graph.version.0,
        },
        None => ffi::Found {
            found: false,
            tree: String::new(),
            node: 0,
            version: runtime.graph.version.0,
        },
    }
}

fn facts_target(runtime: &Runtime, input: &str) -> ffi::Found {
    found(runtime, runtime.facts_target(input))
}

fn apply_form_facts(runtime: &mut Runtime, facts: &ffi::FormFacts) {
    let fields: Vec<(NodeId, String)> = facts
        .fields
        .iter()
        .map(|f| (NodeId(f.node), f.autocomplete.clone()))
        .collect();
    runtime.apply_form_facts(
        &NodeRef::new(TreeId(facts.tree.clone()), NodeId(facts.node)),
        facts.has_action.then(|| facts.action.clone()),
        &fields,
    );
}

fn reconfirm(runtime: &mut Runtime) -> ffi::Reply {
    reply_to_ffi(runtime.reconfirm())
}

fn view_act(runtime: &mut Runtime, key: &str, kind: &str, value: &str) -> ffi::Reply {
    reply_to_ffi(runtime.view_act(key, kind, value))
}

fn devtools_command(runtime: &Runtime, method: &str, params: &str) -> ffi::DevToolsReply {
    match crate::devtools::command(runtime, method, params) {
        Ok(value) => ffi::DevToolsReply {
            ok: true,
            json: value.to_string(),
        },
        Err(message) => ffi::DevToolsReply {
            ok: false,
            json: message,
        },
    }
}

fn redact_input(input: &str) -> String {
    relief_interaction::redact_input(input)
}

fn reply_to_ffi(reply: Reply) -> ffi::Reply {
    let mut out = ffi::Reply {
        kind: ffi::ReplyKind::Answer,
        text: String::new(),
        steps: Vec::new(),
        scroll: ffi::ScrollDirection::Down,
    };
    match reply {
        Reply::Answer(text) => out.text = text,
        Reply::Escape => out.kind = ffi::ReplyKind::Escape,
        Reply::Scroll(direction) => {
            out.kind = ffi::ReplyKind::Scroll;
            out.scroll = match direction {
                ScrollDirection::Down => ffi::ScrollDirection::Down,
                ScrollDirection::Up => ffi::ScrollDirection::Up,
                ScrollDirection::Top => ffi::ScrollDirection::Top,
                ScrollDirection::Bottom => ffi::ScrollDirection::Bottom,
            };
        }
        Reply::Perform(steps) => {
            out.kind = ffi::ReplyKind::Perform;
            out.steps = steps.into_iter().map(step_to_ffi).collect();
        }
    }
    out
}

fn step_to_ffi(step: Step) -> ffi::Step {
    match step {
        Step::Ax(s) => ffi::Step {
            key: ffi::Key::None,
            tree: s.target.tree.0,
            node: s.target.node.0,
            action: action_to_ffi(s.action),
            has_value: s.value.is_some(),
            value: s.value.unwrap_or_default(),
        },
        Step::Key(key) => ffi::Step {
            key: match key {
                Key::Escape => ffi::Key::Escape,
                Key::ArrowUp => ffi::Key::ArrowUp,
                Key::ArrowDown => ffi::Key::ArrowDown,
            },
            tree: String::new(),
            node: 0,
            action: ffi::Action::Focus,
            has_value: false,
            value: String::new(),
        },
    }
}

fn finish_command(runtime: &mut Runtime) -> String {
    runtime.finish()
}

fn take_security_log(runtime: &mut Runtime) -> Vec<String> {
    runtime
        .take_security_log()
        .iter()
        .map(|e| serde_json::to_string(e).expect("SecurityEvent ist serialisierbar"))
        .collect()
}

fn describe_page(runtime: &Runtime) -> String {
    runtime.describe_page()
}

fn scrolled_text(direction: ffi::ScrollDirection, before: f64, after: f64, max: f64) -> String {
    let direction = match direction {
        ffi::ScrollDirection::Down => ScrollDirection::Down,
        ffi::ScrollDirection::Up => ScrollDirection::Up,
        ffi::ScrollDirection::Top => ScrollDirection::Top,
        ffi::ScrollDirection::Bottom => ScrollDirection::Bottom,
        _ => return "Scroll: ungültige Richtung.".into(),
    };
    relief_interaction::respond::scrolled(direction, before, after, max)
}

/// `wait:` fällt weg: Der Fork-Host läuft ereignisgesteuert, die Aufgaben
/// mit Wartezeit (Frames anderer Prozesse im CDP-Host) laufen dort nicht.
fn parse_task_file(text: &str) -> Vec<ffi::Task> {
    relief_interaction::parse_tasks(text)
        .into_iter()
        .filter_map(|line| match line {
            TaskLine::Url(text) => Some(ffi::Task {
                kind: ffi::TaskKind::Url,
                text,
            }),
            TaskLine::Do(text) => Some(ffi::Task {
                kind: ffi::TaskKind::Do,
                text,
            }),
            TaskLine::Assert(text) => Some(ffi::Task {
                kind: ffi::TaskKind::Assert,
                text,
            }),
            TaskLine::Expect(text) => Some(ffi::Task {
                kind: ffi::TaskKind::Expect,
                text,
            }),
            TaskLine::Wait(_) => None,
        })
        .collect()
}

fn expectation_met(answer: &str, expected: &str) -> bool {
    relief_interaction::expectation_met(answer, expected)
}

// ---------------------------------------------------------------------------
// C++ → Modell

/// Delta aus der Grenze ins Modell. Fehler nur bei ungültigen Enum-Werten.
pub fn delta_from_ffi(delta: &ffi::Delta) -> Result<TreeDelta, String> {
    Ok(TreeDelta {
        base: GraphVersion(delta.base),
        root: delta.has_root.then(|| TreeId(delta.root.clone())),
        removed_trees: delta
            .removed_trees
            .iter()
            .map(|t| TreeId(t.clone()))
            .collect(),
        trees: delta
            .trees
            .iter()
            .map(update_from_ffi)
            .collect::<Result<_, _>>()?,
    })
}

fn update_from_ffi(u: &ffi::TreeUpdate) -> Result<TreeUpdate, String> {
    Ok(TreeUpdate {
        tree: TreeId(u.tree.clone()),
        data: u.has_data.then(|| tree_data_from_ffi(&u.data)),
        root: u.has_root.then_some(NodeId(u.root)),
        removed: ids(&u.removed),
        created: u
            .created
            .iter()
            .map(node_from_ffi)
            .collect::<Result<_, _>>()?,
        changed: u
            .changed
            .iter()
            .map(node_from_ffi)
            .collect::<Result<_, _>>()?,
        bounds: u
            .bounds
            .iter()
            .map(|b| BoundsChange {
                node: NodeId(b.node),
                bounds: b.has_bounds.then(|| rect_from_ffi(b.bounds)),
            })
            .collect(),
    })
}

fn tree_data_from_ffi(d: &ffi::TreeData) -> TreeData {
    TreeData {
        parent: d
            .has_parent
            .then(|| NodeRef::new(TreeId(d.parent_tree.clone()), NodeId(d.parent_node))),
        url: d.has_url.then(|| d.url.clone()),
        title: d.has_title.then(|| d.title.clone()),
        focus: d.has_focus.then_some(NodeId(d.focus)),
    }
}

fn node_from_ffi(n: &ffi::Node) -> Result<SemanticNode, String> {
    let bad = |what: &str| format!("Knoten {}: ungültiger Wert für {what}", n.id);
    Ok(SemanticNode {
        id: NodeId(n.id),
        role: Role::from_name(&n.role),
        name: Fact::known(text(n.has_name, &n.name)),
        name_from: match n.name_from {
            ffi::NameFrom::Unset => None,
            ffi::NameFrom::Attribute => Some(NameFrom::Attribute),
            ffi::NameFrom::RelatedElement => Some(NameFrom::RelatedElement),
            ffi::NameFrom::Contents => Some(NameFrom::Contents),
            ffi::NameFrom::Placeholder => Some(NameFrom::Placeholder),
            ffi::NameFrom::Title => Some(NameFrom::Title),
            _ => return Err(bad("name_from")),
        },
        description: Fact::known(text(n.has_description, &n.description)),
        value: Fact::known(text(n.has_value, &n.value)),
        states: States {
            focusable: n.focusable,
            disabled: n.disabled,
            readonly: n.readonly,
            required: n.required,
            modal: n.modal,
            multiline: n.multiline,
            multiselectable: n.multiselectable,
            settable: n.settable,
            busy: n.busy,
            expanded: opt_bool(n.expanded).ok_or_else(|| bad("expanded"))?,
            selected: opt_bool(n.selected).ok_or_else(|| bad("selected"))?,
            checked: toggle(n.checked).ok_or_else(|| bad("checked"))?,
            pressed: toggle(n.pressed).ok_or_else(|| bad("pressed"))?,
            invalid: match n.invalid {
                ffi::Invalid::Unset => None,
                ffi::Invalid::True => Some(Invalid::True),
                ffi::Invalid::Spelling => Some(Invalid::Spelling),
                ffi::Invalid::Grammar => Some(Invalid::Grammar),
                _ => return Err(bad("invalid")),
            },
            editable: match n.editable {
                ffi::Editable::Unset => None,
                ffi::Editable::Plaintext => Some(Editable::Plaintext),
                ffi::Editable::Richtext => Some(Editable::Richtext),
                _ => return Err(bad("editable")),
            },
            has_popup: match n.has_popup {
                ffi::HasPopup::Unset => None,
                ffi::HasPopup::True => Some(HasPopup::True),
                ffi::HasPopup::Menu => Some(HasPopup::Menu),
                ffi::HasPopup::Listbox => Some(HasPopup::Listbox),
                ffi::HasPopup::Tree => Some(HasPopup::Tree),
                ffi::HasPopup::Grid => Some(HasPopup::Grid),
                ffi::HasPopup::Dialog => Some(HasPopup::Dialog),
                _ => return Err(bad("has_popup")),
            },
            orientation: match n.orientation {
                ffi::Orientation::Unset => None,
                ffi::Orientation::Horizontal => Some(Orientation::Horizontal),
                ffi::Orientation::Vertical => Some(Orientation::Vertical),
                _ => return Err(bad("orientation")),
            },
        },
        actions: n
            .actions
            .iter()
            .map(|a| action_from_ffi(*a).ok_or_else(|| bad("actions")))
            .collect::<Result<_, _>>()?,
        bounds: n.has_bounds.then(|| rect_from_ffi(n.bounds)),
        ignored: n.ignored,
        ignored_reasons: Vec::new(),
        level: n.has_level.then_some(n.level),
        url: text(n.has_url, &n.url),
        parent: n.has_parent.then_some(NodeId(n.parent)),
        children: ids(&n.children),
        child_tree: n.has_child_tree.then(|| TreeId(n.child_tree.clone())),
        relations: Relations {
            labelled_by: ids(&n.labelled_by),
            described_by: ids(&n.described_by),
            controls: ids(&n.controls),
            details: ids(&n.details),
            error_message: ids(&n.error_message),
            flow_to: ids(&n.flow_to),
            active_descendant: n
                .has_active_descendant
                .then_some(NodeId(n.active_descendant)),
        },
        dom_node_id: n.has_dom_node_id.then_some(n.dom_node_id),
        extra: n
            .extra
            .iter()
            .map(|a| (a.key.clone(), a.value.clone()))
            .collect::<BTreeMap<_, _>>(),
    })
}

fn text(has: bool, value: &str) -> Option<String> {
    has.then(|| value.to_string())
}

fn ids(v: &[i32]) -> Vec<NodeId> {
    v.iter().map(|&id| NodeId(id)).collect()
}

fn rect_from_ffi(r: ffi::Rect) -> Rect {
    Rect {
        x: r.x,
        y: r.y,
        width: r.width,
        height: r.height,
    }
}

/// `None` = ungültiger Wert; `Some(None)` = nicht gesetzt.
fn opt_bool(v: ffi::OptBool) -> Option<Option<bool>> {
    match v {
        ffi::OptBool::Unset => Some(None),
        ffi::OptBool::False => Some(Some(false)),
        ffi::OptBool::True => Some(Some(true)),
        _ => None,
    }
}

fn toggle(v: ffi::Toggle) -> Option<Option<Toggle>> {
    match v {
        ffi::Toggle::Unset => Some(None),
        ffi::Toggle::False => Some(Some(Toggle::False)),
        ffi::Toggle::True => Some(Some(Toggle::True)),
        ffi::Toggle::Mixed => Some(Some(Toggle::Mixed)),
        _ => None,
    }
}

fn action_from_ffi(a: ffi::Action) -> Option<Action> {
    Some(match a {
        ffi::Action::DoDefault => Action::DoDefault,
        ffi::Action::Focus => Action::Focus,
        ffi::Action::Blur => Action::Blur,
        ffi::Action::SetValue => Action::SetValue,
        ffi::Action::Increment => Action::Increment,
        ffi::Action::Decrement => Action::Decrement,
        ffi::Action::Expand => Action::Expand,
        ffi::Action::Collapse => Action::Collapse,
        ffi::Action::ScrollToMakeVisible => Action::ScrollToMakeVisible,
        ffi::Action::ShowContextMenu => Action::ShowContextMenu,
        ffi::Action::SetSequentialFocusNavigationStartingPoint => {
            Action::SetSequentialFocusNavigationStartingPoint
        }
        _ => return None,
    })
}

// ---------------------------------------------------------------------------
// Modell → Grenze (Gegenrichtung für Tests und Messungen; im Fork füllt der
// C++-Adapter die Strukturen aus `AXTreeUpdate`)

/// Delta in die Grenzstrukturen. Verlustbehaftet wie im Modulkommentar.
pub fn delta_to_ffi(delta: &TreeDelta) -> ffi::Delta {
    ffi::Delta {
        base: delta.base.0,
        has_root: delta.root.is_some(),
        root: delta.root.as_ref().map(|t| t.0.clone()).unwrap_or_default(),
        removed_trees: delta.removed_trees.iter().map(|t| t.0.clone()).collect(),
        trees: delta.trees.iter().map(update_to_ffi).collect(),
    }
}

fn update_to_ffi(u: &TreeUpdate) -> ffi::TreeUpdate {
    ffi::TreeUpdate {
        tree: u.tree.0.clone(),
        has_data: u.data.is_some(),
        data: u.data.as_ref().map(tree_data_to_ffi).unwrap_or_default(),
        has_root: u.root.is_some(),
        root: u.root.map_or(0, |r| r.0),
        removed: raw_ids(&u.removed),
        created: u.created.iter().map(node_to_ffi).collect(),
        changed: u.changed.iter().map(node_to_ffi).collect(),
        bounds: u
            .bounds
            .iter()
            .map(|b| ffi::BoundsChange {
                node: b.node.0,
                has_bounds: b.bounds.is_some(),
                bounds: b.bounds.map(rect_to_ffi).unwrap_or_default(),
            })
            .collect(),
    }
}

fn tree_data_to_ffi(d: &TreeData) -> ffi::TreeData {
    ffi::TreeData {
        has_parent: d.parent.is_some(),
        parent_tree: d
            .parent
            .as_ref()
            .map(|p| p.tree.0.clone())
            .unwrap_or_default(),
        parent_node: d.parent.as_ref().map_or(0, |p| p.node.0),
        has_url: d.url.is_some(),
        url: d.url.clone().unwrap_or_default(),
        has_title: d.title.is_some(),
        title: d.title.clone().unwrap_or_default(),
        has_focus: d.focus.is_some(),
        focus: d.focus.map_or(0, |f| f.0),
    }
}

fn node_to_ffi(n: &SemanticNode) -> ffi::Node {
    let s = &n.states;
    ffi::Node {
        id: n.id.0,
        role: n.role.as_str().to_string(),
        has_name: n.name.value.is_some(),
        name: n.name.value.clone().unwrap_or_default(),
        name_from: match n.name_from {
            None => ffi::NameFrom::Unset,
            Some(NameFrom::Attribute) => ffi::NameFrom::Attribute,
            Some(NameFrom::RelatedElement) => ffi::NameFrom::RelatedElement,
            Some(NameFrom::Contents) => ffi::NameFrom::Contents,
            Some(NameFrom::Placeholder) => ffi::NameFrom::Placeholder,
            Some(NameFrom::Title) => ffi::NameFrom::Title,
        },
        has_description: n.description.value.is_some(),
        description: n.description.value.clone().unwrap_or_default(),
        has_value: n.value.value.is_some(),
        value: n.value.value.clone().unwrap_or_default(),
        focusable: s.focusable,
        disabled: s.disabled,
        readonly: s.readonly,
        required: s.required,
        modal: s.modal,
        multiline: s.multiline,
        multiselectable: s.multiselectable,
        settable: s.settable,
        busy: s.busy,
        expanded: opt_bool_to_ffi(s.expanded),
        selected: opt_bool_to_ffi(s.selected),
        checked: toggle_to_ffi(s.checked),
        pressed: toggle_to_ffi(s.pressed),
        invalid: match s.invalid {
            None => ffi::Invalid::Unset,
            Some(Invalid::True) => ffi::Invalid::True,
            Some(Invalid::Spelling) => ffi::Invalid::Spelling,
            Some(Invalid::Grammar) => ffi::Invalid::Grammar,
        },
        editable: match s.editable {
            None => ffi::Editable::Unset,
            Some(Editable::Plaintext) => ffi::Editable::Plaintext,
            Some(Editable::Richtext) => ffi::Editable::Richtext,
        },
        has_popup: match s.has_popup {
            None => ffi::HasPopup::Unset,
            Some(HasPopup::True) => ffi::HasPopup::True,
            Some(HasPopup::Menu) => ffi::HasPopup::Menu,
            Some(HasPopup::Listbox) => ffi::HasPopup::Listbox,
            Some(HasPopup::Tree) => ffi::HasPopup::Tree,
            Some(HasPopup::Grid) => ffi::HasPopup::Grid,
            Some(HasPopup::Dialog) => ffi::HasPopup::Dialog,
        },
        orientation: match s.orientation {
            None => ffi::Orientation::Unset,
            Some(Orientation::Horizontal) => ffi::Orientation::Horizontal,
            Some(Orientation::Vertical) => ffi::Orientation::Vertical,
        },
        actions: n.actions.iter().map(|a| action_to_ffi(*a)).collect(),
        has_bounds: n.bounds.is_some(),
        bounds: n.bounds.map(rect_to_ffi).unwrap_or_default(),
        ignored: n.ignored,
        has_level: n.level.is_some(),
        level: n.level.unwrap_or_default(),
        has_url: n.url.is_some(),
        url: n.url.clone().unwrap_or_default(),
        has_parent: n.parent.is_some(),
        parent: n.parent.map_or(0, |p| p.0),
        children: raw_ids(&n.children),
        has_child_tree: n.child_tree.is_some(),
        child_tree: n
            .child_tree
            .as_ref()
            .map(|t| t.0.clone())
            .unwrap_or_default(),
        labelled_by: raw_ids(&n.relations.labelled_by),
        described_by: raw_ids(&n.relations.described_by),
        controls: raw_ids(&n.relations.controls),
        details: raw_ids(&n.relations.details),
        error_message: raw_ids(&n.relations.error_message),
        flow_to: raw_ids(&n.relations.flow_to),
        has_active_descendant: n.relations.active_descendant.is_some(),
        active_descendant: n.relations.active_descendant.map_or(0, |a| a.0),
        has_dom_node_id: n.dom_node_id.is_some(),
        dom_node_id: n.dom_node_id.unwrap_or_default(),
        extra: n
            .extra
            .iter()
            .map(|(key, value)| ffi::Attribute {
                key: key.clone(),
                value: value.clone(),
            })
            .collect(),
    }
}

fn raw_ids(v: &[NodeId]) -> Vec<i32> {
    v.iter().map(|id| id.0).collect()
}

fn rect_to_ffi(r: Rect) -> ffi::Rect {
    ffi::Rect {
        x: r.x,
        y: r.y,
        width: r.width,
        height: r.height,
    }
}

fn opt_bool_to_ffi(v: Option<bool>) -> ffi::OptBool {
    match v {
        None => ffi::OptBool::Unset,
        Some(false) => ffi::OptBool::False,
        Some(true) => ffi::OptBool::True,
    }
}

fn toggle_to_ffi(v: Option<Toggle>) -> ffi::Toggle {
    match v {
        None => ffi::Toggle::Unset,
        Some(Toggle::False) => ffi::Toggle::False,
        Some(Toggle::True) => ffi::Toggle::True,
        Some(Toggle::Mixed) => ffi::Toggle::Mixed,
    }
}

fn action_to_ffi(a: Action) -> ffi::Action {
    match a {
        Action::DoDefault => ffi::Action::DoDefault,
        Action::Focus => ffi::Action::Focus,
        Action::Blur => ffi::Action::Blur,
        Action::SetValue => ffi::Action::SetValue,
        Action::Increment => ffi::Action::Increment,
        Action::Decrement => ffi::Action::Decrement,
        Action::Expand => ffi::Action::Expand,
        Action::Collapse => ffi::Action::Collapse,
        Action::ScrollToMakeVisible => ffi::Action::ScrollToMakeVisible,
        Action::ShowContextMenu => ffi::Action::ShowContextMenu,
        Action::SetSequentialFocusNavigationStartingPoint => {
            ffi::Action::SetSequentialFocusNavigationStartingPoint
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ungueltiger_enum_wert_wird_fehler_statt_panik() {
        let mut delta = ffi::Delta {
            has_root: true,
            root: "t".into(),
            ..Default::default()
        };
        let mut node = node_to_ffi(&SemanticNode::new(NodeId(1), Role::from_name("button")));
        node.checked = ffi::Toggle { repr: 42 };
        delta.trees.push(ffi::TreeUpdate {
            tree: "t".into(),
            has_root: true,
            root: 1,
            created: vec![node],
            ..Default::default()
        });
        let mut runtime = Runtime::new();
        let result = apply_delta(&mut runtime, &delta);
        assert!(!result.ok);
        assert!(result.error.contains("checked"), "{}", result.error);
        assert_eq!(result.version, 0);
    }

    #[test]
    fn aktion_ueber_die_grenze() {
        let mut node = SemanticNode::new(NodeId(1), Role::from_name("button"));
        node.actions = vec![Action::DoDefault];
        node.name = Fact::known(Some("Senden".into()));
        let delta = TreeDelta {
            root: Some(TreeId("t".into())),
            trees: vec![TreeUpdate {
                tree: TreeId("t".into()),
                data: Some(TreeData::default()),
                root: Some(NodeId(1)),
                removed: Vec::new(),
                created: vec![node],
                changed: Vec::new(),
                bounds: Vec::new(),
            }],
            ..Default::default()
        };
        let mut runtime = new_runtime();
        let applied = apply_delta(&mut runtime, &delta_to_ffi(&delta));
        assert!(applied.ok, "{}", applied.error);

        let answer = describe_node(&runtime, "t", 1);
        assert!(answer.found && answer.has_name);
        assert_eq!(answer.name, "Senden");
        assert_eq!(answer.name_certainty, ffi::Certainty::Known);

        let mut request = ffi::ActionRequest {
            tree: "t".into(),
            node: 1,
            action: ffi::Action::DoDefault,
            has_value: false,
            value: String::new(),
            version: applied.version,
        };
        let plan = plan_action(&runtime, &request);
        assert!(plan.ok);

        let found = find_node(&runtime, "Senden", ffi::Action::DoDefault);
        assert!(found.found);
        assert_eq!((found.tree.as_str(), found.node), ("t", 1));
        assert!(!find_node(&runtime, "Senden", ffi::Action { repr: 99 }).found);
        assert_eq!(node_count(&runtime), 1);
        request.action = ffi::Action { repr: 99 };
        assert_eq!(
            plan_action(&runtime, &request).rejection,
            ffi::Rejection::Invalid
        );
    }
}
