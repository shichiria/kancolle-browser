use log::{error, info, warn};
use tauri::{AppHandle, Emitter, Manager};

use super::models;
use super::formation::{formation_name, show_formation_hint, hide_formation_hint};
use super::minimap::update_minimap_overlay;
use super::notify_sync;

/// Check if an endpoint is a battle-related API
pub(super) fn is_battle_endpoint(ep: &str) -> bool {
    ep.starts_with("/kcsapi/api_req_map/")
        || ep.starts_with("/kcsapi/api_req_sortie/")
        || ep.starts_with("/kcsapi/api_req_battle_midnight/")
        || ep.starts_with("/kcsapi/api_req_combined_battle/")
}

/// Process battle-related API endpoints
pub(super) fn process_battle(
    state: &mut models::GameStateInner,
    endpoint: &str,
    request_body: &str,
    json: &serde_json::Value,
    app: &AppHandle,
) {
    info!(
        "Battle API: {} (req_body len={})",
        endpoint,
        request_body.len(),
    );

    match endpoint {
        "/kcsapi/api_req_map/start" => {
            let fleets = state.profile.fleets.clone();
            let player_ships = state.profile.ships.clone();
            let player_slotitems = state.profile.slotitems.clone();
            info!(
                "Sortie start: {} ships in fleet, {} slotitems available",
                player_ships.len(),
                player_slotitems.len(),
            );
            let combined_flag = state.profile.combined_flag;
            state.sortie.battle_logger.on_sortie_start(
                json,
                request_body,
                &fleets,
                &player_ships,
                &player_slotitems,
                combined_flag,
            );
            let _ = app.emit(
                "sortie-start",
                serde_json::json!({
                    "in_sortie": true,
                }),
            );
            // Emit sortie-update with the initial sortie record
            if let Some(sortie) = state.sortie.battle_logger.active_sortie_ref() {
                let summary = crate::battle_log::SortieRecordSummary::from(sortie);
                let _ = app.emit("sortie-update", &summary);

                // Update minimap overlay
                update_minimap_overlay(app, sortie);

                // Show formation hint for first cell if previously visited
                if let Some(node) = sortie.nodes.last() {
                    let key = format!("{}-{}-{}", sortie.map_area, sortie.map_no, node.cell_no);
                    if let Some(&formation) = state.formation_memory.get(&key) {
                        let ship_count = sortie.ships.len();
                        info!("Formation hint: {} -> {} (ships={})", key, formation_name(formation), ship_count);
                        show_formation_hint(app, formation, ship_count);
                    }
                }
            }
        }
        "/kcsapi/api_req_map/next" => {
            // Check for taiha (大破) ships advancing — warn the player
            let mut taiha_shown = false;
            let taiha_enabled = app.try_state::<crate::AppState>()
                .map(|s| s.taiha_alert_enabled.load(std::sync::atomic::Ordering::Relaxed))
                .unwrap_or(true);
            if taiha_enabled {
                if let Some(sortie) = state.sortie.battle_logger.active_sortie_ref() {
                    let fleet_id = sortie.fleet_id as usize;
                    let fleet_idx = fleet_id.saturating_sub(1);
                    // Check main fleet + escort fleet (fleet 2) if combined
                    // Combined fleet always uses fleets 0 (main) and 1 (escort)
                    let fleet_indices: Vec<usize> = if sortie.is_combined {
                        vec![0, 1]
                    } else {
                        vec![fleet_idx]
                    };
                    let mut taiha_names: Vec<String> = Vec::new();
                    for &fi in &fleet_indices {
                        if fi < state.profile.fleets.len() {
                            let ship_ids = &state.profile.fleets[fi];
                            for (i, &ship_id) in ship_ids.iter().enumerate() {
                                if let Some(ship) = state.profile.ships.get(&ship_id) {
                                    if ship.maxhp > 0 && ship.hp as f64 / ship.maxhp as f64 <= 0.25 && ship.hp > 0 {
                                        let has_damecon = ship.slot.iter()
                                            .chain(std::iter::once(&ship.slot_ex))
                                            .any(|&slot_id| {
                                                slot_id > 0 && state.profile.slotitems.get(&slot_id)
                                                    .and_then(|p| state.master.slotitems.get(&p.slotitem_id))
                                                    .map(|m| m.icon_type == 14)
                                                    .unwrap_or(false)
                                            });
                                        if has_damecon {
                                            info!("Ship {} ({}) is taiha but has damecon — skipping warning", ship.name, i);
                                        } else {
                                            warn!("Ship {} ({}) is taiha (HP {}/{}) and advancing without damecon!", ship.name, i, ship.hp, ship.maxhp);
                                            taiha_names.push(ship.name.clone());
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if !taiha_names.is_empty() {
                        taiha_shown = true;
                        warn!("TAIHA ADVANCE WARNING: {} ships critically damaged: {:?}", taiha_names.len(), taiha_names);
                        if let Some(overlay) = app.get_webview("game-overlay") {
                            if let Some(win) = app.get_window("game") {
                                if let Ok(size) = win.inner_size() {
                                    let _ = overlay.set_position(tauri::LogicalPosition::new(0.0, 0.0));
                                    let _ = overlay.set_size(size);
                                }
                            }
                            let ships_json = serde_json::to_string(&taiha_names).unwrap_or_else(|_| "[]".to_string());
                            let _ = overlay.eval(&format!("window.showTaihaWarning({})", ships_json));
                        }
                    }
                }
            } // taiha_enabled

            match serde_json::from_value::<
                models::ApiResponse<crate::api::dto::battle::ApiMapNextResponse>,
            >(json.clone())
            {
                Ok(resp) => {
                    if let Some(data) = resp.api_data {
                        state.sortie.battle_logger.on_map_next(&data, json);

                        // Show formation hint for new cell (skip if taiha warning is active)
                        if !taiha_shown {
                            if let Some(sortie) = state.sortie.battle_logger.active_sortie_ref() {
                                if let Some(node) = sortie.nodes.last() {
                                    let key = format!("{}-{}-{}", sortie.map_area, sortie.map_no, node.cell_no);
                                    if let Some(&formation) = state.formation_memory.get(&key) {
                                        let ship_count = sortie.ships.len();
                                        info!("Formation hint: {} -> {} (ships={})", key, formation_name(formation), ship_count);
                                        show_formation_hint(app, formation, ship_count);
                                    }
                                }
                            }
                        }

                        // Emit sortie-update for minimap real-time tracking
                        if let Some(sortie) = state.sortie.battle_logger.active_sortie_ref() {
                            let summary = crate::battle_log::SortieRecordSummary::from(sortie);
                            let _ = app.emit("sortie-update", &summary);
                            update_minimap_overlay(app, sortie);
                        }

                        // 1-6 goal node detection: event_id 9 = goal reached
                        if data.api_event_id == Some(9) {
                            if let Some(sortie) =
                                state.sortie.battle_logger.active_sortie_ref()
                            {
                                if sortie.map_area == 1 && sortie.map_no == 6 {
                                    let bonus =
                                        crate::senka::eo_bonus_for_map(1, 6);
                                    if bonus > 0 {
                                        info!("1-6 goal reached, EO bonus: {}", bonus);
                                        state.senka.add_eo_bonus(bonus, "1-6");
                                        let summary = state.senka.summary();
                                        let _ = app.emit("senka-updated", &summary);
                                        notify_sync(
                                            &state,
                                            vec![crate::senka::SenkaTracker::sync_path()],
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => error!("Failed to parse map/next response: {}", e),
            }
        }
        // Day battles
        "/kcsapi/api_req_sortie/battle"
        | "/kcsapi/api_req_sortie/airbattle"
        | "/kcsapi/api_req_sortie/ld_airbattle"
        | "/kcsapi/api_req_sortie/ld_shooting"
        | "/kcsapi/api_req_sortie/night_to_day"
        | "/kcsapi/api_req_combined_battle/battle"
        | "/kcsapi/api_req_combined_battle/battle_water"
        | "/kcsapi/api_req_combined_battle/each_battle"
        | "/kcsapi/api_req_combined_battle/each_battle_water"
        | "/kcsapi/api_req_combined_battle/ec_battle"
        | "/kcsapi/api_req_combined_battle/ld_airbattle"
        | "/kcsapi/api_req_combined_battle/ld_shooting" => {
            match serde_json::from_value::<
                models::ApiResponse<crate::api::dto::battle::ApiBattleResponse>,
            >(json.clone())
            {
                Ok(resp) => {
                    if let Some(data) = resp.api_data {
                        state.sortie.battle_logger.on_battle(&data, json);

                        // Save formation to memory and hide hint
                        if let Some(arr) = &data.api_formation {
                            let friendly_formation = arr.first().copied().unwrap_or(0);
                            if friendly_formation > 0 {
                                if let Some(sortie) = state.sortie.battle_logger.active_sortie_ref() {
                                    if let Some(node) = sortie.nodes.last() {
                                        let key = format!("{}-{}-{}", sortie.map_area, sortie.map_no, node.cell_no);
                                        info!("Formation saved: {} = {} ({})", key, friendly_formation, formation_name(friendly_formation));
                                        state.formation_memory.insert(key, friendly_formation);
                                        models::save_formation_memory(&state.formation_memory_path, &state.formation_memory);
                                        notify_sync(&state, vec!["formation_memory.json"]);
                                    }
                                }
                            }
                        }
                        hide_formation_hint(app);
                    }
                }
                Err(e) => error!("Failed to parse battle response: {}", e),
            }
        }
        // Night battles
        "/kcsapi/api_req_battle_midnight/battle"
        | "/kcsapi/api_req_battle_midnight/sp_midnight"
        | "/kcsapi/api_req_combined_battle/midnight_battle"
        | "/kcsapi/api_req_combined_battle/sp_midnight"
        | "/kcsapi/api_req_combined_battle/ec_midnight_battle"
        | "/kcsapi/api_req_combined_battle/ec_night_to_day" => {
            match serde_json::from_value::<
                models::ApiResponse<crate::api::dto::battle::ApiBattleResponse>,
            >(json.clone())
            {
                Ok(resp) => {
                    if let Some(data) = resp.api_data {
                        state.sortie.battle_logger.on_midnight_battle(&data, json);
                    }
                }
                Err(e) => error!("Failed to parse midnight battle response: {}", e),
            }
        }
        // Battle results
        "/kcsapi/api_req_sortie/battleresult" | "/kcsapi/api_req_combined_battle/battleresult" => {
            let master_ships = state.master.ships.clone();

            match serde_json::from_value::<
                models::ApiResponse<crate::api::dto::battle::ApiBattleResultResponse>,
            >(json.clone())
            {
                Ok(resp) => {
                    if let Some(data) = resp.api_data {
                        state
                            .sortie.battle_logger
                            .on_battle_result(&data, json, &master_ships);
                    }
                }
                Err(e) => error!("Failed to parse battleresult response: {}", e),
            }

            // Update player ships HP from battle result and re-emit port-data
            if let Some(sortie) = state.sortie.battle_logger.active_sortie_ref() {
                let fleet_id = sortie.fleet_id as usize;
                let fleet_idx = fleet_id.saturating_sub(1);
                let is_combined = sortie.is_combined;
                // Get friendly HP after battle from the last node's battle detail
                let hp_after: Option<Vec<crate::battle_log::HpState>> = sortie
                    .nodes
                    .last()
                    .and_then(|n| n.battle.as_ref())
                    .map(|b| b.friendly_hp.clone());

                if let Some(hp_states) = &hp_after {
                    // Determine main fleet ship count for splitting HP states in combined fleet
                    let main_fleet_count = if fleet_idx < state.profile.fleets.len() {
                        state.profile.fleets[fleet_idx].len()
                    } else {
                        0
                    };

                    // Update main fleet HP
                    if fleet_idx < state.profile.fleets.len() {
                        let ship_ids = state.profile.fleets[fleet_idx].clone();
                        for (i, &ship_id) in ship_ids.iter().enumerate() {
                            if let (Some(hp_state), Some(ship_info)) =
                                (hp_states.get(i), state.profile.ships.get_mut(&ship_id))
                            {
                                ship_info.hp = hp_state.after.max(0);
                            }
                        }
                        info!(
                            "Updated fleet {} ship HP from battle result ({} ships)",
                            fleet_id,
                            ship_ids.len().min(hp_states.len()),
                        );
                    }

                    // Update escort fleet HP for combined fleet sorties
                    // HP states for combined fleet: indices 0..main_count = main, main_count.. = escort
                    if is_combined && 1 < state.profile.fleets.len() {
                        let escort_ship_ids = state.profile.fleets[1].clone();
                        for (i, &ship_id) in escort_ship_ids.iter().enumerate() {
                            let hp_idx = main_fleet_count + i;
                            if let (Some(hp_state), Some(ship_info)) =
                                (hp_states.get(hp_idx), state.profile.ships.get_mut(&ship_id))
                            {
                                ship_info.hp = hp_state.after.max(0);
                            }
                        }
                        info!(
                            "Updated escort fleet ship HP from battle result ({} ships)",
                            escort_ship_ids.len(),
                        );
                    }
                }

                // Re-emit port-data with updated HP
                if let Some(ref mut cached) = state.sortie.last_port_summary {
                    // Update main fleet ship HP in cached summary
                    if fleet_idx < cached.fleets.len() {
                        if let Some(hp_states) = &hp_after {
                            for (i, ship) in cached.fleets[fleet_idx].ships.iter_mut().enumerate() {
                                if let Some(hp_state) = hp_states.get(i) {
                                    ship.hp = hp_state.after.max(0);
                                }
                            }
                        }
                    }
                    // Update escort fleet HP in cached summary for combined fleet
                    if is_combined && 1 < cached.fleets.len() {
                        if let Some(hp_states) = &hp_after {
                            let main_count = if fleet_idx < cached.fleets.len() {
                                cached.fleets[fleet_idx].ships.len()
                            } else {
                                0
                            };
                            for (i, ship) in cached.fleets[1].ships.iter_mut().enumerate() {
                                if let Some(hp_state) = hp_states.get(main_count + i) {
                                    ship.hp = hp_state.after.max(0);
                                }
                            }
                        }
                    }
                    let _ = app.emit("port-data", &*cached);
                    info!("Re-emitted port-data with updated battle HP");
                }

                // Quest progress: extract map area, rank, boss from active sortie
                let gauge_suffix = match sortie.gauge_num {
                    Some(1) => "(1st)",
                    Some(2) => "(2nd)",
                    Some(3) => "(3rd)",
                    _ => "",
                };
                let map_area_str = format!("{}-{}{}", sortie.map_area, sortie.map_no, gauge_suffix);
                let last_node = sortie.nodes.last();
                let is_boss = last_node.map(|n| n.event_id == 5).unwrap_or(false);
                let rank = last_node
                    .and_then(|n| n.battle.as_ref())
                    .map(|b| b.rank.clone())
                    .unwrap_or_default();

                if !rank.is_empty() {
                    // Extract sunk enemy ship stypes for sinking quests
                    let sunk_enemy_stypes: Vec<i32> = last_node
                        .and_then(|n| n.battle.as_ref())
                        .map(|b| {
                            b.enemy_ships
                                .iter()
                                .zip(b.enemy_hp.iter())
                                .filter(|(_, hp)| hp.after <= 0)
                                .filter_map(|(ship, _)| {
                                    master_ships.get(&ship.ship_id).map(|m| m.stype)
                                })
                                .collect()
                        })
                        .unwrap_or_default();

                    let changed = crate::quest_progress::on_battle_result(
                        &mut state.history.quest_progress,
                        &map_area_str,
                        &rank,
                        is_boss,
                        &sunk_enemy_stypes,
                        &state.history.active_quests,
                        &state.history.sortie_quest_defs,
                        &state.quest_progress_path,
                    );
                    if changed {
                        notify_sync(state, vec!["quest_progress.json"]);
                        let path = state.quest_progress_path.clone();
                        let defs = state.history.sortie_quest_defs.clone();
                        let aq = state.history.active_quests.clone();
                        let progress = crate::quest_progress::get_active_progress(
                            &mut state.history.quest_progress,
                            &aq,
                            &defs,
                            &path,
                        );
                        let _ = app.emit("quest-progress-updated", &progress);
                    }
                }
            }

            // Record per-battle HQ exp (api_get_exp) and check for EO bonus
            if let Some(api_data) = json.get("api_data") {
                let mut senka_changed = false;

                // Record HQ exp from this battle
                let hq_exp = api_data
                    .get("api_get_exp")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                if hq_exp > 0 {
                    let map_display = state
                        .sortie
                        .battle_logger
                        .active_sortie_ref()
                        .map(|s| format!("{}-{}", s.map_area, s.map_no))
                        .unwrap_or_default();
                    state.senka.add_battle_exp(hq_exp, &map_display);
                    senka_changed = true;
                }

                // Check for EO ranking bonus (api_get_exmap_rate)
                if let Some(exmap_rate) = api_data.get("api_get_exmap_rate") {
                    let rate = exmap_rate
                        .as_i64()
                        .or_else(|| exmap_rate.as_str().and_then(|s| s.parse().ok()))
                        .unwrap_or(0);
                    if rate > 0 {
                        let map_display = state
                            .sortie
                            .battle_logger
                            .active_sortie_ref()
                            .map(|s| format!("{}-{}", s.map_area, s.map_no))
                            .unwrap_or_default();
                        state.senka.add_eo_bonus(rate, &map_display);
                        senka_changed = true;
                    }
                }

                if senka_changed {
                    let summary = state.senka.summary();
                    let _ = app.emit("senka-updated", &summary);
                    notify_sync(
                        state,
                        vec![crate::senka::SenkaTracker::sync_path()],
                    );
                }
            }

            // Emit sortie-update event for real-time frontend updates
            if let Some(sortie) = state.sortie.battle_logger.active_sortie_ref() {
                let summary = crate::battle_log::SortieRecordSummary::from(sortie);
                let _ = app.emit("sortie-update", &summary);
                update_minimap_overlay(app, sortie);
            }
        }
        _ => {
            info!("Unhandled battle endpoint: {}", endpoint);
        }
    }
}

/// Process exercise battle result (api_req_practice/battle_result)
pub(super) fn process_exercise_result(
    state: &mut models::GameStateInner,
    json: &serde_json::Value,
    app: &AppHandle,
) {
    let api_data = match json.get("api_data") {
        Some(d) => d,
        None => return,
    };

    let rank = api_data
        .get("api_win_rank")
        .and_then(|v| v.as_str())
        .unwrap_or("-")
        .to_string();

    info!("Exercise result: rank={}", rank);

    // Record HQ exp from exercise
    let hq_exp = api_data
        .get("api_get_exp")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    if hq_exp > 0 {
        state.senka.add_battle_exp(hq_exp, "演習");
        let summary = state.senka.summary();
        let _ = app.emit("senka-updated", &summary);
        notify_sync(state, vec![crate::senka::SenkaTracker::sync_path()]);
    }

    let changed = crate::quest_progress::on_exercise_result(
        &mut state.history.quest_progress,
        &rank,
        &state.history.active_quests,
        &state.history.sortie_quest_defs,
        &state.quest_progress_path,
    );
    if changed {
        notify_sync(state, vec!["quest_progress.json"]);
        let path = state.quest_progress_path.clone();
        let defs = state.history.sortie_quest_defs.clone();
        let aq = state.history.active_quests.clone();
        let progress = crate::quest_progress::get_active_progress(
            &mut state.history.quest_progress,
            &aq,
            &defs,
            &path,
        );
        let _ = app.emit("quest-progress-updated", &progress);
    }
}
