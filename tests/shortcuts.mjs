import assert from "node:assert/strict";
import {
  defaultShortcuts,
  shortcutActions,
  shortcutError,
  shortcutFromEvent,
  shortcutLabels,
} from "../src/config/shortcuts.ts";
for (const value of [
  "Ctrl+KeyN",
  "Ctrl+Alt+Digit1",
  "Alt+Shift+Enter",
  "Ctrl+F12",
  "KeyN",
  "Shift+KeyN",
  "Enter",
  "F2",
  "Shift+Enter",
  "Alt+Enter",
])
  assert.equal(shortcutError(value), "", value);
for (const value of [
  "Meta+KeyN",
  "Ctrl+KeyC",
  "Alt+F4",
  "Ctrl+Ctrl+KeyN",
  "Ctrl+F13",
])
  assert.notEqual(shortcutError(value), "", value);
assert.equal(
  shortcutFromEvent({ ctrlKey: true, code: "KeyN" }),
  defaultShortcuts.newDraft,
);
assert.notEqual(
  shortcutFromEvent({ ctrlKey: true, shiftKey: true, code: "KeyN" }),
  defaultShortcuts.newDraft,
);
assert.deepEqual(shortcutLabels("Ctrl+Shift+KeyN"), ["Ctrl", "Shift", "N"]);
assert.deepEqual(shortcutLabels("Alt+Digit1"), ["Alt", "1"]);
console.log(
  "PASS shortcut validation, reserved keys, exact modifier matching and display",
);

assert.equal(
  new Set(Object.values(defaultShortcuts)).size,
  Object.keys(defaultShortcuts).length,
);
for (const action of shortcutActions)
  assert.equal(
    shortcutError(defaultShortcuts[action.id], action.id),
    "",
    action.id,
  );

assert.equal(shortcutError("", "newlineAlternate"), "");
assert.notEqual(shortcutError("Shift"), "");
