<template>
  <div class="about-page">
    <section class="about-identity" aria-labelledby="about-heading">
      <img :src="appIcon" alt="" class="about-logo" />
      <h2 id="about-heading">词稿</h2>
      <span class="about-version">版本 {{ version }}</span>
      <p>专注提示词的编写、优化与管理。</p>
    </section>
    <section
      class="about-updates"
      aria-labelledby="update-heading"
      :aria-busy="updateBusy"
    >
      <div class="about-update-heading">
        <div class="about-update-status" role="status" aria-live="polite">
          <CircleAlert v-if="updateError" :size="20" aria-hidden="true" />
          <Download v-else-if="updateInfo" :size="20" aria-hidden="true" />
          <RefreshCw v-else-if="updateBusy" :size="20" aria-hidden="true" />
          <CircleCheck v-else :size="20" aria-hidden="true" />
          <h3 id="update-heading">{{ updateStatus }}</h3>
        </div>
        <n-button
          size="small"
          :loading="updateBusy && updateStage === 'check'"
          :disabled="updateBusy"
          @click="checkUpdate()"
          ><template #icon><RefreshCw :size="15" aria-hidden="true" /></template
          >{{ updateError ? "重试检查" : "重新检查" }}</n-button
        >
      </div>
      <p
        v-if="updateError && !updateInfo"
        class="about-update-error"
        role="alert"
      >
        {{ updateError }}
      </p>
    </section>
    <div class="about-source preference-row window-behavior-row">
      <div class="about-half">
        <h3>更新来源</h3>
        <p>选择检查和下载更新的来源，首次下载时可记住选择。</p>
      </div>
      <div class="about-half">
        <div class="about-column-action">
          <n-radio-group
            :value="data.updateSource ?? 'ask'"
            size="small"
            aria-label="更新来源"
            :disabled="updateBusy"
            @update:value="changeUpdateSource"
          >
            <n-radio-button value="ask">每次询问</n-radio-button>
            <n-radio-button value="gitee">Gitee 源</n-radio-button>
            <n-radio-button value="github">GitHub 源</n-radio-button>
          </n-radio-group>
        </div>
      </div>
    </div>
    <div class="about-community">
      <section class="about-repository" aria-labelledby="repository-heading">
        <h3 id="repository-heading">开源仓库</h3>
        <p>查看源码与反馈问题，欢迎 Star 支持。</p>
        <n-space justify="start">
          <n-button @click="openRepository('gitee')"
            ><template #icon
              ><ExternalLink :size="16" aria-hidden="true" /></template
            >Gitee 仓库</n-button
          >
          <n-button @click="openRepository('github')"
            ><template #icon
              ><ExternalLink :size="16" aria-hidden="true" /></template
            >GitHub 仓库</n-button
          >
        </n-space>
      </section>
      <section class="about-contact-row" aria-labelledby="contact-heading">
        <div class="about-contact-heading">
          <h3 id="contact-heading">联系作者</h3>
          <p>刘涵宇 · 开发者，欢迎交流使用建议。</p>
        </div>
        <div class="about-contact-details">
          <n-tooltip :delay="250" placement="top">
            <template #trigger>
              <n-button
                class="about-contact"
                :icon-size="24"
                aria-label="复制作者 QQ 号 2510039438"
                @click="copyAboutInfo('qq')"
              >
                <template #icon>
                  <img
                    :src="qqIcon"
                    class="qq-brand"
                    alt=""
                    aria-hidden="true"
                  />
                </template>
                <span class="about-contact-number">{{ authorQQ }}</span>
                <Copy
                  :size="14"
                  class="about-contact-copy"
                  aria-hidden="true"
                />
              </n-button>
            </template>
            点击复制 QQ 号
          </n-tooltip>
        </div>
      </section>
    </div>
    <section class="about-support" aria-labelledby="support-heading">
      <div class="about-support-heading">
        <div class="about-half">
          <h3 id="support-heading">支持信息</h3>
          <p>反馈问题时，可复制以下环境信息。</p>
        </div>
        <div class="about-half">
          <div class="about-column-action">
            <n-button
              size="small"
              :disabled="!supportInfo"
              @click="copyAboutInfo('support')"
              ><template #icon><Copy :size="14" aria-hidden="true" /></template
              >复制支持信息</n-button
            >
          </div>
        </div>
      </div>
      <div class="about-support-grid">
        <div v-for="field in supportFields" :key="field.label">
          <span>{{ field.label }}</span><strong>{{ field.value }}</strong>
        </div>
      </div>
      <div v-if="supportError" class="about-update-error" role="alert">
        {{ supportError }}
        <n-button text size="small" @click="loadSupportInfo">重新获取</n-button>
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import {
  CircleAlert,
  CircleCheck,
  Copy,
  Download,
  ExternalLink,
  RefreshCw,
} from "lucide-vue-next";
import {
  NButton,
  NRadioButton,
  NRadioGroup,
  NSpace,
  NTooltip,
  useMessage,
} from "naive-ui";
import { computed, ref } from "vue";
import { version } from "../../../../package.json";
import appIcon from "../../../../src-tauri/icons/icon.png";
import { desktop, errorMessage, type SupportInfo } from "@/api/desktop";
import qqIcon from "@/assets/brands/qq";

import { useWorkspaceContext } from "@/layout/hooks/workspaceContext";
import { onMounted } from "vue";
const { data, appUpdate } = useWorkspaceContext();
const {
  updateInfo,
  updateBusy,
  updateStage,
  updateError,
  checkUpdate,
  changeUpdateSource,
  updateStatus,
} = appUpdate;
const message = useMessage();
const authorQQ = "2510039438";
const supportInfo = ref<SupportInfo>();
const supportError = ref("");
async function loadSupportInfo() {
  supportError.value = "";
  try {
    supportInfo.value = await desktop.supportInfo();
  } catch (error) {
    supportError.value = errorMessage(error);
  }
}
const supportFields = computed(() => [
  { label: "应用版本", value: supportInfo.value?.version ?? version },
  { label: "运行环境", value: supportInfo.value?.runtime ?? "—" },
  { label: "操作系统", value: supportInfo.value?.os ?? "—" },
  { label: "系统架构", value: supportInfo.value?.arch ?? "—" },
]);
async function copyAboutInfo(kind: "qq" | "support") {
  if (kind === "support" && !supportInfo.value) return;
  try {
    await writeText(
      kind === "qq"
        ? authorQQ
        : [
            "词稿支持信息",
            ...supportFields.value.map(
              (field) => `${field.label}：${field.value}`,
            ),
          ].join("\n"),
    );
    message.success(kind === "qq" ? "已复制 QQ 号" : "已复制支持信息");
  } catch (error) {
    message.error(errorMessage(error));
  }
}
async function openRepository(source: "gitee" | "github") {
  try {
    await desktop.openProjectRepository(source);
  } catch (error) {
    message.error(errorMessage(error));
  }
}

onMounted(() => {
  void loadSupportInfo();
});
</script>
