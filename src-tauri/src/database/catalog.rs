use rusqlite::Connection;
use std::collections::HashSet;
pub(crate) fn priced_models(db: &Connection) -> Result<HashSet<String>, String> {
    let mut stmt = db
        .prepare("SELECT DISTINCT lower(model_id) FROM model_prices")
        .map_err(|_| "读取已有定价失败。")?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|_| "读取已有定价失败。")?
        .collect::<rusqlite::Result<_>>()
        .map_err(|_| "读取已有定价失败。")?;
    Ok(rows)
}
pub(crate) fn read_cache(db: &Connection) -> Option<String> {
    db.query_row(
        "SELECT content_json FROM models_dev_cache WHERE id=1",
        [],
        |row| row.get(0),
    )
    .ok()
}
pub(crate) fn save_cache(db: &Connection, data: &serde_json::Value) {
    let _=db.execute("INSERT INTO models_dev_cache(id,content_json,fetched_at) VALUES(1,?,unixepoch()*1000) ON CONFLICT(id) DO UPDATE SET content_json=excluded.content_json,fetched_at=excluded.fetched_at",[data.to_string()]);
}
pub(crate) fn set_sync_error(db: &Connection, error: Option<&str>) {
    let _ = db.execute(
        "UPDATE pricing_settings SET last_sync_error = ? WHERE id=1",
        [error],
    );
}
