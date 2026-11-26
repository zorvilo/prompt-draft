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
          <AppLayout v-if="ready" />
          <template v-else-if="startupError">
            <window-titlebar />
            <div class="startup">
              <n-alert type="error" :show-icon="false">{{
                startupError
              }}</n-alert>
              <n-button @click="initialize">重新加载</n-button>
            </div>
          </template>
        </div>
      </n-message-provider></n-dialog-provider
    >
  </n-config-provider>
</template>

<script setup lang="ts">
import {
  darkTheme,
  dateZhCN,
  NAlert,
  NButton,
  NConfigProvider,
  NDialogProvider,
  NMessageProvider,
  zhCN,
} from "naive-ui";
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { desktop } from "./api/desktop";
import WindowTitlebar from "./layout/components/WindowTitlebar.vue";
import AppLayout from "./layout/index.vue";
import { useWorkspace } from "./store/modules/workspace";

const workspace = useWorkspace();
const { data, ready, startupError, initialize } = workspace;
let startupShown = false;
watch(
  () => ready.value || Boolean(startupError.value),
  async (canShow) => {
    if (!canShow || startupShown) return;
    await nextTick();
    await desktop.windowAction("show");
    startupShown = true;
  },
  { flush: "post" },
);
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
