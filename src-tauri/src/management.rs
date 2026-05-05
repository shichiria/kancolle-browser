//! Management window (React SPA) show/hide commands.
//!
//! The management window is created at startup with `visible: false` (see
//! `tauri.conf.json`). Users open it from the game window's control bar.
//! Closing the window via the title-bar `×` is intercepted to hide instead
//! (see `lib.rs` run handler), preserving React state across toggles.

use log::info;
use tauri::Manager;

const LABEL: &str = "management";

#[tauri::command]
pub(crate) fn show_management_window(app: tauri::AppHandle) -> Result<(), String> {
    crate::action_log::record("Command", "show_management_window", None);
    let win = app
        .get_window(LABEL)
        .ok_or_else(|| format!("Window `{}` not found", LABEL))?;
    win.show().map_err(|e| e.to_string())?;
    win.set_focus().map_err(|e| e.to_string())?;
    info!("Management window shown");
    Ok(())
}

#[tauri::command]
pub(crate) fn hide_management_window(app: tauri::AppHandle) -> Result<(), String> {
    crate::action_log::record("Command", "hide_management_window", None);
    let win = app
        .get_window(LABEL)
        .ok_or_else(|| format!("Window `{}` not found", LABEL))?;
    win.hide().map_err(|e| e.to_string())?;
    info!("Management window hidden");
    Ok(())
}

#[tauri::command]
pub(crate) fn toggle_management_window(app: tauri::AppHandle) -> Result<(), String> {
    crate::action_log::record("Command", "toggle_management_window", None);
    let win = app
        .get_window(LABEL)
        .ok_or_else(|| format!("Window `{}` not found", LABEL))?;
    let visible = win.is_visible().map_err(|e| e.to_string())?;
    if visible {
        win.hide().map_err(|e| e.to_string())?;
        info!("Management window hidden (toggle)");
    } else {
        win.show().map_err(|e| e.to_string())?;
        win.set_focus().map_err(|e| e.to_string())?;
        info!("Management window shown (toggle)");
    }
    Ok(())
}
