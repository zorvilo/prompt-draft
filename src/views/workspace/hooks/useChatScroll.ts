import type { Draft, ModificationMode } from "@/api/desktop";
import { visibleMessageIds } from "@/utils/workspace/messageVisibility";
import {
  computed,
  nextTick,
  ref,
  watch,
  type ComputedRef,
  type Ref,
} from "vue";
import type { GenerationRequest } from "../types";
export function useChatScroll(
  draft: Ref<Draft>,
  mode: ComputedRef<ModificationMode>,
  props: { request?: GenerationRequest },
) {
  const messages = computed(() => draft.value.messages);
  const streamingContent = computed(() => props.request?.text ?? "");
  const chatScroll = ref<HTMLElement>();
  const visibleIds = ref<string[]>([]);
  function updateVisibleMessages() {
    const area = chatScroll.value;
    if (!area) {
      visibleIds.value = [];
      return;
    }
    const bounds = area.getBoundingClientRect();
    visibleIds.value = visibleMessageIds(
      {
        top: bounds.top + area.clientTop,
        bottom: bounds.top + area.clientTop + area.clientHeight,
      },
      Array.from(
        area.querySelectorAll<HTMLElement>(".from-user[data-message-id]"),
      ).map((element) => {
        const rect = element.getBoundingClientRect();
        return {
          id: element.dataset.messageId!,
          top: rect.top,
          bottom: rect.bottom,
        };
      }),
    );
  }
  watch(
    [chatScroll, () => draft.value.id, () => messages.value.length],
    async (_, __, onCleanup) => {
      let observer: IntersectionObserver | undefined;
      let cancelled = false;
      onCleanup(() => {
        cancelled = true;
        observer?.disconnect();
      });
      await nextTick();
      if (cancelled || !chatScroll.value) return;
      observer = new IntersectionObserver(updateVisibleMessages, {
        root: chatScroll.value,
      });
      chatScroll.value
        .querySelectorAll(".from-user[data-message-id]")
        .forEach((element) => observer!.observe(element));
      updateVisibleMessages();
    },
  );
  function jumpToMessage(id: string) {
    const area = chatScroll.value;
    const target = Array.from(
      area?.querySelectorAll<HTMLElement>("[data-message-id]") ?? [],
    ).find((element) => element.dataset.messageId === id);
    if (!area || !target) return;
    area.scrollTo({
      top:
        area.scrollTop +
        target.getBoundingClientRect().top -
        area.getBoundingClientRect().top -
        12,
      behavior: window.matchMedia("(prefers-reduced-motion: reduce)").matches
        ? "instant"
        : "smooth",
    });
    target.focus({ preventScroll: true });
  }
  watch([() => draft.value.id, mode], async () => {
    visibleIds.value = [];
    await nextTick();
    const area = chatScroll.value;
    if (area && props.request) area.scrollTop = area.scrollHeight;
    updateVisibleMessages();
  });
  watch(
    () => [
      draft.value.id,
      messages.value.length,
      props.request?.userMessage,
      streamingContent.value,
    ],
    async (values, previous) => {
      const area = chatScroll.value;
      const follow =
        !area ||
        area.scrollHeight - area.scrollTop - area.clientHeight < 64 ||
        values[0] !== previous?.[0] ||
        (!!values[2] && values[2] !== previous?.[2]);
      await nextTick();
      if (follow)
        chatScroll.value?.scrollTo({ top: chatScroll.value.scrollHeight });
      updateVisibleMessages();
    },
  );
  return { chatScroll, visibleIds, updateVisibleMessages, jumpToMessage };
}
