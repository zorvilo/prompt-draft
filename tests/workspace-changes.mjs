import assert from "node:assert/strict";
import { workspaceChanges } from "../src/features/workspace/workspaceChanges.ts";
const initial = {
  schema: 1,
  projects: [{ id: "p", name: "项目" }],
  drafts: [
    {
      id: "d",
      projectId: null,
      title: "草稿",
      source: "原稿",
      result: "",

      favorite: false,
      opened: 1,
      messages: [],
    },
  ],
  profiles: [],
  activeDraft: "d",
  activeProfile: null,
  theme: "system",
};
const fresh = workspaceChanges(undefined, initial);
assert.equal(fresh.projects.upsert.length, 1);
assert.equal(fresh.drafts.upsert.length, 1);
assert.ok(fresh.settings);
assert.deepEqual(
  JSON.parse(
    JSON.stringify(workspaceChanges(initial, structuredClone(initial))),
  ),
  {},
);
const edited = structuredClone(initial);
edited.drafts[0].source = "修改正文";
const draftPatch = workspaceChanges(initial, edited);
assert.equal(draftPatch.drafts.upsert.length, 1);
assert.equal(draftPatch.settings, undefined);
assert.equal(draftPatch.projects, undefined);
const settings = structuredClone(initial);
settings.theme = "dark";
assert.equal(workspaceChanges(initial, settings).drafts, undefined);
assert.equal(workspaceChanges(initial, settings).settings.theme, "dark");
const deleted = structuredClone(initial);
deleted.drafts = [];
deleted.activeDraft = null;
assert.deepEqual(workspaceChanges(initial, deleted).drafts.remove, ["d"]);
assert.equal(workspaceChanges(initial, deleted).settings.activeDraft, null);
console.log(
  "PASS initial persistence, entity-only patches, unchanged rows, settings and deletion.",
);
