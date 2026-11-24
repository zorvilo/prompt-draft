import assert from "node:assert/strict";
import { renderTemplate } from "../src/features/library/renderTemplate.ts";
import {
  templates,
  availableTemplates,
  builtinCategories,
  isBuiltinCategory,
  isBuiltinTemplate,
} from "../src/features/library/templates.ts";
const workspace = () => ({
  schema: 1,
  projects: [],
  drafts: [],
  profiles: [],
  theme: "system",
  activeDraft: null,
  activeProfile: null,
});
const fresh = workspace();
const draftContent = '包含 "引号"、换行\n和 {{待保留变量}}';
assert.equal(renderTemplate("{{{draftContent}}}", draftContent), draftContent);
assert.equal(
  JSON.parse(renderTemplate('{"draftContent": {{#helpers.toJson}}{{{draftContent}}}{{/helpers.toJson}}}', draftContent)).draftContent,
  draftContent,
);
assert.equal(availableTemplates(fresh).length, 3);
assert.deepEqual(
  builtinCategories.map((item) => item.value),
  ["basic"],
);
assert.ok(
  templates.every(
    (item) => item.content && item.userContent && isBuiltinTemplate(item.id),
  ),
);
assert.throws(() => {
  templates[0].content = "override";
}, TypeError);
const custom = { ...templates[0], id: "custom", name: "我的模板" };
const user = {
  ...workspace(),
  templates: [custom],
  templateCategories: [{ id: "mine", name: "我的分类" }],
};
assert.equal(availableTemplates(user).length, 4);
assert.equal(availableTemplates(user).at(-1), custom);
const collision = {
  ...workspace(),
  templates: [{ ...templates[0], content: "伪造覆盖" }],
};
assert.equal(availableTemplates(collision).length, 3);
assert.equal(availableTemplates(collision)[0].content, templates[0].content);
assert.ok(isBuiltinCategory("basic") && !isBuiltinCategory("mine"));
console.log(
  "PASS code-defined read-only defaults, custom templates and collision protection",
);
