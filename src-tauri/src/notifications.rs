use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

use crate::models::Character;


pub enum NotificationKind {
    ChatDone(Character, String)
}

pub fn notify_user(app: AppHandle, kind: NotificationKind) -> Result<(), String> {
    match kind {
        NotificationKind::ChatDone(character, content) => {
            return notify_chat_done(app, character, content);
        }
    }
}


pub fn notify_chat_done(
    app: AppHandle,
    character: Character,
    content: String
) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "Main widnow not found".to_string())?;

    let is_window_focused = window
        .is_focused()
        .map_err(|err| err.to_string())?;

    if is_window_focused {
        return Ok(());
    }


    let icon_path= if let Some(avatar) = character.avatar {
        avatar
    }
    else {
        String::new()
    };

    return app.notification()
        .builder()
        .title(format!("\"{}\" has finished typing", character.name))
        .icon(icon_path)
        .body(content)
        .show()
        .map_err(|err| err.to_string());
}