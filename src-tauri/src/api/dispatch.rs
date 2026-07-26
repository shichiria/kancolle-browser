use super::parse::ParsedApi;
use super::port::{
    get_material, MATERIAL_AMMO, MATERIAL_BAUXITE, MATERIAL_DEV_MATERIAL, MATERIAL_FUEL,
    MATERIAL_IMPROVEMENT, MATERIAL_INSTANT_BUILD, MATERIAL_INSTANT_REPAIR, MATERIAL_STEEL,
};
use super::{air_corps, battle, fleet, models, port, quest, ship};
use log::{info, warn};
use tauri::{AppHandle, Emitter};

pub(super) fn apply(
    state: &mut models::GameStateInner,
    parsed: ParsedApi,
    endpoint: &str,
    request_body: &str,
    app: &AppHandle,
) {
    #[cfg(debug_assertions)]
    {
        macro_rules! variant_name {
            ($val:expr, $($variant:ident),+ $(,)?) => {
                match $val {
                    $(ParsedApi::$variant { .. } => stringify!($variant),)+
                }
            };
        }
        let variant_name = variant_name!(
            &parsed,
            Start2,
            Port,
            SlotItem,
            QuestList,
            Battle,
            ExerciseResult,
            HenseiChange,
            HenseiPresetSelect,
            RemodelSlot,
            QuestStart,
            QuestStop,
            QuestClear,
            Ship3,
            SlotDeprive,
            Charge,
            Ranking,
            Powerup,
            SlotExchange,
            GetShip,
            DestroyItem2,
            DestroyShip,
            CreateItem,
            MemberMaterial,
            MemberNDock,
            MemberDeck,
            MissionResult,
            MapInfoData,
            EventMapRankSelected,
            BaseAirCorps,
            AirCorpsSetPlane,
            AirCorpsSetAction,
            AirCorpsSupply,
            AirCorpsChangeName,
            AirCorpsChangeDeployment,
            AirCorpsCondRecovery,
            LogOnly,
            Other,
        );
        crate::action_log::log("API_PARSED", endpoint, &format!("variant={}", variant_name));
    }

    match parsed {
        ParsedApi::Start2(api_data) => {
            port::process_start2(state, &api_data);
        }
        ParsedApi::Port(api_data) => {
            port::process_port(state, &api_data, app);
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
            quest::process_questlist(state, &json, app);
        }
        ParsedApi::Battle(json) => {
            battle::process_battle(state, endpoint, request_body, &json, app);
        }
        ParsedApi::ExerciseResult(api_data) => {
            battle::process_exercise_result(state, &api_data, app);
        }
        ParsedApi::HenseiChange {
            fleet_id,
            ship_idx,
            ship_id,
        } => {
            fleet::process_hensei_change(state, fleet_id, ship_idx, ship_id, app);
        }
        ParsedApi::HenseiPresetSelect(json) => {
            fleet::process_hensei_preset_select(state, &json, app);
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
                        .profile
                        .slotitems
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
                    super::notify_sync(state, vec!["improved_equipment.json"]);
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
                let _ = app.emit(crate::events::QUEST_STARTED, quest_id);
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
                let _ = app.emit(crate::events::QUEST_LIST_UPDATED, &details);
                let _ = app.emit(crate::events::QUEST_STOPPED, quest_id);
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
                let _ = app.emit(crate::events::QUEST_LIST_UPDATED, &details);
                let _ = app.emit(crate::events::QUEST_STOPPED, quest_id);

                // Add senka bonus if present
                if senka_bonus > 0 {
                    state.senka.add_quest_bonus(senka_bonus, quest_id);
                    let summary = state.senka.summary();
                    let _ = app.emit(crate::events::SENKA_UPDATED, &summary);
                    super::notify_sync(state, vec![crate::senka::SenkaTracker::sync_path()]);
                }
            }
        }
        ParsedApi::Charge(api_data) => {
            ship::process_charge(state, &api_data, app);
        }
        ParsedApi::Ship3(api_data) => {
            ship::process_ship3(state, &api_data, app);
        }
        ParsedApi::SlotDeprive(api_data) => {
            ship::process_slot_deprive(state, &api_data, app);
        }
        ParsedApi::Ranking(ranking_data) => {
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
                    crate::senka::decrypt_ranking(&ranking_data, &admiral_name);

                if let Some(senka) = own_senka {
                    state.senka.confirm_ranking(senka);
                    let summary = state.senka.summary();
                    let _ = app.emit(crate::events::SENKA_UPDATED, &summary);
                    super::notify_sync(state, vec![crate::senka::SenkaTracker::sync_path()]);
                } else if !entries.is_empty() {
                    info!(
                        "Ranking: decoded {} entries but own admiral '{}' not found in this page",
                        entries.len(),
                        admiral_name
                    );
                }
            }
        }
        // --- Category B handlers ---
        ParsedApi::Powerup(api_data) => {
            ship::process_powerup(state, &api_data, app);
        }
        ParsedApi::SlotExchange(api_data) => {
            ship::process_slot_exchange(state, &api_data, app);
        }
        ParsedApi::GetShip(api_data) => {
            ship::process_getship(state, &api_data, app);
        }
        ParsedApi::DestroyItem2 { item_ids } => {
            for &id in &item_ids {
                state.profile.slotitems.remove(&id);
            }
            info!("destroyitem2: removed {} equipment items", item_ids.len());
        }
        ParsedApi::DestroyShip { ship_id } => {
            if ship_id > 0 {
                state.profile.ships.remove(&ship_id);
                for fleet in &mut state.profile.fleets {
                    fleet.retain(|&id| id != ship_id);
                }
                info!("destroyship: removed ship {}", ship_id);
            }
            fleet::emit_fleet_update(state, app);
        }
        ParsedApi::CreateItem(api_data) => {
            if api_data.api_create_flag == 1 {
                // Add new items from api_get_items
                for item_val in &api_data.api_get_items {
                    if let Some(slot) = item_val.get("api_slotitem") {
                        if let Ok(item) =
                            serde_json::from_value::<models::PlayerSlotItemApi>(slot.clone())
                        {
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
                    }
                }
                info!(
                    "createitem: success, added {} items",
                    api_data.api_get_items.len()
                );
            } else {
                info!("createitem: development failed");
            }
        }
        ParsedApi::MemberMaterial(materials) => {
            // Update cached port summary if available
            if let Some(ref mut cached) = state.sortie.last_port_summary {
                cached.fuel = get_material(&materials, MATERIAL_FUEL);
                cached.ammo = get_material(&materials, MATERIAL_AMMO);
                cached.steel = get_material(&materials, MATERIAL_STEEL);
                cached.bauxite = get_material(&materials, MATERIAL_BAUXITE);
                cached.instant_repair = get_material(&materials, MATERIAL_INSTANT_REPAIR);
                cached.instant_build = get_material(&materials, MATERIAL_INSTANT_BUILD);
                cached.dev_material = get_material(&materials, MATERIAL_DEV_MATERIAL);
                cached.improvement_material = get_material(&materials, MATERIAL_IMPROVEMENT);
                let _ = app.emit(crate::events::PORT_DATA, &*cached);
            }
            info!("material: updated resource values");
        }
        ParsedApi::MemberNDock(ndock) => {
            // Build dock summaries before borrowing cached summary mutably
            let dock_summaries: Vec<models::DockSummary> = ndock
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
            if let Some(ref mut cached) = state.sortie.last_port_summary {
                cached.ndock = dock_summaries;
                let _ = app.emit(crate::events::PORT_DATA, &*cached);
            }
            info!("ndock: updated {} repair docks", ndock.len());
        }
        ParsedApi::MemberDeck(decks) => {
            for fleet in &decks {
                let ship_ids: Vec<i32> = fleet
                    .api_ship
                    .iter()
                    .filter(|&&id| id > 0)
                    .copied()
                    .collect();
                let fidx = fleet.api_id as usize;
                while state.profile.fleets.len() < fidx {
                    state.profile.fleets.push(Vec::new());
                }
                if fidx > 0 {
                    state.profile.fleets[fidx - 1] = ship_ids;
                }
            }
            info!("deck: updated {} fleets", decks.len());
            fleet::emit_fleet_update(state, app);
        }
        ParsedApi::MissionResult(api_data) => {
            info!(
                "mission/result: clear_result={}, exp={}, materials={:?}",
                api_data.api_clear_result, api_data.api_get_exp, api_data.api_get_material
            );
        }
        ParsedApi::MapInfoData {
            gauges,
            event_maps,
            air_bases,
        } => {
            state.mapinfo_gauges = gauges;
            state.event_map_statuses = event_maps;
            info!(
                "mapinfo: cached {} gauge entries and {} event maps",
                state.mapinfo_gauges.len(),
                state.event_map_statuses.len()
            );
            let _ = app.emit(
                crate::events::EVENT_MAP_UPDATED,
                &state.event_map_statuses,
            );
            let bases = air_corps::parse_air_bases(
                &air_bases,
                &state.profile.slotitems,
                &state.master.slotitems,
                &state.air_bases,
            );
            if !bases.is_empty() {
                state.air_bases = bases;
                info!("mapinfo: parsed {} air-base entries", state.air_bases.len());
                air_corps::emit_air_base_update(state, app);
            }
        }
        ParsedApi::EventMapRankSelected(status) => {
            if let Some(existing) = state
                .event_map_statuses
                .iter_mut()
                .find(|existing| existing.map_id == status.map_id)
            {
                *existing = status;
            } else {
                state.event_map_statuses.push(status);
            }
            let _ = app.emit(
                crate::events::EVENT_MAP_UPDATED,
                &state.event_map_statuses,
            );
        }
        ParsedApi::AirCorpsSetPlane {
            request_body,
            api_data,
        } => {
            if air_corps::apply_set_plane(state, &request_body, &api_data) {
                air_corps::emit_air_base_update(state, app);
            }
        }
        ParsedApi::AirCorpsSetAction { request_body } => {
            if air_corps::apply_set_action(state, &request_body) {
                air_corps::emit_air_base_update(state, app);
            }
        }
        ParsedApi::AirCorpsSupply {
            request_body,
            api_data,
        } => {
            if air_corps::apply_supply(state, &request_body, &api_data) {
                air_corps::emit_air_base_update(state, app);
            }
        }
        ParsedApi::BaseAirCorps(api_data) => {
            let bases = air_corps::parse_base_air_corps(
                &api_data,
                &state.profile.slotitems,
                &state.master.slotitems,
                &state.air_bases,
            );
            if !bases.is_empty() {
                state.air_bases = bases;
                info!(
                    "base_air_corps: parsed {} air-base entries",
                    state.air_bases.len()
                );
                air_corps::emit_air_base_update(state, app);
            }
        }
        ParsedApi::AirCorpsChangeName { request_body } => {
            if air_corps::apply_change_name(state, &request_body) {
                air_corps::emit_air_base_update(state, app);
            }
        }
        ParsedApi::AirCorpsChangeDeployment {
            request_body,
            api_data,
        } => {
            if air_corps::apply_change_deployment(state, &request_body, &api_data) {
                air_corps::emit_air_base_update(state, app);
            }
        }
        ParsedApi::AirCorpsCondRecovery {
            request_body,
            api_data,
        } => {
            if air_corps::apply_supply(state, &request_body, &api_data) {
                air_corps::emit_air_base_update(state, app);
            }
        }
        ParsedApi::LogOnly => {}
        ParsedApi::Other => {}
    }
}
