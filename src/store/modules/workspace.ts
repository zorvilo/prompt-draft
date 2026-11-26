import { defineStore, storeToRefs } from "pinia";
import { computed, ref, watch } from "vue";
import {
  desktop,
  errorMessage,
  type Draft,
  type Workspace,
  type WorkspaceChanges,
} from "../../api/desktop";
import { configuredModels } from "../../config/modelPresets";
import { workspaceChanges } from "./workspaceChanges";

export const useWorkspaceStore = defineStore("workspace", () => {
  const data = ref<Workspace>({
    schema: 1,
    projects: [{ id: "default", name: "我的草稿" }],
    drafts: [],
    profiles: [],
    activeDraft: null,
    activeProfile: null,
    theme: "system",
  });
  const ready = ref(false);
  const startupError = ref("");
  const saveError = ref("");
  const draft = computed(() =>
    data.value.drafts.find((item) => item.id === data.value.activeDraft),
  );
  const models = computed(() => configuredModels(data.value.profiles));
  const profile = computed(() =>
    models.value.find((item) => item.id === draft.value?.providerId),
  );
  let timer: ReturnType<typeof setTimeout>;
  let write = Promise.resolve();
  let lastSaved = "";

  function save(persist?: (changes: WorkspaceChanges) => Promise<void>) {
    clearTimeout(timer);
    const document = JSON.stringify(data.value);
    const task = write
      .then(async () => {
        if (document !== lastSaved || persist) {
          await (persist ?? desktop.saveChanges)(
            workspaceChanges(
              lastSaved ? (JSON.parse(lastSaved) as Workspace) : undefined,
              JSON.parse(document) as Workspace,
            ),
          );
          lastSaved = document;
        }
        saveError.value = "";
        return true;
      })
      .catch((error: unknown) => {
        saveError.value = errorMessage(error);
        return false;
      });
    write = task.then(() => {});
    return task;
  }

  function openDraft(id: string) {
    const item = data.value.drafts.find((entry) => entry.id === id);
    if (!item) return;
    data.value.activeDraft = id;
  }

  function newDraft(projectId: string | null = null, source = "", title = "") {
    const item: Draft = {
      id: crypto.randomUUID(),
      projectId,
      title,
      source,
      templateId: data.value.defaultTemplateId ?? "prompt-general",
      result: "",

      favorite: false,
      pinned: false,
      opened: Date.now(),
      messages: [],
    };
    data.value.drafts.push(item);
    openDraft(item.id);
  }

  watch(
    [
      () => draft.value,
      () => draft.value?.title,
      () => draft.value?.source,
      () => draft.value?.result,
    ],
    ([current], [previous]) => {
      if (current && current === previous) current.updated = Date.now();
    },
    { flush: "sync" },
  );

  watch(
    data,
    () => {
      if (!ready.value) return;
      clearTimeout(timer);
      timer = setTimeout(() => void save(), 250);
    },
    { deep: true, flush: "sync" },
  );

  async function initialize() {
    startupError.value = "";
    try {
      const saved = await desktop.load();
      if (saved) data.value = JSON.parse(saved) as Workspace;
      lastSaved = saved ? JSON.stringify(data.value) : "";
      ready.value = true;
      if (!saved) await save();
    } catch (error) {
      startupError.value = errorMessage(error);
    }
  }

  async function restoreBackup(path: string) {
    clearTimeout(timer);
    await write;
    clearTimeout(timer);
    ready.value = false;
    try {
      await desktop.restoreBackup(path);
      // Reload all mounted views so no stale settings or queued edits overwrite the restored data.
      window.location.reload();
    } catch (error) {
      ready.value = true;
      throw error;
    }
  }

  return {
    data,
    ready,
    startupError,
    saveError,
    draft,
    profile,
    models,
    newDraft,
    openDraft,
    initialize,
    restoreBackup,
    save,
  };
});

export function useWorkspace() {
  const store = useWorkspaceStore();
  const { data, ready, startupError, saveError, draft, profile, models } =
    storeToRefs(store);
  return {
    data,
    ready,
    startupError,
    saveError,
    draft,
    profile,
    models,
    newDraft: store.newDraft,
    openDraft: store.openDraft,
    initialize: store.initialize,
    restoreBackup: store.restoreBackup,
    save: store.save,
  };
}
