// Relief. MIT-Lizenz wie das Relief-Repository.

#ifndef RELIEF_RENDERER_FORM_FACTS_AGENT_H_
#define RELIEF_RENDERER_FORM_FACTS_AGENT_H_

#include "content/public/renderer/render_frame_observer.h"
#include "mojo/public/cpp/bindings/associated_receiver_set.h"
#include "mojo/public/cpp/bindings/pending_associated_receiver.h"
#include "relief/common/form_facts.mojom.h"

namespace relief {

// Beantwortet `relief.mojom.FormFacts` für einen Frame (Paket 75). Lebt so
// lange wie der RenderFrame; ohne Anfrage tut er nichts.
class FormFactsAgent : public content::RenderFrameObserver,
                       public mojom::FormFacts {
 public:
  static void Create(content::RenderFrame* render_frame);

  FormFactsAgent(const FormFactsAgent&) = delete;
  FormFactsAgent& operator=(const FormFactsAgent&) = delete;

  // mojom::FormFacts:
  void Get(int32_t ax_id, GetCallback callback) override;

 private:
  explicit FormFactsAgent(content::RenderFrame* render_frame);
  ~FormFactsAgent() override;

  // content::RenderFrameObserver:
  void OnDestruct() override;

  void Bind(mojo::PendingAssociatedReceiver<mojom::FormFacts> receiver);

  mojo::AssociatedReceiverSet<mojom::FormFacts> receivers_;
};

}  // namespace relief

#endif  // RELIEF_RENDERER_FORM_FACTS_AGENT_H_
