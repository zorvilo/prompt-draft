use crate::models::update::UpdateSource;
impl UpdateSource {
    pub(crate) fn endpoint(self) -> &'static str {
        match self {
            Self::Gitee => "https://gitee.com/liuliang6666/prompt-draft/raw/master/latest.json",
            Self::Github => {
                "https://github.com/zorvilo/prompt-draft/releases/latest/download/latest.json"
            }
        }
    }
}

pub(crate) async fn find_update(
    app: &tauri::AppHandle,
    source: UpdateSource,
) -> Result<Option<tauri_plugin_updater::Update>, String> {
    use tauri_plugin_updater::UpdaterExt;
    app.updater_builder()
        .endpoints(vec![source
            .endpoint()
            .parse()
            .map_err(|_| "更新地址无效。")?])
        .map_err(|_| "更新地址无效。")?
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|_| "无法初始化更新检查。")?
        .check()
        .await
        .map_err(|_| "检查更新失败，请检查网络或稍后重试。".into())
}

pub(crate) async fn download(
    update: &tauri_plugin_updater::Update,
    mut on_progress: impl FnMut(serde_json::Value),
) -> Result<Vec<u8>, String> {
    let mut downloaded = 0u64;
    update
        .download(
            |chunk, total| {
                downloaded += chunk as u64;
                on_progress(serde_json::json!({"downloaded":downloaded,"total":total}));
            },
            || {},
        )
        .await
        .map_err(|_| "下载或签名校验失败，请重试。".into())
}

pub(crate) fn install(update: &tauri_plugin_updater::Update, bytes: &[u8]) -> Result<(), String> {
    update
        .install(bytes)
        .map_err(|_| "安装更新失败，请稍后重试。".to_string())
}

#[cfg(test)]
mod update_source_tests {
    use super::UpdateSource;
    #[test]
    fn sources_are_explicit_and_unknown_sources_are_rejected() {
        for (name, host) in [("gitee", "gitee.com"), ("github", "github.com")] {
            let source: UpdateSource = serde_json::from_value(serde_json::json!(name)).unwrap();
            let url = url::Url::parse(source.endpoint()).unwrap();
            assert_eq!(url.scheme(), "https");
            assert_eq!(url.host_str(), Some(host));
        }
        assert!(
            serde_json::from_value::<UpdateSource>(serde_json::json!("https://example.com"))
                .is_err()
        );
    }
}
