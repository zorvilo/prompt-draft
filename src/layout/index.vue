<template>
  <app-update-dialog :updater="appUpdate" />
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
      <AppSidebar
        class="desktop-library"
        :data="data"
        :active-utility="utilities"
        :update-available="!!appUpdate.updateInfo.value"
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
        @update="openUpdateAbout"
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
          <n-button size="small" @click="save()">重试保存</n-button></n-alert
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
            <router-view />
          </div>
        </section>
        <router-view v-else />
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
    <n-drawer v-model:show="showLibrary" :width="285" placement="left"
      ><n-drawer-content title="资料库" closable :native-scrollbar="false">
        <AppSidebar
          :data="data"
          :active-utility="utilities"
          :update-available="!!appUpdate.updateInfo.value"
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
          @update="openUpdateAbout"
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

<script setup lang="ts">
defineOptions({ name: "AppLayout" });
import { ArrowLeft, Menu } from "lucide-vue-next";
import {
  NAlert,
  NButton,
  NCheckbox,
  NDrawer,
  NDrawerContent,
  NDropdown,
  NModal,
} from "naive-ui";
import { provide } from "vue";
import { RouterView } from "vue-router";
import AppUpdateDialog from "./components/AppUpdateDialog.vue";
import AppSidebar from "./components/AppSidebar.vue";
import WindowTitlebar from "./components/WindowTitlebar.vue";
import { useWorkspaceShell } from "./hooks/useWorkspaceShell";
import { workspaceContextKey } from "./hooks/workspaceContext";
const shell = useWorkspaceShell();
provide(workspaceContextKey, shell);
const {
  appUpdate,
  keyboardNavigation,
  utilities,
  editingTemplate,
  showLibrary,
  data,
  draft,
  sendingDraftIds,
  recentExpanded,
  pinnedExpanded,
  draftListMode,
  create,
  deleteDraft,
  togglePin,
  openDraftMenu,
  loadDraft,
  openUtilities,
  openUpdateAbout,
  saveError,
  save,
  utilitiesTitle,
  usageCatalog,
  libraryCategory,
  draftMenuOpen,
  menuDraft,
  draftMenu,
  draftMenuOptions,
  runDraftMenu,
  closeDraftMenu,
  showClose,
  closing,
  hasRequests,
  rememberClose,
  completeClose,
} = shell;
</script>

<style scoped>
.usage-catalog-action {
  margin-left: auto;
}
</style>
