<template>
  <div class="preferences-body">
    <section class="preference-group" aria-labelledby="workspace-heading">
      <h2 id="workspace-heading">本机数据</h2>
      <div class="preference-card">
        <div class="preference-row">
          <h3>工作区概览</h3>
          <span class="workspace-summary"
            >{{ data.projects.length }} 个项目<span aria-hidden="true"> / </span
            >{{ data.drafts.length }} 份草稿<span aria-hidden="true"> / </span
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
          <n-button size="small" @click="backup()"
            ><template #icon><Download aria-hidden="true" /></template
            >导出备份</n-button
          >
        </div>
        <div class="preference-row">
          <div>
            <h3>从备份恢复</h3>
            <p>覆盖当前全部数据和配置，校验通过后会再次确认。</p>
          </div>
          <n-button size="small" @click="restore()"
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
          <p class="privacy-note">发送前请确认供应商，避免提交敏感信息。</p>
        </section>
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import { Download, Info, Monitor, Plug, Upload } from "lucide-vue-next";
import { NButton } from "naive-ui";

import { useWorkspaceContext } from "@/layout/hooks/workspaceContext";
const { data, backup, restore } = useWorkspaceContext();
</script>
