<template>
  <div class="settings" :class="{ 'is-about': settingsTab === 'about' }">
    <div class="preferences">
      <n-tabs
        v-model:value="settingsTab"
        type="segment"
        class="preferences-tabs"
        pane-class="settings-scroll"
        :animated="false"
      >
        <n-tab-pane name="general"
          ><template #tab
            ><SlidersHorizontal :size="15" aria-hidden="true" />常规</template
          ><GeneralSettings
        /></n-tab-pane>
        <n-tab-pane name="data"
          ><template #tab
            ><Database :size="15" aria-hidden="true" />数据与备份</template
          ><DataSettings
        /></n-tab-pane>
        <n-tab-pane name="about"
          ><template #tab><Info :size="15" aria-hidden="true" />关于</template
          ><AboutSettings
        /></n-tab-pane>
      </n-tabs>
    </div>
  </div>
</template>

<script setup lang="ts">
defineOptions({ name: "SettingsPage" });
import { useWorkspaceContext } from "@/layout/hooks/workspaceContext";
import { Database, Info, SlidersHorizontal } from "lucide-vue-next";
import { NTabPane, NTabs } from "naive-ui";
import { ref, watch } from "vue";
import AboutSettings from "./components/AboutSettings.vue";
import DataSettings from "./components/DataSettings.vue";
import GeneralSettings from "./components/GeneralSettings.vue";
const { updateAboutRequest, appUpdate } = useWorkspaceContext();
const settingsTab = ref("general");
watch(
  updateAboutRequest,
  (request) => {
    settingsTab.value = request > 0 ? "about" : "general";
    if (request > 0) void appUpdate.checkUpdate();
  },
  { immediate: true },
);
</script>
