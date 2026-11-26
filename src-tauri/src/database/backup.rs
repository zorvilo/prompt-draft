use crate::models::backup::{Backup, TABLES};
use rusqlite::{
    params_from_iter,
    types::{Value as SqlValue, ValueRef},
    Connection,
};
use serde_json::{json, Value};
use std::collections::BTreeMap;
pub(crate) fn read_tables(
    db: &Connection,
) -> Result<BTreeMap<String, Vec<BTreeMap<String, Value>>>, String> {
    let tx = db.unchecked_transaction().map_err(|_| "无法开始备份。")?;
    let mut tables = BTreeMap::new();
    for table in TABLES {
        let mut stmt = tx
            .prepare(&format!("SELECT * FROM {table} ORDER BY rowid"))
            .map_err(|_| "无法读取备份数据。")?;
        let names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
        let rows = stmt
            .query_map([], |r| {
                let mut item = BTreeMap::new();
                for (i, name) in names.iter().enumerate() {
                    let value =
                        match r.get_ref(i)? {
                            ValueRef::Null => Value::Null,
                            ValueRef::Integer(v) => json!(v),
                            ValueRef::Real(v) => json!(v),
                            ValueRef::Text(v) => json!(std::str::from_utf8(v)
                                .map_err(|_| rusqlite::Error::InvalidQuery)?),
                            ValueRef::Blob(_) => return Err(rusqlite::Error::InvalidQuery),
                        };
                    item.insert(name.clone(), value);
                }
                Ok(item)
            })
            .map_err(|_| "无法读取备份记录。")?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|_| "备份数据格式无效。")?;
        tables.insert(table.to_string(), rows);
    }
    tx.commit().map_err(|_| "无法完成备份读取。")?;
    Ok(tables)
}
pub(crate) fn replace(db: &Connection, backup: &Backup) -> Result<(), String> {
    let tx = db.unchecked_transaction().map_err(|_| "无法开始恢复。")?;
    for table in TABLES.iter().rev() {
        tx.execute(&format!("DELETE FROM {table}"), [])
            .map_err(|_| "无法清理待恢复数据。")?;
    }
    for table in TABLES {
        let mut stmt = tx
            .prepare(&format!("PRAGMA table_info({table})"))
            .map_err(|_| "无法检查表结构。")?;
        let columns = stmt
            .query_map([], |r| Ok((r.get::<_, String>(1)?, r.get::<_, String>(2)?)))
            .map_err(|_| "无法检查字段。")?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|_| "无法检查字段。")?;
        drop(stmt);
        let sql = format!(
            "INSERT INTO {table}({}) VALUES({})",
            columns
                .iter()
                .map(|(c, _)| c.as_str())
                .collect::<Vec<_>>()
                .join(","),
            vec!["?"; columns.len()].join(",")
        );
        for row in &backup.tables[table] {
            if row.len() != columns.len() || columns.iter().any(|(c, _)| !row.contains_key(c)) {
                return Err("备份字段与当前软件不匹配。".into());
            }
            let values = columns
                .iter()
                .map(|(c, kind)| match (&row[c], kind.as_str()) {
                    (Value::Null, _) => Ok(SqlValue::Null),
                    (Value::String(s), "TEXT") => {
                        if matches!(
                            c.as_str(),
                            "input_price"
                                | "output_price"
                                | "cache_read_price"
                                | "cache_write_price"
                                | "cost_cny"
                                | "usd_to_cny_rate"
                        ) {
                            let amount = s
                                .parse::<rust_decimal::Decimal>()
                                .map_err(|_| "备份金额无效。")?;
                            if amount.is_sign_negative()
                                || (c == "usd_to_cny_rate" && amount.is_zero())
                            {
                                return Err("备份金额无效。");
                            }
                        }
                        Ok(SqlValue::Text(s.clone()))
                    }
                    (Value::Number(n), "INTEGER") => n
                        .as_i64()
                        .filter(|v| *v >= 0)
                        .map(SqlValue::Integer)
                        .ok_or("备份数值无效。"),
                    _ => Err("备份字段类型无效。"),
                })
                .collect::<Result<Vec<_>, _>>()?;
            tx.execute(&sql, params_from_iter(values))
                .map_err(|_| "备份记录或数据关联无效，恢复已回滚。")?;
        }
    }
    crate::database::workspace::validate_state(&tx)?;
    crate::database::workspace::load(&tx).map_err(|_| "备份中的稿件或聊天数据无效。")?;
    for table in ["pricing_settings", "app_settings"] {
        if backup.tables[table].len() != 1 {
            return Err("备份缺少必要设置。".into());
        }
    }
    for (table, columns) in [
        ("providers", vec!["models_json"]),
        ("app_settings", vec!["preferences_json", "ui_state_json"]),
        ("models_dev_cache", vec!["content_json"]),
        ("usage_records", vec!["cost_details_json"]),
    ] {
        for column in columns {
            let invalid:bool=tx.query_row(&format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE {column} IS NOT NULL AND NOT json_valid({column}))"),[],|r|r.get(0)).map_err(|_|"无法检查备份内容。")?;
            if invalid {
                return Err("备份包含无效 JSON 数据。".into());
            }
        }
    }
    let invalid: bool = tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pragma_foreign_key_check)",
            [],
            |r| r.get(0),
        )
        .map_err(|_| "无法检查数据关系。")?;
    if invalid {
        return Err("备份数据关联无效。".into());
    }
    tx.commit().map_err(|_| "恢复失败，原数据已保留。".into())
}
