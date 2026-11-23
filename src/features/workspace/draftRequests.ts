import { reactive } from "vue";
import type { ModificationMode } from "../../lib/tauri";

export function createDraftRequests() {
  const requests = reactive(
    new Map<
      string,
      {
        id: string;
        text: string;
        userMessage?: string;
        mode?: ModificationMode;
        model?: string;
        providerName?: string;
        templateId?: string;
        templateName?: string;
      }
    >(),
  );
  return Object.assign(requests, {
    isCurrent(draftId: string, requestId: string) {
      return requests.get(draftId)?.id === requestId;
    },
    append(draftId: string, requestId: string, text: string) {
      const request = requests.get(draftId);
      if (request?.id === requestId) request.text += text;
    },
    finish(draftId: string, requestId: string) {
      if (requests.get(draftId)?.id === requestId) requests.delete(draftId);
    },
  });
}
