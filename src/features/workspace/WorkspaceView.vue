<script setup lang="ts">
import { contextTokenBudget, contextWarningRatio } from "./contextBudget";
import { computed, h, onMounted, onUnmounted, ref, watch } from "vue";
import {
  ArrowLeft,
  Check,
  CircleHelp,
  Info,
  ChevronRight,
  ChevronDown,
  Copy,
  Download,
  FileText,
  Folder,
  FolderMinus,
  Maximize2,
  Menu,
  Pencil,
  Pin,
  PinOff,
  Plus,
  Star,
  Trash2,
} from "lucide-vue-next";
import {
  NText,
  NTooltip,
  NButton,
  NCheckbox,
  NDrawer,
  NDrawerContent,
  NDropdown,
  NForm,
  NFormItem,
  NInput,
  NModal,
  NSelect,
  useDialog,
  useMessage,
  type DropdownOption,
} from "naive-ui";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  defaultShortcuts,
  shortcutFromEvent,
  shortcutLabels,
} from "../settings/shortcuts";
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
  type Workspace,
} from "../../lib/tauri";
import { createDraftRequests } from "./draftRequests";
import { routeTemplateWheel } from "./templateWheel";
import {
  latestContext,
  prepareConversation,
  conversationUsage,
  recordReply,
} from "./conversation";
import appIcon from "../../../src-tauri/icons/icon.png";
import PromptWorkspace from "./PromptWorkspace.vue";
import { duplicateDraft } from "./duplicateDraft";
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
  restoreBackup,
  save,
} = workspace;
const draftProject = computed(() =>
  data.value.projects.find((project) => project.id === draft.value?.projectId),
);
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
const libraryCategories = computed(() => {
  const categories = [
    ...builtinCategories,
    ...(data.value.templateCategories ?? [])
      .filter((item) => !isBuiltinCategory(item.id))
      .map((item) => ({
        label: item.name,
        value: item.id,
        description: "自定义提示词模板。",
      })),
  ];
  for (const template of data.value.templates ?? []) {
    const id = template.category ?? "basic";
    if (!categories.some((category) => category.value === id)) {
      categories.push({
        label: id,
        value: id,
        description: "自定义提示词模板。",
      });
    }
  }
  return categories;
});
const categoryOptions = computed(() => libraryCategories.value);
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
  { value: "source", label: "编辑" },
  { value: "compare", label: "对比" },
] as const;
const showResult = ref(false);
const keyboardNavigation = ref(false);
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
const requests = createDraftRequests();
const sendingDraftIds = computed(() => [...requests.keys()]);
const hasRequests = computed(() => requests.size > 0);
const activeRequest = computed(() =>
  draft.value ? requests.get(draft.value.id) : undefined,
);
const defaultTemplateId = computed(() =>
  availableTemplates(data.value).some(
    (item) => item.id === data.value.defaultTemplateId,
  )
    ? data.value.defaultTemplateId!
    : "prompt-general",
);
async function setDefaultTemplate(id: string) {
  data.value.defaultTemplateId = id;
  await save();
  if (!saveError.value) message.success("已设为默认模板");
}
const templateList = computed(() => availableTemplates(data.value));
const draftCategory = ref<TemplateCategory>("basic");
watch(
  categoryOptions,
  (items) => {
    if (!items.some((item) => item.value === draftCategory.value))
      draftCategory.value = items[0]?.value ?? "";
  },
  { immediate: true },
);
const templateId = computed(() => draft.value?.templateId);
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
    templateList.value.find((item) => item.id === templateId.value) ??
    draftTemplates.value[0],
);
const focusedTemplateHelp = ref<string | null>(null);
function templateOption(item: PromptTemplate): DropdownOption {
  return {
    key: item.id,
    label: () =>
      h("span", { class: "template-option-heading" }, [
        h(
          "span",
          { class: "template-option-check", "aria-hidden": "true" },
          selectedTemplate.value?.id === item.id
            ? [h(Check, { size: 14 })]
            : [],
        ),
        h("span", { class: "template-option-name" }, item.name),
        ...(item.description?.trim()
          ? [
              h(
                NTooltip,
                {
                  placement: "top",
                  delay: 250,
                  show:
                    focusedTemplateHelp.value === item.id ? true : undefined,
                  style: {
                    width: "360px",
                    maxWidth: "calc(100vw - 32px)",
                    boxSizing: "border-box",
                  },
                },
                {
                  trigger: () =>
                    h(
                      "button",
                      {
                        type: "button",
                        class: "template-option-help",
                        "aria-label": `${item.name}功能说明`,
                        onClick: (event: MouseEvent) => event.stopPropagation(),
                        onFocus: () => {
                          focusedTemplateHelp.value = item.id;
                        },
                        onBlur: () => {
                          focusedTemplateHelp.value = null;
                        },
                      },
                      [h(CircleHelp, { size: 14, "aria-hidden": "true" })],
                    ),
                  default: () => item.description,
                },
              ),
            ]
          : []),
      ]),
  };
}
const templateOptions = computed<DropdownOption[]>(() => {
  const groups = categoryOptions.value
    .map((category) => ({
      category,
      items: templateList.value.filter(
        (item) => (item.category ?? "basic") === category.value,
      ),
    }))
    .filter((group) => group.items.length);
  return groups.map(({ category, items }) => ({
    key: category.value,
    label: category.label,
    children: items.map(templateOption),
  }));
});
function selectOptimizationTemplate(id: string) {
  const template = templateList.value.find((item) => item.id === id);
  if (!template) return;
  draftCategory.value = template.category ?? "basic";
  if (draft.value) draft.value.templateId = id;
}
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
    placeholder: "留空使用默认用户消息。填写 {{{draftContent}}} 可插入原稿。",
  },
] as const;
const templatePreviewOpen = ref({ system: false, user: false });
watch(editingTemplate, () => {
  templatePreviewOpen.value = { system: false, user: false };
  enlargedTemplateRole.value = undefined;
});
const templatePreview = ref<
  Awaited<ReturnType<typeof desktop.previewTemplate>>
>([]);
const templatePreviewError = ref("");
const enlargedTemplateRole = ref<"system" | "user">();
const enlargedTemplateTitle = computed(
  () =>
    templateFields.find((field) => field.role === enlargedTemplateRole.value)
      ?.label ?? "",
);
function templateDisplayText(role: "system" | "user") {
  if (templatePreviewOpen.value[role])
    return templatePreviewError.value
      ? ""
      : (templatePreview.value.find((item) => item.role === role)?.content ??
          "");
  return (
    (role === "system"
      ? editingTemplate.value?.content
      : editingTemplate.value?.userContent) ?? ""
  );
}
async function copyTemplateText(role: "system" | "user") {
  const text = templateDisplayText(role);
  if (!text) return;
  try {
    await writeText(text);
    message.success(
      "已复制" + templateFields.find((field) => field.role === role)?.label,
    );
  } catch (error) {
    message.error(errorMessage(error));
  }
}
watch(
  [
    () => editingTemplate.value?.content,
    () => editingTemplate.value?.userContent,
  ],
  async ([content, userContent], _, onCleanup) => {
    templatePreviewError.value = "";
    if (!content?.trim()) {
      templatePreview.value = [];
      return;
    }

    let current = true;
    onCleanup(() => {
      current = false;
    });
    try {
      const source = "Write a story";
      const messages = await desktop.previewTemplate(
        source,
        renderTemplate(content, source),
        userContent?.trim() ? renderTemplate(userContent, source) : undefined,
      );
      if (current) templatePreview.value = messages;
    } catch (error) {
      if (!current) return;
      templatePreview.value = [];
      templatePreviewError.value = errorMessage(error);
    }
  },
);

watch(
  () => draft.value?.id,
  () => {
    resultMode.value = "preview";
    showResult.value = false;
  },
);
const selectedModel = computed({
  get: () => draft.value?.model ?? null,
  set: (model) => {
    if (draft.value) draft.value.model = model;
  },
});
const modelOptions = computed(() =>
  (profile.value ? modelsFor(profile.value) : []).map((model) => ({
    label: model,
    value: model,
  })),
);
watch(
  () => [
    draft.value?.id,
    draft.value?.providerId,
    draft.value?.model,
    draft.value?.templateId,
    JSON.stringify(data.value.profiles),
    JSON.stringify(templateList.value),
  ],
  () => {
    const item = draft.value;
    if (!item) return;
    if (!templateList.value.some((template) => template.id === item.templateId))
      item.templateId = defaultTemplateId.value;
    if (!models.value.some((provider) => provider.id === item.providerId)) {
      const fallback =
        models.value.find((provider) => provider.isDefault) ??
        (models.value.length === 1 ? models.value[0] : undefined);
      item.providerId = fallback?.id ?? null;
      item.model = fallback?.model ?? null;
    }
    const provider = models.value.find(
      (provider) => provider.id === item.providerId,
    );
    if (provider && !modelsFor(provider).includes(item.model ?? ""))
      item.model = provider.model;
  },
  { immediate: true, flush: "sync" },
);
function selectProvider(id: string) {
  if (!draft.value) return;
  draft.value.providerId = id;
  draft.value.model =
    models.value.find((item) => item.id === id)?.model ?? null;
}
async function setDefaultProfile(id: string) {
  for (const item of data.value.profiles) item.isDefault = item.id === id;
  await save();
  if (saveError.value) message.error(saveError.value);
  else message.success("已设为默认供应商");
}
const contextUsage = computed(() =>
  draft.value ? conversationUsage(draft.value) : undefined,
);
let budgetConfirmationOpen = false;
const comparisonSource = computed(() => draft.value?.source ?? "");

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
      " 个模板，草稿内容和聊天记录不受影响。此操作不可撤销。",
    async () => {
      const previous = {
        templates: data.value.templates,
        categories: data.value.templateCategories,
        defaultTemplateId: data.value.defaultTemplateId,
      };
      if (
        templateList.value.some(
          (item) =>
            item.id === data.value.defaultTemplateId &&
            (item.category ?? "basic") === category.value,
        )
      )
        data.value.defaultTemplateId = "prompt-general";
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
        data.value.defaultTemplateId = previous.defaultTemplateId;
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
      if (data.value.defaultTemplateId === item.id)
        data.value.defaultTemplateId = "prompt-general";
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
  if (!data.value.profiles.length) profile = { ...profile, isDefault: true };
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
    "将永久删除「" +
      project.name +
      "」和其中的 " +
      count +
      " 份草稿及聊天记录。",
    async () => {
      await Promise.all(
        data.value.drafts
          .filter((item) => item.projectId === id)
          .map((item) => cancel(item.id)),
      );
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

const draftMenuOpen = ref(false);
const draftMenu = ref<{
  id: string;
  x: number;
  y: number;
  kind: "title" | "context";
}>();
const menuDraft = computed(() =>
  data.value.drafts.find((item) => item.id === draftMenu.value?.id),
);
const draftMenuOptions = computed<DropdownOption[]>(() => {
  const item = menuDraft.value;
  if (!item) return [];
  const options: DropdownOption[] = [
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
    { type: "divider", key: "result-divider" },
    {
      label: "复制草稿",
      key: "duplicate",
      icon: () => h(Copy, { size: 14, "aria-hidden": "true" }),
    },
    { type: "divider", key: "delete-divider" },
    {
      label: () => h(NText, { type: "error" }, { default: () => "删除" }),
      key: "delete",
      icon: () =>
        h(
          NText,
          {
            type: "error",
            style: {
              display: "inline-flex",
              alignItems: "center",
              lineHeight: "1",
            },
          },
          { default: () => h(Trash2, { size: 14, "aria-hidden": "true" }) },
        ),
    },
    { type: "divider", key: "project-divider" },
    ...(data.value.projects.find((project) => project.id === item.projectId)
      ? [
          {
            type: "render" as const,
            key: "current-project",
            render: () =>
              h(
                "div",
                { class: "draft-menu-project" },
                data.value.projects.find(
                  (project) => project.id === item.projectId,
                )?.name,
              ),
          },
        ]
      : []),
    {
      label: "移动到项目",
      key: "projects",
      disabled: !data.value.projects.length,
      icon: () => h(Folder, { size: 14, "aria-hidden": "true" }),
      children: data.value.projects.map((project) => ({
        label: project.name,
        key: "assign:" + project.id,
        disabled: item.projectId === project.id,
      })),
    },
    {
      label: "从这个项目中移除",
      key: "unassign",
      disabled: !item.projectId,
      icon: () => h(FolderMinus, { size: 14, "aria-hidden": "true" }),
    },
  ];
  const shortcuts = { ...defaultShortcuts, ...data.value.shortcuts };
  const keys =
    draftMenu.value?.kind === "title"
      ? [
          "rename",
          "pin",
          "favorite",
          "result-divider",
          "project-divider",
          "current-project",
          "projects",
          "unassign",
          "delete-divider",
          "delete",
        ]
      : [
          "rename",
          "pin",
          "favorite",
          "result-divider",
          "duplicate",
          "project-divider",
          "projects",
          "unassign",
          "delete-divider",
          "delete",
        ];
  const menuOptions = keys.flatMap((key) => {
    const option = options.find((entry) => entry.key === key);
    if (!option || (key === "unassign" && !item.projectId)) return [];
    return [option];
  });
  return menuOptions.map((option) => {
    if (
      option.type ||
      !["rename", "pin", "favorite", "delete", "duplicate"].includes(
        String(option.key),
      )
    )
      return option;
    const label = option.label;
    return {
      ...option,
      label: () =>
        h("div", { class: "draft-menu-label" }, [
          typeof label === "function" ? label() : label,
          h(
            "span",
            { class: "draft-menu-shortcut", "aria-hidden": "true" },
            shortcutLabels(
              shortcuts[
                (option.key === "duplicate" ? "copyResult" : option.key) as
                  "rename" | "pin" | "favorite" | "delete" | "copyResult"
              ],
            ).join("+"),
          ),
        ]),
    };
  });
});

function openTitleMenu(event: MouseEvent) {
  if (!draft.value) return;
  if (
    draftMenuOpen.value &&
    draftMenu.value?.kind === "title" &&
    draftMenu.value.id === draft.value.id
  ) {
    closeDraftMenu();
    return;
  }
  const target = (event.currentTarget as HTMLElement).getBoundingClientRect();
  draftMenu.value = {
    id: draft.value.id,
    x: target.left,
    y: target.bottom + 4,
    kind: "title",
  };
  draftMenuOpen.value = true;
}

function openDraftMenu(id: string, event: MouseEvent) {
  const target = (
    event.currentTarget as HTMLElement | null
  )?.getBoundingClientRect();
  const byKeyboard = event.clientX === 0 && event.clientY === 0;
  draftMenu.value = {
    id,
    x: byKeyboard ? (target?.left ?? 0) + 12 : event.clientX,
    y: byKeyboard ? (target?.bottom ?? 0) : event.clientY,
    kind: "context",
  };
  draftMenuOpen.value = true;
}

function closeDraftMenu() {
  draftMenuOpen.value = false;
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

function copyDraft(item: NonNullable<typeof draft.value>) {
  const copy = duplicateDraft(item);
  data.value.drafts.push(copy);
  loadDraft(copy.id);
  message.success("已创建草稿副本");
}

function runDraftMenu(key: string | number) {
  const item = menuDraft.value;
  closeDraftMenu();
  if (!item || typeof key !== "string") return;
  switch (key) {
    case "duplicate": {
      copyDraft(item);
      break;
    }
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
    "将永久删除「" + (item.title || "未命名草稿") + "」和它的全部聊天记录。",
    async () => {
      await cancel(item.id);
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
  content: string | (() => ReturnType<typeof h>),
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
  const startingChat = mode === "chat" && !item.result && !item.messages.length;
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
    const conversation = prepareConversation(item, template, instruction, mode);
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
      message.error((item.title || "未命名草稿") + "：" + errorMessage(error), {
        duration: 9000,
        closable: true,
      });
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

async function copyText(content: string) {
  try {
    await writeText(content);
    message.success("已复制");
  } catch (error) {
    message.error(errorMessage(error));
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

function backup() {
  confirm(
    "导出完整备份",
    () =>
      h(
        "div",
        {
          style: {
            display: "flex",
            flexDirection: "column",
            gap: "16px",
            lineHeight: "1.7",
          },
        },
        [
          h(
            "p",
            { style: { margin: "0" } },
            "导出为 JSON 文件，可在新电脑上恢复完整数据。",
          ),
          h(
            "ul",
            {
              style: {
                margin: "0",
                paddingLeft: "20px",
                display: "grid",
                gap: "6px",
              },
            },
            [
              h("li", "项目、草稿、聊天记录及用户模板"),
              h("li", "供应商配置、API Key 及应用设置"),
              h("li", "用量记录、价格目录及目录缓存"),
            ],
          ),
          h(
            "div",
            { style: { display: "flex", flexDirection: "column", gap: "4px" } },
            [
              h(
                NText,
                { type: "warning", strong: true, style: { fontSize: "15px" } },
                { default: () => "API Key 将以明文保存" },
              ),
              h(
                NText,
                { depth: 2 },
                { default: () => "请妥善保管备份文件，不要公开分享。" },
              ),
            ],
          ),
        ],
      ),
    async () => {
      if (hasRequests.value) throw new Error("请等待当前优化完成后再备份。");
      await save();
      if (saveError.value) throw new Error(saveError.value);
      const path = await pickSave({
        defaultPath: "词稿完整备份.json",
        filters: [{ name: "词稿完整备份", extensions: ["json"] }],
      });
      if (!path) return;
      await desktop.exportBackup(path);
      message.success("完整备份已导出");
    },
    "继续导出",
  );
}

async function restore() {
  try {
    if (hasRequests.value) throw new Error("请等待当前优化完成后再恢复。");
    const path = await open({
      multiple: false,
      filters: [{ name: "词稿完整备份", extensions: ["json"] }],
    });
    if (!path) return;
    const incoming = await desktop.previewBackup(path);
    confirm(
      "恢复完整备份",
      () =>
        h(
          "div",
          {
            style: {
              display: "flex",
              flexDirection: "column",
              gap: "16px",
              lineHeight: "1.7",
            },
          },
          [
            h(
              "p",
              { style: { margin: "0" } },
              "备份已通过校验，将恢复以下数据及全部配置。",
            ),
            h(
              "ul",
              {
                style: {
                  margin: "0",
                  paddingLeft: "20px",
                  display: "grid",
                  gap: "6px",
                },
              },
              [
                h(
                  "li",
                  `${incoming.projects} 个项目、${incoming.drafts} 份草稿、${incoming.messages} 条聊天消息`,
                ),
                h(
                  "li",
                  `${incoming.providers} 个供应商及 API Key、用户模板和应用设置`,
                ),
                h(
                  "li",
                  `${incoming.usage} 条用量记录、${incoming.prices} 条价格及目录缓存`,
                ),
              ],
            ),
            h(
              "div",
              {
                style: { display: "flex", flexDirection: "column", gap: "4px" },
              },
              [
                h(
                  NText,
                  {
                    type: "warning",
                    strong: true,
                    style: { fontSize: "15px" },
                  },
                  {
                    default: () =>
                      "将覆盖当前全部数据、API Key 和设置，不会合并",
                  },
                ),
                h(
                  NText,
                  { depth: 2 },
                  { default: () => "建议先导出当前备份，再执行恢复。" },
                ),
              ],
            ),
            h(
              NText,
              { depth: 3, style: { fontSize: "12px" } },
              {
                default: () =>
                  "恢复成功后界面将重新加载。新电脑的开机自启需要重新设置。",
              },
            ),
          ],
        ),
      async () => {
        if (hasRequests.value) throw new Error("请等待当前优化完成后再恢复。");
        await save();
        if (saveError.value) throw new Error(saveError.value);
        await restoreBackup(path);
      },
      "覆盖并恢复",
    );
  } catch (error) {
    message.error(errorMessage(error));
  }
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
</script>

<template>
  <div
    class="shell"
    @pointerdown.capture="keyboardNavigation = false"
    @keydown.capture="
      (event) => {
        if (event.key === 'Tab') keyboardNavigation = true;
      }
    "
    :class="{
      'keyboard-navigation': keyboardNavigation,
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
        :sending-draft-ids="sendingDraftIds"
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
      <main
        class="main-surface"
        :class="{
          'draft-surface': draft && !utilities && !showLibrary,
        }"
      >
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
                    <div
                      class="template-field"
                      @wheel.capture="routeTemplateWheel"
                    >
                      <div class="template-field-actions">
                        <n-button
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
                        <n-button
                          quaternary
                          size="small"
                          :disabled="!templateDisplayText(field.role)"
                          :aria-label="'复制' + field.label"
                          :title="'复制' + field.label"
                          @click="copyTemplateText(field.role)"
                        >
                          <template #icon><Copy aria-hidden="true" /></template>
                        </n-button>
                        <n-button
                          quaternary
                          size="small"
                          :disabled="!templateDisplayText(field.role)"
                          :aria-label="'放大查看' + field.label"
                          :title="'放大查看' + field.label"
                          @click="enlargedTemplateRole = field.role"
                        >
                          <template #icon
                            ><Maximize2 aria-hidden="true"
                          /></template>
                        </n-button>
                      </div>
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
                  <div v-if="!viewingBuiltin" class="template-actions">
                    <n-button @click="editingTemplate = undefined"
                      >取消</n-button
                    >
                    <n-button type="primary" attr-type="submit">保存</n-button>
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
                        :disabled="defaultTemplateId === item.id"
                        :aria-label="'将 ' + item.name + ' 设为默认模板'"
                        @click="setDefaultTemplate(item.id)"
                        >{{
                          defaultTemplateId === item.id
                            ? "默认模板"
                            : "设为默认"
                        }}</n-button
                      >
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
              @default="setDefaultProfile"
              @theme="(value) => (data.theme = value)"
            />
          </div>
        </section>
        <div v-else-if="!draft" class="welcome">
          <div class="welcome-copy">
            <img class="welcome-logo" :src="appIcon" alt="词稿" />
            <h1>你想优化什么提示词？</h1>
            <p>新建一份草稿，写下需求或粘贴已有提示词。</p>
            <n-button type="primary" @click="create()">
              <template #icon><Plus aria-hidden="true" /></template>新建草稿
            </n-button>
          </div>
        </div>
        <template v-else>
          <prompt-workspace
            :draft="draft"
            :request="activeRequest"
            :context-usage="contextUsage"
            :send-shortcut="
              data.shortcuts?.optimize ?? defaultShortcuts.optimize
            "
            :template-name="selectedTemplate?.name"
            @generate="optimize()"
            @copy="copyText"
            @download="
              (content) =>
                exportFile('md', content).catch((error) =>
                  message.error(errorMessage(error)),
                )
            "
            @expand="showResult = true"
            @cancel="cancel()"
          >
            <template #heading>
              <button
                type="button"
                class="draft-heading"
                aria-haspopup="menu"
                :aria-expanded="
                  draftMenuOpen &&
                  draftMenu?.kind === 'title' &&
                  draftMenu.id === draft.id
                "
                @click.stop="openTitleMenu"
              >
                <n-tooltip
                  v-if="draftProject"
                  trigger="hover"
                  :style="{ maxWidth: '300px' }"
                >
                  <template #trigger>
                    <span
                      class="draft-project-icon"
                      :aria-label="'所属项目：' + draftProject.name"
                    >
                      <Folder :size="18" aria-hidden="true" />
                    </span>
                  </template>
                  所属项目：{{ draftProject.name }}
                </n-tooltip>
                <span class="draft-title">{{
                  draft.title || "未命名草稿"
                }}</span>
                <ChevronDown
                  :size="16"
                  class="draft-title-chevron"
                  aria-hidden="true"
                />
              </button>
            </template>
            <template #template
              ><n-dropdown
                trigger="click"
                :options="templateOptions"
                :value="selectedTemplate?.id ?? null"
                :menu-props="() => ({ class: 'optimization-template-menu' })"
                :disabled="!templateOptions.length"
                @select="selectOptimizationTemplate"
              >
                <n-button
                  class="template-menu-trigger"
                  :aria-label="
                    '优化模板：' +
                    (selectedTemplate?.name ?? '未选择') +
                    (selectedTemplate?.description
                      ? '，' + selectedTemplate.description
                      : '')
                  "
                  aria-haspopup="menu"
                  :disabled="!templateOptions.length"
                >
                  <span>{{
                    selectedTemplate?.name ?? "选择模板"
                  }}</span><ChevronDown :size="14" aria-hidden="true" />
                </n-button>
              </n-dropdown>
            </template>
            <template #actions
              ><n-dropdown
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
              >
                <n-button
                  quaternary
                  :disabled="!draft.result.trim()"
                  size="small"
                  title="导出优化稿"
                  aria-label="导出当前稿"
                  ><template #icon><Download aria-hidden="true" /></template
                ></n-button> </n-dropdown
            ></template>
            <template #models
              ><n-select
                class="workbench-provider"
                :consistent-menu-width="false"
                :menu-props="{
                  style: {
                    width: '160px',
                    maxWidth: 'calc(100vw - 32px)',
                  },
                }"
                :style="{
                  width: `${Math.min(220, (profile?.name.length ?? 5) * 14 + 48)}px`,
                }"
                aria-label="本次优化供应商"
                placeholder="选择供应商"
                :value="profile?.id ?? null"
                :options="
                  models.map((item) => ({
                    label: item.name,
                    value: item.id,
                  }))
                "
                :disabled="!models.length"
                @update:value="selectProvider"
              />
              <n-select
                class="workbench-model"
                :consistent-menu-width="false"
                :menu-props="{
                  style: {
                    width: '160px',
                    maxWidth: 'calc(100vw - 32px)',
                  },
                }"
                :style="{
                  width: `${Math.min(240, Math.max(110, (selectedModel?.length ?? 8) * 7.5 + 32))}px`,
                }"
                aria-label="本次优化模型"
                placeholder="选择模型"
                filterable
                :value="selectedModel"
                :options="modelOptions"
                :disabled="!profile"
                @update:value="(model) => (selectedModel = model)"
              />
            </template>
          </prompt-workspace>
        </template>
      </main>
    </div>
    <n-dropdown
      trigger="manual"
      size="small"
      :theme-overrides="{ fontSizeSmall: '12px', optionHeightSmall: '28px' }"
      placement="bottom-start"
      :show="draftMenuOpen && !!menuDraft"
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
      :show="!!enlargedTemplateRole"
      @update:show="
        (show) => {
          if (!show) enlargedTemplateRole = undefined;
        }
      "
      preset="card"
      :title="enlargedTemplateTitle"
      to=".app-root"
      class="result-dialog"
      closable
    >
      <template #header-extra>
        <n-button
          v-if="enlargedTemplateRole"
          size="tiny"
          :secondary="templatePreviewOpen[enlargedTemplateRole]"
          :quaternary="!templatePreviewOpen[enlargedTemplateRole]"
          :aria-label="enlargedTemplateTitle + '预览'"
          :aria-pressed="templatePreviewOpen[enlargedTemplateRole]"
          @click="
            templatePreviewOpen[enlargedTemplateRole] =
              !templatePreviewOpen[enlargedTemplateRole]
          "
          >{{
            templatePreviewOpen[enlargedTemplateRole]
              ? viewingBuiltin
                ? "返回原文"
                : "返回编辑"
              : "预览"
          }}</n-button
        >

        <n-button
          quaternary
          size="small"
          :disabled="
            !enlargedTemplateRole || !templateDisplayText(enlargedTemplateRole)
          "
          :aria-label="'复制' + enlargedTemplateTitle"
          :title="'复制' + enlargedTemplateTitle"
          @click="
            enlargedTemplateRole && copyTemplateText(enlargedTemplateRole)
          "
        >
          <template #icon><Copy aria-hidden="true" /></template>
        </n-button>
      </template>
      <div
        v-if="
          enlargedTemplateRole &&
          templatePreviewOpen[enlargedTemplateRole] &&
          !templatePreviewError
        "
        class="result-preview result-content"
        tabindex="0"
        :aria-label="enlargedTemplateTitle + '预览'"
        v-html="markdown.render(templateDisplayText(enlargedTemplateRole))"
      ></div>
      <pre
        v-else-if="enlargedTemplateRole"
        class="result-content result-dialog-source"
        tabindex="0"
        :aria-label="enlargedTemplateTitle"
        >{{
          (templatePreviewOpen[enlargedTemplateRole] && templatePreviewError) ||
          templateDisplayText(enlargedTemplateRole)
        }}</pre>
    </n-modal>
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
          :source="comparisonSource"
          :result="draft.result"
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
          :sending-draft-ids="sendingDraftIds"
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
    <p v-if="hasRequests" class="close-window-description" role="status">
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
