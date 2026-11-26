use crate::{
    error::CommandResult,
    models::conversation::{OptimizationResult, OptimizeRequest},
    services,
    state::AppState,
};
use tauri::{ipc::Channel, State};
#[tauri::command]
pub fn preview_template(
    source: String,
    template_content: String,
    user_content: Option<String>,
) -> CommandResult<serde_json::Value> {
    services::optimizer::messages(&source, &template_content, user_content.as_deref())
        .map_err(Into::into)
}
#[tauri::command]
pub async fn optimize(
    state: State<'_, AppState>,
    input: OptimizeRequest,
    on_delta: Channel<String>,
) -> CommandResult<OptimizationResult> {
    services::generation::optimize(&state, input, |delta| {
        let _ = on_delta.send(delta.to_owned());
    })
    .await
    .map_err(Into::into)
}
#[tauri::command]
pub fn cancel(state: State<AppState>, request_id: String) {
    services::generation::cancel(&state, request_id)
}
