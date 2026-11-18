<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useThemeVars } from "naive-ui";
import { use } from "echarts/core";
import { BarChart } from "echarts/charts";
import {
  GridComponent,
  TooltipComponent,
  AriaComponent,
} from "echarts/components";
import { SVGRenderer } from "echarts/renderers";
import VChart from "vue-echarts";
import type { EChartsOption } from "echarts";

use([BarChart, GridComponent, TooltipComponent, AriaComponent, SVGRenderer]);
const props = defineProps<{
  points: { date: string; label: string; value: number | null }[];
  label: string;
  metric: "cost" | "count" | "tokens" | "hitRate";
  formatValue: (value: number | null) => string;
}>();
const theme = useThemeVars();
const chart = ref<InstanceType<typeof VChart>>();
const selected = ref(-1);
watch(
  () => [props.points, props.metric],
  () => {
    selected.value = -1;
  },
);
function navigate(event: KeyboardEvent) {
  if (event.key === "Escape") {
    chart.value?.dispatchAction({ type: "hideTip" });
    return;
  }
  if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
  event.preventDefault();
  selected.value = Math.max(
    0,
    Math.min(
      props.points.length - 1,
      selected.value + (event.key === "ArrowRight" ? 1 : -1),
    ),
  );
  chart.value?.dispatchAction({
    type: "showTip",
    seriesIndex: 0,
    dataIndex: selected.value,
  });
}
const option = computed<EChartsOption>(() => ({
  animation: false,
  aria: {
    enabled: true,
    description: `${props.label}趋势图。使用左右方向键查看各时段，Escape 关闭提示。`,
  },
  grid: { left: 10, right: 10, top: 65, bottom: 8, containLabel: true },
  tooltip: {
    trigger: "axis",
    renderMode: "richText",
    confine: true,
    // Reserve a separate band above the plot so tooltips never cover bars.
    position: (_point, _params, _dom, _rect, size) => [
      Math.max(0, size.viewSize[0] - size.contentSize[0] - 8),
      0,
    ],
    backgroundColor: theme.value.popoverColor,
    borderColor: theme.value.borderColor,
    borderWidth: 1,
    padding: [8, 12],
    textStyle: { color: theme.value.textColor2, fontSize: 12 },
    axisPointer: {
      type: "shadow",
      // ECharts defaults the pointer above series; keep the shadow behind bars.
      z: 0,
      shadowStyle: { color: theme.value.hoverColor },
    },
    formatter: (params) => {
      const item = Array.isArray(params) ? params[0] : params;
      const point = item ? props.points[item.dataIndex] : undefined;
      return point
        ? `${point.date.replace("T", " ")}\n${props.label}  ${props.formatValue(point.value)}`
        : "";
    },
  },
  xAxis: {
    type: "category",
    data: props.points.map((point) => point.label),
    axisLine: { show: false },
    axisTick: { show: false },
    axisLabel: {
      color: theme.value.textColor3,
      fontSize: 11,
      hideOverlap: true,
    },
  },
  yAxis: {
    type: "value",
    min: 0,
    max: props.metric === "hitRate" ? 1 : undefined,
    minInterval:
      props.metric === "count" || props.metric === "tokens" ? 1 : undefined,
    splitNumber: 2,
    axisLabel: {
      color: theme.value.textColor3,
      fontSize: 11,
      formatter: (value) => props.formatValue(value),
    },
    splitLine: { lineStyle: { color: theme.value.borderColor } },
  },
  series: [
    {
      type: "bar",
      z: 2,
      name: props.label,
      data: props.points.map((point) => point.value),
      barMaxWidth: 36,
      itemStyle: { color: "#7895b2", borderRadius: [3, 3, 0, 0] },
      emphasis: { itemStyle: { color: "#567b9f" } },
    },
  ],
}));
</script>

<template>
  <v-chart
    ref="chart"
    class="usage-chart"
    :option="option"
    :init-options="{ renderer: 'svg' }"
    autoresize
    tabindex="0"
    :aria-label="label + '趋势图，左右方向键查看数据'"
    @keydown="navigate"
    @blur="chart?.dispatchAction({ type: 'hideTip' })"
  />
</template>

<style scoped>
.usage-chart {
  height: 200px;
  width: 100%;
}
.usage-chart:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
  border-radius: 4px;
}
</style>
