use super::generation::record_usage;
use crate::{clients::model, models::provider::Profile, state::AppState};
use serde_json::json;
pub async fn test_model(
    state: &AppState,
    profile: Profile,
    key: Option<String>,
) -> Result<(), String> {
    let _operation = state.operations.read().await;
    let key = match key {
        Some(key) => key,
        None => state.key(&profile)?,
    };
    let mut usage = crate::models::usage::Usage::default();
    let start = std::time::Instant::now();
    let result = model::complete(
        &state.client,
        &profile,
        &key,
        json!([{"role":"user","content":"Reply with OK."}]),
        None::<fn(&str)>,
        &mut usage,
    )
    .await;
    record_usage(
        state,
        &profile,
        "test",
        if result.is_ok() { "success" } else { "failed" },
        start,
        usage,
    );
    result.map(|_| ())
}

pub async fn fetch_models(
    state: &AppState,
    profile: Profile,
    key: Option<String>,
) -> Result<Vec<String>, String> {
    let _operation = state.operations.read().await;
    let key = match key {
        Some(key) => key,
        None => state.key(&profile)?,
    };
    if key.trim().is_empty() {
        return Err("请先填写该供应商的 API Key。".into());
    }
    model::fetch_models(&state.client, &profile.endpoint, &key).await
}
