import { defaultShortcuts, shortcutLabels } from "@/config/shortcuts";
import { useWorkspace } from "@/store/modules/workspace";
import { duplicateDraft } from "@/utils/workspace/duplicateDraft";
import {
  Copy,
  Folder,
  FolderMinus,
  Pencil,
  Pin,
  PinOff,
  Star,
  Trash2,
} from "lucide-vue-next";
import {
  NInput,
  NText,
  useDialog,
  useMessage,
  type DropdownOption,
} from "naive-ui";
import { computed, h, ref } from "vue";

import { useConfirmDialog } from "@/components/useConfirmDialog";
import type { Utility } from "@/router/routes";
import type { Ref, WritableComputedRef } from "vue";
export function useDraftActions(
  workspace: ReturnType<typeof useWorkspace>,
  utilities: WritableComputedRef<Utility | undefined>,
  showLibrary: Ref<boolean>,
  expandedProjects: WritableComputedRef<string[]>,
  cancel: (id?: string) => Promise<void>,
) {
  const { data, draft, newDraft, openDraft, save, saveError } = workspace;
  const dialog = useDialog();
  const message = useMessage();
  const confirm = useConfirmDialog();
  function create(projectId?: string) {
    utilities.value = undefined;
    newDraft(projectId);
    showLibrary.value = false;
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
    if (!project) return;
    const count = data.value.drafts.filter(
      (item) => item.projectId === id,
    ).length;
    confirm(
      "删除项目",
      `将删除「${project.name}」项目，其中 ${count} 份草稿将改为未分配，草稿内容和聊天记录都会保留。`,
      async () => {
        const previousProjects = data.value.projects;
        const previousExpanded = expandedProjects.value;
        const affectedDrafts = data.value.drafts.filter(
          (item) => item.projectId === id,
        );
        for (const item of affectedDrafts) item.projectId = null;
        data.value.projects = data.value.projects.filter(
          (item) => item.id !== id,
        );
        expandedProjects.value = expandedProjects.value.filter(
          (projectId) => projectId !== id,
        );
        if (!(await save())) {
          data.value.projects = previousProjects;
          expandedProjects.value = previousExpanded;
          for (const item of affectedDrafts) {
            if (data.value.drafts.includes(item) && item.projectId === null)
              item.projectId = id;
          }
          throw new Error(saveError.value || "删除项目失败，请重试。");
        }
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

  return {
    create,
    loadDraft,
    addProject,
    deleteProject,
    togglePin,
    draftMenuOpen,
    draftMenu,
    menuDraft,
    draftMenuOptions,
    openTitleMenu,
    openDraftMenu,
    closeDraftMenu,
    renameDraft,
    copyDraft,
    runDraftMenu,
    deleteDraft,
  };
}
