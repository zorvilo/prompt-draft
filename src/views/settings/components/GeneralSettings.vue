<template>
  <div class="preferences-body">
    <section class="preference-group" aria-labelledby="appearance-heading">
      <h2 id="appearance-heading">外观</h2>
      <div class="preference-card">
        <div class="preference-row">
          <div>
            <h3>主题样式</h3>
            <p>跟随系统时，随系统外观自动切换。</p>
          </div>
          <n-radio-group
            :value="data.theme"
            class="theme-options"
            :theme-overrides="{
              buttonHeightSmall: '34px',
              buttonBoxShadowFocus: 'none',
              buttonColorActive: 'var(--selection)',
              buttonTextColorActive: 'var(--ink)',
            }"
            size="small"
            aria-label="主题样式"
            @update:value="(value: Workspace['theme']) => (data.theme = value)"
          >
            <n-radio-button value="system"
              ><span class="theme-option"
                ><Monitor :size="16" aria-hidden="true" />跟随系统</span
              ></n-radio-button
            >
            <n-radio-button value="light"
              ><span class="theme-option"
                ><Sun :size="16" aria-hidden="true" />浅色</span
              ></n-radio-button
            >
            <n-radio-button value="dark"
              ><span class="theme-option"
                ><Moon :size="16" aria-hidden="true" />深色</span
              ></n-radio-button
            >
          </n-radio-group>
        </div>
      </div>
    </section>
    <section class="preference-group" aria-labelledby="window-heading">
      <h2 id="window-heading">窗口行为</h2>
      <div class="preference-card">
        <div class="preference-row">
          <div>
            <h3>开机自启</h3>
            <p>登录系统后自动启动词稿。</p>
            <p v-if="autostartError" role="alert">
              {{ autostartError }}
            </p>
          </div>
          <div class="shortcut-actions">
            <n-button v-if="autostartError" size="small" @click="loadAutostart"
              >重试</n-button
            ><n-switch
              :value="autostartEnabled"
              :loading="autostartBusy"
              :disabled="autostartBusy || !!autostartError"
              aria-label="开机自启"
              @update:value="changeAutostart"
            />
          </div>
        </div>
        <div class="preference-row window-behavior-row">
          <div>
            <h3>关闭时最小化到托盘</h3>
            <p>选择点击窗口关闭按钮时的行为。</p>
          </div>
          <n-radio-group
            :value="data.closeBehavior ?? 'ask'"
            size="small"
            aria-label="关闭窗口行为"
            @update:value="
              (value: Workspace['closeBehavior']) =>
                (data.closeBehavior = value)
            "
          >
            <n-radio-button value="ask">每次询问</n-radio-button
            ><n-radio-button value="hide">隐藏到托盘</n-radio-button
            ><n-radio-button value="exit">退出程序</n-radio-button>
          </n-radio-group>
        </div>
      </div>
    </section>
    <shortcut-settings :data="data" :update-shortcuts="updateShortcuts" />
  </div>
</template>

<script setup lang="ts">
import { Monitor, Moon, Sun } from "lucide-vue-next";
import {
  NButton,
  NRadioButton,
  NRadioGroup,
  NSwitch,
  useMessage,
} from "naive-ui";
import { ref } from "vue";
import { desktop, errorMessage, type Workspace } from "@/api/desktop";
import ShortcutSettings from "./ShortcutSettings.vue";

import { useWorkspaceContext } from "@/layout/hooks/workspaceContext";
import { onMounted } from "vue";
const { data, updateShortcuts } = useWorkspaceContext();
const message = useMessage();
const autostartEnabled = ref(false);
const autostartBusy = ref(false);
const autostartError = ref("");
async function loadAutostart() {
  autostartBusy.value = true;
  autostartError.value = "";
  try {
    autostartEnabled.value = await desktop.autostart();
  } catch (error) {
    autostartError.value = errorMessage(error);
  } finally {
    autostartBusy.value = false;
  }
}
async function changeAutostart(value: boolean) {
  if (autostartBusy.value) return;
  autostartBusy.value = true;
  try {
    autostartEnabled.value = await desktop.autostart(value);
    message.success(value ? "已开启开机自启" : "已关闭开机自启");
  } catch (error) {
    message.error(errorMessage(error));
  } finally {
    autostartBusy.value = false;
  }
}

onMounted(() => {
  void loadAutostart();
});
</script>
