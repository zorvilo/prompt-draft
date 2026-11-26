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

    <ChatConversation
      v-else
      :draft="draft"
      :request="request"
      :visible-ids="visibleIds"
      @copy="emit('copy', $event)"
      @download="emit('download', $event)"
      @select="jumpToMessage"
      @scroll="updateVisibleMessages"
      @scroll-element="(element) => (chatScroll = element)"
    />
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
  </div>
</template>

<script setup lang="ts">
import { useChatScroll } from "@/views/workspace/hooks/useChatScroll";
import type { GenerationRequest } from "@/views/workspace/types";
import ChatConversation from "./ChatConversation.vue";

import {
  ArrowUp,
  CircleHelp,
  Columns2,
  Copy,
  Eye,
  Maximize2,
  MessagesSquare,
  Square,
} from "lucide-vue-next";
import { renderMarkdown } from "@/utils/markdown";
import { NButton, NButtonGroup, NInput, NSpin, NTooltip } from "naive-ui";
import { computed, nextTick, ref, watch } from "vue";
import type { Draft, ModificationMode } from "@/api/desktop";
import {
  defaultShortcuts,
  shortcutFromEvent,
  shortcutLabels,
} from "@/config/shortcuts";
import {
  contextTokenBudget,
  contextWarningRatio,
} from "@/utils/workspace/contextBudget";
import { changeMode, currentResultReply } from "@/utils/workspace/conversation";
import ResultComparison from "./ResultComparison.vue";

function formatContextTokens(tokens: number) {
  if (tokens < 1000) return String(tokens);
  return `${Math.floor(tokens / 10) / 100}K`;
}

const props = defineProps<{
  request?: GenerationRequest;
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
const before = computed({
  get: () => draft.value.source,
  set: (value: string) => {
    draft.value.source = value;
  },
});
const display = ref<"preview" | "source" | "compare">("preview");
const render = renderMarkdown;
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
const { chatScroll, visibleIds, updateVisibleMessages, jumpToMessage } =
  useChatScroll(draft, mode, props);
const resultReply = computed(() => currentResultReply(draft.value));

async function viewResultReply() {
  const reply = resultReply.value;
  if (!reply || props.request) return;
  changeMode(draft.value, "chat");
  await nextTick();
  jumpToMessage(reply.id);
}
watch(
  () => draft.value.id,
  () => {
    display.value = "preview";
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
