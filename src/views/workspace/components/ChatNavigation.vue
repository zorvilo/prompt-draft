<template>
  <nav
    v-if="entries.length"
    ref="rail"
    class="chat-navigation"
    aria-label="聊天消息导航"
  >
    <n-popover
      v-for="(entry, index) in entries"
      :key="entry.id"
      placement="right"
      :show="previewId === entry.id"
      :show-arrow="false"
      :style="{ maxWidth: 'min(320px, calc(100vw - 76px))' }"
      :to="'.app-root'"
      :keep-alive-on-hover="false"
    >
      <template #trigger>
        <button
          type="button"
          class="chat-navigation-tick"
          :class="{
            'is-visible': visibleIds?.includes(entry.id),
          }"
          :aria-label="`跳转到第 ${index + 1} 次提问：${entry.prompt.slice(0, 60)}`"
          :aria-description="
            visibleIds?.includes(entry.id) ? '该提问当前可见' : undefined
          "
          @mouseenter="previewId = entry.id"
          @mouseleave="previewId = undefined"
          @focus="previewId = entry.id"
          @blur="previewId = undefined"
          @keydown.esc="previewId = undefined"
          @click="
            previewId = undefined;
            emit('select', entry.id);
          "
        >
          <span aria-hidden="true"></span>
        </button>
      </template>
      <div class="chat-navigation-preview">
        <strong>{{ entry.prompt }}</strong>
        <p v-if="entry.reply">{{ entry.reply }}</p>
        <span
          >{{ entry.independent ? "独立优化" : "对话" }} · 第
          {{ index + 1 }} 次提问</span
        >
      </div>
    </n-popover>
  </nav>
</template>

<script setup lang="ts">
import { NPopover } from "naive-ui";
import { computed, nextTick, ref, watch } from "vue";
import type { ChatMessage } from "@/api/desktop";
const props = defineProps<{ messages: ChatMessage[]; visibleIds?: string[] }>();
const emit = defineEmits<{ select: [id: string] }>();
const previewId = ref<string>();
const rail = ref<HTMLElement>();
watch(
  () => props.visibleIds,
  async () => {
    await nextTick();
    const area = rail.value;
    const tick = area?.querySelector<HTMLElement>(".is-visible");
    if (!area || !tick) return;
    const offset =
      tick.getBoundingClientRect().top - area.getBoundingClientRect().top;
    if (offset < 0) area.scrollTop += offset;
    else if (offset + tick.offsetHeight > area.clientHeight)
      area.scrollTop += offset + tick.offsetHeight - area.clientHeight;
  },
);
const entries = computed(() =>
  props.messages.flatMap((message, index) =>
    message.role === "user"
      ? [
          {
            id: message.id,
            prompt: message.displayContent ?? message.content,
            reply:
              props.messages[index + 1]?.role === "assistant"
                ? props.messages[index + 1].content
                : "",
            independent: message.context?.mode === "draft",
          },
        ]
      : [],
  ),
);
</script>
