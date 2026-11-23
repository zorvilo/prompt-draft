import type { PromptTemplate, Workspace } from "../../lib/tauri";
import builtinData from "./upstream/builtin-templates.json" with { type: "json" };

export const builtinCategories = [
  {
    label: "基础",
    value: "basic",
    description: "优化原稿，理顺表达与任务要求。",
  },
] as const;
export const templates: readonly PromptTemplate[] = Object.freeze(
  (builtinData as PromptTemplate[]).map((item) => Object.freeze(item)),
);
export const isBuiltinCategory = (id: string) =>
  builtinCategories.some((item) => item.value === id);
export const isBuiltinTemplate = (id: string) =>
  templates.some((item) => item.id === id);

export function availableTemplates(
  workspace: Workspace,
): readonly PromptTemplate[] {
  return [
    ...templates,
    ...(workspace.templates ?? []).filter(
      (item) => !isBuiltinTemplate(item.id),
    ),
  ];
}
