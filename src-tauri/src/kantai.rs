//! Kantai (fleet) window — show/hide commands.
//!
//! Same hide/show pattern as `management` to preserve React state across
//! toggles. The window loads the same React bundle as the management window
//! but at URL `index.html#kantai`, and the React app branches on
//! `window.location.hash` to render the kantai view.

use log::info;
use tauri::Manager;

const LABEL: &str = "kantai";

#[tauri::command]
pub(crate) fn show_kantai_window(app: tauri::AppHandle) -> Result<(), String> {
    crate::action_log::record("Command", "show_kantai_window", None);
    let win = app
        .get_window(LABEL)
        .ok_or_else(|| format!("Window `{}` not found", LABEL))?;
    win.show().map_err(|e| e.to_string())?;
    win.set_focus().map_err(|e| e.to_string())?;
    info!("Kantai window shown");
    Ok(())
}

#[tauri::command]
pub(crate) fn hide_kantai_window(app: tauri::AppHandle) -> Result<(), String> {
    crate::action_log::record("Command", "hide_kantai_window", None);
    let win = app
        .get_window(LABEL)
        .ok_or_else(|| format!("Window `{}` not found", LABEL))?;
    win.hide().map_err(|e| e.to_string())?;
    info!("Kantai window hidden");
    Ok(())
}

#[tauri::command]
pub(crate) fn toggle_kantai_window(app: tauri::AppHandle) -> Result<(), String> {
    crate::action_log::record("Command", "toggle_kantai_window", None);
    let win = app
        .get_window(LABEL)
        .ok_or_else(|| format!("Window `{}` not found", LABEL))?;
    let visible = win.is_visible().map_err(|e| e.to_string())?;
    if visible {
        win.hide().map_err(|e| e.to_string())?;
        info!("Kantai window hidden (toggle)");
    } else {
        win.show().map_err(|e| e.to_string())?;
        win.set_focus().map_err(|e| e.to_string())?;
        info!("Kantai window shown (toggle)");
    }
    Ok(())
}
