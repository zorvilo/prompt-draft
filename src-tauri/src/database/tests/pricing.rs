use super::*;
use crate::database::usage::{record, stats};
use crate::models::{provider::Profile, usage::Usage};

#[test]
fn fresh_input_is_calculated_per_request_without_changing_input_totals() {
    let db = crate::database::connection::open(std::path::Path::new(":memory:")).unwrap();
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
    let db = crate::database::connection::open(std::path::Path::new(":memory:")).unwrap();
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
    let db = crate::database::connection::open(std::path::Path::new(":memory:")).unwrap();
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
    let db = crate::database::connection::open(std::path::Path::new(":memory:")).unwrap();
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
    let db = crate::database::connection::open(std::path::Path::new(":memory:")).unwrap();
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
    assert!(stats(&db, 0, 0, None, "logs", false, None).unwrap()["summary"]["cost"].is_number());
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
    let db = crate::database::connection::open(std::path::Path::new(":memory:")).unwrap();
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
    let db = crate::database::connection::open(std::path::Path::new(":memory:")).unwrap();
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
    let db = crate::database::connection::open(std::path::Path::new(":memory:")).unwrap();
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
