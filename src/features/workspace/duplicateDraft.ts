import type { Draft } from "../../lib/tauri";

export function duplicateDraft(item: Draft): Draft {
  return {
    id: crypto.randomUUID(),
    projectId: item.projectId,
    title: (item.title || "未命名草稿").slice(0, 97) + " 副本",
    source: item.source,
    templateId: item.templateId,
    providerId: item.providerId,
    model: item.model,
    result: item.result,

    favorite: false,
    pinned: false,
    opened: Date.now(),
    messages: [],
  };
}
