use crate::{
    database::providers, error::CommandResult, models::provider::Profile, services, state::AppState,
};
use tauri::State;
#[tauri::command]
pub fn has_key(state: State<AppState>, profile: Profile) -> CommandResult<bool> {
    Ok(!state.key(&profile)?.is_empty())
}
#[tauri::command]
pub fn set_key(state: State<AppState>, profile: Profile, key: String) -> CommandResult<()> {
    providers::save_key(&state.database.lock().unwrap(), &profile, key.trim())
        .map_err(|_| "无法更新本地 API Key。".into())
}
#[tauri::command]
pub async fn test_model(
    state: State<'_, AppState>,
    profile: Profile,
    key: Option<String>,
) -> CommandResult<()> {
    services::providers::test_model(&state, profile, key)
        .await
        .map_err(Into::into)
}
#[tauri::command]
pub async fn fetch_models(
    state: State<'_, AppState>,
    profile: Profile,
    key: Option<String>,
) -> CommandResult<Vec<String>> {
    services::providers::fetch_models(&state, profile, key)
        .await
        .map_err(Into::into)
}
