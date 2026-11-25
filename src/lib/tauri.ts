import { Channel, invoke } from "@tauri-apps/api/core";

export interface Profile {
  isDefault?: boolean;
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
export interface Draft {
  templateId?: string | null;
  providerId?: string | null;
  model?: string | null;
  id: string;
  projectId: string | null;
  title: string;
  source: string;
  result: string;

  favorite: boolean;
  pinned?: boolean;
  opened: number;
  updated?: number;
  messages: ChatMessage[];
  dialogue?: DialogueState;
}
export type ModificationMode = "draft" | "chat";
export interface OptimizationResult {
  inputTokens?: number;
  content: string;
}
export interface ConversationMessage {
  role: "user" | "assistant";
  content: string;
  displayContent?: string;
}
export interface Conversation {
  source: string;
  templateId: string;
  templateName: string;
  systemPrompt: string;
  messages: ConversationMessage[];
  currentDraft?: string;
}
export interface ChatContext {
  source: string;
  templateId: string;
  templateName: string;
  systemPrompt: string;
  currentDraft?: string;
  mode: ModificationMode;
}
export interface ChatMessage extends ConversationMessage {
  inputTokens?: number;
  contextWarningShown?: boolean;
  contextLimitWarningShown?: boolean;
  durationMs?: number;
  id: string;
  created: number;
  context?: ChatContext;
  model?: string;
  providerName?: string;
}
export interface DialogueState {
  mode?: ModificationMode;
  chatInput?: string;
}
export interface Workspace {
  defaultTemplateId?: string | null;
  schema: 1;
  uiState?: {
    recentExpanded: boolean;
    pinnedExpanded: boolean;
    expandedProjects: string[];
    draftListMode: "recent" | "favorites";
  };
  updateSource?: "gitee" | "github";
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
  | "defaultTemplateId"
  | "theme"
  | "closeBehavior"
  | "updateSource"
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
export type TemplateCategory = string;
export interface PromptTemplate {
  id: string;
  category?: TemplateCategory;
  name: string;
  content: string;
  userContent?: string;
  description: string;
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
export interface AvailableUpdate {
  version: string;
  notes?: string;
  downloaded: boolean;
}
export interface SupportInfo {
  version: string;
  runtime: string;
  os: string;
  arch: string;
}
export const desktop = {
  openProjectRepository: (source: "gitee" | "github") =>
    invoke<void>("open_project_repository", { source }),
  supportInfo: () => invoke<SupportInfo>("support_info"),
  checkUpdate: (source?: "gitee" | "github") =>
    invoke<AvailableUpdate | null>("check_update", { source: source ?? null }),
  downloadUpdate: (
    source: "gitee" | "github",
    onProgress: (value: { downloaded: number; total?: number }) => void,
  ) =>
    invoke<void>("download_update", {
      source,
      onProgress: new Channel(onProgress),
    }),
  installUpdate: () => invoke<void>("install_update"),
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
    templateContent: string,
    userContent?: string,
  ) =>
    invoke<{ role: "system" | "user"; content: string }[]>("preview_template", {
      source,
      templateContent,
      userContent,
    }),
  optimize: (
    profile: Profile,
    requestId: string,
    conversation: Conversation,
    mode: ModificationMode,
    onText?: (text: string) => void,
  ) => {
    const onDelta = new Channel(onText);
    return invoke<OptimizationResult>("optimize", {
      input: { profile, requestId, conversation, mode },
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
      messages: number;
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
