#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod backup;
mod commands;
mod model_client;
mod models_dev_selection;
mod optimizer;
mod storage;
mod usage;
mod window;

use tauri::Manager;

fn main() {
    let builder = tauri::Builder::default();
    // Isolated fixtures must not activate the user's running instance.
    #[cfg(debug_assertions)]
    let isolated = std::env::var_os("PROMPT_DRAFT_TEST_DATA").is_some();
    #[cfg(not(debug_assertions))]
    let isolated = false;
    let builder = if isolated {
        builder
    } else {
        builder.plugin(tauri_plugin_single_instance::init(|app, _, _| {
            window::show(app)
        }))
    };
    builder
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            let directory = app.path().app_data_dir()?;
            #[cfg(debug_assertions)]
            let directory = std::env::var_os("PROMPT_DRAFT_TEST_DATA")
                .map(std::path::PathBuf::from)
                .unwrap_or(directory);
            std::fs::create_dir_all(&directory)?;
            app.manage(commands::AppState::new(storage::open(
                &directory.join("prompt-draft.sqlite"),
            )?)?);
            let _ = window::create_tray(app);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            window::autostart,
            window::window_action,
            commands::load,
            commands::usage_stats,
            commands::usage_catalog_settings,
            commands::models_dev_picker,
            commands::import_models_dev_price,
            commands::usage_catalog_rows,
            commands::save_usage_catalog_price,
            commands::update_usage_catalog,
            commands::save,
            commands::save_changes,
            commands::set_key,
            commands::has_key,
            commands::test_model,
            commands::fetch_models,
            commands::optimize,
            commands::preview_template,
            commands::cancel,
            commands::export_file,
            commands::export_backup,
            commands::preview_backup,
            commands::restore_backup
        ])
        .run(tauri::generate_context!())
        .expect("启动词稿失败");
}
