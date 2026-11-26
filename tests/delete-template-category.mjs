import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const source = readFileSync(new URL("../src/views/templates/hooks/useTemplates.ts", import.meta.url), "utf8");
const handler = source.split("function deleteTemplateCategory(")[1];
const body = handler.split("async () => {")[1].split(/\n\s*},\n\s*"删除",/)[0];
const AsyncFunction = Object.getPrototypeOf(async function () {}).constructor;
const remove = new AsyncFunction("data", "templateList", "category", "save", "saveError", "message", body);
for (const fails of [false, true]) {
  for (const defaultId of ["mine", "other"]) {
    const templates = [{ id: "mine", category: "custom" }, { id: "other", category: "another" }];
    const categories = [{ id: "custom" }, { id: "another" }];
    const data = { value: { templates, templateCategories: categories, defaultTemplateId: defaultId } };
    const task = remove(data, { value: templates }, { value: "custom" }, async () => {}, { value: fails ? "保存失败" : null }, { success() {} });
    if (fails) {
      await assert.rejects(task, /保存失败/);
      assert.equal(data.value.templates, templates);
      assert.equal(data.value.templateCategories, categories);
      assert.equal(data.value.defaultTemplateId, defaultId);
    } else {
      await task;
      assert.deepEqual(data.value.templates.map(item => item.id), ["other"]);
      assert.deepEqual(data.value.templateCategories.map(item => item.id), ["another"]);
      assert.equal(data.value.defaultTemplateId, defaultId === "mine" ? "prompt-general" : "other");
    }
  }
}
console.log("PASS category deletion and default template rollback");
