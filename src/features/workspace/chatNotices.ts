import type { ChatMessage, ModificationMode } from "../../lib/tauri";

export interface ChatNotice {
  kind: "optimization" | "provider" | "model" | "template";
  text: string;
  detail: string;
}

export function chatNotices(
  previous: Pick<ChatMessage, "model" | "providerName" | "context"> | undefined,
  current: {
    mode?: ModificationMode;
    model?: string;
    providerName?: string;
    templateId?: string;
    templateName?: string;
  },
): ChatNotice[] {
  const notices: ChatNotice[] = [];
  if (current.mode === "draft")
    notices.push({
      kind: "optimization",
      text: "已重新优化，后续对话从这里开始",
      detail:
        "本轮使用所选模板重新优化。后续消息仅携带从本轮开始的对话，之前的聊天记录仍保留。",
    });
  if (
    previous?.providerName &&
    current.providerName &&
    previous.providerName !== current.providerName
  )
    notices.push({
      kind: "provider",
      text: `供应商已从 ${previous.providerName} 更改为 ${current.providerName}`,
      detail: "本条消息起使用新的供应商。切换供应商不会单独清空对话上下文。",
    });
  const templateChanged =
    previous?.context?.templateId &&
    current.templateId &&
    previous.context.templateId !== current.templateId;
  if (templateChanged)
    notices.push({
      kind: "template",
      text: `模板已从 ${previous.context?.templateName ?? "原模板"} 更改为 ${current.templateName ?? "新模板"}`,
      detail:
        "本条消息起使用新的模板。连续对话会保留当前对话上下文，按照新模板继续处理。",
    });
  if (previous?.model && current.model && previous.model !== current.model)
    notices.push({
      kind: "model",
      text: `模型已从 ${previous.model} 更改为 ${current.model}`,
      detail: "本条消息起使用新的模型。切换模型不会单独清空对话上下文。",
    });
  if (notices.length > 1) {
    const configuration = [
      templateChanged ? current.templateName : undefined,
      current.providerName,
      current.model,
    ]
      .filter(Boolean)
      .join(" · ");
    return [
      {
        kind: current.mode === "draft" ? "optimization" : "model",
        text:
          current.mode === "draft"
            ? `已重新优化 · ${configuration}`
            : `已切换至 ${configuration}`,
        detail: notices
          .map((notice) => `${notice.text}。${notice.detail}`)
          .join("\n"),
      },
    ];
  }
  return notices;
}
