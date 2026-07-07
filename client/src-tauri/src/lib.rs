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
        .setup(|app| {
            // Open the DB under the OS app-data dir (e.g. %APPDATA%\com.kiwi.character).
            let data_dir = app.path().app_data_dir()?;
            let db_path = data_dir.join("kiwi.db");
            let avatars_dir = data_dir.join("avatars");
            let db = db::Db::open(&db_path, avatars_dir)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            app.manage(AppState { db: Mutex::new(db) });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_characters,
            commands::get_character,
            commands::create_character,
            commands::set_favorite,
            commands::delete_character,
            commands::list_history,
            commands::list_messages,
            commands::send_message,
            commands::stream_message,
            commands::get_settings,
            commands::save_settings,
            commands::test_endpoint,
            commands::load_model,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
