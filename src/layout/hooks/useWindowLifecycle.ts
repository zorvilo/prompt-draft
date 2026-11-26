import { desktop, errorMessage } from "@/api/desktop";
import { defaultShortcuts, shortcutFromEvent } from "@/config/shortcuts";
import { useWorkspace } from "@/store/modules/workspace";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useMessage } from "naive-ui";
import { onMounted, onUnmounted, ref } from "vue";

import type { Utility } from "@/router/routes";
import { useGeneration } from "@/views/workspace/hooks/useGeneration";
import type { ComputedRef } from "vue";
import { useDraftActions } from "./useDraftActions";
export function useWindowLifecycle(
  workspace: ReturnType<typeof useWorkspace>,
  generation: ReturnType<typeof useGeneration>,
  actions: ReturnType<typeof useDraftActions>,
  utilities: ComputedRef<Utility | undefined>,
) {
  const { data, draft, saveError, save } = workspace;
  const message = useMessage();
  const { requests, hasRequests, cancel, optimize } = generation;
  const {
    create,
    closeDraftMenu,
    renameDraft,
    togglePin,
    deleteDraft,
    copyDraft,
  } = actions;
  const showClose = ref(false);
  const rememberClose = ref(false);
  const closing = ref(false);
  async function completeClose(action: "hide" | "exit") {
    if (closing.value) return;
    closing.value = true;
    const previous = data.value.closeBehavior;
    try {
      if (rememberClose.value) data.value.closeBehavior = action;
      await save();
      if (saveError.value) throw new Error(saveError.value);
      if (action === "exit")
        await Promise.all([...requests.keys()].map((id) => cancel(id)));
      await desktop.windowAction(action);
      showClose.value = false;
    } catch (error) {
      data.value.closeBehavior = previous;
      message.error(errorMessage(error));
    } finally {
      closing.value = false;
    }
  }
  function requestClose(forceExit = false) {
    if (showClose.value || closing.value) return;
    rememberClose.value = false;
    const action = forceExit ? "exit" : (data.value.closeBehavior ?? "ask");
    if (action === "ask" || (action === "exit" && hasRequests.value)) {
      showClose.value = true;
    } else {
      void completeClose(action);
    }
  }
  function shortcut(event: KeyboardEvent) {
    if (
      event.defaultPrevented ||
      event.isComposing ||
      event.keyCode === 229 ||
      showClose.value ||
      event.repeat ||
      document.querySelector(".n-modal, .n-dialog")
    )
      return;
    const binding = shortcutFromEvent(event);
    const shortcuts = { ...defaultShortcuts, ...data.value.shortcuts };
    if (binding === shortcuts.newline || binding === shortcuts.newlineAlternate)
      return;
    const target = event.target as HTMLElement | null;
    if (
      !event.ctrlKey &&
      !event.altKey &&
      !event.metaKey &&
      target?.closest(
        'input, textarea, [contenteditable="true"], [role="textbox"]',
      )
    )
      return;
    if (binding === shortcuts.newDraft) {
      event.preventDefault();
      create();
    }
    if (
      binding === shortcuts.optimize &&
      binding !== "Enter" &&
      draft.value?.dialogue?.mode !== "chat"
    ) {
      event.preventDefault();
      void optimize();
    }
    const item = draft.value;
    if (!item || utilities.value) return;
    const action = Object.entries(shortcuts).find(
      ([, key]) => key === binding,
    )?.[0];
    if (!action || action === "newDraft" || action === "optimize") return;
    event.preventDefault();
    closeDraftMenu();
    switch (action) {
      case "rename":
        renameDraft(item.id, item.title);
        break;
      case "pin":
        togglePin(item.id);
        break;
      case "favorite":
        item.favorite = !item.favorite;
        break;
      case "delete":
        deleteDraft(item.id);
        break;
      case "copyResult":
        copyDraft(item);
        break;
    }
  }
  let disposed = false;
  const windowListeners: (() => void)[] = [];
  async function registerWindowEvents() {
    try {
      const window = getCurrentWindow();
      const keep = (remove: () => void) => {
        if (disposed) remove();
        else windowListeners.push(remove);
      };
      keep(
        await window.onCloseRequested((event) => {
          event.preventDefault();
          requestClose();
        }),
      );
      keep(await window.listen("request-exit", () => requestClose(true)));
    } catch (error) {
      message.error(errorMessage(error));
    }
  }
  onMounted(() => {
    window.addEventListener("keydown", shortcut);
    window.addEventListener("contextmenu", closeDraftMenu);
    void registerWindowEvents();
  });
  onUnmounted(() => {
    disposed = true;
    windowListeners.forEach((remove) => remove());
    window.removeEventListener("keydown", shortcut);
    window.removeEventListener("contextmenu", closeDraftMenu);
  });

  return { showClose, rememberClose, closing, completeClose };
}
