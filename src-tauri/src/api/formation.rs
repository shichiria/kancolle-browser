use tauri::{AppHandle, Manager};

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
/// Positions derived from sprite atlas (sally_jin.json: label=150x48) and kcauto reference data.
/// Grid layout: 3 columns (x=668,866,1064) x 2 rows (y=278,517), same for all ship counts.
fn get_formation_button_rect(formation: i32, _ship_count: usize) -> Option<(f64, f64, f64, f64)> {
    // Label sprite is 150x48 in sally_jin atlas; yellow border is slightly wider
    const BW: f64 = 154.0;
    const BH: f64 = 48.0;

    // Button center positions in game canvas coordinates
    let (cx, cy) = match formation {
        1 => (663.0, 278.0),   // 単縦陣 col1 row1
        2 => (858.0, 278.0),   // 複縦陣 col2 row1
        3 => (1056.0, 278.0),  // 輪形陣 col3 row1
        4 => (766.0, 517.0),   // 梯形陣 col1 row2
        5 => (960.0, 517.0),   // 単横陣 col2 row2
        6 => (1048.0, 517.0),  // 警戒陣 col3 row2
        // Combined fleet formations
        11 => (743.0, 263.0),  // 第一警戒航行序列
        12 => (993.0, 263.0),  // 第二警戒航行序列
        13 => (743.0, 468.0),  // 第三警戒航行序列
        14 => (993.0, 468.0),  // 第四警戒航行序列
        _ => return None,
    };

    Some((cx - BW / 2.0, cy - BH / 2.0, BW, BH))
}

/// Show formation highlight using the click-through formation-hint window
pub(crate) fn show_formation_hint(app: &AppHandle, formation: i32, ship_count: usize) {
    // Check if formation hint is enabled
    if let Some(state) = app.try_state::<crate::AppState>() {
        if !state.formation_hint_enabled.load(std::sync::atomic::Ordering::Relaxed) {
            return;
        }
    }

    let game_win = match app.get_window("game") {
        Some(w) => w,
        None => return,
    };
    let hint_win = match app.get_window("formation-hint") {
        Some(w) => w,
        None => return,
    };

    let (bx, by, bw, bh) = match get_formation_button_rect(formation, ship_count) {
        Some(r) => r,
        None => return,
    };

    let inner_pos = match game_win.inner_position() {
        Ok(p) => p,
        Err(_) => return,
    };
    let scale = game_win.scale_factor().unwrap_or(1.0);

    // Get current game zoom level
    let zoom = app.try_state::<crate::AppState>()
        .map(|s| *s.game_zoom.lock().unwrap())
        .unwrap_or(1.0);

    // Control bar is 28 CSS pixels, scaled by zoom and DPI
    // Game coordinates are also scaled by zoom
    let mut dx = (bx * zoom * scale) as i32;
    let mut dy = ((28.0 + by) * zoom * scale) as i32;

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
        let mut rect = app_state.formation_hint_rect.lock().unwrap();
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

    let _ = hint_win.set_size(tauri::PhysicalSize::new(phys_w, phys_h));
    if let Some(wv) = app.get_webview("formation-hint-content") {
        let _ = wv.set_size(tauri::PhysicalSize::new(phys_w, phys_h));
    }
    let _ = hint_win.set_position(tauri::PhysicalPosition::new(screen_x, screen_y));
    let _ = hint_win.show();
}

/// Hide formation hint window
pub fn hide_formation_hint(app: &AppHandle) {
    if let Some(app_state) = app.try_state::<crate::AppState>() {
        app_state.formation_hint_rect.lock().unwrap().visible = false;
    }
    if let Some(hint_win) = app.get_window("formation-hint") {
        let _ = hint_win.hide();
    }
}
