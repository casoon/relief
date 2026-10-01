// Relief. MIT-Lizenz wie das Relief-Repository.

#include "relief/profile_store.h"

#include <memory>
#include <utility>

#include "base/files/file_util.h"
#include "base/functional/bind.h"
#include "base/task/sequenced_task_runner.h"
#include "base/task/thread_pool.h"
#include "content/public/browser/browser_context.h"

namespace relief {

namespace {

const char kKey[] = "relief_profile_store";

std::string Read(const base::FilePath& path) {
  std::string json;
  base::ReadFileToString(path, &json);
  return json;
}

void Write(const base::FilePath& path, const std::string& json) {
  base::CreateDirectory(path.DirName());
  base::WriteFile(path, json);
}

}  // namespace

// static
ProfileStore& ProfileStore::For(content::BrowserContext* context) {
  auto* store = static_cast<ProfileStore*>(context->GetUserData(kKey));
  if (!store) {
    auto owned = std::make_unique<ProfileStore>(context);
    store = owned.get();
    context->SetUserData(kKey, std::move(owned));
  }
  return *store;
}

ProfileStore::ProfileStore(content::BrowserContext* context)
    : file_runner_(base::ThreadPool::CreateSequencedTaskRunner(
          {base::MayBlock(), base::TaskPriority::USER_VISIBLE,
           base::TaskShutdownBehavior::BLOCK_SHUTDOWN})) {
  if (context->IsOffTheRecord()) {
    return;
  }
  path_ = context->GetPath().AppendASCII("Relief").AppendASCII(
      "faehigkeiten.json");
  file_runner_->PostTaskAndReplyWithResult(
      FROM_HERE, base::BindOnce(&Read, path_),
      base::BindOnce(&ProfileStore::Loaded, weak_factory_.GetWeakPtr()));
}

ProfileStore::~ProfileStore() = default;

void ProfileStore::Loaded(std::string json) {
  // Eine Änderung vor dem Laden gewinnt.
  if (!json_.empty() || json.empty()) {
    return;
  }
  json_ = std::move(json);
  callbacks_.Notify();
}

void ProfileStore::Set(std::string json) {
  json_ = std::move(json);
  if (!path_.empty()) {
    file_runner_->PostTask(FROM_HERE, base::BindOnce(&Write, path_, json_));
  }
  callbacks_.Notify();
}

base::CallbackListSubscription ProfileStore::Subscribe(
    base::RepeatingClosure callback) {
  return callbacks_.Add(std::move(callback));
}

}  // namespace relief
