use crate::models::{
    pricing::{calculate_charges, endpoint_scope, parse_decimal, remote_quotes},
    usage::CatalogPrice,
};
use rusqlite::{functions::FunctionFlags, params, Connection};
use rust_decimal::Decimal;
use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH};
pub fn initialize(db: &Connection) -> rusqlite::Result<()> {
    db.create_scalar_function(
        "token_charges",
        8,
        FunctionFlags::SQLITE_DETERMINISTIC,
        |ctx| {
            let tokens = [ctx.get(0)?, ctx.get(1)?, ctx.get(2)?, ctx.get(3)?];
            let prices = [ctx.get(4)?, ctx.get(5)?, ctx.get(6)?, ctx.get(7)?];
            Ok(calculate_charges(tokens, prices))
        },
    )?;
    let snapshot: Value = serde_json::from_str(include_str!("resources/usage_rate_snapshot.json"))
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
        let snapshot: Value =
            serde_json::from_str(include_str!("resources/usage_rate_snapshot.json"))
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
pub(super) fn backfill(db: &Connection, id: Option<i64>) -> rusqlite::Result<()> {
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
        serde_json::from_str(include_str!("resources/usage_prices_builtin.json"))
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

#[cfg(test)]
#[path = "tests/pricing.rs"]
mod tests;
