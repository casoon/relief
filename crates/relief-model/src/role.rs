//! Rolle eines Knotens als eigenes Enum.
//!
//! Nicht `ax::mojom::Role` und nicht der CDP-String: Chromium kennt mehrere
//! interne Rollen für eine ARIA-Rolle (z. B. `kComboBoxMenuButton` und
//! `kTextFieldWithComboBox` für `combobox`), CDP benennt interne Rollen in
//! einer dritten Schreibweise (`StaticText`). Relief benennt eine Rolle nach
//! ARIA, wo es eine ARIA-Rolle gibt, sonst nach Chromiums interner Rolle in
//! lowerCamelCase. Adapter bilden ihre Namen darauf ab; was keiner Variante
//! entspricht, bleibt als [`Role::Other`] mit dem Originalnamen erhalten.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

macro_rules! roles {
    ($($variant:ident => $name:literal, $cdp:literal;)*) => {
        /// Rolle eines Knotens.
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub enum Role {
            $(
                #[doc = concat!("`", $name, "`")]
                $variant,
            )*
            /// Rolle ohne eigene Variante, mit dem Namen des Adapters.
            Other(String),
        }

        impl Role {
            /// Relief-Name der Rolle (so wird sie serialisiert).
            pub fn as_str(&self) -> &str {
                match self {
                    $(Role::$variant => $name,)*
                    Role::Other(name) => name,
                }
            }

            /// Aus dem Relief-Namen; unbekannte Namen werden [`Role::Other`].
            pub fn from_name(name: &str) -> Self {
                match name {
                    $($name => Role::$variant,)*
                    other => Role::Other(other.to_string()),
                }
            }

            /// Aus dem Rollennamen, den CDP (`Accessibility.getFullAXTree`)
            /// meldet; unbekannte Namen werden [`Role::Other`].
            pub fn from_cdp(name: &str) -> Self {
                match name {
                    $($cdp => Role::$variant,)*
                    other => Role::Other(other.to_string()),
                }
            }
        }
    };
}

roles! {
    // ARIA-Rollen (CDP meldet sie unter ihrem ARIA-Namen)
    Alert => "alert", "alert";
    AlertDialog => "alertdialog", "alertdialog";
    Application => "application", "application";
    Article => "article", "article";
    Banner => "banner", "banner";
    Blockquote => "blockquote", "blockquote";
    Button => "button", "button";
    Caption => "caption", "caption";
    Cell => "cell", "cell";
    Checkbox => "checkbox", "checkbox";
    Code => "code", "code";
    ColumnHeader => "columnheader", "columnheader";
    Combobox => "combobox", "combobox";
    Complementary => "complementary", "complementary";
    ContentInfo => "contentinfo", "contentinfo";
    Definition => "definition", "definition";
    Deletion => "deletion", "deletion";
    Dialog => "dialog", "dialog";
    Document => "document", "document";
    Emphasis => "emphasis", "emphasis";
    Feed => "feed", "feed";
    Figure => "figure", "figure";
    Form => "form", "form";
    Generic => "generic", "generic";
    Grid => "grid", "grid";
    GridCell => "gridcell", "gridcell";
    Group => "group", "group";
    Heading => "heading", "heading";
    Image => "image", "image";
    Insertion => "insertion", "insertion";
    Link => "link", "link";
    List => "list", "list";
    Listbox => "listbox", "listbox";
    ListItem => "listitem", "listitem";
    Log => "log", "log";
    Main => "main", "main";
    Mark => "mark", "mark";
    Marquee => "marquee", "marquee";
    Math => "math", "math";
    Menu => "menu", "menu";
    MenuBar => "menubar", "menubar";
    MenuItem => "menuitem", "menuitem";
    MenuItemCheckbox => "menuitemcheckbox", "menuitemcheckbox";
    MenuItemRadio => "menuitemradio", "menuitemradio";
    Meter => "meter", "meter";
    Navigation => "navigation", "navigation";
    None => "none", "none";
    Note => "note", "note";
    Option => "option", "option";
    Paragraph => "paragraph", "paragraph";
    ProgressBar => "progressbar", "progressbar";
    Radio => "radio", "radio";
    RadioGroup => "radiogroup", "radiogroup";
    Region => "region", "region";
    Row => "row", "row";
    RowGroup => "rowgroup", "rowgroup";
    RowHeader => "rowheader", "rowheader";
    ScrollBar => "scrollbar", "scrollbar";
    Search => "search", "search";
    SearchBox => "searchbox", "searchbox";
    SectionFooter => "sectionfooter", "sectionfooter";
    SectionHeader => "sectionheader", "sectionheader";
    Separator => "separator", "separator";
    Slider => "slider", "slider";
    SpinButton => "spinbutton", "spinbutton";
    Status => "status", "status";
    Strong => "strong", "strong";
    Subscript => "subscript", "subscript";
    Superscript => "superscript", "superscript";
    Switch => "switch", "switch";
    Tab => "tab", "tab";
    Table => "table", "table";
    TabList => "tablist", "tablist";
    TabPanel => "tabpanel", "tabpanel";
    Term => "term", "term";
    Textbox => "textbox", "textbox";
    Time => "time", "time";
    Timer => "timer", "timer";
    Toolbar => "toolbar", "toolbar";
    Tooltip => "tooltip", "tooltip";
    Tree => "tree", "tree";
    TreeGrid => "treegrid", "treegrid";
    TreeItem => "treeitem", "treeitem";
    // Chromium-interne Rollen ohne ARIA-Entsprechung (CDP: Großschreibung)
    Abbr => "abbr", "Abbr";
    Audio => "audio", "Audio";
    Canvas => "canvas", "Canvas";
    DescriptionList => "descriptionList", "DescriptionList";
    Details => "details", "Details";
    DisclosureTriangle => "disclosureTriangle", "DisclosureTriangle";
    Figcaption => "figcaption", "Figcaption";
    Iframe => "iframe", "Iframe";
    InlineTextBox => "inlineTextBox", "InlineTextBox";
    LabelText => "labelText", "LabelText";
    LayoutTable => "layoutTable", "LayoutTable";
    LayoutTableCell => "layoutTableCell", "LayoutTableCell";
    LayoutTableRow => "layoutTableRow", "LayoutTableRow";
    Legend => "legend", "Legend";
    LineBreak => "lineBreak", "LineBreak";
    ListMarker => "listMarker", "ListMarker";
    MenuListOption => "menuListOption", "MenuListOption";
    MenuListPopup => "menuListPopup", "MenuListPopup";
    RootWebArea => "rootWebArea", "RootWebArea";
    StaticText => "staticText", "StaticText";
    Video => "video", "Video";
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for Role {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for Role {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let name = String::deserialize(deserializer)?;
        Ok(Role::from_name(&name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cdp_namen_werden_zu_relief_namen() {
        assert_eq!(Role::from_cdp("StaticText"), Role::StaticText);
        assert_eq!(Role::StaticText.as_str(), "staticText");
        assert_eq!(Role::from_cdp("button"), Role::Button);
        assert_eq!(
            Role::from_cdp("SomethingNew"),
            Role::Other("SomethingNew".into())
        );
    }

    #[test]
    fn serialisiert_als_name_und_zurueck() {
        for role in [Role::RootWebArea, Role::Combobox, Role::Other("x".into())] {
            let json = serde_json::to_string(&role).unwrap();
            assert_eq!(serde_json::from_str::<Role>(&json).unwrap(), role);
        }
        assert_eq!(serde_json::to_string(&Role::Iframe).unwrap(), "\"iframe\"");
    }
}
