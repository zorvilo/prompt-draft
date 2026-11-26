use crate::{clients::model, database, models::provider::Profile};
use rusqlite::Connection;
use std::{collections::HashMap, sync::Mutex};
use tokio::sync::oneshot;
pub struct AppState {
    pub(crate) database: Mutex<Connection>,
    pub(crate) operations: tokio::sync::RwLock<()>,
    pub(crate) requests: Mutex<HashMap<String, oneshot::Sender<()>>>,
    pub(crate) client: reqwest::Client,
    pub(crate) models_dev: Mutex<Option<serde_json::Value>>,
    pub(crate) models_dev_loading: tokio::sync::Mutex<()>,
    pub(crate) update: tokio::sync::Mutex<Option<(tauri_plugin_updater::Update, Option<Vec<u8>>)>>,
}

impl AppState {
    pub fn new(database: Connection) -> Result<Self, reqwest::Error> {
        Ok(Self {
            database: Mutex::new(database),
            operations: tokio::sync::RwLock::new(()),
            requests: Mutex::default(),
            client: model::client()?,
            models_dev: Mutex::new(None),
            models_dev_loading: tokio::sync::Mutex::new(()),
            update: tokio::sync::Mutex::new(None),
        })
    }

    pub(crate) fn key(&self, profile: &Profile) -> Result<String, String> {
        database::providers::load_key(&self.database.lock().unwrap(), profile)
            .map(Option::unwrap_or_default)
            .map_err(|_| "无法读取已保存的 API Key。".into())
    }
}
