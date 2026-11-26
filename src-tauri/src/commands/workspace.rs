use crate::models::provider::ProviderKeyUpdate;
use crate::{
    database::workspace, error::CommandResult, models::workspace::WorkspaceChanges, state::AppState,
};
use tauri::State;
#[tauri::command]
pub fn load(state: State<AppState>) -> CommandResult<Option<String>> {
    workspace::load(&state.database.lock().unwrap()).map_err(|_| "无法读取工作区数据。".into())
}
#[tauri::command]
pub fn save_changes(
    state: State<AppState>,
    changes: WorkspaceChanges,
    provider_key: Option<ProviderKeyUpdate>,
) -> CommandResult<()> {
    workspace::apply_changes(
        &state.database.lock().unwrap(),
        &changes,
        provider_key.as_ref(),
    )
    .map_err(Into::into)
}
