import { inject, type InjectionKey } from "vue";
import type { useWorkspaceShell } from "./useWorkspaceShell";
export type WorkspaceContext = ReturnType<typeof useWorkspaceShell>;
export const workspaceContextKey: InjectionKey<WorkspaceContext> =
  Symbol.for("prompt-draft.workspace");
export function useWorkspaceContext() {
  const context = inject(workspaceContextKey);
  if (!context) throw new Error("页面必须在工作区布局中使用。");
  return context;
}
