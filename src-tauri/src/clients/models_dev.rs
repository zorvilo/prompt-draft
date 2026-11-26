pub(crate) async fn fetch(client: &reqwest::Client) -> Result<serde_json::Value, String> {
    const LIMIT: usize = 32 * 1024 * 1024;
    let failed = |_| "无法连接 models.dev，请稍后重试。";
    let mut response = client
        .get("https://models.dev/api.json")
        .header(reqwest::header::ACCEPT, "application/json")
        .header(
            reqwest::header::USER_AGENT,
            concat!("PromptDraft/", env!("CARGO_PKG_VERSION")),
        )
        .timeout(std::time::Duration::from_secs(25))
        .send()
        .await
        .map_err(failed)?
        .error_for_status()
        .map_err(failed)?;
    let mut data = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(failed)? {
        if chunk.len() > LIMIT - data.len() {
            return Err("models.dev 价格目录超出大小限制。".into());
        }
        data.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&data).map_err(|_| "无法解析 models.dev 价格目录。".into())
}
