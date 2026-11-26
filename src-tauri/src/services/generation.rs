use crate::{
    clients::model,
    models::{
        conversation::{OptimizationResult, OptimizeRequest},
        provider::Profile,
    },
    services::optimizer,
    state::AppState,
};
use tokio::sync::oneshot;
pub async fn optimize(
    state: &AppState,
    input: OptimizeRequest,
    mut on_delta: impl FnMut(&str),
) -> Result<OptimizationResult, String> {
    let _operation = state.operations.read().await;
    let OptimizeRequest {
        profile,
        request_id,
        conversation,
        mode,
    } = input;
    let kind = if conversation.messages.len() > 1 {
        "followup"
    } else {
        "optimize"
    };
    let messages = optimizer::conversation_messages(&conversation, mode)?;
    let key = state.key(&profile)?;
    if key.is_empty() {
        return Err("请先在「模型」页面填写 API Key。".into());
    }
    let (sender, receiver) = oneshot::channel();
    state
        .requests
        .lock()
        .unwrap()
        .insert(request_id.clone(), sender);
    let emit_delta = |delta: &str| {
        on_delta(delta);
    };
    let mut usage = crate::models::usage::Usage::default();
    let start = std::time::Instant::now();
    let mut cancelled = false;
    let result = tokio::select! {
        result = model::complete(&state.client, &profile, &key, messages, Some(emit_delta), &mut usage) =>
            result,
        _ = receiver => { cancelled = true; Err("已取消优化。".into()) },
    };
    let result = result
        .and_then(|text| optimizer::parse_reply(&text))
        .map(|mut reply| {
            reply.input_tokens = usage.input;
            reply
        });
    state.requests.lock().unwrap().remove(&request_id);
    record_usage(
        state,
        &profile,
        kind,
        if cancelled {
            "cancelled"
        } else if result.is_ok() {
            "success"
        } else {
            "failed"
        },
        start,
        usage,
    );
    result
}

pub fn cancel(state: &AppState, request_id: String) {
    if let Some(sender) = state.requests.lock().unwrap().remove(&request_id) {
        let _ = sender.send(());
    }
}

pub(super) fn record_usage(
    state: &AppState,
    profile: &Profile,
    kind: &str,
    outcome: &str,
    start: std::time::Instant,
    usage: crate::models::usage::Usage,
) {
    if usage.started {
        if let Err(error) = crate::database::usage::record(
            &state.database.lock().unwrap(),
            profile,
            kind,
            outcome,
            start.elapsed().as_millis() as i64,
            usage,
        ) {
            eprintln!("保存用量统计失败：{error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        database,
        services::{backup, update},
    };
    use serde_json::json;

    #[test]
    fn cancellation_cleans_up_requests_and_preserves_exclusive_operation_guards() {
        // A local listening socket keeps HTTP pending until cancellation; no external API is called.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let profile: Profile = serde_json::from_value(json!({
            "id":"fixture", "name":"Fixture", "model":"mock",
            "endpoint": format!("http://{}", listener.local_addr().unwrap())
        }))
        .unwrap();
        let db = database::connection::open(std::path::Path::new(":memory:")).unwrap();
        database::providers::save_key(&db, &profile, "fixture-key").unwrap();
        let state = AppState::new(db).unwrap();
        let input = serde_json::from_value(json!({
            "profile":profile, "requestId":"pending",
            "mode":"draft", "conversation":{
                "source":"原始需求", "templateId":"prompt-general", "templateName":"基础优化",
                "systemPrompt":"编辑提示词。", "messages":[{"role":"user","content":"原始需求"}]
            }
        }))
        .unwrap();
        let mut deltas = Vec::new();
        let result = tauri::async_runtime::block_on(async {
            let request = optimize(&state, input, |text| deltas.push(text.to_owned()));
            let control = async {
                tokio::task::yield_now().await;
                assert!(state.requests.lock().unwrap().contains_key("pending"));
                assert_eq!(
                    backup::export_backup(&state, String::new()).unwrap_err(),
                    "请等待模型请求或价格同步完成后再导出。"
                );
                assert_eq!(
                    backup::restore_backup(&state, String::new()).unwrap_err(),
                    "请等待模型请求或价格同步完成后再导入。"
                );
                assert_eq!(
                    update::install_update(&state).await.unwrap_err(),
                    "请等待模型请求或数据操作结束后再安装更新。"
                );
                cancel(&state, "pending".into());
            };
            let (result, ()) = tokio::join!(request, control);
            result
        });
        assert_eq!(result.unwrap_err(), "已取消优化。");
        assert!(deltas.is_empty());
        assert!(state.requests.lock().unwrap().is_empty());
        assert!(state.operations.try_write().is_ok());
        let db = state.database.lock().unwrap();
        let outcome: String = db
            .query_row("SELECT status FROM usage_records", [], |row| row.get(0))
            .unwrap();
        assert_eq!(outcome, "cancelled");
    }
}
