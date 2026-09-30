// Relief. MIT-Lizenz wie das Relief-Repository.

#include "relief/marks_overlay.h"

#include <utility>

#include "cc/paint/paint_flags.h"
#include "content/public/browser/web_contents.h"
#include "third_party/skia/include/core/SkColor.h"
#include "ui/gfx/canvas.h"
#include "ui/gfx/font_list.h"
#include "ui/gfx/text_constants.h"
#include "ui/views/accessibility/view_accessibility.h"
#include "ui/views/view.h"
#include "ui/views/widget/widget.h"

namespace relief {

namespace {

// Marke: kräftiger Hintergrund mit dunklem Text (Kontrast unabhängig von der
// Seite); unsicherer Name gestrichelt umrandet und mit „?“.
constexpr SkColor kFill = SkColorSetRGB(0xFF, 0xD6, 0x00);
constexpr SkColor kFillUncertain = SkColorSetRGB(0xFF, 0xFF, 0xFF);
constexpr SkColor kText = SkColorSetRGB(0x10, 0x10, 0x10);
constexpr SkColor kBorder = SkColorSetRGB(0x10, 0x10, 0x10);
constexpr int kPadding = 3;

class MarksView : public views::View {
 public:
  explicit MarksView(std::vector<ReliefMarksOverlay::Box> boxes)
      : boxes_(std::move(boxes)),
        font_(gfx::FontList().DeriveWithSizeDelta(1).DeriveWithWeight(
            gfx::Font::Weight::BOLD)) {
    // Sehhilfe ohne eigene Bedeutung: Assistenztechnik liest die Seite.
    GetViewAccessibility().SetIsIgnored(true);
  }

  void OnPaint(gfx::Canvas* canvas) override {
    for (const ReliefMarksOverlay::Box& box : boxes_) {
      const std::u16string text =
          box.uncertain ? box.label + u"?" : box.label;
      const int width = gfx::Canvas::GetStringWidth(text, font_) + 2 * kPadding;
      const int height = font_.GetHeight() + 2 * kPadding;
      const gfx::Rect label(box.rect.x(), box.rect.y(), width, height);
      cc::PaintFlags fill;
      fill.setAntiAlias(true);
      fill.setColor(box.uncertain ? kFillUncertain : kFill);
      canvas->DrawRoundRect(label, 3, fill);
      cc::PaintFlags border;
      border.setAntiAlias(true);
      border.setStyle(cc::PaintFlags::kStroke_Style);
      border.setStrokeWidth(box.uncertain ? 2 : 1);
      border.setColor(kBorder);
      canvas->DrawRoundRect(label, 3, border);
      canvas->DrawStringRect(text, font_, kText,
                             gfx::Rect(label.x() + kPadding,
                                       label.y() + kPadding,
                                       width - 2 * kPadding,
                                       height - 2 * kPadding));
    }
  }

 private:
  const std::vector<ReliefMarksOverlay::Box> boxes_;
  const gfx::FontList font_;
};

}  // namespace

ReliefMarksOverlay::ReliefMarksOverlay(content::WebContents& contents)
    : contents_(contents) {}

ReliefMarksOverlay::~ReliefMarksOverlay() = default;

void ReliefMarksOverlay::Show(std::vector<Box> boxes) {
  Hide();
  views::Widget::InitParams params(
      views::Widget::InitParams::CLIENT_OWNS_WIDGET,
      views::Widget::InitParams::TYPE_POPUP);
  params.parent = contents_->GetNativeView();
  params.opacity = views::Widget::InitParams::WindowOpacity::kTranslucent;
  params.activatable = views::Widget::InitParams::Activatable::kNo;
  params.accept_events = false;
  params.bounds = contents_->GetContainerBounds();
  params.name = "ReliefMarks";
  widget_ = std::make_unique<views::Widget>();
  widget_->Init(std::move(params));
  widget_->SetContentsView(std::make_unique<MarksView>(std::move(boxes)));
  widget_->ShowInactive();
}

void ReliefMarksOverlay::Hide() {
  if (widget_) {
    widget_->CloseNow();
    widget_.reset();
  }
}

}  // namespace relief
