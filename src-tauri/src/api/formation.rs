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

fn fleet_kind(is_combined: bool) -> &'static str {
    if is_combined {
        "combined"
    } else {
        "normal"
    }
}

fn memory_key(map_area: i32, map_no: i32, cell_no: i32, is_combined: bool) -> String {
    format!(
        "{}-{}-{}-{}",
        map_area,
        map_no,
        cell_no,
        fleet_kind(is_combined)
    )
}

fn smoke_memory_key(formation_key: &str) -> String {
    format!("{formation_key}:smoke")
}

fn formation_matches_fleet(formation: i32, is_combined: bool) -> bool {
    if is_combined {
        (11..=14).contains(&formation)
    } else {
        (1..=6).contains(&formation)
    }
}

/// Event maps use three-digit map IDs (for example 621/622), while regular
/// maps use IDs such as 15 or 75. mapinfo is replaced on every refresh, so the
/// presence of an event-map gauge also tells us whether 警戒陣 is available.
pub(super) fn alert_formation_available(mapinfo_gauges: &HashMap<i32, i32>) -> bool {
    mapinfo_gauges.keys().any(|map_id| *map_id >= 100)
}

/// Read the formation and smoke selection remembered for a map cell.
///
/// Older files used a fleet-agnostic "{area}-{map}-{cell}" key. Keep those
/// entries usable only when the formation ID matches the current fleet type,
/// so a legacy normal-fleet choice is never drawn over the combined-fleet UI
/// (or vice versa).
pub(super) fn remembered_selection(
    memory: &HashMap<String, i32>,
    map_area: i32,
    map_no: i32,
    cell_no: i32,
    is_combined: bool,
) -> Option<(i32, i32)> {
    let key = memory_key(map_area, map_no, cell_no, is_combined);
    if let Some(&formation) = memory.get(&key) {
        let smoke_type = memory
            .get(&smoke_memory_key(&key))
            .copied()
            .unwrap_or(0)
            .max(0);
        return Some((formation, smoke_type));
    }

    let legacy_key = format!("{map_area}-{map_no}-{cell_no}");
    memory
        .get(&legacy_key)
        .copied()
        .filter(|formation| formation_matches_fleet(*formation, is_combined))
        .map(|formation| (formation, 0))
}

/// Store normal and combined-fleet selections independently. Smoke state lives
/// under a companion key so formation_memory.json remains compatible with the
/// existing integer-valued file format.
pub(super) fn remember_selection(
    memory: &mut HashMap<String, i32>,
    map_area: i32,
    map_no: i32,
    cell_no: i32,
    is_combined: bool,
    formation: i32,
    smoke_type: i32,
) -> String {
    let key = memory_key(map_area, map_no, cell_no, is_combined);
    memory.insert(key.clone(), formation);

    let smoke_key = smoke_memory_key(&key);
    if smoke_type > 0 {
        memory.insert(smoke_key, smoke_type);
    } else {
        memory.remove(&smoke_key);
    }

    key
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
fn get_formation_button_rect(
    formation: i32,
    ship_count: usize,
    alert_formation_available: bool,
) -> Option<(f64, f64, f64, f64)> {
    // Standard labels are 150x48. The longer combined-fleet labels use
    // separate 210x45 sprites; using the standard width shifts the visible
    // highlight into the right-hand portion of those buttons.
    let has_ring = ship_count >= 5;
    let grid_x = [663.0, 858.0, 1056.0];
    let compact_x = [762.0, 958.0];

    let (cx, cy, bw, bh) = match formation {
        // With five or more ships the regular formations use three columns
        // on the top row. Four-ship fleets omit 輪形陣 and center two buttons.
        1 => (
            if has_ring { grid_x[0] } else { compact_x[0] },
            278.0,
            154.0,
            48.0,
        ),
        2 => (
            if has_ring { grid_x[1] } else { compact_x[1] },
            278.0,
            154.0,
            48.0,
        ),
        3 if has_ring => (grid_x[2], 278.0, 154.0, 48.0),
        // During an event, 警戒陣 adds a third button to the bottom row.
        // Outside the event period, 梯形陣 and 単横陣 remain centered.
        4 => (
            if alert_formation_available {
                grid_x[0]
            } else {
                compact_x[0]
            },
            517.0,
            154.0,
            48.0,
        ),
        5 => (
            if alert_formation_available {
                grid_x[1]
            } else {
                compact_x[1]
            },
            517.0,
            154.0,
            48.0,
        ),
        6 if alert_formation_available => (grid_x[2], 517.0, 154.0, 48.0),
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

/// Smoke button rect in game canvas (1200x720) coordinates.
///
/// The button moves with the formation layout: it is left of 梯形陣 for a
/// normal fleet and left of 第三警戒航行序列 for a combined fleet.
fn get_smoke_button_rect(
    is_combined: bool,
    ship_count: usize,
    alert_formation_available: bool,
) -> (f64, f64, f64, f64) {
    if is_combined {
        (568.0, 440.0, 52.0, 52.0)
    } else {
        let formation_left = get_formation_button_rect(4, ship_count, alert_formation_available)
            .map(|rect| rect.0)
            .unwrap_or(586.0);
        (formation_left - 64.0, 496.0, 52.0, 52.0)
    }
}

fn union_rect(first: (f64, f64, f64, f64), second: (f64, f64, f64, f64)) -> (f64, f64, f64, f64) {
    let left = first.0.min(second.0);
    let top = first.1.min(second.1);
    let right = (first.0 + first.2).max(second.0 + second.2);
    let bottom = (first.1 + first.3).max(second.1 + second.3);
    (left, top, right - left, bottom - top)
}

/// Show formation highlight using the click-through formation-hint window
pub(crate) fn show_formation_hint(
    app: &AppHandle,
    formation: i32,
    ship_count: usize,
    smoke: bool,
    alert_formation_available: bool,
) {
    log::info!(
        "[FormationHint] show: formation={} ({}), ships={}, smoke={}",
        formation,
        formation_name(formation),
        ship_count,
        smoke
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

    let formation_rect =
        match get_formation_button_rect(formation, ship_count, alert_formation_available) {
            Some(r) => r,
            None => {
                log::warn!("[FormationHint] no button rect for formation={}", formation);
                return;
            }
        };
    let smoke_rect = smoke
        .then(|| get_smoke_button_rect(formation >= 11, ship_count, alert_formation_available));
    let (bx, by, bw, bh) = smoke_rect
        .map(|rect| union_rect(formation_rect, rect))
        .unwrap_or(formation_rect);

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
    let (dx, dy) = (dx + (6.0 * scale) as i32, dy + (30.0 * scale) as i32);
    let phys_w = (bw * zoom * scale).round() as u32;
    let phys_h = (bh * zoom * scale).round() as u32;

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
        let formation_relative = (
            (formation_rect.0 - bx) / bw,
            (formation_rect.1 - by) / bh,
            formation_rect.2 / bw,
            formation_rect.3 / bh,
        );
        let smoke_relative = smoke_rect.map(|rect| {
            (
                (rect.0 - bx) / bw,
                (rect.1 - by) / bh,
                rect.2 / bw,
                rect.3 / bh,
            )
        });
        let script = format!(
            "window.setFormationHint({}, {});",
            serde_json::to_string(&formation_relative).unwrap_or_else(|_| "null".to_string()),
            serde_json::to_string(&smoke_relative).unwrap_or_else(|_| "null".to_string())
        );
        if let Err(e) = wv.eval(script) {
            log::warn!("[FormationHint] failed to configure frames: {}", e);
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
    use super::{
        alert_formation_available, get_formation_button_rect, get_smoke_button_rect,
        remember_selection, remembered_selection, union_rect,
    };
    use std::collections::HashMap;

    #[test]
    fn standard_formation_uses_standard_label_size() {
        let (_, _, width, height) = get_formation_button_rect(1, 6, false).unwrap();
        assert_eq!((width, height), (154.0, 48.0));
    }

    #[test]
    fn event_map_gauge_enables_alert_formation_layout() {
        assert!(!alert_formation_available(&HashMap::from([(75, 3)])));
        assert!(alert_formation_available(&HashMap::from([
            (75, 3),
            (622, 3)
        ])));
    }

    #[test]
    fn combined_fleet_formation_covers_the_long_label() {
        let (x, y, width, height) = get_formation_button_rect(13, 6, false).unwrap();
        assert_eq!((x, y, width, height), (636.0, 445.5, 214.0, 45.0));
    }

    #[test]
    fn event_six_button_layout_uses_three_columns_on_both_rows() {
        let top = get_formation_button_rect(1, 6, true).unwrap();
        let bottom = get_formation_button_rect(4, 6, true).unwrap();
        let alert = get_formation_button_rect(6, 6, true).unwrap();

        assert_eq!(top.0, 586.0);
        assert_eq!(bottom.0, 586.0);
        assert_eq!(alert.0, 979.0);
    }

    #[test]
    fn event_four_ship_layout_centers_top_two_and_keeps_three_bottom_buttons() {
        let single = get_formation_button_rect(1, 4, true).unwrap();
        let double = get_formation_button_rect(2, 4, true).unwrap();
        let echelon = get_formation_button_rect(4, 4, true).unwrap();

        assert_eq!(single.0, 685.0);
        assert_eq!(double.0, 881.0);
        assert_eq!(echelon.0, 586.0);
        assert!(get_formation_button_rect(3, 4, true).is_none());
    }

    #[test]
    fn regular_layout_centers_the_two_bottom_buttons() {
        let echelon = get_formation_button_rect(4, 6, false).unwrap();
        let line_abreast = get_formation_button_rect(5, 6, false).unwrap();

        assert_eq!(echelon.0, 685.0);
        assert_eq!(line_abreast.0, 881.0);
        assert!(get_formation_button_rect(6, 6, false).is_none());
    }

    #[test]
    fn normal_and_combined_selections_are_stored_separately() {
        let mut memory = HashMap::new();
        remember_selection(&mut memory, 46, 1, 7, false, 6, 0);
        remember_selection(&mut memory, 46, 1, 7, true, 13, 2);

        assert_eq!(remembered_selection(&memory, 46, 1, 7, false), Some((6, 0)));
        assert_eq!(remembered_selection(&memory, 46, 1, 7, true), Some((13, 2)));
    }

    #[test]
    fn legacy_selection_is_only_used_for_matching_fleet_type() {
        let mut memory = HashMap::new();
        memory.insert("2-3-4".to_string(), 5);

        assert_eq!(remembered_selection(&memory, 2, 3, 4, false), Some((5, 0)));
        assert_eq!(remembered_selection(&memory, 2, 3, 4, true), None);
    }

    #[test]
    fn clearing_smoke_removes_companion_entry() {
        let mut memory = HashMap::new();
        remember_selection(&mut memory, 1, 1, 1, true, 13, 3);
        remember_selection(&mut memory, 1, 1, 1, true, 14, 0);

        assert_eq!(remembered_selection(&memory, 1, 1, 1, true), Some((14, 0)));
    }

    #[test]
    fn smoke_rect_is_left_of_third_combined_formation() {
        let formation = get_formation_button_rect(13, 12, true).unwrap();
        let smoke = get_smoke_button_rect(true, 12, true);
        let bounds = union_rect(formation, smoke);

        assert!(smoke.0 + smoke.2 <= formation.0);
        assert_eq!(bounds, (568.0, 440.0, 282.0, 52.0));
    }

    #[test]
    fn normal_smoke_button_tracks_the_bottom_row_layout() {
        assert_eq!(
            get_smoke_button_rect(false, 6, true),
            (522.0, 496.0, 52.0, 52.0)
        );
        assert_eq!(
            get_smoke_button_rect(false, 6, false),
            (621.0, 496.0, 52.0, 52.0)
        );
    }
}
