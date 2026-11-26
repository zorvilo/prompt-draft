import { desktop, errorMessage } from "@/api/desktop";
import { useWorkspace } from "@/store/modules/workspace";
import {
  contextTokenBudget,
  contextWarningRatio,
} from "@/utils/workspace/contextBudget";
import {
  conversationUsage,
  prepareConversation,
  recordReply,
} from "@/utils/workspace/conversation";
import { createDraftRequests } from "@/utils/workspace/draftRequests";
import { Info } from "lucide-vue-next";
import { useDialog, useMessage } from "naive-ui";
import { computed, h } from "vue";

import type { Utility } from "@/router/routes";
import { useProviders } from "@/views/models/hooks/useProviders";
import { useTemplates } from "@/views/templates/hooks/useTemplates";
import type { ComputedRef } from "vue";
export function useGeneration(
  workspace: ReturnType<typeof useWorkspace>,
  templates: ReturnType<typeof useTemplates>,
  providers: ReturnType<typeof useProviders>,
  utilities: ComputedRef<Utility | undefined>,
  openUtilities: (section: Utility) => void,
) {
  const { data, draft, profile, models, save } = workspace;
  const dialog = useDialog();
  const message = useMessage();
  const { selectedTemplate } = templates;
  const { selectedModel, modelOptions } = providers;

  const requests = createDraftRequests();
  const sendingDraftIds = computed(() => [...requests.keys()]);
  const hasRequests = computed(() => requests.size > 0);
  const activeRequest = computed(() =>
    draft.value ? requests.get(draft.value.id) : undefined,
  );
  const contextUsage = computed(() =>
    draft.value ? conversationUsage(draft.value) : undefined,
  );
  let budgetConfirmationOpen = false;

  async function optimize() {
    const item = draft.value;
    if (
      !item ||
      requests.has(item.id) ||
      utilities.value ||
      budgetConfirmationOpen
    )
      return;
    const template = selectedTemplate.value;
    if (!template) {
      message.info("请先选择优化模板。");
      return;
    }
    const mode = item.dialogue?.mode ?? "draft";
    const startingChat =
      mode === "chat" && !item.result && !item.messages.length;
    const input = mode === "chat" ? (item.dialogue?.chatInput ?? "") : "";
    if (startingChat && input.trim()) item.source = input.trim();
    if (!item.source.trim()) return;
    const instruction =
      mode === "draft" || startingChat ? undefined : input.trim() || undefined;
    if (mode === "chat" && !startingChat && !instruction) return;
    if (mode === "chat") {
      const reply = item.messages.at(-1);
      const start = item.messages
        .map((entry) => entry.context?.mode)
        .lastIndexOf("draft");
      const segment = item.messages.slice(Math.max(start, 0));
      if (
        reply?.inputTokens !== undefined &&
        reply.inputTokens >= contextTokenBudget &&
        !segment.some((entry) => entry.contextLimitWarningShown)
      ) {
        budgetConfirmationOpen = true;
        const proceed = await new Promise<boolean>((resolve) => {
          dialog.warning({
            title: "上下文已达到参考预算",
            content: `当前草稿上次输入已达到 ${contextTokenBudget.toLocaleString("en-US")} Token 参考预算，继续使用可能导致费用增加，或因超出模型上下文容量而请求失败。建议新建草稿继续使用，或切换到「模板优化」重新优化后再继续对话。是否继续发送？`,
            positiveText: "继续发送",
            negativeText: "取消",
            onPositiveClick: () => resolve(true),
            onNegativeClick: () => resolve(false),
            onClose: () => resolve(false),
            onMaskClick: () => resolve(false),
            onEsc: () => resolve(false),
          });
        });
        budgetConfirmationOpen = false;
        if (
          !proceed ||
          draft.value?.id !== item.id ||
          item.dialogue?.mode !== "chat" ||
          item.dialogue?.chatInput !== input ||
          utilities.value
        )
          return;
        reply.contextLimitWarningShown = true;
        reply.contextWarningShown = true;
      } else if (
        reply?.inputTokens !== undefined &&
        reply.inputTokens >= contextTokenBudget * contextWarningRatio &&
        !segment.some((entry) => entry.contextWarningShown)
      ) {
        reply.contextWarningShown = true;
        message.warning(
          `上下文已使用参考预算的 ${contextWarningRatio * 100}%。为减少历史输入和费用，建议适时新建草稿，或切换到「模板优化」重新优化后继续对话。`,
        );
      }
    }

    if (!profile.value) {
      if (!models.value.length)
        dialog.create({
          title: () =>
            h(
              "span",
              { style: { display: "flex", alignItems: "center", gap: "10px" } },
              [
                h(Info, {
                  size: 20,
                  strokeWidth: 1.7,
                  "aria-hidden": "true",
                  style: { flexShrink: 0, opacity: 0.7 },
                }),
                "请先添加供应商",
              ],
            ),
          showIcon: false,
          positiveButtonProps: { type: "primary" },
          content: "优化提示词需要使用 AI 模型，请先添加供应商并配置 API Key。",
          positiveText: "前往添加",
          negativeText: "暂不添加",
          onPositiveClick: () => openUtilities("model"),
        });
      else message.info("请选择供应商。");
      return;
    }
    if (
      !selectedModel.value ||
      !modelOptions.value.some((option) => option.value === selectedModel.value)
    ) {
      message.info("请选择本次使用的模型。");
      return;
    }
    if (!item.title.trim() || item.title === "未命名草稿")
      item.title = Array.from(item.source.trim().replace(/\s+/g, " "))
        .slice(0, 100)
        .join("");
    const activeProfile = { ...profile.value, model: selectedModel.value };
    const source = item.source;
    const sentAt = Date.now();
    const id = crypto.randomUUID();
    requests.set(item.id, {
      id,
      text: "",
      mode,
      model: activeProfile.model,
      providerName: activeProfile.name,
      templateId: template.id,
      templateName: template.name,
    });
    let sentMessage: string | undefined;
    let completed = false;
    try {
      const conversation = prepareConversation(
        item,
        template,
        instruction,
        mode,
      );
      const request = requests.get(item.id);
      if (mode === "chat") {
        sentMessage = startingChat ? source : instruction;
        if (request) request.userMessage = sentMessage;
        item.dialogue ??= {};
        item.dialogue.chatInput = "";
      } else if (request) {
        request.userMessage = source;
      }
      const available = await desktop.hasKey(activeProfile);
      if (
        !requests.isCurrent(item.id, id) ||
        !data.value.drafts.some((entry) => entry.id === item.id)
      )
        return;
      if (!available) {
        message.warning("请先为所选供应商填写 API Key。");
        openUtilities("model");
        return;
      }
      const started = performance.now();
      const reply = await desktop.optimize(
        activeProfile,
        id,
        conversation,
        mode,
        (text) => requests.append(item.id, id, text),
      );
      if (!requests.isCurrent(item.id, id)) return;
      const current = data.value.drafts.find((entry) => entry.id === item.id);
      if (!current) return;
      recordReply(
        current,
        conversation,
        reply,
        mode,
        activeProfile.model,
        activeProfile.name,
        Math.round(performance.now() - started),
        sentAt,
      );
      completed = true;
      requests.finish(item.id, id);
      await save();
    } catch (error) {
      if (completed || requests.isCurrent(item.id, id))
        message.error(
          (item.title || "未命名草稿") + "：" + errorMessage(error),
          {
            duration: 9000,
            closable: true,
          },
        );
    } finally {
      if (
        !completed &&
        sentMessage &&
        (!requests.has(item.id) || requests.isCurrent(item.id, id)) &&
        item.dialogue?.chatInput === "" &&
        data.value.drafts.some((entry) => entry.id === item.id)
      )
        item.dialogue.chatInput = sentMessage;
      requests.finish(item.id, id);
    }
  }

  async function cancel(draftId = draft.value?.id) {
    if (!draftId) return;
    const id = requests.get(draftId)?.id;
    if (!id) return;
    requests.finish(draftId, id);
    try {
      await desktop.cancel(id);
    } catch (error) {
      message.error(errorMessage(error));
    }
  }

  return {
    requests,
    sendingDraftIds,
    hasRequests,
    activeRequest,
    contextUsage,
    optimize,
    cancel,
  };
}
