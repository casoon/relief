// Relief. MIT-Lizenz wie das Relief-Repository.
//
// Browser-Tests je Integrationspunkt des Fork-Adapters (→
// plan/spezifikation/01, „Umsetzung im Fork“): Beobachter registriert, Baum
// kommt an, Aktion kommt an, Positionen (auch mit Browser-Zoom und im
// iframe), cross-site-iframe (OOPIF), Navigation mit Back-Forward-Cache,
// Discard, begrenzter Neuaufbau nach kaputtem Paket. Die Tests lesen den
// Graphen der Runtime über die Deltas, die sie tatsächlich angewandt hat
// (RuntimeHost::SetDeltaObserverForTesting).

#include <cmath>
#include <map>
#include <optional>
#include <string>
#include <string_view>
#include <vector>

#include "base/command_line.h"
#include "base/functional/bind.h"
#include "base/memory/weak_ptr.h"
#include "base/strings/stringprintf.h"
#include "base/task/bind_post_task.h"
#include "base/test/run_until.h"
#include "base/test/scoped_feature_list.h"
#include "base/time/time.h"
#include "chrome/browser/resource_coordinator/tab_lifecycle_unit_external.h"
#include "chrome/browser/ui/browser.h"
#include "chrome/browser/ui/tabs/tab_strip_model.h"
#include "components/tabs/public/tab_interface.h"
#include "content/public/test/scoped_accessibility_mode_override.h"
#include "chrome/test/base/in_process_browser_test.h"
#include "chrome/test/base/ui_test_utils.h"
#include "content/public/browser/host_zoom_map.h"
#include "content/public/browser/reload_type.h"
#include "content/public/browser/render_frame_host.h"
#include "content/public/browser/render_process_host.h"
#include "content/public/browser/web_contents.h"
#include "content/public/common/content_features.h"
#include "content/public/test/browser_test.h"
#include "content/public/test/browser_test_utils.h"
#include "content/public/test/test_navigation_observer.h"
#include "content/public/test/test_utils.h"
#include "net/dns/mock_host_resolver.h"
#include "net/test/embedded_test_server/embedded_test_server.h"
#include "chrome/browser/ui/side_panel/side_panel_entry.h"
#include "chrome/browser/ui/side_panel/side_panel_ui.h"
#include "relief/inspector/relief_inspector_ui.h"
#include "relief/relief_switches.h"
#include "relief/relief_tab_helper.h"
#include "testing/gtest/include/gtest/gtest.h"
#include "third_party/blink/public/common/page/page_zoom.h"
#include "ui/accessibility/ax_enum_util.h"
#include "ui/accessibility/ax_enums.mojom.h"
#include "ui/accessibility/ax_mode.h"
#include "ui/accessibility/ax_role_properties.h"
#include "components/input/native_web_keyboard_event.h"
#include "content/public/browser/render_widget_host.h"
#include "ui/events/keycodes/dom/dom_code.h"
#include "ui/events/keycodes/dom/dom_key.h"
#include "ui/events/keycodes/keyboard_codes.h"
#include "ui/accessibility/ax_node_data.h"
#include "ui/accessibility/ax_tree_update.h"
#include "ui/accessibility/ax_updates_and_events.h"
#include "ui/base/window_open_disposition.h"

namespace relief {

namespace {

// Stand eines Knotens im Graphen der Runtime, soweit die Tests ihn brauchen.
struct NodeInfo {
  std::string role;
  std::string name;
  std::optional<bridge::Rect> bounds;
};

struct TreeInfo {
  std::map<int32_t, NodeInfo> nodes;
  std::optional<int32_t> root;
  std::string parent_tree;
};

// Führt den Graphen der Runtime aus den angewandten Deltas nach (UI-Thread).
class GraphRecorder {
 public:
  explicit GraphRecorder(ReliefTabHelper& helper) {
    helper.SetDeltaObserverForTesting(base::BindPostTaskToCurrentDefault(
        base::BindRepeating(&GraphRecorder::Add, weak_factory_.GetWeakPtr())));
  }

  // Erster Knoten mit genau diesem Namen, optional nur in `tree` und nur
  // mit Rolle `role`.
  const NodeInfo* Find(std::string_view name,
                       std::string_view tree = {},
                       std::string_view role = {}) const {
    for (const auto& [id, info] : trees_) {
      if (!tree.empty() && id != tree) {
        continue;
      }
      for (const auto& [node, node_info] : info.nodes) {
        if (node_info.name == name &&
            (role.empty() || node_info.role == role)) {
          return &node_info;
        }
      }
    }
    return nullptr;
  }
  const TreeInfo* Tree(std::string_view id) const {
    auto it = trees_.find(std::string(id));
    return it == trees_.end() ? nullptr : &it->second;
  }
  const std::string& root() const { return root_; }
  int removed(std::string_view tree) const {
    auto it = removed_.find(std::string(tree));
    return it == removed_.end() ? 0 : it->second;
  }

 private:
  void Add(const bridge::Delta& delta) {
    for (const rust::String& id : delta.removed_trees) {
      trees_.erase(std::string(id));
      ++removed_[std::string(id)];
      if (root_ == std::string(id)) {
        root_.clear();
      }
    }
    if (delta.has_root) {
      root_ = std::string(delta.root);
    }
    for (const bridge::TreeUpdate& update : delta.trees) {
      TreeInfo& tree = trees_[std::string(update.tree)];
      for (int32_t id : update.removed) {
        tree.nodes.erase(id);
      }
      for (const auto* nodes : {&update.created, &update.changed}) {
        for (const bridge::Node& node : *nodes) {
          NodeInfo& info = tree.nodes[node.id];
          info.role = std::string(node.role);
          info.name = node.has_name ? std::string(node.name) : std::string();
          info.bounds =
              node.has_bounds ? std::optional(node.bounds) : std::nullopt;
        }
      }
      for (const bridge::BoundsChange& change : update.bounds) {
        tree.nodes[change.node].bounds =
            change.has_bounds ? std::optional(change.bounds) : std::nullopt;
      }
      if (update.has_root) {
        tree.root = update.root;
      }
      if (update.has_data && update.data.has_parent) {
        tree.parent_tree = std::string(update.data.parent_tree);
      }
    }
  }

  std::map<std::string, TreeInfo, std::less<>> trees_;
  std::map<std::string, int> removed_;
  std::string root_;
  base::WeakPtrFactory<GraphRecorder> weak_factory_{this};
};

// Position des Buttons mit Namen `name` ist `expected` (± 1 CSS-Pixel).
bool HasBounds(const GraphRecorder& graph,
               std::string_view name,
               const bridge::Rect& expected) {
  const NodeInfo* node = graph.Find(name, {}, "button");
  if (!node || !node->bounds) {
    return false;
  }
  const bridge::Rect& b = *node->bounds;
  return std::abs(b.x - expected.x) <= 1 && std::abs(b.y - expected.y) <= 1 &&
         std::abs(b.width - expected.width) <= 1 &&
         std::abs(b.height - expected.height) <= 1;
}

// Für Fehlermeldungen: Position des Knotens mit Namen `name`.
std::string BoundsOf(const GraphRecorder& graph, std::string_view name) {
  const NodeInfo* node = graph.Find(name, {}, "button");
  if (!node) {
    return std::string(name) + ": kein Knoten";
  }
  if (!node->bounds) {
    return std::string(name) + ": keine Position";
  }
  const bridge::Rect& b = *node->bounds;
  return base::StringPrintf("%s: %.1f,%.1f %.1fx%.1f",
                            std::string(name).c_str(), b.x, b.y, b.width,
                            b.height);
}

std::string TreeOf(content::RenderFrameHost* frame) {
  return frame->GetAXTreeID().ToString();
}

class ReliefBrowserTest : public InProcessBrowserTest {
 protected:
  void SetUpCommandLine(base::CommandLine* command_line) override {
    command_line->AppendSwitch(switches::kEnableRelief);
  }

  void SetUpOnMainThread() override {
    host_resolver()->AddRule("*", "127.0.0.1");
    embedded_test_server()->ServeFilesFromSourceDirectory(
        "relief/testing/data");
    ASSERT_TRUE(embedded_test_server()->Start());
  }

  content::WebContents* web_contents() {
    return browser()->tab_strip_model()->GetActiveWebContents();
  }
  ReliefTabHelper& helper() {
    ReliefTabHelper* helper = ReliefTabHelper::FromWebContents(web_contents());
    CHECK(helper);
    return *helper;
  }
  GURL Url(std::string_view host, std::string_view path) {
    return embedded_test_server()->GetURL(host, path);
  }
};

// Ohne --enable-relief hängt sich nichts ein.
class ReliefDisabledBrowserTest : public InProcessBrowserTest {};

IN_PROC_BROWSER_TEST_F(ReliefDisabledBrowserTest, OhneSchalterKeinHelfer) {
  EXPECT_FALSE(ReliefTabHelper::FromWebContents(
      browser()->tab_strip_model()->GetActiveWebContents()));
}

// Beobachter registriert, Baum kommt an, Hauptbaum ist die Wurzel.
IN_PROC_BROWSER_TEST_F(ReliefBrowserTest, BaumKommtAn) {
  GraphRecorder graph(helper());
  ASSERT_TRUE(
      ui_test_utils::NavigateToURL(browser(), Url("a.test", "/laden.html")));
  const std::string main = TreeOf(web_contents()->GetPrimaryMainFrame());
  ASSERT_TRUE(base::test::RunUntil([&] {
    return graph.Find("In den Warenkorb", main) && graph.root() == main;
  }));
  EXPECT_TRUE(graph.Find("Laufschuh", main));
}

class ReliefActivateBrowserTest : public ReliefBrowserTest {
 protected:
  void SetUpCommandLine(base::CommandLine* command_line) override {
    ReliefBrowserTest::SetUpCommandLine(command_line);
    command_line->AppendSwitchASCII(switches::kReliefActivate,
                                    "In den Warenkorb");
  }
};

// Aktion kommt an: `activate` über AXActionData, Wirkung in der nächsten Delta.
IN_PROC_BROWSER_TEST_F(ReliefActivateBrowserTest, AktionKommtAn) {
  GraphRecorder graph(helper());
  ASSERT_TRUE(
      ui_test_utils::NavigateToURL(browser(), Url("a.test", "/laden.html")));
  ASSERT_TRUE(
      base::test::RunUntil([&] { return graph.Find("Im Warenkorb."); }));
  EXPECT_EQ("Im Warenkorb.",
            content::EvalJs(web_contents(),
                            "document.getElementById('s').textContent"));
}

class ReliefPositionBrowserTest : public ReliefBrowserTest {
 protected:
  void SetUpCommandLine(base::CommandLine* command_line) override {
    ReliefBrowserTest::SetUpCommandLine(command_line);
    // Blink rechnet AX-Positionen in Geräte-Pixeln; Relief gibt CSS-Pixel.
    command_line->AppendSwitchASCII("force-device-scale-factor", "2");
  }
};

// Positionen: Seitenkoordinaten in CSS-Pixeln, nachgeführt über
// AccessibilityLocationChangesReceived, auch unter einem Scroll-Container.
IN_PROC_BROWSER_TEST_F(ReliefPositionBrowserTest, Positionen) {
  GraphRecorder graph(helper());
  ASSERT_TRUE(ui_test_utils::NavigateToURL(browser(),
                                           Url("a.test", "/positionen.html")));
  // Unterhalb des sichtbaren Bereichs: ungeclippt, Seitenkoordinaten.
  ASSERT_TRUE(base::test::RunUntil([&] {
    return HasBounds(graph, "Fest", {100, 1200, 80, 30}) &&
           HasBounds(graph, "Tief", {30, 200, 60, 20});
  })) << BoundsOf(graph, "Fest")
      << " / " << BoundsOf(graph, "Tief");

  // Verschieben: nur die Position ändert sich.
  ASSERT_TRUE(content::ExecJs(web_contents(),
                              "document.getElementById('fest').style.top = "
                              "'1300px'"));
  ASSERT_TRUE(base::test::RunUntil([&] {
    return HasBounds(graph, "Fest", {100, 1300, 80, 30});
  })) << BoundsOf(graph, "Fest");

  // Seite scrollen ändert Seitenkoordinaten nicht; danach verschieben als
  // Marke, dass die Scroll-Änderung durch ist.
  ASSERT_TRUE(content::ExecJs(web_contents(), "window.scrollTo(0, 1000)"));
  ASSERT_TRUE(content::ExecJs(web_contents(),
                              "document.getElementById('fest').style.left = "
                              "'150px'"));
  ASSERT_TRUE(base::test::RunUntil([&] {
    return HasBounds(graph, "Fest", {150, 1300, 80, 30});
  })) << BoundsOf(graph, "Fest");
  EXPECT_TRUE(HasBounds(graph, "Tief", {30, 200, 60, 20}))
      << BoundsOf(graph, "Tief");

  // Scroll-Container scrollen verschiebt seinen Inhalt auf der Seite.
  ASSERT_TRUE(content::ExecJs(web_contents(),
                              "document.getElementById('kasten').scrollTop = "
                              "100"));
  ASSERT_TRUE(base::test::RunUntil([&] {
    return HasBounds(graph, "Tief", {30, 100, 60, 20});
  })) << BoundsOf(graph, "Tief");
}

// Browser-Zoom: Positionen bleiben CSS-Pixel (Blink-Pixel = CSS-Pixel ×
// Geräte-Skalierung × Zoom).
IN_PROC_BROWSER_TEST_F(ReliefPositionBrowserTest, PositionenMitZoom) {
  GraphRecorder graph(helper());
  ASSERT_TRUE(ui_test_utils::NavigateToURL(browser(),
                                           Url("a.test", "/positionen.html")));
  ASSERT_TRUE(base::test::RunUntil([&] {
    return HasBounds(graph, "Fest", {100, 1200, 80, 30}) &&
           HasBounds(graph, "Tief", {30, 200, 60, 20});
  })) << BoundsOf(graph, "Fest")
      << " / " << BoundsOf(graph, "Tief");

  content::HostZoomMap::SetZoomLevel(web_contents(),
                                     blink::ZoomFactorToZoomLevel(1.5));
  // Verschieben als Marke, dass der Zoom im Baum angekommen ist.
  ASSERT_TRUE(content::ExecJs(web_contents(),
                              "document.getElementById('fest').style.top = "
                              "'1300px'"));
  ASSERT_TRUE(base::test::RunUntil([&] {
    return HasBounds(graph, "Fest", {100, 1300, 80, 30}) &&
           HasBounds(graph, "Tief", {30, 200, 60, 20});
  })) << BoundsOf(graph, "Fest")
      << " / " << BoundsOf(graph, "Tief");
  // Die Seite ist tatsächlich gezoomt: 2 (Geräte-Skalierung) × 1,5.
  EXPECT_EQ(3, content::EvalJs(web_contents(), "window.devicePixelRatio"));
}

// Positionen in einem iframe in Koordinaten des Hauptdokuments: Viewport des
// iframes am Host-Knoten im Elternbaum, Scrollen im iframe verschiebt den
// Inhalt, Scrollen der Seite nicht. Parameter: cross-site (OOPIF, eigener
// Prozess) oder same-site (selber Prozess, eigener Baum).
class ReliefRahmenPositionBrowserTest
    : public ReliefPositionBrowserTest,
      public testing::WithParamInterface<bool> {
 protected:
  void SetUpCommandLine(base::CommandLine* command_line) override {
    ReliefPositionBrowserTest::SetUpCommandLine(command_line);
    content::IsolateAllSitesForTesting(command_line);
  }
};

IN_PROC_BROWSER_TEST_P(ReliefRahmenPositionBrowserTest, PositionenImRahmen) {
  GraphRecorder graph(helper());
  ASSERT_TRUE(ui_test_utils::NavigateToURL(
      browser(), Url("a.test", "/rahmen-positionen.html")));
  ASSERT_TRUE(content::NavigateIframeToURL(
      web_contents(), "rahmen",
      Url(GetParam() ? "b.test" : "a.test", "/inhalt.html")));
  content::RenderFrameHost* main = web_contents()->GetPrimaryMainFrame();
  content::RenderFrameHost* child = content::ChildFrameAt(main, 0);
  ASSERT_TRUE(child);
  EXPECT_EQ(GetParam(), main->GetProcess() != child->GetProcess());

  // iframe bei 50,400 mit 5 px Rahmen; Knöpfe bei 20,30 und 20,600 im
  // iframe, „Unten“ außerhalb seines Viewports (ungeclippt).
  ASSERT_TRUE(base::test::RunUntil([&] {
    return HasBounds(graph, "Innen", {75, 435, 60, 20}) &&
           HasBounds(graph, "Unten", {75, 1005, 60, 20});
  })) << BoundsOf(graph, "Innen")
      << " / " << BoundsOf(graph, "Unten");

  // Im iframe scrollen: sein Inhalt rückt auf der Seite nach oben.
  ASSERT_TRUE(content::ExecJs(child, "window.scrollTo(0, 100)"));
  ASSERT_TRUE(base::test::RunUntil([&] {
    return HasBounds(graph, "Innen", {75, 335, 60, 20}) &&
           HasBounds(graph, "Unten", {75, 905, 60, 20});
  })) << BoundsOf(graph, "Innen")
      << " / " << BoundsOf(graph, "Unten");

  // Seite scrollen ändert nichts; danach den iframe verschieben (Marke):
  // sein ganzer Baum rückt mit.
  ASSERT_TRUE(content::ExecJs(web_contents(), "window.scrollTo(0, 1000)"));
  ASSERT_TRUE(content::ExecJs(web_contents(),
                              "document.getElementById('rahmen').style.top = "
                              "'500px'"));
  ASSERT_TRUE(base::test::RunUntil([&] {
    return HasBounds(graph, "Innen", {75, 435, 60, 20}) &&
           HasBounds(graph, "Unten", {75, 1005, 60, 20});
  })) << BoundsOf(graph, "Innen")
      << " / " << BoundsOf(graph, "Unten");
}

INSTANTIATE_TEST_SUITE_P(All,
                         ReliefRahmenPositionBrowserTest,
                         testing::Bool(),
                         [](const testing::TestParamInfo<bool>& info) {
                           return info.param ? "CrossSite" : "SameSite";
                         });

class ReliefOopifBrowserTest : public ReliefBrowserTest {
 protected:
  void SetUpCommandLine(base::CommandLine* command_line) override {
    ReliefBrowserTest::SetUpCommandLine(command_line);
    content::IsolateAllSitesForTesting(command_line);
    command_line->AppendSwitchASCII(switches::kReliefActivate,
                                    "Alle akzeptieren");
  }

  // Lädt rahmen.html von a.test mit einwilligung.html von b.test im iframe.
  content::RenderFrameHost* LoadCrossSite() {
    CHECK(
        ui_test_utils::NavigateToURL(browser(), Url("a.test", "/rahmen.html")));
    CHECK(content::NavigateIframeToURL(web_contents(), "rahmen",
                                       Url("b.test", "/einwilligung.html")));
    return content::ChildFrameAt(web_contents()->GetPrimaryMainFrame(), 0);
  }
};

// OOPIF: Baum des cross-site-iframes kommt an, hängt am iframe-Knoten des
// Hauptbaums, `activate` im iframe wirkt.
IN_PROC_BROWSER_TEST_F(ReliefOopifBrowserTest, CrossSiteIframe) {
  GraphRecorder graph(helper());
  content::RenderFrameHost* child = LoadCrossSite();
  content::RenderFrameHost* main = web_contents()->GetPrimaryMainFrame();
  ASSERT_TRUE(child);
  ASSERT_NE(main->GetProcess(), child->GetProcess());

  ASSERT_TRUE(base::test::RunUntil(
      [&] { return graph.Find("Zustimmung gespeichert.", TreeOf(child)); }));
  EXPECT_EQ(TreeOf(main), graph.Tree(TreeOf(child))->parent_tree);
  EXPECT_EQ(TreeOf(main), graph.root());
}

// Navigation: alter Haupt- und iframe-Baum fallen weg (auch wenn die Seite in
// den Back-Forward-Cache geht), iframe-Eltern werden aufgeräumt; zurück aus
// dem Cache kommt der alte Baum wieder.
IN_PROC_BROWSER_TEST_F(ReliefOopifBrowserTest, Navigation) {
  GraphRecorder graph(helper());
  content::RenderFrameHost* child = LoadCrossSite();
  const std::string old_main = TreeOf(web_contents()->GetPrimaryMainFrame());
  const std::string old_child = TreeOf(child);
  ASSERT_TRUE(base::test::RunUntil(
      [&] { return graph.Find("Zustimmung gespeichert.", old_child); }));
  ASSERT_TRUE(content::ExecJs(web_contents(), "window.reliefMarke = 1"));

  ASSERT_TRUE(
      ui_test_utils::NavigateToURL(browser(), Url("a.test", "/laden.html")));
  const std::string new_main = TreeOf(web_contents()->GetPrimaryMainFrame());
  ASSERT_NE(old_main, new_main);
  ASSERT_TRUE(base::test::RunUntil([&] {
    return !graph.Tree(old_main) && !graph.Tree(old_child) &&
           graph.Find("In den Warenkorb", new_main) && graph.root() == new_main;
  }));
  EXPECT_EQ(1u, helper().trees_for_testing());
  EXPECT_EQ(0u, helper().hosts_for_testing());

  ASSERT_TRUE(content::HistoryGoBack(web_contents()));
  // Die Marke steht noch: Seite kam aus dem Back-Forward-Cache.
  EXPECT_EQ(true, content::EvalJs(web_contents(), "window.reliefMarke === 1"));
  EXPECT_EQ(old_main, TreeOf(web_contents()->GetPrimaryMainFrame()));
  ASSERT_TRUE(base::test::RunUntil([&] {
    return graph.Find("Nachrichten", old_main) &&
           graph.Find("Alle akzeptieren", old_child) &&
           graph.root() == old_main && !graph.Tree(new_main);
  }));
  EXPECT_GE(helper().resets_for_testing(), 1);
}

// Discard: Relief hängt danach an den WebContents des Tabs, und deren
// Runtime bekommt nach dem Aufdecken den neuen Baum. Zwei Wege in Chromium:
// kWebContentsDiscard (Standard aus, im Feldversuchs-Testconfig für Mac an,
// also auch in out/Relief) verwirft in denselben WebContents; ohne ersetzt
// Discard die WebContents (TabInterface::RegisterWillDiscardContents).
class ReliefDiscardBrowserTest : public ReliefBrowserTest,
                                 public testing::WithParamInterface<bool> {
 protected:
  ReliefDiscardBrowserTest() {
    features_.InitWithFeatureState(features::kWebContentsDiscard, GetParam());
  }

 private:
  base::test::ScopedFeatureList features_;
};

IN_PROC_BROWSER_TEST_P(ReliefDiscardBrowserTest, Discard) {
  const GURL url = Url("a.test", "/laden.html");
  ASSERT_TRUE(ui_test_utils::NavigateToURL(browser(), url));
  content::WebContents* old_contents = web_contents();
  const std::string old_main = TreeOf(old_contents->GetPrimaryMainFrame());
  ASSERT_TRUE(ui_test_utils::NavigateToURLWithDisposition(
      browser(), Url("b.test", "/einwilligung.html"),
      WindowOpenDisposition::NEW_FOREGROUND_TAB,
      ui_test_utils::BROWSER_TEST_WAIT_FOR_LOAD_STOP));

  ASSERT_TRUE(resource_coordinator::TabLifecycleUnitExternal::FromWebContents(
                  old_contents)
                  ->DiscardTab(mojom::LifecycleUnitDiscardReason::URGENT));
  content::WebContents* new_contents =
      browser()->tab_strip_model()->GetWebContentsAt(0);
  // GetParam(): in denselben WebContents verworfen.
  EXPECT_EQ(GetParam(), old_contents == new_contents);
  ReliefTabHelper* new_helper = ReliefTabHelper::FromWebContents(new_contents);
  ASSERT_TRUE(new_helper);
  GraphRecorder graph(*new_helper);

  // Aufdecken und neu laden, wie beim Klick auf den verworfenen Tab (im Test
  // lädt das Aufdecken allein nicht verlässlich, vgl.
  // tab_manager_browsertest.cc).
  browser()->tab_strip_model()->ActivateTabAt(0);
  content::TestNavigationObserver reload(new_contents);
  new_contents->GetController().Reload(content::ReloadType::NORMAL, false);
  reload.Wait();
  const std::string main = TreeOf(new_contents->GetPrimaryMainFrame());
  EXPECT_NE(old_main, main);
  ASSERT_TRUE(base::test::RunUntil([&] {
    return graph.Find("In den Warenkorb", main) && graph.root() == main;
  }));
  EXPECT_EQ(1u, new_helper->trees_for_testing());
}

INSTANTIATE_TEST_SUITE_P(All,
                         ReliefDiscardBrowserTest,
                         testing::Bool(),
                         [](const testing::TestParamInfo<bool>& info) {
                           return info.param ? "WebContentsBleiben"
                                             : "WebContentsErsetzt";
                         });

// Neuaufbau nach kaputtem Paket: ein Reset sofort, ein zweiter frühestens
// nach der Mindestspanne, danach ist der Baum wieder da.
//
// Kaputt heißt hier: inkrementelles Paket für einen Baum, den Relief nicht
// (mehr) hat. Ein Paket, an dem AXTree::Unserialize scheitert, lässt sich in
// diesem Build nicht einspielen: ohne is_official_build bzw. mit DCHECK
// bricht AXTree dabei selbst ab (ui/accessibility/ax_common.h).
IN_PROC_BROWSER_TEST_F(ReliefBrowserTest, NeuaufbauBegrenzt) {
  constexpr base::TimeDelta kInterval = base::Seconds(2);
  GraphRecorder graph(helper());
  helper().SetResetIntervalForTesting(kInterval);
  ASSERT_TRUE(
      ui_test_utils::NavigateToURL(browser(), Url("a.test", "/laden.html")));
  const ui::AXTreeID main_id =
      web_contents()->GetPrimaryMainFrame()->GetAXTreeID();
  const std::string main = main_id.ToString();
  ASSERT_TRUE(base::test::RunUntil(
      [&] { return graph.Find("In den Warenkorb", main); }));
  ASSERT_EQ(0, helper().resets_for_testing());

  // Wurzel geändert, aber ohne root_id: nur für einen Baum mit Wurzel gültig.
  ui::AXUpdatesAndEvents broken;
  broken.ax_tree_id = main_id;
  ui::AXTreeUpdate update;
  ui::AXNodeData root;
  root.id = *graph.Tree(main)->root;
  root.role = ax::mojom::Role::kRootWebArea;
  update.nodes.push_back(root);
  broken.updates.push_back(update);

  // Baum verlieren (wie nach TreeRemoved), dann das Paket.
  helper().TreeRemoved(main_id);
  helper().AccessibilityEventReceived(broken);
  const base::TimeTicks first = base::TimeTicks::Now();
  EXPECT_EQ(1, helper().resets_for_testing());
  ASSERT_TRUE(base::test::RunUntil([&] {
    return graph.removed(main) == 1 && graph.Find("In den Warenkorb", main);
  }));

  // Noch einmal innerhalb der Spanne: Reset aufgeschoben, nicht verloren.
  helper().TreeRemoved(main_id);
  helper().AccessibilityEventReceived(broken);
  EXPECT_EQ(1, helper().resets_for_testing());
  ASSERT_TRUE(
      base::test::RunUntil([&] { return helper().resets_for_testing() == 2; }));
  EXPECT_GE(base::TimeTicks::Now() - first, kInterval);
  ASSERT_TRUE(base::test::RunUntil([&] {
    return graph.removed(main) == 2 && graph.Find("In den Warenkorb", main);
  }));
}

}  // namespace

}  // namespace relief

namespace relief {

namespace {

content::WebContents* FindInspector() {
  for (content::WebContents* contents : content::GetAllWebContents()) {
    if (contents->GetLastCommittedURL() == GURL(kInspectorUrl)) {
      return contents;
    }
  }
  return nullptr;
}

// Texte aller Einträge der Knotenliste im Inspector.
std::string InspectorOptions(content::WebContents* inspector) {
  return content::EvalJs(inspector,
                         "Array.from(document.querySelectorAll('#nodes "
                         "option'), o => o.textContent).join('\\n')")
      .ExtractString();
}

}  // namespace

// Semantic Inspector (Paket 20): Panel zeigt den Graph live, Auswahl und
// Aktivierung sind getrennt, „im Dokument zeigen“ löst nichts aus, die
// eigenen Bedienelemente haben Namen.
IN_PROC_BROWSER_TEST_F(ReliefBrowserTest, Inspektor) {
  ASSERT_TRUE(ui_test_utils::NavigateToURL(browser(),
                                           Url("a.test", "/inspektor.html")));
  // Strg+Umschalt+I auf der Seite öffnet den Inspector.
  // Direkt an das Widget des Hauptframes, wie die Tastatur es liefert.
  auto press_shortcut = [&] {
    input::NativeWebKeyboardEvent event(
        blink::WebInputEvent::Type::kRawKeyDown,
        blink::WebInputEvent::kControlKey | blink::WebInputEvent::kShiftKey,
        base::TimeTicks::Now());
    event.windows_key_code = ui::VKEY_I;
    event.dom_key = ui::DomKey::FromCharacter('I');
    event.dom_code = static_cast<int>(ui::DomCode::US_I);
    web_contents()
        ->GetPrimaryMainFrame()
        ->GetRenderWidgetHost()
        ->ForwardKeyboardEvent(event);
  };
  SidePanelUI* side_panel = SidePanelUI::From(browser());
  const SidePanelEntry::Key key(SidePanelEntry::Id::kRelief);
  web_contents()->Focus();
  press_shortcut();
  ASSERT_TRUE(base::test::RunUntil(
      [&] { return side_panel->IsSidePanelEntryShowing(key); }));
  content::WebContents* inspector = nullptr;
  ASSERT_TRUE(base::test::RunUntil([&] {
    inspector = FindInspector();
    return inspector && !inspector->IsLoading() &&
           InspectorOptions(inspector).find("H1 Laufschuh") !=
               std::string::npos;
  }));
  const std::string options = InspectorOptions(inspector);
  EXPECT_NE(options.find("navigation „Hauptmenü“"), std::string::npos);
  EXPECT_NE(options.find("[button] In den Warenkorb"), std::string::npos);
  // Ohne Namen: Herkunft wird genannt, nicht verschwiegen.
  EXPECT_NE(options.find("[Name "), std::string::npos) << options;

  // Live: ein neues Bedienelement erscheint ohne Zutun.
  ASSERT_TRUE(content::ExecJs(web_contents(), R"(
      const b = document.createElement('button');
      b.textContent = 'Neu geladen';
      document.querySelector('main').append(b);)"));
  ASSERT_TRUE(base::test::RunUntil([&] {
    return InspectorOptions(inspector).find("[button] Neu geladen") !=
           std::string::npos;
  }));

  // Auswahl: Details, keine Wirkung auf der Seite.
  const std::string select_kaufen = R"(
      const nodes = document.getElementById('nodes');
      nodes.value = Array.from(nodes.options)
          .find(o => o.textContent.startsWith('[button] In den Warenkorb')).value;
      nodes.dispatchEvent(new Event('change'));
      document.getElementById('details').textContent)";
  EXPECT_NE(content::EvalJs(inspector, select_kaufen).ExtractString().find(
                "Rolle: buttonName: "),
            std::string::npos);
  EXPECT_EQ("BODY", content::EvalJs(web_contents(),
                                     "document.activeElement.tagName"));

  // Aktivierung: fokussiert im Dokument, löst den Button nicht aus.
  ASSERT_TRUE(content::ExecJs(
      inspector,
      "document.getElementById('nodes').dispatchEvent("
      "new KeyboardEvent('keydown', {key: 'Enter'}))"));
  ASSERT_TRUE(base::test::RunUntil([&] {
    return content::EvalJs(web_contents(), "document.activeElement.id")
               .ExtractString() == "kaufen";
  }));
  EXPECT_EQ("", content::EvalJs(web_contents(),
                                "document.getElementById('s').textContent"));
  ASSERT_TRUE(base::test::RunUntil([&] {
    return content::EvalJs(inspector,
                           "document.getElementById('status').textContent")
               .ExtractString()
               .find("Focus auf [button] In den Warenkorb") !=
           std::string::npos;
  }));

  // Semantik des Panels: jedes Bedienelement hat einen Namen.
  content::ScopedAccessibilityModeOverride mode(inspector,
                                                ui::kAXModeComplete);
  content::WaitForAccessibilityTreeToContainNodeWithName(
      inspector, "Im Dokument zeigen");
  const ui::AXTreeUpdate tree = content::GetAccessibilityTreeSnapshot(inspector);
  int controls = 0;
  for (const ui::AXNodeData& node : tree.nodes) {
    if (ui::IsControl(node.role) && !node.IsIgnored()) {
      ++controls;
      EXPECT_FALSE(
          node.GetStringAttribute(ax::mojom::StringAttribute::kName).empty())
          << ui::ToString(node.role);
    }
  }
  EXPECT_GE(controls, 3);

  // Dasselbe Kürzel auf der Seite schließt ihn wieder.
  web_contents()->Focus();
  press_shortcut();
  EXPECT_TRUE(base::test::RunUntil(
      [&] { return !side_panel->IsSidePanelEntryShowing(key); }));
}

}  // namespace relief
