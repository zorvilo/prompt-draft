<script setup lang="ts">
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import qqIcon from "../../assets/brands/qq";
import appIcon from "../../../src-tauri/icons/icon.png";
import ShortcutSettings from "./ShortcutSettings.vue";
import { version } from "../../../package.json";
import { computed, ref, watch } from "vue";
import {
  ArrowLeft,
  Copy,
  CircleCheck,
  CircleAlert,
  Building2,
  Database,
  Download,
  Info,
  Monitor,
  Moon,
  SlidersHorizontal,
  Sun,
  Upload,
  RefreshCw,
  ChevronRight,
  Pencil,
  Plug,
  Plus,
  Search,
  Trash2,
} from "lucide-vue-next";
import {
  NButton,
  NModal,
  NProgress,
  NSwitch,
  NForm,
  NFormItem,
  NInput,
  NSelect,
  NRadioButton,
  NRadioGroup,
  NTabPane,
  NTabs,
  NTooltip,
  NSpace,
  useMessage,
} from "naive-ui";
import {
  desktop,
  errorMessage,
  type Profile,
  type AvailableUpdate,
  type SupportInfo,
  type Workspace,
} from "../../lib/tauri";

import {
  configuredModels,
  allowsHttpApi,
  createProviderProfile,
  modelPresets,
  modelsFor,
  presetFor,
  providerLogoFile,
} from "./modelPresets";
const authorQQ = "2510039438";
const supportInfo = ref<SupportInfo>();
const supportError = ref("");
async function loadSupportInfo() {
  supportError.value = "";
  try {
    supportInfo.value = await desktop.supportInfo();
  } catch (error) {
    supportError.value = errorMessage(error);
  }
}
const supportFields = computed(() => [
  { label: "应用版本", value: supportInfo.value?.version ?? version },
  { label: "运行环境", value: supportInfo.value?.runtime ?? "—" },
  { label: "操作系统", value: supportInfo.value?.os ?? "—" },
  { label: "系统架构", value: supportInfo.value?.arch ?? "—" },
]);
async function copyAboutInfo(kind: "qq" | "support") {
  if (kind === "support" && !supportInfo.value) return;
  try {
    await writeText(
      kind === "qq"
        ? authorQQ
        : [
            "词稿支持信息",
            ...supportFields.value.map(
              (field) => `${field.label}：${field.value}`,
            ),
          ].join("\n"),
    );
    message.success(kind === "qq" ? "已复制 QQ 号" : "已复制支持信息");
  } catch (error) {
    message.error(errorMessage(error));
  }
}
const settingsTab = ref("general");
const updateInfo = ref<AvailableUpdate | null>(null);
const updateBusy = ref(false);
const updateStage = ref<"check" | "download" | "install">("check");
const updateError = ref("");
const checkedUpdate = ref(false);
const updateProgress = ref<number | undefined>();
async function checkUpdate() {
  if (updateBusy.value) return;
  updateBusy.value = true;
  updateStage.value = "check";
  updateError.value = "";
  try {
    updateInfo.value = await desktop.checkUpdate();
    checkedUpdate.value = true;
  } catch (error) {
    updateError.value = errorMessage(error);
  } finally {
    updateBusy.value = false;
  }
}
const showUpdateSource = ref(false);
async function downloadUpdate(source: "gitee" | "github") {
  if (updateBusy.value || !updateInfo.value) return;
  updateBusy.value = true;
  showUpdateSource.value = false;
  updateStage.value = "download";
  updateError.value = "";
  updateProgress.value = undefined;
  try {
    await desktop.downloadUpdate(source, ({ downloaded, total }) => {
      updateProgress.value = total
        ? Math.min(100, Math.floor((downloaded / total) * 100))
        : undefined;
    });
    updateInfo.value.downloaded = true;
    updateStage.value = "install";
    await props.prepareUpdate();
    await desktop.installUpdate();
  } catch (error) {
    updateError.value = errorMessage(error);
  } finally {
    updateBusy.value = false;
  }
}
async function installUpdate() {
  if (updateBusy.value) return;
  updateBusy.value = true;
  updateStage.value = "install";
  updateError.value = "";
  try {
    await props.prepareUpdate();
    await desktop.installUpdate();
  } catch (error) {
    updateError.value = errorMessage(error);
  } finally {
    updateBusy.value = false;
  }
}
const logos = import.meta.glob<string>("../../assets/providers/*.svg", {
  eager: true,
  import: "default",
  query: "?url",
});
function providerLogo(presetId: string | undefined) {
  const file = providerLogoFile(presetId);
  return file ? logos["../../assets/providers/" + file] : undefined;
}

const props = defineProps<{
  data: Workspace;
  prepareUpdate: () => Promise<void>;
  section: "model" | "settings";
  updateShortcuts: (
    value: NonNullable<Workspace["shortcuts"]>,
  ) => Promise<boolean>;
}>();
const emit = defineEmits<{
  default: [id: string];
  backup: [];
  restore: [];
  saved: [profile: Profile];
  remove: [profile: Profile];
  theme: [value: Workspace["theme"]];
  closeBehavior: [value: Workspace["closeBehavior"]];
}>();
const message = useMessage();
watch([settingsTab, () => props.section], ([tab, section]) => {
  if (section === "settings" && tab === "about") {
    void checkUpdate();
    void loadSupportInfo();
  }
});
const updateStatus = computed(() => {
  if (updateBusy.value)
    return updateStage.value === "check"
      ? "正在检查更新…"
      : updateStage.value === "download"
        ? "正在下载更新…"
        : "正在准备安装…";
  if (updateError.value) return "更新操作未完成";
  if (updateInfo.value?.downloaded) return "更新已就绪";
  if (updateInfo.value) return `发现新版本 ${updateInfo.value.version}`;
  return checkedUpdate.value ? "你使用的是最新版本" : "正在准备检查更新";
});
const autostartEnabled = ref(false);
const autostartBusy = ref(false);
const autostartError = ref("");
async function loadAutostart() {
  autostartBusy.value = true;
  autostartError.value = "";
  try {
    autostartEnabled.value = await desktop.autostart();
  } catch (error) {
    autostartError.value = errorMessage(error);
  } finally {
    autostartBusy.value = false;
  }
}
async function changeAutostart(value: boolean) {
  if (autostartBusy.value) return;
  autostartBusy.value = true;
  try {
    autostartEnabled.value = await desktop.autostart(value);
    message.success(value ? "已开启开机自启" : "已关闭开机自启");
  } catch (error) {
    message.error(errorMessage(error));
  } finally {
    autostartBusy.value = false;
  }
}
watch(
  () => props.section,
  (section) => {
    if (section === "settings") void loadAutostart();
  },
  { immediate: true },
);
const key = ref("");
const busy = ref(false);
const fetchingModels = ref(false);
const profile = ref<Profile>();
const hasKey = ref(false);
const adding = ref(false);
const query = ref("");
const configured = computed(() => configuredModels(props.data.profiles));
const testingProvider = ref<string>();

function showModelError(item: Profile, error: unknown) {
  message.error(item.name + " · " + errorMessage(error), {
    duration: 9000,
    closable: true,
  });
}

async function testSaved(item: Profile) {
  if (testingProvider.value) return;
  testingProvider.value = item.id;
  try {
    if (!(await desktop.hasKey(item))) {
      throw new Error("请先点「编辑」，填写API Key。");
    }
    await desktop.test(item, null);
    message.success(item.name + " 连接成功");
  } catch (error) {
    showModelError(item, error);
  } finally {
    testingProvider.value = undefined;
  }
}

const selectedPreset = computed(
  () => profile.value && presetFor(profile.value),
);
const allowManualModel = computed(
  () =>
    selectedPreset.value?.id !== "preset-deepseek" &&
    selectedPreset.value?.id !== "preset-glm",
);
function selectPreset(item: (typeof modelPresets)[number]) {
  profile.value = createProviderProfile(item, configured.value);
}
function editProfile(item: Profile) {
  const preset = presetFor(item);
  profile.value = {
    ...item,
    presetId: preset?.id,
    website: item.website ?? preset?.website ?? "",
  };
}
const filtered = computed(() => {
  const term = query.value.trim().toLowerCase();
  return modelPresets.filter((item) =>
    [item.name, item.model, item.endpoint].some((text) =>
      text.toLowerCase().includes(term),
    ),
  );
});

function closeForm() {
  profile.value = undefined;
  adding.value = false;
  query.value = "";
}
watch(
  [profile, () => profile.value?.endpoint.trim()],
  async ([item, endpoint], [previous, previousEndpoint], onCleanup) => {
    let current = true;
    onCleanup(() => {
      current = false;
    });
    if (item !== previous) key.value = "";
    if (item && item === previous && endpoint !== previousEndpoint)
      item.catalog = [];
    hasKey.value = false;
    if (!item) return;
    try {
      const keyAvailable = await desktop.hasKey({
        ...item,
        endpoint: item.endpoint.trim(),
      });
      if (current) hasKey.value = keyAvailable;
    } catch (error) {
      if (current) showModelError(item, error);
    }
  },
  { immediate: true, flush: "sync" },
);

async function refreshModels() {
  const item = profile.value;
  if (!item || busy.value || fetchingModels.value) return;
  fetchingModels.value = true;
  try {
    const catalog = await desktop.fetchModels(
      { ...item, endpoint: item.endpoint.trim() },
      key.value.trim() || null,
    );
    if (profile.value !== item) return;
    item.catalog = catalog;
    message.success(
      "已获取 " + catalog.length + " 个模型。保存配置后可在工作台选择。",
    );
  } catch (error) {
    showModelError(item, error);
  } finally {
    fetchingModels.value = false;
  }
}

async function run(action: "test" | "save" | "clear") {
  const current = profile.value && {
    ...profile.value,
    endpoint: profile.value.endpoint.trim(),
    website: profile.value.website?.trim(),
  };
  if (busy.value || fetchingModels.value || !current || !selectedPreset.value)
    return;
  busy.value = true;
  try {
    if (action === "clear") {
      await desktop.setKey(current, "");
      hasKey.value = false;
      key.value = "";
      message.success("已清除已保存的密钥");
      return;
    }
    let endpoint: URL;
    try {
      endpoint = new URL(current.endpoint);
    } catch {
      throw new Error("请输入有效的模型接口地址。");
    }
    const local = allowsHttpApi(endpoint.hostname);
    if (
      !(
        endpoint.protocol === "https:" ||
        (local && endpoint.protocol === "http:")
      ) ||
      endpoint.username ||
      endpoint.password ||
      endpoint.search ||
      endpoint.hash
    )
      throw new Error(
        "公网接口请使用 HTTPS，本机和内网接口可使用 HTTP；地址中请勿加入账号、查询参数或片段。",
      );
    if (current.website) {
      let website: URL;
      try {
        website = new URL(current.website);
      } catch {
        throw new Error("请输入有效的官方网站地址。");
      }
      if (
        !["https:", "http:"].includes(website.protocol) ||
        website.username ||
        website.password
      )
        throw new Error("官方网站地址须使用 HTTP 或 HTTPS，且不包含账号信息。");
    }
    const token = key.value.trim();
    if (!token && !hasKey.value) throw new Error("请输入所选服务的 API Key。");
    if (action === "test") {
      await desktop.test(current, token || null);
      message.success(current.name + " 连接成功");
      return;
    }
    if (!current.name.trim()) throw new Error("请输入配置名称。");
    if (!current.model.trim()) throw new Error("请选择或输入模型 ID。");
    if (token) await desktop.setKey(current, token);
    hasKey.value = true;
    key.value = "";
    emit("saved", {
      ...current,
      name: current.name.trim(),
      model: current.model.trim(),
    });
    closeForm();
  } catch (error) {
    showModelError(current, error);
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <n-modal
    v-model:show="showUpdateSource"
    preset="card"
    title="选择更新下载来源"
    style="width: 440px; max-width: calc(100vw - 32px)"
  >
    <p>
      下载进度将在关于页面显示。下载并验证完成后，将自动保存数据并安装更新，应用会退出并重新启动。
    </p>
    <n-space>
      <n-button @click="downloadUpdate('gitee')">Gitee（国内推荐）</n-button>
      <n-button @click="downloadUpdate('github')">GitHub（海外推荐）</n-button>
    </n-space>
  </n-modal>
  <div
    class="settings"
    :class="{ 'is-about': section === 'settings' && settingsTab === 'about' }"
  >
    <template v-if="section === 'model'">
      <section
        v-if="!adding && !profile"
        class="provider-list"
        aria-label="供应商"
      >
        <div class="provider-toolbar">
          <h2>供应商</h2>
          <n-button
            type="primary"
            :disabled="!!testingProvider"
            @click="adding = true"
          >
            <template #icon><Plus aria-hidden="true" /></template>添加供应商
          </n-button>
        </div>
        <p v-if="!configured.length" class="provider-empty">
          还没有供应商。添加后即可用它优化提示词。
        </p>
        <div v-else class="provider-rows">
          <div v-for="item in configured" :key="item.id" class="saved-provider">
            <div class="provider-row provider-record">
              <span class="provider-icon"
                ><img
                  v-if="providerLogo(item.presetId)"
                  :src="providerLogo(item.presetId)"
                  alt="" /><Building2 v-else :size="20" aria-hidden="true"
              /></span>
              <span class="provider-row-name"
                ><strong>{{ item.name }}</strong
                ><small>{{ item.model || "OpenAI 兼容接口" }}</small></span
              >
              <div class="provider-record-actions">
                <n-button
                  size="small"
                  :disabled="item.isDefault || configured.length === 1"
                  @click="emit('default', item.id)"
                >
                  {{
                    item.isDefault || configured.length === 1
                      ? "默认供应商"
                      : "设为默认"
                  }}
                </n-button>
                <n-button
                  size="small"
                  :disabled="!!testingProvider"
                  :aria-label="'编辑 ' + item.name"
                  @click="editProfile(item)"
                >
                  <template #icon><Pencil aria-hidden="true" /></template>编辑
                </n-button>
                <n-button
                  size="small"
                  :loading="testingProvider === item.id"
                  :disabled="!!testingProvider"
                  :aria-label="'测试连接 ' + item.name"
                  @click="testSaved(item)"
                >
                  <template #icon><Plug aria-hidden="true" /></template>测试连接
                </n-button>
                <n-button
                  size="small"
                  quaternary
                  :disabled="!!testingProvider"
                  :aria-label="'删除 ' + item.name"
                  @click="emit('remove', item)"
                >
                  <template #icon><Trash2 aria-hidden="true" /></template>删除
                </n-button>
              </div>
            </div>
          </div>
        </div>
      </section>
      <section v-else-if="!profile" class="provider-list" aria-label="选择预设">
        <div class="provider-toolbar">
          <h2>选择供应商预设</h2>
          <n-button quaternary @click="closeForm"
            ><template #icon><ArrowLeft aria-hidden="true" /></template
            >返回供应商</n-button
          >
        </div>
        <n-input
          v-model:value="query"
          aria-label="搜索供应商预设"
          placeholder="搜索名称、模型或地址"
          autofocus
        >
          <template #prefix><Search :size="16" aria-hidden="true" /></template>
        </n-input>
        <div v-if="filtered.length" class="provider-grid">
          <template v-for="item in filtered" :key="item.id">
            <button
              type="button"
              class="provider-row"
              :class="{ 'provider-custom-preset': item.id === 'preset-custom' }"
              @click="selectPreset(item)"
            >
              <span class="provider-icon"
                ><img
                  v-if="providerLogo(item.id)"
                  :src="providerLogo(item.id)"
                  alt="" /><Building2 v-else :size="20" aria-hidden="true"
              /></span>
              <span class="provider-row-name"
                ><strong>{{ item.name }}</strong
                ><small>{{ item.model || "OpenAI 兼容接口" }}</small></span
              >
              <ChevronRight :size="18" aria-hidden="true" />
            </button>
            <div
              v-if="item.id === 'preset-custom' && filtered.length > 1"
              class="provider-preset-divider"
              role="separator"
            />
          </template>
        </div>
        <p v-else class="provider-empty">没有匹配的供应商预设。</p>
      </section>
      <section
        v-else-if="profile"
        class="provider-config"
        aria-label="模型配置"
      >
        <div class="provider-toolbar">
          <h2>{{ adding ? "添加供应商" : "编辑供应商" }}</h2>
          <n-button
            quaternary
            :disabled="busy || fetchingModels"
            @click="profile = undefined"
          >
            <template #icon><ArrowLeft aria-hidden="true" /></template
            >{{ adding ? "返回预设" : "返回供应商" }}
          </n-button>
        </div>
        <div class="provider-selection">
          <span class="provider-icon"
            ><img
              v-if="providerLogo(profile.presetId)"
              :src="providerLogo(profile.presetId)"
              alt="" /><Building2 v-else :size="20" aria-hidden="true"
          /></span>
          <span class="provider-row-name"
            ><strong>{{ selectedPreset?.name }}</strong
            ><small>供应商预设</small></span
          >
          <n-button
            v-if="adding"
            quaternary
            size="small"
            :disabled="busy || fetchingModels"
            @click="profile = undefined"
            >更换</n-button
          >
        </div>
        <p v-if="selectedPreset?.id === 'preset-custom'" class="provider-empty">
          支持 OpenAI 兼容 API。填写接口基础地址（例如
          https://your-api.example/v1），无需添加 /chat/completions。模型 ID
          可手动输入，或点击“获取模型”后选择。
        </p>
        <n-form
          class="model-form provider-form"
          :disabled="busy || fetchingModels"
        >
          <section class="provider-fields" aria-labelledby="provider-basic">
            <h3 id="provider-basic">基本信息</h3>
            <div class="provider-field-grid">
              <n-form-item label="配置名称" label-for="provider-name">
                <n-input
                  v-model:value="profile.name"
                  :input-props="{ id: 'provider-name' }"
                />
              </n-form-item>
              <n-form-item label="官方网站" label-for="provider-website">
                <n-input
                  v-model:value="profile!.website"
                  placeholder="选填，例如 https://example.com"
                  :input-props="{ id: 'provider-website' }"
                />
              </n-form-item>
            </div>
          </section>
          <section class="provider-fields" aria-labelledby="provider-api">
            <h3 id="provider-api">API 配置</h3>
            <n-form-item label="API Key" label-for="profile-key"
              ><n-input
                v-model:value="key"
                type="password"
                :input-props="{ id: 'profile-key', autocomplete: 'off' }"
                :placeholder="
                  hasKey
                    ? '已保存在本机，可输入新密钥替换'
                    : '输入所选服务的 API Key'
                "
            /></n-form-item>
            <div class="provider-field-grid">
              <n-form-item label="接口地址" label-for="provider-endpoint">
                <n-input
                  v-model:value="profile!.endpoint"
                  placeholder="例如 https://your-api.example/v1"
                  :input-props="{ id: 'provider-endpoint' }"
                />
              </n-form-item>
              <n-form-item label="默认模型" label-for="provider-model">
                <div class="model-catalog-control">
                  <n-select
                    id="provider-model"
                    aria-label="默认模型"
                    filterable
                    :tag="allowManualModel"
                    v-model:value="profile!.model"
                    :options="
                      modelsFor(profile!).map((model) => ({
                        label: model,
                        value: model,
                      }))
                    "
                    :placeholder="
                      allowManualModel ? '选择模型或输入 ID 后回车' : '选择模型'
                    "
                  />
                  <n-button
                    :loading="fetchingModels"
                    :disabled="busy || (!key.trim() && !hasKey)"
                    @click="refreshModels"
                  >
                    <template #icon><RefreshCw aria-hidden="true" /></template
                    >获取模型
                  </n-button>
                </div>
              </n-form-item>
            </div>
          </section>
        </n-form>
        <div class="provider-actions">
          <n-button
            text
            :disabled="busy || fetchingModels || !hasKey"
            @click="run('clear')"
            >清除密钥</n-button
          >
          <n-space justify="end">
            <n-button :disabled="busy || fetchingModels" @click="closeForm"
              >取消</n-button
            >
            <n-button
              :disabled="busy || fetchingModels || (!key.trim() && !hasKey)"
              @click="run('test')"
              >测试连接</n-button
            >
            <n-button
              type="primary"
              :disabled="busy || fetchingModels || (!key.trim() && !hasKey)"
              @click="run('save')"
              >保存配置</n-button
            >
          </n-space>
        </div>
      </section>
    </template>
    <div v-else class="preferences">
      <n-tabs
        type="segment"
        class="preferences-tabs"
        pane-class="settings-scroll"
        :animated="false"
        v-model:value="settingsTab"
      >
        <n-tab-pane name="general">
          <template #tab
            ><SlidersHorizontal :size="15" aria-hidden="true" />常规</template
          >
          <div class="preferences-body">
            <section
              class="preference-group"
              aria-labelledby="appearance-heading"
            >
              <h2 id="appearance-heading">外观</h2>
              <div class="preference-card">
                <div class="preference-row">
                  <div>
                    <h3>主题样式</h3>
                    <p>跟随系统时，随系统外观自动切换。</p>
                  </div>
                  <n-radio-group
                    :value="data.theme"
                    class="theme-options"
                    :theme-overrides="{
                      buttonHeightSmall: '34px',
                      buttonBoxShadowFocus: 'none',
                      buttonColorActive: 'var(--selection)',
                      buttonTextColorActive: 'var(--ink)',
                    }"
                    size="small"
                    aria-label="主题样式"
                    @update:value="
                      (value: Workspace['theme']) => emit('theme', value)
                    "
                  >
                    <n-radio-button value="system"
                      ><span class="theme-option"
                        ><Monitor :size="16" aria-hidden="true" />跟随系统</span
                      ></n-radio-button
                    >
                    <n-radio-button value="light"
                      ><span class="theme-option"
                        ><Sun :size="16" aria-hidden="true" />浅色</span
                      ></n-radio-button
                    >
                    <n-radio-button value="dark"
                      ><span class="theme-option"
                        ><Moon :size="16" aria-hidden="true" />深色</span
                      ></n-radio-button
                    >
                  </n-radio-group>
                </div>
              </div>
            </section>
            <section class="preference-group" aria-labelledby="window-heading">
              <h2 id="window-heading">窗口行为</h2>
              <div class="preference-card">
                <div class="preference-row">
                  <div>
                    <h3>开机自启</h3>
                    <p>登录系统后自动启动词稿。</p>
                    <p v-if="autostartError" role="alert">
                      {{ autostartError }}
                    </p>
                  </div>
                  <div class="shortcut-actions">
                    <n-button
                      v-if="autostartError"
                      size="small"
                      @click="loadAutostart"
                      >重试</n-button
                    ><n-switch
                      :value="autostartEnabled"
                      :loading="autostartBusy"
                      :disabled="autostartBusy || !!autostartError"
                      aria-label="开机自启"
                      @update:value="changeAutostart"
                    />
                  </div>
                </div>
                <div class="preference-row window-behavior-row">
                  <div>
                    <h3>关闭时最小化到托盘</h3>
                    <p>选择点击窗口关闭按钮时的行为。</p>
                  </div>
                  <n-radio-group
                    :value="data.closeBehavior ?? 'ask'"
                    size="small"
                    aria-label="关闭窗口行为"
                    @update:value="
                      (value: Workspace['closeBehavior']) =>
                        emit('closeBehavior', value)
                    "
                  >
                    <n-radio-button value="ask">每次询问</n-radio-button
                    ><n-radio-button value="hide">隐藏到托盘</n-radio-button
                    ><n-radio-button value="exit">退出程序</n-radio-button>
                  </n-radio-group>
                </div>
              </div>
            </section>
            <shortcut-settings
              :data="data"
              :update-shortcuts="updateShortcuts"
            />
          </div>
        </n-tab-pane>
        <n-tab-pane name="data">
          <template #tab
            ><Database :size="15" aria-hidden="true" />数据与备份</template
          >
          <div class="preferences-body">
            <section
              class="preference-group"
              aria-labelledby="workspace-heading"
            >
              <h2 id="workspace-heading">本机数据</h2>
              <div class="preference-card">
                <div class="preference-row">
                  <h3>工作区概览</h3>
                  <span class="workspace-summary"
                    >{{ data.projects.length }} 个项目<span aria-hidden="true">
                      / </span
                    >{{ data.drafts.length }} 份草稿<span aria-hidden="true">
                      / </span
                    >{{
                      data.drafts.reduce(
                        (total, item) => total + item.messages.length,
                        0,
                      )
                    }}
                    条聊天消息</span
                  >
                </div>
                <div class="preference-row">
                  <h3>存储方式</h3>
                  <span class="muted">本地存储</span>
                </div>
              </div>
            </section>
            <section class="preference-group" aria-labelledby="backup-heading">
              <h2 id="backup-heading">备份与恢复</h2>
              <div class="preference-card">
                <div class="preference-row">
                  <div>
                    <h3>导出完整备份</h3>
                    <p>将全部本地数据与配置保存为 JSON，可迁移到新电脑。</p>
                  </div>
                  <n-button size="small" @click="emit('backup')"
                    ><template #icon><Download aria-hidden="true" /></template
                    >导出备份</n-button
                  >
                </div>
                <div class="preference-row">
                  <div>
                    <h3>从备份恢复</h3>
                    <p>覆盖当前全部数据和配置，校验通过后会再次确认。</p>
                  </div>
                  <n-button size="small" @click="emit('restore')"
                    ><template #icon><Upload aria-hidden="true" /></template
                    >恢复备份</n-button
                  >
                </div>
                <div class="backup-scope">
                  <Info :size="16" aria-hidden="true" />
                  <p>
                    备份包含项目、草稿、聊天记录、用户模板及分类、供应商、应用设置、用量、价格和目录缓存，<strong
                      >包含明文 API Key</strong
                    >。请妥善保管文件。新电脑请使用同版本软件恢复，开机自启需重新设置。
                  </p>
                </div>
              </div>
            </section>
            <section class="preference-group" aria-labelledby="privacy-heading">
              <h2 id="privacy-heading">数据与隐私</h2>
              <div class="privacy-panel">
                <section
                  class="privacy-section"
                  aria-labelledby="local-privacy-heading"
                >
                  <div class="privacy-title">
                    <Monitor :size="20" aria-hidden="true" />
                    <h3 id="local-privacy-heading">数据留在你的电脑</h3>
                  </div>
                  <p class="privacy-summary">
                    全部应用数据本地保存，不上传至词稿服务器。
                  </p>
                  <p class="privacy-detail">
                    草稿、聊天记录、项目、模板、设置、用量记录和 API Key
                    均保存在本机。词稿不提供云端同步。
                  </p>
                  <p class="privacy-note">请妥善保管电脑、数据文件及备份。</p>
                </section>
                <section
                  class="privacy-section"
                  aria-labelledby="request-privacy-heading"
                >
                  <div class="privacy-title">
                    <Plug :size="20" aria-hidden="true" />
                    <h3 id="request-privacy-heading">模型请求由你选择</h3>
                  </div>
                  <p class="privacy-summary">
                    优化与迭代时，相关内容直接发往所选供应商。
                  </p>
                  <p class="privacy-detail">
                    请求包含提示词和必要上下文，API Key
                    用于鉴权。供应商可能按其隐私政策和服务条款处理、保留请求数据。
                  </p>
                  <p class="privacy-note">
                    发送前请确认供应商，避免提交敏感信息。
                  </p>
                </section>
              </div>
            </section>
          </div>
        </n-tab-pane>
        <n-tab-pane name="about">
          <template #tab><Info :size="15" aria-hidden="true" />关于</template>
          <div class="about-page">
            <section class="about-identity" aria-labelledby="about-heading">
              <img :src="appIcon" alt="" class="about-logo" />
              <h2 id="about-heading">词稿</h2>
              <span class="about-version">版本 {{ version }}</span>
              <p>专注提示词的编写、优化与管理。</p>
            </section>
            <section
              class="about-updates"
              aria-labelledby="update-heading"
              :aria-busy="updateBusy"
            >
              <div class="about-update-heading">
                <div
                  class="about-update-status"
                  role="status"
                  aria-live="polite"
                >
                  <CircleAlert
                    v-if="updateError"
                    :size="20"
                    aria-hidden="true"
                  />
                  <Download
                    v-else-if="updateInfo"
                    :size="20"
                    aria-hidden="true"
                  />
                  <RefreshCw
                    v-else-if="updateBusy"
                    :size="20"
                    aria-hidden="true"
                  />
                  <CircleCheck v-else :size="20" aria-hidden="true" />
                  <h3 id="update-heading">{{ updateStatus }}</h3>
                </div>
                <n-button
                  size="small"
                  :loading="updateBusy && updateStage === 'check'"
                  :disabled="updateBusy"
                  @click="checkUpdate"
                  ><template #icon
                    ><RefreshCw :size="15" aria-hidden="true" /></template
                  >{{ updateError ? "重试检查" : "重新检查" }}</n-button
                >
              </div>
              <p v-if="updateError" class="about-update-error" role="alert">
                {{ updateError }}
              </p>
              <div v-if="updateInfo" class="about-update-content">
                <p v-if="updateInfo.notes" class="app-update-notes">
                  {{ updateInfo.notes }}
                </p>
                <n-progress
                  v-if="updateBusy && updateStage === 'download'"
                  type="line"
                  :percentage="updateProgress ?? 0"
                  :processing="true"
                  :show-indicator="updateProgress !== undefined"
                />
                <div class="about-update-actions">
                  <n-button
                    v-if="!updateInfo.downloaded"
                    type="primary"
                    :loading="updateBusy && updateStage === 'download'"
                    :disabled="updateBusy"
                    @click="showUpdateSource = true"
                    >下载更新</n-button
                  >
                  <n-button
                    v-else
                    type="primary"
                    :loading="updateBusy && updateStage === 'install'"
                    :disabled="updateBusy"
                    @click="installUpdate"
                    >安装并重启</n-button
                  >
                  <span v-if="updateInfo.downloaded"
                    >安装前请结束模型请求，应用会先保存当前数据。</span
                  >
                </div>
              </div>
            </section>
            <section
              class="about-contact-row"
              aria-labelledby="contact-heading"
            >
              <div class="about-contact-heading">
                <h3 id="contact-heading">联系作者</h3>
                <p>使用建议与问题反馈，欢迎联系。</p>
              </div>
              <div class="about-contact-details">
                <span class="about-author-name">刘涵宇<span>开发者</span></span>
                <n-tooltip :delay="250" placement="top">
                  <template #trigger>
                    <n-button
                      class="about-contact"
                      :icon-size="24"
                      size="small"
                      aria-label="复制作者 QQ 号 2510039438"
                      @click="copyAboutInfo('qq')"
                    >
                      <template #icon>
                        <img
                          :src="qqIcon"
                          class="qq-brand"
                          alt=""
                          aria-hidden="true"
                        />
                      </template>
                      <span class="about-contact-number">{{ authorQQ }}</span>
                      <Copy
                        :size="14"
                        class="about-contact-copy"
                        aria-hidden="true"
                      />
                    </n-button>
                  </template>
                  点击复制 QQ 号
                </n-tooltip>
              </div>
            </section>
            <section class="about-support" aria-labelledby="support-heading">
              <div class="about-support-heading">
                <div>
                  <h3 id="support-heading">支持信息</h3>
                  <p>反馈问题时，可复制以下环境信息。</p>
                </div>
                <n-button
                  size="small"
                  :disabled="!supportInfo"
                  @click="copyAboutInfo('support')"
                  ><template #icon
                    ><Copy :size="14" aria-hidden="true" /></template
                  >复制支持信息</n-button
                >
              </div>
              <div class="about-support-grid">
                <div v-for="field in supportFields" :key="field.label">
                  <span>{{ field.label }}</span><strong>{{
                    field.value
                  }}</strong>
                </div>
              </div>
              <div v-if="supportError" class="about-update-error" role="alert">
                {{ supportError }}
                <n-button text size="small" @click="loadSupportInfo"
                  >重新获取</n-button
                >
              </div>
            </section>
          </div>
        </n-tab-pane>
      </n-tabs>
    </div>
  </div>
</template>
