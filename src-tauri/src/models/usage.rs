use serde_json::Value;
#[derive(Default)]
pub struct Usage {
    pub started: bool,
    pub input: Option<i64>,
    pub(crate) output: Option<i64>,
    pub(crate) cache_read: Option<i64>,
    pub(crate) cache_write: Option<i64>,
    pub(crate) response_model: Option<String>,
}

impl Usage {
    pub fn capture(&mut self, body: &Value) {
        let count = |key| {
            body.pointer(key)
                .and_then(Value::as_i64)
                .filter(|n| *n >= 0)
        };
        self.input = count("/usage/prompt_tokens").or(self.input);
        self.output = count("/usage/completion_tokens").or(self.output);
        self.cache_read = count("/usage/prompt_tokens_details/cached_tokens")
            .or_else(|| count("/usage/prompt_cache_hit_tokens"))
            .or(self.cache_read);
        self.cache_write = count("/usage/prompt_tokens_details/cache_write_tokens")
            .or_else(|| count("/usage/cache_creation_input_tokens"))
            .or(self.cache_write);
        if let Some(model) = body
            .get("model")
            .and_then(Value::as_str)
            .filter(|m| !m.trim().is_empty() && m.len() <= 200)
        {
            self.response_model = Some(model.trim().into());
        }
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogPrice {
    pub(crate) host: String,
    pub(crate) model: String,
    pub(crate) input: String,
    pub(crate) output: String,
    pub(crate) cache_read: String,
    pub(crate) cache_write: String,
}
