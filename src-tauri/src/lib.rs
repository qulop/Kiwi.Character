mod commands;
mod db;
mod models;
mod openai;
mod state;

use std::sync::Mutex;

use tauri::Manager;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(
            // Writes to stdout (visible in `tauri dev`) and to a rotating file
            // under the OS log dir (e.g. %APPDATA%\com.kiwi.character\logs on
            // Windows) — see `agent-docs`/README for the exact path per OS.
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .targets([
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout),
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir {
                        file_name: None,
                    }),
                ])
                .build(),
        )
        .setup(|app| {
            // Open the DB under the OS app-data dir (e.g. %APPDATA%\com.kiwi.character).
            let data_dir = app.path().app_data_dir()?;
            let db_path = data_dir.join("kiwi.db");
            let avatars_dir = data_dir.join("avatars");
            let db = db::Db::open(&db_path, avatars_dir)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            log::info!("Kiwi.Character starting; database at {}", db_path.display());
            app.manage(AppState { db: Mutex::new(db) });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_characters,
            commands::get_character,
            commands::create_character,
            commands::update_character,
            commands::character_name_available,
            commands::set_favorite,
            commands::delete_character,
            commands::list_personas,
            commands::create_persona,
            commands::update_persona,
            commands::delete_persona,
            commands::get_active_persona,
            commands::set_active_persona,
            commands::list_history,
            commands::list_messages,
            commands::send_message,
            commands::stream_message,
            commands::stream_continue,
            commands::update_message,
            commands::delete_message,
            commands::rewind_to_message,
            commands::delete_conversation,
            commands::get_settings,
            commands::save_settings,
            commands::test_endpoint,
            commands::loaded_models,
            commands::unload_model,
            commands::load_model,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
