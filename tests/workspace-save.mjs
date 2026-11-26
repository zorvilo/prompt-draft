import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { readFileSync } from "node:fs";
import * as presets from "../src/config/modelPresets.ts";
import { workspaceChanges } from "../src/store/modules/workspaceChanges.ts";

const require = createRequire(import.meta.url);
const ts = require("typescript");
const { createPinia } = require("pinia");
const initial = {
  schema: 1,
  projects: [{ id: "p", name: "项目" }],
  drafts: [],
  profiles: [],
  activeDraft: null,
  activeProfile: null,
  theme: "system",
};
const calls = [];
let failFirst;
const failed = new Promise((_, reject) => {
  failFirst = () => reject(new Error("保存失败"));
});
const desktop = {
  load: async () => JSON.stringify(initial),
  async saveChanges(changes, providerKey) {
    calls.push({ changes, providerKey });
    if (calls.length === 1) await failed;
  },
};
const { outputText } = ts.transpileModule(
  readFileSync(
    new URL("../src/store/modules/workspace.ts", import.meta.url),
    "utf8",
  ),
  { compilerOptions: { module: ts.ModuleKind.CommonJS } },
);
const exports = {};
new Function("require", "exports", outputText)((name) => {
  if (name.includes("api/desktop"))
    return { desktop, errorMessage: (error) => error.message };
  if (name.includes("modelPresets")) return presets;
  if (name.includes("workspaceChanges")) return { workspaceChanges };
  return require(name);
}, exports);
const store = exports.useWorkspaceStore(createPinia());
const actionArgs = [];
store.$onAction(({ args }) => actionArgs.push(args));
try {
  await store.initialize();
  const providerKey = {
    providerId: "p",
    endpoint: "https://example.com",
    key: "fixture-key",
  };
  const persist = (changes) => desktop.saveChanges(changes, providerKey);
  store.data.theme = "dark";
  const first = store.save(persist);
  store.data.theme = "light";
  const second = store.save();
  // The queued save must still run after the preceding write fails.
  await Promise.resolve();
  failFirst();
  assert.equal(await first, false);
  assert.equal(await second, true);
  assert.equal(store.saveError, "");
  assert.equal(calls.length, 2);
  assert.deepEqual(calls[0].providerKey, providerKey);
  assert.equal(calls[1].providerKey, undefined);
  assert.equal(calls[1].changes.settings.theme, "light");
  assert.ok(!JSON.stringify(store.data).includes("fixture-key"));
  assert.equal(
    await store.save(persist),
    true,
    "key-only edits are persisted even without a workspace diff",
  );
  assert.equal(calls.length, 3);
  assert.ok(
    !JSON.stringify(actionArgs).includes("fixture-key"),
    "typed keys stay out of Pinia action records",
  );
  assert.equal(await store.save(), true);
  assert.equal(calls.length, 3, "unchanged workspace writes are still skipped");
} finally {
  store.ready = false;
  await store.save();
  store.$dispose();
}
console.log(
  "PASS independent queued save outcomes and transient, key-only credential updates",
);
