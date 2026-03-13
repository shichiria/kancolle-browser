use base64::Engine;
use log::info;
use std::path::PathBuf;
use tauri::{Emitter, Manager};

use crate::api;
use crate::drive_sync;
use crate::expedition;
use crate::improvement;
use crate::quest_progress;
use crate::sortie_quest;

use api::models::GameState;

/// Get all expedition definitions for the frontend
#[tauri::command]
pub(crate) fn get_expeditions() -> Vec<expedition::ExpeditionDef> {
    expedition::get_all_expeditions()
}

/// Get all sortie quest definitions for the frontend
#[tauri::command]
pub(crate) fn get_sortie_quests() -> Vec<sortie_quest::SortieQuestDef> {
    sortie_quest::get_all_sortie_quests()
}

/// Get currently active (accepted/completed) quest details
#[tauri::command]
pub(crate) async fn get_active_quest_ids(
    state: tauri::State<'_, api::models::GameState>,
) -> Result<Vec<api::models::ActiveQuestDetail>, String> {
    let inner = state.inner.read().await;
    Ok(inner.history.active_quest_details.values().cloned().collect())
}

/// Check if a fleet meets the conditions for a specific sortie quest
#[tauri::command]
pub(crate) async fn check_sortie_quest_cmd(
    fleet_index: usize,
    quest_id: String,
    state: tauri::State<'_, api::models::GameState>,
) -> Result<sortie_quest::SortieQuestCheckResult, String> {
    let inner = state.inner.read().await;

    if fleet_index >= inner.profile.fleets.len() {
        return Err(format!(
            "Invalid fleet index: {} (have {} fleets)",
            fleet_index,
            inner.profile.fleets.len()
        ));
    }

    let fleet_ship_ids = &inner.profile.fleets[fleet_index];
    if fleet_ship_ids.is_empty() {
        return Err("Fleet is empty".to_string());
    }

    let mut ships = Vec::new();
    for &ship_id in fleet_ship_ids {
        if let Some(info) = inner.profile.ships.get(&ship_id) {
            ships.push(sortie_quest::FleetShipData {
                name: info.name.clone(),
                ship_type: info.stype,
                level: info.lv,
            });
        }
    }

    let fleet_data = sortie_quest::FleetCheckData { ships };
    Ok(sortie_quest::check_sortie_quest(&quest_id, &fleet_data))
}

/// Get all map recommendation definitions for the frontend
#[tauri::command]
pub(crate) fn get_map_recommendations() -> Vec<sortie_quest::MapRecommendationDef> {
    sortie_quest::get_all_map_recommendations()
}

/// Check if a fleet meets the route conditions for a specific map
#[tauri::command]
pub(crate) async fn check_map_recommendation_cmd(
    fleet_index: usize,
    area: String,
    state: tauri::State<'_, api::models::GameState>,
) -> Result<sortie_quest::MapRecommendationCheckResult, String> {
    let inner = state.inner.read().await;

    if fleet_index >= inner.profile.fleets.len() {
        return Err(format!(
            "Invalid fleet index: {} (have {} fleets)",
            fleet_index,
            inner.profile.fleets.len()
        ));
    }

    let fleet_ship_ids = &inner.profile.fleets[fleet_index];
    if fleet_ship_ids.is_empty() {
        return Err("Fleet is empty".to_string());
    }

    let mut ships = Vec::new();
    for &ship_id in fleet_ship_ids {
        if let Some(info) = inner.profile.ships.get(&ship_id) {
            ships.push(sortie_quest::FleetShipData {
                name: info.name.clone(),
                ship_type: info.stype,
                level: info.lv,
            });
        }
    }

    let fleet_data = sortie_quest::FleetCheckData { ships };
    Ok(sortie_quest::check_map_recommendation(&area, &fleet_data))
}

/// Check if a fleet meets the conditions for a specific expedition
#[tauri::command]
pub(crate) async fn check_expedition_cmd(
    fleet_index: usize,
    expedition_id: i32,
    state: tauri::State<'_, api::models::GameState>,
) -> Result<expedition::ExpeditionCheckResult, String> {
    let inner = state.inner.read().await;

    // Validate fleet index
    if fleet_index >= inner.profile.fleets.len() {
        return Err(format!(
            "Invalid fleet index: {} (have {} fleets)",
            fleet_index,
            inner.profile.fleets.len()
        ));
    }

    let fleet_ship_ids = &inner.profile.fleets[fleet_index];
    if fleet_ship_ids.is_empty() {
        return Err("Fleet is empty".to_string());
    }

    // Drum canister: master slotitem category (api_type[2]) == 30 (輸送機材)
    const DRUM_CATEGORY: i32 = 30;

    // Build FleetCheckData from GameState
    let mut ships = Vec::new();
    for &ship_id in fleet_ship_ids {
        if let Some(info) = inner.profile.ships.get(&ship_id) {
            // Count drums on this ship (regular slots + reinforcement expansion)
            let mut drum_count = 0i32;
            for &slot_id in info.slot.iter().chain(std::iter::once(&info.slot_ex)) {
                if slot_id <= 0 {
                    continue;
                }
                if let Some(player_item) = inner.profile.slotitems.get(&slot_id) {
                    if let Some(master_item) = inner.master.slotitems.get(&player_item.slotitem_id)
                    {
                        if master_item.item_type == DRUM_CATEGORY {
                            drum_count += 1;
                        }
                    }
                }
            }

            ships.push(expedition::FleetShipData {
                ship_type: info.stype,
                ship_id: info.ship_id,
                level: info.lv,
                firepower: info.firepower,
                aa: info.aa,
                asw: info.asw,
                los: info.los,
                cond: info.cond,
                has_drum: drum_count > 0,
                drum_count,
            });
        }
    }

    let fleet_data = expedition::FleetCheckData { ships };
    Ok(expedition::check_expedition(expedition_id, &fleet_data))
}

/// Get improvement list for the improvement tab
#[tauri::command]
pub(crate) async fn get_improvement_list(
    state: tauri::State<'_, api::models::GameState>,
) -> Result<improvement::ImprovementListResponse, String> {
    let inner = state.inner.read().await;
    Ok(improvement::build_improvement_list(&inner))
}

/// Get all player ships for the ship list tab
#[tauri::command]
pub(crate) async fn get_ship_list(
    state: tauri::State<'_, api::models::GameState>,
) -> Result<api::models::ShipListResponse, String> {
    let inner = state.inner.read().await;
    let mut ships: Vec<api::models::ShipListItem> = inner
        .profile
        .ships
        .iter()
        .map(|(&id, info)| {
            let stype_name = inner
                .master
                .stypes
                .get(&info.stype)
                .cloned()
                .unwrap_or_default();
            api::models::ShipListItem {
                id,
                ship_id: info.ship_id,
                name: info.name.clone(),
                stype: info.stype,
                stype_name,
                lv: info.lv,
                hp: info.hp,
                maxhp: info.maxhp,
                cond: info.cond,
                firepower: info.firepower,
                torpedo: info.torpedo,
                aa: info.aa,
                armor: info.armor,
                asw: info.asw,
                evasion: info.evasion,
                los: info.los,
                luck: info.luck,
                locked: info.locked,
            }
        })
        .collect();
    ships.sort_by(|a, b| b.lv.cmp(&a.lv).then(a.ship_id.cmp(&b.ship_id)));

    let mut stypes: Vec<(i32, String)> = inner
        .master
        .stypes
        .iter()
        .map(|(&id, name)| (id, name.clone()))
        .collect();
    stypes.sort_by_key(|(id, _)| *id);

    Ok(api::models::ShipListResponse { ships, stypes })
}

/// Get all player equipment grouped by master ID for the equipment list tab
#[tauri::command]
pub(crate) async fn get_equipment_list(
    state: tauri::State<'_, api::models::GameState>,
) -> Result<api::models::EquipListResponse, String> {
    use std::collections::BTreeMap;

    let inner = state.inner.read().await;

    // Group player items by master slotitem_id
    let mut groups: std::collections::HashMap<i32, Vec<&api::models::PlayerSlotItem>> =
        std::collections::HashMap::new();
    for item in inner.profile.slotitems.values() {
        groups.entry(item.slotitem_id).or_default().push(item);
    }

    let mut items: Vec<api::models::EquipListItem> = groups
        .into_iter()
        .filter_map(|(master_id, player_items)| {
            let master = inner.master.slotitems.get(&master_id)?;
            let type_name = inner
                .master
                .equip_types
                .get(&master.item_type)
                .cloned()
                .unwrap_or_default();

            let total_count = player_items.len() as i32;
            let locked_count = player_items.iter().filter(|i| i.locked).count() as i32;

            // Count by improvement level
            let mut level_counts: BTreeMap<i32, i32> = BTreeMap::new();
            for item in &player_items {
                *level_counts.entry(item.level).or_insert(0) += 1;
            }
            let improvements: Vec<(i32, i32)> = level_counts.into_iter().collect();

            Some(api::models::EquipListItem {
                master_id,
                name: master.name.clone(),
                type_id: master.item_type,
                type_name,
                icon_type: master.icon_type,
                total_count,
                locked_count,
                improvements,
            })
        })
        .collect();

    items.sort_by(|a, b| a.type_id.cmp(&b.type_id).then(a.name.cmp(&b.name)));

    // Build equip type filter list (only types that exist in player's equipment)
    let mut used_types: std::collections::HashSet<i32> = std::collections::HashSet::new();
    for item in &items {
        used_types.insert(item.type_id);
    }
    let mut equip_types: Vec<(i32, String)> = inner
        .master
        .equip_types
        .iter()
        .filter(|(id, _)| used_types.contains(id))
        .map(|(&id, name)| (id, name.clone()))
        .collect();
    equip_types.sort_by_key(|(id, _)| *id);

    Ok(api::models::EquipListResponse { items, equip_types })
}

/// Clear improved equipment history
#[tauri::command]
pub(crate) async fn clear_improved_history(
    state: tauri::State<'_, api::models::GameState>,
) -> Result<(), String> {
    let mut inner = state.inner.write().await;
    inner.history.improved_equipment.clear();
    improvement::save_improved_history(&inner.improved_equipment_path, &inner.history.improved_equipment);
    info!("Cleared improved equipment history");
    Ok(())
}

/// Clear battle log records
#[tauri::command]
pub(crate) async fn clear_battle_logs(state: tauri::State<'_, api::models::GameState>) -> Result<(), String> {
    let mut inner = state.inner.write().await;
    inner.sortie.battle_logger.clear_records();
    info!("Cleared battle logs");
    Ok(())
}

/// Clear raw API dumps
#[tauri::command]
pub(crate) async fn clear_raw_api(state: tauri::State<'_, api::models::GameState>) -> Result<(), String> {
    let inner = state.inner.read().await;
    inner.sortie.battle_logger.clear_raw_api();
    info!("Cleared raw API dumps");
    Ok(())
}

/// Toggle raw API log saving (developer option)
#[tauri::command]
pub(crate) async fn set_raw_api_enabled(
    state: tauri::State<'_, api::models::GameState>,
    enabled: bool,
) -> Result<(), String> {
    let mut inner = state.inner.write().await;
    inner.sortie.battle_logger.set_raw_enabled(enabled);
    info!("Raw API saving: {}", if enabled { "ON" } else { "OFF" });
    Ok(())
}

/// Get raw API log saving state
#[tauri::command]
pub(crate) async fn get_raw_api_enabled(
    state: tauri::State<'_, api::models::GameState>,
) -> Result<bool, String> {
    let inner = state.inner.read().await;
    Ok(inner.sortie.battle_logger.is_raw_enabled())
}

/// Reset all browsing data (cookies, session, cache, etc.).
/// If the game webview is open, uses the WebView API first, then deletes files.
#[tauri::command]
pub(crate) fn reset_browser_data(app: tauri::AppHandle) -> Result<String, String> {
    // Windows: require game window to be closed (EBWebView directory is locked)
    #[cfg(not(target_os = "macos"))]
    if app.get_window("game").is_some() {
        return Err("ゲーム画面を閉じてから実行してください".to_string());
    }

    let mut deleted = false;

    // macOS: if game webview is open, clear via API and close the window
    #[cfg(target_os = "macos")]
    {
        if let Some(game_wv) = app.get_webview("game-content") {
            if let Err(e) = game_wv.clear_all_browsing_data() {
                log::warn!("Failed to clear browsing data via API: {}", e);
            } else {
                info!("Cleared browsing data via WebView API");
                deleted = true;
            }
        }
        if let Some(win) = app.get_window("game") {
            let _ = win.close();
        }
    }

    // Windows: delete WebView2 user data
    #[cfg(not(target_os = "macos"))]
    {
        let webview_dir = app
            .path()
            .app_local_data_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("local")
            .join("game-webview")
            .join("EBWebView");

        if webview_dir.exists() {
            if let Err(e) = std::fs::remove_dir_all(&webview_dir) {
                log::warn!("Failed to delete WebView2 data: {}", e);
            } else {
                info!("Deleted WebView2 data: {}", webview_dir.display());
                deleted = true;
            }
        }
    }

    // macOS: delete WKWebView caches and WKWebsiteDataStore data
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = dirs::home_dir() {
            let app_names = ["kancolle-browser", "com.eo.kancolle-browser"];

            // ~/Library/Caches/<app-name>/ (HTTP cache, WebKit cache)
            let caches_dir = home.join("Library/Caches");
            for app_name in &app_names {
                let app_cache = caches_dir.join(app_name);
                if app_cache.exists() {
                    if let Err(e) = std::fs::remove_dir_all(&app_cache) {
                        log::warn!("Failed to delete cache for {}: {}", app_name, e);
                    } else {
                        info!("Deleted WKWebView cache: {}", app_name);
                        deleted = true;
                    }
                }
            }

            // ~/Library/WebKit/<app-name>/ (WKWebsiteDataStore: cookies, local storage, etc.)
            let webkit_dir = home.join("Library/WebKit");
            for app_name in &app_names {
                let app_data = webkit_dir.join(app_name);
                if app_data.exists() {
                    if let Err(e) = std::fs::remove_dir_all(&app_data) {
                        log::warn!("Failed to delete WebKit data for {}: {}", app_name, e);
                    } else {
                        info!("Deleted WKWebsiteDataStore: {}", app_name);
                        deleted = true;
                    }
                }
            }

            // ~/Library/HTTPStorages/<app-name>/ (cookies and HTTP storage)
            let http_storages_dir = home.join("Library/HTTPStorages");
            for app_name in &app_names {
                let app_storage = http_storages_dir.join(app_name);
                if app_storage.exists() {
                    if let Err(e) = std::fs::remove_dir_all(&app_storage) {
                        log::warn!("Failed to delete HTTPStorages for {}: {}", app_name, e);
                    } else {
                        info!("Deleted HTTPStorages: {}", app_name);
                        deleted = true;
                    }
                }
            }
        }
    }

    // Delete saved cookies
    let cookie_path = crate::cookie::cookie_file_path(&app);
    if cookie_path.exists() {
        if let Err(e) = std::fs::remove_file(&cookie_path) {
            log::warn!("Failed to delete cookies: {}", e);
        } else {
            info!("Deleted saved cookies");
            deleted = true;
        }
    }

    if deleted {
        Ok("ブラウザデータをリセットしました（次回ゲーム起動時に再ログインが必要です）".to_string())
    } else {
        Ok("リセット対象のデータはありません".to_string())
    }
}

/// Get a cached game resource (image or JSON) from the local cache.
/// For images, returns a data URI (data:image/png;base64,...).
/// For JSON/text files, returns the raw content string.
/// Returns empty string if the file is not cached.
#[tauri::command]
pub(crate) async fn get_cached_resource(app: tauri::AppHandle, path: String) -> Result<String, String> {
    let cache_dir = app
        .path()
        .app_local_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("local")
        .join("cache");

    let file_path = cache_dir.join(&path);

    // Security: ensure the resolved path stays within cache_dir
    let canonical_cache = cache_dir
        .canonicalize()
        .unwrap_or_else(|_| cache_dir.clone());
    if let Ok(canonical_file) = file_path.canonicalize() {
        if !canonical_file.starts_with(&canonical_cache) {
            return Err("Invalid path".to_string());
        }
    }

    if !file_path.exists() {
        return Ok(String::new());
    }

    let ext = file_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "json" | "js" | "css" | "txt" | "html" => tokio::fs::read_to_string(&file_path)
            .await
            .map_err(|e| format!("Failed to read {}: {}", path, e)),
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" => {
            let mime = match ext.as_str() {
                "png" => "image/png",
                "jpg" | "jpeg" => "image/jpeg",
                "gif" => "image/gif",
                "webp" => "image/webp",
                "svg" => "image/svg+xml",
                _ => "application/octet-stream",
            };
            let data = tokio::fs::read(&file_path)
                .await
                .map_err(|e| format!("Failed to read {}: {}", path, e))?;
            let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
            Ok(format!("data:{};base64,{}", mime, b64))
        }
        _ => {
            // Binary fallback: return base64 with generic MIME
            let data = tokio::fs::read(&file_path)
                .await
                .map_err(|e| format!("Failed to read {}: {}", path, e))?;
            let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
            Ok(format!("data:application/octet-stream;base64,{}", b64))
        }
    }
}

/// Clear the proxy resource cache directory (game images, JSON, etc.).
#[tauri::command]
pub(crate) async fn clear_resource_cache(app: tauri::AppHandle) -> Result<String, String> {
    let cache_dir = app
        .path()
        .app_local_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("local")
        .join("cache");

    if !cache_dir.exists() {
        return Ok("保存リソースはありません".to_string());
    }

    fn count_files(dir: &std::path::Path) -> u64 {
        let mut count = 0u64;
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    count += count_files(&path);
                } else {
                    count += 1;
                }
            }
        }
        count
    }
    let count = count_files(&cache_dir);

    std::fs::remove_dir_all(&cache_dir).map_err(|e| format!("削除失敗: {}", e))?;
    std::fs::create_dir_all(&cache_dir).map_err(|e| format!("ディレクトリ再作成失敗: {}", e))?;

    info!("Resource cache cleared: {} files deleted", count);
    Ok(format!("保存リソースを削除しました（{}ファイル）", count))
}

/// Clear the browser cache (HTTP cache, code cache, GPU cache, etc.).
/// If the game webview is open, uses the WebView API (clear_all_browsing_data).
/// If the game webview is closed, falls back to file-system deletion.
#[tauri::command]
pub(crate) async fn clear_browser_cache(app: tauri::AppHandle) -> Result<String, String> {
    // If game webview is open, use the WebView API to clear browsing data
    if let Some(game_wv) = app.get_webview("game-content") {
        game_wv
            .clear_all_browsing_data()
            .map_err(|e| e.to_string())?;
        info!("Browser cache cleared via WebView API");
        return Ok("ブラウザキャッシュを削除しました".to_string());
    }

    // Game webview is closed — fall back to file-system deletion
    let mut deleted = 0u64;

    #[cfg(not(target_os = "macos"))]
    {
        let webview_dir = app
            .path()
            .app_local_data_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("local")
            .join("game-webview")
            .join("EBWebView");

        if webview_dir.exists() {
            let cache_dirs = [
                "Default/Cache",
                "Default/Code Cache",
                "Default/GPUCache",
                "Default/DawnGraphiteCache",
                "Default/DawnWebGPUCache",
                "ShaderCache",
                "GrShaderCache",
                "GraphiteDawnCache",
            ];

            for dir_name in &cache_dirs {
                let dir_path = webview_dir.join(dir_name);
                if dir_path.exists() {
                    if let Ok(_) = std::fs::remove_dir_all(&dir_path) {
                        deleted += 1;
                        info!("Deleted browser cache: {}", dir_name);
                    }
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        // WKWebView stores NetworkCache under ~/Library/Caches/<app-name>/WebKit/
        if let Some(home) = dirs::home_dir() {
            let caches_dir = home.join("Library/Caches");
            let app_names = ["kancolle-browser", "com.eo.kancolle-browser"];

            for app_name in &app_names {
                let webkit_dir = caches_dir.join(app_name).join("WebKit");
                if webkit_dir.exists() {
                    match std::fs::remove_dir_all(&webkit_dir) {
                        Ok(_) => {
                            deleted += 1;
                            info!("Deleted WKWebView cache: {}/WebKit", app_name);
                        }
                        Err(e) => {
                            log::warn!("Failed to delete WebKit cache for {}: {}", app_name, e);
                        }
                    }
                }
            }
        }
    }

    if deleted == 0 {
        return Ok("ブラウザキャッシュはありません".to_string());
    }

    info!(
        "Browser cache cleared: {} directories/caches deleted",
        deleted
    );
    Ok(format!(
        "ブラウザキャッシュを削除しました（{}箇所）",
        deleted
    ))
}

/// Extract a sprite from a map sprite sheet and return as base64 data URI.
/// `map_display` is e.g. "1-1", `frame_name` is e.g. "map1-1" (from _info.json bg[0]).
#[tauri::command]
pub(crate) async fn get_map_sprite(
    app: tauri::AppHandle,
    map_display: String,
    frame_name: String,
    #[allow(unused)] tint_cyan: Option<bool>,
    route_idx: Option<i32>,
    spot_no: Option<i32>,
) -> Result<String, String> {
    info!(
        "get_map_sprite: map={}, frame={}, route_idx={:?}, spot_no={:?}, tint_cyan={:?}",
        map_display, frame_name, route_idx, spot_no, tint_cyan
    );
    let cache_dir = app
        .path()
        .app_local_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("local")
        .join("cache");

    let parts: Vec<&str> = map_display.split('-').collect();
    let area = format!(
        "{:03}",
        parts
            .first()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0)
    );
    let map = format!(
        "{:02}",
        parts
            .get(1)
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0)
    );

    let atlas_path = cache_dir.join(format!("kcs2/resources/map/{}/{}_image.json", area, map));
    let image_path = cache_dir.join(format!("kcs2/resources/map/{}/{}_image.png", area, map));

    if !atlas_path.exists() || !image_path.exists() {
        return Ok(String::new());
    }

    // Read and parse the atlas JSON
    let atlas_bytes = tokio::fs::read(&atlas_path)
        .await
        .map_err(|e| format!("Failed to read atlas: {}", e))?;

    // The atlas might be brotli-compressed (if cached before the brotli fix)
    let atlas_str = if atlas_bytes.starts_with(b"{") {
        String::from_utf8(atlas_bytes).map_err(|e| format!("Invalid atlas UTF-8: {}", e))?
    } else {
        // Try brotli decompression for old cached files
        let mut decoder = brotli::Decompressor::new(atlas_bytes.as_slice(), 4096);
        let mut decompressed = Vec::new();
        std::io::Read::read_to_end(&mut decoder, &mut decompressed)
            .map_err(|e| format!("Failed to decompress atlas: {}", e))?;
        String::from_utf8(decompressed)
            .map_err(|e| format!("Invalid decompressed atlas UTF-8: {}", e))?
    };

    let atlas: serde_json::Value = serde_json::from_str(&atlas_str)
        .map_err(|e| format!("Failed to parse atlas JSON: {}", e))?;

    // Build the full frame name: map{area}{map}_{frame_name}
    let full_frame_name = format!("map{}{}_{}", area, map, frame_name);

    let frame = atlas
        .get("frames")
        .and_then(|f| f.get(&full_frame_name))
        .and_then(|f| f.get("frame"))
        .ok_or_else(|| format!("Frame '{}' not found in atlas", full_frame_name))?;

    let fx = frame.get("x").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    let fy = frame.get("y").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    let fw = frame.get("w").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    let fh = frame.get("h").and_then(|v| v.as_u64()).unwrap_or(0) as u32;

    if fw == 0 || fh == 0 {
        return Err("Invalid frame dimensions".to_string());
    }

    // Read the sprite sheet and crop - do heavy work in blocking thread
    let image_path_clone = image_path.clone();
    let apply_tint = tint_cyan.unwrap_or(false);
    let result = tokio::task::spawn_blocking(move || -> Result<String, String> {
        let img = image::open(&image_path_clone)
            .map_err(|e| format!("Failed to open sprite sheet: {}", e))?;
        let cropped = img.crop_imm(fx, fy, fw, fh);

        // Apply cyan tint if requested: replace RGB with cyan, preserve alpha
        let output = if apply_tint {
            let mut rgba = cropped.to_rgba8();
            for pixel in rgba.pixels_mut() {
                if pixel[3] == 0 {
                    continue;
                } // skip fully transparent
                  // Original pixel luminescence (0.0 - 1.0)
                let lum =
                    (pixel[0] as f32 * 0.299 + pixel[1] as f32 * 0.587 + pixel[2] as f32 * 0.114)
                        / 255.0;

                // For white dotted lines, lum is high. Map brightness to cyan.
                // Pure white -> Cyan (#26c6da or similar bright color)
                pixel[0] = (38.0 * lum) as u8; // R
                pixel[1] = (198.0 * lum) as u8; // G
                pixel[2] = (218.0 * lum) as u8; // B
                                                // Keep original alpha (pixel[3]) to preserve anti-aliasing edges
            }
            image::DynamicImage::ImageRgba8(rgba)
        } else {
            cropped
        };

        let mut buf = std::io::Cursor::new(Vec::new());
        output
            .write_to(&mut buf, image::ImageFormat::Png)
            .map_err(|e| format!("Failed to encode cropped sprite: {}", e))?;

        let b64 = base64::engine::general_purpose::STANDARD.encode(buf.into_inner());
        Ok(format!("data:image/png;base64,{}", b64))
    })
    .await
    .map_err(|e| format!("Spawn blocking failed: {}", e))?;

    result
}

/// Get battle log records
#[tauri::command]
pub(crate) async fn get_battle_logs(
    limit: Option<usize>,
    offset: Option<usize>,
    date_from: Option<String>,
    date_to: Option<String>,
    state: tauri::State<'_, api::models::GameState>,
) -> Result<serde_json::Value, String> {
    let inner = state.inner.read().await;
    if let (Some(from), Some(to)) = (&date_from, &date_to) {
        let records = inner.sortie.battle_logger.get_records_by_date_range(from, to);
        let total = records.len();
        Ok(serde_json::json!({
            "records": records,
            "total": total,
        }))
    } else {
        let limit = limit.unwrap_or(50);
        let offset = offset.unwrap_or(0);
        let records = inner.sortie.battle_logger.get_records(limit, offset);
        let total = inner.sortie.battle_logger.record_count();
        Ok(serde_json::json!({
            "records": records,
            "total": total,
        }))
    }
}

/// Get quest progress for active quests
#[tauri::command]
pub(crate) async fn get_quest_progress(
    state: tauri::State<'_, api::models::GameState>,
) -> Result<Vec<quest_progress::QuestProgressSummary>, String> {
    let mut inner = state.inner.write().await;
    let path = inner.quest_progress_path.clone();
    let defs = inner.history.sortie_quest_defs.clone();
    let aq = inner.history.active_quests.clone();
    Ok(quest_progress::get_active_progress(
        &mut inner.history.quest_progress,
        &aq,
        &defs,
        &path,
    ))
}

/// Manually update quest progress (toggle area or set count)
#[tauri::command]
pub(crate) async fn update_quest_progress(
    quest_id: i32,
    area: Option<String>,
    count: Option<i32>,
    state: tauri::State<'_, api::models::GameState>,
    app: tauri::AppHandle,
) -> Result<bool, String> {
    let mut inner = state.inner.write().await;
    let path = inner.quest_progress_path.clone();
    let defs = inner.history.sortie_quest_defs.clone();
    let changed = quest_progress::manual_update(
        &mut inner.history.quest_progress,
        quest_id,
        area,
        count,
        &defs,
        &path,
    );
    if changed {
        let aq = inner.history.active_quests.clone();
        let progress =
            quest_progress::get_active_progress(&mut inner.history.quest_progress, &aq, &defs, &path);
        let _ = app.emit("quest-progress-updated", &progress);
    }
    Ok(changed)
}

/// Clear all quest progress data
#[tauri::command]
pub(crate) async fn clear_quest_progress(
    state: tauri::State<'_, api::models::GameState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let mut inner = state.inner.write().await;
    inner.history.quest_progress = quest_progress::QuestProgressState::default();
    quest_progress::save_progress(&inner.quest_progress_path, &inner.history.quest_progress);
    info!("Cleared quest progress");
    let progress: Vec<quest_progress::QuestProgressSummary> = Vec::new();
    let _ = app.emit("quest-progress-updated", &progress);
    Ok(())
}

// =============================================================================
// Google Drive Sync Commands
// =============================================================================

/// Start Google Drive OAuth login flow (opens browser)
#[tauri::command]
pub(crate) async fn drive_login(
    app: tauri::AppHandle,
    state: tauri::State<'_, GameState>,
) -> Result<(), String> {
    info!("drive_login: started");
    let inner = state.inner.read().await;
    let data_dir = inner.data_dir.clone();
    drop(inner);

    let (client_id, client_secret) = drive_sync::auth::client_credentials()
        .ok_or("Google Drive sync is not available in this build.")?;

    let auth = drive_sync::auth::authenticate(client_id, client_secret, &data_dir).await?;

    // Start sync engine
    let sync_tx = drive_sync::engine::start_sync_engine(app.clone(), data_dir, auth).await;

    // Store notifier in GameState
    let mut inner = state.inner.write().await;
    inner.sync_notifier = Some(sync_tx);

    info!("Google Drive sync started");
    Ok(())
}

/// Log out from Google Drive
#[tauri::command]
pub(crate) async fn drive_logout(state: tauri::State<'_, GameState>) -> Result<(), String> {
    let mut inner = state.inner.write().await;

    // Shut down sync engine
    if let Some(tx) = inner.sync_notifier.take() {
        let _ = tx.send(drive_sync::SyncCommand::Shutdown).await;
    }

    drive_sync::auth::logout(&inner.data_dir);
    info!("Google Drive logged out");
    Ok(())
}

/// Get Google Drive sync status
#[tauri::command]
pub(crate) async fn get_drive_status(
    state: tauri::State<'_, GameState>,
) -> Result<drive_sync::SyncStatus, String> {
    let inner = state.inner.read().await;
    let has_notifier = inner.sync_notifier.is_some();

    let manifest = drive_sync::load_manifest(&inner.data_dir);
    let last_sync = manifest.last_full_sync.map(|t| t.to_rfc3339());

    Ok(drive_sync::SyncStatus {
        authenticated: has_notifier,
        email: None,
        syncing: false,
        last_sync,
        error: None,
    })
}

/// Force a full sync with Google Drive
#[tauri::command]
pub(crate) async fn drive_force_sync(state: tauri::State<'_, GameState>) -> Result<(), String> {
    let inner = state.inner.read().await;
    let tx = inner
        .sync_notifier
        .as_ref()
        .ok_or("Not connected to Google Drive")?;
    tx.send(drive_sync::SyncCommand::FullSync)
        .await
        .map_err(|e| format!("Failed to send sync command: {}", e))?;
    Ok(())
}

/// Get the proxy port for the frontend
#[tauri::command]
pub(crate) fn get_proxy_port(state: tauri::State<'_, crate::AppState>) -> u16 {
    *state.proxy_port.lock().unwrap()
}
