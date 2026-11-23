# 词稿

词稿是一个开源 Windows 桌面提示词编辑器。它通过你配置的 AI 模型改写原稿，并在本机保留稿件与聊天记录。

## 使用

在左侧底部打开「模型」，点击「添加供应商」，选择国内供应商预设并填写对应平台的 API Key。点击「获取模型」从平台同步目录。DeepSeek 和智谱 GLM 初始仅提供默认模型，获取成功后可选择目录里的其他型号。其余预设也支持在默认模型下拉框手动输入模型 ID 后按 Enter。测试连接后保存配置。服务地址由应用预设。每个供应商可以保存多份独立命名的配置，并为每份配置选择默认模型。

按 Ctrl+N 新建草稿，在工作台选择优化模板及底部的供应商和模型，按 Ctrl+Enter 优化。系统内置基础优化、专业优化和步骤化规划三个模板，只读查看；用户可以添加分类和模板，编辑、复制或删除自己的模板。可选择模板优化或连续对话，无需单独选择迭代模板。两种模式的回复都会更新右侧优化稿并保存到聊天记录。每次双框优化也记入聊天，重新优化会开始新的上下文，旧消息保留供回看和复制。

草稿支持项目归属、置顶、收藏、搜索和聊天记录，优化稿支持预览、原文、对比、复制、导出及放大查看。用量页提供时间范围筛选、供应商与模型统计、ECharts 趋势图、Token 分项及人民币预估成本；价格目录支持离线内置价格、更新已有价格及从 models.dev 导入。

稿件、聊天记录、设置和供应商配置保存在本机 SQLite 数据库，API Key 保存在供应商记录中，仅由 Rust 读取。模型请求发送到用户选择的供应商地址。完整 JSON 备份包含全部 12 张表：项目、稿件、聊天消息、对话输入状态、用户模板及分类、供应商（含明文 API Key）、应用设置、用量记录、价格、汇率设置及目录缓存。设置 → 数据与备份中导出，再在同版本新电脑上恢复；导入覆盖全部数据，不合并，校验失败会回滚。备份文件含敏感信息，请妥善保管；开机自启属于系统注册状态，需在新电脑重新开启。旧工作区备份不支持导入。

数据库路径为 `%APPDATA%\cn.promptdraft.desktop\prompt-draft.sqlite`。当前采用 11 张业务表与 1 张缓存表，日常保存通过增量接口更新变化记录。项目处于测试阶段，仅维护当前数据结构，不提供历史数据库迁移和旧版兼容；修改结构后需重建测试库。

## 开发

在 Windows 安装 Node.js、Rust、Visual Studio C++ 构建工具和 WebView2。详见 [Tauri 环境要求](https://v2.tauri.app/start/prerequisites/)。

```powershell
npm ci
npm run tauri dev
npm run check
node tests/model-presets.mjs
node tests/templates.mjs
node tests/shortcuts.mjs
node tests/workspace-changes.mjs
node tests/hover-scroll-text.mjs
```

运行 `npm run tauri build` 生成 NSIS 安装包。产品与技术设计见 [docs/design.md](docs/design.md)。编码规范见 [AGENTS.md](AGENTS.md)。

## 许可证

项目自有代码采用 [MIT](LICENSE)。第三方资产保留各自的版权声明和许可证。

供应商品牌 SVG 取自 [Lobe Icons](https://github.com/lobehub/lobe-icons)，许可证见 [src/assets/providers/LICENSE](src/assets/providers/LICENSE)。


草稿页内置基础优化、专业优化和步骤化规划三种方式。双框优化只发送左侧原稿和模板，成功后更新右侧正文并追加一轮聊天记录。连续对话延续最近一次独立优化之后的消息，回复成功后更新右侧稿件。重新优化保留旧记录并开启新上下文；聊天及模板、模型快照保存在本机，重启可继续。
