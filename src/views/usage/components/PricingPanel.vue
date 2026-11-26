<template>
  <div class="pricing-page">
    <div class="pricing-toolbar">
      <n-button quaternary @click="$emit('back')"
        ><template #icon><ArrowLeft aria-hidden="true" /></template
        >返回统计</n-button
      >
      <span class="pricing-spacer" />
      <n-button :disabled="loading" @click="refresh(true)"
        ><template #icon><RefreshCw aria-hidden="true" /></template
        >同步</n-button
      >
      <n-button type="primary" @click="edit()"
        ><template #icon><Plus aria-hidden="true" /></template
        >新增模型定价</n-button
      >
    </div>
    <p v-if="settings?.lastError" role="alert">
      上次同步失败 {{ settings.lastError }}
    </p>
    <div class="pricing-notice">
      <strong>人民币 / 百万 Token</strong>
      <p>单价显示四舍五入至一位小数，费用计算仍使用完整精度。</p>
      <p>
        同步不会覆盖 deepseek-flash 和 deepseek-v4-pro，保留系统内置参考价。
      </p>
      <form class="pricing-rate-form" @submit.prevent="saveRate">
        <label for="pricing-exchange-rate">1 美元折合人民币</label>
        <n-input
          v-model:value="exchangeRate"
          :input-props="{ id: 'pricing-exchange-rate', inputmode: 'decimal' }"
          :disabled="loading || saving"
          placeholder="例如 6.663"
        />
        <span>元</span>
        <n-button
          attr-type="submit"
          size="small"
          :loading="loading && !syncing"
          :disabled="
            loading || saving || exchangeRate.trim() === settings?.rate
          "
          >保存汇率</n-button
        >
      </form>
      <p v-if="rateError" role="alert">{{ rateError }}</p>
      <p v-if="settings?.rateDate">参考汇率日期 {{ settings.rateDate }}</p>
      <p>
        内置及 models.dev
        参考报价以美元为基础，按汇率折算成人民币，可能与供应商人民币挂牌价或实际账单存在差异。费用仅供参考。
      </p>
      <p>
        保存汇率会重新换算参考单价，不修改自定义人民币单价及已计价的历史金额。同步
        models.dev
        时会重新获取汇率。需要按实际价格计费，可在下表点击“编辑”填写人民币单价。
      </p>
      <p v-if="settings?.syncedAt">
        上次同步
        {{ new Date(settings.syncedAt * 1000).toLocaleString("zh-CN") }}
      </p>
    </div>
    <div class="pricing-toolbar">
      <h2>
        模型定价 <small>{{ filtered.length }} 条</small>
      </h2>
      <n-input
        v-model:value="query"
        clearable
        aria-label="搜索模型定价"
        placeholder="搜索模型 ID 或接口范围"
      />
      <n-button @click="picker = true">从 models.dev 导入</n-button>
      <n-select
        v-model:value="source"
        aria-label="价格来源"
        :options="[
          { label: '全部来源', value: 'all' },
          { label: '系统内置', value: 'CC Switch' },
          { label: 'models.dev', value: 'models.dev' },
          { label: '自定义', value: 'manual' },
        ]"
      />
    </div>
    <p v-if="error" role="alert">
      {{ error }} <n-button size="small" @click="refresh()">重试</n-button>
    </p>
    <n-modal
      :show="syncing"
      preset="card"
      aria-label="正在同步模型价格"
      :content-style="{ padding: '20px 24px' }"
      :closable="false"
      :mask-closable="false"
      :close-on-esc="false"
      :style="{ width: 'min(400px, calc(100vw - 32px))', marginTop: '10vh' }"
    >
      <div role="status" aria-live="polite" class="pricing-sync-status">
        <n-spin :size="22" class="pricing-sync-spinner" />
        <div class="pricing-sync-copy">
          <strong>正在同步模型价格</strong
          ><span>获取 models.dev 报价与最新汇率…</span>
        </div>
      </div>
    </n-modal>
    <div class="pricing-table-wrap" :aria-busy="loading">
      <table aria-label="模型定价表">
        <thead>
          <tr>
            <th>模型 / 接口范围</th>
            <th>来源</th>
            <th v-for="field in fields" :key="field.key">{{ field.label }}</th>
            <th>操作</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="row in visible" :key="row.host + ':' + row.model">
            <td>
              <strong>{{ row.model }}</strong><small>{{
                row.host || "所有供应商"
              }}</small>
            </td>
            <td>{{ label(row.source) }}</td>
            <td v-for="field in fields" :key="field.key">
              {{ money(row[field.key]) }}
            </td>
            <td>
              <n-button size="small" @click="edit(row)">编辑</n-button
              ><n-button
                v-if="row.source === 'manual'"
                size="small"
                quaternary
                @click="remove(row)"
                >移除</n-button
              >
            </td>
          </tr>
          <tr v-if="!visible.length">
            <td colspan="7">暂无匹配的模型定价。</td>
          </tr>
        </tbody>
      </table>
    </div>
    <n-pagination
      class="pricing-pagination"
      v-if="filtered.length > 20"
      v-model:page="page"
      :page-size="20"
      :item-count="filtered.length"
      :page-slot="5"
    />
    <models-dev-picker
      v-if="picker"
      @close="picker = false"
      @saved="
        picker = false;
        refresh();
      "
    />
    <n-modal
      :show="!!editing"
      preset="card"
      :title="adding ? '新增模型定价' : '编辑模型定价'"
      :style="{ width: 'min(470px, calc(100vw - 32px))' }"
      :closable="!saving"
      :mask-closable="false"
      :close-on-esc="!saving"
      @update:show="
        (value) => {
          if (!value) editing = undefined;
        }
      "
    >
      <form v-if="editing" class="pricing-form" @submit.prevent="save">
        <label
          >模型 ID<n-input
            v-model:value="editing.model"
            :readonly="!adding"
            :disabled="saving"
            aria-label="定价模型 ID"
        /></label>
        <label
          >接口范围<n-input
            v-model:value="editing.host"
            :readonly="!adding"
            :disabled="saving"
            placeholder="留空通用，或填写完整接口基础地址"
            aria-label="定价接口范围"
        /></label>
        <p>
          单价单位为人民币元 / 百万
          Token。编辑系统或同步报价会创建自定义价格，已有账单保留。
        </p>
        <label v-for="field in fields" :key="field.key"
          >{{ field.label }}单价<n-input
            v-model:value="editing[field.key]"
            :disabled="saving"
            :aria-label="field.label + '单价'"
            placeholder="输入非负单价，免费填 0"
        /></label>
        <p v-if="formError" role="alert">{{ formError }}</p>
        <n-button type="primary" attr-type="submit" :loading="saving"
          >保存</n-button
        >
      </form>
    </n-modal>
  </div>
</template>

<script setup lang="ts">
import { ArrowLeft, Plus, RefreshCw } from "lucide-vue-next";
import {
  NButton,
  NInput,
  NModal,
  NPagination,
  NSelect,
  NSpin,
  useDialog,
  useMessage,
} from "naive-ui";
import { computed, onMounted, ref, watch } from "vue";
import {
  desktop,
  errorMessage,
  type CatalogPrice,
  type UsageCatalogSettings,
} from "@/api/desktop";
import ModelsDevPicker from "./ModelsDevPicker.vue";

defineEmits<{ back: [] }>();
const picker = ref(false);
const rows = ref<CatalogPrice[]>([]);
const settings = ref<UsageCatalogSettings>();
const exchangeRate = ref("");
const rateError = ref("");
const loading = ref(false);
const syncing = ref(false);
const saving = ref(false);
const error = ref("");
const query = ref("");
const source = ref("all");
const page = ref(1);
const editing = ref<CatalogPrice>();
const adding = ref(false);
const formError = ref("");
const dialog = useDialog();
const message = useMessage();
const fields = [
  { key: "input", label: "输入" },
  { key: "output", label: "输出" },
  { key: "cacheRead", label: "缓存读取" },
  { key: "cacheWrite", label: "缓存写入" },
] as const;
const filtered = computed(() =>
  rows.value.filter(
    (row) =>
      (source.value === "all" || row.source === source.value) &&
      (row.model + " " + row.host)
        .toLowerCase()
        .includes(query.value.trim().toLowerCase()),
  ),
);
const visible = computed(() =>
  filtered.value.slice((page.value - 1) * 20, page.value * 20),
);
watch([query, source], () => {
  page.value = 1;
});
const label = (source: string) =>
  source === "manual" ? "自定义" : source === "CC Switch" ? "系统内置" : source;
const money = (value: string) =>
  new Intl.NumberFormat("zh-CN", {
    style: "currency",
    currency: "CNY",
    minimumFractionDigits: 1,
    maximumFractionDigits: 1,
  }).format(Number(value));

async function load() {
  const result = await Promise.all([
    desktop.usageCatalogRows(),
    desktop.usageCatalogSettings(),
  ]);
  rows.value = result[0];
  settings.value = result[1];
  exchangeRate.value = result[1].rate ?? "";
  page.value = Math.min(
    page.value,
    Math.max(1, Math.ceil(filtered.value.length / 20)),
  );
}
async function refresh(sync = false) {
  if (loading.value) return;
  syncing.value = sync;
  loading.value = true;
  error.value = "";
  try {
    if (sync) {
      const result = await desktop.updateUsageCatalog(true);
      message.success("已同步 " + (result.synced ?? 0) + " 条所选报价");
    }
    await load();
  } catch (cause) {
    error.value = errorMessage(cause);
  } finally {
    loading.value = false;
    syncing.value = false;
  }
}
async function saveRate() {
  if (loading.value || saving.value) return;
  rateError.value = "";
  const rate = exchangeRate.value.trim();
  if (
    !rate ||
    !Number.isFinite(Number(rate)) ||
    Number(rate) <= 0 ||
    Number(rate) > 100
  ) {
    rateError.value = "请输入大于 0 且不超过 100 的换算汇率。";
    return;
  }
  loading.value = true;
  try {
    await desktop.updateUsageCatalog(false, rate);
    await load();
    message.success("汇率已保存，参考单价已重新换算");
  } catch (cause) {
    rateError.value = errorMessage(cause);
  } finally {
    loading.value = false;
  }
}
function edit(row?: CatalogPrice) {
  adding.value = !row;
  editing.value = row
    ? { ...row }
    : {
        host: "",
        model: "",
        input: "",
        output: "",
        cacheRead: "",
        cacheWrite: "",
        source: "manual",
      };
  formError.value = "";
}
async function save() {
  if (!editing.value || saving.value) return;
  saving.value = true;
  formError.value = "";
  try {
    await desktop.saveUsageCatalogPrice(editing.value);
    editing.value = undefined;
    await load();
    message.success("定价已保存");
  } catch (cause) {
    formError.value = errorMessage(cause);
  } finally {
    saving.value = false;
  }
}
function remove(row: CatalogPrice) {
  dialog.warning({
    title: "移除自定义价格",
    content: "移除后会恢复对应的系统内置或同步报价。已经入账的金额保留。",
    positiveText: "移除",
    negativeText: "取消",
    onPositiveClick: async () => {
      try {
        await desktop.saveUsageCatalogPrice(row, true);
        await load();
      } catch (cause) {
        message.error(errorMessage(cause));
        return false;
      }
    },
  });
}
onMounted(() => {
  void refresh();
  void desktop.modelsDevPicker().catch(() => {});
});
</script>

<style scoped>
.pricing-sync-status {
  display: flex;
  align-items: center;
  gap: 16px;
}
.pricing-sync-spinner {
  flex-shrink: 0;
}
.pricing-sync-copy {
  display: grid;
  gap: 5px;
  min-width: 0;
}
.pricing-sync-copy strong {
  font-size: 14px;
  font-weight: 600;
  line-height: 1.5;
}
.pricing-sync-copy span {
  font-size: 12px;
  color: var(--muted);
  line-height: 1.6;
}
.pricing-rate-form {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
  margin-top: 12px;
}
.pricing-rate-form .n-input {
  width: 140px;
}

.pricing-pagination {
  justify-self: end;
  align-self: flex-end;
  margin-left: auto;
  flex-shrink: 0;
}

.pricing-page {
  display: grid;
  grid-template-columns: minmax(0, 1fr);
  gap: 16px;
}
.pricing-toolbar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 10px;
}
.pricing-spacer {
  flex: 1;
}
.pricing-toolbar h2 {
  font-size: 16px;
  margin: 0;
}
.pricing-toolbar h2 small {
  font-size: 12px;
  color: var(--muted);
  font-weight: 400;
}
.pricing-toolbar > .n-input {
  flex: 1;
  min-width: 160px;
}
.pricing-toolbar > .n-select {
  width: 140px;
}
.pricing-notice {
  border: 1px solid var(--line);
  background: var(--rail);
  padding: 14px 16px;
  border-radius: 8px;
}
.pricing-notice p {
  margin: 5px 0 0;
  font-size: 12px;
  color: var(--muted);
}
.pricing-table-wrap {
  overflow: auto;
  border: 1px solid var(--line);
  border-radius: 8px;
}
table {
  width: 100%;
  border-collapse: collapse;
  text-align: left;
  font-size: 13px;
}
th,
td {
  padding: 10px 12px;
  border-bottom: 1px solid var(--line);
}
th {
  background: var(--rail);
  white-space: nowrap;
}
td {
  white-space: nowrap;
}
td:first-child {
  white-space: normal;
  overflow-wrap: anywhere;
  min-width: 180px;
}
td small {
  display: block;
  color: var(--muted);
  margin-top: 4px;
}
.pricing-form {
  display: grid;
  gap: 12px;
  max-height: calc(100dvh - 160px);
  overflow: auto;
}
.pricing-form label {
  display: grid;
  gap: 5px;
}
.pricing-form p {
  margin: 0;
  color: var(--muted);
  font-size: 12px;
}
</style>
