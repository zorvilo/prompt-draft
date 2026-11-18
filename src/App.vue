<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import {
  darkTheme,
  zhCN,
  dateZhCN,
  NAlert,
  NButton,
  NConfigProvider,
  NDialogProvider,
  NMessageProvider,
} from "naive-ui";
import WindowTitlebar from "./components/WindowTitlebar.vue";
import WorkspaceView from "./features/workspace/WorkspaceView.vue";
import { useWorkspace } from "./features/workspace/workspace";

const workspace = useWorkspace();
const { data, ready, startupError, initialize } = workspace;
const media = matchMedia("(prefers-color-scheme: dark)");
const systemDark = ref(media.matches);
const updateTheme = () => {
  systemDark.value = media.matches;
};
const dark = computed(
  () =>
    data.value.theme === "dark" ||
    (data.value.theme === "system" && systemDark.value),
);
const themeOverrides = computed(() => {
  const accent = dark.value ? "#d0d4da" : "#384454";
  const hover = dark.value ? "#e1e4e8" : "#4b5869";
  return {
    common: {
      fontFamily: 'Inter, "Microsoft YaHei UI", sans-serif',
      primaryColor: accent,
      primaryColorHover: hover,
      primaryColorPressed: accent,
      primaryColorSuppl: hover,
      borderRadius: "7px",
    },
  };
});
function suppressContextMenu(event: MouseEvent) {
  event.preventDefault();
}
onMounted(() => {
  document.addEventListener("contextmenu", suppressContextMenu, true);
  media.addEventListener("change", updateTheme);
  void initialize();
});
onUnmounted(() => {
  media.removeEventListener("change", updateTheme);
  document.removeEventListener("contextmenu", suppressContextMenu, true);
});
</script>

<template>
  <n-config-provider
    :locale="zhCN"
    :date-locale="dateZhCN"
    :theme="dark ? darkTheme : null"
    :theme-overrides="themeOverrides"
  >
    <n-dialog-provider
      ><n-message-provider>
        <div class="app-root" :data-theme="dark ? 'dark' : 'light'">
          <workspace-view v-if="ready" :workspace="workspace" />
          <template v-else>
            <window-titlebar />
            <div class="startup">
              <template v-if="startupError">
                <n-alert type="error" :show-icon="false">{{
                  startupError
                }}</n-alert>
                <n-button @click="initialize">重新加载</n-button>
              </template>
              <p v-else role="status">正在打开本地工作区…</p>
            </div>
          </template>
        </div>
      </n-message-provider></n-dialog-provider
    >
  </n-config-provider>
</template>
