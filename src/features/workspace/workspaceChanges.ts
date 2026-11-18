import type {
  Workspace,
  WorkspaceChanges,
  WorkspaceSettings,
} from "../../lib/tauri";
function changed<T extends { id: string }>(before: T[], after: T[]) {
  const previous = new Map(
    before.map((item) => [item.id, JSON.stringify(item)]),
  );
  const remaining = new Set(after.map((item) => item.id));
  const upsert = after.filter(
    (item) => previous.get(item.id) !== JSON.stringify(item),
  );
  const remove = before
    .filter((item) => !remaining.has(item.id))
    .map((item) => item.id);
  return upsert.length || remove.length ? { upsert, remove } : undefined;
}
function settings(value: Workspace): WorkspaceSettings {
  const {
    theme,
    closeBehavior,
    shortcuts,
    activeDraft,
    activeProfile,
    uiState,
  } = value;
  return {
    theme,
    closeBehavior,
    shortcuts,
    activeDraft,
    activeProfile,
    uiState,
  };
}
export function workspaceChanges(
  before: Workspace | undefined,
  after: Workspace,
): WorkspaceChanges {
  const next = settings(after);
  return {
    projects: changed(before?.projects ?? [], after.projects),
    drafts: changed(before?.drafts ?? [], after.drafts),
    profiles: changed(before?.profiles ?? [], after.profiles),
    templates: changed(before?.templates ?? [], after.templates ?? []),
    templateCategories: changed(
      before?.templateCategories ?? [],
      after.templateCategories ?? [],
    ),
    settings:
      !before || JSON.stringify(settings(before)) !== JSON.stringify(next)
        ? next
        : undefined,
  };
}
