<script setup lang="ts">
import { computed, onMounted, ref, shallowRef, watch } from "vue";
import {
  NButton,
  NCheckbox,
  NInput,
  NModal,
  NSelect,
  NSpin,
  NPagination,
} from "naive-ui";
import { desktop, errorMessage, type ModelsDevEntry } from "../../lib/tauri";
const emit = defineEmits<{ close: []; saved: [] }>();
const rows = shallowRef<ModelsDevEntry[]>([]);
const query = ref("");
const provider = ref("");
const chosen = ref<string>();
const loading = ref(false);
const saving = ref(false);
const error = ref("");
const providers = computed(() => [
  { label: "全部供应商", value: "" },
  ...[
    ...new Map(
      rows.value.map((row) => [row.provider, row.providerName]),
    ).entries(),
  ].map(([value, label]) => ({ value, label })),
]);
const filtered = computed(() =>
  rows.value.filter(
    (row) =>
      (!provider.value || row.provider === provider.value) &&
      [row.model, row.name, row.providerName]
        .join(" ")
        .toLowerCase()
        .includes(query.value.trim().toLowerCase()),
  ),
);
const page = ref(1);
const visible = computed(() =>
  filtered.value.slice((page.value - 1) * 50, page.value * 50),
);
watch([query, provider], () => {
  page.value = 1;
});
const picked = computed(() =>
  rows.value.find((row) => row.key === chosen.value),
);
async function load(refresh = false) {
  loading.value = true;
  error.value = "";
  try {
    const result = await desktop.modelsDevPicker(refresh);
    rows.value = result.entries;
  } catch (cause) {
    error.value = errorMessage(cause);
  } finally {
    loading.value = false;
  }
}
function toggle(key: string) {
  chosen.value = chosen.value === key ? undefined : key;
}
async function save() {
  saving.value = true;
  error.value = "";
  try {
    if (!chosen.value) return;
    await desktop.importModelsDevPrice(chosen.value);
    emit("saved");
  } catch (cause) {
    error.value = errorMessage(cause);
  } finally {
    saving.value = false;
  }
}
onMounted(() => {
  void load();
});
</script>
<template>
  <n-modal
    :show="true"
    preset="card"
    title="从 models.dev 导入定价"
    :style="{ width: 'min(900px, calc(100vw - 32px))' }"
    :mask-closable="false"
    :closable="!saving"
    :close-on-esc="!saving"
    @update:show="
      (value) => {
        if (!value) emit('close');
      }
    "
  >
    <template #header-extra
      ><n-button size="small" :disabled="loading || saving" @click="load(true)"
        >刷新目录</n-button
      ></template
    >
    <div class="models-picker">
      <p>每次选择一个模型导入人民币定价，导入后可编辑。</p>
      <div class="picker-toolbar">
        <n-select
          v-model:value="provider"
          :options="providers"
          :disabled="saving"
          aria-label="筛选 models.dev 供应商"
        />
        <n-input
          v-model:value="query"
          :disabled="saving"
          clearable
          placeholder="搜索模型或供应商"
          aria-label="搜索 models.dev 模型"
        />
      </div>
      <p v-if="error" role="alert">
        {{ error }}
        <n-button :disabled="saving" @click="load(true)">重试</n-button>
      </p>
      <n-spin v-if="loading" />
      <div v-else class="picker-list">
        <button
          v-for="row in visible"
          :key="row.key"
          type="button"
          :disabled="saving"
          :aria-pressed="chosen === row.key"
          @click="toggle(row.key)"
        >
          <n-checkbox
            :checked="chosen === row.key"
            :focusable="false"
            aria-hidden="true"
            style="pointer-events: none"
          />
          <span
            ><strong>{{ row.name }}</strong
            ><small>{{ row.providerName }} · {{ row.model }}</small></span
          >
          <span class="picker-price"
            >输入 ${{ row.input }} / 输出 ${{ row.output
            }}<small
              >缓存读 ${{ row.cacheRead }} / 写 ${{ row.cacheWrite }}</small
            ></span
          >
        </button>
        <p v-if="!visible.length">暂无匹配模型。</p>
      </div>
      <n-pagination
        class="pricing-pagination"
        v-if="filtered.length > 50"
        v-model:page="page"
        :page-size="50"
        :item-count="filtered.length"
        :page-slot="5"
      />

      <p v-if="picked">
        导入 {{ picked.name }}，单价按已缓存汇率折算成人民币。
      </p>
    </div>
    <template #footer
      ><div class="picker-toolbar picker-footer">
        <n-button :disabled="saving" @click="emit('close')">取消</n-button>
        <n-button
          type="primary"
          :loading="saving"
          :disabled="loading || !rows.length || !chosen"
          @click="save"
          >导入所选模型</n-button
        >
      </div></template
    >
  </n-modal>
</template>
<style scoped>
.pricing-pagination {
  justify-self: end;
  align-self: flex-end;
  margin-left: auto;
  flex-shrink: 0;
}

.models-picker {
  display: flex;
  flex-direction: column;
  gap: 12px;
  height: 65dvh;
  min-height: 0;
  overflow: hidden;
}
.models-picker p {
  margin: 0;
  font-size: 12px;
  color: var(--muted);
}
.picker-toolbar {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}
.picker-toolbar > .n-select {
  width: 190px;
}
.picker-toolbar > .n-input {
  flex: 1;
  min-width: 160px;
}
.picker-list {
  flex: 1;
  min-height: 0;
  overscroll-behavior: contain;
  scrollbar-width: thin;
  scrollbar-color: var(--scroll-thumb) transparent;
  overflow: auto;
  border: 1px solid var(--line);
  border-radius: 6px;
}
.picker-list button {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  background: var(--paper);
  color: var(--ink);
  padding: 12px;
  text-align: left;
  border: 0;
  border-bottom: 1px solid var(--line);
  cursor: pointer;
}
.picker-list button[aria-pressed="true"] {
  background: var(--selection);
}
.picker-list button > span:nth-child(2) {
  flex: 1;
  min-width: 0;
  overflow-wrap: anywhere;
}
.picker-list small {
  display: block;
  color: var(--muted);
  margin-top: 4px;
}
.picker-price {
  font-size: 12px;
  text-align: right;
}
.picker-footer {
  justify-content: flex-end;
}
</style>
