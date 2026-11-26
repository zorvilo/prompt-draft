<template>
  <div class="projects-panel">
    <div class="page-actions">
      <n-button type="primary" @click="addProject()">
        <template #icon><Plus aria-hidden="true" /></template>
        新建项目
      </n-button>
    </div>
    <n-collapse v-model:expanded-names="expanded" class="projects-list">
      <n-collapse-item
        v-for="project in projects"
        :key="project.id"
        :name="project.id"
      >
        <template #header>
          <span class="project-heading">
            <Folder :size="16" aria-hidden="true" />
            <strong>{{ project.name }}</strong>
            <span class="muted"
              >{{ draftsByProject.get(project.id)?.length ?? 0 }} 份草稿</span
            >
          </span>
        </template>
        <template #header-extra>
          <div class="project-actions">
            <n-button quaternary size="tiny" @click.stop="create(project.id)">
              <template #icon><Plus aria-hidden="true" /></template>
              新建草稿
            </n-button>
            <n-tooltip :delay="250" placement="top">
              <template #trigger
                ><n-button
                  quaternary
                  size="tiny"
                  :aria-label="'删除项目 ' + project.name"
                  @click.stop="deleteProject(project.id)"
                  ><template #icon
                    ><Trash2 aria-hidden="true" /></template></n-button
              ></template>
              删除项目
            </n-tooltip>
          </div>
        </template>
        <ul
          v-if="draftsByProject.get(project.id)?.length"
          class="project-drafts"
        >
          <li
            v-for="draft in draftsByProject.get(project.id)"
            :key="draft.id"
            @contextmenu.prevent.stop="openDraftMenu(draft.id, $event)"
          >
            <n-button
              text
              class="project-draft-title"
              @click="loadDraft(draft.id)"
            >
              <template #icon><FileText aria-hidden="true" /></template>
              {{ draft.title || "未命名草稿" }}
            </n-button>
            <n-button
              quaternary
              size="small"
              :aria-label="'编辑 ' + (draft.title || '未命名草稿')"
              @click="loadDraft(draft.id)"
            >
              <template #icon><Pencil aria-hidden="true" /></template>
              编辑
            </n-button>
          </li>
        </ul>
        <p v-else class="muted">此项目暂无草稿。</p>
      </n-collapse-item>
    </n-collapse>
  </div>
</template>

<script setup lang="ts">
defineOptions({ name: "ProjectsPage" });
import { FileText, Folder, Pencil, Plus, Trash2 } from "lucide-vue-next";
import { NButton, NCollapse, NCollapseItem, NTooltip } from "naive-ui";
import { computed } from "vue";
import type { Draft } from "@/api/desktop";

import { useWorkspaceContext } from "@/layout/hooks/workspaceContext";
const {
  data,
  expandedProjects: expanded,
  addProject,
  deleteProject,
  create,
  loadDraft,
  openDraftMenu,
} = useWorkspaceContext();
const projects = computed(() => data.value.projects);
const draftsByProject = computed(() => {
  const groups = new Map<string, Draft[]>();
  for (const draft of [...data.value.drafts].sort(
    (a, b) => (b.updated ?? b.opened) - (a.updated ?? a.opened),
  )) {
    if (!draft.projectId) continue;
    const group = groups.get(draft.projectId) ?? [];
    group.push(draft);
    groups.set(draft.projectId, group);
  }
  return groups;
});
</script>
