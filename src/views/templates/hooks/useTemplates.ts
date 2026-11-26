import {
  desktop,
  errorMessage,
  type PromptTemplate,
  type TemplateCategory,
} from "@/api/desktop";
import {
  availableTemplates,
  builtinCategories,
  isBuiltinCategory,
  isBuiltinTemplate,
} from "@/config/templates";
import { useWorkspace } from "@/store/modules/workspace";
import { renderTemplate } from "@/utils/template/renderTemplate";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { Check, CircleHelp } from "lucide-vue-next";
import {
  NInput,
  NTooltip,
  useDialog,
  useMessage,
  type DropdownOption,
} from "naive-ui";
import { computed, h, ref, watch } from "vue";

import { useConfirmDialog } from "@/components/useConfirmDialog";
export function useTemplates(workspace: ReturnType<typeof useWorkspace>) {
  const { data, draft, saveError, save } = workspace;
  const dialog = useDialog();
  const message = useMessage();
  const confirm = useConfirmDialog();

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
                          onClick: (event: MouseEvent) =>
                            event.stopPropagation(),
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
    () =>
      !!editingTemplate.value && isBuiltinTemplate(editingTemplate.value.id),
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

  const templateSaving = ref(false);
  async function saveTemplate() {
    const editing = editingTemplate.value;
    if (templateSaving.value || !editing || isBuiltinTemplate(editing.id))
      return;
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
    const previousTemplates = data.value.templates;
    const userTemplates = previousTemplates ?? [];
    const exists = userTemplates.some((entry) => entry.id === item.id);
    data.value.templates = exists
      ? userTemplates.map((entry) => (entry.id === item.id ? item : entry))
      : [...userTemplates, item];
    const attempted = data.value.templates;
    templateSaving.value = true;
    try {
      if (!(await save())) {
        if (data.value.templates === attempted)
          data.value.templates = previousTemplates;
        message.error(saveError.value || "模板保存失败，请重试。");
        return;
      }
      if (editingTemplate.value === editing) editingTemplate.value = undefined;
      message.success("模板已保存");
    } finally {
      templateSaving.value = false;
    }
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

  return {
    libraryCategory,
    libraryCategories,
    categoryOptions,
    defaultTemplateId,
    setDefaultTemplate,
    templateList,
    libraryTemplates,
    selectedTemplate,
    templateOptions,
    selectOptimizationTemplate,
    editingTemplate,
    viewingBuiltin,
    templateFields,
    templatePreviewOpen,
    templatePreview,
    templatePreviewError,
    enlargedTemplateRole,
    enlargedTemplateTitle,
    templateDisplayText,
    copyTemplateText,
    editTemplateCategory,
    deleteTemplateCategory,
    newTemplate,
    saveTemplate,
    templateSaving,
    copyTemplate,
    deleteTemplate,
  };
}
