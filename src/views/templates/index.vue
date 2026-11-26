<template>
  <div v-if="!libraryCategory" class="template-categories">
    <div class="page-actions">
      <n-button type="primary" @click="editTemplateCategory()">
        <template #icon><Plus aria-hidden="true" /></template>
        添加模板分类
      </n-button>
    </div>
    <div
      v-for="category in libraryCategories"
      :key="category.value"
      class="template-category-row"
    >
      <button
        type="button"
        class="template-category"
        @click="libraryCategory = category.value"
      >
        <strong>{{ category.label }}</strong>
        <span>{{ category.description }}</span>
        <ChevronRight aria-hidden="true" :size="16" />
      </button>
      <span
        v-if="isBuiltinCategory(category.value)"
        class="builtin-category-label"
        >系统内置</span
      >
      <div v-else class="template-category-actions">
        <n-button
          size="small"
          :aria-label="'编辑分类 ' + category.label"
          @click="editTemplateCategory(category)"
          >编辑</n-button
        >
        <n-button
          size="small"
          quaternary
          :aria-label="'删除分类 ' + category.label"
          @click="deleteTemplateCategory(category)"
          >删除</n-button
        >
      </div>
    </div>
    <p v-if="!libraryCategories.length" class="muted">
      暂无模板分类，点击“添加模板分类”开始创建。
    </p>
  </div>
  <div v-else-if="editingTemplate" class="template-editor">
    <n-form
      :show-feedback="false"
      :disabled="templateSaving"
      @submit.prevent="saveTemplate"
    >
      <p v-if="viewingBuiltin" class="muted">系统内置模板，仅供查看。</p>
      <n-form-item label="提示词名称" label-for="template-name">
        <n-input
          v-model:value="editingTemplate.name"
          :readonly="viewingBuiltin"
          :input-props="{ id: 'template-name' }"
          :maxlength="100"
        />
      </n-form-item>
      <n-form-item
        v-for="field in templateFields"
        :key="field.key"
        :label="field.label"
        :label-for="field.id"
      >
        <div class="template-field" @wheel.capture="routeTemplateWheel">
          <div class="template-field-actions">
            <n-button
              size="tiny"
              :secondary="templatePreviewOpen[field.role]"
              :quaternary="!templatePreviewOpen[field.role]"
              :aria-label="field.label + '预览'"
              :aria-pressed="templatePreviewOpen[field.role]"
              @click="
                templatePreviewOpen[field.role] =
                  !templatePreviewOpen[field.role]
              "
              >{{
                templatePreviewOpen[field.role]
                  ? viewingBuiltin
                    ? "返回原文"
                    : "返回编辑"
                  : "预览"
              }}</n-button
            >
            <n-button
              quaternary
              size="small"
              :disabled="!templateDisplayText(field.role)"
              :aria-label="'复制' + field.label"
              :title="'复制' + field.label"
              @click="copyTemplateText(field.role)"
            >
              <template #icon><Copy aria-hidden="true" /></template>
            </n-button>
            <n-button
              quaternary
              size="small"
              :disabled="!templateDisplayText(field.role)"
              :aria-label="'放大查看' + field.label"
              :title="'放大查看' + field.label"
              @click="enlargedTemplateRole = field.role"
            >
              <template #icon><Maximize2 aria-hidden="true" /></template>
            </n-button>
          </div>
          <div
            v-if="templatePreviewOpen[field.role]"
            :id="field.id"
            class="template-content-input template-inline-preview"
          >
            <pre tabindex="0" :aria-label="field.label + '预览'">{{
              templatePreviewError ||
              templatePreview.find((item) => item.role === field.role)
                ?.content ||
              "填写系统提示词后即可预览。"
            }}</pre>
          </div>
          <n-input
            v-else
            v-model:value="editingTemplate[field.key]"
            :readonly="viewingBuiltin"
            type="textarea"
            :input-props="{ id: field.id }"
            class="template-content-input"
            :resizable="false"
            :maxlength="20000"
            :placeholder="field.placeholder"
          />
        </div>
      </n-form-item>
      <n-form-item label="提示词描述" label-for="template-description">
        <n-input
          v-model:value="editingTemplate.description"
          :readonly="viewingBuiltin"
          type="textarea"
          :input-props="{ id: 'template-description' }"
          class="template-description-input"
          :resizable="false"
          :maxlength="500"
        />
      </n-form-item>
      <div v-if="!viewingBuiltin" class="template-actions">
        <n-button
          :disabled="templateSaving"
          @click="editingTemplate = undefined"
          >取消</n-button
        >
        <n-button type="primary" attr-type="submit" :loading="templateSaving"
          >保存</n-button
        >
      </div>
    </n-form>
  </div>
  <template v-else>
    <div class="page-actions">
      <n-button type="primary" @click="newTemplate">
        <template #icon><Plus aria-hidden="true" /></template>
        新增模板
      </n-button>
    </div>
    <ul class="template-list" aria-label="优化模板">
      <li v-for="item in libraryTemplates" :key="item.id" class="template-row">
        <div>
          <h2>
            {{ item.name }}
            <span
              v-if="isBuiltinTemplate(item.id)"
              class="builtin-template-label"
              >系统内置</span
            >
          </h2>
          <p v-if="item.description">{{ item.description }}</p>
        </div>
        <div class="template-actions">
          <n-button
            size="small"
            :disabled="defaultTemplateId === item.id"
            :aria-label="'将 ' + item.name + ' 设为默认模板'"
            @click="setDefaultTemplate(item.id)"
            >{{
              defaultTemplateId === item.id ? "默认模板" : "设为默认"
            }}</n-button
          >
          <n-button
            size="small"
            :aria-label="
              (isBuiltinTemplate(item.id) ? '查看 ' : '编辑 ') + item.name
            "
            @click="editingTemplate = { ...item }"
            >{{ isBuiltinTemplate(item.id) ? "查看" : "编辑" }}</n-button
          >
          <n-button
            size="small"
            v-if="!isBuiltinTemplate(item.id)"
            :aria-label="'复制 ' + item.name"
            @click="copyTemplate(item)"
            >复制</n-button
          >
          <n-button
            size="small"
            quaternary
            v-if="!isBuiltinTemplate(item.id)"
            :aria-label="'删除 ' + item.name"
            @click="deleteTemplate(item)"
            >删除</n-button
          >
        </div>
      </li>
    </ul>
    <p v-if="!libraryTemplates.length">
      暂无模板，创建一个模板后就能在草稿页选择。
    </p>
  </template>
  <n-modal
    :show="!!enlargedTemplateRole"
    @update:show="
      (show) => {
        if (!show) enlargedTemplateRole = undefined;
      }
    "
    preset="card"
    :title="enlargedTemplateTitle"
    to=".app-root"
    class="result-dialog"
    closable
  >
    <template #header-extra>
      <n-button
        v-if="enlargedTemplateRole"
        size="tiny"
        :secondary="templatePreviewOpen[enlargedTemplateRole]"
        :quaternary="!templatePreviewOpen[enlargedTemplateRole]"
        :aria-label="enlargedTemplateTitle + '预览'"
        :aria-pressed="templatePreviewOpen[enlargedTemplateRole]"
        @click="
          templatePreviewOpen[enlargedTemplateRole] =
            !templatePreviewOpen[enlargedTemplateRole]
        "
        >{{
          templatePreviewOpen[enlargedTemplateRole]
            ? viewingBuiltin
              ? "返回原文"
              : "返回编辑"
            : "预览"
        }}</n-button
      >

      <n-button
        quaternary
        size="small"
        :disabled="
          !enlargedTemplateRole || !templateDisplayText(enlargedTemplateRole)
        "
        :aria-label="'复制' + enlargedTemplateTitle"
        :title="'复制' + enlargedTemplateTitle"
        @click="enlargedTemplateRole && copyTemplateText(enlargedTemplateRole)"
      >
        <template #icon><Copy aria-hidden="true" /></template>
      </n-button>
    </template>
    <div
      v-if="
        enlargedTemplateRole &&
        templatePreviewOpen[enlargedTemplateRole] &&
        !templatePreviewError
      "
      class="result-preview result-content"
      tabindex="0"
      :aria-label="enlargedTemplateTitle + '预览'"
      v-html="renderMarkdown(templateDisplayText(enlargedTemplateRole))"
    ></div>
    <pre
      v-else-if="enlargedTemplateRole"
      class="result-content result-dialog-source"
      tabindex="0"
      :aria-label="enlargedTemplateTitle"
      >{{
        (templatePreviewOpen[enlargedTemplateRole] && templatePreviewError) ||
        templateDisplayText(enlargedTemplateRole)
      }}</pre>
  </n-modal>
</template>

<script setup lang="ts">
defineOptions({ name: "TemplatesPage" });
import { isBuiltinCategory, isBuiltinTemplate } from "@/config/templates";
import { useWorkspaceContext } from "@/layout/hooks/workspaceContext";
import { routeTemplateWheel } from "@/utils/workspace/templateWheel";
import { ChevronRight, Copy, Maximize2, Plus } from "lucide-vue-next";
import { NButton, NForm, NFormItem, NInput, NModal } from "naive-ui";
const {
  libraryCategory,
  libraryCategories,
  editTemplateCategory,
  deleteTemplateCategory,
  editingTemplate,
  viewingBuiltin,
  templateFields,
  templatePreviewOpen,
  templateDisplayText,
  copyTemplateText,
  enlargedTemplateRole,
  templatePreviewError,
  templatePreview,
  libraryTemplates,
  defaultTemplateId,
  setDefaultTemplate,
  newTemplate,
  saveTemplate,
  templateSaving,
  copyTemplate,
  deleteTemplate,
  enlargedTemplateTitle,
  renderMarkdown,
} = useWorkspaceContext();
</script>
