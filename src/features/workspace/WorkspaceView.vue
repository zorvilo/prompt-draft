<script setup lang="ts">
import { computed, h, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import {
  ArrowLeft,
  ChevronRight,
  Copy,
  Download,
  FileText,
  Folder,
  History,
  Maximize2,
  Menu,
  Pencil,
  Pin,
  PinOff,
  Plus,
  Send,
  Square,
  Star,
  Trash2,
} from "lucide-vue-next";
import {
  NAlert,
  NButton,
  NButtonGroup,
  NCheckbox,
  NDrawer,
  NDrawerContent,
  NDropdown,
  NForm,
  NFormItem,
  NInput,
  NModal,
  NSelect,
  NSpin,
  useDialog,
  useMessage,
  type DropdownOption,
} from "naive-ui";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { defaultShortcuts, shortcutFromEvent } from "../settings/shortcuts";
import { diffChars } from "diff";
import MarkdownIt from "markdown-it";
import { open, save as pickSave } from "@tauri-apps/plugin-dialog";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import WindowTitlebar from "../../components/WindowTitlebar.vue";
import LibraryPanel from "../library/LibraryPanel.vue";
import ProjectsPanel from "../library/ProjectsPanel.vue";
import UsagePanel from "../settings/UsagePanel.vue";
import SettingsPanel from "../settings/SettingsPanel.vue";
import { modelsFor } from "../settings/modelPresets";
import {
  availableTemplates,
  builtinCategories,
  isBuiltinCategory,
  isBuiltinTemplate,
} from "../library/templates";
import { renderTemplate } from "../library/renderTemplate";
import {
  desktop,
  errorMessage,
  type PromptTemplate,
  type TemplateCategory,
  type Profile,
  type Version,
  type Workspace,
} from "../../lib/tauri";
import ResultComparison from "./ResultComparison.vue";
import type { useWorkspace } from "./workspace";

const { workspace } = defineProps<{
  workspace: ReturnType<typeof useWorkspace>;
}>();
const {
  data,
  draft,
  profile,
  models,
  saveError,
  newDraft,
  openDraft,
  replace,
  save,
} = workspace;
const dialog = useDialog();
const message = useMessage();
const usageCatalog = ref(false);
const utilities = ref<
  "projects" | "model" | "settings" | "templates" | "usage"
>();
watch(utilities, () => {
  usageCatalog.value = false;
});
const libraryCategory = ref<TemplateCategory>();
const libraryCategories = computed(() => [
  ...builtinCategories,
  ...(data.value.templateCategories ?? [])
    .filter((item) => !isBuiltinCategory(item.id))
    .map((item) => ({
      label: item.name,
      value: item.id,
      description: "自定义提示词模板。",
    })),
]);
const categoryOptions = computed(() =>
  libraryCategories.value.filter((item) => item.value !== "iteration"),
);
const utilityLabels = {
  projects: "项目",
  model: "模型",
  templates: "模板",
  settings: "设置",
  usage: "用量",
};
const utilitiesTitle = computed(() =>
  utilities.value ? utilityLabels[utilities.value] : "",
);
function openUtilities(section: NonNullable<typeof utilities.value>) {
  showLibrary.value = false;
  editingTemplate.value = undefined;
  libraryCategory.value = undefined;
  utilities.value = section;
}
const showHistory = ref(false);
const showLibrary = ref(false);
const uiDefaults = {
  recentExpanded: true,
  pinnedExpanded: true,
  expandedProjects: [] as string[],
  draftListMode: "recent" as const,
};
function uiSetting<K extends keyof NonNullable<Workspace["uiState"]>>(key: K) {
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
  { value: "source", label: "原文" },
  { value: "compare", label: "对比" },
] as const;
const showResult = ref(false);
const resultMode = ref<(typeof resultModes)[number]["value"]>("preview");
const markdown = new MarkdownIt({ html: false, breaks: true });
markdown.renderer.rules.link_open = () => "<span>";
markdown.renderer.rules.link_close = () => "</span>";
markdown.renderer.rules.image = (tokens, index) =>
  markdown.utils.escapeHtml(tokens[index].content);
const resultPreview = computed(() =>
  resultMode.value === "preview"
    ? markdown.render(draft.value?.result ?? "")
    : "",
);
const selectedVersion = ref<Version>();
const request = ref<{ id: string; draftId: string; text: string }>();
const activeRequest = computed(() =>
  request.value?.draftId === draft.value?.id ? request.value : undefined,
);
const streamOutput = ref<HTMLPreElement>();
watch(
  () => activeRequest.value?.text,
  async () => {
    const output = streamOutput.value;
    const follow =
      !output ||
      output.scrollHeight - output.clientHeight - output.scrollTop < 32;
    if (!follow) return;
    await nextTick();
    if (streamOutput.value)
      streamOutput.value.scrollTop = streamOutput.value.scrollHeight;
  },
);
const templateList = computed(() => availableTemplates(data.value));
const draftCategory = ref<Exclude<TemplateCategory, "iteration">>("basic");
watch(
  categoryOptions,
  (items) => {
    if (!items.some((item) => item.value === draftCategory.value))
      draftCategory.value = items[0]?.value ?? "";
  },
  { immediate: true },
);
const templateId = ref<string>();
const iterationTemplateId = ref<string>();
const iterationTemplates = computed(() =>
  templateList.value.filter((item) => item.category === "iteration"),
);
const iterationOptions = computed(() =>
  iterationTemplates.value.map((item) => ({
    label: item.name,
    value: item.id,
  })),
);
const iterationTemplate = computed(
  () =>
    iterationTemplates.value.find(
      (item) => item.id === iterationTemplateId.value,
    ) ?? iterationTemplates.value[0],
);
const draftTemplates = computed(() =>
  templateList.value.filter(
    (item) => (item.category ?? "basic") === draftCategory.value,
  ),
);
const libraryTemplates = computed(() =>
  templateList.value.filter(
    (item) => (item.category ?? "basic") === libraryCategory.value,
  ),
);
const selectedTemplate = computed(
  () =>
    draftTemplates.value.find((item) => item.id === templateId.value) ??
    draftTemplates.value[0],
);
const templateOptions = computed(() =>
  draftTemplates.value.map((item) => ({ label: item.name, value: item.id })),
);
const editingTemplate = ref<PromptTemplate>();
const viewingBuiltin = computed(
  () => !!editingTemplate.value && isBuiltinTemplate(editingTemplate.value.id),
);
const templateFields = [
  {
    key: "content",
    id: "template-content",
    role: "system",
    label: "系统提示词内容",
    placeholder: "",
  },
  {
    key: "userContent",
    id: "template-user-content",
    role: "user",
    label: "用户提示词内容",
    placeholder: "留空使用默认用户消息。填写 {{{originalPrompt}}} 可插入原稿。",
  },
] as const;
const templatePreviewOpen = ref({ system: false, user: false });
watch(editingTemplate, () => {
  templatePreviewOpen.value = { system: false, user: false };
});
const templatePreview = ref<
  Awaited<ReturnType<typeof desktop.previewTemplate>>
>([]);
const templatePreviewError = ref("");
const iterationInstruction =
  "对照原稿检查上一版优化稿，补齐遗漏的要求并完善表达。";
watch(
  [
    () => editingTemplate.value?.content,
    () => editingTemplate.value?.strategy,
    () => editingTemplate.value?.userContent,
  ],
  async ([content, strategy, userContent], _, onCleanup) => {
    templatePreviewError.value = "";
    if (!content?.trim() || !strategy) {
      templatePreview.value = [];
      return;
    }

    let current = true;
    onCleanup(() => {
      current = false;
    });
    try {
      const source = "Write a story";
      const revision =
        strategy === "iterate"
          ? {
              previousResult: "Write a creative story about space exploration",
              instruction: iterationInstruction,
            }
          : undefined;
      const messages = await desktop.previewTemplate(
        source,
        strategy,
        renderTemplate(content, source, revision),
        revision,
        userContent?.trim()
          ? renderTemplate(userContent, source, revision)
          : undefined,
      );
      if (current) templatePreview.value = messages;
    } catch (error) {
      if (!current) return;
      templatePreview.value = [];
      templatePreviewError.value = errorMessage(error);
    }
  },
);
const followup = ref("");
watch([() => draft.value?.id, () => selectedTemplate.value?.id], () => {
  followup.value = "";
});
watch(
  () => draft.value?.id,
  () => {
    resultMode.value = "preview";
    showResult.value = false;
  },
);
const selectedModel = ref<string | null>(null);
const modelOptions = computed(() =>
  (profile.value ? modelsFor(profile.value) : []).map((model) => ({
    label: model,
    value: model,
  })),
);
watch(
  [() => profile.value?.id, () => profile.value?.model],
  ([, defaultModel]) => {
    selectedModel.value = defaultModel ?? null;
  },
  { immediate: true },
);
const sourceCount = computed(
  () => Array.from(draft.value?.source ?? "").length,
);
const changes = computed(() => {
  if (resultMode.value !== "compare" || !draft.value) return [];
  return diffChars(draft.value.source, draft.value.result, { timeout: 150 });
});
const drawerWidth = computed(() => "min(100vw, 490px)");

function create(projectId?: string) {
  utilities.value = undefined;
  newDraft(projectId);
  showLibrary.value = false;
}

function editTemplateCategory(category?: { value: string; label: string }) {
  if (category && isBuiltinCategory(category.value)) return;
  const name = ref(category?.label ?? "");
  const modal = dialog.create({
    title: category ? "编辑模板分类" : "添加模板分类",
    content: () =>
      h(NInput, {
        value: name.value,
        maxlength: 30,
        autofocus: true,
        placeholder: "输入分类名称",
        "aria-label": "分类名称",
        "onUpdate:value": (value: string) => {
          name.value = value;
        },
      }),
    positiveText: category ? "保存" : "添加",
    negativeText: "取消",
    onPositiveClick: async () => {
      const label = name.value.trim();
      if (
        !label ||
        libraryCategories.value.some(
          (item) => item.label === label && item.value !== category?.value,
        )
      ) {
        message.warning(label ? "该分类名称已存在" : "请输入分类名称");
        return false;
      }
      const previous = data.value.templateCategories;
      const id = category?.value ?? crypto.randomUUID();
      data.value.templateCategories = [
        ...(previous ?? []).filter((entry) => entry.id !== id),
        { id, name: label },
      ];
      await save();
      if (saveError.value) {
        data.value.templateCategories = previous;
        message.error(saveError.value);
        return false;
      }
      message.success(category ? "分类已更新" : "分类已添加");
      modal.destroy();
    },
  });
}
function deleteTemplateCategory(category: { value: string; label: string }) {
  if (isBuiltinCategory(category.value)) return;
  const count = templateList.value.filter(
    (item) => (item.category ?? "basic") === category.value,
  ).length;
  confirm(
    "删除模板分类",
    "将删除「" +
      category.label +
      "」分类及其中的 " +
      count +
      " 个模板，草稿内容和历史版本不受影响。此操作不可撤销。",
    async () => {
      const previous = {
        templates: data.value.templates,
        categories: data.value.templateCategories,
      };
      data.value.templates = (data.value.templates ?? []).filter(
        (item) => (item.category ?? "basic") !== category.value,
      );
      data.value.templateCategories = data.value.templateCategories?.filter(
        (item) => item.id !== category.value,
      );
      await save();
      if (saveError.value) {
        data.value.templates = previous.templates;
        data.value.templateCategories = previous.categories;
        throw new Error(saveError.value);
      }
      message.success("分类及其中的模板已删除");
    },
    "删除",
  );
}

function newTemplate() {
  editingTemplate.value = {
    id: crypto.randomUUID(),
    strategy: libraryCategory.value === "iteration" ? "iterate" : "general",
    category: libraryCategory.value,
    name: "",
    content: "",
    description: "",
  };
}

async function saveTemplate() {
  const editing = editingTemplate.value;
  if (!editing || isBuiltinTemplate(editing.id)) return;
  const item = {
    ...editing,
    name: editing.name.trim(),
    content: editing.content.trim(),
    description: editing.description.trim(),
  };
  if (item.category === "iteration" && !item.userContent?.trim()) {
    message.warning("请填写迭代模板的用户提示词内容。");
    return;
  }
  if (!item.name || !item.content) {
    message.warning("请填写提示词名称和内容。");
    return;
  }
  const userTemplates = data.value.templates ?? [];
  const exists = userTemplates.some((entry) => entry.id === item.id);
  data.value.templates = exists
    ? userTemplates.map((entry) => (entry.id === item.id ? item : entry))
    : [...userTemplates, item];
  editingTemplate.value = undefined;
  await save();
  if (!saveError.value) message.success("模板已保存");
}

async function copyTemplate(item: PromptTemplate) {
  if (isBuiltinTemplate(item.id)) return;
  data.value.templates = [
    ...(data.value.templates ?? []),
    {
      ...item,
      id: crypto.randomUUID(),
      name: item.name.slice(0, 90) + "（副本）",
    },
  ];
  await save();
  if (!saveError.value) message.success("已创建模板副本");
}

function deleteTemplate(item: PromptTemplate) {
  if (isBuiltinTemplate(item.id)) return;
  confirm(
    "删除模板",
    "确定删除「" + item.name + "」？",
    async () => {
      data.value.templates = (data.value.templates ?? []).filter(
        (entry) => entry.id !== item.id,
      );
      await save();
      if (!saveError.value) message.success("模板已删除");
    },
    "删除",
  );
}

async function saveProfile(profile: Profile) {
  const index = data.value.profiles.findIndex((item) => item.id === profile.id);
  if (index === -1) data.value.profiles.push(profile);
  else data.value.profiles[index] = profile;
  await save();
  if (!saveError.value) message.success("供应商配置已保存");
}
function removeProfile(item: Profile) {
  confirm(
    "删除供应商",
    "删除「" + item.name + "」并清除已保存的密钥？",
    async () => {
      if (data.value.activeProfile === item.id) data.value.activeProfile = null;
      data.value.profiles = data.value.profiles.filter(
        (entry) => entry.id !== item.id,
      );
      await save();
      if (!saveError.value) message.success("供应商已删除");
    },
    "删除",
  );
}

function loadDraft(id: string) {
  utilities.value = undefined;
  openDraft(id);
  showLibrary.value = false;
}

function addProject() {
  const name = ref("");
  dialog.create({
    title: "新建项目",
    positiveText: "创建",
    negativeText: "取消",
    showIcon: false,
    content: () =>
      h(NInput, {
        value: name.value,
        "onUpdate:value": (value) => {
          name.value = value;
        },
        placeholder: "项目名称",
        maxlength: 60,
        inputProps: { "aria-label": "项目名称" },
      }),
    onPositiveClick: () => {
      if (!name.value.trim()) {
        message.error("请输入项目名称");
        return false;
      }
      const project = { id: crypto.randomUUID(), name: name.value.trim() };
      data.value.projects.push(project);
      expandedProjects.value = [...expandedProjects.value, project.id];
    },
  });
}

function deleteProject(id: string) {
  const project = data.value.projects.find((item) => item.id === id);
  if (!project || data.value.projects.length < 2) return;
  const count = data.value.drafts.filter(
    (item) => item.projectId === id,
  ).length;
  confirm(
    "删除项目",
    "将永久删除「" + project.name + "」和其中的 " + count + " 份草稿及版本。",
    async () => {
      if (
        request.value &&
        data.value.drafts.some(
          (item) => item.id === request.value?.draftId && item.projectId === id,
        )
      )
        await cancel();
      if (draft.value?.projectId === id) data.value.activeDraft = null;
      data.value.drafts = data.value.drafts.filter(
        (item) => item.projectId !== id,
      );
      data.value.projects = data.value.projects.filter(
        (item) => item.id !== id,
      );
      expandedProjects.value = expandedProjects.value.filter(
        (projectId) => projectId !== id,
      );
      await save();
    },
    "删除",
  );
}

function togglePin(id: string) {
  const item = data.value.drafts.find((entry) => entry.id === id);
  if (item) item.pinned = !item.pinned;
}

const draftMenu = ref<{ id: string; x: number; y: number }>();
const menuDraft = computed(() =>
  data.value.drafts.find((item) => item.id === draftMenu.value?.id),
);
const draftMenuOptions = computed<DropdownOption[]>(() => {
  const item = menuDraft.value;
  if (!item) return [];
  return [
    {
      label: "重命名",
      key: "rename",
      icon: () => h(Pencil, { size: 14, "aria-hidden": "true" }),
    },
    {
      label: item.pinned ? "取消置顶" : "置顶",
      key: "pin",
      icon: () =>
        h(item.pinned ? PinOff : Pin, { size: 14, "aria-hidden": "true" }),
    },
    {
      label: item.favorite ? "取消收藏" : "收藏",
      key: "favorite",
      icon: () => h(Star, { size: 14, "aria-hidden": "true" }),
    },
    {
      label: "项目",
      key: "projects",
      icon: () => h(Folder, { size: 14, "aria-hidden": "true" }),
      children: [
        { label: "未分配项目", key: "unassign", disabled: !item.projectId },
        ...data.value.projects.map((project) => ({
          label: project.name,
          key: "assign:" + project.id,
          disabled: item.projectId === project.id,
        })),
      ],
    },
    {
      label: "永久删除",
      key: "delete",
      icon: () => h(Trash2, { size: 14, "aria-hidden": "true" }),
    },
  ];
});

function openDraftMenu(id: string, event: MouseEvent) {
  const target = (
    event.currentTarget as HTMLElement | null
  )?.getBoundingClientRect();
  const byKeyboard = event.clientX === 0 && event.clientY === 0;
  draftMenu.value = {
    id,
    x: byKeyboard ? (target?.left ?? 0) + 12 : event.clientX,
    y: byKeyboard ? (target?.bottom ?? 0) : event.clientY,
  };
}

function closeDraftMenu() {
  draftMenu.value = undefined;
}

function renameDraft(id: string, title: string) {
  const name = ref(title);
  dialog.create({
    title: "重命名草稿",
    positiveText: "保存",
    negativeText: "取消",
    showIcon: false,
    content: () =>
      h(NInput, {
        value: name.value,
        "onUpdate:value": (value) => {
          name.value = value;
        },
        maxlength: 100,
        inputProps: { "aria-label": "草稿名称" },
      }),
    onPositiveClick: () => {
      const title = name.value.trim();
      if (!title) {
        message.warning("请输入草稿名称。");
        return false;
      }
      const item = data.value.drafts.find((draft) => draft.id === id);
      if (item && item.title !== title) {
        item.title = title;
        item.updated = Date.now();
      }
    },
  });
}

function runDraftMenu(key: string | number) {
  const item = menuDraft.value;
  closeDraftMenu();
  if (!item || typeof key !== "string") return;
  switch (key) {
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
    case "unassign":
      item.projectId = null;
      break;
    default: {
      const project = data.value.projects.find(
        (entry) => key === "assign:" + entry.id,
      );
      if (project) item.projectId = project.id;
    }
  }
}

function deleteDraft(id: string) {
  const item = data.value.drafts.find((entry) => entry.id === id);
  if (!item) return;
  confirm(
    "删除草稿",
    "将永久删除「" + (item.title || "未命名草稿") + "」和它的全部版本。",
    async () => {
      if (request.value?.draftId === item.id) await cancel();
      if (data.value.activeDraft === item.id) data.value.activeDraft = null;
      data.value.drafts = data.value.drafts.filter(
        (entry) => entry.id !== item.id,
      );
      await save();
    },
    "删除",
  );
}

function confirm(
  title: string,
  content: string,
  action: () => Promise<void>,
  positiveText = "确认",
) {
  dialog.warning({
    title,
    content,
    positiveText,
    negativeText: "取消",
    showIcon: false,
    onPositiveClick: async () => {
      try {
        await action();
      } catch (error) {
        message.error(errorMessage(error));
        return false;
      }
    },
  });
}

async function optimize(isFollowup = false) {
  const item = draft.value;
  if (!item || !item.source.trim() || request.value) return;
  if (utilities.value) return;
  const template = selectedTemplate.value;
  if (!template) {
    message.info("请先在模板页创建一个优化模板。");
    return;
  }
  if (!isFollowup && (!item.title.trim() || item.title === "未命名草稿")) {
    item.title = Array.from(item.source.trim().replace(/\s+/g, " "))
      .slice(0, 100)
      .join("");
  }
  const previousResult = item.result;
  const instruction = followup.value.trim();
  if (
    isFollowup &&
    (template.category !== "context" || !previousResult.trim() || !instruction)
  )
    return;
  const requestTemplate = isFollowup ? iterationTemplate.value : template;
  if (!requestTemplate) {
    message.info("请先在模板页创建一个迭代优化模板。");
    return;
  }
  const followupRevision = isFollowup
    ? { previousResult, instruction }
    : undefined;

  const revision =
    template.strategy === "iterate"
      ? {
          previousResult,
          instruction: iterationInstruction,
        }
      : undefined;
  if (revision && !revision.previousResult.trim()) {
    message.warning("请先生成或填写一份优化稿，再使用通用迭代。");
    return;
  }
  if (!profile.value) {
    if (!models.value.length) openUtilities("model");
    else message.info("请先在工作台底部选择供应商。");
    return;
  }
  if (
    !modelOptions.value.some(
      (option) => option.value === selectedModel.value,
    ) ||
    !selectedModel.value
  ) {
    message.info("请先选择本次使用的模型。");
    return;
  }
  const activeProfile = { ...profile.value, model: selectedModel.value };
  const source = item.source;
  const id = crypto.randomUUID();
  request.value = { id, draftId: item.id, text: "" };
  try {
    const available = await desktop.hasKey(activeProfile);
    if (
      request.value?.id !== id ||
      draft.value?.id !== item.id ||
      utilities.value
    )
      return;
    if (!available) {
      message.warning("请先为所选供应商填写API Key。");
      openUtilities("model");
      return;
    }
    resultMode.value = "preview";
    const renderRevision = followupRevision ?? revision;
    const result = await desktop.optimize(
      activeProfile,
      source,
      id,
      requestTemplate.strategy,
      revision,
      renderTemplate(requestTemplate.content, source, renderRevision),
      requestTemplate.userContent?.trim()
        ? renderTemplate(requestTemplate.userContent, source, renderRevision)
        : undefined,
      followupRevision,
      (text) => {
        if (request.value?.id === id) request.value.text += text;
      },
    );
    if (request.value?.id !== id) return;
    const current = data.value.drafts.find((entry) => entry.id === item.id);
    if (!current) return;
    const version: Version = {
      id,
      source,
      result,
      question: null,
      model: activeProfile.model,
      providerName: activeProfile.name,
      instruction: followupRevision?.instruction,
      created: Date.now(),
    };
    current.versions.unshift(version);
    current.updated = version.created;
    if (current.source === source && current.result === previousResult) {
      current.result = version.result;
      current.question = null;
      if (
        followupRevision &&
        draft.value?.id === item.id &&
        followup.value.trim() === instruction
      )
        followup.value = "";
      if (draft.value?.id === item.id) resultMode.value = "preview";
    } else {
      message.info("草稿内容已修改，本次优化结果已保存到版本历史。");
    }
    await save();
  } catch (error) {
    if (
      request.value?.id === id &&
      draft.value?.id === item.id &&
      draft.value.source === source
    ) {
      message.error(errorMessage(error), { duration: 9000, closable: true });
    }
  } finally {
    if (request.value?.id === id) request.value = undefined;
  }
}

async function cancel() {
  const id = request.value?.id;
  if (!id) return;
  request.value = undefined;
  try {
    await desktop.cancel(id);
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

async function exportFile(format: string) {
  const item = draft.value;
  if (!item?.result) return;
  const filename = item.title.replace(/[<>:"/\\|?*]/g, "_") || "词稿";
  const path = await pickSave({
    defaultPath: filename + "." + format,
    filters: [
      { name: format === "md" ? "Markdown" : "纯文本", extensions: [format] },
    ],
  });
  if (!path) return;
  await desktop.exportFile(path, item.result);
  message.success("已导出优化稿");
}

async function backup() {
  try {
    const path = await pickSave({
      defaultPath: "词稿备份.json",
      filters: [{ name: "词稿备份", extensions: ["json"] }],
    });
    if (!path) return;
    await desktop.exportFile(path, JSON.stringify(data.value));
    message.success("已导出备份");
  } catch (error) {
    message.error(errorMessage(error));
  }
}

async function restore() {
  try {
    const path = await open({
      multiple: false,
      filters: [{ name: "词稿备份", extensions: ["json"] }],
    });
    if (!path) return;
    const document = await desktop.importFile(path);
    const incoming = JSON.parse(document) as {
      drafts: unknown[];
      projects: unknown[];
    };
    confirm(
      "恢复备份",
      "将用备份中的 " +
        incoming.projects.length +
        " 个项目和 " +
        incoming.drafts.length +
        " 份草稿替换当前工作区。建议先导出当前备份。",
      async () => {
        await cancel();
        await replace(document);
        expandedProjects.value = [];
        selectedVersion.value = undefined;
        message.success("备份已恢复");
      },
      "恢复备份",
    );
  } catch (error) {
    message.error(errorMessage(error));
  }
}

function history() {
  selectedVersion.value = draft.value?.versions[0];
  showHistory.value = true;
}

function restoreVersion() {
  const item = draft.value;
  const previous = selectedVersion.value;
  if (!item || !previous) return;
  confirm(
    "恢复版本",
    "当前编辑内容也会保存为一个版本。",
    async () => {
      const snapshot: Version = {
        id: crypto.randomUUID(),
        source: item.source,
        result: item.result,
        question: item.question,
        model: "恢复前的草稿",
        created: Date.now(),
      };
      item.versions.unshift(
        { ...previous, id: crypto.randomUUID(), created: Date.now() },
        snapshot,
      );
      item.source = previous.source;
      item.result = previous.result;
      item.updated = Date.now();
      item.question = previous.question;
      showHistory.value = false;
      await save();
      if (!saveError.value) message.success("已恢复为新版本");
    },
    "恢复版本",
  );
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
    if (action === "exit" && request.value) await cancel();
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
  if (action === "ask" || (action === "exit" && request.value)) {
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
    showHistory.value ||
    showClose.value ||
    event.repeat ||
    document.querySelector(".n-modal, .n-dialog")
  )
    return;
  const binding = shortcutFromEvent(event);
  const shortcuts = data.value.shortcuts ?? defaultShortcuts;
  if (binding === shortcuts.newDraft) {
    event.preventDefault();
    create();
  }
  if (binding === shortcuts.optimize) {
    event.preventDefault();
    void optimize();
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
</script>

<template>
  <div
    class="shell"
    :class="{
      'template-editor-open': utilities === 'templates' && editingTemplate,
      'settings-open': utilities === 'settings',
      'utilities-open': !!utilities,
    }"
  >
    <window-titlebar>
      <template #leading>
        <n-button
          class="compact-library"
          quaternary
          aria-label="打开资料库"
          @click="showLibrary = true"
          ><template #icon><Menu /></template
        ></n-button>
      </template>
      <span class="topbar-divider"></span>
      <span class="breadcrumb" data-tauri-drag-region>{{
        draft
          ? data.projects.find((p) => p.id === draft?.projectId)?.name
          : "本地工作区"
      }}</span>
    </window-titlebar>
    <div class="workspace-layout">
      <library-panel
        class="desktop-library"
        :data="data"
        :active-utility="utilities"
        v-model:recent-expanded="recentExpanded"
        v-model:pinned-expanded="pinnedExpanded"
        v-model:list-mode="draftListMode"
        @create="create()"
        @browse="utilities = undefined"
        @delete-draft="deleteDraft"
        @toggle-pin="togglePin"
        @draft-context="openDraftMenu"
        @open="loadDraft"
        @utilities="openUtilities"
      />
      <main class="main-surface">
        <n-alert
          v-if="saveError"
          class="surface-alert"
          type="error"
          :show-icon="false"
          >{{ saveError }}
          <n-button size="small" @click="save">重试保存</n-button></n-alert
        >
        <section
          v-if="utilities"
          class="utilities-page"
          :aria-label="utilitiesTitle"
        >
          <header class="utilities-header">
            <h1>{{ utilitiesTitle }}</h1>
            <n-button
              v-if="utilities === 'usage' && !usageCatalog"
              class="usage-catalog-action"
              @click="usageCatalog = true"
              >价格目录</n-button
            >
            <n-button
              v-if="utilities === 'templates' && libraryCategory"
              quaternary
              @click="
                editingTemplate
                  ? (editingTemplate = undefined)
                  : (libraryCategory = undefined)
              "
            >
              <template #icon><ArrowLeft aria-hidden="true" /></template>
              {{ editingTemplate ? "返回模板列表" : "返回分类" }}
            </n-button>
          </header>
          <div
            :key="
              utilities +
              ':' +
              (libraryCategory ?? '') +
              ':' +
              (editingTemplate?.id ?? '')
            "
            class="utility-body"
            :class="{ 'utility-scroll': utilities !== 'settings' }"
            :tabindex="utilities === 'settings' ? undefined : 0"
            :aria-label="utilitiesTitle + '内容'"
          >
            <template v-if="utilities === 'templates'">
              <div v-if="!libraryCategory" class="template-categories">
                <div class="page-actions">
                  <n-button type="primary" @click="editTemplateCategory()">
                    <template #icon><Plus aria-hidden="true" /></template>
                    添加模板分类
                  </n-button>
                </div>
                <div
                  v-for="category in libraryCategories"
                  :key="category.value"
                  class="template-category-row"
                >
                  <button
                    type="button"
                    class="template-category"
                    @click="libraryCategory = category.value"
                  >
                    <strong>{{ category.label }}</strong>
                    <span>{{ category.description }}</span>
                    <ChevronRight aria-hidden="true" :size="16" />
                  </button>
                  <span
                    v-if="isBuiltinCategory(category.value)"
                    class="builtin-category-label"
                    >系统内置</span
                  >
                  <div v-else class="template-category-actions">
                    <n-button
                      size="small"
                      :aria-label="'编辑分类 ' + category.label"
                      @click="editTemplateCategory(category)"
                      >编辑</n-button
                    >
                    <n-button
                      size="small"
                      quaternary
                      :aria-label="'删除分类 ' + category.label"
                      @click="deleteTemplateCategory(category)"
                      >删除</n-button
                    >
                  </div>
                </div>
                <p v-if="!libraryCategories.length" class="muted">
                  暂无模板分类，点击“添加模板分类”开始创建。
                </p>
              </div>
              <div v-else-if="editingTemplate" class="template-editor">
                <n-form :show-feedback="false" @submit.prevent="saveTemplate">
                  <p v-if="viewingBuiltin" class="muted">
                    系统内置模板，仅供查看。
                  </p>
                  <n-form-item label="提示词名称" label-for="template-name">
                    <n-input
                      v-model:value="editingTemplate.name"
                      :readonly="viewingBuiltin"
                      :input-props="{ id: 'template-name' }"
                      :maxlength="100"
                    />
                  </n-form-item>
                  <n-form-item
                    v-for="field in templateFields"
                    :key="field.key"
                    :label="field.label"
                    :label-for="field.id"
                  >
                    <div class="template-field">
                      <n-button
                        class="template-preview-toggle"
                        size="tiny"
                        :secondary="templatePreviewOpen[field.role]"
                        :quaternary="!templatePreviewOpen[field.role]"
                        :aria-label="field.label + '预览'"
                        :aria-pressed="templatePreviewOpen[field.role]"
                        @click="
                          templatePreviewOpen[field.role] =
                            !templatePreviewOpen[field.role]
                        "
                        >{{
                          templatePreviewOpen[field.role]
                            ? viewingBuiltin
                              ? "返回原文"
                              : "返回编辑"
                            : "预览"
                        }}</n-button
                      >
                      <div
                        v-if="templatePreviewOpen[field.role]"
                        :id="field.id"
                        class="template-content-input template-inline-preview"
                      >
                        <pre tabindex="0" :aria-label="field.label + '预览'">{{
                          templatePreviewError ||
                          templatePreview.find(
                            (item) => item.role === field.role,
                          )?.content ||
                          "填写系统提示词后即可预览。"
                        }}</pre>
                      </div>
                      <n-input
                        v-else
                        v-model:value="editingTemplate[field.key]"
                        :readonly="viewingBuiltin"
                        type="textarea"
                        :input-props="{ id: field.id }"
                        class="template-content-input"
                        :resizable="false"
                        :maxlength="20000"
                        :placeholder="field.placeholder"
                      />
                    </div>
                  </n-form-item>
                  <n-form-item
                    label="提示词描述"
                    label-for="template-description"
                  >
                    <n-input
                      v-model:value="editingTemplate.description"
                      :readonly="viewingBuiltin"
                      type="textarea"
                      :input-props="{ id: 'template-description' }"
                      class="template-description-input"
                      :resizable="false"
                      :maxlength="500"
                    />
                  </n-form-item>
                  <div class="template-actions">
                    <n-button @click="editingTemplate = undefined">{{
                      viewingBuiltin ? "关闭" : "取消"
                    }}</n-button>
                    <n-button
                      v-if="!viewingBuiltin"
                      type="primary"
                      attr-type="submit"
                      >保存</n-button
                    >
                  </div>
                </n-form>
              </div>
              <template v-else>
                <div class="page-actions">
                  <n-button type="primary" @click="newTemplate">
                    <template #icon><Plus aria-hidden="true" /></template>
                    新增模板
                  </n-button>
                </div>
                <ul class="template-list" aria-label="优化模板">
                  <li
                    v-for="item in libraryTemplates"
                    :key="item.id"
                    class="template-row"
                  >
                    <div>
                      <h2>
                        {{ item.name }}
                        <span
                          v-if="isBuiltinTemplate(item.id)"
                          class="builtin-template-label"
                          >系统内置</span
                        >
                      </h2>
                      <p v-if="item.description">{{ item.description }}</p>
                    </div>
                    <div class="template-actions">
                      <n-button
                        size="small"
                        :aria-label="
                          (isBuiltinTemplate(item.id) ? '查看 ' : '编辑 ') +
                          item.name
                        "
                        @click="editingTemplate = { ...item }"
                        >{{
                          isBuiltinTemplate(item.id) ? "查看" : "编辑"
                        }}</n-button
                      >
                      <n-button
                        size="small"
                        v-if="!isBuiltinTemplate(item.id)"
                        :aria-label="'复制 ' + item.name"
                        @click="copyTemplate(item)"
                        >复制</n-button
                      >
                      <n-button
                        size="small"
                        quaternary
                        v-if="!isBuiltinTemplate(item.id)"
                        :aria-label="'删除 ' + item.name"
                        @click="deleteTemplate(item)"
                        >删除</n-button
                      >
                    </div>
                  </li>
                </ul>
                <p v-if="!libraryTemplates.length">
                  暂无模板，创建一个模板后就能在草稿页选择。
                </p>
              </template>
            </template>
            <projects-panel
              v-else-if="utilities === 'projects'"
              :projects="data.projects"
              :drafts="data.drafts"
              v-model:expanded="expandedProjects"
              @add-project="addProject"
              @delete-project="deleteProject"
              @create-draft="create"
              @open-draft="loadDraft"
              @draft-context="openDraftMenu"
            />
            <usage-panel
              v-else-if="utilities === 'usage'"
              v-model:catalog="usageCatalog"
            />
            <settings-panel
              v-else
              :key="utilities"
              :section="utilities"
              :update-shortcuts="updateShortcuts"
              @close-behavior="(value) => (data.closeBehavior = value)"
              :data="data"
              @backup="backup"
              @restore="restore"
              @saved="saveProfile"
              @remove="removeProfile"
              @theme="(value) => (data.theme = value)"
            />
          </div>
        </section>
        <div v-else-if="!draft" class="welcome">
          <div class="welcome-copy">
            <div class="welcome-mark">
              <FileText :size="35" aria-hidden="true" />
            </div>
            <h1>开启一份草稿</h1>
            <p>从一句真实的需求开始，把它整理成可直接交给 AI 的提示词。</p>
            <n-button type="primary" @click="create()"
              ><template #icon><Plus /></template>新建草稿</n-button
            >
          </div>
        </div>
        <template v-else>
          <div class="draft-toolbar">
            <input
              v-model="draft.title"
              class="draft-title"
              aria-label="草稿名称"
              maxlength="100"
              placeholder="未命名草稿"
            />
            <div class="draft-tools">
              <n-button
                quaternary
                size="small"
                :aria-pressed="draft.favorite"
                :aria-label="draft.favorite ? '取消收藏' : '收藏草稿'"
                @click="draft.favorite = !draft.favorite"
                ><template #icon
                  ><Star
                    :fill="
                      draft.favorite ? 'currentColor' : 'none'
                    " /></template
              ></n-button>
              <n-button quaternary size="small" @click="history"
                ><template #icon><History /></template>历史</n-button
              >
              <n-button
                quaternary
                size="small"
                aria-label="删除草稿"
                @click="deleteDraft(draft.id)"
                ><template #icon><Trash2 /></template
              ></n-button>
            </div>
          </div>
          <div class="editor-spread">
            <section class="editor">
              <header>
                <h2>原稿</h2>
                <span>{{ sourceCount.toLocaleString("zh-CN") }} 字</span>
              </header>
              <textarea
                v-model="draft.source"
                aria-label="原稿"
                spellcheck="false"
                maxlength="30000"
                placeholder="写下你的目标、背景，以及你希望 AI 交付什么。"
              />
            </section>
            <section class="editor result-editor">
              <header>
                <h2>优化稿</h2>
                <div class="editor-actions">
                  <n-button-group
                    class="result-modes"
                    role="group"
                    aria-label="优化稿显示方式"
                  >
                    <n-button
                      v-for="mode in resultModes"
                      :key="mode.value"
                      size="tiny"
                      quaternary
                      :disabled="!!activeRequest"
                      :aria-pressed="resultMode === mode.value"
                      @click="resultMode = mode.value"
                      >{{ mode.label }}</n-button
                    >
                  </n-button-group>
                  <span
                    class="result-actions-divider"
                    aria-hidden="true"
                  ></span>
                  <n-button
                    quaternary
                    size="small"
                    :disabled="!draft.result || !!activeRequest"
                    aria-label="复制优化稿"
                    title="复制优化稿"
                    @click="copy"
                    ><template #icon><Copy /></template
                  ></n-button>
                  <n-dropdown
                    trigger="click"
                    :options="[
                      { label: '导出 Markdown', key: 'md' },
                      { label: '导出纯文本', key: 'txt' },
                    ]"
                    @select="
                      (key) =>
                        exportFile(String(key)).catch((error) =>
                          message.error(errorMessage(error)),
                        )
                    "
                    ><n-button
                      quaternary
                      size="small"
                      :disabled="!draft.result || !!activeRequest"
                      aria-label="导出优化稿"
                      ><template #icon><Download /></template></n-button
                  ></n-dropdown>
                  <n-button
                    quaternary
                    size="small"
                    :disabled="!draft.result || !!activeRequest"
                    aria-label="放大查看优化稿"
                    title="放大查看优化稿"
                    @click="showResult = true"
                    ><template #icon><Maximize2 aria-hidden="true" /></template
                  ></n-button>
                </div>
              </header>
              <div class="result-output">
                <div
                  v-if="activeRequest && !activeRequest.text"
                  class="result-content result-progress"
                  role="status"
                >
                  <n-spin :size="16" />
                  <span>正在优化提示词…</span>
                </div>
                <pre
                  v-else-if="activeRequest"
                  ref="streamOutput"
                  class="result-content result-stream"
                  tabindex="0"
                  aria-label="正在生成的优化稿"
                  >{{ activeRequest.text }}</pre>
                <result-comparison
                  v-else-if="resultMode === 'compare' && draft.result"
                  :source="draft.source"
                  :result="draft.result"
                  :changes="changes"
                />
                <div
                  v-else-if="resultMode === 'preview' && draft.result"
                  class="result-preview result-content"
                  tabindex="0"
                  aria-label="优化稿预览"
                  v-html="resultPreview"
                ></div>
                <textarea
                  v-else-if="resultMode === 'source'"
                  class="result-content"
                  v-model="draft.result"
                  aria-label="优化稿"
                  spellcheck="false"
                  placeholder="改写结果会显示在这里，也可以直接编辑。"
                />
                <p v-else class="result-empty result-content">
                  优化结果会显示在这里。
                </p>
                <form
                  v-if="draftCategory === 'context'"
                  class="context-compose"
                  @submit.prevent="optimize(true)"
                >
                  <n-input
                    v-model:value="followup"
                    :maxlength="4000"
                    :input-props="{ 'aria-label': '继续追问' }"
                    :placeholder="
                      draft.result.trim()
                        ? '继续追问，例如：把第二步写具体一点'
                        : '先生成优化稿，再继续追问'
                    "
                    :disabled="!!request || !draft.result.trim()"
                  />
                  <n-select
                    aria-label="迭代优化模板"
                    placeholder="暂无迭代模板"
                    :value="iterationTemplate?.id ?? null"
                    :options="iterationOptions"
                    :disabled="!!request || !iterationOptions.length"
                    @update:value="(id) => (iterationTemplateId = id)"
                  />
                  <n-button
                    attr-type="submit"
                    :disabled="
                      !!request ||
                      !selectedTemplate ||
                      !iterationTemplate ||
                      !draft.source.trim() ||
                      !draft.result.trim() ||
                      !followup.trim()
                    "
                  >
                    <template #icon><Send aria-hidden="true" /></template>追问
                  </n-button>
                </form>
              </div>
            </section>
          </div>
          <footer class="workbench-footer">
            <div class="statusbar">
              <div class="optimization-controls">
                <n-select
                  class="workbench-category"
                  aria-label="模板分类"
                  v-model:value="draftCategory"
                  :options="categoryOptions"
                  :disabled="!!request"
                />
                <n-select
                  class="workbench-template"
                  aria-label="优化模板"
                  :value="selectedTemplate?.id ?? null"
                  :options="templateOptions"
                  placeholder="选择模板"
                  :disabled="!!request || !draftTemplates.length"
                  @update:value="(id) => (templateId = id)"
                />
                <n-select
                  class="workbench-provider"
                  aria-label="本次优化供应商"
                  placeholder="选择供应商"
                  :value="profile?.id ?? null"
                  :options="
                    models.map((item) => ({ label: item.name, value: item.id }))
                  "
                  :disabled="!!request || !models.length"
                  @update:value="(id) => (data.activeProfile = id)"
                />
                <n-select
                  class="workbench-model"
                  aria-label="本次优化模型"
                  placeholder="选择模型"
                  filterable
                  :value="selectedModel"
                  :options="modelOptions"
                  :disabled="!profile || !!request"
                  @update:value="(model) => (selectedModel = model)"
                />
                <n-button
                  v-if="!models.length"
                  quaternary
                  @click="openUtilities('model')"
                  >添加供应商</n-button
                >
                <n-button v-if="request" @click="cancel"
                  ><template #icon><Square /></template>取消优化</n-button
                >
                <n-button
                  v-else
                  type="primary"
                  :disabled="
                    !selectedTemplate ||
                    !draft.source.trim() ||
                    (selectedTemplate?.strategy === 'iterate' &&
                      !draft.result.trim())
                  "
                  @click="optimize()"
                  ><template #icon><Send /></template>优化提示词</n-button
                >
              </div>
            </div>
          </footer>
        </template>
      </main>
    </div>
    <n-dropdown
      trigger="manual"
      size="small"
      :theme-overrides="{ fontSizeSmall: '12px', optionHeightSmall: '28px' }"
      placement="bottom-start"
      :show="!!menuDraft"
      :x="draftMenu?.x"
      :y="draftMenu?.y"
      :options="draftMenuOptions"
      @select="runDraftMenu"
      @clickoutside="closeDraftMenu"
      @update:show="(show) => !show && closeDraftMenu()"
    >
      <span aria-hidden="true"></span>
    </n-dropdown>
    <n-modal
      v-model:show="showResult"
      preset="card"
      title="优化稿"
      to=".app-root"
      class="result-dialog"
      closable
    >
      <template #header-extra>
        <n-button
          quaternary
          size="small"
          :disabled="!draft?.result"
          aria-label="复制优化稿"
          title="复制优化稿"
          @click="copy"
        >
          <template #icon><Copy aria-hidden="true" /></template>
        </n-button>
      </template>
      <template v-if="draft">
        <result-comparison
          v-if="resultMode === 'compare'"
          :source="draft.source"
          :result="draft.result"
          :changes="changes"
        />
        <pre
          v-else-if="resultMode === 'source'"
          class="result-content result-dialog-source"
          tabindex="0"
          aria-label="优化稿原文"
          >{{ draft.result }}</pre>
        <div
          v-else
          class="result-preview result-content"
          tabindex="0"
          aria-label="优化稿预览"
          v-html="resultPreview"
        ></div>
      </template>
    </n-modal>
    <n-drawer v-model:show="showLibrary" :width="285" placement="left"
      ><n-drawer-content title="资料库" closable :native-scrollbar="false">
        <library-panel
          :data="data"
          :active-utility="utilities"
          v-model:recent-expanded="recentExpanded"
          v-model:pinned-expanded="pinnedExpanded"
          v-model:list-mode="draftListMode"
          @create="create()"
          @browse="utilities = undefined"
          @delete-draft="deleteDraft"
          @toggle-pin="togglePin"
          @draft-context="openDraftMenu"
          @open="loadDraft"
          @utilities="openUtilities"
        /> </n-drawer-content
    ></n-drawer>
    <n-drawer v-model:show="showHistory" :width="drawerWidth" to=".app-root">
      <n-drawer-content
        title="版本历史"
        closable
        :native-scrollbar="false"
        :scrollbar-props="{
          themeOverrides: {
            width: '5px',
            borderRadius: '999px',
            color: 'var(--scroll-thumb)',
            colorHover:
              'color-mix(in srgb, var(--scroll-thumb) 70%, var(--accent))',
            railColor: 'transparent',
          },
        }"
      >
        <p v-if="!draft?.versions.length" class="muted">
          完成一次优化后，历史版本会出现在这里。
        </p>
        <template v-else>
          <div
            class="version-list"
            role="group"
            aria-label="历史版本列表"
            tabindex="0"
          >
            <button
              v-for="version in draft.versions"
              :key="version.id"
              :class="{ selected: selectedVersion?.id === version.id }"
              @click="selectedVersion = version"
            >
              <strong>{{ version.model }}</strong><span>{{
                new Date(version.created).toLocaleString("zh-CN")
              }}</span>
            </button>
          </div>
          <div
            v-if="selectedVersion"
            :key="selectedVersion.id"
            class="version-detail"
          >
            <h3>原稿</h3>
            <pre
              class="version-source"
              tabindex="0"
              aria-label="历史版本原稿"
              >{{ selectedVersion.source }}</pre>
            <h3>优化稿</h3>
            <pre
              class="version-result"
              tabindex="0"
              aria-label="历史版本优化稿"
              >{{ selectedVersion.result }}</pre>
            <div class="version-actions">
              <n-button type="primary" @click="restoreVersion">
                恢复当前版本
              </n-button>
            </div>
          </div>
        </template>
      </n-drawer-content></n-drawer
    >
  </div>
  <n-modal
    v-model:show="showClose"
    preset="card"
    title="关闭词稿"
    class="close-window-modal"
    to=".app-root"
    :mask-closable="false"
    :close-on-esc="!closing"
    :closable="!closing"
    :style="{ width: 'min(440px, calc(100vw - 32px))' }"
  >
    <p class="close-window-description">
      隐藏到托盘后，词稿将继续运行。你可以点击系统托盘中的词稿图标重新打开窗口。退出程序将结束运行。
    </p>
    <p v-if="request" class="close-window-description" role="status">
      正在优化提示词。隐藏后将继续生成，退出程序会中断本次任务。
    </p>
    <n-checkbox v-model:checked="rememberClose" :disabled="closing"
      >记住我的选择</n-checkbox
    >
    <template #footer
      ><div class="template-actions">
        <n-button :disabled="closing" @click="completeClose('exit')"
          >退出程序</n-button
        ><n-button
          type="primary"
          :loading="closing"
          @click="completeClose('hide')"
          >隐藏到托盘</n-button
        >
      </div></template
    >
  </n-modal>
</template>

<style scoped>
.usage-catalog-action {
  margin-left: auto;
}
</style>
