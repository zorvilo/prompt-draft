<template>
  <pricing-panel
    v-if="showCatalog"
    @back="
      showCatalog = false;
      revision++;
    "
  />
  <div v-else class="usage-panel">
    <div class="usage-controls">
      <usage-date-range-picker
        v-model:days="days"
        v-model:custom-range="customRange"
        :range="timeRange"
      />
      <form class="usage-search" @submit.prevent="applySearch">
        <n-input
          v-model:value="search"
          clearable
          placeholder="搜索供应商或模型"
          aria-label="搜索供应商或模型"
          @clear="
            filter = '';
            page = 0;
          "
        />
        <n-button attr-type="submit">筛选</n-button>
      </form>
      <n-button :loading="loading" @click="refreshUsage">刷新</n-button>
    </div>
    <p v-if="error" role="alert">{{ error }}</p>
    <n-spin v-else-if="loading" />
    <template v-else-if="data">
      <section class="usage-overview" aria-label="用量概览">
        <div class="usage-hero">
          <div class="usage-metrics">
            <button
              v-for="(label, key) in metricLabels"
              :key="key"
              type="button"
              :class="{ selected: metric === key }"
              :aria-pressed="metric === key"
              @click="metric = key"
            >
              <span class="metric-label"
                >{{ label
                }}<n-tooltip
                  v-if="key === 'tokens' || key === 'hitRate'"
                  trigger="hover"
                  :style="{ maxWidth: '300px' }"
                  ><template #trigger
                    ><span
                      class="metric-help"
                      tabindex="0"
                      :aria-label="
                        key === 'tokens'
                          ? '真实消耗 Token 说明'
                          : '缓存命中率说明'
                      "
                      @click.stop
                      ><CircleHelp
                        :size="12"
                        aria-hidden="true" /></span></template
                  >{{
                    key === "tokens"
                      ? "真实消耗 = 新增输入 + 输出 + 缓存写入 + 缓存命中。输入 Token 已包含缓存部分，统计时不会重复相加。仅统计接口实际返回的用量，未提供的部分不做估算。"
                      : "缓存命中率 = 缓存命中 Token ÷ 总输入 Token（新增输入 + 缓存写入 + 缓存命中）。没有输入用量时显示 —。"
                  }}</n-tooltip
                ></span
              >
              <strong
                :title="formatMetric(metricValue(data.summary, key), key)"
                >{{
                  data.summary.count
                    ? formatMetric(metricValue(data.summary, key), key)
                    : "—"
                }}</strong
              >
            </button>
          </div>
          <div v-if="expanded" id="usage-token-details" class="usage-details">
            <div>
              <span>新增输入</span
              ><strong>{{ number(data.summary.freshInput) }}</strong>
            </div>
            <div>
              <span>输出</span
              ><strong>{{ number(data.summary.output) }}</strong>
            </div>
            <div>
              <span class="metric-label"
                >缓存写入<n-tooltip :style="{ maxWidth: '300px' }"
                  ><template #trigger
                    ><span
                      class="metric-help"
                      tabindex="0"
                      aria-label="缓存写入说明"
                      ><CircleHelp
                        :size="12"
                        aria-hidden="true" /></span></template
                  >缓存写入 Token
                  来自接口返回的用量。部分供应商不提供此项，未提供时显示“未提供”，不按零估算。</n-tooltip
                ></span
              ><strong>{{ number(data.summary.cacheWrite) }}</strong>
            </div>
            <div>
              <span>缓存命中</span
              ><strong>{{ number(data.summary.cacheRead) }}</strong>
            </div>
          </div>
          <n-button
            circle
            size="tiny"
            class="usage-expand"
            :aria-expanded="expanded"
            aria-controls="usage-token-details"
            :aria-label="expanded ? '收起 Token 明细' : '展开 Token 明细'"
            @click="expanded = !expanded"
            ><ChevronDown :size="14" :class="{ rotated: expanded }"
          /></n-button>
        </div>
        <div class="usage-trend">
          <h2>
            {{ metricLabels[metric] }} ·
            {{ hourly ? "每小时趋势" : "每日趋势" }}
          </h2>
          <usage-trend-chart
            :points="chart"
            :label="metricLabels[metric]"
            :metric="metric"
            :format-value="formatMetric"
          />
          <p v-if="!data.summary.count" class="chart-empty">
            此范围暂无请求记录
          </p>
        </div>
      </section>
      <div class="usage-controls" role="group" aria-label="统计视图">
        <n-button
          v-for="tab in tabs"
          :key="tab.value"
          :type="group === tab.value ? 'primary' : 'default'"
          :aria-pressed="group === tab.value"
          @click="group = tab.value"
          >{{ tab.label }}</n-button
        >
      </div>
      <p v-if="!data.total" class="muted">此范围暂无请求记录。</p>
      <div v-else class="usage-table-wrap">
        <table
          class="usage-table"
          :aria-label="tabs.find((tab) => tab.value === group)?.label"
        >
          <thead v-if="group === 'logs'">
            <tr>
              <th>时间</th>
              <th>供应商</th>
              <th>模型</th>
              <th>操作</th>
              <th>结果</th>
              <th>耗时</th>
              <th>新增输入</th>
              <th>输出</th>
              <th>缓存命中</th>
              <th>预估成本</th>
            </tr>
          </thead>
          <thead v-else>
            <tr>
              <th>{{ group === "provider" ? "供应商" : "模型" }}</th>
              <th>请求次数</th>
              <th>成功率</th>
              <th>新增输入</th>
              <th>输出</th>
              <th>缓存命中</th>
              <th>预估成本</th>
            </tr>
          </thead>
          <tbody v-if="group === 'logs'">
            <tr v-for="row in logs" :key="row.id">
              <td>
                {{ new Date(row.created * 1000).toLocaleString("zh-CN") }}
              </td>
              <td>{{ row.provider }}</td>
              <td>
                {{ row.model }}
                <div v-if="row.pricingModel !== row.model" class="muted">
                  计价：{{ row.pricingModel }}
                </div>
              </td>
              <td>{{ kinds[row.kind] }}</td>
              <td>{{ outcomes[row.outcome] }}</td>
              <td>{{ (row.elapsed / 1000).toFixed(2) }} 秒</td>
              <td class="input-token-cell">
                <span class="input-token-value"
                  ><span>{{ number(row.freshInput) }}</span>
                  <n-tooltip trigger="hover" :style="{ maxWidth: '280px' }">
                    <template #trigger
                      ><n-button
                        class="input-token-help"
                        quaternary
                        circle
                        size="tiny"
                        aria-label="查看输入用量说明"
                        ><CircleHelp :size="12" aria-hidden="true" /></n-button
                    ></template>
                    <div>
                      输入总量：{{
                        row.input === null
                          ? "未提供"
                          : number(row.input) + " Token"
                      }}
                    </div>
                    <div v-if="row.cacheWrite !== null && row.cacheWrite > 0">
                      缓存写入：{{ number(row.cacheWrite) }} Token
                    </div>
                    <div>
                      新增输入只扣除接口已报告的缓存命中和缓存写入，不重复计数。
                    </div>
                  </n-tooltip></span
                >
              </td>
              <td>{{ number(row.output) }}</td>
              <td>{{ number(row.cacheRead) }}</td>
              <td>
                {{ cost(row.cost) }}
              </td>
            </tr>
          </tbody>
          <tbody v-else>
            <tr v-for="row in groups" :key="row.id">
              <td>{{ row.label }}</td>
              <td>{{ number(row.count) }}</td>
              <td>{{ rate(row) }}</td>
              <td class="input-token-cell">
                <span class="input-token-value"
                  ><span>{{ number(row.freshInput) }}</span>
                  <n-tooltip trigger="hover" :style="{ maxWidth: '280px' }">
                    <template #trigger
                      ><n-button
                        class="input-token-help"
                        quaternary
                        circle
                        size="tiny"
                        aria-label="查看输入用量说明"
                        ><CircleHelp :size="12" aria-hidden="true" /></n-button
                    ></template>
                    <div>
                      输入总量：{{
                        row.input === null
                          ? "未提供"
                          : number(row.input) + " Token"
                      }}
                    </div>
                    <div v-if="row.cacheWrite !== null && row.cacheWrite > 0">
                      缓存写入：{{ number(row.cacheWrite) }} Token
                    </div>
                    <div>
                      新增输入只扣除接口已报告的缓存命中和缓存写入，不重复计数。
                    </div>
                  </n-tooltip></span
                >
              </td>
              <td>{{ number(row.output) }}</td>
              <td>{{ number(row.cacheRead) }}</td>
              <td>
                {{ cost(row.cost) }}（{{ row.priced }}/{{ row.count }} 次）
              </td>
            </tr>
          </tbody>
        </table>
      </div>
      <div v-if="data.total > 20" class="usage-controls">
        <n-button :disabled="page === 0" @click="page--">上一页</n-button>
        <span>第 {{ page + 1 }} / {{ Math.ceil(data.total / 20) }} 页</span>
        <n-button :disabled="(page + 1) * 20 >= data.total" @click="page++"
          >下一页</n-button
        >
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
defineOptions({ name: "UsagePage" });
import { ChevronDown, CircleHelp } from "lucide-vue-next";
import { NButton, NInput, NSpin, NTooltip } from "naive-ui";
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import {
  desktop,
  errorMessage,
  type UsageCounts,
  type UsageStats,
} from "@/api/desktop";
import PricingPanel from "./components/PricingPanel.vue";
import UsageDateRangePicker from "./components/UsageDateRangePicker.vue";
import UsageTrendChart from "./components/UsageTrendChart.vue";

import { useWorkspaceContext } from "@/layout/hooks/workspaceContext";
const { usageCatalog: showCatalog } = useWorkspaceContext();
const currency = new Intl.NumberFormat("zh-CN", {
  style: "currency",
  currency: "CNY",
  minimumFractionDigits: 4,
  maximumFractionDigits: 4,
});
const cost = (value: number | null) =>
  value === null ? "—" : currency.format(value);

const days = ref(1);
const customRange = ref<[number, number] | null>(null);
const clock = ref(Date.now());
const timeRange = computed<[number, number]>(() => {
  if (customRange.value) return customRange.value;
  const end = clock.value;
  if (days.value === 0) return [0, end];
  if (days.value === -1) return [end - 86400000, end];
  const start = new Date(end);
  start.setHours(0, 0, 0, 0);
  start.setDate(start.getDate() - days.value + 1);
  return [start.getTime(), end];
});
const group = ref("logs");
const page = ref(0);
const search = ref("");
const filter = ref("");
const revision = ref(0);
const data = ref<UsageStats>();
const logs = computed(() =>
  (data.value?.rows ?? []).filter((row) => "created" in row),
);
const groups = computed(() =>
  (data.value?.rows ?? []).filter((row) => "label" in row),
);
const loading = ref(false);
const error = ref("");
const tabs = [
  { label: "请求明细", value: "logs" },
  { label: "供应商统计", value: "provider" },
  { label: "模型统计", value: "model" },
];
const kinds: Record<string, string> = {
  test: "连接测试",
  optimize: "优化",
  followup: "追问",
};
const outcomes: Record<string, string> = {
  success: "成功",
  failed: "失败",
  cancelled: "已取消",
};
const number = (value: number | null) =>
  value?.toLocaleString("zh-CN") ?? "未提供";
const rate = (row: UsageCounts) =>
  row.count ? Math.round((row.success / row.count) * 100) + "%" : "—";
const metric = ref<"cost" | "count" | "tokens" | "hitRate">("tokens");
const expanded = ref(false);
const metricLabels = {
  cost: "总成本",
  count: "总请求数",
  tokens: "真实消耗 Token",
  hitRate: "缓存命中率",
};
function metricValue(row: UsageCounts, key = metric.value): number | null {
  if (key === "tokens")
    return row.input === null && row.output === null
      ? null
      : (row.input ?? 0) + (row.output ?? 0);
  if (key === "hitRate")
    return row.input && row.cacheRead !== null
      ? row.cacheRead / row.input
      : null;
  return row[key];
}
function formatMetric(value: number | null, key = metric.value) {
  if (value === null) return "—";
  if (key === "cost") return cost(value);
  if (key === "hitRate") return (value * 100).toFixed(1) + "%";
  return number(value);
}
const hourly = computed(
  () => timeRange.value[1] - timeRange.value[0] <= 86400000,
);
const chart = computed(() => {
  const rows = data.value?.trend ?? [];
  const first = new Date(timeRange.value[0] || (rows[0]?.date ?? clock.value));
  const last = new Date(timeRange.value[1]);
  if (hourly.value) {
    first.setMinutes(0, 0, 0);
    last.setMinutes(0, 0, 0);
  } else {
    first.setHours(0, 0, 0, 0);
    last.setHours(0, 0, 0, 0);
  }
  const points = [];
  for (
    const date = new Date(first);
    date <= last;
    hourly.value
      ? date.setHours(date.getHours() + 1)
      : date.setDate(date.getDate() + 1)
  ) {
    const day = `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
    const hour = `${String(date.getHours()).padStart(2, "0")}:00`;
    const key = hourly.value ? `${day}T${hour}` : day;
    const row = rows.find((row) => row.date === key);
    points.push({
      date: key,
      label: hourly.value ? hour : day.slice(5).replace("-", "/"),
      value: row ? metricValue(row) : metric.value === "hitRate" ? null : 0,
    });
  }
  return points;
});
let refreshTimer: ReturnType<typeof setInterval>;
function refreshUsage() {
  clock.value = Date.now();
  revision.value++;
}
onMounted(() => {
  refreshTimer = setInterval(refreshUsage, 60000);
  window.addEventListener("usage-updated", refreshUsage);
});
onUnmounted(() => {
  clearInterval(refreshTimer);
  window.removeEventListener("usage-updated", refreshUsage);
});
function applySearch() {
  page.value = 0;
  filter.value = search.value.trim();
}

watch([days, customRange, group], () => {
  clock.value = Date.now();
  page.value = 0;
});
watch(
  [days, customRange, group, page, filter, revision],
  async (_, __, onCleanup) => {
    let current = true;
    onCleanup(() => {
      current = false;
    });
    loading.value = true;
    error.value = "";
    data.value = undefined;
    try {
      const result = await desktop.usageStats({
        since: Math.floor(timeRange.value[0] / 1000),
        until: Math.floor(timeRange.value[1] / 1000),
        hourly: hourly.value,
        page: page.value,
        group: group.value,
        filter: filter.value || null,
      });
      if (current) data.value = result;
    } catch (cause) {
      if (current) error.value = errorMessage(cause);
    } finally {
      if (current) loading.value = false;
    }
  },
  { immediate: true },
);
</script>

<style scoped>
.input-token-value {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  white-space: nowrap;
}
.input-token-help {
  flex: none;
  color: var(--muted);
  width: 18px;
  height: 18px;
}
.usage-panel {
  display: grid;
  grid-template-columns: minmax(0, 1fr);
  gap: 16px;
}
.usage-controls,
.usage-search {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}
.usage-search {
  flex: 1;
}
.usage-search > .n-input {
  flex: 1;
  min-width: 145px;
}
.usage-overview,
.usage-table-wrap {
  border: 1px solid var(--line);
  border-radius: 8px;
  overflow: hidden;
  background: var(--paper);
}
.usage-hero {
  position: relative;
  border-bottom: 1px solid var(--line);
}
.usage-metrics,
.usage-details {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
}
.usage-metrics button {
  appearance: none;
  background: transparent;
  border: 0;
  color: var(--muted);
  padding: 10px 16px 14px;
  text-align: left;
  cursor: pointer;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.metric-label {
  display: flex;
  align-items: center;
  gap: 4px;
}
.metric-help {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  border-radius: 50%;
  color: var(--muted);
  cursor: pointer;
}
.metric-help:hover {
  background: var(--selection);
}
.metric-help:focus-visible {
  outline: 2px solid var(--accent);
  border-radius: 50%;
}
.usage-metrics button:hover {
  background: var(--rail);
}
.usage-metrics button.selected {
  color: var(--ink);
  box-shadow: inset 0 -2px var(--ink);
}
.usage-metrics button:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: -2px;
}
.usage-metrics span,
.usage-details span {
  font-size: 12px;
  line-height: 18px;
}
.usage-metrics strong {
  font-size: 24px;
  line-height: 28px;
  font-weight: 600;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 100%;
}
.usage-metrics strong,
.usage-details strong {
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
}
.usage-details {
  border-top: 1px solid var(--line);
}
.usage-details > div {
  display: grid;
  gap: 2px;
  padding: 10px 16px;
  color: var(--muted);
}
.usage-details strong {
  font-size: 14px;
  font-weight: 500;
}
.usage-expand {
  position: absolute;
  bottom: -12px;
  left: calc(50% - 12px);
  z-index: 1;
  background: var(--paper);
}
.rotated {
  transform: rotate(180deg);
}
.usage-panel p {
  margin: 0;
}
.usage-trend {
  position: relative;
  padding: 20px 16px 10px;
}
.usage-trend h2 {
  margin: 0 0 4px;
  font-size: 12px;
  font-weight: 600;
  color: var(--muted);
  line-height: 28px;
}
.chart-empty {
  position: absolute;
  inset: 145px 0 auto;
  text-align: center;
  color: var(--muted);
  font-size: 12px;
  pointer-events: none;
}
@media (max-width: 700px) {
  .usage-metrics,
  .usage-details {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
  .usage-metrics strong {
    font-size: 20px;
  }
}
.usage-table-wrap {
  overflow: auto;
}
.usage-table {
  border-collapse: collapse;
  width: 100%;
  text-align: left;
  font-size: 13px;
}
.usage-table :is(th, td) {
  padding: 10px;
  border-bottom: 1px solid var(--line);
  overflow-wrap: anywhere;
}
.usage-table th {
  background: var(--rail);
  white-space: nowrap;
}
.usage-table tbody tr:last-child td {
  border-bottom: 0;
}
</style>
