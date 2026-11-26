<template>
  <div class="chat-layout">
    <chat-navigation
      :messages="messages"
      :visible-ids="visibleIds"
      @select="emit('select', $event)"
    />
    <div
      :ref="setScrollElement"
      class="modification-chat"
      tabindex="0"
      aria-label="修改对话记录"
      @scroll.passive="emit('scroll')"
    >
      <div
        v-if="!messages.length && !request?.userMessage"
        class="chat-introduction"
      >
        <img :src="appIcon" alt="词稿" />
        <h2>
          {{ draft.result ? "从当前稿继续聊" : "你想写一份什么提示词？" }}
        </h2>
        <p>
          {{
            draft.result
              ? "说说哪里不满意，也可以先讨论修改方向。"
              : "写下需求，或先聊聊你想达到的效果。"
          }}
        </p>
      </div>
      <template v-for="(entry, index) in messages" :key="entry.id">
        <chat-notice
          v-for="notice in entry.role === 'user'
            ? chatNotices(
                {
                  ...messages[index - 1],
                  context: messages[index - 2]?.context,
                },
                {
                  mode: entry.context?.mode,
                  templateId: entry.context?.templateId,
                  templateName: entry.context?.templateName,
                  model: entry.model,
                  providerName: entry.providerName,
                },
              )
            : []"
          :key="notice.kind"
          :notice="notice"
        />
        <article
          class="conversation-entry"
          :data-message-id="entry.id"
          tabindex="-1"
          :class="entry.role === 'user' ? 'from-user' : 'from-assistant'"
        >
          <div
            v-if="entry.role === 'assistant'"
            class="conversation-entry-heading"
          >
            <img :src="appIcon" alt="" aria-hidden="true" /><strong>词稿</strong
            ><span
              v-if="entry.durationMs !== undefined"
              class="conversation-duration"
              >用时 {{ (entry.durationMs / 1000).toFixed(1) }} 秒</span
            >
          </div>
          <p v-if="entry.role === 'user'" class="conversation-user-text">
            {{ entry.displayContent ?? entry.content }}
          </p>
          <div
            v-else
            class="markdown-body"
            v-html="render(entry.content)"
          ></div>
          <div v-if="entry.role === 'user'" class="conversation-user-actions">
            <time
              :datetime="new Date(entry.created).toISOString()"
              :title="new Date(entry.created).toLocaleString('zh-CN')"
              >{{ messageTime(entry.created) }}</time
            >
            <n-button
              size="tiny"
              quaternary
              aria-label="复制消息"
              title="复制消息"
              @click="emit('copy', entry.displayContent ?? entry.content)"
              ><template #icon><Copy :size="15" aria-hidden="true" /></template
            ></n-button>
          </div>
          <div
            v-if="entry.role === 'assistant'"
            class="conversation-entry-actions"
          >
            <n-button
              size="tiny"
              quaternary
              aria-label="复制回复"
              title="复制回复"
              @click="emit('copy', entry.content)"
              ><template #icon><Copy :size="15" aria-hidden="true" /></template
            ></n-button>
            <n-button
              size="tiny"
              quaternary
              aria-label="下载回复"
              title="下载回复"
              @click="emit('download', entry.content)"
              ><template #icon
                ><Download :size="15" aria-hidden="true" /></template
            ></n-button>
            <n-button
              size="tiny"
              quaternary
              aria-label="放大查看回复"
              title="放大查看回复"
              @click="expandedReply = entry.content"
              ><template #icon
                ><Maximize2 :size="15" aria-hidden="true" /></template
            ></n-button>
            <time
              class="conversation-message-time"
              :datetime="new Date(entry.created).toISOString()"
              :title="new Date(entry.created).toLocaleString('zh-CN')"
              >{{ messageTime(entry.created) }}</time
            >
            <span class="conversation-word-count"
              >{{ Array.from(entry.content).length }} 字</span
            >
          </div>
        </article>
      </template>
      <template v-if="request?.userMessage">
        <chat-notice
          v-for="notice in chatNotices(
            {
              ...messages.at(-1),
              context: messages
                .slice()
                .reverse()
                .find((message) => message.context)?.context,
            },
            request,
          )"
          :key="notice.kind"
          :notice="notice"
        />
      </template>
      <article v-if="request?.userMessage" class="conversation-entry from-user">
        <p class="conversation-user-text">{{ request.userMessage }}</p>
      </article>
      <article
        v-if="request"
        class="conversation-entry from-assistant chat-streaming"
        aria-live="polite"
      >
        <div class="conversation-entry-heading">
          <img :src="appIcon" alt="" aria-hidden="true" /><strong>词稿</strong>
        </div>
        <div
          v-if="streamingContent"
          class="markdown-body"
          v-html="render(streamingContent)"
        ></div>
        <div v-else class="chat-typing" role="status" aria-label="词稿正在回复">
          <i></i><i></i><i></i>
        </div>
      </article>
    </div>
  </div>
  <n-modal
    :show="expandedReply !== null"
    preset="card"
    title="查看回复"
    to=".app-root"
    class="chat-reply-dialog"
    :style="{ width: 'min(900px, calc(100vw - 32px))' }"
    @update:show="
      (show) => {
        if (!show) expandedReply = null;
      }
    "
  >
    <div
      class="markdown-body chat-reply-expanded"
      tabindex="0"
      aria-label="回复全文"
      v-html="render(expandedReply ?? '')"
    ></div>
  </n-modal>
</template>

<script setup lang="ts">
import type { Draft } from "@/api/desktop";
import { renderMarkdown } from "@/utils/markdown";
import { chatNotices } from "@/utils/workspace/chatNotices";
import { Copy, Download, Maximize2 } from "lucide-vue-next";
import { NButton, NModal } from "naive-ui";
import { computed, ref, type ComponentPublicInstance } from "vue";
import appIcon from "../../../../src-tauri/icons/icon.png";
import type { GenerationRequest } from "@/views/workspace/types";
import ChatNavigation from "./ChatNavigation.vue";
import ChatNotice from "./ChatNotice.vue";
const props = defineProps<{
  draft: Draft;
  request?: GenerationRequest;
  visibleIds: string[];
}>();
const emit = defineEmits<{
  copy: [content: string];
  download: [content: string];
  select: [id: string];
  scroll: [];
  scrollElement: [element: HTMLElement | undefined];
}>();
const messages = computed(() => props.draft.messages);
const streamingContent = computed(() => props.request?.text ?? "");
const expandedReply = ref<string | null>(null);
const render = renderMarkdown;
function setScrollElement(element: Element | ComponentPublicInstance | null) {
  emit("scrollElement", element instanceof HTMLElement ? element : undefined);
}
const messageTime = (created: number) =>
  new Date(created).toLocaleString("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
  });
</script>
