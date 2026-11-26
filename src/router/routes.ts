import Models from "@/views/models/index.vue";
import Projects from "@/views/projects/index.vue";
import Settings from "@/views/settings/index.vue";
import Templates from "@/views/templates/index.vue";
import Usage from "@/views/usage/index.vue";
import Workspace from "@/views/workspace/index.vue";
import type { RouteRecordRaw } from "vue-router";

export type Utility = "projects" | "templates" | "model" | "usage" | "settings";

export const routes: RouteRecordRaw[] = [
  { path: "/", name: "workspace", component: Workspace },
  {
    path: "/projects",
    name: "projects",
    component: Projects,
    meta: { utility: "projects", title: "项目" },
  },
  {
    path: "/templates",
    name: "templates",
    component: Templates,
    meta: { utility: "templates", title: "模板" },
  },
  {
    path: "/models",
    name: "model",
    component: Models,
    meta: { utility: "model", title: "模型" },
  },
  {
    path: "/usage",
    name: "usage",
    component: Usage,
    meta: { utility: "usage", title: "用量" },
  },
  {
    path: "/settings",
    name: "settings",
    component: Settings,
    meta: { utility: "settings", title: "设置" },
  },
  { path: "/:pathMatch(.*)*", redirect: "/" },
];
