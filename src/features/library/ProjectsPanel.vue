<script setup lang="ts">
import { computed } from "vue";
import { FileText, Folder, Pencil, Plus, Trash2 } from "lucide-vue-next";
import { NButton, NCollapse, NCollapseItem } from "naive-ui";
import type { Draft, Project } from "../../lib/tauri";

const props = defineProps<{ projects: Project[]; drafts: Draft[] }>();
const emit = defineEmits<{
  addProject: [];
  deleteProject: [id: string];
  createDraft: [projectId: string];
  openDraft: [id: string];
  draftContext: [id: string, event: MouseEvent];
}>();
const expanded = defineModel<string[]>("expanded", { required: true });
const draftsByProject = computed(() => {
  const groups = new Map<string, Draft[]>();
  for (const draft of [...props.drafts].sort(
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

<template>
  <div class="projects-panel">
    <div class="page-actions">
      <n-button type="primary" @click="emit('addProject')">
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
            <n-button
              quaternary
              size="tiny"
              @click.stop="emit('createDraft', project.id)"
            >
              <template #icon><Plus aria-hidden="true" /></template>
              新建草稿
            </n-button>
            <n-button
              v-if="projects.length > 1"
              quaternary
              size="tiny"
              :aria-label="'删除项目 ' + project.name"
              title="删除项目"
              @click.stop="emit('deleteProject', project.id)"
              ><template #icon><Trash2 aria-hidden="true" /></template
            ></n-button>
          </div>
        </template>
        <ul
          v-if="draftsByProject.get(project.id)?.length"
          class="project-drafts"
        >
          <li
            v-for="draft in draftsByProject.get(project.id)"
            :key="draft.id"
            @contextmenu.prevent.stop="emit('draftContext', draft.id, $event)"
          >
            <n-button
              text
              class="project-draft-title"
              @click="emit('openDraft', draft.id)"
            >
              <template #icon><FileText aria-hidden="true" /></template>
              {{ draft.title || "未命名草稿" }}
            </n-button>
            <n-button
              quaternary
              size="small"
              :aria-label="'编辑 ' + (draft.title || '未命名草稿')"
              @click="emit('openDraft', draft.id)"
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
