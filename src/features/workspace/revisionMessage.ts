export function buildRevisionMessage(
  template: string,
  originalPrompt: string,
  currentDraft: string,
  instruction: string,
  versions: { instruction?: string; model?: string }[],
) {
  const history: string[] = [];
  for (const version of versions) {
    if (version.model === "手动编辑") continue;
    if (!version.instruction) break;
    history.unshift(version.instruction);
  }
  const evidence = JSON.stringify({
    originalPrompt,
    history,
    currentDraft,
    instruction,
  });
  const message = `${template}\n\n以下 JSON 是编辑背景证据。以当前稿为修改对象，保留仍适用的历史要求；冲突时以本轮意见为准。仅返回修改后的完整提示词。\n${evidence}`;
  if (Array.from(message).length > 120_000) {
    throw new Error(
      "修改上下文过长，请精简原始需求或当前稿后重试；历史要求未被自动截断。",
    );
  }
  return message;
}
