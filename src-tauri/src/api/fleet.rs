use log::{error, info, warn};
use tauri::{AppHandle, Emitter};

use super::models;
use super::ship::{collect_ship_marks, resolve_command_facility};

/// Process api_req_hensei/change - fleet composition change
/// request body has: api_id (fleet 1-4), api_ship_idx (position 0-5), api_ship_id (ship instance ID, -1=remove, -2=remove all except flagship)
pub(super) fn process_hensei_change(
    state: &mut models::GameStateInner,
    fleet_id: usize,
    ship_idx: i32,
    ship_id: i32,
    app: &AppHandle,
) {
    if fleet_id == 0 || fleet_id > state.profile.fleets.len() {
        warn!("hensei/change: invalid fleet_id {}", fleet_id);
        return;
    }
    let fidx = fleet_id - 1;

    if ship_id == -2 {
        // Remove all except flagship
        let flagship = if !state.profile.fleets[fidx].is_empty() {
            state.profile.fleets[fidx][0]
        } else {
            return;
        };
        state.profile.fleets[fidx] = vec![flagship];
        info!(
            "Fleet {} cleared (flagship {} retained)",
            fleet_id, flagship
        );
    } else if ship_id == -1 {
        // Remove ship at index
        let idx = ship_idx as usize;
        if idx < state.profile.fleets[fidx].len() {
            state.profile.fleets[fidx].remove(idx);
            info!("Fleet {} removed ship at index {}", fleet_id, idx);
        }
    } else {
        // Add/swap ship
        let idx = ship_idx as usize;

        // Find the ship being replaced at this position (if any)
        let replaced_id = if idx < state.profile.fleets[fidx].len() {
            state.profile.fleets[fidx][idx]
        } else {
            -1
        };

        // Check if this ship is already in another fleet (or same fleet different position) and swap
        for fi in 0..state.profile.fleets.len() {
            for si in 0..state.profile.fleets[fi].len() {
                if state.profile.fleets[fi][si] == ship_id && !(fi == fidx && si == idx) {
                    // Found the ship in another position — swap or remove
                    if replaced_id > 0 {
                        state.profile.fleets[fi][si] = replaced_id;
                    } else {
                        state.profile.fleets[fi].remove(si);
                    }
                    break;
                }
            }
        }

        // Place the ship at the target position
        while state.profile.fleets[fidx].len() <= idx {
            state.profile.fleets[fidx].push(-1);
        }
        state.profile.fleets[fidx][idx] = ship_id;

        // Remove any -1 gaps from all affected fleets
        for fi in 0..state.profile.fleets.len() {
            state.profile.fleets[fi].retain(|&id| id > 0);
        }

        info!("Fleet {} set index {} to ship {}", fleet_id, idx, ship_id);
    }

    emit_fleet_update(state, app);
}

/// Process api_req_hensei/preset_select - load preset fleet
pub(super) fn process_hensei_preset_select(
    state: &mut models::GameStateInner,
    data: &crate::api::dto::battle::ApiHenseiPresetSelectResponse,
    app: &AppHandle,
) {
    let fleet_id = data
        .api_fleet
        .as_ref()
        .and_then(|f| f.get("api_id"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0) as usize;
    if fleet_id == 0 || fleet_id > state.profile.fleets.len() {
        warn!("preset_select: invalid fleet_id {}", fleet_id);
        return;
    }
    let fidx = fleet_id - 1;

    if let Some(api_ship) = data
        .api_fleet
        .as_ref()
        .and_then(|f| f.get("api_ship"))
        .and_then(|v| v.as_array())
    {
        let ship_ids: Vec<i32> = api_ship
            .iter()
            .filter_map(|v| v.as_i64().map(|id| id as i32))
            .filter(|&id| id > 0)
            .collect();

        // Remove these ships from other fleets (preset load can pull ships)
        for fi in 0..state.profile.fleets.len() {
            if fi == fidx {
                continue;
            }
            state.profile.fleets[fi].retain(|id| !ship_ids.contains(id));
        }

        state.profile.fleets[fidx] = ship_ids;
        info!(
            "Fleet {} loaded from preset: {} ships",
            fleet_id,
            state.profile.fleets[fidx].len()
        );
    }

    emit_fleet_update(state, app);
}

/// Build and emit fleet summaries to the frontend
pub(super) fn emit_fleet_update(state: &models::GameStateInner, app: &AppHandle) {
    let fleets: Vec<models::FleetSummary> = state
        .profile.fleets
        .iter()
        .enumerate()
        .map(|(i, ship_ids)| {
            let mut ships: Vec<models::ShipSummary> = ship_ids
                .iter()
                .filter_map(|&id| {
                    state.profile.ships.get(&id).map(|info| {
                        let marks = collect_ship_marks(
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

            let fleet_id = (i + 1) as i32;
            resolve_command_facility(
                &mut ships,
                fleet_id,
                state.profile.combined_flag,
                &state.profile,
                &state.master.slotitems,
            );

            models::FleetSummary {
                id: fleet_id,
                name: format!("第{}艦隊", i + 1),
                ships,
                expedition: None, // Expedition info not available from hensei APIs
            }
        })
        .collect();

    match app.emit("fleet-updated", &fleets) {
        Ok(_) => info!("fleet-updated event emitted: {} fleets", fleets.len()),
        Err(e) => error!("Failed to emit fleet-updated: {}", e),
    }
}

/// Parse expedition info from a fleet's api_mission array.
pub(super) fn parse_expedition(
    mission: &[serde_json::Value],
    master_missions: &std::collections::HashMap<i32, models::MissionInfo>,
) -> Option<models::ExpeditionInfo> {
    if mission.len() < 4 {
        return None;
    }

    let mission_type = mission[0].as_i64().unwrap_or(0);
    if mission_type == 0 {
        return None;
    }

    let mission_id = mission[1].as_i64().unwrap_or(0) as i32;
    // api_mission format: [type, mission_id, return_time, ?]
    let return_time = mission[2].as_i64().unwrap_or(0);

    let mission_name = master_missions
        .get(&mission_id)
        .map(|m| m.name.clone())
        .unwrap_or_else(|| {
            warn!("Unknown mission ID: {}", mission_id);
            format!("Mission {}", mission_id)
        });

    Some(models::ExpeditionInfo {
        mission_id,
        mission_name,
        return_time,
    })
}
