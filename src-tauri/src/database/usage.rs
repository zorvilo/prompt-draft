use super::pricing::backfill;
use crate::models::{pricing::endpoint_scope, provider::Profile, usage::Usage};
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH};
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
