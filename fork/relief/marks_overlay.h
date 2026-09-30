// Relief. MIT-Lizenz wie das Relief-Repository.

#ifndef RELIEF_MARKS_OVERLAY_H_
#define RELIEF_MARKS_OVERLAY_H_

#include <memory>
#include <string>
#include <vector>

#include "base/memory/raw_ref.h"
#include "ui/gfx/geometry/rect.h"

namespace content {
class WebContents;
}  // namespace content

namespace views {
class Widget;
}  // namespace views

namespace relief {

// Zeichnet Sprungmarken (Paket 38) über den Inhalt eines Tabs: ein
// transparentes, nicht aktivierbares Fenster ohne eigene Eingaben, dessen
// Inhalt für Assistenztechnik ausgeblendet ist (die Marken sind eine
// Sehhilfe für Tastaturbedienung; VoiceOver liest weiter die Seite). Die
// Tasten fängt der Tab-Helfer ab.
class ReliefMarksOverlay {
 public:
  struct Box {
    std::u16string label;
    // Relativ zum sichtbaren Inhalt des Tabs, in DIP.
    gfx::Rect rect;
    // Name nicht gesichert: anders gezeichnet.
    bool uncertain = false;
  };

  explicit ReliefMarksOverlay(content::WebContents& contents);
  ReliefMarksOverlay(const ReliefMarksOverlay&) = delete;
  ReliefMarksOverlay& operator=(const ReliefMarksOverlay&) = delete;
  ~ReliefMarksOverlay();

  void Show(std::vector<Box> boxes);
  void Hide();
  bool visible() const { return widget_ != nullptr; }

 private:
  const raw_ref<content::WebContents> contents_;
  std::unique_ptr<views::Widget> widget_;
};

}  // namespace relief

#endif  // RELIEF_MARKS_OVERLAY_H_
