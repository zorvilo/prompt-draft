use crate::{
    error::CommandResult,
    models::update::{AvailableUpdate, UpdateSource},
    services::update,
    state::AppState,
};
use tauri::{ipc::Channel, AppHandle, State};
#[tauri::command]
pub async fn check_update(
    app: AppHandle,
    source: Option<UpdateSource>,
    state: State<'_, AppState>,
) -> CommandResult<Option<AvailableUpdate>> {
    update::check_update(app, source, &state)
        .await
        .map_err(Into::into)
}
#[tauri::command]
pub async fn download_update(
    app: AppHandle,
    source: UpdateSource,
    state: State<'_, AppState>,
    on_progress: Channel<serde_json::Value>,
) -> CommandResult<()> {
    update::download_update(app, source, &state, |value| {
        let _ = on_progress.send(value);
    })
    .await
    .map_err(Into::into)
}
#[tauri::command]
pub async fn install_update(state: State<'_, AppState>) -> CommandResult<()> {
    update::install_update(&state).await.map_err(Into::into)
}
