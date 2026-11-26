use crate::{error::CommandResult, services::backup, state::AppState};
use tauri::State;
#[tauri::command]
pub async fn export_file(path: String, content: String) -> CommandResult<()> {
    backup::export_file(path, content).await.map_err(Into::into)
}
#[tauri::command]
pub fn export_backup(state: State<AppState>, path: String) -> CommandResult<()> {
    backup::export_backup(&state, path).map_err(Into::into)
}
#[tauri::command]
pub fn preview_backup(path: String) -> CommandResult<serde_json::Value> {
    backup::preview_backup(path).map_err(Into::into)
}
#[tauri::command]
pub fn restore_backup(state: State<AppState>, path: String) -> CommandResult<()> {
    backup::restore_backup(&state, path).map_err(Into::into)
}
