<script setup lang="ts">
import { Box, Info, Plug, RotateCcw, LayoutTemplate } from "lucide-vue-next";
import { NTooltip } from "naive-ui";
import type { ChatNotice } from "./chatNotices";
defineProps<{ notice: ChatNotice }>();
</script>

<template>
  <div class="chat-event-notice" role="note">
    <div class="chat-event-notice-content">
      <component
        :is="
          notice.kind === 'model'
            ? Box
            : notice.kind === 'template'
              ? LayoutTemplate
              : notice.kind === 'provider'
                ? Plug
                : RotateCcw
        "
        :size="14"
        aria-hidden="true"
      />
      <span>{{ notice.text }}</span>
      <n-tooltip
        trigger="hover"
        :style="{
          maxWidth: 'min(360px, calc(100vw - 32px))',
          whiteSpace: 'pre-line',
          overflowWrap: 'anywhere',
          lineHeight: '1.7',
        }"
      >
        <template #trigger>
          <button
            type="button"
            class="chat-event-info"
            :aria-label="notice.detail"
          >
            <Info :size="12" aria-hidden="true" />
          </button>
        </template>
        {{ notice.detail }}
      </n-tooltip>
    </div>
  </div>
</template>
