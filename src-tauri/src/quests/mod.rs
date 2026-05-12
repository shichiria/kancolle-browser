use tauri::{AppHandle, Manager, Wry};

#[tauri::command]
pub async fn show_quests_window(app: AppHandle<Wry>) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("quests") {
        win.show().map_err(|e| e.to_string())?;
        win.set_focus().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn hide_quests_window(app: AppHandle<Wry>) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("quests") {
        win.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn toggle_quests_window(app: AppHandle<Wry>) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("quests") {
        if win.is_visible().unwrap_or(false) {
            win.hide().map_err(|e| e.to_string())?;
        } else {
            win.show().map_err(|e| e.to_string())?;
            win.set_focus().map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
