use crate::{
    database,
    models::backup::{Backup, LIMIT, TABLES},
    state::AppState,
};
use database::backup::replace;
use rusqlite::Connection;
use serde_json::{json, Value};
use std::io::Read;
pub(crate) fn export(db: &Connection) -> Result<String, String> {
    let tables = database::backup::read_tables(db)?;
    let document = serde_json::to_string_pretty(&Backup {
        format: "prompt-draft-full-backup".into(),
        version: 1,
        application_version: env!("CARGO_PKG_VERSION").into(),
        schema: database::SCHEMA.into(),
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

fn parse(text: &str) -> Result<Backup, String> {
    if text.len() > LIMIT {
        return Err("备份文件超过 256 MB。".into());
    }
    let backup: Backup = serde_json::from_str(text).map_err(|_| "请选择有效的完整 JSON 备份。")?;
    if backup.format != "prompt-draft-full-backup"
        || backup.version != 1
        || backup.application_version != env!("CARGO_PKG_VERSION")
        || backup.schema != database::SCHEMA
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
    let staging = crate::database::connection::open(std::path::Path::new(":memory:"))
        .map_err(|_| "无法校验备份。")?;
    replace(&staging, &backup)?;
    Ok(
        json!({"projects":backup.tables["projects"].len(),"drafts":backup.tables["drafts"].len(),"messages":backup.tables["draft_messages"].len(),"providers":backup.tables["providers"].len(),"usage":backup.tables["usage_records"].len(),"prices":backup.tables["model_prices"].len()}),
    )
}

pub fn restore(db: &Connection, text: &str) -> Result<(), String> {
    let backup = parse(text)?;
    replace(db, &backup)
}

pub async fn export_file(path: String, content: String) -> Result<(), String> {
    std::fs::write(path, content).map_err(|_| "导出失败，请检查文件路径和写入权限。".into())
}

pub fn export_backup(state: &AppState, path: String) -> Result<(), String> {
    let _guard = state
        .operations
        .try_write()
        .map_err(|_| "请等待模型请求或价格同步完成后再导出。")?;
    let document = export(&state.database.lock().unwrap())?;
    std::fs::write(path, document).map_err(|_| "无法保存完整备份文件。".into())
}

pub fn preview_backup(path: String) -> Result<serde_json::Value, String> {
    preview(&read(&path)?)
}

pub fn restore_backup(state: &AppState, path: String) -> Result<(), String> {
    let _guard = state
        .operations
        .try_write()
        .map_err(|_| "请等待模型请求或价格同步完成后再导入。")?;
    let text = read(&path)?;
    restore(&state.database.lock().unwrap(), &text)?;
    *state.models_dev.lock().unwrap() = None;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn full_backup_roundtrip_and_failure_rollback() {
        let db = crate::database::connection::open(std::path::Path::new(":memory:")).unwrap();
        let workspace=crate::database::workspace::parse(r#"{"schema":1,"projects":[{"id":"p","name":"项目"}],"drafts":[],"profiles":[],"activeProfile":null,"activeDraft":null,"theme":"dark"}"#).unwrap();
        crate::database::workspace::save(&db, &workspace).unwrap();
        db.execute("INSERT INTO providers VALUES('account','测试','preset-custom',NULL,'https://example.com',0,'model','fixture-key',NULL,1,2)",[]).unwrap();
        db.execute(
            "INSERT INTO drafts VALUES('d','p',NULL,NULL,NULL,'草稿','原稿','结果',1,1,1,2)",
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
        db.execute(
            "INSERT INTO templates VALUES('custom-t','custom','模板','说明','system','user',1,2)",
            [],
        )
        .unwrap();
        db.execute("INSERT INTO usage_records VALUES(1,1,'account','测试','model','model','optimize','success',100,713,266,512,NULL,'example.com','0.0025','{}')",[]).unwrap();
        db.execute("INSERT INTO model_prices VALUES('example.com','model','override','1','2','0.1','0','manual',1)",[]).unwrap();
        db.execute("INSERT INTO models_dev_cache VALUES(1,'{}',123)", [])
            .unwrap();
        let text = export(&db).unwrap();
        assert!(text.contains("fixture-key"));
        assert_eq!(preview(&text).unwrap()["providers"], 1);
        let target = crate::database::connection::open(std::path::Path::new(":memory:")).unwrap();
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
