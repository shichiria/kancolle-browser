use log::info;
use tauri::Emitter;

use crate::api;
use crate::drive_sync;
use crate::expedition;
use crate::improvement;
use crate::quest_progress;
use crate::sortie_quest;

use api::models::GameState;

pub(crate) mod cache;

/// Persist a browser-side console message or unhandled JavaScript error in the
/// same per-launch diagnostic log as the Rust backend.
#[tauri::command]
pub(crate) fn log_frontend_event(level: String, message: String, source: Option<String>) {
    crate::diagnostics::frontend_event(&level, &message, source.as_deref());
}

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
    Ok(inner
        .history
        .active_quest_details
        .values()
        .cloned()
        .collect())
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
        .map(|(&id, name): (&i32, &String)| (id, name.clone()))
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
        .filter_map(
            |(master_id, player_items): (i32, Vec<&api::models::PlayerSlotItem>)| {
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
            },
        )
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
        .map(|(&id, name): (&i32, &String)| (id, name.clone()))
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
    improvement::save_improved_history(
        &inner.improved_equipment_path,
        &inner.history.improved_equipment,
    );
    info!("Cleared improved equipment history");
    Ok(())
}

/// Clear battle log records
#[tauri::command]
pub(crate) async fn clear_battle_logs(
    state: tauri::State<'_, api::models::GameState>,
) -> Result<(), String> {
    let mut inner = state.inner.write().await;
    inner.sortie.battle_logger.clear_records();
    info!("Cleared battle logs");
    Ok(())
}

/// Clear raw API dumps
#[tauri::command]
pub(crate) async fn clear_raw_api(
    state: tauri::State<'_, api::models::GameState>,
) -> Result<(), String> {
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
        let records = inner
            .sortie
            .battle_logger
            .get_records_by_date_range(from, to);
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
        let progress = quest_progress::get_active_progress(
            &mut inner.history.quest_progress,
            &aq,
            &defs,
            &path,
        );
        let _ = app.emit(crate::events::QUEST_PROGRESS_UPDATED, &progress);
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
    let _ = app.emit(crate::events::QUEST_PROGRESS_UPDATED, &progress);
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
    crate::action_log::record("Command", "drive_login", None);
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
    crate::action_log::record("Command", "drive_logout", None);
    let mut inner = state.inner.write().await;

    // Shut down sync engine
    if let Some(tx) = inner.sync_notifier.take() {
        let _: Result<(), _> = tx.send(drive_sync::SyncCommand::Shutdown).await;
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
    crate::action_log::record("Command", "drive_force_sync", None);
    let inner = state.inner.read().await;
    let tx = inner
        .sync_notifier
        .as_ref()
        .ok_or("Not connected to Google Drive")?;
    tx.send(drive_sync::SyncCommand::FullSync).await.map_err(
        |e: tokio::sync::mpsc::error::SendError<drive_sync::SyncCommand>| {
            format!("Failed to send sync command: {}", e)
        },
    )?;
    Ok(())
}

/// Get recent action log entries in every build where they are recorded.
#[tauri::command]
pub(crate) fn get_action_log(limit: Option<usize>) -> Vec<serde_json::Value> {
    let entries = crate::action_log::get_recent(limit.unwrap_or(100));
    entries
        .into_iter()
        .map(|e| serde_json::to_value(e).unwrap_or_default())
        .collect()
}

/// Get the currently inferred game screen (for the Debug tab).
#[tauri::command]
pub(crate) fn get_current_screen(state: tauri::State<'_, crate::AppState>) -> String {
    format!(
        "{:?}",
        *crate::lock_or_recover(&state.navigation.current_screen, "current_screen")
    )
}

/// Get the currently selected fleet (1-4) within fleet-compatible screens.
#[tauri::command]
pub(crate) fn get_current_fleet(state: tauri::State<'_, crate::AppState>) -> Option<u32> {
    *crate::lock_or_recover(&state.navigation.current_fleet, "current_fleet")
}

#[derive(serde::Serialize)]
pub(crate) struct QuestFilters {
    pub period: Option<String>,
    pub category: Option<String>,
}

/// Get the QuestList sub-screen filters (period × category).
#[tauri::command]
pub(crate) fn get_quest_filters(state: tauri::State<'_, crate::AppState>) -> QuestFilters {
    QuestFilters {
        period: crate::lock_or_recover(
            &state.navigation.current_quest_period,
            "current_quest_period",
        )
        .clone(),
        category: crate::lock_or_recover(
            &state.navigation.current_quest_category,
            "current_quest_category",
        )
        .clone(),
    }
}

/// Get the current Land-Based Air Squadron (基地航空隊) state.
/// Used by the kantai window's 陣形 tab on initial mount; subsequent updates
/// arrive via the `air-base-updated` event.
#[tauri::command]
pub(crate) async fn get_air_bases(
    state: tauri::State<'_, api::models::GameState>,
) -> Result<Vec<api::models::AirBase>, String> {
    let inner = state.inner.read().await;
    Ok(inner.air_bases.clone())
}

/// Get the proxy port for the frontend
#[tauri::command]
pub(crate) fn get_proxy_port(state: tauri::State<'_, crate::AppState>) -> u16 {
    *crate::lock_or_recover(&state.runtime.proxy_port, "proxy_port")
}
