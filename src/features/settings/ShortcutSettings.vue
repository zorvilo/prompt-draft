<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { NButton, useMessage } from "naive-ui";
import type { Workspace } from "../../lib/tauri";
import {
  defaultShortcuts,
  shortcutActions,
  shortcutError,
  shortcutFromEvent,
  shortcutLabels,
  type ShortcutAction,
} from "./shortcuts";

const props = defineProps<{
  data: Workspace;
  updateShortcuts: (
    value: NonNullable<Workspace["shortcuts"]>,
  ) => Promise<boolean>;
}>();
const message = useMessage();
const recording = ref<ShortcutAction>();
const candidate = ref("");
const feedbackAction = ref<ShortcutAction>();
const error = ref("");
const saving = ref(false);
function start(action: ShortcutAction) {
  if (saving.value) return;
  feedbackAction.value = action;
  recording.value = action;
  candidate.value = "";
  error.value = "";
}
function capture(event: KeyboardEvent) {
  if (!recording.value) return;
  event.stopImmediatePropagation();
  if (event.isComposing || event.keyCode === 229) return;
  if (
    event.key === "Tab" ||
    (event.key === "Enter" &&
      !event.ctrlKey &&
      !event.altKey &&
      !event.shiftKey &&
      (event.target as HTMLElement)?.closest("button:not(.shortcut-recorder)"))
  )
    return;
  event.preventDefault();
  if (event.repeat || saving.value) return;
  if (event.key === "Escape") {
    recording.value = undefined;
    feedbackAction.value = undefined;
    return;
  }
  if (["Control", "Alt", "Shift", "Meta"].includes(event.key)) return;
  candidate.value = shortcutFromEvent(event);
  error.value = shortcutError(candidate.value);
  const other = shortcutActions.find(
    (item) =>
      item.id !== recording.value &&
      candidate.value ===
        { ...defaultShortcuts, ...props.data.shortcuts }[item.id],
  );
  if (!error.value && other)
    error.value = "与「" + other.label + "」冲突，请换一个组合。";
}
async function update(value: NonNullable<Workspace["shortcuts"]>) {
  saving.value = true;
  try {
    if (await props.updateShortcuts(value)) {
      recording.value = undefined;
      feedbackAction.value = undefined;
      message.success("快捷键已保存");
    } else {
      error.value = "保存失败，原快捷键已保留，请重新录入后重试。";
    }
  } catch {
    error.value = "保存失败，原快捷键已保留，请重新录入后重试。";
    message.error(error.value);
  } finally {
    saving.value = false;
  }
}
async function finish(action: ShortcutAction) {
  if (recording.value !== action || saving.value) return;
  recording.value = undefined;
  if (error.value) return;
  const current = { ...defaultShortcuts, ...props.data.shortcuts };
  if (!candidate.value || candidate.value === current[action]) {
    feedbackAction.value = undefined;
    return;
  }
  await update({ ...current, [action]: candidate.value });
}
function restore(action: ShortcutAction) {
  const next = {
    ...defaultShortcuts,
    ...props.data.shortcuts,
    [action]: defaultShortcuts[action],
  };
  if (new Set(Object.values(next)).size !== Object.values(next).length) {
    message.warning("默认快捷键已被另一操作使用，请先调整该操作。");
    return;
  }
  void update(next);
}
onMounted(() => window.addEventListener("keydown", capture, true));
onUnmounted(() => window.removeEventListener("keydown", capture, true));
</script>
<template>
  <section class="preference-group" aria-labelledby="shortcuts-heading">
    <div class="preference-heading">
      <h2 id="shortcuts-heading">快捷键</h2>
      <n-button
        size="tiny"
        quaternary
        :disabled="saving"
        @click="update({ ...defaultShortcuts })"
        >全部恢复默认</n-button
      >
    </div>
    <div class="preference-card">
      <div
        v-for="action in shortcutActions"
        :key="action.id"
        class="shortcut-setting"
      >
        <div class="preference-row">
          <h3>{{ action.label }}</h3>
          <div class="shortcut-actions">
            <n-button
              class="shortcut-recorder"
              :class="{
                'is-recording': recording === action.id,
                'has-error': feedbackAction === action.id && !!error,
              }"
              :aria-invalid="feedbackAction === action.id && !!error"
              :aria-label="'修改' + action.label + '快捷键'"
              :aria-pressed="recording === action.id"
              :disabled="saving"
              title="点击修改快捷键"
              @click="start(action.id)"
              @blur="finish(action.id)"
            >
              <span v-if="recording === action.id && !candidate"
                >请按下组合键</span
              >
              <span v-else class="shortcut-keys"
                ><kbd
                  v-for="key in shortcutLabels(
                    recording === action.id
                      ? candidate
                      : { ...defaultShortcuts, ...data.shortcuts }[action.id],
                  )"
                  :key="key"
                  >{{ key }}</kbd
                ></span
              >
            </n-button>
            <n-button
              size="tiny"
              quaternary
              :disabled="
                saving ||
                recording === action.id ||
                { ...defaultShortcuts, ...data.shortcuts }[action.id] ===
                  defaultShortcuts[action.id]
              "
              @click="restore(action.id)"
              >恢复默认</n-button
            >
          </div>
        </div>
        <div v-if="feedbackAction === action.id" class="shortcut-feedback">
          <p :class="{ 'shortcut-error': error }" role="status">
            {{
              error ||
              (saving ? "正在保存…" : "移出输入框自动保存，按 Esc 取消。")
            }}
          </p>
        </div>
      </div>
    </div>
  </section>
</template>
