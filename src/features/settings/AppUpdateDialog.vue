<script setup lang="ts">
import {
  NModal,
  NProgress,
  NRadioGroup,
  NRadioButton,
  NCheckbox,
  NSpace,
  NButton,
} from "naive-ui";
import type { AppUpdate } from "./useAppUpdate";
const props = defineProps<{ updater: AppUpdate }>();
const {
  updateInfo,
  showUpdateDialog,
  updateBusy,
  updateStage,
  updateError,
  updateProgress,
  selectedUpdateSource,
  rememberUpdateSource,
  downloadUpdate,
  installUpdate,
} = props.updater;
</script>
<template>
  <n-modal
    v-model:show="showUpdateDialog"
    preset="card"
    :title="`发现新版本 ${updateInfo?.version ?? ''}`"
    style="width: 560px; max-width: calc(100vw - 32px)"
    :closable="!updateBusy"
    :mask-closable="!updateBusy"
    :close-on-esc="!updateBusy"
  >
    <div v-if="updateInfo" class="update-dialog-content">
      <p v-if="updateInfo.notes" class="app-update-notes">
        {{ updateInfo.notes }}
      </p>
      <n-progress
        v-if="updateBusy && updateStage === 'download'"
        type="line"
        :percentage="updateProgress ?? 0"
        :processing="true"
        :show-indicator="updateProgress !== undefined"
      />
      <div v-if="!updateInfo.downloaded" class="update-dialog-source">
        <span>下载来源</span>
        <n-radio-group
          v-model:value="selectedUpdateSource"
          :disabled="updateBusy"
          aria-label="更新下载来源"
        >
          <n-radio-button value="gitee">Gitee（国内推荐）</n-radio-button>
          <n-radio-button value="github">GitHub</n-radio-button>
        </n-radio-group>
        <n-checkbox
          v-model:checked="rememberUpdateSource"
          :disabled="updateBusy"
          >记住选择，下次直接使用此来源</n-checkbox
        >
      </div>
      <p class="update-dialog-hint">
        {{
          updateInfo.downloaded
            ? "安装前请结束模型请求，应用会先保存当前数据。"
            : "下载并验证完成后，将自动保存数据、安装更新并重新启动。"
        }}
      </p>
    </div>
    <p v-if="updateError" class="about-update-error" role="alert">
      {{ updateError }}
    </p>
    <template #footer>
      <n-space justify="end">
        <n-button :disabled="updateBusy" @click="showUpdateDialog = false"
          >稍后再说</n-button
        >
        <n-button
          v-if="updateInfo && !updateInfo.downloaded"
          type="primary"
          :loading="updateBusy && updateStage === 'download'"
          :disabled="updateBusy"
          @click="downloadUpdate(selectedUpdateSource)"
          >下载更新</n-button
        >
        <n-button
          v-else-if="updateInfo"
          type="primary"
          :loading="updateBusy && updateStage === 'install'"
          :disabled="updateBusy"
          @click="installUpdate"
          >安装并重启</n-button
        >
      </n-space>
    </template>
  </n-modal>
</template>
