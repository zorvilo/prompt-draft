<template>
  <div v-if="!draft" class="welcome">
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
      :send-shortcut="data.shortcuts?.optimize ?? defaultShortcuts.optimize"
      :newline-bindings="
        [
          data.shortcuts?.newline ?? defaultShortcuts.newline,
          data.shortcuts?.newlineAlternate ?? defaultShortcuts.newlineAlternate,
        ].filter(Boolean)
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
          <span class="draft-title">{{ draft.title || "未命名草稿" }}</span>
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
            <span>{{ selectedTemplate?.name ?? "选择模板" }}</span><ChevronDown
              :size="14"
              aria-hidden="true"
            />
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
</template>

<script setup lang="ts">
defineOptions({ name: "WorkspacePage" });
import { errorMessage } from "@/api/desktop";
import { defaultShortcuts } from "@/config/shortcuts";
import { useWorkspaceContext } from "@/layout/hooks/workspaceContext";
import { ChevronDown, Copy, Download, Folder, Plus } from "lucide-vue-next";
import {
  NButton,
  NDropdown,
  NModal,
  NSelect,
  NTooltip,
  useMessage,
} from "naive-ui";
import appIcon from "../../../src-tauri/icons/icon.png";
import PromptWorkspace from "./components/PromptWorkspace.vue";
import ResultComparison from "./components/ResultComparison.vue";
const message = useMessage();
const {
  draft,
  create,
  activeRequest,
  contextUsage,
  data,
  selectedTemplate,
  optimize,
  copyText,
  exportFile,
  showResult,
  cancel,
  draftProject,
  draftMenuOpen,
  draftMenu,
  openTitleMenu,
  templateOptions,
  selectOptimizationTemplate,
  profile,
  models,
  selectProvider,
  selectedModel,
  modelOptions,
  copy,
  resultMode,
  comparisonSource,
  resultPreview,
} = useWorkspaceContext();
</script>
