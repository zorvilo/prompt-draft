# Prompt Optimizer 小步迭代模板

`context-iterate.json` contains the two original prompt strings from [Prompt Optimizer, commit cbba5f9](https://github.com/linshenkx/prompt-optimizer/blob/cbba5f959f26a4e7846e5458664564b265def8b3/packages/core/src/services/template/default-templates/iterate/context/context-iterate.ts).

Copyright (C) 2025 linshenkx. The upstream template is distributed under the [GNU Affero General Public License v3.0 only](./LICENSE). Its original template strings are unchanged. The JSON file adapts the upstream TypeScript container to the format used by Prompt Draft.

## 内置模板目录

`builtin-templates.json` 是词稿的七个系统模板，按基础、上下文、迭代三类组织。内容从当前已有模板原样整理，包含此前参考 Prompt Optimizer 引入的模板及改写内容；系统提示词和用户提示词同时随源码发布。参考项目模板目录见 [default-templates](https://github.com/linshenkx/prompt-optimizer/tree/master/packages/core/src/services/template/default-templates)。沿用本目录的上游版权与 AGPL-3.0-only 许可说明。

应用从源码读取系统模板，不把系统正文作为用户模板写入 SQLite。用户分类、模板仍随工作区保存和备份。
