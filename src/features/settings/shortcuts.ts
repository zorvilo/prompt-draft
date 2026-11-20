export const defaultShortcuts = {
  newDraft: "Ctrl+KeyN",
  optimize: "Ctrl+Enter",
  rename: "Ctrl+Alt+KeyR",
  pin: "Ctrl+Alt+KeyP",
  favorite: "Ctrl+Alt+KeyF",
  delete: "Ctrl+Shift+Delete",
  copyResult: "Ctrl+Alt+KeyC",
  history: "Ctrl+Alt+KeyH",
};
export type ShortcutAction = keyof typeof defaultShortcuts;
export const shortcutActions: { id: ShortcutAction; label: string }[] = [
  { id: "newDraft", label: "新建草稿" },
  { id: "optimize", label: "优化当前提示词" },
  { id: "rename", label: "重命名" },
  { id: "pin", label: "置顶 / 取消置顶" },
  { id: "favorite", label: "收藏 / 取消收藏" },
  { id: "delete", label: "删除草稿" },
  { id: "copyResult", label: "复制草稿" },
  { id: "history", label: "版本历史" },
];

export function shortcutFromEvent(event: KeyboardEvent): string {
  return [
    event.ctrlKey && "Ctrl",
    event.altKey && "Alt",
    event.shiftKey && "Shift",
    event.metaKey && "Meta",
    event.code,
  ]
    .filter(Boolean)
    .join("+");
}

export function shortcutError(value: string): string {
  if (
    value.includes("Meta+") ||
    [
      "Alt+F4",
      "Alt+Space",
      "Ctrl+Alt+Delete",
      "Ctrl+KeyW",
      "Ctrl+KeyR",
      "Ctrl+Shift+KeyR",
      "Ctrl+KeyC",
      "Ctrl+KeyV",
      "Ctrl+KeyX",
      "Ctrl+KeyA",
      "Ctrl+KeyZ",
      "Ctrl+KeyY",
      "Ctrl+KeyF",
      "Ctrl+KeyP",
      "Ctrl+KeyS",
      "Ctrl+KeyL",
      "Ctrl+KeyT",
    ].includes(value)
  )
    return "该组合键由系统或编辑器使用，请换一个组合。";
  if (
    !/^(Ctrl\+)?(Alt\+)?(Shift\+)?(Key[A-Z]|Digit[0-9]|F([1-9]|1[0-2])|Enter|Space|Arrow(Up|Down|Left|Right)|Home|End|PageUp|PageDown|Insert|Delete|Backquote|Minus|Equal|BracketLeft|BracketRight|Backslash|Semicolon|Quote|Comma|Period|Slash)$/.test(
      value,
    ) ||
    !/^(Ctrl|Alt)\+/.test(value)
  )
    return "请使用 Ctrl 或 Alt 与普通按键的组合。";
  return "";
}

export function shortcutLabels(value: string): string[] {
  return value.split("+").map((key) =>
    key
      .replace(/^Key|^Digit/, "")
      .replace(/^Arrow/, "")
      .replace("Meta", "Win")
      .replace("Space", "空格"),
  );
}
