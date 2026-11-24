use crate::storage::Profile;
use reqwest::{Client, Url};
use serde_json::{json, Value};
use std::time::Duration;

pub fn client() -> Result<Client, reqwest::Error> {
    Client::builder()
        .timeout(Duration::from_secs(90))
        .redirect(reqwest::redirect::Policy::none())
        .build()
}

fn completion_url(endpoint: &str) -> Result<Url, String> {
    let mut url = Url::parse(endpoint.trim()).map_err(|_| "请输入有效的模型地址。")?;
    let local = match url.host() {
        Some(url::Host::Domain("localhost")) => true,
        Some(url::Host::Ipv4(ip)) => ip.is_loopback() || ip.is_private(),
        Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
        _ => false,
    };
    if !(url.scheme() == "https" || local && url.scheme() == "http")
        || url.host().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(
            "公网模型使用 HTTPS，本机和内网模型可使用 HTTP。地址中请勿加入密钥或查询参数。".into(),
        );
    }
    let path = format!("{}/chat/completions", url.path().trim_end_matches('/'));
    url.set_path(&path);
    Ok(url)
}

pub async fn fetch_models(
    client: &Client,
    endpoint: &str,
    key: &str,
) -> Result<Vec<String>, String> {
    let mut url = completion_url(endpoint)?;
    let base_path = url.path().trim_end_matches("/chat/completions");
    let path = format!("{base_path}/models");
    url.set_path(&path);
    let mut response = client
        .get(url)
        .bearer_auth(key)
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .map_err(|_| "获取模型失败，请检查网络或手动输入模型 ID。")?;
    if !response.status().is_success() {
        return Err(format!(
            "获取模型失败（HTTP {}）。请检查 API Key，也可手动输入模型 ID。",
            response.status().as_u16()
        ));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| "读取模型列表失败。")? {
        if bytes.len() + chunk.len() > 4 * 1024 * 1024 {
            return Err("模型目录过大，请手动输入模型 ID。".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    let response: Value =
        serde_json::from_slice(&bytes).map_err(|_| "平台返回了无法读取的模型列表。")?;
    let models = catalog_ids(&response);
    if models.is_empty() {
        return Err("平台未返回模型，请使用预设或手动输入模型 ID。".into());
    }
    Ok(models)
}

fn catalog_ids(response: &Value) -> Vec<String> {
    use std::collections::BTreeSet;
    response
        .get("data")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .chain(
            response
                .get("models")
                .and_then(Value::as_array)
                .into_iter()
                .flatten(),
        )
        .filter(|item| {
            ["input_modalities", "output_modalities"]
                .iter()
                .all(|field| {
                    item.get(*field)
                        .and_then(Value::as_array)
                        .is_none_or(|values| {
                            values.is_empty()
                                || values.iter().any(|value| value.as_str() == Some("text"))
                        })
                })
        })
        .filter_map(|item| {
            item.get("id")
                .or_else(|| item.get("slug"))
                .and_then(Value::as_str)
        })
        .map(str::trim)
        .filter(|id| !id.is_empty() && id.len() <= 200 && !id.chars().any(char::is_control))
        .take(2000)
        .map(str::to_owned)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub async fn complete<F: FnMut(&str)>(
    client: &Client,
    profile: &Profile,
    key: &str,
    messages: Value,
    mut on_delta: Option<F>,
    usage: &mut crate::usage::Usage,
) -> Result<String, String> {
    if profile.model.trim().is_empty() {
        return Err("请填写模型名称。".into());
    }
    let mut body = json!({
        "model": profile.model.trim(), "messages": messages, "stream": on_delta.is_some()
    });
    if matches!(
        (profile.endpoint.as_str(), profile.model.as_str()),
        (
            "https://api.deepseek.com",
            "deepseek-flash" | "deepseek-v4-pro"
        ) | ("https://open.bigmodel.cn/api/paas/v4", "glm-4.7-flash")
    ) {
        body["thinking"] = json!({"type": "disabled"});
    }
    let url = completion_url(&profile.endpoint)?;
    if on_delta.is_some() && matches!(url.host_str(), Some("api.deepseek.com" | "api.openai.com")) {
        body["stream_options"] = json!({"include_usage": true});
    }
    usage.started = true;
    let is_zhipu = url.host_str() == Some("open.bigmodel.cn");
    let mut request = client.post(url).json(&body);
    if !key.is_empty() {
        request = request.bearer_auth(key);
    }
    let response = request.send().await.map_err(|error| {
        if error.is_timeout() {
            "模型请求超时，可以重试。"
        } else {
            "无法连接模型，请检查地址和网络。"
        }
    })?;
    if !response.status().is_success() {
        let status = response.status().as_u16();
        if status == 429 && is_zhipu {
            let error = response.json::<Value>().await.unwrap_or(Value::Null);
            return Err(zhipu_limit_message(&error));
        }
        return Err(match status {
            401 | 403 => "鉴权失败，请检查 API Key 和模型权限。".into(),
            402 => "模型平台账户余额不足，请到开放平台检查余额。".into(),
            404 => "所选服务未找到预设模型，请检查账号权限或改用另一项服务。".into(),
            429 => "模型请求受限，请检查额度或稍后重试。".into(),
            status => format!("模型接口返回 HTTP {status}，请检查配置。"),
        });
    }
    if let Some(on_delta) = on_delta.as_mut() {
        let is_stream = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| {
                value
                    .split(';')
                    .next()
                    .is_some_and(|kind| kind.trim().eq_ignore_ascii_case("text/event-stream"))
            });
        if is_stream {
            return stream_completion(response, on_delta, usage).await;
        }
    }
    let body: Value = response
        .json()
        .await
        .map_err(|_| "模型接口返回了无法读取的数据。")?;
    usage.capture(&body);
    body.pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .filter(|content| !content.trim().is_empty())
        .map(str::to_owned)
        .ok_or_else(|| "模型没有返回文本，请检查模型配置。".into())
}

async fn stream_completion(
    mut response: reqwest::Response,
    mut on_delta: impl FnMut(&str),
    usage: &mut crate::usage::Usage,
) -> Result<String, String> {
    let mut buffered = Vec::new();
    let mut event = Vec::new();
    let mut output = String::new();
    let mut finished = false;
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "模型输出中断，请重试。")?
    {
        buffered.extend_from_slice(&chunk);
        let mut consumed = 0;
        for line in buffered.split_inclusive(|byte| *byte == b'\n') {
            let Some(line) = line.strip_suffix(b"\n") else {
                break;
            };
            consumed += line.len() + 1;
            let line = line.strip_suffix(b"\r").unwrap_or(line);
            if let Some(data) = line.strip_prefix(b"data:") {
                if !event.is_empty() {
                    event.push(b'\n');
                }
                event.extend_from_slice(data.strip_prefix(b" ").unwrap_or(data));
            } else if line.is_empty() && !event.is_empty() {
                if event == b"[DONE]" {
                    return finish_stream(output);
                }
                let value: Value =
                    serde_json::from_slice(&event).map_err(|_| "模型返回了无法读取的流式数据。")?;
                event.clear();
                usage.capture(&value);
                if value.get("error").is_some_and(|error| !error.is_null()) {
                    return Err("模型在生成过程中返回错误，请重试。".into());
                }
                if let Some(text) = value
                    .pointer("/choices/0/delta/content")
                    .and_then(Value::as_str)
                {
                    if output.len() + text.len() > 4 * 1024 * 1024 {
                        return Err("模型生成的内容过长，请缩小任务范围。".into());
                    }
                    output.push_str(text);
                    on_delta(text);
                }
                finished |= value
                    .pointer("/choices/0/finish_reason")
                    .and_then(Value::as_str)
                    .is_some();
            }
        }
        buffered.drain(..consumed);
        if buffered.len() + event.len() > 1024 * 1024 {
            return Err("模型返回的单条流式消息过长。".into());
        }
    }
    if !finished {
        return Err("模型输出未完整结束，请重试。".into());
    }
    finish_stream(output)
}

fn finish_stream(output: String) -> Result<String, String> {
    if output.trim().is_empty() {
        Err("模型没有返回文本，请检查模型配置。".into())
    } else {
        Ok(output)
    }
}

fn zhipu_limit_message(error: &Value) -> String {
    let code = error.pointer("/error/code").and_then(|value| {
        value.as_u64().or_else(|| {
            let code = value.as_str()?;
            (code.len() == 4 && code.bytes().all(|byte| byte.is_ascii_digit()))
                .then(|| code.parse::<u64>().ok())
                .flatten()
        })
    });
    let message = match code {
        Some(1113) => "智谱账户已欠费，请到开放平台检查账户余额。",
        Some(1302) => "智谱账户已达到请求频率限制，请稍后单独重试。",
        Some(1305) => "当前 GLM 模型繁忙，请稍后重试。",
        Some(1308 | 1310 | 1316..=1318) => "智谱用量已达上限，请到开放平台查看剩余额度和重置时间。",
        Some(1309) => "智谱编程套餐已到期，请检查订阅状态。",
        Some(1311) => "当前智谱套餐未开放此模型权限，请查看套餐支持的模型。",
        Some(1314 | 1315) => "此智谱企业套餐或 API Key 无法用于当前请求，请核对产品类型。",
        _ => "智谱拒绝了本次请求，请检查开放平台的用量、余额和模型权限。",
    };
    match code {
        Some(code @ 1000..=9999) => format!("{message}（HTTP 429，智谱 {code}）"),
        _ => format!("{message}（HTTP 429）"),
    }
}

#[cfg(test)]
mod tests {
    use super::{completion_url, zhipu_limit_message};
    use serde_json::json;

    #[test]
    fn fetches_catalog_from_openai_models_endpoint() {
        use std::io::{BufRead, BufReader, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}/v1", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .unwrap();
            let mut request = BufReader::new(&stream);
            let mut line = String::new();
            request.read_line(&mut line).unwrap();
            assert!(line.starts_with("GET /v1/models "));
            let mut authenticated = false;
            loop {
                line.clear();
                if request.read_line(&mut line).unwrap() == 0 || line == "\r\n" {
                    break;
                }
                authenticated |= line.eq_ignore_ascii_case("authorization: Bearer fixture-key\r\n");
            }
            assert!(authenticated);
            let body = r#"{"data":[{"id":"online-only-model"}]}"#;
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
        });
        let models = tauri::async_runtime::block_on(async {
            super::fetch_models(&super::client().unwrap(), &endpoint, "fixture-key").await
        })
        .unwrap();
        server.join().unwrap();
        assert_eq!(models, ["online-only-model"]);
    }

    #[test]
    fn streams_fragmented_utf8_sse_without_emitting_reasoning() {
        use std::io::{BufRead, BufReader, Read, Write};

        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let profile = crate::storage::Profile {
            is_default: false,
            id: "fixture".into(),
            name: "Fixture".into(),
            endpoint: format!("http://{}", listener.local_addr().unwrap()),
            model: "mock".into(),
            catalog: None,
            preset_id: None,
            website: None,
        };
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .unwrap();
            let mut request = BufReader::new(&stream);
            let mut length = 0;
            for line in request.by_ref().lines() {
                let line = line.unwrap();
                if line.is_empty() {
                    break;
                }
                if let Some(value) = line.strip_prefix("content-length:") {
                    length = value.trim().parse().unwrap();
                }
            }
            let mut body = vec![0; length];
            request.read_exact(&mut body).unwrap();
            let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(payload["stream"], true);

            let events = concat!(
                "data: {\"choices\":[{\"delta\":{\"reasoning_content\":\"内部推理\"}}]}\r\n\r\n",
                "data: {\"choices\":[{\"delta\":{\"content\":\"第\"}}]}\r\n\r\n",
                "data: {\"choices\":[{\"delta\":{\"content\":\"一行\"}}]}\n\n",
                r#"data: {"choices":[],"usage":{"prompt_tokens":12,"completion_tokens":3}}"#,
                "\n\n",
                "data: [DONE]\n\n",
            );
            let bytes = events.as_bytes();
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", bytes.len()).unwrap();
            let boundary = events.find('第').unwrap() + 1;
            stream.write_all(&bytes[..boundary]).unwrap();
            stream.flush().unwrap();
            std::thread::sleep(std::time::Duration::from_millis(20));
            stream.write_all(&bytes[boundary..]).unwrap();
        });
        let mut deltas = Vec::new();
        let mut usage = crate::usage::Usage::default();
        let result = tauri::async_runtime::block_on(super::complete(
            &super::client().unwrap(),
            &profile,
            "",
            json!([]),
            Some(|text: &str| deltas.push(text.to_owned())),
            &mut usage,
        ))
        .unwrap();
        server.join().unwrap();
        assert_eq!(deltas, ["第", "一行"]);
        assert_eq!(result, "第一行");
        let db = crate::storage::open(std::path::Path::new(":memory:")).unwrap();
        crate::usage::record(&db, &profile, "optimize", "success", 10, usage).unwrap();
        let report = crate::usage::stats(&db, 0, 0, None, "logs", false, None).unwrap();
        assert_eq!(report["summary"]["input"], 12);
        assert_eq!(report["summary"]["output"], 3);
    }

    #[test]
    fn catalog_accepts_both_response_shapes_and_deduplicates() {
        assert_eq!(
            super::catalog_ids(&json!({
                "data": [{"id": "qwen-plus"}, {"id": "qwen-plus"}, {"id": ""}, {"id": "audio-only", "output_modalities": ["audio"]}],
                "models": [{"slug": "glm-5.3"}, {"id": "bad\nvalue"}, {"other": "ignored"}]
            })),
            ["glm-5.3", "qwen-plus"]
        );
        assert!(super::catalog_ids(&json!({"error":{"message":"no"}})).is_empty());
    }

    #[test]
    fn distinguishes_zhipu_errors_without_exposing_response_text() {
        assert!(zhipu_limit_message(&json!({"error": {"code": "1113"}})).contains("欠费"));
        assert!(zhipu_limit_message(&json!({"error": {"code": 1305}})).contains("繁忙"));
        let unknown =
            zhipu_limit_message(&json!({"error": {"code": "secret", "message": "secret"}}));
        assert!(unknown.contains("HTTP 429"));
        assert!(!unknown.contains("secret"));
    }
    #[test]
    fn accepts_local_and_rejects_remote_http_or_credentials() {
        assert!(completion_url("http://127.0.0.1:11434/v1").is_ok());
        for address in [
            "http://10.147.103.23:9666/v1",
            "http://172.16.0.1/v1",
            "http://172.31.255.255/v1",
            "http://192.168.1.1/v1",
        ] {
            assert!(completion_url(address).is_ok(), "{address}");
        }
        for address in [
            "http://172.15.255.255/v1",
            "http://172.32.0.1/v1",
            "http://8.8.8.8/v1",
            "http://10.147.103.23:9666/v1?key=secret",
            "http://key@10.147.103.23:9666/v1",
        ] {
            assert!(completion_url(address).is_err(), "{address}");
        }
        assert_eq!(
            completion_url("http://10.147.103.23:9666/v1")
                .unwrap()
                .as_str(),
            "http://10.147.103.23:9666/v1/chat/completions"
        );
        assert!(completion_url("http://example.com/v1").is_err());
        assert!(completion_url("https://abc@example.com/v1").is_err());
    }
}
