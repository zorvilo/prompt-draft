# 前端目录与组件约定

目录分层参考 [Smart Admin Vue](https://gitee.com/liuliang6666/smart-admin/tree/master/smart-admin-vue)，保留本应用的 Vue 3、TypeScript、Composition API、Naive UI 与 Tauri 技术栈。

```text
src/
  api/                 # Tauri IPC、请求/响应类型与错误转换
  assets/              # 图片与品牌资源
  components/          # 多个业务页面共用的组件和交互工具
  config/              # 供应商预设、快捷键、内置模板
  layout/
    index.vue          # 标题栏、侧栏、路由出口、全局弹窗
    components/        # 仅布局使用的组件
    hooks/             # 路由协调、更新、窗口生命周期、草稿菜单
  router/
    index.ts           # Hash 路由实例
    routes.ts          # 路由、页面标题与菜单归属
  store/
    index.ts           # Pinia 实例
    modules/           # 工作区状态、差异保存与恢复
  style/
    index.css          # 按顺序导入全局样式，保持原有层叠顺序
  utils/               # 不依赖页面实例的计算与转换
  views/
    workspace/         # 提示词编辑与连续对话
    projects/          # 项目管理
    templates/         # 模板分类、编辑与预览
    models/            # 供应商列表、预设与配置
    usage/             # 用量统计、价格目录
    settings/          # 常规设置、备份与关于
```

## 页面与组件

- 每个路由页面以业务目录下的 `index.vue` 为入口，路由集中声明在 `router/routes.ts`。
- 只属于一个页面的组件放在该页面的 `components` 下，采用 PascalCase 文件名；页面组合子组件，不在同一页面中用分支容纳其他业务页面。
- 页面交互逻辑放在本业务的 `hooks/useXxx.ts`，保持明确的输入和返回值。纯计算放 `utils`，固定配置放 `config`。
- 公共组件放 `components`，布局专用组件放 `layout/components`。只有实际复用时才上移，不建立空目录或占位模块。
- Vue 文件依次编排 `<template>`、`<script setup lang="ts">` 和 `<style>`，与参考项目保持一致；公共跨层引用使用 `@/`，同目录内部可使用相对路径。
- 不以固定行数机械拆分。列表、表单、聊天记录、备份和关于等独立职责分别维护。

## 状态与生命周期

工作区数据由 `useWorkspaceStore` 统一管理，`useWorkspace` 提供保持 Ref 语义的视图接口。页面不会重复创建工作区状态或自行读写数据库。

布局在切换路由时保持挂载。生成请求、后台更新、窗口事件与快捷键由布局调用对应业务 hook，并通过带类型的工作区上下文提供给页面。请求按草稿和请求 ID 隔离，页面卸载不取消后台生成；退出、删除草稿和安装更新继续走原有校验。

路由使用 Vue Router Hash 模式，页面静态导入，不启用按需加载。启动仍先隐藏主窗口，工作区就绪并渲染后显示。Pinia 不接入浏览器持久化插件；持久化仍通过 Rust 写入 SQLite，已保存的 API Key 不进入 Pinia 状态。

## 样式与接口

全局主题和布局样式由 `style/index.css` 按既定顺序导入。页面已有的局部样式保留 scoped；拆分样式时不能改变选择器、主题变量及媒体查询的优先顺序。

桌面能力集中在 `api/desktop.ts`，页面与 hook 不直接写入文件或数据库。IPC 字段与 Rust 同步维护，禁止绕过密钥、安全确认和数据保存边界。

## 验证

执行 `npm run check` 检查 ESLint、类型、生产构建与 Rust；执行 `npm run test:frontend` 验证前端行为。涉及布局时还需检查常规/窄窗口、浅色/深色、输入法、焦点和空白/处理中/失败/成功状态。不得将模拟 IPC 的界面检查表述为真实安装包验收。
