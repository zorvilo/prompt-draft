<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Copy, Minus, Square, X } from "lucide-vue-next";
import { NButton, useMessage } from "naive-ui";
import appIcon from "../../src-tauri/icons/icon.png";
import { errorMessage } from "../lib/tauri";

const desktopWindow = getCurrentWindow();
const message = useMessage();
const maximized = ref(false);
let unlisten: (() => void) | undefined;
let mounted = true;

async function updateMaximized() {
  const current = await desktopWindow.isMaximized();
  if (mounted) maximized.value = current;
}
async function run(action: () => Promise<void>) {
  try {
    await action();
  } catch (error) {
    if (mounted) message.error(errorMessage(error));
  }
}

onMounted(() => {
  void run(async () => {
    const remove = await desktopWindow.onResized(
      () => void run(updateMaximized),
    );
    if (!mounted) {
      remove();
      return;
    }
    unlisten = remove;
    await updateMaximized();
  });
});
onUnmounted(() => {
  mounted = false;
  unlisten?.();
});
</script>

<template>
  <header class="topbar" data-tauri-drag-region>
    <slot name="leading" />
    <div class="brand" data-tauri-drag-region>
      <img class="brand-mark" :src="appIcon" alt="" />
      <span>词稿</span>
    </div>
    <slot />
    <div class="titlebar-spacer" data-tauri-drag-region></div>
    <div class="window-actions" role="group" aria-label="窗口操作">
      <n-button
        quaternary
        class="window-action"
        aria-label="最小化"
        title="最小化"
        @click="run(() => desktopWindow.minimize())"
      >
        <template #icon><Minus :size="16" aria-hidden="true" /></template>
      </n-button>
      <n-button
        quaternary
        class="window-action"
        :aria-label="maximized ? '还原' : '最大化'"
        :title="maximized ? '还原' : '最大化'"
        @click="run(() => desktopWindow.toggleMaximize())"
      >
        <template #icon
          ><Copy v-if="maximized" :size="14" aria-hidden="true" /><Square
            v-else
            :size="14"
            aria-hidden="true"
        /></template>
      </n-button>
      <n-button
        quaternary
        class="window-action window-action-close"
        aria-label="关闭"
        title="关闭"
        @click="run(() => desktopWindow.close())"
      >
        <template #icon><X :size="17" aria-hidden="true" /></template>
      </n-button>
    </div>
  </header>
</template>
