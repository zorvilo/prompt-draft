use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Backup {
    pub(crate) format: String,
    pub(crate) version: u32,
    pub(crate) application_version: String,
    pub(crate) schema: String,
    pub(crate) exported_at: u64,
    pub(crate) tables: BTreeMap<String, Vec<BTreeMap<String, Value>>>,
}

pub(crate) const TABLES: [&str; 12] = [
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

pub(crate) const LIMIT: usize = 256 * 1024 * 1024;
