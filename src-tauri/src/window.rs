use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};
use tauri_plugin_autostart::ManagerExt;

pub fn show(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

pub fn create_tray(app: &tauri::App) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "打开词稿", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出程序", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &quit])?;
    let icon = app
        .default_window_icon()
        .ok_or_else(|| std::io::Error::other("缺少应用图标"))?;
    TrayIconBuilder::with_id("main")
        .icon(icon.clone())
        .tooltip("词稿")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show(app),
            "quit" => {
                show(app);
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.emit("request-exit", ());
                }
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                show(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WindowAction {
    Show,
    Hide,
    Exit,
}

#[tauri::command]
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

#[tauri::command]
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
