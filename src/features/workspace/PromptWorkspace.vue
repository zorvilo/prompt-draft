<script setup lang="ts">
import { contextTokenBudget, contextWarningRatio } from "./contextBudget";
import {
  defaultShortcuts,
  shortcutFromEvent,
  shortcutLabels,
} from "../settings/shortcuts";
import { computed, nextTick, ref, watch } from "vue";
import {
  NButton,
  NButtonGroup,
  NInput,
  NSpin,
  NModal,
  NTooltip,
} from "naive-ui";
import {
  ArrowUp,
  Columns2,
  CircleHelp,
  Copy,
  Download,
  Eye,
  Maximize2,
  MessagesSquare,
  Square,
} from "lucide-vue-next";
import MarkdownIt from "markdown-it";
import type { Draft, ModificationMode } from "../../lib/tauri";
import { changeMode, currentResultReply } from "./conversation";
import { visibleMessageIds } from "./messageVisibility";
import ChatNavigation from "./ChatNavigation.vue";
import ResultComparison from "./ResultComparison.vue";
import ChatNotice from "./ChatNotice.vue";
import { chatNotices } from "./chatNotices";
import appIcon from "../../../src-tauri/icons/icon.png";

function formatContextTokens(tokens: number) {
  if (tokens < 1000) return String(tokens);
  return `${Math.floor(tokens / 10) / 100}K`;
}

const props = defineProps<{
  request?: {
    text: string;
    userMessage?: string;
    mode?: ModificationMode;
    model?: string;
    providerName?: string;
    templateId?: string;
    templateName?: string;
  };
  templateName?: string;
  contextUsage?: {
    tokens?: number;
    ratio: number;
    exceeded: boolean;
  };
  sendShortcut?: string;
  newlineBindings?: string[];
}>();
const draft = defineModel<Draft>("draft", { required: true });
const emit = defineEmits<{
  generate: [];
  copy: [content: string];
  download: [content: string];
  expand: [];
  cancel: [];
}>();
const focusedMode = ref<ModificationMode | null>(null);
const mode = computed(() => draft.value.dialogue?.mode ?? "draft");
const output = computed(() => draft.value.result);
const messages = computed(() => draft.value.messages);
const startingChat = computed(
  () => mode.value === "chat" && !draft.value.result && !messages.value.length,
);
const input = computed({
  get: () =>
    draft.value.dialogue?.chatInput ??
    (startingChat.value ? draft.value.source : ""),
  set: (value: string) => {
    draft.value.dialogue ??= {};
    draft.value.dialogue.chatInput = value;
  },
});
const streamingContent = computed(() => props.request?.text ?? "");
const before = computed({
  get: () => draft.value.source,
  set: (value: string) => {
    draft.value.source = value;
  },
});
const messageTime = (created: number) =>
  new Date(created).toLocaleString("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
  });
const expandedReply = ref<string | null>(null);
const display = ref<"preview" | "source" | "compare">("preview");
const markdown = new MarkdownIt({ html: false, breaks: true });
markdown.renderer.rules.image = (tokens, index) =>
  markdown.utils.escapeHtml(tokens[index].content);
markdown.renderer.rules.link_open = () => "<span>";
markdown.renderer.rules.link_close = () => "</span>";
const render = (text: string) => markdown.render(text);
const canSend = computed(
  () =>
    !props.request &&
    (mode.value === "draft"
      ? !!draft.value.source.trim()
      : !!input.value.trim()),
);
const sendShortcut = computed(
  () => props.sendShortcut ?? defaultShortcuts.optimize,
);
const newlineShortcuts = computed(
  () =>
    props.newlineBindings ?? [
      defaultShortcuts.newline,
      defaultShortcuts.newlineAlternate,
    ],
);
async function onSubmitKeydown(event: KeyboardEvent) {
  const binding = shortcutFromEvent(event);
  if (
    newlineShortcuts.value.includes(binding) &&
    !event.isComposing &&
    event.keyCode !== 229 &&
    !event.defaultPrevented &&
    event.target instanceof HTMLTextAreaElement
  ) {
    event.preventDefault();
    const textarea = event.target;
    const { selectionStart, selectionEnd, value, maxLength } = textarea;
    const text =
      value.slice(0, selectionStart) + "\n" + value.slice(selectionEnd);
    if (maxLength >= 0 && text.length > maxLength) return;
    if (mode.value === "chat") input.value = text;
    else before.value = text;
    await nextTick();
    textarea.setSelectionRange(selectionStart + 1, selectionStart + 1);
    return;
  }
  if (
    binding !== sendShortcut.value ||
    event.isComposing ||
    event.keyCode === 229 ||
    event.defaultPrevented
  )
    return;

  event.preventDefault();
  if (!event.repeat && canSend.value) emit("generate");
}
const chatScroll = ref<HTMLElement>();
const visibleIds = ref<string[]>([]);
function updateVisibleMessages() {
  const area = chatScroll.value;
  if (!area) {
    visibleIds.value = [];
    return;
  }
  const bounds = area.getBoundingClientRect();
  visibleIds.value = visibleMessageIds(
    {
      top: bounds.top + area.clientTop,
      bottom: bounds.top + area.clientTop + area.clientHeight,
    },
    Array.from(
      area.querySelectorAll<HTMLElement>(".from-user[data-message-id]"),
    ).map((element) => {
      const rect = element.getBoundingClientRect();
      return {
        id: element.dataset.messageId!,
        top: rect.top,
        bottom: rect.bottom,
      };
    }),
  );
}
watch(
  [chatScroll, () => draft.value.id, () => messages.value.length],
  async (_, __, onCleanup) => {
    let observer: IntersectionObserver | undefined;
    let cancelled = false;
    onCleanup(() => {
      cancelled = true;
      observer?.disconnect();
    });
    await nextTick();
    if (cancelled || !chatScroll.value) return;
    observer = new IntersectionObserver(updateVisibleMessages, {
      root: chatScroll.value,
    });
    chatScroll.value
      .querySelectorAll(".from-user[data-message-id]")
      .forEach((element) => observer!.observe(element));
    updateVisibleMessages();
  },
);
function jumpToMessage(id: string) {
  const area = chatScroll.value;
  const target = Array.from(
    area?.querySelectorAll<HTMLElement>("[data-message-id]") ?? [],
  ).find((element) => element.dataset.messageId === id);
  if (!area || !target) return;
  area.scrollTo({
    top:
      area.scrollTop +
      target.getBoundingClientRect().top -
      area.getBoundingClientRect().top -
      12,
    behavior: window.matchMedia("(prefers-reduced-motion: reduce)").matches
      ? "instant"
      : "smooth",
  });
  target.focus({ preventScroll: true });
}
const resultReply = computed(() => currentResultReply(draft.value));

async function viewResultReply() {
  const reply = resultReply.value;
  if (!reply || props.request) return;
  changeMode(draft.value, "chat");
  await nextTick();
  jumpToMessage(reply.id);
}
watch([() => draft.value.id, mode], async () => {
  visibleIds.value = [];
  await nextTick();
  const area = chatScroll.value;
  if (area && props.request) area.scrollTop = area.scrollHeight;
  updateVisibleMessages();
});
watch(
  () => draft.value.id,
  () => {
    display.value = "preview";
  },
);
watch(
  () => [
    draft.value.id,
    messages.value.length,
    props.request?.userMessage,
    streamingContent.value,
  ],
  async (values, previous) => {
    const area = chatScroll.value;
    const follow =
      !area ||
      area.scrollHeight - area.scrollTop - area.clientHeight < 64 ||
      values[0] !== previous?.[0] ||
      (!!values[2] && values[2] !== previous?.[2]);
    await nextTick();
    if (follow)
      chatScroll.value?.scrollTo({ top: chatScroll.value.scrollHeight });
    updateVisibleMessages();
  },
);
async function selectMode(value: ModificationMode) {
  changeMode(draft.value, value);
  if (value === "chat") {
    await nextTick();
    chatScroll.value?.scrollTo({ top: chatScroll.value.scrollHeight });
    updateVisibleMessages();
  }
}
</script>

<template>
  <div class="prompt-workspace" :class="{ 'is-chat': mode === 'chat' }">
    <div class="modification-toolbar">
      <div class="modification-heading">
        <slot name="heading" />
      </div>
      <n-button-group aria-label="提示词修改方式">
        <div
          v-for="item in [
            {
              value: 'draft' as const,
              label: '模板优化',
              description:
                '已经写好提示词，想让 AI 帮你改得更好？选这个。把内容填在左侧，选择模板后生成，右侧就能看到修改后的提示词。',
              icon: Columns2,
            },
            {
              value: 'chat' as const,
              label: '连续对话',
              description:
                '想边聊边改提示词？选这个。直接告诉 AI 你想改哪里，比如“再简短一点”或“加一个例子”，它会接着上面的内容修改。还没写好提示词，也可以先说说你想做什么。',
              icon: MessagesSquare,
            },
          ]"
          :key="item.value"
          class="mode-option"
        >
          <n-button
            :type="mode === item.value ? 'primary' : 'default'"
            :aria-pressed="mode === item.value"
            @click="selectMode(item.value)"
          >
            <template #icon
              ><component :is="item.icon" aria-hidden="true"
            /></template>
            {{ item.label }}
          </n-button>
          <n-tooltip
            :delay="250"
            placement="top"
            :show="focusedMode === item.value ? true : undefined"
            :style="{
              width: '360px',
              maxWidth: 'calc(100vw - 32px)',
              boxSizing: 'border-box',
            }"
          >
            <template #trigger>
              <button
                type="button"
                class="mode-help"
                :aria-label="`${item.label}功能说明`"
                @focus="focusedMode = item.value"
                @blur="focusedMode = null"
              >
                <CircleHelp :size="14" aria-hidden="true" />
              </button>
            </template>
            {{ item.description }}
          </n-tooltip>
        </div>
      </n-button-group>
    </div>

    <div v-if="mode === 'draft'" class="draft-comparison-panels">
      <section class="draft-text-panel">
        <header>
          <h2>原稿</h2>
          <span>{{ Array.from(before).length }} 字</span>
        </header>
        <n-input
          v-model:value="before"
          @keydown="onSubmitKeydown"
          type="textarea"
          :bordered="false"
          :maxlength="30000"
          :input-props="{
            'aria-label': '原稿',
          }"
          placeholder="写下你的目标、背景，以及希望 AI 交付什么。"
        />
      </section>
      <section class="draft-text-panel">
        <header>
          <h2>优化稿</h2>
        </header>
        <div class="output-view-toolbar">
          <n-button-group aria-label="修改结果显示方式"
            ><n-button
              v-for="item in [
                ['preview', '预览'],
                ['source', '原文'],
                ['compare', '对比'],
              ] as const"
              :key="item[0]"
              size="tiny"
              :secondary="display === item[0]"
              :aria-pressed="display === item[0]"
              @click="display = item[0]"
              >{{ item[1] }}</n-button
            ></n-button-group
          >
          <div class="output-file-actions">
            <n-button
              size="small"
              quaternary
              :disabled="!resultReply || !!request"
              aria-label="查看优化稿对应的回复"
              title="查看优化稿对应的回复"
              @click="viewResultReply"
              ><template #icon><Eye aria-hidden="true" /></template
            ></n-button>

            <n-button
              size="small"
              quaternary
              :disabled="!output"
              aria-label="复制优化稿"
              title="复制优化稿"
              @click="emit('copy', output)"
              ><template #icon><Copy aria-hidden="true" /></template
            ></n-button>
            <slot name="actions" />
            <n-button
              size="small"
              quaternary
              :disabled="!output"
              aria-label="放大查看优化稿"
              title="放大查看优化稿"
              @click="emit('expand')"
              ><template #icon><Maximize2 aria-hidden="true" /></template
            ></n-button>
          </div>
        </div>
        <div
          v-if="request && !request.text"
          class="optimization-waiting"
          role="status"
          aria-live="polite"
        >
          <n-spin :size="22" />
          <strong>正在优化提示词</strong>
          <span>等待模型回复，内容将在这里逐步显示</span>
        </div>
        <div
          v-else-if="request"
          class="modification-output"
          aria-label="正在生成的优化稿"
          :aria-busy="true"
        >
          <pre>{{ request.text }}</pre>
        </div>
        <result-comparison
          v-else-if="output && display === 'compare'"
          :source="before"
          :result="output"
        />
        <div
          v-else-if="output && display === 'preview'"
          class="modification-output markdown-body"
          tabindex="0"
          aria-label="修改结果预览"
          v-html="render(output)"
        ></div>
        <pre
          v-else-if="output"
          class="modification-output plain-output"
          tabindex="0"
          >{{ output }}</pre>
        <div v-else class="modification-empty">
          生成的结果会显示在这里。<br />修改原稿后可重新生成，也可以切到聊天继续追问。
        </div>
      </section>
    </div>

    <div v-else class="chat-layout">
      <chat-navigation
        :messages="messages"
        :visible-ids="visibleIds"
        @select="jumpToMessage"
      />
      <div
        ref="chatScroll"
        class="modification-chat"
        tabindex="0"
        aria-label="修改对话记录"
        @scroll.passive="updateVisibleMessages"
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
              <img :src="appIcon" alt="" aria-hidden="true" /><strong
                >词稿</strong
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
                ><template #icon
                  ><Copy :size="15" aria-hidden="true" /></template
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
                ><template #icon
                  ><Copy :size="15" aria-hidden="true" /></template
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
        <article
          v-if="request?.userMessage"
          class="conversation-entry from-user"
        >
          <p class="conversation-user-text">{{ request.userMessage }}</p>
        </article>
        <article
          v-if="request"
          class="conversation-entry from-assistant chat-streaming"
          aria-live="polite"
        >
          <div class="conversation-entry-heading">
            <img :src="appIcon" alt="" aria-hidden="true" /><strong
              >词稿</strong
            >
          </div>
          <div
            v-if="streamingContent"
            class="markdown-body"
            v-html="render(streamingContent)"
          ></div>
          <div
            v-else
            class="chat-typing"
            role="status"
            aria-label="词稿正在回复"
          >
            <i></i><i></i><i></i>
          </div>
        </article>
      </div>
    </div>
    <div
      :class="
        mode === 'chat' ? 'modification-composer' : 'draft-generation-controls'
      "
    >
      <n-input
        v-if="mode === 'chat'"
        v-model:value="input"
        type="textarea"
        :autosize="{ minRows: 2, maxRows: 5 }"
        :bordered="false"
        :maxlength="startingChat && !request ? 30000 : 4000"
        :input-props="{ 'aria-label': '对话消息' }"
        @keydown="onSubmitKeydown"
        placeholder="描述需求、讨论方向，或告诉我哪里需要修改…"
      />
      <div class="modification-composer-tools">
        <slot name="template" /><slot name="models" />
        <n-tooltip
          v-if="
            mode === 'chat' && contextUsage && (contextUsage.tokens ?? 0) > 0
          "
          trigger="hover"
          :delay="250"
          :style="{ maxWidth: 'calc(100vw - 32px)' }"
        >
          <template #trigger>
            <button
              type="button"
              class="context-usage"
              :class="{
                'is-warning': contextUsage.ratio >= contextWarningRatio,
                'is-exceeded': contextUsage.exceeded,
              }"
              :aria-label="`应用上下文用量 ${Math.round(contextUsage.ratio * 100)}%，${contextUsage.tokens ?? '未知'} Token`"
            >
              <svg
                width="18"
                height="18"
                viewBox="0 0 20 20"
                aria-hidden="true"
              >
                <circle class="context-usage-track" cx="10" cy="10" r="7" />
                <circle
                  cx="10"
                  cy="10"
                  r="7"
                  pathLength="100"
                  :stroke-dasharray="`${Math.min(100, contextUsage.ratio * 100)} 100`"
                  transform="rotate(-90 10 10)"
                />
              </svg>
            </button>
          </template>
          <div class="context-usage-detail context-usage-compact">
            <span>上下文参考预算：</span>
            <strong
              >{{ Math.round(contextUsage.ratio * 100) }}% 已用（剩余
              {{
                Math.max(0, 100 - Math.round(contextUsage.ratio * 100))
              }}%）</strong
            >
            <span
              >已用
              {{ formatContextTokens(contextUsage.tokens ?? 0) }} Token，共
              {{ formatContextTokens(contextTokenBudget) }}</span
            >
          </div>
        </n-tooltip>
        <n-button
          v-if="request"
          :circle="mode === 'chat'"
          :class="{
            'chat-send-button': mode === 'chat',
            'optimization-action': mode === 'draft',
          }"
          type="primary"
          aria-label="停止生成"
          @click="emit('cancel')"
          ><template #icon><Square :size="18" aria-hidden="true" /></template
          ><span v-if="mode === 'draft'">停止生成</span></n-button
        ><n-button
          v-else
          :circle="mode === 'chat'"
          :class="{
            'chat-send-button': mode === 'chat',
            'optimization-action': mode === 'draft',
          }"
          type="primary"
          :disabled="!canSend"
          :aria-label="
            mode === 'chat'
              ? '发送消息'
              : draft.result
                ? '确认优化'
                : '生成初稿'
          "
          :title="
            mode === 'chat'
              ? shortcutLabels(sendShortcut).join(' + ') +
                ' 发送，' +
                newlineShortcuts.join(' / ') +
                ' 换行'
              : '确认优化'
          "
          @click="emit('generate')"
          ><template #icon><ArrowUp :size="18" aria-hidden="true" /></template
          ><span v-if="mode === 'draft'">确认优化</span></n-button
        >
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
  </div>
</template>
