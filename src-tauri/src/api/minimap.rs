use tauri::{AppHandle, Manager};

/// Send minimap data to overlay webview (only if minimap is enabled)
pub(crate) fn update_minimap_overlay(app: &AppHandle, sortie: &crate::battle_log::SortieRecord) {
    let minimap_on = app
        .try_state::<crate::AppState>()
        .map(|s| s.minimap_enabled.load(std::sync::atomic::Ordering::Relaxed))
        .unwrap_or(false);
    if !minimap_on {
        return;
    }
    send_minimap_data(app, sortie);
}

/// Send minimap data to overlay and resize it (called from toggle_minimap too)
pub fn send_minimap_data(app: &AppHandle, sortie: &crate::battle_log::SortieRecord) {
    if let Some(overlay) = app.get_webview("game-overlay") {
        let nodes_json: Vec<serde_json::Value> = sortie
            .nodes
            .iter()
            .map(|n| {
                serde_json::json!({
                    "cell_no": n.cell_no,
                    "event_kind": n.event_kind,
                    "event_id": n.event_id,
                    "has_battle": n.battle.is_some(),
                })
            })
            .collect();
        let map_display = &sortie.map_display;
        let js = format!(
            "window.updateMinimap({}, {})",
            serde_json::to_string(map_display).unwrap_or_else(|_| "\"\"".into()),
            serde_json::to_string(&nodes_json).unwrap_or_else(|_| "[]".into()),
        );
        let _ = crate::overlay::show_minimap_overlay(app);
        let _ = overlay.eval(&js);
    }
}

/// Hide minimap on overlay
pub(crate) fn hide_minimap_overlay(app: &AppHandle) {
    if let Some(overlay) = app.get_webview("game-overlay") {
        let _ = overlay.eval("window.hideMinimap()");
        let _ = overlay.set_size(tauri::LogicalSize::new(1.0, 1.0));
    }
}
