import Mustache from "mustache";

export function renderTemplate(content: string, draftContent: string) {
  // Plain templates can contain variables intended for the model's output.
  if (
    !/\{\{\s*(?:=|[{#^&]?\s*(?:draftContent|helpers\.toJson)\s*\}\})/.test(
      content,
    )
  ) {
    return content;
  }

  try {
    return Mustache.render(content, {
      draftContent,
      helpers: {
        toJson: () => (text: string, render: (text: string) => string) =>
          JSON.stringify(render(text)),
      },
    });
  } catch {
    throw new Error("模板的 Mustache 语法有误，请在模板页检查变量和分隔符。");
  }
}
