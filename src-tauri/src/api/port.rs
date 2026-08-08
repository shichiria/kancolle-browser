use super::{battle_info, fleet, minimap, models, notify_sync, ship};
use log::{error, info};
use tauri::{AppHandle, Emitter};

// Material ID constants
pub(super) const MATERIAL_FUEL: i32 = 1;
pub(super) const MATERIAL_AMMO: i32 = 2;
pub(super) const MATERIAL_STEEL: i32 = 3;
pub(super) const MATERIAL_BAUXITE: i32 = 4;
pub(super) const MATERIAL_INSTANT_REPAIR: i32 = 5;
pub(super) const MATERIAL_INSTANT_BUILD: i32 = 6;
pub(super) const MATERIAL_DEV_MATERIAL: i32 = 7;
pub(super) const MATERIAL_IMPROVEMENT: i32 = 8;

/// Helper to get a material value by api_id from the material array
pub(super) fn get_material(materials: &[models::Material], id: i32) -> i32 {
    materials
        .iter()
        .find(|m| m.api_id == id)
        .map(|m| m.api_value)
        .unwrap_or(0)
}

pub(super) fn process_start2(state: &mut models::GameStateInner, api_data: &models::ApiStart2) {
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
            .master
            .stypes
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
            .master
            .equip_types
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
}

/// Process api_port data
pub(super) fn process_port(
    state: &mut models::GameStateInner,
    api_data: &models::ApiPort,
    app: &AppHandle,
) {
    crate::action_log::log(
        "State",
        "process_port",
        &format!("ships={}", api_data.api_ship.len()),
    );
    // Finalize active sortie if any
    if state.sortie.battle_logger.is_in_sortie() {
        if let Some(record) = state.sortie.battle_logger.on_port() {
            let filename = format!("battle_logs/{}.json", record.id);
            notify_sync(state, vec![&filename]);
            let summary = crate::battle_log::SortieRecordSummary::from(&record);
            crate::action_log::log("Event", "sortie-complete", &format!("id={}", record.id));
            let _ = app.emit(crate::events::SORTIE_COMPLETE, &summary);
        }
        minimap::hide_minimap_overlay(app);
        battle_info::hide_battle_info_overlay(app);
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
        state
            .profile
            .ships
            .insert(s.api_id, ship::build_ship_info(s, master));
    }

    // Update fleet compositions
    state.profile.fleets.clear();
    for f in &api_data.api_deck_port {
        let ship_ids: Vec<i32> = f.api_ship.iter().filter(|&&id| id > 0).copied().collect();
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
    crate::nozaki_timer::sync(app, state, crate::nozaki_timer::SyncReason::PortRefresh);

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
                    .profile
                    .ships
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

    crate::action_log::log(
        "Event",
        "port-data",
        &format!(
            "admiral={} lv={} ships={}",
            port_data.admiral_name, port_data.admiral_level, port_data.ship_count
        ),
    );
    match app.emit(crate::events::PORT_DATA, &port_data) {
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
        let _ = app.emit(crate::events::SENKA_UPDATED, &summary);
        if changed || checkpoint_crossed {
            notify_sync(state, vec![crate::senka::SenkaTracker::sync_path()]);
        }
    }
}
