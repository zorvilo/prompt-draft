pub(crate) mod backup;
pub(crate) mod catalog;
pub(crate) mod connection;
pub(crate) mod pricing;
pub(crate) mod providers;
pub(crate) mod usage;
pub(crate) mod workspace;

pub(crate) const SCHEMA: &str = include_str!("schema.sql");

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}
