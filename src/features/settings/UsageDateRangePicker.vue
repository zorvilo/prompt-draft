<script setup lang="ts">
import { computed, ref } from "vue";
import { CalendarDays, ChevronDown } from "lucide-vue-next";
import { NButton, NDatePicker, NPopover } from "naive-ui";
const days = defineModel<number>("days", { required: true });
const customRange = defineModel<[number, number] | null>("customRange", {
  required: true,
});
const props = defineProps<{ range: [number, number] }>();
const open = ref(false);
const draft = ref<[number, number] | null>(null);
const presets = [
  { label: "今天", value: 1 },
  { label: "近 24 小时", value: -1 },
  { label: "近 7 天", value: 7 },
  { label: "近 14 天", value: 14 },
  { label: "近 30 天", value: 30 },
  { label: "全部", value: 0 },
];
const format = (value: number) =>
  new Date(value).toLocaleString("zh-CN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  });
const label = computed(() =>
  customRange.value
    ? "自定义时间"
    : presets.find((item) => item.value === days.value)?.label,
);
function setOpen(value: boolean) {
  if (value)
    draft.value = [
      props.range[0] || props.range[1] - 30 * 86400000,
      props.range[1],
    ];
  open.value = value;
}
function select(value: number) {
  days.value = value;
  customRange.value = null;
  open.value = false;
}
function apply(value: number | [number, number] | null) {
  if (
    !Array.isArray(value) ||
    !value.every(Number.isFinite) ||
    value[0] > value[1]
  )
    return;
  customRange.value = value;
  open.value = false;
}
</script>

<template>
  <n-popover
    :show="open"
    trigger="click"
    placement="bottom-start"
    :show-arrow="false"
    :style="{
      width: 'min(620px, calc(100vw - 32px))',
      maxHeight: 'calc(100dvh - 160px)',
      overflow: 'auto',
      padding: '12px',
      borderRadius: '8px',
    }"
    @update:show="setOpen"
  >
    <template #trigger>
      <n-button
        class="usage-range-trigger"
        :aria-expanded="open"
        aria-label="选择统计时间范围"
        :title="
          customRange
            ? format(customRange[0]) + ' — ' + format(customRange[1])
            : label
        "
      >
        <CalendarDays :size="16" aria-hidden="true" /><span>{{ label }}</span
        ><ChevronDown :size="14" aria-hidden="true" />
      </n-button>
    </template>
    <div class="usage-range-presets" role="group" aria-label="快捷时间范围">
      <n-button
        v-for="preset in presets"
        :key="preset.value"
        size="small"
        :class="{ 'range-selected': !customRange && days === preset.value }"
        :aria-pressed="!customRange && days === preset.value"
        @click="select(preset.value)"
        >{{ preset.label }}</n-button
      >
    </div>
    <div class="usage-range-calendar">
      <n-date-picker
        v-model:value="draft"
        type="datetimerange"
        panel
        :actions="['confirm']"
        :clearable="false"
        :show-week-numbers="false"
        @confirm="apply"
      />
    </div>
  </n-popover>
</template>

<style scoped>
.usage-range-trigger :deep(.n-button__content) {
  gap: 6px;
}
.usage-range-trigger :deep(svg) {
  color: var(--muted);
  flex-shrink: 0;
}
.usage-range-presets {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  padding-bottom: 8px;
  border-bottom: 1px solid var(--line);
}
.range-selected {
  font-weight: 600;
  color: var(--ink);
  background: var(--selection);
  --n-border: 1px solid var(--ink) !important;
}
.usage-range-calendar {
  padding-top: 10px;
  overflow: auto;
}
.usage-range-calendar :deep(.n-date-panel) {
  box-shadow: none;
}
</style>
