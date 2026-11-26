import assert from "node:assert/strict";
import { createServer } from "vite";
import { createSSRApp, h } from "vue";
import { renderToString } from "vue/server-renderer";
import { createPinia } from "pinia";
import { createMemoryHistory, createRouter } from "vue-router";
import { NConfigProvider, NDialogProvider, NMessageProvider } from "naive-ui";
import { setup } from "@css-render/vue3-ssr";

// SSR mounts no native window handlers and never touches the user's database.
globalThis.window = {
  __TAURI_INTERNALS__: { metadata: { currentWindow: { label: "main" } } },
};
const server = await createServer({
  server: { middlewareMode: true, hmr: false },
  appType: "custom",
});
try {
  const { routes } = await server.ssrLoadModule("/src/router/routes.ts");
  const { default: Layout } = await server.ssrLoadModule(
    "/src/layout/index.vue",
  );
  const { useWorkspaceStore } = await server.ssrLoadModule(
    "/src/store/modules/workspace.ts",
  );
  const { defaultShortcuts } = await server.ssrLoadModule(
    "/src/config/shortcuts.ts",
  );
  const expected = [
    ["/", "原稿"],
    ["/projects", "新建项目"],
    ["/templates", "添加模板分类"],
    ["/models", "添加供应商"],
    ["/usage", "价格目录"],
    ["/settings", "主题样式"],
  ];
  for (const [path, text] of expected) {
    const pinia = createPinia();
    const store = useWorkspaceStore(pinia);
    store.newDraft(null, "路由切换后保留的原稿", "测试草稿");
    const id = store.data.activeDraft;
    const router = createRouter({ history: createMemoryHistory(), routes });
    await router.push(path);
    await router.isReady();
    const render = async () => {
      const app = createSSRApp({
        render: () =>
          h(NConfigProvider, null, {
            default: () =>
              h(NDialogProvider, null, {
                default: () =>
                  h(NMessageProvider, null, {
                    default: () => h(Layout),
                  }),
              }),
          }),
      });
      app.use(pinia).use(router);
      setup(app);
      return renderToString(app);
    };
    const html = await render();
    assert.ok(html.includes(text), `${path} renders its own page`);
    if (path === "/projects") {
      assert.equal(store.data.projects.length, 1);
      assert.ok(
        html.includes("删除项目 我的草稿"),
        "last project keeps its delete button",
      );
    }
    assert.ok(html.includes("测试草稿"), `${path} keeps the shared sidebar`);
    if (path === "/") {
      store.data.shortcuts = {
        ...defaultShortcuts,
        newline: "Ctrl+Enter",
        newlineAlternate: "",
      };
      store.draft.dialogue = { mode: "chat" };
      const chat = await render();
      assert.ok(
        chat.includes("Ctrl+Enter 换行"),
        "saved newline binding reaches the editor",
      );
      assert.ok(
        !chat.includes("Alt+Enter"),
        "disabled alternate newline is not restored by fallback",
      );
    }
    await router.push("/");
    assert.equal(store.data.activeDraft, id);
    assert.equal(store.draft.source, "路由切换后保留的原稿");
    store.$dispose();
  }
  console.log(
    "PASS six routes render with shared layout and preserve draft state",
  );
} finally {
  await server.close();
  delete globalThis.window;
}
