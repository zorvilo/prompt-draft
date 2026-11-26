import { desktop, errorMessage, type Workspace } from "@/api/desktop";
import { useWorkspace } from "@/store/modules/workspace";
import type { Utility } from "@/router/routes";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { save as pickSave } from "@tauri-apps/plugin-dialog";
import { renderMarkdown } from "@/utils/markdown";
import { useMessage } from "naive-ui";
import { computed, nextTick, onMounted, ref, watch } from "vue";
import { useAppUpdate } from "./useAppUpdate";

import { useProviders } from "@/views/models/hooks/useProviders";
import { useBackup } from "@/views/settings/hooks/useBackup";
import { useTemplates } from "@/views/templates/hooks/useTemplates";
import { useGeneration } from "@/views/workspace/hooks/useGeneration";
import { useRoute, useRouter } from "vue-router";
import { useDraftActions } from "./useDraftActions";
import { useWindowLifecycle } from "./useWindowLifecycle";

export function useWorkspaceShell() {
  const workspace = useWorkspace();
  const { data, draft, saveError, save } = workspace;
  const draftProject = computed(() =>
    data.value.projects.find(
      (project) => project.id === draft.value?.projectId,
    ),
  );
  const message = useMessage();
  const templates = useTemplates(workspace);
  const { libraryCategory, editingTemplate } = templates;
  const providers = useProviders(workspace, templates);
  const usageCatalog = ref(false);
  const route = useRoute();
  const router = useRouter();
  const utilities = computed<Utility | undefined>({
    get: () => route.meta.utility as Utility | undefined,
    set: (section) => {
      void router.push({ name: section ?? "workspace" });
    },
  });
  watch(utilities, () => {
    usageCatalog.value = false;
  });
  const utilitiesTitle = computed(() => String(route.meta.title ?? ""));
  const updateAboutRequest = ref(0);
  function openUtilities(section: NonNullable<typeof utilities.value>) {
    showLibrary.value = false;
    editingTemplate.value = undefined;
    libraryCategory.value = undefined;
    updateAboutRequest.value = 0;
    utilities.value = section;
  }
  async function openUpdateAbout() {
    openUtilities("settings");
    await nextTick();
    updateAboutRequest.value += 1;
  }
  const showLibrary = ref(false);
  const uiDefaults = {
    recentExpanded: true,
    pinnedExpanded: true,
    expandedProjects: [] as string[],
    draftListMode: "recent" as const,
  };
  function uiSetting<K extends keyof NonNullable<Workspace["uiState"]>>(
    key: K,
  ) {
    return computed({
      get: () => data.value.uiState?.[key] ?? uiDefaults[key],
      set: (value: NonNullable<Workspace["uiState"]>[K]) => {
        data.value.uiState = {
          ...uiDefaults,
          ...data.value.uiState,
          [key]: value,
        };
      },
    });
  }
  const recentExpanded = uiSetting("recentExpanded");
  const pinnedExpanded = uiSetting("pinnedExpanded");
  const expandedProjects = uiSetting("expandedProjects");
  const draftListMode = uiSetting("draftListMode");
  const resultModes = [
    { value: "preview", label: "预览" },
    { value: "source", label: "编辑" },
    { value: "compare", label: "对比" },
  ] as const;
  const showResult = ref(false);
  const keyboardNavigation = ref(false);
  const resultMode = ref<(typeof resultModes)[number]["value"]>("preview");
  const resultPreview = computed(() =>
    resultMode.value === "preview"
      ? renderMarkdown(draft.value?.result ?? "")
      : "",
  );
  watch(
    () => draft.value?.id,
    () => {
      resultMode.value = "preview";
      showResult.value = false;
    },
  );
  const generation = useGeneration(
    workspace,
    templates,
    providers,
    utilities,
    openUtilities,
  );
  const { hasRequests } = generation;
  const actions = useDraftActions(
    workspace,
    utilities,
    showLibrary,
    expandedProjects,
    generation.cancel,
  );
  const backups = useBackup(workspace, hasRequests);
  const lifecycle = useWindowLifecycle(
    workspace,
    generation,
    actions,
    utilities,
  );
  const comparisonSource = computed(() => draft.value?.source ?? "");
  async function setUpdateSource(value: Workspace["updateSource"]) {
    const previous = data.value.updateSource;
    data.value.updateSource = value;
    await save();
    if (saveError.value) {
      data.value.updateSource = previous;
      throw new Error(saveError.value);
    }
  }
  const appUpdate = useAppUpdate({
    getSource: () => data.value.updateSource,
    setSource: setUpdateSource,
    prepareUpdate,
  });
  onMounted(() => {
    void appUpdate.checkUpdate(true);
  });
  async function prepareUpdate() {
    if (hasRequests.value) throw new Error("请等待模型请求结束后再安装更新。");
    await save();
    if (saveError.value) throw new Error(saveError.value);
  }
  async function copyText(content: string) {
    try {
      await writeText(content);
      message.success("已复制");
    } catch (error) {
      message.error(errorMessage(error));
    }
  }

  async function copy() {
    if (!draft.value?.result) return;
    try {
      await writeText(draft.value.result);
      message.success("已复制优化稿");
    } catch (error) {
      message.error(errorMessage(error));
    }
  }

  async function exportFile(format: string, content = draft.value?.result) {
    const item = draft.value;
    if (!item || !content) return;
    const filename = item.title.replace(/[<>:"/\\|?*]/g, "_") || "词稿";
    const path = await pickSave({
      defaultPath: filename + "." + format,
      filters: [
        { name: format === "md" ? "Markdown" : "纯文本", extensions: [format] },
      ],
    });
    if (!path) return;
    await desktop.exportFile(path, content);
    message.success("已导出");
  }

  async function updateShortcuts(value: NonNullable<Workspace["shortcuts"]>) {
    const previous = data.value.shortcuts;
    data.value.shortcuts = value;
    await save();
    if (saveError.value) {
      data.value.shortcuts = previous;
      message.error(saveError.value);
      return false;
    }
    return true;
  }

  return {
    ...workspace,
    ...templates,
    ...providers,
    ...generation,
    ...actions,
    ...backups,
    ...lifecycle,
    appUpdate,
    draftProject,
    usageCatalog,
    utilities,
    utilitiesTitle,
    updateAboutRequest,
    openUtilities,
    openUpdateAbout,
    showLibrary,
    recentExpanded,
    pinnedExpanded,
    expandedProjects,
    draftListMode,
    showResult,
    keyboardNavigation,
    resultMode,
    renderMarkdown,
    resultPreview,
    comparisonSource,
    copyText,
    copy,
    exportFile,
    updateShortcuts,
  };
}
