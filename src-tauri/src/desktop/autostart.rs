use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;
pub fn autostart(app: AppHandle, enabled: Option<bool>) -> Result<bool, String> {
    let manager = app.autolaunch();
    let previous = manager
        .is_enabled()
        .map_err(|_| "无法读取开机自启状态，请重试。")?;
    if let Some(enabled) = enabled {
        if enabled != previous {
            let result = if enabled {
                manager.enable()
            } else {
                manager.disable()
            };
            if result.is_err() {
                let _ = if previous {
                    manager.enable()
                } else {
                    manager.disable()
                };
                return Err("无法修改开机自启设置，请检查系统权限后重试。".into());
            }
        }
    }
    match manager.is_enabled() {
        Ok(value) => Ok(value),
        Err(_) => {
            if enabled.is_some() {
                let _ = if previous {
                    manager.enable()
                } else {
                    manager.disable()
                };
            }
            Err("无法读取开机自启状态，请重试。".into())
        }
    }
}
