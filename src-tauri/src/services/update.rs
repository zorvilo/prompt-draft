use crate::{
    desktop::updater::find_update,
    models::update::{AvailableUpdate, UpdateSource},
    state::AppState,
};
pub async fn check_update(
    app: tauri::AppHandle,
    source: Option<UpdateSource>,
    state: &AppState,
) -> Result<Option<AvailableUpdate>, String> {
    let mut slot = state
        .update
        .try_lock()
        .map_err(|_| "更新操作正在进行，请稍后重试。")?;
    if let Some((update, bytes)) = slot.as_ref() {
        if bytes.is_some() {
            return Ok(Some(AvailableUpdate {
                version: update.version.clone(),
                notes: update.body.clone(),
                downloaded: true,
            }));
        }
    }
    let found = if let Some(source) = source {
        find_update(&app, source).await?
    } else {
        match find_update(&app, UpdateSource::Gitee).await {
            Ok(found) => found,
            Err(_) => find_update(&app, UpdateSource::Github).await?,
        }
    };
    let info = found.as_ref().map(|update| AvailableUpdate {
        version: update.version.clone(),
        notes: update.body.clone(),
        downloaded: false,
    });
    *slot = found.map(|update| (update, None));
    Ok(info)
}

pub async fn download_update(
    app: tauri::AppHandle,
    source: UpdateSource,
    state: &AppState,
    on_progress: impl FnMut(serde_json::Value),
) -> Result<(), String> {
    let mut slot = state
        .update
        .try_lock()
        .map_err(|_| "更新操作正在进行，请稍后重试。")?;
    let (current, bytes) = slot.as_ref().ok_or("请先检查更新。")?;
    if bytes.is_some() {
        return Ok(());
    }
    let selected = find_update(&app, source)
        .await?
        .ok_or("所选来源尚未发布更新，请选择其他来源。")?;
    if selected.version != current.version {
        return Err("来源版本不一致，请重新检查或选择其他来源。".into());
    }
    *slot = Some((selected, None));
    let (update, bytes) = slot.as_mut().ok_or("请先检查更新。")?;
    let data = crate::desktop::updater::download(update, on_progress).await?;
    *bytes = Some(data);
    Ok(())
}

pub async fn install_update(state: &AppState) -> Result<(), String> {
    let _operation = state
        .operations
        .try_write()
        .map_err(|_| "请等待模型请求或数据操作结束后再安装更新。")?;
    let slot = state
        .update
        .try_lock()
        .map_err(|_| "更新操作正在进行，请稍后重试。")?;
    let (update, bytes) = slot.as_ref().ok_or("请先检查并下载更新。")?;
    let bytes = bytes.as_ref().ok_or("请先下载更新。")?;
    crate::desktop::updater::install(update, bytes)
}
