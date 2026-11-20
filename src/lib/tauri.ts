import { Channel, invoke } from "@tauri-apps/api/core";

export interface Profile {
  id: string;
  name: string;
  endpoint: string;
  presetId?: string;
  website?: string;
  model: string;
  catalog?: string[];
}
export interface Project {
  id: string;
  name: string;
}
export interface Version {
  id: string;
  source: string;
  result: string;
  question: string | null;
  model: string;
  providerName?: string;
  created: number;
  instruction?: string;
}
export interface Draft {
  id: string;
  projectId: string | null;
  title: string;
  source: string;
  result: string;
  question: string | null;
  favorite: boolean;
  pinned?: boolean;
  opened: number;
  updated?: number;
  versions: Version[];
}
export interface Workspace {
  schema: 1;
  uiState?: {
    recentExpanded: boolean;
    pinnedExpanded: boolean;
    expandedProjects: string[];
    draftListMode: "recent" | "favorites";
  };
  closeBehavior?: "ask" | "hide" | "exit";
  shortcuts?: Partial<
    typeof import("../features/settings/shortcuts").defaultShortcuts
  >;
  templates?: PromptTemplate[];
  templateCategories?: Project[];
  projects: Project[];
  drafts: Draft[];
  profiles: Profile[];
  activeProfile: string | null;
  activeDraft: string | null;
  theme: "system" | "light" | "dark";
}
type EntityChanges<T> = { upsert: T[]; remove: string[] };
export type WorkspaceSettings = Pick<
  Workspace,
  | "theme"
  | "closeBehavior"
  | "shortcuts"
  | "activeDraft"
  | "activeProfile"
  | "uiState"
>;
export type WorkspaceChanges = {
  projects?: EntityChanges<Project>;
  drafts?: EntityChanges<Draft>;
  profiles?: EntityChanges<Profile>;
  templates?: EntityChanges<PromptTemplate>;
  templateCategories?: EntityChanges<Project>;
  settings?: WorkspaceSettings;
};
export type OptimizationTemplate =
  "general" | "outputFormat" | "planning" | "iterate";
export type TemplateCategory = string;
export interface PromptTemplate {
  id: string;
  strategy: OptimizationTemplate;
  category?: TemplateCategory;
  name: string;
  content: string;
  userContent?: string;
  description: string;
}
export interface Revision {
  previousResult: string;
  instruction: string;
}
export type UsageCounts = {
  freshInput: number | null;
  cacheRead: number | null;
  cacheWrite: number | null;
  count: number;
  success: number;
  input: number | null;
  output: number | null;
  reported: number;
  cost: number | null;
  priced: number;
};
export type UsageRow = {
  freshInput: number | null;
  id: number;
  created: number;
  provider: string;
  model: string;
  kind: string;
  outcome: string;
  elapsed: number;
  cost: number | null;
  pricingModel: string;
  cacheRead: number | null;
  cacheWrite: number | null;
  input: number | null;
  output: number | null;
};
type GroupRow = UsageCounts & { id: string; label: string };
export type UsageStats = {
  summary: UsageCounts;
  trend: (UsageCounts & { date: string })[];
  rows: (UsageRow | GroupRow)[];
  total: number;
};
export type UsageCatalogSettings = {
  rate: string | null;
  syncedAt: number | null;
  rateDate: string | null;
  cachedRate?: boolean;
  lastError?: string | null;
  synced?: number;
};
export type CatalogPrice = {
  host: string;
  model: string;
  input: string;
  output: string;
  cacheRead: string;
  cacheWrite: string;
  source: string;
};
export type ModelsDevEntry = {
  key: string;
  provider: string;
  providerName: string;
  model: string;
  name: string;
  date: string;
  input: number;
  output: number;
  cacheRead: number;
  cacheWrite: number;
};
export const desktop = {
  modelsDevPicker: (refresh = false) =>
    invoke<{
      entries: ModelsDevEntry[];
    }>("models_dev_picker", { refresh }),
  importModelsDevPrice: (key: string) =>
    invoke<void>("import_models_dev_price", { key }),
  usageCatalogRows: () => invoke<CatalogPrice[]>("usage_catalog_rows"),
  saveUsageCatalogPrice: (price: CatalogPrice, remove = false) =>
    invoke<void>("save_usage_catalog_price", { price, remove }),
  usageCatalogSettings: () =>
    invoke<UsageCatalogSettings>("usage_catalog_settings"),
  updateUsageCatalog: (sync: boolean, rate?: string) =>
    invoke<UsageCatalogSettings>("update_usage_catalog", { sync, rate }),
  usageStats: (query: {
    hourly?: boolean;
    until?: number;
    since: number;
    page: number;
    group: string;
    filter: string | null;
  }) => invoke<UsageStats>("usage_stats", query),
  autostart: (enabled?: boolean) => invoke<boolean>("autostart", { enabled }),
  windowAction: (action: "hide" | "exit") =>
    invoke<void>("window_action", { action }),
  load: () => invoke<string | null>("load"),
  save: (document: string) => invoke<void>("save", { document }),
  saveChanges: (changes: WorkspaceChanges) =>
    invoke<void>("save_changes", { changes }),
  hasKey: (profile: Profile) => invoke<boolean>("has_key", { profile }),
  setKey: (profile: Profile, key: string) =>
    invoke<void>("set_key", { profile, key }),
  fetchModels: (profile: Profile, key: string | null) =>
    invoke<string[]>("fetch_models", { profile, key }),
  test: (profile: Profile, key: string | null) =>
    invoke<void>("test_model", { profile, key }).finally(() =>
      window.dispatchEvent(new Event("usage-updated")),
    ),
  previewTemplate: (
    source: string,
    template: OptimizationTemplate,
    templateContent: string,
    revision?: Revision,
    userContent?: string,
  ) =>
    invoke<{ role: "system" | "user"; content: string }[]>("preview_template", {
      source,
      template,
      templateContent,
      userContent,
      revision,
    }),
  optimize: (
    profile: Profile,
    source: string,
    requestId: string,
    template: OptimizationTemplate,
    revision: Revision | undefined,
    templateContent: string,
    userContent?: string,
    followup?: Revision,
    onText?: (text: string) => void,
  ) => {
    const onDelta = new Channel(onText);
    return invoke<string>("optimize", {
      input: {
        profile,
        source,
        requestId,
        template,
        revision,
        templateContent,
        userContent,
        followup,
      },
      onDelta,
    }).finally(() => window.dispatchEvent(new Event("usage-updated")));
  },
  cancel: (requestId: string) => invoke<void>("cancel", { requestId }),
  exportFile: (path: string, content: string) =>
    invoke<void>("export_file", { path, content }),
  exportBackup: (path: string) => invoke<void>("export_backup", { path }),
  previewBackup: (path: string) =>
    invoke<{
      projects: number;
      drafts: number;
      versions: number;
      providers: number;
      usage: number;
      prices: number;
    }>("preview_backup", { path }),
  restoreBackup: (path: string) => invoke<void>("restore_backup", { path }),
};
export function errorMessage(error: unknown) {
  return typeof error === "string"
    ? error
    : error instanceof Error
      ? error.message
      : "操作失败，请重试。";
}
