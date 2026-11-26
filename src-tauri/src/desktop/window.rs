use tauri::{AppHandle, Manager};
pub fn show(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WindowAction {
    Show,
    Hide,
    Exit,
}

pub fn window_action(app: AppHandle, action: WindowAction) -> Result<(), String> {
    match action {
        WindowAction::Show => app
            .get_webview_window("main")
            .ok_or("窗口不可用")?
            .show()
            .map_err(|_| "无法显示窗口。".into()),
        WindowAction::Hide => {
            if app.tray_by_id("main").is_none() {
                return Err("系统托盘不可用，窗口未隐藏。请重试或选择退出程序。".into());
            }
            app.get_webview_window("main")
                .ok_or("窗口不可用")?
                .hide()
                .map_err(|_| "无法隐藏窗口，请重试。".into())
        }
        WindowAction::Exit => {
            app.exit(0);
            Ok(())
        }
    }
}
