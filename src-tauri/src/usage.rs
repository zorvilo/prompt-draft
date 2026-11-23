use crate::storage::Profile;
use rusqlite::{functions::FunctionFlags, params, Connection};
use rust_decimal::Decimal;
use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Default)]
pub struct Usage {
    pub started: bool,
    input: Option<i64>,
    output: Option<i64>,
    cache_read: Option<i64>,
    cache_write: Option<i64>,
    response_model: Option<String>,
}

impl Usage {
    pub fn capture(&mut self, body: &Value) {
        let count = |key| {
            body.pointer(key)
                .and_then(Value::as_i64)
                .filter(|n| *n >= 0)
        };
        self.input = count("/usage/prompt_tokens").or(self.input);
        self.output = count("/usage/completion_tokens").or(self.output);
        self.cache_read = count("/usage/prompt_tokens_details/cached_tokens")
            .or_else(|| count("/usage/prompt_cache_hit_tokens"))
            .or(self.cache_read);
        self.cache_write = count("/usage/prompt_tokens_details/cache_write_tokens")
            .or_else(|| count("/usage/cache_creation_input_tokens"))
            .or(self.cache_write);
        if let Some(model) = body
            .get("model")
            .and_then(Value::as_str)
            .filter(|m| !m.trim().is_empty() && m.len() <= 200)
        {
            self.response_model = Some(model.trim().into());
        }
    }
}

fn parse_decimal(value: &str) -> Option<Decimal> {
    value
        .parse()
        .or_else(|_| Decimal::from_scientific(value))
        .ok()
}

pub fn initialize(db: &Connection) -> rusqlite::Result<()> {
    db.create_scalar_function(
        "token_charges",
        8,
        FunctionFlags::SQLITE_DETERMINISTIC,
        |ctx| {
            let mut components = Vec::with_capacity(4);
            let mut total = Decimal::ZERO;
            for index in 0..4 {
                let tokens: i64 = ctx.get(index)?;
                let price: String = ctx.get(index + 4)?;
                let Some(amount) = parse_decimal(&price)
                    .and_then(|price| Decimal::from(tokens).checked_mul(price))
                    .map(|amount| amount / Decimal::from(1_000_000))
                else {
                    return Ok(None);
                };
                let Some(sum) = total.checked_add(amount) else {
                    return Ok(None);
                };
                total = sum;
                components.push(amount.to_string());
            }
            Ok(Some(
                json!({
                    "total": total.to_string(), "input": components[0], "output": components[1],
                    "cacheRead": components[2], "cacheWrite": components[3]
                })
                .to_string(),
            ))
        },
    )?;
    let snapshot: Value = serde_json::from_str(include_str!("usage_rate_snapshot.json"))
        .map_err(|_| rusqlite::Error::InvalidQuery)?;
    db.execute(
        "INSERT OR IGNORE INTO pricing_settings(id,usd_to_cny_rate,rate_date) VALUES(1,?1,?2)",
        params![
            snapshot["rate"].as_str().unwrap(),
            snapshot["date"].as_str().unwrap()
        ],
    )?;
    let empty: bool = db.query_row(
        "SELECT count(*) = 0 FROM model_prices WHERE layer='base'",
        [],
        |row| row.get(0),
    )?;
    if empty {
        let snapshot: Value = serde_json::from_str(include_str!("usage_rate_snapshot.json"))
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let settings = catalog_settings(db)?;
        let rate = settings["rate"]
            .as_str()
            .unwrap_or(snapshot["rate"].as_str().unwrap());
        let date = settings["rateDate"]
            .as_str()
            .unwrap_or(snapshot["date"].as_str().unwrap());
        update_catalog(db, rate, Some(date), None).map_err(|_| rusqlite::Error::InvalidQuery)?;
    }
    Ok(())
}

// Charge only rows with complete usage and a price. Previously priced requests retain their amount.
fn backfill(db: &Connection, id: Option<i64>) -> rusqlite::Result<()> {
    db.execute("
        UPDATE usage_records AS u SET cost_details_json = (
            SELECT token_charges(
                max(0, max(0, u.input_tokens - coalesce(u.cache_read_tokens, 0)) - coalesce(u.cache_write_tokens, 0)),
                u.output_tokens, coalesce(u.cache_read_tokens, 0), coalesce(u.cache_write_tokens, 0),
                p.input_price, p.output_price, p.cache_read_price, p.cache_write_price
            ) FROM usage_effective_prices AS p WHERE p.id = u.id AND p.input_price IS NOT NULL
        )
        WHERE u.cost_details_json IS NULL AND u.input_tokens IS NOT NULL AND u.output_tokens IS NOT NULL
          AND (?1 IS NULL OR u.id = ?1)
          AND EXISTS (SELECT 1 FROM usage_effective_prices AS p WHERE p.id = u.id AND p.input_price IS NOT NULL)
    ", params![id])?;
    db.execute("UPDATE usage_records SET cost_cny=json_extract(cost_details_json,'$.total') WHERE cost_cny IS NULL AND cost_details_json IS NOT NULL",[])?;
    Ok(())
}

pub fn record(
    db: &Connection,
    profile: &Profile,
    kind: &str,
    outcome: &str,
    elapsed: i64,
    usage: Usage,
) -> rusqlite::Result<()> {
    let created = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;
    let tx = db.unchecked_transaction()?;
    let pricing_model = usage.response_model.as_ref().unwrap_or(&profile.model);
    tx.execute(
        "INSERT INTO usage_records(created_at, provider_id, provider_name, requested_model, operation, status, duration_ms, input_tokens, output_tokens, billing_model, cache_read_tokens, cache_write_tokens, endpoint_scope) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        params![created, profile.id, profile.name, profile.model, kind, outcome, elapsed, usage.input, usage.output, pricing_model, usage.cache_read, usage.cache_write, endpoint_scope(&profile.endpoint)],
    )?;
    backfill(&tx, Some(tx.last_insert_rowid()))?;
    tx.commit()
}

const FRESH_INPUT: &str = "CASE WHEN input_tokens IS NOT NULL THEN max(0,input_tokens-coalesce(cache_read_tokens,0)-coalesce(cache_write_tokens,0)) END";

pub fn stats(
    db: &Connection,
    since: i64,
    page: u32,
    filter: Option<&str>,
    group: &str,
    hourly: bool,
    until: Option<i64>,
) -> rusqlite::Result<Value> {
    const FILTER: &str = " FROM usage_records WHERE created_at >= ?1*1000 AND (?3 IS NULL OR created_at < (?3+1)*1000) AND (?2 IS NULL OR instr(lower(provider_name), lower(?2)) > 0 OR instr(lower(requested_model), lower(?2)) > 0)";
    const TOTALS: &str = "count(*), count(CASE WHEN status = 'success' THEN 1 END), sum(input_tokens), sum(output_tokens), count(CASE WHEN input_tokens IS NOT NULL AND output_tokens IS NOT NULL THEN 1 END), sum(CAST(cost_cny AS REAL)), count(cost_cny), sum(cache_read_tokens), sum(cache_write_tokens)";
    let columns = format!("{TOTALS},sum({FRESH_INPUT})");
    let args = params![since, filter, until];
    let summary = db.query_row(&format!("SELECT {columns}{FILTER}"), args, |row| {
        totals(row, 0)
    })?;
    let groups = match group {
        "provider" => "provider_name, provider_id",
        "model" => "requested_model, requested_model",
        _ => "",
    };
    let mut rows = Vec::new();
    let offset = i64::from(page) * 20;
    let total = if groups.is_empty() {
        let mut stmt = db.prepare(&format!("SELECT id,created_at,provider_name,requested_model,operation,status,duration_ms,input_tokens,output_tokens,CAST(cost_cny AS REAL),billing_model,cache_read_tokens,cache_write_tokens,{FRESH_INPUT}{FILTER} ORDER BY created_at DESC,id DESC LIMIT 20 OFFSET ?4"))?;
        let results = stmt.query_map(params![since,filter,until,offset],|row|Ok(json!({
            "id":row.get::<_,i64>(0)?, "created":row.get::<_,i64>(1)?/1000,"provider":row.get::<_,String>(2)?,
            "model":row.get::<_,String>(3)?,"kind":row.get::<_,String>(4)?,"outcome":row.get::<_,String>(5)?,
            "elapsed":row.get::<_,i64>(6)?,"input":row.get::<_,Option<i64>>(7)?,"output":row.get::<_,Option<i64>>(8)?,
            "cost":row.get::<_,Option<f64>>(9)?,"pricingModel":row.get::<_,String>(10)?,
            "cacheRead":row.get::<_,Option<i64>>(11)?,"cacheWrite":row.get::<_,Option<i64>>(12)?,"freshInput":row.get::<_,Option<i64>>(13)?
        })))?;
        rows = results.collect::<rusqlite::Result<_>>()?;
        summary["count"].as_i64().unwrap_or_default()
    } else {
        let (label, key) = groups.split_once(", ").unwrap();
        let mut stmt = db.prepare(&format!("SELECT {key}, max({label}), {columns}{FILTER} GROUP BY {key} ORDER BY count(*) DESC, {key} LIMIT 20 OFFSET ?4"))?;
        let results = stmt.query_map(params![since, filter, until, offset], |row| {
            let mut item = totals(row, 2)?;
            item["id"] = json!(row.get::<_, String>(0)?);
            item["label"] = json!(row.get::<_, String>(1)?);
            Ok(item)
        })?;
        rows.extend(results.collect::<rusqlite::Result<Vec<_>>>()?);
        db.query_row(
            &format!("SELECT count(DISTINCT {key}){FILTER}"),
            args,
            |row| row.get(0),
        )?
    };
    let bucket = if hourly {
        "strftime('%Y-%m-%dT%H:00', created_at/1000, 'unixepoch', 'localtime')"
    } else {
        "date(created_at/1000, 'unixepoch', 'localtime')"
    };
    let mut stmt = db.prepare(&format!(
        "SELECT {bucket}, {columns}{FILTER} GROUP BY 1 ORDER BY 1"
    ))?;
    let trend = stmt
        .query_map(args, |row| {
            let mut item = totals(row, 1)?;
            item["date"] = json!(row.get::<_, String>(0)?);
            Ok(item)
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(json!({ "summary": summary, "rows": rows, "trend": trend, "total": total }))
}

fn totals(row: &rusqlite::Row<'_>, offset: usize) -> rusqlite::Result<Value> {
    Ok(json!({
        "count": row.get::<_, i64>(offset)?, "success": row.get::<_, i64>(offset+1)?,
        "input": row.get::<_, Option<i64>>(offset+2)?, "output": row.get::<_, Option<i64>>(offset+3)?,
        "reported": row.get::<_, i64>(offset+4)?,
        "cost": row.get::<_, Option<f64>>(offset+5)?, "priced": row.get::<_, i64>(offset+6)?,
        "cacheRead": row.get::<_, Option<i64>>(offset+7)?, "cacheWrite": row.get::<_, Option<i64>>(offset+8)?,
        "freshInput": row.get::<_, Option<i64>>(offset+9)?
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fresh_input_is_calculated_per_request_without_changing_input_totals() {
        let db = crate::storage::open(std::path::Path::new(":memory:")).unwrap();
        let profile: Profile = serde_json::from_value(
            json!({"id":"fresh","name":"Example","model":"test","endpoint":"https://example.com"}),
        )
        .unwrap();
        for (input, read, write) in [
            (Some(708), 512, 0),
            (Some(708), 512, 20),
            (None, 16, 0),
            (Some(5), 10, 0),
        ] {
            let mut usage = Usage::default();
            usage.capture(&json!({"usage":{"prompt_tokens":input,"completion_tokens":1,"prompt_cache_hit_tokens":read,"cache_creation_input_tokens":write}}));
            record(&db, &profile, "optimize", "success", 1, usage).unwrap();
        }
        for group in ["logs", "model", "provider"] {
            let report = stats(&db, 0, 0, None, group, false, None).unwrap();
            assert_eq!(report["summary"]["input"], 1421);
            assert_eq!(report["summary"]["freshInput"], 372);
            assert_eq!(report["summary"]["cacheRead"], 1050);
            assert_eq!(report["trend"][0]["freshInput"], 372);
            if group == "logs" {
                assert_eq!(report["rows"][0]["freshInput"], 0);
                assert!(report["rows"][1]["freshInput"].is_null());
                assert_eq!(report["rows"][2]["freshInput"], 176);
                assert_eq!(report["rows"][3]["freshInput"], 196);
            } else {
                assert_eq!(report["rows"][0]["freshInput"], 372);
            }
        }
    }
    #[test]
    fn hourly_trends_keep_summary_and_split_hours() {
        let db = crate::storage::open(std::path::Path::new(":memory:")).unwrap();
        let profile: Profile = serde_json::from_value(
            json!({"id":"hours","name":"Hours","model":"test","endpoint":"https://example.com"}),
        )
        .unwrap();
        for (time, input) in [
            ("2026-11-18 09:10:00", 10),
            ("2026-11-18 09:40:00", 20),
            ("2026-11-18 10:05:00", 30),
        ] {
            let mut usage = Usage::default();
            usage.capture(&json!({"usage":{"prompt_tokens":input,"completion_tokens":5}}));
            record(&db, &profile, "optimize", "success", 100, usage).unwrap();
            db.execute("UPDATE usage_records SET created_at = strftime('%s', ?1, 'utc')*1000 WHERE id = (SELECT max(id) FROM usage_records)", [time]).unwrap();
        }
        let hourly = stats(&db, 0, 0, None, "logs", true, None).unwrap();
        let daily = stats(&db, 0, 0, None, "logs", false, None).unwrap();
        assert_eq!(hourly["summary"], daily["summary"]);
        assert_eq!(hourly["trend"].as_array().unwrap().len(), 2);
        assert_eq!(hourly["trend"][0]["date"], "2026-11-18T09:00");
        assert_eq!(hourly["trend"][0]["input"], 30);
        assert_eq!(hourly["trend"][0]["output"], 10);
        assert_eq!(hourly["trend"][1]["count"], 1);
        assert_eq!(daily["trend"].as_array().unwrap().len(), 1);
        let cutoff: i64 = db
            .query_row(
                "SELECT CAST(strftime('%s', '2026-11-18 09:40:00', 'utc') AS INTEGER)",
                [],
                |row| row.get(0),
            )
            .unwrap();
        for group in ["logs", "provider", "model"] {
            let bounded = stats(&db, 0, 0, None, group, true, Some(cutoff)).unwrap();
            assert_eq!(bounded["summary"]["count"], 2);
            assert_eq!(bounded["summary"]["input"], 30);
            assert_eq!(bounded["trend"].as_array().unwrap().len(), 1);
        }
        assert_eq!(
            stats(&db, cutoff, 0, None, "logs", true, Some(cutoff)).unwrap()["summary"]["count"],
            1
        );

        assert_eq!(
            stats(&db, 0, 0, Some("missing"), "logs", true, None).unwrap()["trend"],
            json!([])
        );
    }
    #[test]
    fn sync_updates_prices_in_place_without_adding_rows() {
        let db = crate::storage::open(std::path::Path::new(":memory:")).unwrap();
        let before = catalog_rows(&db).unwrap().len();
        let data = json!({"deepseek":{"api":"https://api.deepseek.com","models":{
            "deepseek-v3.2":{"cost":{"input":2,"output":4,"cache_read":0.5}},
            "new-unselected-model":{"cost":{"input":1,"output":2}}
        }}});
        sync_existing(&db, "7", "2026-11-18", &data).unwrap();
        sync_existing(&db, "7", "2026-11-18", &data).unwrap();
        let rows = catalog_rows(&db).unwrap();
        assert_eq!(rows.len(), before);
        assert_eq!(before, 83);
        let row = rows
            .iter()
            .find(|row| row["model"] == "deepseek-v3.2")
            .unwrap();
        assert_eq!(row["input"], "14");
        assert_eq!(row["source"], "models.dev");
        assert!(!rows
            .iter()
            .any(|row| row["model"] == "new-unselected-model"));
    }

    #[test]
    fn sync_preserves_protected_deepseek_prices_and_updates_other_models() {
        let db = crate::storage::open(std::path::Path::new(":memory:")).unwrap();
        let snapshot = |model: &str| -> Vec<(String, String, String, String, String, Option<i64>)> {
            db.prepare("SELECT input_price,output_price,cache_read_price,cache_write_price,source,synced_at FROM model_prices WHERE model_id=? ORDER BY endpoint_scope,layer").unwrap().query_map([model],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).unwrap().collect::<rusqlite::Result<_>>().unwrap()
        };
        let flash = snapshot("deepseek-flash");
        let pro = snapshot("deepseek-v4-pro");
        let protected = json!({"deepseek":{"api":"https://api.deepseek.com","models":{
            "DeepSeek-Flash":{"cost":{"input":999,"output":999,"cache_read":999,"cache_write":999}},
            "deepseek-v4-pro":{"cost":{"input":999,"output":999,"cache_read":999,"cache_write":999}}
        }}});
        let result = sync_existing(&db, "7", "2026-11-20", &protected).unwrap();
        assert_eq!(result["synced"], 0);
        assert_eq!(snapshot("deepseek-flash"), flash);
        assert_eq!(snapshot("deepseek-v4-pro"), pro);
        let mut mixed = protected.clone();
        mixed["deepseek"]["models"]["deepseek-v3.2"] =
            json!({"cost":{"input":2,"output":4,"cache_read":0.5,"cache_write":0}});
        let result = sync_existing(&db, "7", "2026-11-20", &mixed).unwrap();
        assert_eq!(result["synced"], 1);
        assert_eq!(snapshot("deepseek-flash"), flash);
        assert_eq!(snapshot("deepseek-v4-pro"), pro);
        assert_eq!(snapshot("deepseek-v3.2")[0].0, "14");
        update_catalog(&db, "7", Some("2026-11-20"), Some(&protected)).unwrap();
        for model in ["deepseek-flash", "deepseek-v4-pro"] {
            let rows = snapshot(model);
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].4, "CC Switch");
            assert_ne!(rows[0].0, "6993");
        }
    }

    #[test]
    fn offline_first_launch_prices_without_network_and_overrides_survive_refresh() {
        let db = crate::storage::open(std::path::Path::new(":memory:")).unwrap();
        assert_eq!(catalog_rows(&db).unwrap().len(), 83);
        let models = catalog_rows(&db).unwrap();
        assert!(models.iter().any(|row| row["model"] == "deepseek-v4-pro"));
        assert!(!models.iter().any(|row| [
            "gpt", "claude", "gemini", "grok", "mistral", "command", "o1", "o3", "codex"
        ]
        .iter()
        .any(|prefix| row["model"].as_str().unwrap().starts_with(prefix))));
        assert!(catalog_settings(&db).unwrap()["rateDate"].is_string());
        let profile: Profile = serde_json::from_value(
            json!({"id":"local","name":"Internal API","model":"deepseek-flash","endpoint":"http://10.147.103.23:9666/v1"}),
        ).unwrap();
        let mut usage = Usage::default();
        usage.capture(&json!({"usage":{"prompt_tokens":100,"completion_tokens":10}}));
        record(&db, &profile, "optimize", "success", 20, usage).unwrap();
        assert!(
            stats(&db, 0, 0, None, "logs", false, None).unwrap()["summary"]["cost"].is_number()
        );
        let custom = || CatalogPrice {
            host: String::new(),
            model: "deepseek-flash".into(),
            input: "2".into(),
            output: "3".into(),
            cache_read: "1".into(),
            cache_write: "2".into(),
        };
        save_catalog_price(&db, custom(), false).unwrap();
        update_catalog(&db, "7", Some("2026-11-18"), None).unwrap();
        assert_eq!(
            catalog_rows(&db)
                .unwrap()
                .iter()
                .find(|row| row["model"] == "deepseek-flash")
                .unwrap()["source"],
            "manual"
        );
        save_catalog_price(&db, custom(), true).unwrap();
        assert_eq!(
            catalog_rows(&db)
                .unwrap()
                .iter()
                .find(|row| row["model"] == "deepseek-flash")
                .unwrap()["source"],
            "CC Switch"
        );
    }

    #[test]
    fn catalog_converts_to_cny_backfills_and_preserves_manual_priority() {
        let db = crate::storage::open(std::path::Path::new(":memory:")).unwrap();
        let profile: Profile = serde_json::from_value(
            json!({"id":"one","name":"Gateway","model":"test-sync-model","endpoint":"https://api.deepseek.com/v1"}),
        ).unwrap();
        let record_sample = || {
            let mut usage = Usage::default();
            usage.capture(&json!({"usage":{"prompt_tokens":1000,"completion_tokens":500,"prompt_cache_hit_tokens":200}}));
            record(&db, &profile, "optimize", "success", 100, usage).unwrap();
        };
        record_sample();
        let catalog = json!({"deepseek":{"models":{"test-sync-model":{"cost":{"input":2,"output":4,"cache_read":0.5}}}}});
        let result = update_catalog(&db, "7", Some("2026-11-17"), Some(&catalog)).unwrap();
        assert_eq!(result["synced"], 1);
        assert_eq!(result["rateDate"], "2026-11-17");
        let report = stats(&db, 0, 0, None, "logs", false, None).unwrap();
        assert_eq!(report["rows"][0]["cost"], 0.0259);
        let seed_count: i64 = db
            .query_row(
                "SELECT count(*) FROM model_prices WHERE endpoint_scope = '' AND layer='base'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(seed_count, 83);
        assert!(update_catalog(&db, "8", Some("2026-11-18"), Some(&json!({}))).is_err());
        update_catalog(&db, "8", Some("2026-11-18"), None).unwrap();
        save_catalog_price(
            &db,
            CatalogPrice {
                host: String::new(),
                model: "test-sync-model".into(),
                input: "10".into(),
                output: "20".into(),
                cache_read: "1".into(),
                cache_write: "10".into(),
            },
            false,
        )
        .unwrap();
        record_sample();
        let report = stats(&db, 0, 0, None, "logs", false, None).unwrap();
        assert_eq!(report["rows"][0]["cost"], 0.0182);
        assert_eq!(report["rows"][1]["cost"], 0.0259);
        update_catalog(&db, "6.5", None, None).unwrap();
        let rows = catalog_rows(&db).unwrap();
        let custom = rows
            .iter()
            .find(|row| row["model"] == "test-sync-model" && row["source"] == "manual")
            .unwrap();
        assert_eq!(custom["input"], "10");
        let builtin = rows
            .iter()
            .find(|row| row["model"] == "deepseek-v4-pro")
            .unwrap();
        assert_eq!(builtin["input"], "8.580");
        let after = stats(&db, 0, 0, None, "logs", false, None).unwrap();
        assert_eq!(after["rows"][0]["cost"], 0.0182);
        assert_eq!(after["rows"][1]["cost"], 0.0259);
        for invalid in ["0", "-1", "101", "NaN", ""] {
            assert!(update_catalog(&db, invalid, None, None).is_err());
        }
    }

    #[test]
    fn pricing_uses_response_model_when_reported() {
        let db = crate::storage::open(std::path::Path::new(":memory:")).unwrap();
        let profile: Profile = serde_json::from_value(
            json!({"id":"one","name":"Gateway","model":"alias","endpoint":"https://example.com"}),
        )
        .unwrap();
        let mut usage = Usage::default();
        usage.capture(&json!({"model":"actual","usage":{"prompt_tokens":8,"completion_tokens":3}}));
        record(&db, &profile, "optimize", "success", 100, usage).unwrap();
        let report = stats(&db, 0, 0, None, "logs", false, None).unwrap();
        assert_eq!(report["rows"][0]["model"], "alias");
        assert_eq!(report["rows"][0]["pricingModel"], "actual");
    }

    #[test]
    fn missing_usage_stays_unknown_and_provider_instances_stay_separate() {
        let db = crate::storage::open(std::path::Path::new(":memory:")).unwrap();
        let mut profile: Profile = serde_json::from_value(
            json!({"id":"one","name":"DeepSeek","model":"model","endpoint":"https://example.com"}),
        )
        .unwrap();
        let mut usage = Usage::default();
        usage.capture(&json!({"usage":{"prompt_tokens":8,"completion_tokens":3,"prompt_cache_hit_tokens":2,"cache_creation_input_tokens":2}}));
        usage.capture(&json!({"usage":null}));
        record(&db, &profile, "optimize", "success", 100, usage).unwrap();
        profile.id = "two".into();
        record(&db, &profile, "test", "failed", 200, Usage::default()).unwrap();
        let result = stats(&db, 0, 0, Some("model"), "provider", false, None).unwrap();
        assert_eq!(result["total"], 2);
        assert!(result["summary"]["cost"].is_null());
        let price = |input: &str, output: &str| CatalogPrice {
            host: String::new(),
            model: "model".into(),
            input: input.into(),
            output: output.into(),
            cache_read: "2".into(),
            cache_write: "5".into(),
        };
        save_catalog_price(&db, price("10", "20"), false).unwrap();
        let priced = stats(&db, 0, 0, None, "logs", false, None).unwrap();
        assert_eq!(priced["summary"]["priced"], 1);
        assert!((priced["summary"]["cost"].as_f64().unwrap() - 0.000114).abs() < 1e-10);
        assert!(priced["rows"][0]["cost"].is_null());
        let charge: String = db
            .query_row(
                "SELECT cost_details_json FROM usage_records WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let breakdown: Value = serde_json::from_str(&charge).unwrap();
        assert_eq!(breakdown["cacheRead"], "0.000004");
        save_catalog_price(&db, price("200", "200"), false).unwrap();
        assert_eq!(
            stats(&db, 0, 0, None, "logs", false, None).unwrap()["summary"]["cost"],
            priced["summary"]["cost"]
        );
        assert!(save_catalog_price(&db, price("-1", "0"), false).is_err());
        assert_eq!(result["summary"]["reported"], 1);
        assert_eq!(result["summary"]["input"], 8);
        assert_eq!(result["summary"]["cacheRead"], 2);
        assert_eq!(result["summary"]["cacheWrite"], 2);
        let trend = stats(&db, 0, 0, None, "logs", false, None).unwrap();
        assert_eq!(trend["trend"][0]["cacheRead"], 2);
        assert_eq!(trend["trend"][0]["cacheWrite"], 2);
        assert!(result["rows"][1]["input"].is_null());
        assert_eq!(
            stats(&db, 0, 0, Some("missing"), "logs", false, None).unwrap()["total"],
            0
        );
    }
}
fn endpoint_scope(endpoint: &str) -> String {
    let Ok(url) = url::Url::parse(endpoint) else {
        return String::new();
    };
    let Some(host) = url.host_str() else {
        return String::new();
    };
    let path = url.path().trim_end_matches('/');
    // DeepSeek documents /v1 as a compatibility alias for the same API.
    let path = if host == "api.deepseek.com" && path == "/v1" {
        ""
    } else {
        path
    };
    let port = url
        .port()
        .map(|port| format!(":{port}"))
        .unwrap_or_default();
    format!("{host}{port}{path}")
}

pub fn catalog_settings(db: &Connection) -> rusqlite::Result<Value> {
    let mut stmt = db.prepare(
        "SELECT usd_to_cny_rate, last_synced_at, rate_date, last_sync_error FROM pricing_settings WHERE id = 1",
    )?;
    let mut rows = stmt.query([])?;
    match rows.next()? {
        Some(row) => Ok(
            json!({ "rate": row.get::<_, String>(0)?, "syncedAt": row.get::<_, Option<i64>>(1)?.map(|v|v/1000), "rateDate": row.get::<_, Option<String>>(2)?, "lastError": row.get::<_, Option<String>>(3)? }),
        ),
        None => Ok(json!({ "rate": null, "syncedAt": null, "rateDate": null })),
    }
}

fn remote_quotes(remote: &Value) -> Result<Vec<(String, String, [Decimal; 4])>, String> {
    let providers = remote
        .as_object()
        .ok_or("models.dev 返回了无效价格目录。")?;
    let mut quotes = Vec::new();
    for (id, provider) in providers {
        let host = provider
            .get("api")
            .and_then(Value::as_str)
            .map(endpoint_scope)
            .filter(|h| !h.is_empty())
            .unwrap_or_else(|| {
                match id.as_str() {
                    "openai" => "api.openai.com/v1",
                    "deepseek" => "api.deepseek.com",
                    "anthropic" => "api.anthropic.com/v1",
                    "openrouter" => "openrouter.ai/api/v1",
                    "zhipuai" => "open.bigmodel.cn/api/paas/v4",
                    "moonshotai" => "api.moonshot.ai/v1",
                    _ => "",
                }
                .into()
            });
        if host.is_empty() {
            continue;
        }
        let Some(models) = provider.get("models").and_then(Value::as_object) else {
            continue;
        };
        for (model, entry) in models {
            let count = |key| {
                entry
                    .get("cost")
                    .and_then(|c| c.get(key))
                    .filter(|v| v.is_number())
                    .and_then(|v| parse_decimal(&v.to_string()))
                    .filter(|v| *v >= Decimal::ZERO)
            };
            let (Some(input), Some(output)) = (count("input"), count("output")) else {
                continue;
            };
            quotes.push((
                host.clone(),
                model.to_ascii_lowercase(),
                [
                    input,
                    output,
                    count("cache_read").unwrap_or(input),
                    count("cache_write").unwrap_or(input),
                ],
            ));
        }
    }
    if quotes.is_empty() {
        return Err("models.dev 未返回可匹配的报价。".into());
    }
    quotes.retain(|(_, model, _)| !matches!(model.as_str(), "deepseek-flash" | "deepseek-v4-pro"));
    Ok(quotes)
}

pub fn update_catalog(
    db: &Connection,
    rate: &str,
    rate_date: Option<&str>,
    remote: Option<&Value>,
) -> Result<Value, String> {
    let rate = parse_decimal(rate)
        .filter(|r| *r > Decimal::ZERO && *r <= Decimal::from(100))
        .ok_or("请输入有效的人民币兑美元换算值。")?;
    let failed = |_| "更新价格目录失败。";
    let seed: std::collections::BTreeMap<String, [String; 4]> =
        serde_json::from_str(include_str!("usage_prices_builtin.json"))
            .map_err(|_| "无法读取内置价格目录。")?;
    let mut quotes = Vec::new();
    for (model, values) in seed {
        if let [Some(input), Some(output), Some(read), Some(write)] =
            values.map(|p| parse_decimal(&p))
        {
            quotes.push((String::new(), model, [input, output, read, write]));
        }
    }
    let tx = db.unchecked_transaction().map_err(failed)?;
    let synced = if let Some(remote) = remote {
        let downloaded = remote_quotes(remote)?;
        let count = downloaded.len();
        quotes.extend(downloaded);
        count
    } else {
        let previous = catalog_settings(&tx).map_err(failed)?;
        if let Some(old) = previous["rate"].as_str().and_then(parse_decimal) {
            let mut stmt = tx.prepare("SELECT endpoint_scope, model_id, input_price, output_price, cache_read_price, cache_write_price FROM model_prices WHERE layer='base' AND endpoint_scope <> ''").map_err(failed)?;
            let rows = stmt
                .query_map([], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        [r.get::<_, String>(2)?, r.get(3)?, r.get(4)?, r.get(5)?],
                    ))
                })
                .map_err(failed)?;
            for row in rows {
                let (host, model, values) = row.map_err(failed)?;
                if let [Some(a), Some(b), Some(c), Some(d)] =
                    values.map(|value| parse_decimal(&value).and_then(|p| p.checked_div(old)))
                {
                    quotes.push((host, model, [a, b, c, d]));
                }
            }
        }
        0
    };
    tx.execute("DELETE FROM model_prices WHERE layer='base'", [])
        .map_err(failed)?;
    {
        let mut insert = tx
            .prepare("INSERT OR REPLACE INTO model_prices(endpoint_scope,model_id,layer,input_price,output_price,cache_read_price,cache_write_price,source) VALUES (?,?,'base',?,?,?, ?, CASE WHEN ?1='' THEN 'CC Switch' ELSE 'models.dev' END)")
            .map_err(failed)?;
        for (host, model, values) in quotes {
            if let [Some(a), Some(b), Some(c), Some(d)] =
                values.map(|p| p.checked_mul(rate).map(|v| v.to_string()))
            {
                insert
                    .execute(params![host, model, a, b, c, d])
                    .map_err(failed)?;
            }
        }
    }
    let synced_at = remote.map(|_| {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64
    });
    tx.execute("INSERT INTO pricing_settings (id, usd_to_cny_rate, last_synced_at, rate_date) VALUES (1, ?, ?, ?) ON CONFLICT(id) DO UPDATE SET usd_to_cny_rate = excluded.usd_to_cny_rate, last_synced_at = coalesce(excluded.last_synced_at, pricing_settings.last_synced_at), rate_date = excluded.rate_date", params![rate.to_string(), synced_at, rate_date.unwrap_or("")]).map_err(failed)?;
    backfill(&tx, None).map_err(failed)?;
    let mut result = catalog_settings(&tx).map_err(failed)?;
    result["synced"] = json!(synced);
    tx.commit().map_err(failed)?;
    Ok(result)
}

pub fn catalog_rows(db: &Connection) -> rusqlite::Result<Vec<Value>> {
    let mut stmt = db.prepare(
        "
        SELECT c.endpoint_scope,c.model_id,c.input_price,c.output_price,c.cache_read_price,c.cache_write_price,c.source
        FROM model_prices c WHERE (c.layer='override' OR NOT EXISTS(
            SELECT 1 FROM model_prices o WHERE o.endpoint_scope=c.endpoint_scope AND o.model_id=c.model_id AND o.layer='override'))
        ORDER BY 2, 1
    ",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(json!({
            "host": row.get::<_, String>(0)?, "model": row.get::<_, String>(1)?,
            "input": row.get::<_, String>(2)?, "output": row.get::<_, String>(3)?,
            "cacheRead": row.get::<_, String>(4)?, "cacheWrite": row.get::<_, String>(5)?,
            "source": row.get::<_, String>(6)?
        }))
    })?;
    rows.collect()
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogPrice {
    host: String,
    model: String,
    input: String,
    output: String,
    cache_read: String,
    cache_write: String,
}

pub fn save_catalog_price(
    db: &Connection,
    price: CatalogPrice,
    remove: bool,
) -> Result<(), String> {
    let model = price.model.trim().to_ascii_lowercase();
    if model.is_empty() || model.len() > 200 || model.chars().any(char::is_control) {
        return Err("请输入有效的模型 ID。".into());
    }
    let scope = price.host.trim();
    let normalized;
    let host = if scope.contains("://") {
        normalized = endpoint_scope(scope);
        if normalized.is_empty() {
            return Err("请输入有效的接口范围。".into());
        }
        normalized.as_str()
    } else {
        scope
    };
    if host.len() > 500 || host.chars().any(char::is_control) {
        return Err("接口范围格式不正确。".into());
    }
    let error = |_| "保存价格失败。";
    let tx = db.unchecked_transaction().map_err(error)?;
    if remove {
        tx.execute(
            "DELETE FROM model_prices WHERE layer='override' AND endpoint_scope=? AND model_id=?",
            params![host, model],
        )
        .map_err(error)?;
    } else {
        let values = [
            price.input,
            price.output,
            price.cache_read,
            price.cache_write,
        ]
        .map(|v| {
            parse_decimal(&v)
                .filter(|n| *n >= Decimal::ZERO)
                .map(|n| n.to_string())
        });
        let [Some(input), Some(output), Some(read), Some(write)] = values else {
            return Err("请输入有效的非负单价。".into());
        };
        tx.execute("INSERT INTO model_prices(endpoint_scope,model_id,layer,input_price,output_price,cache_read_price,cache_write_price,source) VALUES (?,?,'override',?,?,?,?,'manual') ON CONFLICT(endpoint_scope,model_id,layer) DO UPDATE SET input_price = excluded.input_price, output_price = excluded.output_price, cache_read_price = excluded.cache_read_price, cache_write_price = excluded.cache_write_price",
            params![host, model, input, output, read, write]).map_err(error)?;
    }
    backfill(&tx, None).map_err(error)?;
    tx.commit().map_err(error)?;
    Ok(())
}

pub fn sync_existing(
    db: &Connection,
    rate: &str,
    date: &str,
    data: &Value,
) -> Result<Value, String> {
    let rate = parse_decimal(rate)
        .filter(|v| *v > Decimal::ZERO)
        .ok_or("汇率无效。")?;
    let quotes = remote_quotes(data)?;
    let fail = |_| "更新已有定价失败。";
    let tx = db.unchecked_transaction().map_err(fail)?;
    let mut changed = 0;
    for (host, model, prices) in quotes {
        let [Some(a), Some(b), Some(c), Some(d)] =
            prices.map(|v| v.checked_mul(rate).map(|v| v.to_string()))
        else {
            return Err("价格超出范围。".into());
        };
        changed += tx
            .execute(
                "UPDATE model_prices SET input_price=?1,output_price=?2,cache_read_price=?3,cache_write_price=?4,
            source=CASE WHEN layer='base' THEN 'models.dev' ELSE source END,synced_at=unixepoch()*1000
            WHERE model_id=?5 AND (endpoint_scope='' OR endpoint_scope=?6)",
                params![a, b, c, d, model, host],
            )
            .map_err(fail)?;
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;
    tx.execute(
        "UPDATE pricing_settings SET usd_to_cny_rate=?, rate_date=?, last_synced_at=? WHERE id=1",
        params![rate.to_string(), date, now],
    )
    .map_err(fail)?;
    backfill(&tx, None).map_err(fail)?;
    let mut result = catalog_settings(&tx).map_err(fail)?;
    result["synced"] = json!(changed);
    tx.commit().map_err(fail)?;
    Ok(result)
}
