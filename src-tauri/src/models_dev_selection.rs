use serde_json::{json, Value};
use std::collections::HashSet;

pub fn entries(data: &Value) -> Vec<Value> {
    let mut result = Vec::new();
    let Some(providers) = data.as_object() else {
        return result;
    };
    for (provider_id, provider) in providers {
        let Some(models) = provider["models"].as_object() else {
            continue;
        };
        for (id, model) in models {
            let name = model["name"].as_str().unwrap_or(id);
            let text = format!("{id} {name}").to_ascii_lowercase();
            if model["status"] == "deprecated"
                || [
                    "audio",
                    "deprecated",
                    "embedding",
                    "image",
                    "moderation",
                    "realtime",
                    "transcribe",
                    "tts",
                    "video",
                ]
                .iter()
                .any(|word| text.contains(word))
            {
                continue;
            }
            if let Some(output) = model
                .pointer("/modalities/output")
                .and_then(Value::as_array)
            {
                if !output.is_empty()
                    && (!output.iter().any(|v| v == "text")
                        || output
                            .iter()
                            .any(|v| ["audio", "image", "video"].iter().any(|kind| v == kind)))
                {
                    continue;
                }
            }
            let cost = &model["cost"];
            if !cost["input"].is_number() || !cost["output"].is_number() {
                continue;
            }
            result.push(json!({
                "key": format!("{provider_id}/{id}"), "provider": provider_id,
                "providerName": provider["name"].as_str().unwrap_or(provider_id),
                "model": id, "name": name, "date": model["release_date"].as_str().unwrap_or(""),
                "input": cost["input"], "output": cost["output"],
                "cacheRead": cost["cache_read"].as_f64().unwrap_or(0.0), "cacheWrite": cost["cache_write"].as_f64().unwrap_or(0.0)
            }));
        }
    }
    result.sort_by(|a, b| {
        b["date"]
            .as_str()
            .cmp(&a["date"].as_str())
            .then_with(|| a["key"].as_str().cmp(&b["key"].as_str()))
    });
    result
}
pub fn common(entries: &[Value]) -> HashSet<String> {
    let rules = [
        ("anthropic", "claude-"),
        ("openai", "gpt-"),
        ("google", "gemini-"),
        ("xai", "grok-"),
        ("deepseek", "deepseek-"),
        ("alibaba", "qwen"),
        ("xiaomi", "mimo-"),
        ("longcat", "longcat-"),
        ("moonshotai", "kimi-"),
        ("minimax-cn", "minimax-m"),
        ("zai", "glm-"),
    ];
    let mut result = HashSet::new();
    for (provider, prefix) in rules {
        for entry in entries
            .iter()
            .filter(|entry| {
                entry["provider"] == provider && {
                    let id = entry["model"].as_str().unwrap_or("").to_ascii_lowercase();
                    id.starts_with(prefix)
                        || provider == "openai"
                            && ["o1-", "o3-", "o4-"].iter().any(|p| id.starts_with(p))
                }
            })
            .take(6)
        {
            result.insert(entry["key"].as_str().unwrap().to_owned());
        }
    }
    result
}
pub fn selection(data: &Value, keys: &[String]) -> (Value, usize) {
    let entries = entries(data);
    let mut selected = json!({});
    let mut count = 0;
    for entry in entries {
        let key = entry["key"].as_str().unwrap();
        let include = keys.iter().any(|selected| selected == key);
        if !include {
            continue;
        }
        let provider = entry["provider"].as_str().unwrap();
        let model = entry["model"].as_str().unwrap();
        if selected.get(provider).is_none() {
            selected[provider] = json!({"api":data[provider]["api"],"models":{}});
        }
        selected[provider]["models"][model] = data[provider]["models"][model].clone();
        count += 1;
    }
    (selected, count)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_selection_does_not_add_common_or_unselected_models() {
        let data = json!({"deepseek":{"api":"https://api.deepseek.com","models":{
            "deepseek-flash":{"cost":{"input":1,"output":2}},
            "deepseek-other":{"cost":{"input":1,"output":2}},
            "embedding":{"cost":{"input":1,"output":2}}
        }}});
        let (selected, count) = selection(&data, &["deepseek/deepseek-flash".into()]);
        assert_eq!(count, 1);
        assert_eq!(selected["deepseek"]["models"].as_object().unwrap().len(), 1);
        assert_eq!(entries(&data).len(), 2);
        assert_eq!(selection(&data, &[]).1, 0);
    }
}
