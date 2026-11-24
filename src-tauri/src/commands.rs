use crate::{model_client, optimizer, storage};
use rusqlite::Connection;
use serde_json::json;
use std::{collections::HashMap, sync::Mutex};
use tauri::{ipc::Channel, State};
use tokio::sync::oneshot;

pub struct AppState {
    database: Mutex<Connection>,
    operations: tokio::sync::RwLock<()>,
    requests: Mutex<HashMap<String, oneshot::Sender<()>>>,
    client: reqwest::Client,
    models_dev: Mutex<Option<serde_json::Value>>,
    models_dev_loading: tokio::sync::Mutex<()>,
}

impl AppState {
    pub fn new(database: Connection) -> Result<Self, reqwest::Error> {
        Ok(Self {
            database: Mutex::new(database),
            operations: tokio::sync::RwLock::new(()),
            requests: Mutex::default(),
            client: model_client::client()?,
            models_dev: Mutex::new(None),
            models_dev_loading: tokio::sync::Mutex::new(()),
        })
    }

    fn key(&self, profile: &storage::Profile) -> Result<String, String> {
        storage::load_key(&self.database.lock().unwrap(), profile)
            .map(Option::unwrap_or_default)
            .map_err(|_| "无法读取已保存的 API Key。".into())
    }
}

#[tauri::command]
pub fn load(state: State<AppState>) -> Result<Option<String>, String> {
    storage::load(&state.database.lock().unwrap()).map_err(|_| "无法读取工作区数据。".into())
}

#[tauri::command]
pub fn save_changes(
    state: State<AppState>,
    changes: storage::WorkspaceChanges,
) -> Result<(), String> {
    storage::apply_changes(&state.database.lock().unwrap(), &changes)
}

#[tauri::command]
pub fn has_key(state: State<AppState>, profile: storage::Profile) -> Result<bool, String> {
    Ok(!state.key(&profile)?.is_empty())
}

#[tauri::command]
pub fn set_key(
    state: State<AppState>,
    profile: storage::Profile,
    key: String,
) -> Result<(), String> {
    storage::save_key(&state.database.lock().unwrap(), &profile, key.trim())
        .map_err(|_| "无法更新本地 API Key。".into())
}

#[tauri::command]
pub async fn test_model(
    state: State<'_, AppState>,
    profile: storage::Profile,
    key: Option<String>,
) -> Result<(), String> {
    let _operation = state.operations.read().await;
    let key = match key {
        Some(key) => key,
        None => state.key(&profile)?,
    };
    let mut usage = crate::usage::Usage::default();
    let start = std::time::Instant::now();
    let result = model_client::complete(
        &state.client,
        &profile,
        &key,
        json!([{"role":"user","content":"Reply with OK."}]),
        None::<fn(&str)>,
        &mut usage,
    )
    .await;
    state.record_usage(
        &profile,
        "test",
        if result.is_ok() { "success" } else { "failed" },
        start,
        usage,
    );
    result.map(|_| ())
}

#[tauri::command]
pub async fn fetch_models(
    state: State<'_, AppState>,
    profile: storage::Profile,
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
    model_client::fetch_models(&state.client, &profile.endpoint, &key).await
}

#[tauri::command]
pub fn preview_template(
    source: String,
    template_content: String,
    user_content: Option<String>,
) -> Result<serde_json::Value, String> {
    optimizer::messages(&source, &template_content, user_content.as_deref())
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OptimizeRequest {
    profile: storage::Profile,
    request_id: String,
    conversation: optimizer::Conversation,
    mode: optimizer::ModificationMode,
}

#[tauri::command]
pub async fn optimize(
    state: State<'_, AppState>,
    input: OptimizeRequest,
    on_delta: Channel<String>,
) -> Result<optimizer::OptimizationResult, String> {
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
        let _ = on_delta.send(delta.to_owned());
    };
    let mut usage = crate::usage::Usage::default();
    let start = std::time::Instant::now();
    let mut cancelled = false;
    let result = tokio::select! {
        result = model_client::complete(&state.client, &profile, &key, messages, Some(emit_delta), &mut usage) =>
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
    state.record_usage(
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

#[tauri::command]
pub fn cancel(state: State<AppState>, request_id: String) {
    if let Some(sender) = state.requests.lock().unwrap().remove(&request_id) {
        let _ = sender.send(());
    }
}

#[tauri::command]
pub async fn export_file(path: String, content: String) -> Result<(), String> {
    std::fs::write(path, content).map_err(|_| "导出失败，请检查文件路径和写入权限。".into())
}

#[tauri::command]
pub fn export_backup(state: State<AppState>, path: String) -> Result<(), String> {
    let _guard = state
        .operations
        .try_write()
        .map_err(|_| "请等待模型请求或价格同步完成后再导出。")?;
    let document = crate::backup::export(&state.database.lock().unwrap())?;
    std::fs::write(path, document).map_err(|_| "无法保存完整备份文件。".into())
}
#[tauri::command]
pub fn preview_backup(path: String) -> Result<serde_json::Value, String> {
    crate::backup::preview(&crate::backup::read(&path)?)
}
#[tauri::command]
pub fn restore_backup(state: State<AppState>, path: String) -> Result<(), String> {
    let _guard = state
        .operations
        .try_write()
        .map_err(|_| "请等待模型请求或价格同步完成后再导入。")?;
    let text = crate::backup::read(&path)?;
    crate::backup::restore(&state.database.lock().unwrap(), &text)?;
    *state.models_dev.lock().unwrap() = None;
    Ok(())
}

impl AppState {
    fn record_usage(
        &self,
        profile: &storage::Profile,
        kind: &str,
        outcome: &str,
        start: std::time::Instant,
        usage: crate::usage::Usage,
    ) {
        if usage.started {
            if let Err(error) = crate::usage::record(
                &self.database.lock().unwrap(),
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
}

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
) -> Result<serde_json::Value, String> {
    crate::usage::stats(
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
pub fn usage_catalog_settings(state: State<AppState>) -> Result<serde_json::Value, String> {
    crate::usage::catalog_settings(&state.database.lock().unwrap())
        .map_err(|_| "读取价格目录设置失败。".into())
}

#[tauri::command]
pub async fn update_usage_catalog(
    state: State<'_, AppState>,
    sync: bool,
    rate: Option<String>,
) -> Result<serde_json::Value, String> {
    let _operation = state.operations.read().await;
    if let Some(rate) = rate {
        if sync {
            return Err("请分别保存汇率或同步报价。".into());
        }
        return crate::usage::update_catalog(&state.database.lock().unwrap(), &rate, None, None);
    }
    let result = update_usage_catalog_impl(state.clone(), sync).await;
    if sync {
        let db = state.database.lock().unwrap();
        let error = result.as_ref().err().map(String::as_str);
        let _ = db.execute(
            "UPDATE pricing_settings SET last_sync_error = ? WHERE id=1",
            [error],
        );
    }
    result
}

async fn update_usage_catalog_impl(
    state: State<'_, AppState>,
    sync: bool,
) -> Result<serde_json::Value, String> {
    let remote = if sync {
        let data = fetch_models_dev_data(&state.client).await?;
        let (selected, count) = {
            let db = state.database.lock().unwrap();
            let mut stmt = db
                .prepare("SELECT DISTINCT lower(model_id) FROM model_prices")
                .map_err(|_| "读取已有定价失败。")?;
            let models = stmt
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(|_| "读取已有定价失败。")?
                .collect::<rusqlite::Result<std::collections::HashSet<_>>>()
                .map_err(|_| "读取已有定价失败。")?;
            let mut entries = crate::models_dev_selection::entries(&data);
            let common = crate::models_dev_selection::common(&entries);
            entries.sort_by_key(|entry| !common.contains(entry["key"].as_str().unwrap_or("")));
            let mut seen = std::collections::HashSet::new();
            let selected: Vec<String> = entries
                .into_iter()
                .filter(|entry| {
                    let model = entry["model"].as_str().unwrap_or("").to_ascii_lowercase();
                    models.contains(&model) && seen.insert(model)
                })
                .map(|entry| entry["key"].as_str().unwrap().to_owned())
                .collect();
            crate::models_dev_selection::selection(&data, &selected)
        };
        {
            let db = state.database.lock().unwrap();
            let _ = db.execute("INSERT INTO models_dev_cache(id,content_json,fetched_at) VALUES(1,?,unixepoch()*1000) ON CONFLICT(id) DO UPDATE SET content_json=excluded.content_json,fetched_at=excluded.fetched_at", [data.to_string()]);
        }
        *state.models_dev.lock().unwrap() = Some(data);
        if count == 0 {
            return Err("models.dev 没有匹配到已有模型定价。".into());
        }
        Some(selected)
    } else {
        None
    };
    let rate_response = fetch_usage_rate(&state.client).await;
    let db = state.database.lock().unwrap();
    let previous = crate::usage::catalog_settings(&db).map_err(|_| "无法读取已保存的汇率。")?;
    let cached = rate_response.is_none();
    let (rate, date) = match rate_response {
        Some((rate, date))
            if previous["rateDate"]
                .as_str()
                .is_some_and(|old| old > date.as_str()) =>
        {
            (
                previous["rate"].as_str().unwrap_or(&rate).to_owned(),
                previous["rateDate"].as_str().unwrap().to_owned(),
            )
        }
        Some(value) => value,
        None => {
            let rate = previous["rate"]
                .as_str()
                .ok_or("暂时无法获取汇率，请稍后重试。")?;
            let date = previous["rateDate"].as_str().unwrap_or("");
            (rate.to_owned(), date.to_owned())
        }
    };
    let mut result = match remote.as_ref() {
        Some(remote) => crate::usage::sync_existing(&db, &rate, &date, remote)?,
        None => crate::usage::update_catalog(&db, &rate, Some(&date), None)?,
    };
    result["cachedRate"] = json!(cached);
    Ok(result)
}

async fn fetch_usage_rate(client: &reqwest::Client) -> Option<(String, String)> {
    let body = client
        .get("https://api.frankfurter.dev/v2/rate/usd/cny")
        .timeout(std::time::Duration::from_secs(8))
        .header(reqwest::header::ACCEPT, "application/json")
        .header(
            reqwest::header::USER_AGENT,
            concat!("PromptDraft/", env!("CARGO_PKG_VERSION")),
        )
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?
        .json::<serde_json::Value>()
        .await
        .ok()?;
    let date = body["date"].as_str()?;
    if body["base"] != "USD"
        || body["quote"] != "CNY"
        || date.len() != 10
        || !date.bytes().enumerate().all(|(i, b)| {
            if i == 4 || i == 7 {
                b == b'-'
            } else {
                b.is_ascii_digit()
            }
        })
        || !body["rate"]
            .as_f64()
            .is_some_and(|value| value > 0.0 && value <= 100.0)
    {
        return None;
    }
    Some((body["rate"].to_string(), date.to_owned()))
}

#[tauri::command]
pub fn usage_catalog_rows(state: State<AppState>) -> Result<Vec<serde_json::Value>, String> {
    crate::usage::catalog_rows(&state.database.lock().unwrap())
        .map_err(|_| "读取定价表失败。".into())
}

#[tauri::command]
pub fn save_usage_catalog_price(
    state: State<AppState>,
    price: crate::usage::CatalogPrice,
    remove: bool,
) -> Result<(), String> {
    crate::usage::save_catalog_price(&state.database.lock().unwrap(), price, remove)
}

async fn fetch_models_dev_data(client: &reqwest::Client) -> Result<serde_json::Value, String> {
    const LIMIT: usize = 32 * 1024 * 1024;
    let failed = |_| "无法连接 models.dev，请稍后重试。";
    let mut response = client
        .get("https://models.dev/api.json")
        .header(reqwest::header::ACCEPT, "application/json")
        .header(
            reqwest::header::USER_AGENT,
            concat!("PromptDraft/", env!("CARGO_PKG_VERSION")),
        )
        .timeout(std::time::Duration::from_secs(25))
        .send()
        .await
        .map_err(failed)?
        .error_for_status()
        .map_err(failed)?;
    let mut data = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(failed)? {
        if chunk.len() > LIMIT - data.len() {
            return Err("models.dev 价格目录超出大小限制。".into());
        }
        data.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&data).map_err(|_| "无法解析 models.dev 价格目录。".into())
}
#[tauri::command]
pub async fn models_dev_picker(
    state: State<'_, AppState>,
    refresh: Option<bool>,
) -> Result<serde_json::Value, String> {
    let _operation = state.operations.read().await;
    let _loading = state.models_dev_loading.lock().await;
    let cached = if refresh == Some(true) {
        None
    } else {
        state.models_dev.lock().unwrap().clone()
    };
    let data = if let Some(data) = cached {
        data
    } else {
        let stored = {
            let db = state.database.lock().unwrap();
            if refresh == Some(true) {
                None
            } else {
                db.query_row(
                    "SELECT content_json FROM models_dev_cache WHERE id=1",
                    [],
                    |row| row.get::<_, String>(0),
                )
                .ok()
            }
        };
        let (data, fetched) = match stored.and_then(|text| serde_json::from_str(&text).ok()) {
            Some(data) => (data, false),
            None => (fetch_models_dev_data(&state.client).await?, true),
        };
        *state.models_dev.lock().unwrap() = Some(data.clone());
        if fetched {
            let db = state.database.lock().unwrap();
            let _ = db.execute("INSERT INTO models_dev_cache(id,content_json,fetched_at) VALUES(1,?,unixepoch()*1000) ON CONFLICT(id) DO UPDATE SET content_json=excluded.content_json,fetched_at=excluded.fetched_at", [data.to_string()]);
        }
        data
    };
    let entries = crate::models_dev_selection::entries(&data);
    Ok(json!({"entries": entries}))
}
#[tauri::command]
pub fn import_models_dev_price(state: State<AppState>, key: String) -> Result<(), String> {
    let data = state
        .models_dev
        .lock()
        .unwrap()
        .clone()
        .ok_or("请先加载 models.dev 目录。")?;
    let entry = crate::models_dev_selection::entries(&data)
        .into_iter()
        .find(|entry| entry["key"] == key)
        .ok_or("找不到选中的模型。")?;
    let db = state.database.lock().unwrap();
    let settings = crate::usage::catalog_settings(&db).map_err(|_| "无法读取汇率。")?;
    let rate = settings["rate"]
        .as_str()
        .ok_or("缺少汇率。")?
        .parse::<rust_decimal::Decimal>()
        .map_err(|_| "汇率无效。")?;
    let price = |key: &str| {
        entry[key]
            .to_string()
            .parse::<rust_decimal::Decimal>()
            .ok()
            .and_then(|n| n.checked_mul(rate))
            .map(|n| n.to_string())
            .ok_or("价格无效")
    };
    let model = entry["model"].as_str().ok_or("模型无效。")?;
    let price = serde_json::from_value(json!({
        "host":"", "model":model, "input":price("input")?, "output":price("output")?,
        "cacheRead":price("cacheRead")?, "cacheWrite":price("cacheWrite")?
    }))
    .map_err(|_| "价格无效。")?;
    crate::usage::save_catalog_price(&db, price, false)
}
