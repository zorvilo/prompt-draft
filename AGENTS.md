# 词稿贡献规范

产品范围、模块职责和界面设计见 [设计方案](docs/design.md)。

- Vue 使用 TypeScript 严格模式、Composition API 和 `<script setup lang="ts">`。组件用 PascalCase，TS 函数用 camelCase，Rust 函数用 snake_case。
- 页面通过 `src/lib/tauri.ts` 调用桌面命令。Tauri 命令负责输入校验和错误转换，改写逻辑与 HTTP 客户端保持独立。
- Rust 与 Vue 共享明确的请求、响应和错误字段约定。更新 IPC 字段时同步修改两端类型。
- Vue 基础组件使用 Naive UI，界面功能图标统一用 Lucide SVG 组件。禁止把 Emoji、Unicode 表情或文本符号用作图标。纯图标按钮提供可访问名称，带文字的按钮把装饰图标对辅助技术隐藏。
- API Key 与供应商配置一起保存在 SQLite 的 `providers.api_key` 字段，仅由 Rust 读取；前端查询和工作区导出排除密钥。输入框提交成功后清空。禁止把密钥写入工作区备份、日志和报错正文。提示词、草稿和版本只保存在本机。清空或永久删除数据前明确展示影响范围。
- 使用 ESLint、Prettier、`vue-tsc --noEmit`、`cargo fmt --check`、`cargo clippy` 和 `cargo test`。初始化项目时把检查加入项目脚本。改动改写契约或错误处理时加入对应回归用例。
- 提交应用的前端锁文件和 Cargo.lock。优先按功能拆分文件，公共逻辑有实际复用时再抽取。

- 模型请求、文件操作和数据库读写由 Rust 处理。前端按工作台、资料库和设置拆分。测试阶段只维护当前数据库结构，不编写迁移或旧版兼容代码；结构调整后重建测试数据库。
- 合并页面变更前检查常规和窄窗口、浅色和深色、中文输入法、可见焦点，以及空白、处理中、失败和成功四类状态。
