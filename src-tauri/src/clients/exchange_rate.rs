pub(crate) async fn fetch(client: &reqwest::Client) -> Option<(String, String)> {
    let body = client
        .get("https://api.frankfurter.dev/v2/rate/usd/cny")
        .timeout(std::time::Duration::from_secs(8))
        .header(reqwest::header::ACCEPT, "application/json")
        .header(
            reqwest::header::USER_AGENT,
            concat!("PromptDraft/", env!("CARGO_PKG_VERSION")),
        )
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?
        .json::<serde_json::Value>()
        .await
        .ok()?;
    let date = body["date"].as_str()?;
    if body["base"] != "USD"
        || body["quote"] != "CNY"
        || date.len() != 10
        || !date.bytes().enumerate().all(|(i, b)| {
            if i == 4 || i == 7 {
                b == b'-'
            } else {
                b.is_ascii_digit()
            }
        })
        || !body["rate"]
            .as_f64()
            .is_some_and(|value| value > 0.0 && value <= 100.0)
    {
        return None;
    }
    Some((body["rate"].to_string(), date.to_owned()))
}
