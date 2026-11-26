import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { computed, effectScope, nextTick, ref } from "vue";
import * as presets from "../src/config/modelPresets.ts";
import * as templates from "../src/config/templates.ts";
import { workspaceChanges } from "../src/store/modules/workspaceChanges.ts";
import * as shortcuts from "../src/config/shortcuts.ts";
import { duplicateDraft } from "../src/utils/workspace/duplicateDraft.ts";
import { renderTemplate } from "../src/utils/template/renderTemplate.ts";

const require = createRequire(import.meta.url);
const ts = require("typescript");
const vue = require("vue");
const messages = [];
const message = Object.fromEntries(
  ["error", "success", "warning"].map((kind) => [
    kind,
    (text) => messages.push({ kind, text }),
  ]),
);
let confirmation;
let context;
const desktop = {
  saveChanges: async (_changes, providerKey) => {
    credentialWrites.push(providerKey);
  },
  hasKey: async () => true,
  previewTemplate: async () => [],
  setKey: async () => {
    throw new Error("Saving a provider must not write its key separately");
  },
};
const credentialWrites = [];
function load(file) {
  let source = readFileSync(new URL(file, import.meta.url), "utf8");
  // Asset resolution belongs to Vite; form behavior is tested with an empty logo map.
  source = source.replace(
    /import\.meta\.glob<string>\([\s\S]*?\n  \}\)/,
    "({})",
  );
  const { outputText } = ts.transpileModule(source, {
    compilerOptions: { module: ts.ModuleKind.CommonJS },
  });
  const exports = {};
  new Function("require", "exports", outputText)((name) => {
    if (name === "vue") return vue;
    if (name === "naive-ui")
      return {
        useMessage: () => message,
        useDialog: () => ({}),
        NInput: {},
        NText: {},
        NTooltip: {},
      };
    if (name === "lucide-vue-next") return {};
    if (name === "@/components/useConfirmDialog")
      return {
        useConfirmDialog: () => (title, content, action) => {
          confirmation = { title, content, action };
        },
      };
    if (name === "@/layout/hooks/workspaceContext")
      return { useWorkspaceContext: () => context };
    if (name === "@/api/desktop")
      return { desktop, errorMessage: (error) => error.message };
    if (name === "@/config/modelPresets") return presets;
    if (name === "@/config/templates") return templates;
    if (name === "@/config/shortcuts") return shortcuts;
    if (name === "@/utils/workspace/duplicateDraft") return { duplicateDraft };
    if (name === "@/utils/template/renderTemplate") return { renderTemplate };
    if (name === "@tauri-apps/plugin-clipboard-manager")
      return { writeText: async () => {} };
    throw new Error(`Unexpected test dependency: ${name}`);
  }, exports);
  return exports;
}

const { useDraftActions } = load("../src/layout/hooks/useDraftActions.ts");
const { useProviders } = load("../src/views/models/hooks/useProviders.ts");
const { useProviderForm } = load(
  "../src/views/models/hooks/useProviderForm.ts",
);
const { useTemplates } = load("../src/views/templates/hooks/useTemplates.ts");
function fixture() {
  const data = ref({
    schema: 1,
    projects: [
      { id: "p", name: "项目" },
      { id: "other", name: "其他" },
    ],
    drafts: [
      {
        id: "d",
        projectId: "p",
        messages: [{ id: "reply", content: "保留聊天" }],
        source: "原稿",
        result: "结果",
        providerId: "provider",
        model: "deepseek-flash",
        templateId: "prompt-general",
      },
    ],
    profiles: [
      {
        id: "provider",
        name: "原供应商",
        presetId: "preset-deepseek",
        endpoint: "https://api.deepseek.com",
        model: "deepseek-flash",
        isDefault: true,
      },
    ],
    activeDraft: "d",
    theme: "system",
    templates: [],
  });
  const saveError = ref("");
  const saved = [];
  let failed = false;
  const workspace = {
    data,
    saveError,
    draft: computed(() =>
      data.value.drafts.find((item) => item.id === data.value.activeDraft),
    ),
    profile: computed(() =>
      data.value.profiles.find(
        (item) => item.id === data.value.drafts[0].providerId,
      ),
    ),
    models: computed(() => data.value.profiles),
    async save(persist) {
      if (persist) await persist({});
      saved.push({
        document: structuredClone(JSON.parse(JSON.stringify(data.value))),
        providerKey: persist ? credentialWrites.at(-1) : undefined,
      });
      saveError.value = failed ? "模拟保存失败" : "";
      return !failed;
    },
  };
  return {
    workspace,
    saved,
    fail: (value) => {
      failed = value;
    },
  };
}

for (const failed of [false, true]) {
  const scope = effectScope();
  try {
    const { workspace, saved, fail } = fixture();
    workspace.data.value.projects = workspace.data.value.projects.filter(
      (project) => project.id === "p",
    );
    fail(failed);
    const before = JSON.parse(JSON.stringify(workspace.data.value));
    const draft = workspace.draft.value;
    let cancelled = 0;
    const expanded = ref(["p"]);
    const actions = scope.run(() =>
      useDraftActions(workspace, ref(), ref(false), expanded, async () => {
        cancelled++;
      }),
    );
    actions.deleteProject("p");
    assert.ok(confirmation.content.includes("草稿内容和聊天记录都会保留"));
    const task = confirmation.action();
    if (failed) {
      await assert.rejects(task, /模拟保存失败/);
      assert.deepEqual(
        JSON.parse(JSON.stringify(workspace.data.value)),
        before,
      );
      assert.deepEqual(expanded.value, ["p"]);
    } else {
      await task;
      assert.equal(workspace.data.value.projects.length, 0);
      assert.equal(workspace.draft.value, draft);
      assert.equal(draft.projectId, null);
      assert.equal(workspace.data.value.activeDraft, "d");
      assert.deepEqual(draft.messages, before.drafts[0].messages);
      const changes = workspaceChanges(before, saved[0].document);
      assert.deepEqual(changes.drafts.remove, []);
      assert.deepEqual(changes.projects.remove, ["p"]);
    }
    assert.equal(
      cancelled,
      0,
      "project deletion keeps active generation running",
    );
  } finally {
    scope.stop();
  }
}

{
  const scope = effectScope();
  try {
    const { workspace, saved, fail } = fixture();
    const providers = scope.run(() =>
      useProviders(workspace, {
        templateList: ref([{ id: "prompt-general" }]),
        defaultTemplateId: ref("prompt-general"),
      }),
    );
    context = { ...workspace, ...providers };
    const form = scope.run(useProviderForm);
    form.editProfile(workspace.data.value.profiles[0]);
    await nextTick();
    form.profile.value.name = "修改后的名称";
    form.key.value = "fixture-input-key";
    fail(true);
    await form.run("save");
    assert.equal(form.profile.value.name, "修改后的名称");
    assert.equal(
      form.key.value,
      "fixture-input-key",
      "failed save preserves the typed key",
    );
    assert.equal(workspace.data.value.profiles[0].name, "原供应商");
    assert.deepEqual(saved.at(-1).providerKey, {
      providerId: "provider",
      endpoint: "https://api.deepseek.com",
      key: "fixture-input-key",
    });
    assert.ok(
      !JSON.stringify(saved.at(-1).document).includes("fixture-input-key"),
      "key stays outside workspace state",
    );
    fail(false);
    await form.run("save");
    assert.equal(form.profile.value, undefined);
    assert.equal(form.key.value, "");
    assert.equal(workspace.data.value.profiles[0].name, "修改后的名称");
  } finally {
    scope.stop();
  }
}

{
  const scope = effectScope();
  try {
    const { workspace, fail } = fixture();
    const api = scope.run(() => useTemplates(workspace));
    api.libraryCategory.value = "basic";
    api.newTemplate();
    const edit = api.editingTemplate.value;
    edit.name = "新模板";
    edit.content = "模板正文";
    fail(true);
    await api.saveTemplate();
    assert.equal(api.editingTemplate.value, edit);
    assert.equal(api.editingTemplate.value.content, "模板正文");
    assert.deepEqual(workspace.data.value.templates, []);
    assert.equal(api.templateSaving.value, false);
    fail(false);
    await api.saveTemplate();
    assert.equal(api.editingTemplate.value, undefined);
    assert.equal(workspace.data.value.templates[0].name, "新模板");
  } finally {
    scope.stop();
  }
}
console.log(
  "PASS project unassignment/rollback, provider save failure/retry/key clearing, and template save failure/retry",
);
