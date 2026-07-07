mod commands;
mod models;
mod openai;
mod state;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::seeded())
        .invoke_handler(tauri::generate_handler![
            commands::list_characters,
            commands::get_character,
            commands::create_character,
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
