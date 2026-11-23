use rusqlite::{
    params_from_iter,
    types::{Value as SqlValue, ValueRef},
    Connection,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::BTreeMap, io::Read};
const TABLES: [&str; 12] = [
    "projects",
    "providers",
    "template_categories",
    "drafts",
    "draft_messages",
    "draft_dialogues",
    "templates",
    "usage_records",
    "model_prices",
    "pricing_settings",
    "app_settings",
    "models_dev_cache",
];
const LIMIT: usize = 256 * 1024 * 1024;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Backup {
    format: String,
    version: u32,
    application_version: String,
    schema: String,
    exported_at: u64,
    tables: BTreeMap<String, Vec<BTreeMap<String, Value>>>,
}
pub fn export(db: &Connection) -> Result<String, String> {
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
    let document = serde_json::to_string_pretty(&Backup {
        format: "prompt-draft-full-backup".into(),
        version: 1,
        application_version: env!("CARGO_PKG_VERSION").into(),
        schema: include_str!("schema.sql").into(),
        exported_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64,
        tables,
    })
    .map_err(|_| "无法生成备份。")?;
    if document.len() > LIMIT {
        return Err("完整备份超过 256 MB。".into());
    }
    Ok(document)
}
fn replace(db: &Connection, backup: &Backup) -> Result<(), String> {
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
    crate::storage::validate_state(&tx)?;
    crate::storage::load(&tx).map_err(|_| "备份中的稿件或聊天数据无效。")?;
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
fn parse(text: &str) -> Result<Backup, String> {
    if text.len() > LIMIT {
        return Err("备份文件超过 256 MB。".into());
    }
    let backup: Backup = serde_json::from_str(text).map_err(|_| "请选择有效的完整 JSON 备份。")?;
    if backup.format != "prompt-draft-full-backup"
        || backup.version != 1
        || backup.application_version != env!("CARGO_PKG_VERSION")
        || backup.schema != include_str!("schema.sql")
        || backup.tables.len() != TABLES.len()
        || TABLES.iter().any(|t| !backup.tables.contains_key(*t))
    {
        return Err("备份版本或数据结构与当前软件不一致。".into());
    }
    Ok(backup)
}
pub fn read(path: &str) -> Result<String, String> {
    let file = std::fs::File::open(path).map_err(|_| "无法打开完整备份文件。")?;
    let mut text = String::new();
    file.take((LIMIT + 1) as u64)
        .read_to_string(&mut text)
        .map_err(|_| "无法读取 UTF-8 备份文件。")?;
    if text.len() > LIMIT {
        return Err("备份文件超过 256 MB。".into());
    }
    Ok(text)
}
pub fn preview(text: &str) -> Result<Value, String> {
    let backup = parse(text)?;
    let staging =
        crate::storage::open(std::path::Path::new(":memory:")).map_err(|_| "无法校验备份。")?;
    replace(&staging, &backup)?;
    Ok(
        json!({"projects":backup.tables["projects"].len(),"drafts":backup.tables["drafts"].len(),"messages":backup.tables["draft_messages"].len(),"providers":backup.tables["providers"].len(),"usage":backup.tables["usage_records"].len(),"prices":backup.tables["model_prices"].len()}),
    )
}
pub fn restore(db: &Connection, text: &str) -> Result<(), String> {
    let backup = parse(text)?;
    replace(db, &backup)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn full_backup_roundtrip_and_failure_rollback() {
        let db = crate::storage::open(std::path::Path::new(":memory:")).unwrap();
        let workspace=crate::storage::parse(r#"{"schema":1,"projects":[{"id":"p","name":"项目"}],"drafts":[],"profiles":[],"activeProfile":null,"activeDraft":null,"theme":"dark"}"#).unwrap();
        crate::storage::save(&db, &workspace).unwrap();
        db.execute("INSERT INTO providers VALUES('account','测试','preset-custom',NULL,'https://example.com','model','fixture-key',NULL,1,2)",[]).unwrap();
        db.execute(
            "INSERT INTO drafts VALUES('d','p','草稿','原稿','结果',1,1,1,2)",
            [],
        )
        .unwrap();
        let user = json!({"id":"u","role":"user","content":"原稿","created":1,"context":{"mode":"draft","source":"原稿","templateId":"basic","templateName":"基础优化","systemPrompt":"模板"}}).to_string();
        let assistant =
            json!({"id":"a","role":"assistant","content":"结果","created":1}).to_string();
        db.execute("INSERT INTO draft_messages VALUES('u','d',0,?1)", [&user])
            .unwrap();
        db.execute(
            "INSERT INTO draft_messages VALUES('a','d',1,?1)",
            [&assistant],
        )
        .unwrap();
        db.execute(
            "INSERT INTO template_categories VALUES('custom','分类',1,2)",
            [],
        )
        .unwrap();
        db.execute("INSERT INTO templates VALUES('custom-t','custom','模板','说明','system','user','general',1,2)",[]).unwrap();
        db.execute("INSERT INTO usage_records VALUES(1,1,'account','测试','model','model','optimize','success',100,713,266,512,NULL,'example.com','0.0025','{}')",[]).unwrap();
        db.execute("INSERT INTO model_prices VALUES('example.com','model','override','1','2','0.1','0','manual',1)",[]).unwrap();
        db.execute("INSERT INTO models_dev_cache VALUES(1,'{}',123)", [])
            .unwrap();
        let text = export(&db).unwrap();
        assert!(text.contains("fixture-key"));
        assert_eq!(preview(&text).unwrap()["providers"], 1);
        let target = crate::storage::open(std::path::Path::new(":memory:")).unwrap();
        restore(&target, &text).unwrap();
        let a: Value = serde_json::from_str(&text).unwrap();
        let b: Value = serde_json::from_str(&export(&target).unwrap()).unwrap();
        assert_eq!(a["tables"], b["tables"]);
        let mut broken = a.clone();
        broken["tables"]["app_settings"][0]["preferences_json"] = json!("invalid");
        assert!(restore(&target, &broken.to_string()).is_err());
        assert_eq!(
            serde_json::from_str::<Value>(&export(&target).unwrap()).unwrap()["tables"],
            b["tables"]
        );
        broken = a.clone();
        broken["tables"]["draft_messages"][0]["draft_id"] = json!("missing");
        assert!(restore(&target, &broken.to_string()).is_err());
        assert_eq!(
            serde_json::from_str::<Value>(&export(&target).unwrap()).unwrap()["tables"],
            b["tables"]
        );
        broken = a.clone();
        broken["tables"]["usage_records"][0]["input_tokens"] = json!("not-a-number");
        assert!(restore(&target, &broken.to_string()).is_err());
        broken = a;
        broken["schema"] = json!("other");
        assert!(restore(&target, &broken.to_string()).is_err());
    }
}
