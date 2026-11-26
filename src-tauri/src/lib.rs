mod clients;
mod commands;
mod database;
mod desktop;
mod error;
mod models;
mod services;
mod state;

use tauri::Manager;

pub fn run() {
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
            desktop::window::show(app)
        }))
    };
    builder
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            let directory = app.path().app_data_dir()?;
            #[cfg(debug_assertions)]
            let directory = std::env::var_os("PROMPT_DRAFT_TEST_DATA")
                .map(std::path::PathBuf::from)
                .unwrap_or(directory);
            std::fs::create_dir_all(&directory)?;
            app.manage(state::AppState::new(database::connection::open(
                &directory.join("prompt-draft.sqlite"),
            )?)?);
            let _ = desktop::tray::create_tray(app);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::desktop::autostart,
            commands::desktop::window_action,
            commands::desktop::support_info,
            commands::desktop::open_project_repository,
            commands::update::check_update,
            commands::update::download_update,
            commands::update::install_update,
            commands::workspace::load,
            commands::usage::usage_stats,
            commands::usage::usage_catalog_settings,
            commands::usage::models_dev_picker,
            commands::usage::import_models_dev_price,
            commands::usage::usage_catalog_rows,
            commands::usage::save_usage_catalog_price,
            commands::usage::update_usage_catalog,
            commands::workspace::save_changes,
            commands::providers::set_key,
            commands::providers::has_key,
            commands::providers::test_model,
            commands::providers::fetch_models,
            commands::optimization::optimize,
            commands::optimization::preview_template,
            commands::optimization::cancel,
            commands::backup::export_file,
            commands::backup::export_backup,
            commands::backup::preview_backup,
            commands::backup::restore_backup
        ])
        .run(tauri::generate_context!())
        .expect("启动词稿失败");
}
