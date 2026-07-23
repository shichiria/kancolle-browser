use std::collections::HashMap;
use std::path::Path;
use tauri::{AppHandle, Manager};

pub(super) fn load_memory(path: &Path) -> HashMap<String, i32> {
    match std::fs::read_to_string(path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
        Err(_) => HashMap::new(),
    }
}

pub(super) fn save_memory(path: &Path, memory: &HashMap<String, i32>) {
    if let Ok(json) = serde_json::to_string_pretty(memory) {
        let _ = std::fs::write(path, json);
    }
}

/// Get Japanese name for a formation ID
pub(crate) fn formation_name(id: i32) -> &'static str {
    match id {
        1 => "単縦陣",
        2 => "複縦陣",
        3 => "輪形陣",
        4 => "梯形陣",
        5 => "単横陣",
        6 => "警戒陣",
        11 => "第一警戒航行序列(対潜警戒)",
        12 => "第二警戒航行序列(前方警戒)",
        13 => "第三警戒航行序列(輪形陣)",
        14 => "第四警戒航行序列(戦闘隊形)",
        _ => "不明",
    }
}

/// Formation button label rect in game canvas (1200x720) coordinates: (x, y, w, h)
/// Positions and sizes are derived from the sally_jin sprite atlas and kcauto reference data.
fn get_formation_button_rect(formation: i32, _ship_count: usize) -> Option<(f64, f64, f64, f64)> {
    // Standard labels are 150x48. The longer combined-fleet labels use
    // separate 210x45 sprites; using the standard width shifts the visible
    // highlight into the right-hand portion of those buttons.
    let (cx, cy, bw, bh) = match formation {
        1 => (663.0, 278.0, 154.0, 48.0),  // 単縦陣 col1 row1
        2 => (858.0, 278.0, 154.0, 48.0),  // 複縦陣 col2 row1
        3 => (1056.0, 278.0, 154.0, 48.0), // 輪形陣 col3 row1
        4 => (766.0, 517.0, 154.0, 48.0),  // 梯形陣 col1 row2
        5 => (960.0, 517.0, 154.0, 48.0),  // 単横陣 col2 row2
        6 => (1048.0, 517.0, 154.0, 48.0), // 警戒陣 col3 row2
        // Combined-fleet label sprites are 210x45; add the same 2px
        // horizontal margin used by the standard formation highlight.
        11 => (743.0, 263.0, 214.0, 45.0), // 第一警戒航行序列
        12 => (993.0, 263.0, 214.0, 45.0), // 第二警戒航行序列
        13 => (743.0, 468.0, 214.0, 45.0), // 第三警戒航行序列
        14 => (993.0, 468.0, 214.0, 45.0), // 第四警戒航行序列
        _ => return None,
    };

    Some((cx - bw / 2.0, cy - bh / 2.0, bw, bh))
}

/// Show formation highlight using the click-through formation-hint window
pub(crate) fn show_formation_hint(app: &AppHandle, formation: i32, ship_count: usize) {
    log::info!(
        "[FormationHint] show: formation={} ({}), ships={}",
        formation,
        formation_name(formation),
        ship_count
    );

    // Check if formation hint is enabled
    if let Some(state) = app.try_state::<crate::AppState>() {
        if !state
            .prefs
            .formation_hint_enabled
            .load(std::sync::atomic::Ordering::Relaxed)
        {
            log::info!("[FormationHint] disabled, skipping");
            return;
        }
    }

    let game_win = match app.get_window("game") {
        Some(w) => w,
        None => {
            log::warn!("[FormationHint] game window not found");
            return;
        }
    };
    let hint_win = match app.get_window("formation-hint") {
        Some(w) => w,
        None => {
            log::warn!("[FormationHint] formation-hint window not found");
            return;
        }
    };

    let (bx, by, bw, bh) = match get_formation_button_rect(formation, ship_count) {
        Some(r) => r,
        None => {
            log::warn!("[FormationHint] no button rect for formation={}", formation);
            return;
        }
    };

    let inner_pos = match game_win.inner_position() {
        Ok(p) => p,
        Err(e) => {
            log::warn!("[FormationHint] failed to get game window position: {}", e);
            return;
        }
    };
    let scale = game_win.scale_factor().unwrap_or(1.0);

    // Get current game zoom level
    let zoom = app
        .try_state::<crate::AppState>()
        .map(|s| *crate::lock_or_recover(&s.overlay.game_zoom, "game_zoom"))
        .unwrap_or(1.0);

    // Control bar is 28 CSS pixels, scaled by zoom and DPI
    // Game coordinates are also scaled by zoom
    let dx = (bx * zoom * scale) as i32;
    let dy = ((28.0 + by) * zoom * scale) as i32;

    // macOS: adjust for platform-specific coordinate offset
    #[cfg(target_os = "macos")]
    {
        dx += (6.0 * scale) as i32;
        dy += (30.0 * scale) as i32;
    }
    let phys_w = (bw * zoom * scale) as u32;
    let phys_h = (bh * zoom * scale) as u32;

    // Save offset in AppState for window-move tracking
    if let Some(app_state) = app.try_state::<crate::AppState>() {
        let mut rect = crate::lock_or_recover(
            &app_state.overlay.formation_hint_rect,
            "formation_hint_rect",
        );
        rect.dx = dx;
        rect.dy = dy;
        rect.w = phys_w;
        rect.h = phys_h;
        rect.visible = true;
    }

    let screen_x = inner_pos.x + dx;
    let screen_y = inner_pos.y + dy;

    // Also check outer_position and game webview position for debugging
    let outer_pos = game_win.outer_position().ok();
    let win_size = game_win.inner_size().ok();
    log::info!(
        "FormationHint: formation={}, ship_count={}, scale={}, inner_pos=({},{}), outer_pos={:?}, win_size={:?}, dx={}, dy={}, screen=({},{}), rect={}x{}",
        formation, ship_count, scale, inner_pos.x, inner_pos.y, outer_pos, win_size, dx, dy, screen_x, screen_y, phys_w, phys_h
    );

    if let Err(e) = hint_win.set_size(tauri::PhysicalSize::new(phys_w, phys_h)) {
        log::warn!("[FormationHint] failed to set window size: {}", e);
    }
    if let Some(wv) = app.get_webview("formation-hint-content") {
        if let Err(e) = wv.set_size(tauri::PhysicalSize::new(phys_w, phys_h)) {
            log::warn!("[FormationHint] failed to set webview size: {}", e);
        }
    } else {
        log::warn!("[FormationHint] formation-hint-content webview not found");
    }
    if let Err(e) = hint_win.set_position(tauri::PhysicalPosition::new(screen_x, screen_y)) {
        log::warn!("[FormationHint] failed to set position: {}", e);
    }
    if let Err(e) = hint_win.show() {
        log::warn!("[FormationHint] failed to show window: {}", e);
    }
}

/// Hide formation hint window
pub fn hide_formation_hint(app: &AppHandle) {
    log::debug!("[FormationHint] hiding");
    if let Some(app_state) = app.try_state::<crate::AppState>() {
        crate::lock_or_recover(
            &app_state.overlay.formation_hint_rect,
            "formation_hint_rect",
        )
        .visible = false;
    }
    if let Some(hint_win) = app.get_window("formation-hint") {
        if let Err(e) = hint_win.hide() {
            log::warn!("[FormationHint] failed to hide window: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::get_formation_button_rect;

    #[test]
    fn standard_formation_uses_standard_label_size() {
        let (_, _, width, height) = get_formation_button_rect(1, 6).unwrap();
        assert_eq!((width, height), (154.0, 48.0));
    }

    #[test]
    fn combined_fleet_formation_covers_the_long_label() {
        let (x, y, width, height) = get_formation_button_rect(13, 6).unwrap();
        assert_eq!((x, y, width, height), (636.0, 445.5, 214.0, 45.0));
    }
}
