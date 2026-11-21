import assert from "node:assert/strict";
import { draftCandidates, selectCandidate } from "../src/features/workspace/draftCandidates.ts";

const version = (id, result, extra = {}) => ({ id, source: "需求", result, question: null, model: "模型", created: 1, ...extra });
const first = version("a", "第一版");
const second = version("b", "第二版");
const draft = { source: "需求", result: "手动修改", question: null, versions: [version("c", "迭代稿", { instruction: "精简" }), second, version("x", "其他需求", { source: "其他" }), first] };
assert.deepEqual(draftCandidates(draft).map(v => v.id), ["b", "a"]);
selectCandidate(draft, first);
assert.equal(draft.result, "第一版");
assert.equal(draft.versions[0].result, "手动修改");
assert.equal(draft.versions[0].model, "手动编辑");
assert.deepEqual(draftCandidates(draft).map(v => v.id), ["b", "a"]);
const count = draft.versions.length;
selectCandidate(draft, second);
assert.equal(draft.versions.length, count);
assert.equal(draft.source, "需求");
assert.equal(draft.result, "第二版");
console.log("draft candidates passed");
