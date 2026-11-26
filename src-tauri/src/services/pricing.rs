use crate::{
    clients::{exchange_rate, models_dev},
    database::catalog,
    state::AppState,
};
use serde_json::json;
pub async fn update_usage_catalog(
    state: &AppState,
    sync: bool,
    rate: Option<String>,
) -> Result<serde_json::Value, String> {
    let _operation = state.operations.read().await;
    if let Some(rate) = rate {
        if sync {
            return Err("请分别保存汇率或同步报价。".into());
        }
        return crate::database::pricing::update_catalog(
            &state.database.lock().unwrap(),
            &rate,
            None,
            None,
        );
    }
    let result = update_usage_catalog_impl(state, sync).await;
    if sync {
        let db = state.database.lock().unwrap();
        let error = result.as_ref().err().map(String::as_str);
        catalog::set_sync_error(&db, error);
    }
    result
}

async fn update_usage_catalog_impl(
    state: &AppState,
    sync: bool,
) -> Result<serde_json::Value, String> {
    let remote = if sync {
        let data = models_dev::fetch(&state.client).await?;
        let (selected, count) = {
            let db = state.database.lock().unwrap();
            let models = catalog::priced_models(&db)?;
            let mut entries = crate::models::catalog::entries(&data);
            let common = crate::models::catalog::common(&entries);
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
            crate::models::catalog::selection(&data, &selected)
        };
        {
            let db = state.database.lock().unwrap();
            catalog::save_cache(&db, &data);
        }
        *state.models_dev.lock().unwrap() = Some(data);
        if count == 0 {
            return Err("models.dev 没有匹配到已有模型定价。".into());
        }
        Some(selected)
    } else {
        None
    };
    let rate_response = exchange_rate::fetch(&state.client).await;
    let db = state.database.lock().unwrap();
    let previous =
        crate::database::pricing::catalog_settings(&db).map_err(|_| "无法读取已保存的汇率。")?;
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
        Some(remote) => crate::database::pricing::sync_existing(&db, &rate, &date, remote)?,
        None => crate::database::pricing::update_catalog(&db, &rate, Some(&date), None)?,
    };
    result["cachedRate"] = json!(cached);
    Ok(result)
}

pub async fn models_dev_picker(
    state: &AppState,
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
                catalog::read_cache(&db)
            }
        };
        let (data, fetched) = match stored.and_then(|text| serde_json::from_str(&text).ok()) {
            Some(data) => (data, false),
            None => (models_dev::fetch(&state.client).await?, true),
        };
        *state.models_dev.lock().unwrap() = Some(data.clone());
        if fetched {
            let db = state.database.lock().unwrap();
            catalog::save_cache(&db, &data);
        }
        data
    };
    let entries = crate::models::catalog::entries(&data);
    Ok(json!({"entries": entries}))
}

pub fn import_models_dev_price(state: &AppState, key: String) -> Result<(), String> {
    let data = state
        .models_dev
        .lock()
        .unwrap()
        .clone()
        .ok_or("请先加载 models.dev 目录。")?;
    let entry = crate::models::catalog::entries(&data)
        .into_iter()
        .find(|entry| entry["key"] == key)
        .ok_or("找不到选中的模型。")?;
    let db = state.database.lock().unwrap();
    let settings = crate::database::pricing::catalog_settings(&db).map_err(|_| "无法读取汇率。")?;
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
    crate::database::pricing::save_catalog_price(&db, price, false)
}
