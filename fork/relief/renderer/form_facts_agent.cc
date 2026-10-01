// Relief. MIT-Lizenz wie das Relief-Repository.

#include "relief/renderer/form_facts_agent.h"

#include <optional>
#include <string>
#include <utility>
#include <vector>

#include "base/functional/bind.h"
#include "base/strings/string_util.h"
#include "content/public/renderer/render_frame.h"
#include "third_party/blink/public/common/associated_interfaces/associated_interface_registry.h"
#include "third_party/blink/public/mojom/forms/form_control_type.mojom-shared.h"
#include "third_party/blink/public/web/web_ax_object.h"
#include "third_party/blink/public/web/web_document.h"
#include "third_party/blink/public/web/web_element.h"
#include "third_party/blink/public/web/web_form_control_element.h"
#include "third_party/blink/public/web/web_form_element.h"
#include "third_party/blink/public/web/web_local_frame.h"

namespace relief {

namespace {

using blink::mojom::FormControlType;

// Button, der sein Formular absendet (wie facts.rs im CDP-Host: `button`
// ohne oder mit ungültigem `type`, `input type=submit|image`).
bool IsSubmit(const blink::WebFormControlElement& control) {
  const FormControlType type = control.FormControlType();
  return type == FormControlType::kButtonSubmit ||
         type == FormControlType::kInputSubmit ||
         type == FormControlType::kInputImage;
}

std::string Trimmed(const blink::WebString& value) {
  return std::string(
      base::TrimWhitespaceASCII(value.Utf8(), base::TRIM_ALL));
}

std::string Attr(const blink::WebElement& element, const char* name) {
  return Trimmed(element.GetAttribute(blink::WebString::FromUtf8(name)));
}

}  // namespace

// static
void FormFactsAgent::Create(content::RenderFrame* render_frame) {
  new FormFactsAgent(render_frame);
}

FormFactsAgent::FormFactsAgent(content::RenderFrame* render_frame)
    : content::RenderFrameObserver(render_frame) {
  render_frame->GetAssociatedInterfaceRegistry()
      ->AddInterface<mojom::FormFacts>(base::BindRepeating(
          &FormFactsAgent::Bind, base::Unretained(this)));
}

FormFactsAgent::~FormFactsAgent() = default;

void FormFactsAgent::OnDestruct() {
  delete this;
}

void FormFactsAgent::Bind(
    mojo::PendingAssociatedReceiver<mojom::FormFacts> receiver) {
  receivers_.Add(this, std::move(receiver));
}

void FormFactsAgent::Get(int32_t ax_id, GetCallback callback) {
  std::optional<std::string> action;
  std::vector<mojom::FieldFactPtr> fields;
  const blink::WebDocument document =
      render_frame()->GetWebFrame()->GetDocument();
  const blink::WebAXObject object =
      blink::WebAXObject::FromWebDocumentByID(document, ax_id);
  const blink::WebNode node = object.IsNull() ? blink::WebNode()
                                              : object.GetNode();
  const blink::WebFormControlElement control =
      node.IsNull() ? blink::WebFormControlElement()
                    : node.DynamicTo<blink::WebFormControlElement>();
  const blink::WebFormElement form =
      control.IsNull() ? blink::WebFormElement() : control.Form();
  if (!control.IsNull() && form.IsNull()) {
    // Feld ohne Formular: nur sein eigenes `autocomplete` (Paket 112).
    const std::string autocomplete = Attr(control, "autocomplete");
    if (!autocomplete.empty()) {
      fields.push_back(mojom::FieldFact::New(ax_id, autocomplete));
    }
  }
  if (!form.IsNull()) {
    std::string method = Attr(control, "formmethod");
    if (method.empty()) {
      method = Attr(form, "method");
    }
    if (IsSubmit(control) && !base::EqualsCaseInsensitiveASCII(method, "dialog")) {
      // `formaction` vor `action`; leer heißt: das Dokument selbst.
      std::string target = Attr(control, "formaction");
      if (target.empty()) {
        target = Trimmed(form.Action());
      }
      const blink::WebURL url =
          target.empty() ? document.Url()
                         : document.CompleteURL(blink::WebString::FromUtf8(
                               target));
      if (url.IsValid()) {
        action = url.GetString().Utf8();
      }
    }
    for (const blink::WebFormControlElement& field :
         form.GetFormControlElements()) {
      const std::string autocomplete = Attr(field, "autocomplete");
      const blink::WebAXObject field_object =
          blink::WebAXObject::FromWebNode(field);
      if (autocomplete.empty() || field_object.IsNull()) {
        continue;
      }
      fields.push_back(
          mojom::FieldFact::New(field_object.AxID(), autocomplete));
    }
  }
  std::move(callback).Run(
      mojom::FormInfo::New(std::move(action), std::move(fields)));
}

}  // namespace relief
