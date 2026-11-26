<template>
  <div class="settings">
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
                @click="setDefaultProfile(item.id)"
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
                @click="removeProfile(item)"
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
    <section v-else-if="profile" class="provider-config" aria-label="模型配置">
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
  </div>
</template>

<script setup lang="ts">
import {
  ArrowLeft,
  Building2,
  ChevronRight,
  Pencil,
  Plug,
  Plus,
  RefreshCw,
  Search,
  Trash2,
} from "lucide-vue-next";
import { NButton, NForm, NFormItem, NInput, NSelect, NSpace } from "naive-ui";

import { modelsFor } from "@/config/modelPresets";

import { useWorkspaceContext } from "@/layout/hooks/workspaceContext";
import { useProviderForm } from "@/views/models/hooks/useProviderForm";
const { removeProfile, setDefaultProfile } = useWorkspaceContext();
const {
  providerLogo,
  key,
  busy,
  fetchingModels,
  profile,
  hasKey,
  adding,
  query,
  configured,
  testingProvider,
  testSaved,
  selectedPreset,
  allowManualModel,
  selectPreset,
  editProfile,
  filtered,
  closeForm,
  refreshModels,
  run,
} = useProviderForm();
</script>
