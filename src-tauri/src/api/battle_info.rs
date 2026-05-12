/// Battle info overlay — shows engagement form and air superiority on game overlay.

/// Get engagement form display name from API value.
/// api_formation[2]: 1=同航戦, 2=反航戦, 3=T字有利, 4=T字不利
pub fn engagement_name(id: i32) -> &'static str {
    match id {
        1 => "同航戦",
        2 => "反航戦",
        3 => "T字有利",
        4 => "T字不利",
        _ => "不明",
    }
}

/// Get air superiority display name and CSS color from API value.
/// api_disp_seiku: 0=航空劣勢, 1=航空優勢, 2=制空権確保, 3=航空均衡, 4=制空権喪失
pub fn air_superiority_label(id: i32) -> (&'static str, &'static str) {
    match id {
        0 => ("航空劣勢", "#f44336"),
        1 => ("航空優勢", "#4caf50"),
        2 => ("制空権確保", "#2196f3"),
        3 => ("航空均衡", "#ff9800"),
        4 => ("制空権喪失", "#d32f2f"),
        _ => ("不明", "#78909c"),
    }
}

/// Engagement form CSS color.
pub fn engagement_color(id: i32) -> &'static str {
    match id {
        1 => "#b0bec5",   // 同航戦 — neutral grey
        2 => "#ff9800",   // 反航戦 — orange
        3 => "#4caf50",   // T字有利 — green (favorable)
        4 => "#f44336",   // T字不利 — red (unfavorable)
        _ => "#78909c",
    }
}

/// Build battle info overlay data from battle detail.
/// Returns (engagement_text, engagement_color, air_text, air_color) or None if no data.
pub fn build_battle_info(
    formation: &[i32; 3],
    air_battle: Option<&crate::battle_log::AirBattleResult>,
) -> BattleInfoData {
    let engagement_id = formation[2];
    let (air_text, air_color) = air_battle
        .and_then(|ab| ab.air_superiority)
        .map(|id| air_superiority_label(id))
        .unwrap_or(("", ""));

    BattleInfoData {
        engagement: engagement_name(engagement_id).to_string(),
        engagement_color: engagement_color(engagement_id).to_string(),
        air_control: air_text.to_string(),
        air_control_color: air_color.to_string(),
        lbas_waves: Vec::new(),
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct BattleInfoData {
    pub engagement: String,
    pub engagement_color: String,
    pub air_control: String,
    pub air_control_color: String,
    /// Per-wave air superiority from LBAS attacks (api_air_base_attack[]).
    /// Ordered by wave appearance, typically up to 4 entries (2 bases × 2 waves).
    pub lbas_waves: Vec<LbasWaveLabel>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct LbasWaveLabel {
    /// Base id (`api_base_id`) e.g. 1, 2.
    pub base_id: i32,
    /// 1-based wave index within this base (1 or 2).
    pub wave: i32,
    pub text: String,
    pub color: String,
}

/// Extract per-wave LBAS air-superiority labels from a battle response's `api_data`.
/// Returns empty if `api_air_base_attack` is missing or empty.
pub fn extract_lbas_waves(api_data: &serde_json::Value) -> Vec<LbasWaveLabel> {
    let arr = match api_data.get("api_air_base_attack").and_then(|v| v.as_array()) {
        Some(a) => a,
        None => return Vec::new(),
    };
    use std::collections::HashMap;
    let mut wave_for_base: HashMap<i32, i32> = HashMap::new();
    let mut out = Vec::with_capacity(arr.len());
    for entry in arr {
        let base_id = entry
            .get("api_base_id")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;
        let seiku = entry
            .get("api_stage1")
            .and_then(|s| s.get("api_disp_seiku"))
            .and_then(|v| v.as_i64())
            .map(|n| n as i32);
        let (text, color) = match seiku {
            Some(id) => air_superiority_label(id),
            None => ("-", "#78909c"),
        };
        let w = {
            let counter = wave_for_base.entry(base_id).or_insert(0);
            *counter += 1;
            *counter
        };
        out.push(LbasWaveLabel {
            base_id,
            wave: w,
            text: text.to_string(),
            color: color.to_string(),
        });
    }
    out
}

/// Send battle info to the battle-info window (only if enabled).
pub fn show_battle_info_overlay(app: &tauri::AppHandle, data: &BattleInfoData) {
    use std::sync::atomic::Ordering;
    use tauri::Manager;

    log::info!(
        "[BattleInfo] show_battle_info_overlay: engagement={}, air={}",
        data.engagement,
        data.air_control
    );

    // Always store the latest data for re-display on toggle re-enable
    if let Some(state) = app.try_state::<crate::AppState>() {
        *state.last_battle_info.lock().unwrap() = Some(data.clone());
    }

    let enabled = app
        .try_state::<crate::AppState>()
        .map(|s| s.battle_info_enabled.load(Ordering::Relaxed))
        .unwrap_or(false);
    if !enabled {
        log::info!("[BattleInfo] overlay disabled, skipping (data stored for later)");
        return;
    }

    let battle_info_win = match app.get_window("battle-info") {
        Some(w) => w,
        None => {
            log::warn!("[BattleInfo] battle-info window not found");
            return;
        }
    };
    let game_win = match app.get_window("game") {
        Some(w) => w,
        None => {
            log::warn!("[BattleInfo] game window not found");
            return;
        }
    };

    // Position at top-left of game window, below control bar
    let scale = game_win.scale_factor().unwrap_or(1.0);
    let inner_pos = match game_win.inner_position() {
        Ok(p) => p,
        Err(e) => {
            log::warn!("[BattleInfo] failed to get game window position: {}", e);
            return;
        }
    };
    let zoom = app
        .try_state::<crate::AppState>()
        .map(|s| *s.game_zoom.lock().unwrap())
        .unwrap_or(1.0);
    let bar_h = crate::game_window::CONTROL_BAR_HEIGHT * zoom;
    let margin = 8.0;

    let x = inner_pos.x + (margin * scale) as i32;
    let y = inner_pos.y + ((bar_h + margin) * scale) as i32;

    log::debug!("[BattleInfo] position=({}, {}), scale={}, zoom={}", x, y, scale, zoom);

    if let Err(e) = battle_info_win.set_position(tauri::PhysicalPosition::new(x, y)) {
        log::warn!("[BattleInfo] failed to set position: {}", e);
    }
    if let Err(e) = battle_info_win.show() {
        log::warn!("[BattleInfo] failed to show window: {}", e);
    }

    match app.get_webview("battle-info-content") {
        Some(wv) => {
            let json = serde_json::to_string(data).unwrap_or_default();
            log::debug!("[BattleInfo] eval JS payload: {}", json);
            if let Err(e) = wv.eval(&format!("window.showBattleInfo({})", json)) {
                log::warn!("[BattleInfo] failed to eval JS: {}", e);
            }
        }
        None => {
            log::warn!("[BattleInfo] battle-info-content webview not found");
        }
    }
}

/// Hide battle info window and clear stored data.
pub fn hide_battle_info_overlay(app: &tauri::AppHandle) {
    use tauri::Manager;
    log::debug!("[BattleInfo] hiding overlay");
    // Clear stored data so re-enable won't show stale info
    if let Some(state) = app.try_state::<crate::AppState>() {
        *state.last_battle_info.lock().unwrap() = None;
    }
    if let Some(win) = app.get_window("battle-info") {
        if let Err(e) = win.hide() {
            log::warn!("[BattleInfo] failed to hide window: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engagement_name_all_values() {
        assert_eq!(engagement_name(1), "同航戦");
        assert_eq!(engagement_name(2), "反航戦");
        assert_eq!(engagement_name(3), "T字有利");
        assert_eq!(engagement_name(4), "T字不利");
        assert_eq!(engagement_name(0), "不明");
        assert_eq!(engagement_name(99), "不明");
    }

    #[test]
    fn test_engagement_color_all_values() {
        assert_eq!(engagement_color(1), "#b0bec5");
        assert_eq!(engagement_color(2), "#ff9800");
        assert_eq!(engagement_color(3), "#4caf50");
        assert_eq!(engagement_color(4), "#f44336");
        assert_eq!(engagement_color(0), "#78909c");
    }

    #[test]
    fn test_air_superiority_label_all_values() {
        assert_eq!(air_superiority_label(0), ("航空劣勢", "#f44336"));
        assert_eq!(air_superiority_label(1), ("航空優勢", "#4caf50"));
        assert_eq!(air_superiority_label(2), ("制空権確保", "#2196f3"));
        assert_eq!(air_superiority_label(3), ("航空均衡", "#ff9800"));
        assert_eq!(air_superiority_label(4), ("制空権喪失", "#d32f2f"));
        assert_eq!(air_superiority_label(99), ("不明", "#78909c"));
    }

    #[test]
    fn test_build_battle_info_with_air_battle() {
        let formation = [1, 2, 3]; // friendly=単縦陣, enemy=複縦陣, engagement=T字有利
        let air = crate::battle_log::AirBattleResult {
            air_superiority: Some(1), // 航空優勢
            friendly_plane_count: None,
            enemy_plane_count: None,
        };
        let info = build_battle_info(&formation, Some(&air));
        assert_eq!(info.engagement, "T字有利");
        assert_eq!(info.engagement_color, "#4caf50");
        assert_eq!(info.air_control, "航空優勢");
        assert_eq!(info.air_control_color, "#4caf50");
    }

    #[test]
    fn test_build_battle_info_without_air_battle() {
        let formation = [1, 2, 4]; // T字不利
        let info = build_battle_info(&formation, None);
        assert_eq!(info.engagement, "T字不利");
        assert_eq!(info.engagement_color, "#f44336");
        assert_eq!(info.air_control, "");
        assert_eq!(info.air_control_color, "");
    }

    #[test]
    fn test_build_battle_info_air_battle_without_seiku() {
        let formation = [1, 2, 1]; // 同航戦
        let air = crate::battle_log::AirBattleResult {
            air_superiority: None,
            friendly_plane_count: Some([20, 3]),
            enemy_plane_count: Some([15, 8]),
        };
        let info = build_battle_info(&formation, Some(&air));
        assert_eq!(info.engagement, "同航戦");
        assert_eq!(info.air_control, "");
    }
}
