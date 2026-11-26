use rust_decimal::Decimal;
use serde_json::Value;

pub(crate) fn calculate_charges(tokens: [i64; 4], prices: [String; 4]) -> Option<String> {
    let mut components = Vec::with_capacity(4);
    let mut total = Decimal::ZERO;
    for (tokens, price) in tokens.into_iter().zip(prices) {
        let amount = parse_decimal(&price)
            .and_then(|price| Decimal::from(tokens).checked_mul(price))?
            / Decimal::from(1_000_000);
        total = total.checked_add(amount)?;
        components.push(amount.to_string());
    }
    Some(
        serde_json::json!({
            "total": total.to_string(), "input": components[0], "output": components[1],
            "cacheRead": components[2], "cacheWrite": components[3]
        })
        .to_string(),
    )
}
pub(crate) fn parse_decimal(value: &str) -> Option<Decimal> {
    value
        .parse()
        .or_else(|_| Decimal::from_scientific(value))
        .ok()
}

pub(crate) fn endpoint_scope(endpoint: &str) -> String {
    let Ok(url) = url::Url::parse(endpoint) else {
        return String::new();
    };
    let Some(host) = url.host_str() else {
        return String::new();
    };
    let path = url.path().trim_end_matches('/');
    // DeepSeek documents /v1 as a compatibility alias for the same API.
    let path = if host == "api.deepseek.com" && path == "/v1" {
        ""
    } else {
        path
    };
    let port = url
        .port()
        .map(|port| format!(":{port}"))
        .unwrap_or_default();
    format!("{host}{port}{path}")
}

pub(crate) fn remote_quotes(remote: &Value) -> Result<Vec<(String, String, [Decimal; 4])>, String> {
    let providers = remote
        .as_object()
        .ok_or("models.dev 返回了无效价格目录。")?;
    let mut quotes = Vec::new();
    for (id, provider) in providers {
        let host = provider
            .get("api")
            .and_then(Value::as_str)
            .map(endpoint_scope)
            .filter(|h| !h.is_empty())
            .unwrap_or_else(|| {
                match id.as_str() {
                    "openai" => "api.openai.com/v1",
                    "deepseek" => "api.deepseek.com",
                    "anthropic" => "api.anthropic.com/v1",
                    "openrouter" => "openrouter.ai/api/v1",
                    "zhipuai" => "open.bigmodel.cn/api/paas/v4",
                    "moonshotai" => "api.moonshot.ai/v1",
                    _ => "",
                }
                .into()
            });
        if host.is_empty() {
            continue;
        }
        let Some(models) = provider.get("models").and_then(Value::as_object) else {
            continue;
        };
        for (model, entry) in models {
            let count = |key| {
                entry
                    .get("cost")
                    .and_then(|c| c.get(key))
                    .filter(|v| v.is_number())
                    .and_then(|v| parse_decimal(&v.to_string()))
                    .filter(|v| *v >= Decimal::ZERO)
            };
            let (Some(input), Some(output)) = (count("input"), count("output")) else {
                continue;
            };
            quotes.push((
                host.clone(),
                model.to_ascii_lowercase(),
                [
                    input,
                    output,
                    count("cache_read").unwrap_or(input),
                    count("cache_write").unwrap_or(input),
                ],
            ));
        }
    }
    if quotes.is_empty() {
        return Err("models.dev 未返回可匹配的报价。".into());
    }
    quotes.retain(|(_, model, _)| !matches!(model.as_str(), "deepseek-flash" | "deepseek-v4-pro"));
    Ok(quotes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn charges_preserve_decimal_precision_and_reject_invalid_or_overflowing_prices() {
        let text = calculate_charges([800, 500, 200, 0], ["2", "4", "0.5", "0"].map(String::from))
            .unwrap();
        let value: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(
            parse_decimal(value["total"].as_str().unwrap()),
            Some(Decimal::new(37, 4))
        );
        assert_eq!(
            parse_decimal(value["cacheRead"].as_str().unwrap()),
            Some(Decimal::new(1, 4))
        );
        assert!(calculate_charges([1; 4], ["bad", "0", "0", "0"].map(String::from)).is_none());
        assert!(calculate_charges(
            [2; 4],
            [Decimal::MAX.to_string(), "0".into(), "0".into(), "0".into()]
        )
        .is_none());
    }
}
