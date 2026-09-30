# 01 · Chromium-Integration

Belegt gegen Chromium **154.0.8037.58** (Commit `a654841425914cbb`),
vollständiger, flacher Checkout ohne Historie. Pfade relativ zu `src/`,
Zeilennummern gelten für diesen Tag. Ab einem anderen Tag Zeilen neu
nachschlagen; Dateien und Klassennamen sind stabiler als Zeilen.

## Grundregel [Entscheidung]

Blink nicht modifizieren, solange Chromium die Information bereits über seine
Accessibility-Infrastruktur liefert. Relief dockt **im Browser-Prozess** an
den Accessibility-Datenstrom an, neben dem nativen Pfad zur
OS-Accessibility — nicht statt ihm.

## Vorhandene Pipeline [belegt]

```
DOM + CSS → Layout → Blink AXObject (AXObjectCacheImpl)
  → AXTreeSerializer → AXUpdatesAndEvents (AXTreeUpdate[] + AXEvent[])
  → Mojo blink.mojom.RenderAccessibilityHost.HandleAXEvents
     (Empfang auf dem ThreadPool, Weitergabe an den UI-Thread)
  → RenderFrameHostImpl::HandleAXEvents (UI-Thread, je Frame)
     ├─ 1. WebContentsObserver::AccessibilityEventReceived   ← braucht kWebContents
     └─ 2. BrowserAccessibilityManager::OnAccessibilityEvents ← braucht kNativeAPIs
            → AXTree::Unserialize → AXTreeObserver → OS-API (macOS AX / UIA / AT-SPI)
```

Belege:

- Serialisierung im Renderer: `ui/accessibility/ax_tree_serializer.h:75`,
  Blink-Seite `third_party/blink/renderer/modules/accessibility/`
  (`ax_object_cache_impl.cc`, `blink_ax_tree_source.h`).
- Mojo-Schnittstellen `RenderAccessibilityHost` (Renderer → Browser) und
  `RenderAccessibility` (Browser → Renderer, `SetMode`, `PerformAction`,
  `Reset`): `third_party/blink/public/mojom/render_accessibility.mojom:44`
  und `:85`. `HandleAXEvents` hat eine Antwort (`=> ()`); der Renderer
  schickt erst nach der Quittung das nächste Paket.
- Empfang auf dem ThreadPool, dann `PostTaskAndReply` auf den UI-Thread:
  `content/browser/accessibility/render_accessibility_host.cc:60–84`.
  Kommentar dort: Nachrichten können gegenüber Navigationen umsortiert
  werden, deshalb trägt jede Nachricht die Tree-ID
  (`render_accessibility_host.h`, Klassenkommentar).
- Reihenfolge im Browser: `content/browser/renderer_host/render_frame_host_impl.cc`
  `:12049` ruft `delegate_->ProcessAccessibilityUpdatesAndEvents` (→
  `WebContentsImpl::ProcessAccessibilityUpdatesAndEvents`,
  `content/browser/web_contents/web_contents_impl.cc:6377–6383`, benachrichtigt
  alle `WebContentsObserver`), **danach** `:12052`
  `SendAccessibilityEventsToManager`.
- `SendAccessibilityEventsToManager` verlangt `kNativeAPIs`
  (`render_frame_host_impl.cc:9251`, `CHECK`); der
  `BrowserAccessibilityManager` entsteht nur mit `kNativeAPIs`
  (`render_frame_host_impl.cc:14123–14145`).

Relief-Abzweig (umgesetzt in 17, siehe „Beobachtungspunkt“ und „Umsetzung im Fork“):

```
          RenderFrameHostImpl::HandleAXEvents (UI-Thread)
                       │
          ┌────────────┴─────────────────────┐
          ▼                                  ▼
WebContentsObserver (Relief)        BrowserAccessibilityManager
  → eigener ui::AXTree je AXTreeID    (nur wenn ein OS-Screenreader o. ä.
  → AXTreeObserver → Delta              kNativeAPIs setzt)
  → Bridge → Rust-Runtime                    │
          │                            VoiceOver etc.
     eigene Assistenz
```

## Relevante Klassen [belegt]

| Baustein | Ort (154.0.8037.58) | Rolle für Relief |
|---|---|---|
| `AXNodeData` | `ui/accessibility/ax_node_data.h:43` | Rohdaten eines Knotens: Rolle, Attribute, States, Actions, Relationen. Integer-ID, nur innerhalb eines Baums eindeutig |
| `AXTreeUpdate` | `ui/accessibility/ax_tree_update.h:51` | inkrementelle Änderung vom Renderer |
| `AXUpdatesAndEvents` | `ui/accessibility/ax_updates_and_events.h:19` | Paket aus mehreren `AXTreeUpdate` + Events + `ax_tree_id`; das bekommen Beobachter |
| `AXTree` | `ui/accessibility/ax_tree.h:102` (`AddObserver` `:146`, `Unserialize` `:178`) | lebender Baum; von außen befüllbar mit `Unserialize` |
| `AXTreeObserver` | `ui/accessibility/ax_tree_observer.h:33` | Callbacks je Knoten und je atomarem Update |
| `AXTreeManager` | `ui/accessibility/ax_tree_manager.h:31` (besitzt den Baum: `std::unique_ptr<AXTree> ax_tree_` `:199`; `FromID` `:33`, `ForChildTree` `:36`) | globale Zuordnung Tree-ID → Manager, Child-Tree-Auflösung |
| `AXTreeID` | `ui/accessibility/ax_tree_id.h:27` (`FromToken` `:45`) | je Frame und Dokument ein Baum; Identität = (TreeID, NodeID) |
| `BrowserAccessibilityManager` | `ui/accessibility/platform/browser_accessibility_manager.h:117` (`FromID` `:129`, Aktionen `:233–281`, `OnAccessibilityEvents` `:289`) | hält den Baum pro Frame für die OS-Anbindung; ist selbst `AXTreeObserver` (`:430–440`) |
| `BrowserAccessibility` | `ui/accessibility/platform/browser_accessibility.h:43` | plattformnahes Knotenobjekt |
| `AXActionData` | `ui/accessibility/ax_action_data.h:24` | **Rückweg**: Aktion + Ziel (`target_tree_id` `:39`, `target_node_id` `:48`, `value` `:87`) |
| `ax::mojom::Action` | `ui/accessibility/ax_enums.mojom:406–527` | Aktionsliste, siehe Rückweg |
| `AXActionHandlerRegistry` | `ui/accessibility/ax_action_handler_registry.h:48` (`GetActionHandler` `:66`, Observer `TreeRemoved` `:36`) | Tree-ID → Aktionsziel (`RenderFrameHostImpl`) |
| `AXMode` | `ui/accessibility/ax_mode.h:26` (Flags `:30–114`, Bündel `:238–279`) | welche Teile des Baums gebaut werden |
| `BrowserAccessibilityState` | `content/public/browser/browser_accessibility_state.h:27` (`CreateScopedModeForWebContents` `:59`) | Modus anfordern, öffentlich |
| `ScopedAccessibilityMode` | `content/public/browser/scoped_accessibility_mode.h` | Modus gilt, solange das Objekt lebt |
| `WebContentsObserver::AccessibilityEventReceived` | `content/public/browser/web_contents_observer.h:862` | öffentlicher Einstieg in den Datenstrom aller Frames eines `WebContents` |
| `RenderFrameHost` | `content/public/browser/render_frame_host.h:161` (`FromAXTreeID` `:185`, `GetAXTreeID` `:257`, `AccessibilityPerformAction` `:653`) | Frame ↔ Tree-ID, Aktion senden |
| `WebContents` (AX-Teil) | `content/public/browser/web_contents.h` (`RequestAXTreeSnapshot` `:656`, `GetAccessibilityMode` `:740`, `ResetAccessibility` `:748`, `GetAccessibilityRootNode` `:752`) | Snapshot, Reset, Zugriff auf Wurzel |
| TTS | `content/public/browser/tts_controller.h:119`; macOS `content/browser/speech/tts_mac.mm` (AVSpeechSynthesizer) | native Sprachausgabe |
| Side Panel | `chrome/browser/ui/views/side_panel/`, Eintrags-IDs `chrome/browser/ui/side_panel/side_panel_entry_id.h:19` | Ort für Inspector-Panel und Befehlsleiste |

Korrektur zur vorigen Fassung: `BrowserAccessibilityManager` liegt in
`ui/accessibility/platform/`, nicht in `content/` [belegt]. Der
`BrowserAccessibilityManager` ist **nicht** immer vorhanden (siehe unten).

## 1 · `BrowserAccessibilityManager` pro Frame [belegt]

- Ein Manager je `RenderFrameHostImpl`, Besitz als
  `std::unique_ptr<ui::BrowserAccessibilityManager> browser_accessibility_manager_`
  (`content/browser/renderer_host/render_frame_host_impl.h:4913–4914`).
- Erzeugt nur, wenn der `AXMode` des `WebContents` `kNativeAPIs` enthält
  (`render_frame_host_impl.cc:14123–14132`); gelöscht, sobald der Modus
  wegfällt (`render_frame_host_impl.cc:13994–14004`).
- Wurzel-Manager eines Tabs: `WebContentsImpl::GetRootBrowserAccessibilityManager`
  (`web_contents_impl.cc:6596`) — nur content-intern.
- Öffentliche Wege ohne `content/browser`-Include:
  `ui::BrowserAccessibilityManager::FromID(rfh->GetAXTreeID())`
  (`browser_accessibility_manager.h:129`) oder
  `WebContents::GetAccessibilityRootNode()` (`web_contents.h:752`, liefert
  `nullptr` ohne Manager; Implementierung `web_contents_impl.cc:6403–6410`).
- GN: `//ui/accessibility/platform` hat eine Sichtbarkeitsliste
  (`ui/accessibility/platform/BUILD.gn:33–38`), wird aber über die Gruppe
  `//ui/accessibility` öffentlich weitergereicht (`ui/accessibility/BUILD.gn:157–161`).
  Kein BUILD-Patch nötig.

## 2 · Beobachtungspunkt: `AXTreeObserver` [belegt, Empfehlung: Annahme]

**Callbacks** (`ui/accessibility/ax_tree_observer.h`): vor der Änderung
`OnNodeDataWillChange`, `OnIgnoredWillChange`, `OnNodeWillBeDeleted`,
`OnSubtreeWillBeDeleted`, `OnNodeWillBeReparented`, `OnAtomicUpdateStarting`
(`:191`); nach der Änderung `OnTreeDataChanged` (`:108`), `OnNodeCreated`
(`:132`), `OnNodeDeleted` (`:138`), `OnNodeReparented`, `OnNodeChanged`
(`:147`), attributgenaue `On*AttributeChanged` (`:64–105`),
`OnChildTreeConnectionChanged` (`:151`), `OnTreeManagerWillBeRemoved` (`:160`)
und zum Schluss `OnAtomicUpdateFinished(tree, root_changed, changes)`
(`:201`) mit `Change{node, NODE_CREATED | SUBTREE_CREATED | NODE_CHANGED |
NODE_REPARENTED | SUBTREE_REPARENTED}` (`:165–180`).

**Reihenfolge in einem `Unserialize`** (`ui/accessibility/ax_tree.cc`):
`OnAtomicUpdateStarting` (`:1334`) → Mutation → `OnTreeDataChanged` (`:1547`)
→ angelegte/umgehängte Knoten (`:1555–1560`) → gelöschte Knoten
(`:1564–1567`) → geänderte Knoten mit Attribut-Callbacks und `OnNodeChanged`
(`:1571–1595`) → `OnAtomicUpdateFinished` (`:1597`). Jedes `AXTreeUpdate` im
Paket ist ein eigenes atomares Update
(`browser_accessibility_manager.cc:530`, Schleife über die Updates).
Kommentar in `ax_tree_observer.h:38–43`: Knotenzeiger in den „Will“-Callbacks
nicht über den Aufruf hinaus halten.

**Threading:** alles auf dem Browser-UI-Thread (Weitergabe
`render_accessibility_host.cc:76–83`). Weil der Renderer auf die Quittung
wartet, bremst ein langsamer Beobachter die AX-Updates des Renderers
[belegt über `=> ()` im Mojom; Auswirkung Annahme]. Schwere Arbeit gehört
nicht in den Callback.

**Reihenfolge-Garantien über Frames hinweg:** keine gefunden. Je Frame ist die
Reihenfolge durch die Quittung seriell; zwischen Frames nicht [Annahme].
Veraltete Pakete nach Navigation werden über Tree-ID und `reset_token`
verworfen (`render_frame_host_impl.cc:11954–11979`).

**Zwei Wege, einen Beobachter anzuhängen:**

| | Weg A: am `BrowserAccessibilityManager` | Weg B: eigener Baum aus dem Datenstrom |
|---|---|---|
| Einstieg | `BrowserAccessibilityManager::FromID(tree_id)->ax_tree()->AddObserver()` | `WebContentsObserver::AccessibilityEventReceived` (`web_contents_observer.h:862`) → eigener `ui::AXTree` je `AXTreeID`, `Unserialize`, eigener `AXTreeObserver` |
| nötiger Modus | `kNativeAPIs` + `kWebContents` (mind. `kAXModeBasic`, `ax_mode.h:238`) | `kWebContents` genügt (`kAXModeWebContentsOnly`, `ax_mode.h:243`) |
| Nebenwirkung | schaltet die OS-Accessibility-Brücke ein (Plattformobjekte, Events an VoiceOver/UIA) auch ohne Screenreader | keine OS-Brücke; läuft ein Screenreader, existiert der Baum zweimal (Speicher) |
| Lebenszyklus | Manager entsteht erst beim ersten Paket (`render_frame_host_impl.cc:12014`), verschwindet bei Moduswechsel; ob `FromID` beim ersten Paket schon greift, ist offen (`ax_tree_manager.h:22–30`: Bäume mit unbekannter ID stehen nicht in der Map) | Relief besitzt die Bäume selbst; Entfernen über `AXActionHandlerObserver::TreeRemoved` (`ax_action_handler_registry.h:36`) |
| Vorbilder in Chromium | Plattform-Manager selbst | Reading Mode (`read_anything_untrusted_page_handler.cc:176–177, 290–293, 304`), `chrome.automation` (`automation_internal_api.cc:345–372`) |
| Eingriff außerhalb `//relief/` | nur Einhängen je Tab | nur Einhängen je Tab |

**Entscheidung: Weg B** [Entscheidung, umgesetzt und gemessen in 17]. Er
braucht nur öffentliche `content/public`- und `ui/accessibility`-API, zwingt
die OS-Brücke nicht an und ist der Weg, den Chromium selbst für zwei
Nicht-Screenreader-Verbraucher nutzt. Weg A wurde nicht gebaut: B trägt
(0 Fehler auf spiegel.de und Wikipedia, → 09), und A würde für jeden
Relief-Tab `kNativeAPIs` erzwingen. Kosten von B: ein eigenes `Unserialize`
im UI-Thread (gemessen p95 < 0,4 ms je Paket, Vollbaum ~8 000 Knoten
15–23 ms), bei laufendem Screenreader zusätzlich zum
`BrowserAccessibilityManager`. Umsetzung unten, „Umsetzung im Fork“.

Hinweis aus beiden Vorbildern: Ist `kWebContents` schon aktiv, muss ein neuer
Beobachter `WebContents::ResetAccessibility()` auslösen, um einen vollständigen
Baum zu bekommen (`read_anything_untrusted_page_handler.cc:283–297`,
`automation_internal_api.cc:360–377`). Jeder Reset serialisiert die Seite neu.

**Positionen** kommen getrennt: `AccessibilityLocationChangesReceived`
(`web_contents_observer.h:864`) mit `AXLocationAndScrollUpdates`
(`ax_location_and_scroll_updates.h:48`: neue `relative_bounds` je Knoten und
Scroll-Positionen); Bounds-Änderungen stehen nicht zwingend im
`AXTreeUpdate`. Das Delta-Format führt sie deshalb getrennt
(`BoundsChange`, → 03). Der Browser meldet sie vor den Baum-Updates
desselben Pakets (`render_frame_host_impl.cc:12016–12024` vor `:12049`),
nur für Knoten, die es schon gibt; neue Knoten tragen ihre Position im
`AXNodeData`. `relative_bounds` sind relativ zum Offset-Container und in
Blink-Pixeln (CSS-Pixel × Geräte-Skalierungsfaktor); in Viewport-Koordinaten
rechnet `AXTree::RelativeToTreeBounds` (`ax_tree.h:191`, zieht die
Scroll-Position der Container ab), in Seitenkoordinaten zusätzlich die
Scroll-Position des Root-Scrollers zurück wie
`BrowserAccessibility::RelativeToAbsoluteBounds` (`browser_accessibility.cc:870–887`).
Umsetzung unten, „Umsetzung im Fork“.

## 3 · `AXMode` dauerhaft aktivieren [belegt, Kosten: Annahme]

- API: `content::BrowserAccessibilityState::GetInstance()->CreateScopedModeForWebContents(web_contents, mode)`
  (`browser_accessibility_state.h:54–61`); gilt, solange der zurückgegebene
  `ScopedAccessibilityMode` lebt. Varianten für Prozess und `BrowserContext`.
  Ohne Scoper: Schalter `--force-renderer-accessibility[=basic|form-controls|complete|screen-reader|on-screen]`
  (`browser_accessibility_state_impl.cc:341–373`), der weitere Moduswechsel
  sperrt — nur für Tests brauchbar.
- Flags (`ax_mode.h:39–108`): `kNativeAPIs`, `kWebContents`,
  `kInlineTextBoxes`, `kExtendedProperties`, `kHTML`, `kHTMLMetadata`,
  `kLabelImages`, `kPDFPrinting`, `kAnnotateMainNode`, `kFromPlatform`,
  `kScreenReader`, `kNativeAdaptedWebContents`; Filter
  `kFormsAndLabelsOnly`, `kOnScreenOnly`.
- **Relief-Vorschlag [Annahme]:** `kAXModeWebContentsOnly`
  (`kWebContents | kInlineTextBoxes | kExtendedProperties`), wie
  `chrome.automation`. Reading Mode nimmt zusätzlich `kHTML` und nennt das
  selbst „heavy-handed“ (`read_anything_untrusted_page_handler.cc:168–177`).
- **Unterschied zum CDP-Spike [belegt]:** Ohne `kScreenReader` beschneidet
  Blink display-gesperrte Inhalte (`content-visibility`), siehe
  `ax_object_cache_impl.cc:626–631`. CDP serialisiert mit `kAXModeInspector`,
  der `kScreenReader` enthält (`ax_mode.h:256–259`,
  `inspector_accessibility_agent.cc:457`). Der Fork sieht mit
  `kAXModeWebContentsOnly` also weniger als der Spike. **Entscheidung
  [17]:** Relief setzt `kScreenReader` nicht; der Schalter
  `--relief-screen-reader-mode` fügt ihn für Vergleiche hinzu. Gemessen
  ändert er die Knotenzahl auf Wikipedia (7 950 = 7 950) und spiegel.de
  (8 312 zu 8 311) kaum (→ 09); `kScreenReader` ist ein Modus für
  Screenreader, Relief soll Blink nicht mehr Arbeit abverlangen als nötig.
  Das Modell (→ 03) trägt, was ankommt.
- **Kosten:** Chromium misst `Accessibility.Performance.HandleAXEvents2`
  (`render_frame_host_impl.cc:11936`); Relief-Zahlen in 09 („Messung im
  Fork“).
- **Takt der Updates [belegt]:** Blink serialisiert nicht-interaktive
  Änderungen höchstens alle 150 ms nach dem Laden (350 ms davor),
  Aktionen und Fokusänderungen sofort
  (`ax_object_cache_impl.cc:3507–3520, 3593–3621, 5428–5460`). Das begrenzt
  die Ende-zu-Ende-Latenz auf unruhigen Seiten (spiegel.de p95 ~180 ms),
  unabhängig von Relief (→ 09).
  Versteckte Tabs: `kProgressiveAccessibilityPhase2` würde den Modus fünf
  Minuten nach dem Verstecken entziehen (`browser_accessibility_state_impl.cc:688–726`),
  ist aber standardmäßig aus (`content/common/features.cc:613–614`). Beim
  Anlegen versteckter Tabs setzt Chromium keinen Anfangsmodus
  (`browser_accessibility_state_impl.cc:653–656`); ein Scoper je
  `WebContents` greift beim Aufdecken (`:662–685`).
- **Beobachtbarkeit durch Seiten:** Kein Web-IDL in
  `third_party/blink/renderer` liest `AXMode` oder `AXObjectCache` (Suche
  über `*.idl` ohne Treffer) [belegt]. Indirekt denkbar: Laufzeit
  (zusätzliche Layout-/AX-Arbeit, `RenderAccessibilityImpl::PerformAction`
  erzwingt `UpdateAXForAllDocuments`, `render_accessibility_impl.cc:327–328`)
  und Nebenwirkungen von Aktionen [Annahme, nicht geprüft] → offen für
  Paket 24 und 07.

## 4 · Rückweg `AXActionData` [belegt]

- Aufruf: `RenderFrameHost::AccessibilityPerformAction(const ui::AXActionData&)`
  (`render_frame_host.h:653`) oder, ohne Frame-Zeiger,
  `ui::AXActionHandlerRegistry::GetInstance()->GetActionHandler(tree_id)->PerformAction(data)`
  (`ax_action_handler_registry.h:66`; `RenderFrameHostImpl` ist der Handler,
  `render_frame_host_impl.cc:4658–4660`). So macht es Reading Mode
  (`read_anything_untrusted_page_handler.cc:1284–1293`).
- `RenderFrameHostImpl::AccessibilityPerformAction`
  (`render_frame_host_impl.cc:4084–4116`): verwirft bei inaktivem Frame, ohne
  `kWebContents` und bei `ShouldIgnoreInputEvents`; `kHitTest` gesondert;
  sonst `render_accessibility_->PerformAction` über das Mojo-Interface
  `RenderAccessibility.PerformAction` (`render_accessibility.mojom:119`,
  **ohne Antwort**).
- Renderer: `RenderAccessibilityImpl::PerformAction`
  (`content/renderer/accessibility/render_accessibility_impl.cc:323`) aktualisiert
  Layout und AX, plant sofortige Serialisierung, führt aus; Blink-Seite
  `AXObject::PerformAction` (`ax_object.cc:7806`).
- Aktionen (`ax_enums.mojom:406–527`): `kDoDefault` (`:431`, Blink:
  `RequestClickAction`), `kFocus` (`:436`), `kBlur`, `kSetValue` (`:510`),
  `kIncrement`, `kDecrement`, `kExpand`, `kCollapse`, `kScrollToMakeVisible`
  (`:487`), `kScrollToPoint`, `kScrollUp/Down/Left/Right/Forward/Backward`,
  `kSetScrollOffset`, `kSetSelection`, `kReplaceSelectedText`,
  `kReplaceRanges`, `kSetSequentialFocusNavigationStartingPoint`,
  `kShowContextMenu`, `kShowTooltip`, `kHideTooltip`,
  `kSetAccessibilityFocus`, `kClearAccessibilityFocus`, `kHitTest`,
  `kLoadInlineTextBoxes`, `kGetImageData`, `kGetTextLocation`,
  `kCustomAction`, `kStitchChildTree`, Medien-Aktionen, `kLongClick`,
  `kScrollToPositionAtRowColumn`, `kRequestLayoutBasedAction`,
  `kAnnotatePageImages`, `kInternalInvalidateTree`, `kSignalEndOfTest`.
  Welche ein Knoten anbietet, steht in seinen `AXNodeData`-Actions
  (`ax_object.cc:1405–1414` für Scroll-Aktionen).
- Threading: Aufruf auf dem UI-Thread (`RenderFrameHostImpl`), Zustellung
  asynchron über den assoziierten Kanal des Frames
  (`render_accessibility.mojom:82–84`). Ergebnis nur über das nächste
  `AXTreeUpdate` sichtbar — bestätigt die Regel aus 02, Erfolg am Diff zu
  messen.

## 5 · iframes/OOPIFs als Child-Trees [belegt]

- **Jeder Frame** (nicht nur OOPIFs) hat einen eigenen Baum mit eigener
  Tree-ID; Node-IDs sind nur je Baum eindeutig
  (`docs/accessibility/overview.md:441–456`).
- `AXTreeID` eines Frames = sein Embedding-Token
  (`render_frame_host_impl.cc:19978–19988`). Neues Dokument im selben Frame
  → neues Token → **neue Tree-ID**; Relief-Identität (TreeID, NodeID) ist
  also pro Dokument, nicht pro Frame stabil.
- Verknüpfung: Der iframe-Knoten im Elternbaum trägt
  `StringAttribute::kChildTreeId` (`ax_enums.mojom:622`), gesetzt in
  `AXObject::SerializeChildTreeID` (`ax_object.cc:1417–1470`); versteckte
  iframes werden nicht verknüpft (`:1438–1441`). Der Kindbaum kennt
  `parent_tree_id` aus `AXTreeData`, gesetzt vom Browser
  (`render_frame_host_impl.cc:14683–14687`).
- Auflösung: `AXTreeManager::ForChildTree(node)` (`ax_tree_manager.h:36`),
  `GetParentNodeFromParentTree` (`:118`). In Weg B muss Relief diese Zuordnung
  selbst halten; Reading Mode tut das mit einer Map
  Tree-ID → `AXTreeManager` (`read_anything_app_model.h:93–101`).
- Browserseitiges Einhängen fremder Bäume: `kStitchChildTree`
  (`ax_enums.mojom:522`).

## 6 · Reading Mode und `chrome.automation` [belegt]

**Reading Mode („Read Anything“)** — Browser:
`chrome/browser/ui/webui/side_panel/read_anything/`, Steuerung
`chrome/browser/ui/read_anything/`, Renderer:
`chrome/renderer/accessibility/read_anything/`.

- Bezug des Baums: `ReadAnythingWebContentsObserver` (ein
  `WebContentsObserver`) fordert `kAXModeWebContentsOnly | kHTML` per
  `CreateScopedModeForWebContents` an und reicht jedes
  `AccessibilityEventReceived` per Mojo an die WebUI weiter
  (`read_anything_untrusted_page_handler.cc:270–307, 540–544`). Die WebUI
  (chrome-untrusted, eigener Renderer) baut die Bäume nach
  (`read_anything_app_model.h:93–101`, `read_anything_app_controller.h:83`).
- Aktionen zurück über `AXActionHandlerRegistry` (siehe 4).
- Hauptinhalt kommt aus ScreenAI (`kMainContentExtraction`,
  `read_anything_untrusted_page_handler.cc:408–412`) und DOM Distiller.
- **Für Relief:** Vorbild für Weg B und für ein WebUI-Panel im Side Panel
  (Eintrag `kReadAnything`, `side_panel_entry_id.h:25`; WebUI-Registrierung
  `chrome_untrusted_web_ui_configs.cc:65`). Den Code selbst nicht mitbenutzen:
  er ist an Reading-Mode-UI und ScreenAI gebunden.

**`chrome.automation`** — `extensions/browser/api/automation_internal/automation_internal_api.cc`,
Renderer `extensions/renderer/api/automation/`.

- Gleicher Bezug: `AutomationWebContentsObserver` mit
  `kAXModeWebContentsOnly` und Reset (`:345–377`), Aktionen über die Registry
  (`:574–633`).
- Freigabe: nur `privileged_extension` (`extensions/common/api/_api_features.json:99–104`)
  und Manifest-Allowlist mit vier IDs, Kommentar „should not be exposed
  outside of first-party allow-listed very trusted extensions and component
  extensions“ (`extensions/common/api/_manifest_features.json:48–61`).
- **Für Relief:** nicht als Hauptweg. Eine eigene Component-Extension bräuchte
  einen Eintrag in der Allowlist (Eingriff außerhalb `//relief/`) und brächte
  den Baum in einen JS-Kontext. Brauchbar höchstens als Debug-Werkzeug.

## Vorarbeiten innerhalb von Chromium

- **Reading Mode**, **`chrome.automation`**: siehe 6 [belegt].
- **ScreenAI-Dienst** [belegt]: Bibliothek wird auf Linux, Mac und Windows
  „on-demand through the component updater“ geliefert
  (`chrome/browser/screen_ai/README.md:20`); im Quellcode liegen nur Wrapper
  und Fake (`services/screen_ai/screen_ai_library_wrapper_*.cc`). **Nicht als
  verfügbaren Baustein einplanen**; Nutzbarkeit in einem Fork ohne
  Google-Komponenten bleibt unbelegt.
- **Bildbeschreibungen** [belegt, Ort]: `chrome/browser/accessibility/accessibility_labels_service.h`;
  Flag `kLabelImages` (`ax_mode.h:83`). Vorbild für Consent bei Cloud-KI (→ 07).

## 8 · `docs/accessibility/` in Chromium [belegt]

- `docs/accessibility/overview.md`: Architektur (Mehrprozess, Cache im
  Browser, inkrementelle Updates, ein Baum je Frame, Aktionen über
  `AXPlatformTreeManagerDelegate` → `RenderFrameHostImpl` →
  `RenderAccessibility.PerformAction`). **Aktuell** in den Kernaussagen und
  in den Pfaden zu `ui/accessibility/platform/`. **Veraltet:**
  `AXLayoutObject` (`overview.md:464, 550`) gibt es nicht mehr (nur noch
  `ax_layout_object_test.cc`); das Mojo-Interface heißt
  `blink.mojom.RenderAccessibilityHost`, nicht `ax.mojom…` (`:490`);
  `NotifyAccessibilityEvent` (`:504`) steht nicht mehr in
  `browser_accessibility_manager.h`; der Abschnitt „content layer“ verschweigt,
  dass der Manager nur mit `kNativeAPIs` existiert und dass
  `WebContentsObserver` den Strom vorher bekommen.
- `docs/accessibility/browser/how_a11y_works{,_2,_3}.md`: Einführung in
  Konzepte (Cache, Push statt Pull, `AXEventGenerator`, Aktionen, Hit-Tests,
  iframes); konzeptionell, wenig Pfade, daher kaum veraltet.
- `docs/accessibility/browser/perf.md`: Telemetry-Stories und
  `blink_perf.accessibility`; Vorlage für Relief-Messungen in 17.
- Zu Reading Mode, `AccessibilityEventReceived` oder
  `ScopedAccessibilityMode` steht in `docs/accessibility/` nichts; die
  Belege oben stammen aus dem Code.

## Nötige Eingriffe außerhalb `//relief/` [belegt: Orte · Annahme: Umfang]

Grundlage für 17 (Umsetzung) und den Rebase-Aufwand (Abschnitt
„Fork-Strategie“). Weg B vorausgesetzt.

| # | Datei (154.0.8037.58) | Eingriff | Warum nicht in `//relief/` |
|---|---|---|---|
| 1 | `chrome/browser/ui/tabs/tab_features.cc:197` (`TabFeatures::Init`) — **umgesetzt [17] ohne Header-Patch** | ein Include (`relief/relief_attach.h`) und eine Zeile `tab_subscriptions_.push_back(relief::AttachToTab(tab));` direkt nach `webui::InitEmbeddingContext` | Tab-Features werden nur hier erzeugt; `TabHelpers::AttachTabHelpers` ist für Desktop ausdrücklich nicht mehr zu verwenden (`chrome/browser/ui/tab_helpers.cc:308–311`) |
| 2 | `chrome/browser/ui/tabs/BUILD.gn` (Target `impl` mit `tab_features.cc`, `:501`) — **umgesetzt [17]** | `"//relief",` nach `"//net",` in `deps` | GN-Abhängigkeit muss beim Nutzer stehen |
| 3 | `chrome/browser/DEPS` (ein `chrome/browser/ui/tabs/DEPS` gibt es nicht [belegt]) | `"+relief"` in `include_rules` | checkdeps (Presubmit) verbietet sonst das Include; für den Build nicht nötig → **entfällt**, solange Relief den Chromium-Presubmit nicht fährt [Entscheidung] |
| 4 | `chrome/browser/ui/side_panel/side_panel_entry_id.h` und `chrome/browser/ui/actions/chrome_action_id.h` — **umgesetzt [20], ein Patch** | `V(kRelief, kActionSidePanelShowRelief, "Relief")` nach `kTestTabScopedEntry`; `E(kActionSidePanelShowRelief)` nach `kActionSidePanelShowReadAnything` | Side-Panel-IDs und Aktions-IDs sind zentrale Makro-Enums. Eine Aktions-ID ist Pflicht [belegt]: Kopfzeile (`SidePanelHelper::GetActionItem`) und Toolbar-Zustand (`SidePanelToolbarPinningController::UpdateActiveState`) prüfen sie per `CHECK`; `std::nullopt` geht nur für Sonderfälle (`kWebView`, `kExtension`). Das Aktions-Element selbst meldet `//relief` zur Laufzeit an `BrowserActions` an |
| 5 | ~~`chrome/browser/ui/webui/chrome_web_ui_configs.cc`~~ — **entfällt [20]** | — | die WebUI registriert `//relief` zur Laufzeit über `content::WebUIConfigMap::AddWebUIConfig`; Ressourcen ohne grit (Header aus `inspector/embed_resources.py`), also auch kein Eintrag in `tools/gritsettings/resource_ids.spec`; `WebUIContentsWrapperT` wird umgangen, weil es den WebUI-Namen gegen eine Histogramm-Liste prüft (`tools/metrics`) |
| 6 | `chrome/app/theme/chromium/BRANDING`, `chrome/app/chromium_strings.grd` (`IDS_PRODUCT_NAME`, `IDS_SHORT_PRODUCT_NAME`, nicht übersetzt), `chrome/app/app-Info.plist` — **umgesetzt [36], ein Patch** | Produktname „Relief“, Bundle-ID `de.casoon.relief`, `CrProductDirName` = `Relief` | Chromiums vorgesehener Weg für Produktnamen; `.app`-Name, Helfer, Framework und Profilverzeichnis leiten sich daraus ab |
| 7 | `chrome/renderer/chrome_content_renderer_client.cc` (`RenderFrameCreated`), `chrome/renderer/BUILD.gn` — **umgesetzt [75], ein Patch** | Include und `relief::FormFactsAgent::Create(render_frame);`; `"//relief/renderer"` in `deps` | Renderer-Beobachter je Frame entstehen nur hier; der Agent beantwortet `relief.mojom.FormFacts` (Formularziel, `autocomplete`), → spezifikation/07 |
| 8 | `chrome/browser/devtools/chrome_devtools_manager_delegate.cc` (`HandleCommand`, `ClientDetached`), `chrome/browser/devtools/BUILD.gn` — **umgesetzt [45], ein Patch** | `relief::HandleDevToolsCommand` vor der Chrome-Sitzung, `relief::OnDevToolsClientDetached`; `"//relief"` in `deps` | Eigene CDP-Methoden nimmt nur der DevTools-Delegate des Embedders an; Domäne `Relief.*` → spezifikation/12 |

Stand: sechs Patches (`fork/patches/series`).

Nicht nötig [belegt]:

- Kein Patch an `content/`, `ui/accessibility/`, Blink: Beobachter, Modus
  und Aktionen gehen über `content/public` und `ui/accessibility`.
- Kein GN-Sichtbarkeits-Patch (siehe 1).
- Rust-Abhängigkeiten `serde`, `serde_json`, `cxx` liegen schon in
  `//third_party/rust` (`third_party/rust/chromium_crates_io/Cargo.toml:25, 46–47`).
  `a11y-perception` hängt nur von `serde` ab und kann unter `//relief/`
  gebaut werden (→ 02); ein Eintrag in `//third_party/rust` wäre ein
  zusätzlicher Eingriff und ist vermeidbar [Annahme].

Nur bei Weg A zusätzlich: nichts an Dateien, aber `kNativeAPIs` für alle
Relief-Tabs (Verhaltenseingriff statt Code-Eingriff).

Nur mit `chrome.automation`: Allowlist in
`extensions/common/api/_manifest_features.json:56–61` — vermeiden.

## Präzisierungen gegenüber den Ausgangsannahmen

1. **„Observer am `BrowserAccessibilityManager`/Browser-Prozess-Baum“**
   (`docs/decisions.md` „Nativer Accessibility-Pfad bleibt
   erhalten“: „Relief zweigt neben `BrowserAccessibilityManager` ab“). Der
   Manager existiert nur mit `kNativeAPIs`, also wenn die OS-Brücke aktiv
   ist. Ein Observer direkt an seinem Baum zwingt diese Brücke an. Der von
   Chromium selbst genutzte Weg für Nicht-Screenreader zweigt **vor** dem
   Manager ab und baut einen eigenen Baum (Weg B). „Neben dem Manager“
   stimmt damit, „am Browser-Prozess-Baum“ nur, wenn Relief den Baum
   selbst hält. Entschieden in 17: Weg B, Relief hält den Baum selbst.
2. **„Chromium baut den AXTree nur, wenn Accessibility aktiv ist“** stimmt,
   ist aber zu grob: Es gibt zwei Stufen (`kWebContents` für den
   Renderer-Baum, `kNativeAPIs` für den Browser-Manager).
3. **CDP-Spike als Referenz:** CDP serialisiert mit `kScreenReader`, der Fork
   mit `kAXModeWebContentsOnly` nicht. Knotenmengen können abweichen
   (`content-visibility`). Messungen aus dem Spike sind nicht 1:1
   übertragbar.
4. **Plattform-Einschränkung Windows** (vorige Fassung: „Cross-Build vom Mac
   ist nicht unterstützt“): falsch. `docs/win_cross.md:7–21` beschreibt
   Cross-Builds von Linux/Mac; nicht Google-intern muss das Windows-SDK
   einmal auf einem Windows-Rechner paketiert werden (`:60–80`). Laufen und
   Testen braucht weiterhin Windows.
5. **Plattform-Einschränkung Linux** (vorige Fassung: „ARM-VM auf dem M4“):
   `docs/linux/build_instructions.md:15` verlangt eine x86-64-Maschine. Eine
   ARM-VM ist kein unterstützter Build-Host → x86-64-Runner einplanen
   (Paket 31).

## Folgen eines Forks [Annahme]

| Punkt | Auswirkung |
|---|---|
| Google-API-Keys | Ein Fork hat keine Google-Keys. Die Spracherkennung der Web Speech API in Chrome läuft serverseitig bei Google → **fällt weg**. STT muss OS-nativ, lokal oder per eigenem API-Key laufen (→ 08). |
| ScreenAI | Kommt über den Component Updater [belegt, s. o.] → im Fork voraussichtlich nicht verfügbar; Reading Mode verliert damit seine Hauptinhalts-Erkennung. |
| Widevine/DRM | Nicht in Chromium-Builds. Streaming-Seiten funktionieren nicht. Im Forschungsprojekt irrelevant. |
| Code-Signing/Notarisierung | Erst bei Verteilung an Dritte nötig (macOS Developer-ID, Windows Authenticode). Im Forschungsprojekt zurückgestellt. |
| Updates | Sicherheitsupdates von Chromium müssen zeitnah nachgezogen werden, sonst ist der Browser für Alltagsnutzung nicht verantwortbar. Im Forschungsprojekt: nur für Tests/Demos nutzen, nicht als Alltagsbrowser. |
| Build-Ressourcen | Checkout ~100 GB+ (`docs/linux/build_instructions.md:17` nennt mindestens 100 GB), Erstbuild auf M4 mehrere Stunden; inkrementell deutlich schneller. |
| Lizenz | Chromium ist BSD-3-Clause (plus Drittlizenzen in `third_party/`); Lizenzhinweise müssen erhalten bleiben. Eigener Code in `//relief/` und den Rust-Crates frei wählbar → siehe `docs/decisions.md`. |

## Plattformen [Entscheidung: Ziel · Annahme: Reihenfolge]

Ziel sind **macOS, Windows und Linux**. Chromium unterstützt alle drei
offiziell. Plattformneutral sind: Rust-Runtime, AXTree-Adapter
(`ui/accessibility` ist plattformübergreifend), UI über WebUI/Views.
Plattformspezifisch sind nur: STT/TTS-Adapter, Screenreader-Koexistenz,
Schlüsselspeicher für API-Keys (→ 07, 08).

Reihenfolge: **macOS zuerst** (M4 vorhanden), **Linux** danach, **Windows**
zuletzt, aber nicht optional — dort sind die meisten Screenreader-Nutzer
(NVDA, JAWS).

Build-Hosts [belegt, siehe Widersprüche 4 und 5]: Windows lässt sich vom Mac
cross-bauen (SDK einmal auf Windows paketieren), getestet wird auf Windows.
Linux braucht einen x86-64-Host (CI-Runner oder Maschine), keine ARM-VM auf
dem M4. → Paket 31.

Regel: Kein Relief-Code außerhalb der Plattform-Adapter darf
`#if BUILDFLAG(IS_MAC)` o. ä. enthalten.

## Fork-Strategie

Stand 2026-09-24. Format und Befehle: `fork/README.md`; Skripte
`scripts/fork-apply.sh`, `scripts/fork-export.sh`.

### Optionen [belegt: Quellen]

| Option | Vorbild und Beleg | Mechanik | Für Relief |
|---|---|---|---|
| **A · Branch auf Upstream-Tag** | Chromiums eigene Release-Branches (`refs/branch-heads/<build>`); jeder Fork mit eigenem Chromium-Repo | Relief-Commits auf einem Tag, Rebase mit `git rebase --onto <neu> <alt>` | Rebase-Ergonomie gut, aber das Artefakt ist ein Chromium-Repo: schon die blobarme Historie von vier Monaten sind ~800 MB (Messung unten). Passt nicht in dieses Repo, PR-Review sieht keine Patches |
| **B · Patch-Serie** | ungoogled-chromium: `patches/series` (111 Einträge, Quilt-Format), angewendet von `utils/patches.py` mit GNU `patch -p1`, gepflegt mit `quilt push -a --refresh` (`docs/developing.md`) — [github.com/ungoogled-software/ungoogled-chromium](https://github.com/ungoogled-software/ungoogled-chromium). Electron: `patches/chromium/.patches` (150 Einträge, `git format-patch`-Dateien), `script/git-import-patches` (`git am`, mit `-3` für 3-Wege) und `script/git-export-patches`; jede Nachricht muss den Grund nennen (`docs/development/patches.md`) — [github.com/electron/electron](https://github.com/electron/electron) | Patches im eigenen Repo, beim Anwenden entsteht lokal ein Branch | klein, reviewbar, ein Patch je Eingriff |
| **C · Override-Verzeichnis** | Brave `chromium_src/`: gleichnamige Datei unter `brave/chromium_src/` gewinnt über Include-Pfad-Vorrang, Eingriffe per `#define`-Tricks (`docs/patching_and_chromium_src.md` in [github.com/brave/brave-core](https://github.com/brave/brave-core)). Der Vorrang selbst ist ein Patch (`patches/build-config-BUILDCONFIG.gn.patch` hängt `//brave/build:compiler` in `default_compiler_configs`). Daneben 1158 Patches in `patches/`, eine Datei je geänderter Chromium-Datei, `pnpm run update_patches` / `apply_patches` | Überschreiben statt Ändern, Patches nur, wo Overrides nicht reichen | lohnt bei Hunderten Eingriffen. Brave patcht trotz Overrides genau die Dateien, die Relief braucht (`chrome-browser-ui-tabs-public-tab_features.h.patch`, `…side_panel_entry_id.h.patch`, `…chrome_action_id.h.patch`) |

### Entscheidung [Entscheidung]

**B in der Electron-Form:** `git format-patch`-Dateien plus
Reihenfolgedatei `series` in `fork/patches/`, angewendet mit `git am --3way`
als Commits auf einen lokalen Branch `relief` (damit bekommt man lokal die
Rebase-Ergonomie von A), eigener Code als Verzeichnis `fork/relief/`, das nach
`src/relief/` kopiert wird. **Kein** Override-Verzeichnis: Es bräuchte selbst
einen Build-Patch und Präprozessor-Tricks für zwei bis vier Eingriffe, die
Brave trotzdem als Patch führt.

### Messung: Änderungshäufigkeit [belegt]

Stable-Tags je Milestone (erstes Stable, Referenz 154.0.8037.58):
149.0.7827.29 · 150.0.7871.24 · 151.0.7922.34 · 152.0.7977.42 ·
153.0.8010.12 · 154.0.8037.58.
Methode: flacher, blobloser Abruf der Tags (siehe „Quellcode lesen“),
`git log --oneline <a>..<b> -- <pfad>`; Zeilen und Änderungsstellen aus
Gitiles-Inhalten.

Commits je Datei zwischen aufeinanderfolgenden Milestones:

| Datei | 149→150 | 150→151 | 151→152 | 152→153 | 153→154 |
|---|---|---|---|---|---|
| Commits im Intervall gesamt | 16 123 | 15 831 | 15 863 | 12 413 | 9 694 |
| `chrome/browser/ui/tabs/public/tab_features.h` | 7 | 2 | 4 | 15 | 11 |
| `chrome/browser/ui/tabs/tab_features.cc` | 12 | 3 | 11 | 19 | 14 |
| `chrome/browser/ui/tabs/BUILD.gn` | 7 | 12 | 7 | 15 | 19 |
| `chrome/browser/DEPS` | 3 | 5 | 8 | 8 | 10 |
| `chrome/browser/ui/side_panel/side_panel_entry_id.h` | 3 | 0 | 0 | 2 | 0 |
| `chrome/browser/ui/actions/chrome_action_id.h` | 2 | 12 | 30 | 8 | 7 |
| `chrome/browser/ui/webui/chrome_web_ui_configs.cc` | 8 | 6 | 6 | 8 | 5 |
| `chrome/browser/ui/webui/chrome_untrusted_web_ui_configs.cc` | 0 | 0 | 0 | 1 | 3 |

Geänderte Zeilen (+/−) in denselben Intervallen: `tab_features.h`
+18/−16 · +12/−0 · +25/−1 · +75/−15 · +48/−16 (730 Zeilen);
`tab_features.cc` +50/−22 · +85/−29 · +106/−111 · +69/−37 · +164/−28
(952 Zeilen). Die Dateien der Eingriffe 1–2 ändern sich also in **jedem**
Milestone, 17–49 Commits zusammen.

Öffentliche API, die `//relief/` nutzt (kein Patch, aber Kompilierrisiko),
Commits je Intervall: `content/public/browser/web_contents.h` 3 · 0 · 5 · 9 ·
9; `render_frame_host.h` 2 · 3 · 1 · 5 · 5; `web_contents_observer.h`
1 · 1 · 2 · 1 · 0; `ax_tree.h` 0 · 2 · 1 · 1 · 0; `ax_enums.mojom`
2 · 3 · 0 · 0 · 0; `ax_node_data.h`, `ax_action_data.h`, `ax_mode.h` je
höchstens 1. **Unverändert in allen fünf Intervallen:** `ax_tree_observer.h`,
`ax_tree_update.h`, `ax_updates_and_events.h`,
`ax_action_handler_registry.h`, `browser_accessibility_state.h`,
`scoped_accessibility_mode.h` — der Kern von Weg B ist stabil.

Sicherheitsreleases innerhalb eines Milestones (erstes → letztes Stable,
z. B. 149.0.7827.29 → .199): Patch-Dateien in 5 von 6 Milestones
unverändert; in 151 (→ .176) änderten Rückportierungen `tab_features.h/.cc`
und `BUILD.gn`. Je Milestone gab es 12–25 Stable-Versionen über Mac, Windows
und Linux ([chromiumdash](https://chromiumdash.appspot.com/fetch_releases?channel=Stable&platform=Mac&num=100)).

### Messung: Rebase simuliert [belegt: Simulation · Annahme: übertragbar auf echte Patches]

Die Eingriffe 1–5 als Zeileneinfügungen nachgebaut (Header-Member und
Vorwärtsdeklaration, Include und Erzeugung in `Init`, `deps`-Eintrag,
`"+relief"`, Side-Panel-Eintrag, Action-ID, WebUI-Registrierung), auf Tag
*a* angewendet und gegen Tag *b* geprüft: passt der unveränderte Kontext
(`git apply`), reicht ein 3-Wege-Merge (`git merge-file`, entspricht
`git am --3way`) oder bleibt ein Konflikt.

| Szenario | Anker naiv (7 Eingriffe) | Anker nach Regeln (5 Eingriffe) |
|---|---|---|
| je Milestone, 5 Schritte 149→154 | 26 passen, 2 × 3-Wege, **7 Konflikte** | 23 passen, 2 × 3-Wege, **0 Konflikte** |
| 149 → 154 direkt (4 Milestones übersprungen) | 2 passen, 1 × 3-Wege, **4 Konflikte** | 3 passen, 2 × 3-Wege, **0 Konflikte** |
| Sicherheitsreleases, 6 Milestones | alle passen | alle passen |

„Naiv“ heißt: am Ende wachsender Listen (`// Must be the last member.` in
`tab_features.h`, Ende der Side-Panel-Liste, Ende der WebUI-Registrierung).
Dort fügt Upstream selbst ständig an; jede angrenzende Einfügung ist für Git
ein Konflikt. Die 7 Konflikte lagen in `tab_features.h` (3),
`chrome_action_id.h` (2), `side_panel_entry_id.h` (1),
`chrome_untrusted_web_ui_configs.cc` (1). Die rechte Spalte hat zwei
Eingriffe weniger (DEPS und Action-ID entfallen nach Regel 3); die übrigen
fünf Konflikte in `tab_features.h`, `side_panel_entry_id.h` und der
WebUI-Registrierung verschwinden allein durch die Wahl des Ankers.

### Ankerregeln für Patches [Entscheidung]

1. Neue Zeilen **neben Zeilen, die Upstream selten anfasst**: direkt nach dem
   eigenen Header-Include bzw. dem letzten Include des Blocks, nach einem
   alten, festen Member (`side_panel_registry_`), am **Anfang** einer
   Funktion (nach `initialized_ = true;` in `TabFeatures::Init`), in der
   **Mitte** einer Liste neben einem alten Eintrag (`kReadAnything`) — nie am
   Ende einer wachsenden Liste. Brave macht es beim Side-Panel genauso
   (Einfügung hinter `kAssistant`).
2. So wenige Zeilen wie möglich, möglichst eine je Stelle; Logik gehört nach
   `//relief/`.
3. Eingriff vermeiden, wo ein vorgesehener Weg existiert: kein DEPS-Patch
   (checkdeps läuft nicht im Build), Side-Panel-Eintrag mit `std::nullopt`
   statt neuer Action-ID.
4. Ein Patch je Eingriff, Nachricht mit Grund und Nummer aus der
   Eingriffsliste oben.

### Rebase-Aufwand je Release [Annahme, aus den Messungen]

- **Patches:** mit Ankerregeln je Milestone 0 Handkonflikte, bis zu zwei
  automatische 3-Wege-Merges → unter 30 Minuten. Ohne Regeln im Mittel 1,4
  Handkonflikte je Milestone, jeder trivial (Einfügung neu platzieren).
- **`//relief/` gegen öffentliche API:** 9–17 Commits je Milestone in den
  genutzten Headern, Kern-Header unverändert → Kompilierfehler selten,
  `//relief/` nutzt davon heute `web_contents.h`, `web_contents_observer.h`,
  `render_frame_host.h`, `browser_accessibility_state.h`, `ax_tree.h`,
  `ax_tree_observer.h`, `ax_node_data.h`, `ax_action_data.h`,
  `ax_action_handler_registry.h` (Stand 17); Umfang je Rebase erst beim
  ersten Rebase messbar.
- **Dominant ist der Build:** 9 700–16 100 Upstream-Commits je Intervall
  heißen praktisch Vollbuild nach `gclient sync`. Dauer auf dem M4 aus
  Paket 14 (offen).
- **Sicherheitsreleases:** Patches passen unverändert, Aufwand = Sync +
  Build.
- Ein 3-Wege-Merge braucht die Dateien der alten Basis; im flachen Checkout
  fehlen sie und müssen einmal geholt werden (`fork/README.md`).

### Takt [Entscheidung]

Upstream-Takt [belegt, chromiumdash]: erste Stable-Versionen 149 20.05.,
150 17.06., 151 15.07., 152 12.08., 153 26.08., 154 09.09., 155 23.09. —
bis 152 alle vier Wochen, seitdem alle **zwei** Wochen.

- **Bis zum Go/No-Go (19) bleibt die Basis 154.0.8037.58.** Messungen in 17
  sollen nicht gegen ein wanderndes Chromium laufen.
- **Danach Rebase alle vier Wochen** auf das neueste Stable-Tag, also beim
  jetzigen Takt jeder zweite Milestone. Grund: Die Patch-Arbeit wächst beim
  Überspringen kaum (149→154 direkt: 0 Konflikte), der Vollbuild kostet je
  Rebase gleich viel.
- **Sicherheitsreleases** desselben Milestones nicht routinemäßig, sondern
  vor Arbeit mit fremden Websites (Messreihen, Demos): dann auf das neueste
  Stable des aktuellen Milestones gehen. Relief ist kein Alltagsbrowser
  (`docs/constraints.md`).
- Nach jedem Rebase laufen die Integrationstests je Integrationspunkt
  (Beobachter registriert, Snapshot kommt an, Aktion kommt an; → 10):
  `autoninja -C out/Relief relief_browsertests && out/Relief/relief_browsertests`
  (Fälle unten, „Umsetzung im Fork“).

### Offen

- ~~Eingriff 1 ohne Header-Patch?~~ Erledigt in 17: Der Helfer ist
  `WebContentsUserData`, `TabFeatures` hält nur die
  `base::CallbackListSubscription` aus `relief::AttachToTab` in
  `tab_subscriptions_` (Muster `webui::InitEmbeddingContext`,
  `chrome/browser/ui/webui/webui_embedding_context.cc:247–259`). Nach einem
  Discard hängt die Subscription (`RegisterWillDiscardContents`) den Helfer
  an die neuen `WebContents`; der alte stirbt mit den alten. Discard in 33
  erprobt (beide Wege, → „Umsetzung im Fork“).
- Dauer von `gclient sync` + Vollbuild je Rebase: Erstbuild auf dem M4 2 h 11 min, inkrementell < 20 s (→ 09, Chromium-Build); ein Rebase-Build liegt dazwischen, gemessen erst beim ersten Rebase.
- Die Simulation nutzt nachgebaute Einfügungen; mit den echten Patches aus 17
  einmal nachmessen (`scripts/fork-apply.sh --force` auf ein neueres Tag).

## Umsetzung im Fork (Pakete 17, 24, 33) [belegt]

Code in `fork/relief/` (→ `src/relief/`), Patches in `fork/patches/`,
gebaut und gemessen gegen 154.0.8037.58; Messwerte in 09.

```
TabFeatures::Init ──(Patch 1)──> relief::AttachToTab(tab)       nur mit --enable-relief
  → ReliefTabHelper (WebContentsObserver + WebContentsUserData, UI-Thread)
      ScopedAccessibilityMode(kAXModeWebContentsOnly [+ kScreenReader])
      AccessibilityEventReceived(AXUpdatesAndEvents)
        → neuer Baum nur für aktive Frames; ohne Wurzel nur mit Anfang (CheckStart)
        → AXTreeMirror je AXTreeID: ui::AXTree::Unserialize + AXTreeObserver
          (OnNodeDataWillChange, OnNodeWillBeDeleted, OnTreeDataChanged,
           OnAtomicUpdateFinished)
        → TakeUpdate: angelegt/geändert/entfernt + Positionen → relief::bridge::TreeUpdate
      AccessibilityLocationChangesReceived → SetLocation/SetScrollInfo im eigenen Baum
        → TakeUpdate: BoundsChange für verschobene Teilbäume
      TreeRemoved (AXActionHandlerObserver), PrimaryPageChanged → removed_trees
      DidFinishNavigation aus dem Back-Forward-Cache → Reset (begrenzt)
  ──SequenceBound──> RuntimeHost (eigene ThreadPool-Sequenz, MayBlock)
        apply_delta (Rust) · Protokoll · bei --relief-activate: find_node + plan_action
  ──BindPostTask──> ReliefTabHelper::Perform(ActionPlan)
        Knoten im eigenen Baum noch da? → AXActionHandlerRegistry::GetActionHandler(tree)
        → PerformAction(AXActionData{kDoDefault, tree, node})

--relief-run (Paket 24): ReliefTaskRunner im ersten Tab
  url: LoadURL → Ruhe + Hauptbaum der neuen Seite → describe_page
  do:  RunCommand ──> RuntimeHost::RunCommand → run_command (Rust, Session)
       ← Reply{Answer | Perform(Schritte) | Escape | Scroll}
       Schritte: PerformStep → AXActionData bzw. Taste (ForwardKeyboardEvent)
       Ruhe (kein AX-Paket 300 ms) → FinishCommand → Antwort
  expect: Teilstring der Antwort; Ende: Zusammenfassung, Prozess endet
```

- **Dateien:** `relief_attach.h` (einziger Header, den Chromium-Code
  einbindet), `relief_tab_helper.{h,cc}`, `relief_switches.h`,
  `bridge/ax_tree_mirror.{h,cc}` (eigener Baum, `AXNodeData` → Grenze,
  Rollennamen nach ARIA, Positionen), `bridge/runtime_host.{h,cc}`
  (Runtime-Sequenz, Protokoll, Befehle), `relief_task_runner.{h,cc}`
  (`--relief-run`), `BUILD.gn` (drei `rust_static_library`:
  `relief-model`, `relief-interaction`, `relief-bridge`; ein `source_set`,
  Gruppe `relief_tests`), `testing/` (Browser-Tests, unten). Welche Aktion
  auf welchem AX-Weg läuft und die Ersatzwege: 05, „Im Fork über
  `AXActionData`“.
- **Schalter:** `--enable-relief`, `--relief-log=<datei>` (Tab-getrennte
  Zeilen `packet`/`location`/`drop`/`tree`/`page`/`reset`/`skip`/`probe`/
  `activate`/`diff`/`error`, sonst `LOG(INFO)`),
  `--relief-activate=<Name>` (einmal je Tab: erster Knoten in
  Dokumentreihenfolge mit genau diesem Namen, der DoDefault meldet),
  `--relief-run=<aufgaben,…>` (Aufgabendateien abarbeiten, Ausgabe auf
  stdout, danach endet der Prozess; `scripts/fork-run-tasks.sh`),
  `--relief-screen-reader-mode`, `--relief-log-nodes=<Rolle,…>` (nur
  lesend: Knoten dieser Rollen mit Baum, ID, Namen und Position als Zeilen
  `node`, spätere Verschiebungen als `bounds`). Messläufe zusätzlich mit
  Chromiums `--use-mock-keychain`, sonst wartet die erste Navigation eines
  frischen Profils auf den Schlüsselbund (09, „Nachtrag Paket 35“).
- **Delta aus dem eigenen Baum:** Geänderte und angelegte Knoten kommen aus
  `OnAtomicUpdateFinished`, gelöschte aus `OnNodeWillBeDeleted`. Erst am
  Ende eines Pakets wird aus dem Baum konvertiert: ein Knoten, den die
  Runtime schon hat, geht als „geändert“, sonst als „angelegt“; gelöscht und
  im selben Paket neu angelegt (Umhängen, Reset) zählt als geändert. Ein
  Paket (ein Frame) ist eine Delta. Hauptbaum = Tree-ID von
  `GetPrimaryMainFrame()`; iframe-Eltern über `kChildTreeId` am Host-Knoten.
- **Positionen [33, 35]:** Seitenkoordinaten des Hauptdokuments in
  CSS-Pixeln: `AXTree::RelativeToTreeBounds` ungeclippt, geteilt durch
  Blink-Pixel je CSS-Pixel = Geräte-Skalierungsfaktor der
  `RenderWidgetHostView` × Browser-Zoom
  (`blink::ZoomLevelToZoomFactor(HostZoomMap::GetZoomLevel)`); im
  Hauptdokument plus Scroll-Position des Root-Scrollers
  (`AXTreeData::root_scroller_id`); leere Fläche = keine Position. Angelegte und geänderte Knoten tragen sie
  im Knoten. Ändert sich die Position oder Scroll-Position eines Knotens
  (Location-Kanal oder `OnNodeDataWillChange`), wird sein Teilbaum neu
  gerechnet — Nachfahren, deren Offset-Container er ist, melden keine eigene
  Änderung —, und nur Knoten mit neuer Position gehen als `BoundsChange`
  hinüber. Scrollt der Root-Scroller, bleiben Seitenkoordinaten gleich und
  es wird nichts gerechnet.
- **iframes [35]:** Ein iframe-Baum (Frame mit Eltern-Frame) rechnet in
  seinem Viewport — `RelativeToTreeBounds` zieht die Scroll-Position seines
  Root-Scrollers schon ab — und verschiebt um die Seitenkoordinaten seines
  Host-Knotens im Elternbaum (`hosts_`). Wie
  `BrowserAccessibility::RelativeToAbsoluteBounds` setzt das am Host-Knoten
  an; ein 5-px-Rahmen des iframes ist dabei schon enthalten (Browser-Test).
  Umsetzung im Adapter, nicht in der Runtime: Scroll-Positionen und
  Offset-Container hat nur der eigene `ui::AXTree`, die Runtime bekommt
  fertige Positionen, und die Rust-Crates bleiben unverändert. Nach jedem
  Paket eines Baums rechnet der Helfer die Offsets seiner iframe-Bäume neu
  (je iframe ein Knoten); ändert sich einer, geht der ganze iframe-Baum als
  `BoundsChange` in dieselbe Delta, rekursiv für verschachtelte iframes.
  Scrollt der Root-Scroller eines iframes, wird sein Teilbaum neu
  gerechnet (im Hauptdokument nicht). Solange der Host-Knoten fehlt (iframe
  vor dem Elternbaum), hat der iframe-Baum keine Positionen; sie kommen als
  `BoundsChange`, sobald der Host da ist.
- **Zoom-Wechsel [35, Annahme]:** Pakete zwischen Zoom-Wechsel und neuem
  Layout in Blink rechnen kurz mit neuem Faktor auf alten Blink-Pixeln; die
  Positionspakete nach dem Layout korrigieren das.
- **Lebenszyklus der Bäume [33]:** Die Runtime hält nur Bäume von Frames der
  angezeigten Seite. Wegfall über `TreeRemoved` (Frame gelöscht oder neues
  Dokument im selben Frame, `render_frame_host_impl.cc:19967–19991` über
  `AXActionHandlerBase::SetAXTreeID`) und über `PrimaryPageChanged`: Seiten
  im Back-Forward-Cache behalten ihren Frame und melden kein `TreeRemoved`,
  deshalb verwirft Relief beim Seitenwechsel jeden Baum, der nicht zu einem
  Frame der neuen Seite gehört, und räumt dabei `hosts_` (Einträge des
  Baums und seiner iframes) auf. Pakete inaktiver Frames
  (`RenderFrameHost::IsActive`, Back-Forward-Cache) legen keinen Baum an.
  Kommt eine Seite aus dem Cache zurück, löst `DidFinishNavigation` einen
  Reset aus, weil sie keinen neuen Baum schickt.
- **Kein Paket ohne Anfang [33]:** Ein `ui::AXTree` ohne Wurzel verträgt nur
  ein Update, das Wurzel und Knoten mitbringt. Alles andere — auch ein
  Update nur mit Baumdaten, wie es `RenderFrameHostImpl::UpdateAXTreeData`
  beim Commit aus dem Cache schickt (`render_frame_host_impl.cc:14087`) —
  lässt `Unserialize` scheitern, und das ist in Builds ohne
  `is_official_build` oder mit DCHECK ein **Absturz des Browsers**
  (`ui/accessibility/ax_common.h:11–17`, `AXTree::RecordError`
  `ax_tree.cc:3153–3187`, „Tree has no root.“ `:1415–1418`). Deshalb prüft
  `AXTreeMirror::CheckStart` vor dem ersten Update: nur Baumdaten → Paket
  übergehen; Knoten ohne Wurzel → Baum verwerfen und Reset anfordern.
- **Wiederaufsetzen [33]:** `WebContents::ResetAccessibility()` höchstens
  einmal je 5 s (`kResetInterval`, Vollbaum ~8 000 Knoten kostet 15–23 ms
  UI-Thread); eine Anfrage innerhalb der Spanne wird bis zu ihrem Ende
  aufgeschoben (eine offene genügt), nicht verworfen. Auslöser: Paket ohne
  Anfang, gescheitertes `Unserialize` (nur in offiziellen Builds
  erreichbar, sonst bricht AXTree vorher ab), Rückkehr aus dem Cache,
  Anhängen an `WebContents` mit schon aktivem `kWebContents`.
- **Discard [33]:** Zwei Wege in Chromium. Mit `kWebContentsDiscard`
  (`content_features.cc:381`, Standard aus, im Feldversuchs-Testconfig für
  Mac an und damit auch in `out/Relief`) bleiben die `WebContents`; der
  Helfer bleibt, der alte Baum fällt über `TreeRemoved`, nach dem Neuladen
  kommt der neue. Ohne ersetzt Discard die `WebContents`
  (`TabInterface::RegisterWillDiscardContents`,
  `components/tabs/public/tab_interface.h:182`): ein neuer Helfer mit neuer
  Runtime hängt an den neuen, der alte endet mit den alten. Beide Wege im
  Browser-Test.
- **Aktionen, die der Adapter meldet:** DoDefault, wenn Blink ein
  Default-Action-Verb setzt (Blink trägt DoDefault nicht ins Aktions-Bitfeld,
  `ax_object.cc:1400–1414, 1993–1994`); Focus/Blur bei `kFocusable`;
  SetValue/Increment/Decrement aus dem Bitfeld; Expand/Collapse aus dem
  Zustand.
- **Nicht übertragen:** `extra`-Attribute; Aktionen, die Chromium nicht im
  Knoten meldet (ScrollToMakeVisible, ShowContextMenu) → 24.
- **Texte:** `rust::String::lossy`, weil Blink-Texte ungepaarte Surrogate
  enthalten können und `rust::String(std::string)` bei ungültigem UTF-8
  ohne Ausnahmen abbricht.
- **Live auf echten Seiten [35]:** Consent-iframe im Graphen (nur
  gelesen, kein `--relief-activate`, keine Entscheidung im Dialog). Knoten mit Rolle, Namen und Position aus dem
  Protokoll (`--relief-log-nodes=button,iframe,dialog`, Zeilen `node`/`bounds`),
  Gegenprobe über CDP (`Page.getFrameTree`, `getBoundingClientRect` in einer
  isolierten Welt je Frame). Viewport 1280 × 813 CSS-Pixel,
  Geräte-Skalierung 1.

  | | spiegel.de | bild.de |
  |---|---|---|
  | Frame-Baum | `tree main` www.spiegel.de, `tree iframe` sp-spiegel-de.spiegel.de ~1,5 s danach | `tree main` www.bild.de, `tree iframe` cmp2.bild.de ~0,3 s danach |
  | Elternbaum | `dialog` und `iframe` „Privacy Center“, je 0 / 0 / 1280 × 813 | `dialog` und `iframe` „Cookie- und Einwilligungsbanner“, je 0 / 0 / 1280 × 813 |
  | Buttons im iframe-Baum | „Einwilligen und weiter“ 327 / 228,4 / 265 × 43, „Jetzt abonnieren“ 656 / 228,4 / 265 × 43, „Einstellungen“ 327 / 415 / 596 × 39 | „Alle akzeptieren“ 279,5 / 267,5 / 347,5 × 40, „Einstellungen“ 279,5 / 323,5, „Jetzt BILD PUR abonnieren“ 647 / 323,6, zwölf Zweck-Buttons 720 × 28–44 bei y 1 102,8–1 426,8 |
  | Gegenprobe DOM | alle drei gleich | alle 15 gleich |

  - **Positionen plausibel:** Beide iframes liegen bei 0 / 0 im
    unverschobenen Hauptdokument, Seitenkoordinaten = Koordinaten im iframe;
    sie stimmen auf 0,1 px mit dem DOM überein, auch unterhalb des
    iframe-Viewports (bild.de, y > 813). Eine Verschiebung des iframes ließ
    sich live nicht auslösen (Seite unter dem Dialog gesperrt); die Fälle
    „iframe verschieben“ und „Seite scrollen“ decken die Browser-Tests ab.
  - **Nachgereichte Namen kommen an:** Auf bild.de trug ein Zweck-Button zuerst
    „Loading...“ und wurde als geänderter Knoten mit dem endgültigen Namen
    nachgeliefert.
  - **Kein OOPIF:** Beide Consent-iframes sind same-site (Subdomain der Seite)
    und laufen im Prozess des Hauptframes; das Protokoll meldet sie als
    `iframe`, nicht `oopif`. Cross-site-iframes als OOPIF sind nur über den
    Browser-Test belegt, live auf diesen Seiten nicht vorhanden.
  - **Folge für den Graphen:** Solange ein Dialog mit `aria-modal` offen ist,
    nimmt Blink alles außerhalb aus dem Baum; auf spiegel.de schrumpft der
    Graph von ~7 800 auf 627 Knoten (Hauptbaum samt Dialog plus
    Consent-iframe, 09). Der Assistenz fehlt dann der Seiteninhalt, bis der
    Dialog zu ist.
- **Integrationstests [33]:** `//relief/testing/relief_browsertests`
  (`InProcessBrowserTest`, eigenes `test()`-Target nach Vorbild
  `sync_performance_tests`, eingehängt über die Gruppe
  `//relief:relief_tests`, also ohne Patch an einem Chromium-Build-File).
  Die Tests lesen den Graphen aus den Deltas, die die Runtime tatsächlich
  angewandt hat (`RuntimeHost::SetDeltaObserverForTesting`). Zwölf Fälle:
  ohne Schalter kein Helfer; Baum kommt an; `activate` wirkt; Positionen
  (Geräte-Skalierung 2, unterhalb des Viewports, Verschieben, Seite und
  Scroll-Container scrollen); Positionen mit Browser-Zoom 150 %; Positionen
  im iframe same-site und cross-site (Rahmen, außerhalb des iframe-Viewports,
  im iframe scrollen, Seite scrollen, iframe verschieben); cross-site-iframe als OOPIF
  (`IsolateAllSitesForTesting`, eigener Prozess, Elternbaum, `activate` im
  iframe); Navigation mit Back-Forward-Cache hin und zurück; Discard
  (beide Wege); begrenzter Neuaufbau. Erstbau des Targets 8,6 min, danach
  wie `chrome`; Lauf 24–55 s.

## Semantic Inspector (Paket 20) [belegt]

Side Panel je Tab mit einer WebUI (`chrome://relief-inspector.top-chrome`),
die den Graph der Seite live zeigt. Entscheidung **WebUI statt Views**:
semantisches HTML mit nativen Bedienelementen, dieselben Prüfwerkzeuge wie
für Webseiten und Chromiums eingebauter WebUI-Semantikprüfer (Blink bricht
in Builds mit DCHECK bei Namen auf verbotenen Rollen ab; er hat im Test
einen Namen auf `<dt>` gefunden, die Details sind deshalb eine Liste).

```
Strg+Umschalt+I (KeyPressEventCallback am Widget des Hauptframes) oder --relief-inspector
  → ToggleInspector(tab): Eintrag kRelief in der SidePanelRegistry des Tabs (beim ersten Öffnen),
    Aktions-Element kActionSidePanelShowRelief an BrowserActions (Titel der Kopfzeile)
  → InspectorView (SidePanelWebUIView) + InspectorContentsWrapper → ReliefInspectorUI
      Ressourcen aus inspector/resources/ als Header (embed_resources.py, kein grit)
      InspectorHandler (chrome.send): "ready" → ShowUI (erst dann zeigt das Side Panel
      den Eintrag) + Daten; "show" → „im Dokument zeigen“
ReliefTabHelper: nach jeder Delta OnGraphChanged → Handler bündelt 250 ms
  → RuntimeHost::InspectorJson → inspector_json (Rust) → WebUI-Listener "graph"
```

- **Daten** (`crates/relief-bridge/src/inspector.rs`): Bereiche,
  Überschriften und Bedienelemente des Interaction Graph mit Rolle, Namen
  und dessen Herkunft (`Certainty`, Quelle, Evidence), Bereich, Wert,
  Zuständen, gemeldeten Aktionen, Beziehungen (mit Namen der Ziele) und
  Erreichbarkeit bei offenem modalem Dialog; Schlüssel `<Baum>#<Knoten>`,
  stabil über Deltas; Position der Sitzung markiert.
- **Auswahl und Aktivierung getrennt:** Pfeiltasten in der Liste (natives
  `<select size>` mit `<optgroup>`) wählen aus und zeigen Details, sonst
  nichts. Eingabetaste oder „Im Dokument zeigen“ bewegt Relief zum Eintrag
  (`Runtime::show`: Bedienelemente fokussieren, Überschriften und Bereiche
  ansteuern, gesperrte ablehnen) und löst nie etwas aus, auch keinen
  riskanten Button. Der Fokus der Oberfläche bleibt im Panel.
- **Ansagen gebündelt:** Statuszeile (`role=status`) meldet Änderungen der
  Seite höchstens alle 3 s und nur, wenn sich die Zahl der Einträge ändert;
  abschaltbar.
- **Test:** `relief_browsertests --gtest_filter=*Inspektor*` — Kürzel öffnet
  und schließt, Graph erscheint und folgt einer DOM-Änderung, Auswahl ohne
  Wirkung auf der Seite, Aktivierung fokussiert ohne Klick, jedes
  Bedienelement des Panels hat einen Namen (AX-Baum der WebUI).
- **Nachweis live:** `spike/fixtures/shop-clean.html` und
  de.wikipedia.org/wiki/Barrierefreiheit (46 Bereiche, Seitentyp Artikel mit
  Evidence) im eigenen Build, Bildschirmfoto im PR.
- **Grenzen:** Das Kürzel hängt am Widget des Hauptframes; liegt der Fokus
  in einem cross-site-iframe (eigenes Widget), kommt es dort nicht an. Die
  Liste wird bei jeder Aktualisierung neu aufgebaut (Auswahl bleibt); ob
  VoiceOver dabei die Position hält, ist ungeprüft. Kürzel-Konflikte unter
  Windows/Linux (Strg+Umschalt+I = Entwicklertools) → 31/47.

## Befunde im Inspector (Paket 21) [belegt]

Der Inspector zeigt Befunde aus `a11y-rules`/`a11y-report` (barrierlab)
ohne eigenes Befundmodell: `Outcome` (Fehler, prüfen, nicht geprüft),
Severity und Regel-ID stehen am Knoten (Liste „[N Befunde]“, Details
„Befund“), seitenweite Befunde und die nicht gelaufenen Regeln im Abschnitt
„Prüfung“.

- **Welche Regeln laufen** (`crates/relief-interaction/src/rules.rs`,
  Feature `rules`): Der Fork hat nur den AXTree. `AxDocument` stellt ihn als
  `a11y_dom::Document` + `Semantics` dar (Dokumentreihenfolge über alle
  Bäume, iframes unter ihrem Host; Tag aus `kHtmlTag`, das Blink im
  Relief-Modus mitschickt und `ax_tree_mirror.cc` als `extra["htmlTag"]`
  überträgt; Text aus `StaticText`; als Attribut nur `href` an `a`).
  Darauf laufen die Regeln der Stufe `Semantics` (Stand a11y-rules 0.13.3:
  `links/name-missing`, `buttons/name-missing`, `svg/name-missing`,
  `links/ambiguous-name`, `links/generic-name`). Die Stufen `Structure`
  (Markup: `lang`, `title`, `alt`, Labels, ARIA-Attribute, IDs, tabindex …)
  und `Rendering` (Kontrast) stehen als `NotRun::CapabilityMissing` mit Grund
  „Nur der Accessibility-Tree liegt vor …“ im Bericht — nicht geprüft ist
  nicht bestanden.
- **Anbindung:** `inspector_json` hängt je Eintrag `findings` an (Ort über
  `AxDocument::node_ref`), dazu `checks` (Zahl gelaufener Regeln,
  Befunde ohne Eintrag in der Liste, nicht gelaufene Regeln mit Grund).
- **Bau im Fork:** Die barrierlab-Crates (seit Paket 45 auch `accname`) stehen nicht in
  `//third_party/rust`. Entscheidung: nicht ins Repository kopieren
  (barrierlab bleibt die Quelle), sondern `scripts/fork-apply.sh` holt die
  in `Cargo.lock` festgelegte Version aus der lokalen Cargo-Registry
  (`cargo fetch`, Pfad aus `cargo metadata`) nach
  `//relief/third_party/<crate>/src`; `//relief/third_party/BUILD.gn` baut
  sie (Edition 2024, `a11y_rules` mit Feature `de`). Die Dateilisten dort
  gelten für 0.13.3 und sind bei einem Versionswechsel nachzuziehen.
- **Test:** `relief_browsertests --gtest_filter=*Inspektor*` — der Button
  ohne Namen trägt „[1 Befund]“, der Abschnitt „Prüfung“ nennt die nicht
  geprüften Regeln.

## Name und Branding (Paket 36) [belegt]

Der Build heißt `out/Relief/Relief.app` (Binärdatei `Contents/MacOS/Relief`,
`Relief Framework.framework`, `Relief Helper*.app`), Bundle-ID
`de.casoon.relief`; Menüleiste und Dock zeigen `CFBundleName` = „Relief“.
Das Profil liegt unter `~/Library/Application Support/Relief`
(`CrProductDirName` im Info.plist, ausgewertet in
`chrome/common/chrome_paths_mac.mm:31`), Relief läuft also neben
Chrome/Chromium. Weg: Patch 6 in der Tabelle oben; der Neubau nach der
Änderung dauerte 2,5 min.

- **Belegt:** Info.plist (`CFBundleName`, `CFBundleIdentifier`,
  `CrProductDirName` = Relief/de.casoon.relief/Relief), Profilverzeichnis
  beim ersten Start angelegt, Über-Seite zeigt „Relief“ als Produkt,
  `relief_browsertests` und Fork-Aufgaben 01–05 laufen mit dem neuen Pfad.
- **Entscheidung:** Bundle-ID unter der Domain des Projekts
  (`de.casoon.relief`); Unternehmensangaben in BRANDING bleiben bei den
  Chromium-Autoren (Copyright des Codes).
- **Offen (→ 111):** übersetzte Texte mit wörtlichem „Chromium“ („Über
  Chromium“, „Hilfe für Chromium aufrufen“) und das Symbol.
- `scripts/chromium-setup.sh` baut weiter unverändertes Chromium
  (`Chromium.app`); erst `fork-apply.sh` bringt den Namen.

## Verzeichnisstruktur im Fork [Stand 20 · Rest Annahme]

```
//relief/
├── BUILD.gn       # rust_static_library relief_model_rs, relief_interaction_rs, relief_bridge_rs; action inspector_resources; source_set relief
├── relief_*.h/cc  # Einstieg je Tab, Schalter, Aufgaben-Runner
├── bridge/        # C++-Adapter AXTree ↔ Relief-Modell, Runtime-Sequenz, AXActionData-Rückweg
├── inspector/     # Semantic Inspector: Side-Panel-Eintrag, WebUI, Ressourcen
├── common/        # Mojo-Schnittstellen Browser ↔ Renderer (form_facts.mojom)
├── renderer/      # Renderer-Seite: FormFactsAgent je Frame
├── devtools/      # CDP-Domäne Relief.* (Paket 45)
├── third_party/   # BUILD.gn für a11y-dom, a11y-report, a11y-rules, accname; Quellen legt scripts/fork-apply.sh ab
├── crates/        # Kopie der Crates relief-model, relief-interaction, relief-bridge (scripts/fork-apply.sh)
├── speech/        # STT/TTS-Adapter (Annahme)
├── ai/            # Modell-Adapter (lokal/Cloud), nur hinter der Privacy Boundary (Annahme)
└── testing/       # Browser-Tests der Integrationspunkte (relief_browsertests, data/)
```

Fachlogik (Semantik, Graph, Intents, Validierung) liegt **nicht** hier, sondern
in browserfreien Rust-Crates (→ 02).

## Quellcode lesen ohne Build

Lokal (M4) liegt ein vollständiger Checkout unter `~/chromium/src`. Ohne ihn
(Cloud) reicht ein Sparse-Checkout ohne Historie außerhalb des Repos:

```bash
git clone --filter=blob:none --no-checkout --depth 1 --branch 154.0.8037.58 \
  https://chromium.googlesource.com/chromium/src.git chromium-src
cd chromium-src && git sparse-checkout set ui/accessibility \
  content/browser/accessibility content/browser/renderer_host \
  content/browser/web_contents content/public/browser \
  chrome/browser/ui/webui/side_panel/read_anything chrome/browser/ui/tabs \
  chrome/browser/ui/side_panel chrome/renderer/accessibility \
  extensions/browser/api/automation_internal \
  third_party/blink/renderer/modules/accessibility \
  third_party/blink/public/mojom build/rust docs/accessibility docs/rust \
  && git checkout
```

Spiegel bei gesperrtem `googlesource.com`: `github.com/chromium/chromium`,
gleiche Optionen.

Tag-Vergleiche (Historie einzelner Pfade) [belegt, 2026-09-24]: Gitiles
liefert Historienseiten (`+log/…`) nur noch angemeldet (403 „Please sign in
to view the history pages“); Dateiinhalte (`+/<tag>/<pfad>?format=TEXT`,
Base64) und Diffs (`+/<a>..<b>/<pfad>`) gehen weiter ohne Anmeldung. Für
`git log` über Tags reicht ein flacher, blobloser Abruf nur der gebrauchten
Tags, außerhalb jedes Checkouts:

```bash
git init cr && cd cr
git remote add origin https://chromium.googlesource.com/chromium/src.git
git fetch --no-tags --filter=blob:none --shallow-since=2026-04-10 origin \
  refs/tags/154.0.8037.58:refs/tags/154.0.8037.58   # weitere Tags ebenso
git log --oneline 153.0.8010.12..154.0.8037.58 -- chrome/browser/ui/tabs/tab_features.cc
```

Ein Tag: ~400 MB, zwölf Tags von 149 bis 154: ~800 MB. `--shallow-since`
muss vor dem ältesten Branch-Punkt liegen. `git diff` darauf lädt jede Datei
einzeln nach (~10 s je Datei) — Inhalte besser über Gitiles holen.
