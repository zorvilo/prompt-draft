#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailableUpdate {
    pub(crate) version: String,
    pub(crate) notes: Option<String>,
    pub(crate) downloaded: bool,
}

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpdateSource {
    Gitee,
    Github,
}
