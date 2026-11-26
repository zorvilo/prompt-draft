import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import assert from "node:assert/strict";
const require = createRequire(import.meta.url);
const ts = require("typescript");
const { outputText } = ts.transpileModule(
  readFileSync(
    new URL("../src/layout/hooks/useAppUpdate.ts", import.meta.url),
    "utf8",
  ),
  { compilerOptions: { module: ts.ModuleKind.CommonJS } },
);
let result = { version: "0.1.4", downloaded: false };
let fail = false;
let calls = 0;
const desktop = {
  async checkUpdate() {
    calls++;
    if (fail) throw new Error("offline");
    return result;
  },
};
const exports = {};
new Function("require", "exports", outputText)(
  (name) =>
    name === "vue"
      ? require("vue")
      : { desktop, errorMessage: (error) => error.message },
  exports,
);
const updater = exports.useAppUpdate({
  getSource: () => undefined,
  setSource: async () => {},
  prepareUpdate: async () => {},
});
await updater.checkUpdate(true);
assert.equal(updater.updateInfo.value.version, "0.1.4");
assert.equal(updater.showUpdateDialog.value, false);
await updater.checkUpdate();
assert.equal(updater.showUpdateDialog.value, true);
updater.showUpdateDialog.value = false;
assert.ok(updater.updateInfo.value, "postponing preserves the sidebar badge");
fail = true;
await updater.checkUpdate(true);
assert.equal(updater.showUpdateDialog.value, false);
assert.ok(
  updater.updateInfo.value,
  "a failed recheck preserves the known update",
);
fail = false;
result = null;
await updater.checkUpdate();
assert.equal(updater.updateInfo.value, null);
assert.equal(updater.showUpdateDialog.value, false);
assert.equal(calls, 4);
console.log(
  "PASS silent discovery, user-triggered dialog, postponed badge, failure and no-update states",
);
