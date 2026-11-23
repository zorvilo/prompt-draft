import assert from "node:assert/strict";
import { chatNotices } from "../src/features/workspace/chatNotices.ts";

const old = { providerName: "供应商 A", model: "model-a" };
assert.deepEqual(chatNotices(undefined, { mode: "chat", ...old }), []);
assert.deepEqual(chatNotices(old, { mode: "chat", ...old }), []);
assert.deepEqual(
  chatNotices(old, { mode: "draft", ...old }).map((n) => n.kind),
  ["optimization"],
);
const changed = chatNotices(old, {
  mode: "chat",
  providerName: "供应商 B",
  model: "model-b",
});
assert.equal(changed.length, 1);
assert.equal(changed[0].text, "已切换至 供应商 B · model-b");
assert.ok(changed[0].detail.includes("供应商 A"));
assert.ok(changed[0].detail.includes("model-a"));
const optimized = chatNotices(old, {
  mode: "draft",
  providerName: "供应商 B",
  model: "model-b",
});
assert.equal(optimized.length, 1);
assert.equal(optimized[0].kind, "optimization");
assert.ok(optimized[0].detail.includes("后续消息仅携带从本轮开始"));
assert.equal(
  chatNotices(old, { ...old, model: "model-b" })[0].text,
  "模型已从 model-a 更改为 model-b",
);
assert.deepEqual(
  chatNotices({ model: undefined }, { mode: "chat", model: "model-b" }),
  [],
);

const templateChange = chatNotices(
  { ...old, context: { templateId: "basic", templateName: "基础优化" } },
  {
    ...old,
    mode: "chat",
    templateId: "professional",
    templateName: "专业优化",
  },
);
assert.equal(templateChange.length, 1);
assert.equal(templateChange[0].kind, "template");
assert.equal(templateChange[0].text, "模板已从 基础优化 更改为 专业优化");
assert.equal(
  chatNotices(
    { ...old, context: { templateId: "basic" } },
    { ...old, templateId: "basic" },
  ).length,
  0,
);
