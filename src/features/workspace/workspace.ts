import { workspaceChanges } from "./workspaceChanges";
import { configuredModels } from "../settings/modelPresets";
import { computed, ref, watch } from "vue";
import {
  desktop,
  errorMessage,
  type Draft,
  type Workspace,
} from "../../lib/tauri";

export function useWorkspace() {
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
    models.value.find((item) => item.id === data.value.activeProfile),
  );
  let timer: ReturnType<typeof setTimeout>;
  let write = Promise.resolve();
  let lastSaved = "";

  function save() {
    clearTimeout(timer);
    const document = JSON.stringify(data.value);
    write = write
      .then(async () => {
        if (document !== lastSaved) {
          await desktop.saveChanges(
            workspaceChanges(
              lastSaved ? (JSON.parse(lastSaved) as Workspace) : undefined,
              JSON.parse(document) as Workspace,
            ),
          );
          lastSaved = document;
        }
        saveError.value = "";
      })
      .catch((error: unknown) => {
        saveError.value = errorMessage(error);
      });
    return write;
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
      result: "",
      question: null,
      favorite: false,
      pinned: false,
      opened: Date.now(),
      versions: [],
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

  async function replace(document: string) {
    clearTimeout(timer);
    await write;
    await desktop.save(document);
    data.value = JSON.parse(document) as Workspace;
    lastSaved = JSON.stringify(data.value);
    await save();
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
    replace,
    save,
  };
}
