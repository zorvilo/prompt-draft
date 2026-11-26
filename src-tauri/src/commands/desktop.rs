use crate::{desktop, error::CommandResult, models::update::UpdateSource};
use tauri::AppHandle;
#[tauri::command]
pub fn support_info() -> serde_json::Value {
    desktop::support::support_info()
}
#[tauri::command]
pub fn open_project_repository(source: UpdateSource) -> CommandResult<()> {
    desktop::support::open_project_repository(source).map_err(Into::into)
}
#[tauri::command]
pub fn window_action(app: AppHandle, action: desktop::window::WindowAction) -> CommandResult<()> {
    desktop::window::window_action(app, action).map_err(Into::into)
}
#[tauri::command]
pub fn autostart(app: AppHandle, enabled: Option<bool>) -> CommandResult<bool> {
    desktop::autostart::autostart(app, enabled).map_err(Into::into)
}
