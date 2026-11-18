import Mustache from "mustache";
import type { Revision } from "../../lib/tauri";

export function renderTemplate(
  content: string,
  originalPrompt: string,
  revision?: Revision,
) {
  // Plain templates can contain variables intended for the model's output.
  if (
    !/\{\{\s*(?:=|[{#^&]?\s*(?:originalPrompt|previousResult|lastOptimizedPrompt|iterateInput|helpers\.toJson)\s*\}\})/.test(
      content,
    )
  ) {
    return content;
  }

  try {
    return Mustache.render(content, {
      originalPrompt,
      previousResult: revision?.previousResult,
      lastOptimizedPrompt: revision?.previousResult,
      iterateInput: revision?.instruction,
      helpers: {
        toJson: () => (text: string, render: (text: string) => string) =>
          JSON.stringify(render(text)),
      },
    });
  } catch {
    throw new Error("模板的 Mustache 语法有误，请在模板页检查变量和分隔符。");
  }
}
