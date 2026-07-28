use log::info;
use std::sync::atomic::Ordering;
use tauri::{Manager, State};

use crate::game_window::{CONTROL_BAR_HEIGHT, MACOS_TITLEBAR_HEIGHT};
use crate::AppState;

#[tauri::command]
pub(crate) fn set_formation_hint_enabled(
    app: tauri::AppHandle,
    state: State<AppState>,
    enabled: bool,
) -> Result<(), String> {
    state
        .prefs
        .formation_hint_enabled
        .store(enabled, Ordering::Relaxed);

    crate::settings::persist_flag(&app, crate::settings::FORMATION_HINT_ENABLED, enabled)?;

    // Hide hint window immediately when disabled
    if !enabled {
        crate::api::hide_formation_hint(&app);
    }

    info!("Formation hint set to {}", if enabled { "enabled" } else { "disabled" });
    Ok(())
}

#[tauri::command]
pub(crate) fn get_formation_hint_enabled(state: State<AppState>) -> bool {
    state.prefs.formation_hint_enabled.load(Ordering::Relaxed)
}

#[tauri::command]
pub(crate) fn set_taiha_alert_enabled(
    app: tauri::AppHandle,
    state: State<AppState>,
    enabled: bool,
) -> Result<(), String> {
    state.prefs.taiha_alert_enabled.store(enabled, Ordering::Relaxed);

    crate::settings::persist_flag(&app, crate::settings::TAIHA_ALERT_ENABLED, enabled)?;

    info!("Taiha alert set to {}", if enabled { "enabled" } else { "disabled" });
    Ok(())
}

#[tauri::command]
pub(crate) fn get_taiha_alert_enabled(state: State<AppState>) -> bool {
    state.prefs.taiha_alert_enabled.load(Ordering::Relaxed)
}

#[tauri::command]
pub(crate) fn set_battle_info_enabled(
    app: tauri::AppHandle,
    state: State<AppState>,
    enabled: bool,
) -> Result<(), String> {
    state
        .prefs
        .battle_info_enabled
        .store(enabled, Ordering::Relaxed);

    crate::settings::persist_flag(&app, crate::settings::BATTLE_INFO_ENABLED, enabled)?;

    if enabled {
        // Re-show overlay with stored data if available
        let stored = crate::lock_or_recover(
            &state.overlay.last_battle_info,
            "last_battle_info",
        )
        .clone();
        if let Some(data) = stored {
            info!("Battle info re-enabled, re-showing stored data");
            crate::api::battle_info::show_battle_info_overlay(&app, &data);
        }
    } else {
        // Hide but keep stored data (hide_battle_info_overlay clears it on port return)
        if let Some(win) = app.get_window("battle-info") {
            let _ = win.hide();
        }
    }

    info!("Battle info overlay set to {}", if enabled { "enabled" } else { "disabled" });
    Ok(())
}

#[tauri::command]
pub(crate) fn get_battle_info_enabled(state: State<AppState>) -> bool {
    state.prefs.battle_info_enabled.load(Ordering::Relaxed)
}

/// Show or hide the overlay webview.
#[tauri::command]
pub(crate) fn set_overlay_visible(app: tauri::AppHandle, visible: bool) -> Result<(), String> {
    let overlay = app
        .get_webview("game-overlay")
        .ok_or("Overlay not found")?;
    if visible {
        let win = app.get_window("game").ok_or("Game window not found")?;
        let size = win.inner_size().map_err(|e| e.to_string())?;
        overlay
            .set_position(tauri::LogicalPosition::new(0.0, 0.0))
            .map_err(|e| e.to_string())?;
        overlay.set_size(size).map_err(|e| e.to_string())?;
    } else {
        overlay
            .set_size(tauri::LogicalSize::new(1.0, 1.0))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Dismiss taiha overlay — restore minimap if active, otherwise hide overlay.
#[tauri::command]
pub(crate) fn dismiss_overlay(app: tauri::AppHandle, state: State<AppState>) -> Result<(), String> {
    let minimap_on = state.prefs.minimap_enabled.load(Ordering::Relaxed);
    if minimap_on {
        show_minimap_overlay(&app)?;
    } else {
        set_overlay_visible(app, false)?;
    }
    Ok(())
}

/// Minimap overlay defaults and constraints
pub(crate) const MINIMAP_DEFAULT_W: f64 = 310.0;
pub(crate) const MINIMAP_DEFAULT_H: f64 = 210.0;
const MINIMAP_MIN_W: f64 = 200.0;
const MINIMAP_MAX_W: f64 = 600.0;
const MINIMAP_MARGIN: f64 = 6.0;
/// Aspect ratio: 5:3 map + titlebar(18px) + footer(~24px) overhead
const MINIMAP_ASPECT: f64 = 0.68; // h/w ratio

/// Position overlay to minimap area (saved position or default bottom-right)
pub fn show_minimap_overlay(app: &tauri::AppHandle) -> Result<(), String> {
    let overlay = app.get_webview("game-overlay").ok_or_else(|| {
        log::warn!("[Minimap] game-overlay webview not found in show_minimap_overlay");
        "Overlay not found".to_string()
    })?;
    let win = app.get_window("game").ok_or_else(|| {
        log::warn!("[Minimap] game window not found in show_minimap_overlay");
        "Game window not found".to_string()
    })?;
    let phys = win.inner_size().map_err(|e| e.to_string())?;
    let scale = win.scale_factor().unwrap_or(1.0);
    let logical = phys.to_logical::<f64>(scale);

    let state = app.state::<AppState>();
    let (mw, mh) = *crate::lock_or_recover(&state.overlay.minimap_size, "minimap_size");
    let zoom = *crate::lock_or_recover(&state.overlay.game_zoom, "game_zoom");
    let bar_h = CONTROL_BAR_HEIGHT * zoom;

    let saved_pos =
        *crate::lock_or_recover(&state.overlay.minimap_position, "minimap_position");
    let (x, y) = match saved_pos {
        Some((sx, sy)) => {
            let x = sx.max(0.0).min(logical.width - mw);
            let y = sy.max(bar_h).min(logical.height - mh);
            (x, y)
        }
        None => {
            let x = logical.width - mw - MINIMAP_MARGIN;
            let y = logical.height - mh - MINIMAP_MARGIN;
            (x, y)
        }
    };

    overlay.set_position(tauri::LogicalPosition::new(x, y)).map_err(|e| e.to_string())?;
    overlay.set_size(tauri::LogicalSize::new(mw, mh)).map_err(|e| e.to_string())?;
    Ok(())
}

/// Toggle minimap on/off (called from game control bar)
#[tauri::command]
pub(crate) async fn toggle_minimap(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    game_state: State<'_, crate::api::models::GameState>,
) -> Result<bool, String> {
    let was_enabled = state.prefs.minimap_enabled.load(Ordering::Relaxed);
    let enabled = !was_enabled;
    state.prefs.minimap_enabled.store(enabled, Ordering::Relaxed);

    crate::settings::persist_flag(&app, crate::settings::MINIMAP_ENABLED, enabled)?;

    let overlay = app.get_webview("game-overlay").ok_or("Overlay not found")?;
    if enabled {
        // Immediately show minimap with current sortie data if in sortie
        let inner = game_state.inner.read().await;
        if let Some(sortie) = inner.sortie.battle_logger.active_sortie_ref() {
            crate::api::send_minimap_data(&app, sortie);
        }
        // If no active sortie, overlay stays 1x1 — nothing to show
    } else {
        let _ = overlay.eval("window.hideMinimap()");
        overlay.set_size(tauri::LogicalSize::new(1.0, 1.0)).map_err(|e| e.to_string())?;
    }
    Ok(enabled)
}

#[tauri::command]
pub(crate) fn get_minimap_enabled(state: State<AppState>) -> bool {
    state.prefs.minimap_enabled.load(Ordering::Relaxed)
}

/// Move minimap overlay by delta (called from overlay JS during drag)
#[tauri::command]
pub(crate) fn move_minimap(app: tauri::AppHandle, state: State<AppState>, dx: f64, dy: f64) -> Result<(), String> {
    let overlay = app.get_webview("game-overlay").ok_or("Overlay not found")?;
    let win = app.get_window("game").ok_or("Game window not found")?;
    let phys = win.inner_size().map_err(|e| e.to_string())?;
    let scale = win.scale_factor().unwrap_or(1.0);
    let logical = phys.to_logical::<f64>(scale);

    let (mw, mh) = *crate::lock_or_recover(&state.overlay.minimap_size, "minimap_size");
    let zoom = *crate::lock_or_recover(&state.overlay.game_zoom, "game_zoom");
    let bar_h = CONTROL_BAR_HEIGHT * zoom;

    let cur_pos = overlay.position().map_err(|e| e.to_string())?;
    let cur_logical = cur_pos.to_logical::<f64>(scale);

    let x = (cur_logical.x + dx).max(0.0).min(logical.width - mw);
    let y = (cur_logical.y + dy).max(bar_h).min(logical.height - mh);

    overlay.set_position(tauri::LogicalPosition::new(x, y)).map_err(|e| e.to_string())?;

    *crate::lock_or_recover(&state.overlay.minimap_position, "minimap_position") = Some((x, y));

    crate::settings::persist_json(&app, crate::settings::MINIMAP_POSITION, &(x, y))?;

    Ok(())
}

/// Resize minimap overlay (called from overlay JS during resize drag)
#[tauri::command]
pub(crate) fn resize_minimap(app: tauri::AppHandle, state: State<AppState>, w: f64) -> Result<(), String> {
    let new_w = w.clamp(MINIMAP_MIN_W, MINIMAP_MAX_W);
    let new_h = (new_w * MINIMAP_ASPECT).round();

    *crate::lock_or_recover(&state.overlay.minimap_size, "minimap_size") = (new_w, new_h);
    show_minimap_overlay(&app)?;

    crate::settings::persist_json(&app, crate::settings::MINIMAP_SIZE, &(new_w, new_h))?;

    Ok(())
}

/// Expedition notification window dimensions
const EXPEDITION_NOTIFY_W: f64 = 250.0;
const EXPEDITION_NOTIFY_ITEM_H: f64 = 18.0;
const EXPEDITION_NOTIFY_BASE_H: f64 = 28.0;
const EXPEDITION_NOTIFY_MARGIN: f64 = 8.0;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub(crate) struct ExpeditionNotifyItem {
    fleet_id: i32,
    mission_name: String,
}

/// Show expedition completion notification at top-right of game window
#[tauri::command]
pub(crate) fn show_expedition_notification(
    app: tauri::AppHandle,
    state: State<AppState>,
    notifications: Vec<ExpeditionNotifyItem>,
) -> Result<(), String> {
    let notify_win = app
        .get_window("expedition-notify")
        .ok_or("Notification window not found")?;
    let game_win = app.get_window("game").ok_or("Game window not found")?;

    let scale = game_win.scale_factor().unwrap_or(1.0);
    let phys_pos = game_win.inner_position().map_err(|e| e.to_string())?;
    let phys_size = game_win.inner_size().map_err(|e| e.to_string())?;

    let notify_h = EXPEDITION_NOTIFY_BASE_H + notifications.len() as f64 * EXPEDITION_NOTIFY_ITEM_H;
    let top_offset = MACOS_TITLEBAR_HEIGHT + CONTROL_BAR_HEIGHT + EXPEDITION_NOTIFY_MARGIN;

    let x = phys_pos.x + phys_size.width as i32
        - ((EXPEDITION_NOTIFY_W + EXPEDITION_NOTIFY_MARGIN) * scale) as i32;
    let y = phys_pos.y + (top_offset * scale) as i32;

    notify_win
        .set_position(tauri::PhysicalPosition::new(x, y))
        .map_err(|e| e.to_string())?;
    notify_win
        .set_size(tauri::LogicalSize::new(EXPEDITION_NOTIFY_W, notify_h))
        .map_err(|e| e.to_string())?;

    if let Some(wv) = app.get_webview("expedition-notify-content") {
        if let Err(e) = wv.set_size(tauri::LogicalSize::new(EXPEDITION_NOTIFY_W, notify_h)) {
            log::warn!("[ExpeditionNotify] failed to set webview size: {}", e);
        }
        let json = serde_json::to_string(&notifications).unwrap_or_default();
        if let Err(e) = wv.eval(format!("window.showNotifications({})", json)) {
            log::warn!("[ExpeditionNotify] failed to eval JS: {}", e);
        }
    } else {
        log::warn!("[ExpeditionNotify] expedition-notify-content webview not found");
    }

    if let Err(e) = notify_win.show() {
        log::warn!("[ExpeditionNotify] failed to show window: {}", e);
    }
    state
        .overlay
        .expedition_notify_visible
        .store(true, Ordering::Relaxed);
    Ok(())
}

/// Hide expedition completion notification
#[tauri::command]
pub(crate) fn hide_expedition_notification(app: tauri::AppHandle, state: State<AppState>) -> Result<(), String> {
    if let Some(win) = app.get_window("expedition-notify") {
        let _ = win.hide();
    }
    state
        .overlay
        .expedition_notify_visible
        .store(false, Ordering::Relaxed);
    Ok(())
}

/// Reposition expedition notification to follow the game window
pub(crate) fn reposition_expedition_notification(app: &tauri::AppHandle) {
    let state = app.state::<AppState>();
    if !state
        .overlay
        .expedition_notify_visible
        .load(Ordering::Relaxed)
    {
        return;
    }
    let game_win = match app.get_window("game") {
        Some(w) => w,
        None => return,
    };
    let notify_win = match app.get_window("expedition-notify") {
        Some(w) => w,
        None => return,
    };

    let scale = game_win.scale_factor().unwrap_or(1.0);
    let phys_pos = match game_win.inner_position() {
        Ok(p) => p,
        Err(_) => return,
    };
    let phys_size = match game_win.inner_size() {
        Ok(s) => s,
        Err(_) => return,
    };

    let top_offset = MACOS_TITLEBAR_HEIGHT + CONTROL_BAR_HEIGHT + EXPEDITION_NOTIFY_MARGIN;
    let x = phys_pos.x + phys_size.width as i32
        - ((EXPEDITION_NOTIFY_W + EXPEDITION_NOTIFY_MARGIN) * scale) as i32;
    let y = phys_pos.y + (top_offset * scale) as i32;

    if let Err(e) = notify_win.set_position(tauri::PhysicalPosition::new(x, y)) {
        log::warn!("[ExpeditionNotify] failed to reposition: {}", e);
    }
}

const EXERCISE_NOTIFY_W: f64 = 360.0;
const EXERCISE_NOTIFY_H: f64 = 70.0;
const EXERCISE_NOTIFY_MARGIN: f64 = 8.0;

/// Show the practice-refresh warning centered above the game.
pub(crate) fn show_exercise_notification(
    app: &tauri::AppHandle,
    minutes_remaining: i64,
) -> Result<(), String> {
    let notify_win = app
        .get_window("exercise-notify")
        .ok_or("Exercise notification window not found")?;
    let game_win = app.get_window("game").ok_or("Game window not found")?;

    let scale = game_win.scale_factor().unwrap_or(1.0);
    let phys_pos = game_win.inner_position().map_err(|e| e.to_string())?;
    let phys_size = game_win.inner_size().map_err(|e| e.to_string())?;
    let x = phys_pos.x
        + (phys_size.width as i32 - (EXERCISE_NOTIFY_W * scale) as i32) / 2;
    let top_offset = MACOS_TITLEBAR_HEIGHT + CONTROL_BAR_HEIGHT + EXERCISE_NOTIFY_MARGIN;
    let y = phys_pos.y + (top_offset * scale) as i32;

    notify_win
        .set_position(tauri::PhysicalPosition::new(x, y))
        .map_err(|e| e.to_string())?;
    notify_win
        .set_size(tauri::LogicalSize::new(
            EXERCISE_NOTIFY_W,
            EXERCISE_NOTIFY_H,
        ))
        .map_err(|e| e.to_string())?;

    if let Some(webview) = app.get_webview("exercise-notify-content") {
        webview
            .set_size(tauri::LogicalSize::new(
                EXERCISE_NOTIFY_W,
                EXERCISE_NOTIFY_H,
            ))
            .map_err(|e| e.to_string())?;
        webview
            .eval(format!(
                "window.showExerciseAlert({})",
                minutes_remaining.clamp(1, 15)
            ))
            .map_err(|e| e.to_string())?;
    }

    notify_win.show().map_err(|e| e.to_string())?;
    app.state::<AppState>()
        .overlay
        .exercise_notify_visible
        .store(true, Ordering::Relaxed);
    Ok(())
}

pub(crate) fn hide_exercise_notification(app: &tauri::AppHandle) {
    if let Some(window) = app.get_window("exercise-notify") {
        let _ = window.hide();
    }
    app.state::<AppState>()
        .overlay
        .exercise_notify_visible
        .store(false, Ordering::Relaxed);
}

pub(crate) fn reposition_exercise_notification(app: &tauri::AppHandle) {
    if !app
        .state::<AppState>()
        .overlay
        .exercise_notify_visible
        .load(Ordering::Relaxed)
    {
        return;
    }

    let Some(game_win) = app.get_window("game") else {
        return;
    };
    let Some(notify_win) = app.get_window("exercise-notify") else {
        return;
    };
    let Ok(phys_pos) = game_win.inner_position() else {
        return;
    };
    let Ok(phys_size) = game_win.inner_size() else {
        return;
    };
    let scale = game_win.scale_factor().unwrap_or(1.0);
    let x = phys_pos.x
        + (phys_size.width as i32 - (EXERCISE_NOTIFY_W * scale) as i32) / 2;
    let top_offset = MACOS_TITLEBAR_HEIGHT + CONTROL_BAR_HEIGHT + EXERCISE_NOTIFY_MARGIN;
    let y = phys_pos.y + (top_offset * scale) as i32;
    let _ = notify_win.set_position(tauri::PhysicalPosition::new(x, y));
}

/// Reposition the battle-info overlay to follow the game window.
/// Mirrors the positioning math in `battle_info::show_battle_info_overlay` so
/// that move/resize events keep the overlay anchored to the top-left of the
/// game canvas, just below the control bar.
pub(crate) fn reposition_battle_info(app: &tauri::AppHandle) {
    let battle_info_win = match app.get_window("battle-info") {
        Some(w) => w,
        None => return,
    };
    // Only reposition if currently shown — avoids moving a hidden window.
    if !battle_info_win.is_visible().unwrap_or(false) {
        return;
    }
    let game_win = match app.get_window("game") {
        Some(w) => w,
        None => return,
    };

    let scale = game_win.scale_factor().unwrap_or(1.0);
    let inner_pos = match game_win.inner_position() {
        Ok(p) => p,
        Err(_) => return,
    };
    let zoom = app
        .try_state::<AppState>()
        .map(|s| *crate::lock_or_recover(&s.overlay.game_zoom, "game_zoom"))
        .unwrap_or(1.0);
    let bar_h = CONTROL_BAR_HEIGHT * zoom;
    let margin = 8.0;

    let x = inner_pos.x + (margin * scale) as i32;
    let y = inner_pos.y + ((bar_h + margin) * scale) as i32;

    if let Err(e) = battle_info_win.set_position(tauri::PhysicalPosition::new(x, y)) {
        log::warn!("[BattleInfo] failed to reposition: {}", e);
    }
}

/// Reposition the formation hint window to follow the game window
pub(crate) fn reposition_formation_hint(app: &tauri::AppHandle) {
    let state = app.state::<AppState>();
    let rect = *crate::lock_or_recover(
        &state.overlay.formation_hint_rect,
        "formation_hint_rect",
    );
    if !rect.visible {
        return;
    }
    let game_win = match app.get_window("game") {
        Some(w) => w,
        None => return,
    };
    let hint_win = match app.get_window("formation-hint") {
        Some(w) => w,
        None => return,
    };
    let inner_pos = match game_win.inner_position() {
        Ok(p) => p,
        Err(_) => return,
    };
    let screen_x = inner_pos.x + rect.dx;
    let screen_y = inner_pos.y + rect.dy;
    if let Err(e) = hint_win.set_position(tauri::PhysicalPosition::new(screen_x, screen_y)) {
        log::warn!("[FormationHint] failed to reposition: {}", e);
    }
}
