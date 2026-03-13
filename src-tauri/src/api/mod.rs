pub mod dto;
pub mod models;
mod battle;
mod fleet;
pub(crate) mod formation;
pub(crate) mod minimap;
mod ship;

use dto::request::{HenseiChangeReq, QuestReq, RemodelSlotReq};

#[cfg(test)]
mod tests;
use log::{error, info, warn};
use tauri::{AppHandle, Emitter, Manager};

use models::GameState;

// Re-export public functions used by other crates
pub use formation::hide_formation_hint;
pub use minimap::send_minimap_data;

// Material ID constants
const MATERIAL_FUEL: i32 = 1;
const MATERIAL_AMMO: i32 = 2;
const MATERIAL_STEEL: i32 = 3;
const MATERIAL_BAUXITE: i32 = 4;
const MATERIAL_INSTANT_REPAIR: i32 = 5;
const MATERIAL_INSTANT_BUILD: i32 = 6;
const MATERIAL_DEV_MATERIAL: i32 = 7;
const MATERIAL_IMPROVEMENT: i32 = 8;

/// Send sync notification for changed files.
fn notify_sync(state: &models::GameStateInner, paths: Vec<&str>) {
    if let Some(tx) = &state.sync_notifier {
        let _ = tx.try_send(crate::drive_sync::SyncCommand::UploadChanged(
            paths.into_iter().map(|s| s.to_string()).collect(),
        ));
    }
}

/// Helper to get a material value by api_id from the material array
fn get_material(materials: &[models::Material], id: i32) -> i32 {
    materials
        .iter()
        .find(|m| m.api_id == id)
        .map(|m| m.api_value)
        .unwrap_or(0)
}

/// Pre-parsed API data to pass into the single async task
enum ParsedApi {
    Start2(Box<models::ApiStart2>),
    Port(Box<models::ApiPort>),
    SlotItem(Vec<models::PlayerSlotItemApi>),
    QuestList(crate::api::dto::battle::ApiQuestListResponse),
    Battle(serde_json::Value),
    ExerciseResult(serde_json::Value),
    HenseiChange {
        fleet_id: usize,
        ship_idx: i32,
        ship_id: i32,
    },
    HenseiPresetSelect(crate::api::dto::battle::ApiHenseiPresetSelectResponse),
    RemodelSlot {
        slot_id: i32,
        success: bool,
        eq_id: i32,
    },
    QuestStart {
        quest_id: i32,
    },
    QuestStop {
        quest_id: i32,
    },
    QuestClear {
        quest_id: i32,
        senka_bonus: i64,
    },
    Ship3(serde_json::Value),
    SlotDeprive(serde_json::Value),
    Ranking(String), // raw JSON string for ranking decryption (needs admiral name from state)
    Other,
}

/// Process intercepted KanColle API data.
/// All state updates happen in a SINGLE async task to guarantee ordering.
pub fn process_api(app_handle: &AppHandle, endpoint: &str, json_str: &str, request_body: &str) {
    let game_state = app_handle.state::<GameState>();

    // Parse data on the calling thread (sync) to avoid cloning large json_str
    let parsed = match endpoint {
        "/kcsapi/api_start2/getData" => {
            info!("Processing api_start2/getData (master data)");
            match serde_json::from_str::<models::ApiResponse<models::ApiStart2>>(json_str) {
                Ok(data) => match data.api_data {
                    Some(api_data) => ParsedApi::Start2(Box::new(api_data)),
                    None => ParsedApi::Other,
                },
                Err(e) => {
                    error!("Failed to parse api_start2: {}", e);
                    ParsedApi::Other
                }
            }
        }
        "/kcsapi/api_port/port" => {
            info!("Processing api_port/port (home screen)");
            match serde_json::from_str::<models::ApiResponse<models::ApiPort>>(json_str) {
                Ok(data) => match data.api_data {
                    Some(api_data) => ParsedApi::Port(Box::new(api_data)),
                    None => ParsedApi::Other,
                },
                Err(e) => {
                    error!("Failed to parse api_port: {}", e);
                    ParsedApi::Other
                }
            }
        }
        "/kcsapi/api_get_member/slot_item" => {
            info!("Processing api_get_member/slot_item (player equipment)");
            match serde_json::from_str::<models::ApiResponse<Vec<models::PlayerSlotItemApi>>>(
                json_str,
            ) {
                Ok(data) => match data.api_data {
                    Some(items) => ParsedApi::SlotItem(items),
                    None => ParsedApi::Other,
                },
                Err(e) => {
                    error!("Failed to parse slot_item: {}", e);
                    ParsedApi::Other
                }
            }
        }
        "/kcsapi/api_get_member/require_info" => {
            info!("Processing api_get_member/require_info (includes slot_item)");
            // require_info contains api_slot_item in the same format as api_get_member/slot_item
            match serde_json::from_str::<models::ApiResponse<serde_json::Value>>(json_str) {
                Ok(data) => {
                    if let Some(api_data) = data.api_data {
                        if let Some(items_val) = api_data.get("api_slot_item") {
                            match serde_json::from_value::<Vec<models::PlayerSlotItemApi>>(
                                items_val.clone(),
                            ) {
                                Ok(items) => ParsedApi::SlotItem(items),
                                Err(e) => {
                                    error!("Failed to parse require_info slot_item: {}", e);
                                    ParsedApi::Other
                                }
                            }
                        } else {
                            ParsedApi::Other
                        }
                    } else {
                        ParsedApi::Other
                    }
                }
                Err(e) => {
                    error!("Failed to parse require_info: {}", e);
                    ParsedApi::Other
                }
            }
        }
        "/kcsapi/api_get_member/questlist" => {
            info!("Processing api_get_member/questlist");
            match serde_json::from_str::<
                models::ApiResponse<crate::api::dto::battle::ApiQuestListResponse>,
            >(json_str)
            {
                Ok(data) => match data.api_data {
                    Some(api_data) => ParsedApi::QuestList(api_data),
                    None => ParsedApi::Other,
                },
                Err(e) => {
                    error!("Failed to parse questlist: {}", e);
                    ParsedApi::Other
                }
            }
        }
        "/kcsapi/api_req_hensei/change" => {
            info!("Processing api_req_hensei/change (fleet composition change)");
            match serde_urlencoded::from_str::<HenseiChangeReq>(request_body) {
                Ok(req) => ParsedApi::HenseiChange {
                    fleet_id: req.api_id,
                    ship_idx: req.api_ship_idx,
                    ship_id: req.api_ship_id,
                },
                Err(e) => {
                    error!("Failed to parse hensei/change req: {}", e);
                    ParsedApi::Other
                }
            }
        }
        "/kcsapi/api_req_hensei/preset_select" => {
            info!("Processing api_req_hensei/preset_select (preset fleet load)");
            match serde_json::from_str::<
                models::ApiResponse<crate::api::dto::battle::ApiHenseiPresetSelectResponse>,
            >(json_str)
            {
                Ok(data) => match data.api_data {
                    Some(api_data) => ParsedApi::HenseiPresetSelect(api_data),
                    None => ParsedApi::Other,
                },
                Err(e) => {
                    error!("Failed to parse preset_select: {}", e);
                    ParsedApi::Other
                }
            }
        }
        "/kcsapi/api_req_kousyou/remodel_slot" => {
            info!("Processing api_req_kousyou/remodel_slot (equipment improvement)");
            let req = serde_urlencoded::from_str::<RemodelSlotReq>(request_body).ok();
            let slot_id = req.as_ref().map(|r| r.api_slot_id).unwrap_or(-1);
            let req_eq_id = req.as_ref().map(|r| r.api_id).unwrap_or(-1);

            // Extract eq_id + success from response
            let (success, resp_eq_id) = match serde_json::from_str::<
                models::ApiResponse<crate::api::dto::battle::ApiRemodelSlotResponse>,
            >(json_str)
            {
                Ok(data) => {
                    let api_data = &data.api_data;
                    let flag = api_data.as_ref().and_then(|d| d.api_remodel_flag);
                    // Get master eq_id from api_after_slot.api_slotitem_id in response
                    let mut eq_id = api_data
                        .as_ref()
                        .and_then(|d| d.api_after_slot.as_ref())
                        .and_then(|s| s.api_slotitem_id)
                        .unwrap_or(-1) as i32;

                    if eq_id <= 0 {
                        eq_id = req_eq_id; // Fallback to request body's api_id
                    }
                    info!(
                        "remodel_slot: slot_id={}, resp_eq_id={}, flag={:?}",
                        slot_id, eq_id, flag
                    );
                    (flag.map(|f| f == 1).unwrap_or(false), eq_id)
                }
                Err(e) => {
                    error!("Failed to parse remodel_slot response: {}", e);
                    (false, -1)
                }
            };
            ParsedApi::RemodelSlot {
                slot_id,
                success,
                eq_id: resp_eq_id,
            }
        }
        "/kcsapi/api_req_quest/start" => {
            info!("Processing {} (quest started)", endpoint);
            let req = serde_urlencoded::from_str::<QuestReq>(request_body).ok();
            let quest_id = req.map(|r| r.api_quest_id).unwrap_or(0);
            ParsedApi::QuestStart { quest_id }
        }
        "/kcsapi/api_req_quest/stop" => {
            info!("Processing {} (quest cancelled)", endpoint);
            let req = serde_urlencoded::from_str::<QuestReq>(request_body).ok();
            let quest_id = req.map(|r| r.api_quest_id).unwrap_or(0);
            ParsedApi::QuestStop { quest_id }
        }
        "/kcsapi/api_req_quest/clearitemget" => {
            info!("Processing {} (quest completed)", endpoint);
            let req = serde_urlencoded::from_str::<QuestReq>(request_body).ok();
            let quest_id = req.map(|r| r.api_quest_id).unwrap_or(0);
            // Parse response to extract senka bonus from api_bounus
            let senka_bonus = extract_senka_from_clearitemget(json_str);
            ParsedApi::QuestClear {
                quest_id,
                senka_bonus,
            }
        }
        "/kcsapi/api_req_practice/battle_result" => {
            info!("Processing api_req_practice/battle_result (exercise result)");
            match serde_json::from_str::<serde_json::Value>(json_str) {
                Ok(v) => ParsedApi::ExerciseResult(v),
                Err(e) => {
                    error!("Failed to parse exercise battle_result: {}", e);
                    ParsedApi::Other
                }
            }
        }
        "/kcsapi/api_get_member/ship3" => {
            info!("Processing api_get_member/ship3 (ship data after equipment change)");
            match serde_json::from_str::<models::ApiResponse<serde_json::Value>>(json_str) {
                Ok(data) => match data.api_data {
                    Some(api_data) => ParsedApi::Ship3(api_data),
                    None => ParsedApi::Other,
                },
                Err(e) => {
                    error!("Failed to parse ship3: {}", e);
                    ParsedApi::Other
                }
            }
        }
        "/kcsapi/api_req_kaisou/slot_deprive" => {
            info!("Processing api_req_kaisou/slot_deprive (equipment transfer between ships)");
            match serde_json::from_str::<models::ApiResponse<serde_json::Value>>(json_str) {
                Ok(data) => match data.api_data {
                    Some(api_data) => ParsedApi::SlotDeprive(api_data),
                    None => ParsedApi::Other,
                },
                Err(e) => {
                    error!("Failed to parse slot_deprive: {}", e);
                    ParsedApi::Other
                }
            }
        }
        "/kcsapi/api_req_ranking/mxltvkpyuklh" => {
            info!("Processing api_req_ranking/mxltvkpyuklh (ranking data)");
            ParsedApi::Ranking(json_str.to_string())
        }
        ep if battle::is_battle_endpoint(ep) => match serde_json::from_str::<serde_json::Value>(json_str) {
            Ok(v) => ParsedApi::Battle(v),
            Err(e) => {
                let preview: String = json_str.chars().take(200).collect();
                error!(
                    "Failed to parse battle API JSON for {}: {} (len={}, first 200: {:?})",
                    ep,
                    e,
                    json_str.len(),
                    preview
                );
                ParsedApi::Other
            }
        },
        _ => {
            info!("Unhandled API endpoint: {}", endpoint);
            ParsedApi::Other
        }
    };

    // Single async task: raw save + state update (guarantees ordering)
    let inner = game_state.inner.clone();
    let endpoint = endpoint.to_string();
    let request_body = request_body.to_string();
    let json_str = json_str.to_string();
    let app = app_handle.clone();

    tauri::async_runtime::spawn(async move {
        // Step 1: Briefly lock to allocate filename + seq number (no I/O)
        let raw_info = {
            let mut state = inner.write().await;
            state.sortie.battle_logger.allocate_raw_api_filename(&endpoint)
        };

        // Step 2: Write raw API dump to disk OUTSIDE the lock
        let raw_filename = if let Some((dir, filename)) = raw_info {
            if crate::battle_log::save_raw_api_to_disk(
                &dir,
                &filename,
                &endpoint,
                &request_body,
                &json_str,
            ) {
                Some(filename)
            } else {
                None
            }
        } else {
            None
        };

        // Step 3: Re-acquire lock for state updates
        let mut state = inner.write().await;

        // Notify sync engine about new raw API file
        if let (Some(filename), Some(tx)) = (&raw_filename, &state.sync_notifier) {
            let path = format!("raw_api/{}", filename);
            let _ = tx.try_send(crate::drive_sync::SyncCommand::UploadChanged(vec![path]));
        }

        match parsed {
            ParsedApi::Start2(api_data) => {
                process_start2(&mut state, &api_data, &app);
            }
            ParsedApi::Port(api_data) => {
                process_port(&mut state, &api_data, &app);
            }
            ParsedApi::SlotItem(items) => {
                let count = items.len();
                state.profile.slotitems.clear();
                for item in &items {
                    state.profile.slotitems.insert(
                        item.api_id,
                        models::PlayerSlotItem {
                            item_id: item.api_id,
                            slotitem_id: item.api_slotitem_id,
                            level: item.api_level,
                            alv: item.api_alv,
                            locked: item.api_locked == 1,
                        },
                    );
                }
                info!("GameState updated: {} player slot items", count);
            }
            ParsedApi::QuestList(json) => {
                process_questlist(&mut state, &json, &app);
            }
            ParsedApi::Battle(json) => {
                battle::process_battle(&mut state, &endpoint, &request_body, &json, &app);
            }
            ParsedApi::ExerciseResult(json) => {
                battle::process_exercise_result(&mut state, &json, &app);
            }
            ParsedApi::HenseiChange {
                fleet_id,
                ship_idx,
                ship_id,
            } => {
                fleet::process_hensei_change(&mut state, fleet_id, ship_idx, ship_id, &app);
            }
            ParsedApi::HenseiPresetSelect(json) => {
                fleet::process_hensei_preset_select(&mut state, &json, &app);
            }
            ParsedApi::RemodelSlot {
                slot_id,
                success,
                eq_id,
            } => {
                if success {
                    // Use eq_id from request body (api_id param), fallback to player_slotitems lookup
                    let resolved_eq_id = if eq_id > 0 {
                        eq_id
                    } else if slot_id > 0 {
                        state
                            .profile.slotitems
                            .get(&slot_id)
                            .map(|item| item.slotitem_id)
                            .unwrap_or(-1)
                    } else {
                        -1
                    };
                    if resolved_eq_id > 0 {
                        state.history.improved_equipment.insert(resolved_eq_id);
                        crate::improvement::save_improved_history(
                            &state.improved_equipment_path,
                            &state.history.improved_equipment,
                        );
                        notify_sync(&state, vec!["improved_equipment.json"]);
                        info!(
                            "Equipment improved: eq_id={} (instance={})",
                            resolved_eq_id, slot_id
                        );
                    } else {
                        warn!(
                            "remodel_slot success but could not resolve eq_id: slot_id={}, req_eq_id={}",
                            slot_id, eq_id
                        );
                    }
                }
            }
            ParsedApi::QuestStart { quest_id } => {
                if quest_id > 0 {
                    state.history.active_quests.insert(quest_id);
                    info!("Quest {} started", quest_id);
                    let _ = app.emit("quest-started", quest_id);
                }
            }
            ParsedApi::QuestStop { quest_id } => {
                if quest_id > 0 {
                    state.history.active_quests.remove(&quest_id);
                    state.history.active_quest_details.remove(&quest_id);
                    let details: Vec<&models::ActiveQuestDetail> =
                        state.history.active_quest_details.values().collect();
                    info!(
                        "Quest {} cancelled, {} active quests remaining",
                        quest_id,
                        details.len()
                    );
                    let _ = app.emit("quest-list-updated", &details);
                    let _ = app.emit("quest-stopped", quest_id);
                }
            }
            ParsedApi::QuestClear {
                quest_id,
                senka_bonus,
            } => {
                if quest_id > 0 {
                    state.history.active_quests.remove(&quest_id);
                    state.history.active_quest_details.remove(&quest_id);
                    let details: Vec<&models::ActiveQuestDetail> =
                        state.history.active_quest_details.values().collect();
                    info!(
                        "Quest {} completed (senka bonus: {}), {} active quests remaining",
                        quest_id,
                        senka_bonus,
                        details.len()
                    );
                    let _ = app.emit("quest-list-updated", &details);
                    let _ = app.emit("quest-stopped", quest_id);

                    // Add senka bonus if present
                    if senka_bonus > 0 {
                        state.senka.add_quest_bonus(senka_bonus, quest_id);
                        let summary = state.senka.summary();
                        let _ = app.emit("senka-updated", &summary);
                        notify_sync(
                            &state,
                            vec![crate::senka::SenkaTracker::sync_path()],
                        );
                    }
                }
            }
            ParsedApi::Ship3(api_data) => {
                ship::process_ship3(&mut state, &api_data, &app);
            }
            ParsedApi::SlotDeprive(api_data) => {
                ship::process_slot_deprive(&mut state, &api_data, &app);
            }
            ParsedApi::Ranking(raw_json) => {
                // Get admiral name from cached port data
                let admiral_name = state
                    .sortie
                    .last_port_summary
                    .as_ref()
                    .map(|p| p.admiral_name.clone())
                    .unwrap_or_default();

                if admiral_name.is_empty() {
                    warn!("Ranking: admiral name not available, skipping decryption");
                } else {
                    let (entries, own_senka) =
                        crate::senka::decrypt_ranking(&raw_json, &admiral_name);

                    if let Some(senka) = own_senka {
                        state.senka.confirm_ranking(senka);
                        let summary = state.senka.summary();
                        let _ = app.emit("senka-updated", &summary);
                        notify_sync(
                            &state,
                            vec![crate::senka::SenkaTracker::sync_path()],
                        );
                    } else if !entries.is_empty() {
                        info!(
                            "Ranking: decoded {} entries but own admiral '{}' not found in this page",
                            entries.len(),
                            admiral_name
                        );
                    }
                }
            }
            ParsedApi::Other => {}
        }
    });
}

/// Extract senka bonus from clearitemget response's api_bounus array
fn extract_senka_from_clearitemget(json_str: &str) -> i64 {
    let parsed: Result<models::ApiResponse<serde_json::Value>, _> =
        serde_json::from_str(json_str);
    let api_data = match parsed {
        Ok(resp) => match resp.api_data {
            Some(d) => d,
            None => return 0,
        },
        Err(_) => return 0,
    };

    let bounus = match api_data.get("api_bounus").and_then(|v| v.as_array()) {
        Some(arr) => arr,
        None => return 0,
    };

    let mut total_bonus = 0i64;
    for item in bounus {
        if item.is_null() {
            continue;
        }
        let api_type = item.get("api_type").and_then(|v| v.as_i64()).unwrap_or(0);
        if api_type == 18 {
            // Ranking points bonus
            let api_count = item.get("api_count").and_then(|v| v.as_i64()).unwrap_or(1);
            let api_id = item
                .get("api_item")
                .and_then(|i| i.get("api_id"))
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let bonus_per = crate::senka::senka_item_bonus(api_id);
            total_bonus += bonus_per * api_count;
            info!(
                "clearitemget: senka bonus detected: api_id={}, count={}, bonus={}",
                api_id,
                api_count,
                bonus_per * api_count
            );
        }
    }
    total_bonus
}

/// Process api_start2 master data
fn process_start2(
    state: &mut models::GameStateInner,
    api_data: &models::ApiStart2,
    app: &AppHandle,
) {
    // Populate master ships (name + stype)
    state.master.ships.clear();
    for s in &api_data.api_mst_ship {
        state.master.ships.insert(
            s.api_id,
            models::MasterShipInfo {
                name: s.api_name.clone(),
                stype: s.api_stype,
            },
        );
    }

    // Populate master stypes
    state.master.stypes.clear();
    for stype in &api_data.api_mst_stype {
        state
            .master.stypes
            .insert(stype.api_id, stype.api_name.clone());
    }

    // Populate master missions
    state.master.missions.clear();
    for mission in &api_data.api_mst_mission {
        state.master.missions.insert(
            mission.api_id,
            models::MissionInfo {
                name: mission.api_name.clone(),
                time: mission.api_time,
            },
        );
    }

    // Populate master slotitems
    state.master.slotitems.clear();
    for item in &api_data.api_mst_slotitem {
        let type_arr = item.api_type.as_array();
        let item_type = type_arr
            .and_then(|arr| arr.get(2))
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;
        let icon_type = type_arr
            .and_then(|arr| arr.get(3))
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;
        state.master.slotitems.insert(
            item.api_id,
            models::MasterSlotItemInfo {
                name: item.api_name.clone(),
                item_type,
                icon_type,
                firepower: item.api_houg,
                torpedo: item.api_raig,
                bombing: item.api_baku,
                aa: item.api_tyku,
                asw: item.api_tais,
                los: item.api_saku,
            },
        );
    }

    // Populate master equip types
    state.master.equip_types.clear();
    for et in &api_data.api_mst_slotitem_equiptype {
        state
            .master.equip_types
            .insert(et.api_id, et.api_name.clone());
    }

    info!(
        "GameState updated: {} master ships, {} stypes, {} missions, {} slotitems, {} equip_types",
        state.master.ships.len(),
        state.master.stypes.len(),
        state.master.missions.len(),
        state.master.slotitems.len(),
        state.master.equip_types.len(),
    );

    let _ = app.emit(
        "master-data-loaded",
        serde_json::json!({
            "shipCount": state.master.ships.len(),
            "stypeCount": state.master.stypes.len(),
            "missionCount": state.master.missions.len(),
            "equipCount": state.master.slotitems.len(),
        }),
    );
}

/// Process api_port data
fn process_port(state: &mut models::GameStateInner, api_data: &models::ApiPort, app: &AppHandle) {
    // Finalize active sortie if any
    if state.sortie.battle_logger.is_in_sortie() {
        if let Some(record) = state.sortie.battle_logger.on_port() {
            let filename = format!("battle_logs/{}.json", record.id);
            notify_sync(state, vec![&filename]);
            let summary = crate::battle_log::SortieRecordSummary::from(&record);
            let _ = app.emit("sortie-complete", &summary);
        }
        minimap::hide_minimap_overlay(app);
    }

    // Check quest progress resets on returning to port
    crate::quest_progress::check_resets(
        &mut state.history.quest_progress,
        &state.history.sortie_quest_defs,
        &state.quest_progress_path,
    );

    // Update player ships from port data
    state.profile.ships.clear();
    for s in &api_data.api_ship {
        let master = state.master.ships.get(&s.api_ship_id);
        state.profile.ships.insert(
            s.api_id,
            ship::build_ship_info(s, master),
        );
    }

    // Update fleet compositions
    state.profile.fleets.clear();
    for f in &api_data.api_deck_port {
        let ship_ids: Vec<i32> = f
            .api_ship
            .iter()
            .filter(|&&id| id > 0)
            .copied()
            .collect();
        while state.profile.fleets.len() < f.api_id as usize {
            state.profile.fleets.push(Vec::new());
        }
        state.profile.fleets[f.api_id as usize - 1] = ship_ids;
    }

    // Update combined fleet flag
    state.profile.combined_flag = api_data.api_combined_flag;

    info!(
        "GameState updated: {} player ships, {} slotitems in memory",
        state.profile.ships.len(),
        state.profile.slotitems.len(),
    );

    // Build enriched fleet summaries
    let fleets: Vec<models::FleetSummary> = api_data
        .api_deck_port
        .iter()
        .map(|f| {
            let mut ships: Vec<models::ShipSummary> = f
                .api_ship
                .iter()
                .filter(|&&id| id > 0)
                .filter_map(|&id| {
                    state.profile.ships.get(&id).map(|info| {
                        let marks = ship::collect_ship_marks(
                            info,
                            &state.profile.slotitems,
                            &state.master.slotitems,
                        );
                        models::ShipSummary {
                            id,
                            name: info.name.clone(),
                            lv: info.lv,
                            hp: info.hp,
                            maxhp: info.maxhp,
                            cond: info.cond,
                            fuel: info.fuel,
                            bull: info.bull,
                            damecon_name: marks.damecon_name,
                            command_facility_name: None,
                            special_equips: marks.special_equips,
                            can_opening_asw: marks.can_opening_asw,
                            soku: info.soku,
                        }
                    })
                })
                .collect();

            ship::resolve_command_facility(
                &mut ships,
                f.api_id,
                state.profile.combined_flag,
                &state.profile,
                &state.master.slotitems,
            );

            let expedition = fleet::parse_expedition(&f.api_mission, &state.master.missions);

            models::FleetSummary {
                id: f.api_id,
                name: f.api_name.clone(),
                ships,
                expedition,
            }
        })
        .collect();

    // Build enriched dock summaries
    let ndock: Vec<models::DockSummary> = api_data
        .api_ndock
        .iter()
        .map(|dock| {
            let ship_name = if dock.api_ship_id > 0 {
                state
                    .profile.ships
                    .get(&dock.api_ship_id)
                    .map(|info| info.name.clone())
                    .unwrap_or_else(|| format!("Unknown({})", dock.api_ship_id))
            } else {
                String::new()
            };

            models::DockSummary {
                id: dock.api_id,
                state: dock.api_state,
                ship_id: dock.api_ship_id,
                ship_name,
                complete_time: dock.api_complete_time,
            }
        })
        .collect();

    let port_data = models::PortSummary {
        admiral_name: api_data.api_basic.api_nickname.clone(),
        admiral_level: api_data.api_basic.api_level,
        admiral_rank: api_data.api_basic.api_rank,
        ship_count: api_data.api_ship.len(),
        ship_capacity: api_data.api_basic.api_max_chara,
        fuel: get_material(&api_data.api_material, MATERIAL_FUEL),
        ammo: get_material(&api_data.api_material, MATERIAL_AMMO),
        steel: get_material(&api_data.api_material, MATERIAL_STEEL),
        bauxite: get_material(&api_data.api_material, MATERIAL_BAUXITE),
        instant_repair: get_material(&api_data.api_material, MATERIAL_INSTANT_REPAIR),
        instant_build: get_material(&api_data.api_material, MATERIAL_INSTANT_BUILD),
        dev_material: get_material(&api_data.api_material, MATERIAL_DEV_MATERIAL),
        improvement_material: get_material(&api_data.api_material, MATERIAL_IMPROVEMENT),
        fleets,
        ndock,
    };

    info!(
        "Port data: Admiral {} Lv.{}, {} ships",
        port_data.admiral_name, port_data.admiral_level, port_data.ship_count
    );

    // Cache for re-emitting during sortie
    state.sortie.last_port_summary = Some(port_data.clone());

    match app.emit("port-data", &port_data) {
        Ok(_) => info!("port-data event emitted successfully"),
        Err(e) => error!("Failed to emit port-data: {}", e),
    }

    // Update senka tracker with HQ experience
    let hq_exp = match &api_data.api_basic.api_experience {
        serde_json::Value::Number(n) => n.as_i64().unwrap_or(0),
        serde_json::Value::Array(arr) => arr.first().and_then(|v| v.as_i64()).unwrap_or(0),
        _ => 0,
    };
    if hq_exp > 0 {
        let (changed, checkpoint_crossed) = state.senka.update_experience(hq_exp);
        let summary = state.senka.summary_with_checkpoint(checkpoint_crossed);
        let _ = app.emit("senka-updated", &summary);
        if changed || checkpoint_crossed {
            notify_sync(state, vec![crate::senka::SenkaTracker::sync_path()]);
        }
    }
}

/// Process api_get_member/questlist data
/// api_list contains quest objects (with api_no, api_state) or 0 values (gaps).
/// api_state: 1=not accepted, 2=accepted/in progress, 3=completed
fn process_questlist(
    state: &mut models::GameStateInner,
    data: &crate::api::dto::battle::ApiQuestListResponse,
    app: &AppHandle,
) {
    if let Some(api_list) = data.api_list.as_ref() {
        for item in api_list {
            let api_no = match item.get("api_no").and_then(|v| v.as_i64()) {
                Some(n) => n as i32,
                None => continue, // skip 0 / null entries
            };
            let api_state = item.get("api_state").and_then(|v| v.as_i64()).unwrap_or(0) as i32;

            match api_state {
                2 | 3 => {
                    // Accepted or completed -> add to active set
                    state.history.active_quests.insert(api_no);
                    let title = item
                        .get("api_title")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let category = item
                        .get("api_category")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0) as i32;
                    state.history.active_quest_details.insert(
                        api_no,
                        models::ActiveQuestDetail {
                            id: api_no,
                            title,
                            category,
                        },
                    );
                }
                1 => {
                    // Not accepted -> remove from active set
                    state.history.active_quests.remove(&api_no);
                    state.history.active_quest_details.remove(&api_no);
                }
                _ => {}
            }
        }

        let details: Vec<&models::ActiveQuestDetail> =
            state.history.active_quest_details.values().collect();
        info!("Active quests updated: {} quests", details.len());
        let _ = app.emit("quest-list-updated", &details);
    }
}
