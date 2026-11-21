import assert from "node:assert/strict";
import { buildRevisionMessage } from "../src/features/workspace/revisionMessage.ts";
const versions = [{ instruction: "加例子" }, { model: "手动编辑" }, { instruction: "面向新手" }, {}, { instruction: "旧分支" }];
const result = buildRevisionMessage("模板", "原始需求", "手动编辑后的稿", "再短一点", versions);
const evidence = JSON.parse(result.slice(result.lastIndexOf("\n") + 1));
assert.deepEqual(evidence, { originalPrompt: "原始需求", history: ["面向新手", "加例子"], currentDraft: "手动编辑后的稿", instruction: "再短一点" });
assert.throws(() => buildRevisionMessage("", "长".repeat(120000), "稿", "改", []), /上下文过长/);
assert.equal(versions.length, 5);
console.log("revision message passed");
