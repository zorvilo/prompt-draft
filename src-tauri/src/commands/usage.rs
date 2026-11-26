use crate::{error::CommandResult, services, state::AppState};
use tauri::State;
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn usage_stats(
    state: State<AppState>,
    since: i64,
    page: u32,
    filter: Option<String>,
    group: String,
    hourly: Option<bool>,
    until: Option<i64>,
) -> CommandResult<serde_json::Value> {
    crate::database::usage::stats(
        &state.database.lock().unwrap(),
        since,
        page,
        filter.as_deref(),
        &group,
        hourly.unwrap_or(false),
        until,
    )
    .map_err(|_| "读取用量统计失败。".into())
}

#[tauri::command]
pub fn usage_catalog_settings(state: State<AppState>) -> CommandResult<serde_json::Value> {
    crate::database::pricing::catalog_settings(&state.database.lock().unwrap())
        .map_err(|_| "读取价格目录设置失败。".into())
}

#[tauri::command]
pub fn usage_catalog_rows(state: State<AppState>) -> CommandResult<Vec<serde_json::Value>> {
    crate::database::pricing::catalog_rows(&state.database.lock().unwrap())
        .map_err(|_| "读取定价表失败。".into())
}

#[tauri::command]
pub fn save_usage_catalog_price(
    state: State<AppState>,
    price: crate::models::usage::CatalogPrice,
    remove: bool,
) -> CommandResult<()> {
    crate::database::pricing::save_catalog_price(&state.database.lock().unwrap(), price, remove)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn update_usage_catalog(
    state: State<'_, AppState>,
    sync: bool,
    rate: Option<String>,
) -> CommandResult<serde_json::Value> {
    services::pricing::update_usage_catalog(&state, sync, rate)
        .await
        .map_err(Into::into)
}
#[tauri::command]
pub async fn models_dev_picker(
    state: State<'_, AppState>,
    refresh: Option<bool>,
) -> CommandResult<serde_json::Value> {
    services::pricing::models_dev_picker(&state, refresh)
        .await
        .map_err(Into::into)
}
#[tauri::command]
pub fn import_models_dev_price(state: State<AppState>, key: String) -> CommandResult<()> {
    services::pricing::import_models_dev_price(&state, key).map_err(Into::into)
}
