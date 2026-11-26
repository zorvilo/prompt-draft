use super::now;
use crate::models::provider::{Profile, ProviderKeyUpdate};
use rusqlite::{params, Connection, OptionalExtension};
pub fn load_key(connection: &Connection, profile: &Profile) -> rusqlite::Result<Option<String>> {
    connection
        .query_row(
            "SELECT nullif(api_key,'') FROM providers WHERE id=?1 AND endpoint=?2",
            params![profile.id, profile.endpoint],
            |r| r.get(0),
        )
        .optional()
        .map(Option::flatten)
}

pub fn save_key(connection: &Connection, profile: &Profile, key: &str) -> rusqlite::Result<()> {
    let tx = connection.unchecked_transaction()?;
    upsert_provider(&tx, profile)?;
    tx.execute(
        "UPDATE providers SET api_key=?1,updated_at=?4 WHERE id=?2 AND endpoint=?3 AND api_key IS NOT ?1",
        params![key, profile.id, profile.endpoint, now()],
    )?;
    tx.commit()
}

pub(super) fn update_key(db: &Connection, update: &ProviderKeyUpdate) -> Result<(), String> {
    let rows = db
        .execute(
            "UPDATE providers SET api_key=?1,updated_at=?4 WHERE id=?2 AND endpoint=?3",
            params![
                update.key.trim(),
                update.provider_id,
                update.endpoint,
                now()
            ],
        )
        .map_err(|_| "无法更新本地 API Key。")?;
    if rows != 1 {
        return Err("供应商或接口地址已变化，请重新保存配置。".into());
    }
    Ok(())
}

pub(super) fn upsert_provider(db: &Connection, item: &Profile) -> rusqlite::Result<()> {
    if item.is_default {
        db.execute(
            "UPDATE providers SET is_default=0 WHERE id<>?1 AND is_default=1",
            [&item.id],
        )?;
    }
    let models = item
        .catalog
        .as_ref()
        .map(|v| serde_json::to_string(v).unwrap());
    db.execute("INSERT INTO providers(id,name,preset_id,website,endpoint,default_model,models_json,created_at,updated_at,is_default) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?8,?9)
        ON CONFLICT(id) DO UPDATE SET name=excluded.name,preset_id=excluded.preset_id,website=excluded.website,endpoint=excluded.endpoint,default_model=excluded.default_model,models_json=excluded.models_json,is_default=excluded.is_default,
        api_key=CASE WHEN providers.endpoint=excluded.endpoint THEN providers.api_key ELSE '' END,updated_at=excluded.updated_at
        WHERE (name,preset_id,website,endpoint,default_model,models_json,is_default) IS NOT (excluded.name,excluded.preset_id,excluded.website,excluded.endpoint,excluded.default_model,excluded.models_json,excluded.is_default)",params![item.id,item.name,item.preset_id,item.website,item.endpoint,item.model,models,now(),item.is_default])?;
    Ok(())
}
