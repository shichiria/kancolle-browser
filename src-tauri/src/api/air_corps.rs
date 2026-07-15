use log::{info, warn};
use std::collections::HashMap;
use tauri::{AppHandle, Emitter};

use super::dto::air_corps as dto;
use super::models;

/// Parse the `api_air_base` array from `api_get_member/mapinfo` or
/// `api_get_member/base_air_corps`. Both endpoints share the same payload shape.
/// Returns owned AirBase entries with planes enriched (name/level/alv/icon_type).
///
/// `existing_bases` lets us preserve `recent_attacks` across a full refresh —
/// the user expects last-sortie results to remain visible after returning to
/// port (which fires `api_get_member/mapinfo` on the next 出撃 screen).
pub(super) fn parse_air_bases(
    api_data: &dto::MapInfoAirBases,
    profile_slotitems: &HashMap<i32, models::PlayerSlotItem>,
    master_slotitems: &HashMap<i32, models::MasterSlotItemInfo>,
    existing_bases: &[models::AirBase],
) -> Vec<models::AirBase> {
    parse_air_base_array(
        &api_data.api_air_base,
        profile_slotitems,
        master_slotitems,
        existing_bases,
    )
}

/// Parse `api_get_member/base_air_corps` whose `api_data` is the array directly.
pub(super) fn parse_base_air_corps(
    api_data: &[dto::AirBase],
    profile_slotitems: &HashMap<i32, models::PlayerSlotItem>,
    master_slotitems: &HashMap<i32, models::MasterSlotItemInfo>,
    existing_bases: &[models::AirBase],
) -> Vec<models::AirBase> {
    parse_air_base_array(
        api_data,
        profile_slotitems,
        master_slotitems,
        existing_bases,
    )
}

fn parse_air_base_array(
    arr: &[dto::AirBase],
    profile_slotitems: &HashMap<i32, models::PlayerSlotItem>,
    master_slotitems: &HashMap<i32, models::MasterSlotItemInfo>,
    existing_bases: &[models::AirBase],
) -> Vec<models::AirBase> {
    arr.iter()
        .map(|entry| {
            let rid = entry.api_rid;
            let area_id = entry.api_area_id;
            let name = entry.api_name.clone();
            let action_kind = entry.api_action_kind;
            let distance = parse_distance(&entry.api_distance);
            let planes = entry
                .api_plane_info
                .iter()
                .map(|plane| parse_plane(plane, profile_slotitems, master_slotitems))
                .collect();
            let recent_attacks = existing_bases
                .iter()
                .find(|b| b.area_id == area_id && b.rid == rid)
                .map(|b| b.recent_attacks.clone())
                .unwrap_or_default();

            models::AirBase {
                rid,
                area_id,
                name,
                action_kind,
                distance,
                planes,
                recent_attacks,
            }
        })
        .collect()
}

fn parse_distance(value: &dto::AirBaseDistance) -> models::AirBaseDistance {
    models::AirBaseDistance {
        base: value.api_base,
        bonus: value.api_bonus,
    }
}

fn parse_plane(
    plane: &dto::AirBasePlane,
    profile_slotitems: &HashMap<i32, models::PlayerSlotItem>,
    master_slotitems: &HashMap<i32, models::MasterSlotItemInfo>,
) -> models::AirBasePlane {
    let mut parsed = models::AirBasePlane {
        squadron_id: plane.api_squadron_id,
        slotid: plane.api_slotid,
        state: plane.api_state,
        count: plane.api_count,
        max_count: plane.api_max_count,
        cond: plane.api_cond,
        ..Default::default()
    };

    enrich_plane(&mut parsed, profile_slotitems, master_slotitems);
    parsed
}

/// Resolve master + player slotitem data and populate name/level/alv/icon_type.
fn enrich_plane(
    plane: &mut models::AirBasePlane,
    profile_slotitems: &HashMap<i32, models::PlayerSlotItem>,
    master_slotitems: &HashMap<i32, models::MasterSlotItemInfo>,
) {
    if plane.slotid <= 0 {
        return;
    }
    let player = match profile_slotitems.get(&plane.slotid) {
        Some(p) => p,
        None => return,
    };
    plane.slotitem_id = Some(player.slotitem_id);
    plane.level = Some(player.level);
    plane.alv = player.alv;
    if let Some(master) = master_slotitems.get(&player.slotitem_id) {
        plane.name = Some(master.name.clone());
        plane.icon_type = Some(master.icon_type);
    }
}

/// Apply the response of `api_req_air_corps/set_plane` (single squadron update).
/// Returns true if the in-memory state was modified.
pub(super) fn apply_set_plane(
    state: &mut models::GameStateInner,
    request_body: &str,
    api_data: &dto::PlaneUpdate,
) -> bool {
    let params =
        serde_urlencoded::from_str::<dto::AirBaseRequest>(request_body).unwrap_or_default();
    let area_id = params.api_area_id;
    let base_id = params.api_base_id;
    if area_id == 0 || base_id == 0 {
        warn!(
            "air_corps/set_plane: missing area_id/base_id in request: {}",
            request_body
        );
        return false;
    }

    let distance = parse_distance(&api_data.api_distance);

    // Snapshot data needed for enrichment, then take a mutable reference to the base.
    let profile_clone = state.profile.slotitems.clone();
    let master_clone = state.master.slotitems.clone();

    let base = match find_or_insert_base(&mut state.air_bases, area_id, base_id) {
        Some(b) => b,
        None => return false,
    };

    base.distance = distance;
    for entry in &api_data.api_plane_info {
        let updated = parse_plane(entry, &profile_clone, &master_clone);
        upsert_squadron(&mut base.planes, updated);
    }
    true
}

/// Apply `api_req_air_corps/set_action` — base action change. The response has
/// no body data, so parse the request: `api_base_id=1,2&api_action_kind=1,1`.
pub(super) fn apply_set_action(state: &mut models::GameStateInner, request_body: &str) -> bool {
    let params =
        serde_urlencoded::from_str::<dto::SetActionRequest>(request_body).unwrap_or_default();
    let area_id = params.api_area_id;
    if area_id == 0 {
        return false;
    }
    let base_ids: Vec<i32> = params
        .api_base_id
        .split(',')
        .filter_map(|token| token.trim().parse::<i32>().ok())
        .collect();
    let actions: Vec<i32> = params
        .api_action_kind
        .split(',')
        .filter_map(|token| token.trim().parse::<i32>().ok())
        .collect();
    if base_ids.is_empty() || base_ids.len() != actions.len() {
        warn!(
            "air_corps/set_action: malformed pairs (bases={:?} actions={:?})",
            base_ids, actions
        );
        return false;
    }

    let mut changed = false;
    for (rid, kind) in base_ids.iter().zip(actions.iter()) {
        if let Some(base) = find_or_insert_base(&mut state.air_bases, area_id, *rid) {
            base.action_kind = *kind;
            changed = true;
        }
    }
    changed
}

/// Apply `api_req_air_corps/supply` — refill planes for one or more squadrons.
/// Response includes `api_plane_info[]` with updated counts.
pub(super) fn apply_supply(
    state: &mut models::GameStateInner,
    request_body: &str,
    api_data: &dto::PlaneUpdate,
) -> bool {
    apply_single_base_plane_update(state, request_body, api_data)
}

/// Apply `api_req_air_corps/change_deployment_base` — 配備換え.
/// Response shape: `api_base_items[]` where each item has `api_rid` + `api_distance`
/// + `api_plane_info[]`. The area_id is only in the request body.
pub(super) fn apply_change_deployment(
    state: &mut models::GameStateInner,
    request_body: &str,
    api_data: &dto::DeploymentUpdate,
) -> bool {
    let params =
        serde_urlencoded::from_str::<dto::AirBaseRequest>(request_body).unwrap_or_default();
    let area_id = params.api_area_id;
    if area_id == 0 {
        return false;
    }
    let profile_clone = state.profile.slotitems.clone();
    let master_clone = state.master.slotitems.clone();
    let mut changed = false;
    for item in &api_data.api_base_items {
        let rid = item.api_rid;
        if rid == 0 {
            continue;
        }
        let distance = parse_distance(&item.api_distance);
        if let Some(base) = find_or_insert_base(&mut state.air_bases, area_id, rid) {
            base.distance = distance;
            for entry in &item.api_plane_info {
                let updated = parse_plane(entry, &profile_clone, &master_clone);
                upsert_squadron(&mut base.planes, updated);
            }
            changed = true;
        }
    }
    changed
}

fn apply_single_base_plane_update(
    state: &mut models::GameStateInner,
    request_body: &str,
    api_data: &dto::PlaneUpdate,
) -> bool {
    let params =
        serde_urlencoded::from_str::<dto::AirBaseRequest>(request_body).unwrap_or_default();
    let area_id = params.api_area_id;
    let base_id = params.api_base_id;
    if area_id == 0 || base_id == 0 {
        return false;
    }
    let distance = parse_distance(&api_data.api_distance);

    let profile_clone = state.profile.slotitems.clone();
    let master_clone = state.master.slotitems.clone();
    let base = match find_or_insert_base(&mut state.air_bases, area_id, base_id) {
        Some(b) => b,
        None => return false,
    };
    base.distance = distance;
    for entry in &api_data.api_plane_info {
        let updated = parse_plane(entry, &profile_clone, &master_clone);
        upsert_squadron(&mut base.planes, updated);
    }
    true
}

/// Apply `api_req_air_corps/change_name` — base rename. Request body has the new name.
pub(super) fn apply_change_name(state: &mut models::GameStateInner, request_body: &str) -> bool {
    let params =
        serde_urlencoded::from_str::<dto::ChangeNameRequest>(request_body).unwrap_or_default();
    let area_id = params.api_area_id;
    let base_id = params.api_base_id;
    let new_name = params.api_name;
    if new_name.is_empty() {
        return false;
    }
    if area_id == 0 || base_id == 0 {
        return false;
    }
    let base = match find_or_insert_base(&mut state.air_bases, area_id, base_id) {
        Some(b) => b,
        None => return false,
    };
    base.name = new_name;
    true
}

fn find_or_insert_base(
    bases: &mut Vec<models::AirBase>,
    area_id: i32,
    rid: i32,
) -> Option<&mut models::AirBase> {
    if let Some(idx) = bases
        .iter()
        .position(|b| b.area_id == area_id && b.rid == rid)
    {
        return bases.get_mut(idx);
    }
    // No prior mapinfo for this base — create a stub so the update isn't lost.
    bases.push(models::AirBase {
        rid,
        area_id,
        name: format!("第{}基地航空隊", rid),
        action_kind: 0,
        distance: models::AirBaseDistance::default(),
        planes: (1..=4)
            .map(|sid| models::AirBasePlane {
                squadron_id: sid,
                ..Default::default()
            })
            .collect(),
        recent_attacks: Vec::new(),
    });
    bases.last_mut()
}

fn upsert_squadron(planes: &mut Vec<models::AirBasePlane>, updated: models::AirBasePlane) {
    if let Some(existing) = planes
        .iter_mut()
        .find(|p| p.squadron_id == updated.squadron_id)
    {
        *existing = updated;
    } else {
        planes.push(updated);
    }
}

/// Process `api_air_base_attack[]` from a battle response.
///
/// For each wave entry, compute the total losses (stage1 + stage2), distribute
/// them across the 4 squadrons proportionally to current `count` (matches the
/// expected value of the kc-web simulator's per-squadron stage1 formula), and
/// append a record to that base's `recent_attacks`.
///
/// `current_area_id` is the area the player is sortieing in — the API entry only
/// has `api_base_id` so we resolve `(area_id, rid)` ourselves.
///
/// Returns true if any state was modified.
pub fn apply_battle_attack(
    state: &mut models::GameStateInner,
    current_area_id: i32,
    api_data: &dto::BattleAirBaseAttacks,
) -> bool {
    if current_area_id == 0 {
        return false;
    }
    if api_data.api_air_base_attack.is_empty() {
        return false;
    }

    let mut wave_index_for_base: HashMap<i32, i32> = HashMap::new();
    let mut changed = false;

    for entry in &api_data.api_air_base_attack {
        let base_id = entry.api_base_id;
        if base_id == 0 {
            continue;
        }
        let disp_seiku = entry
            .api_stage1
            .as_ref()
            .map_or(0, |stage| stage.api_disp_seiku);
        let f_count = entry
            .api_stage1
            .as_ref()
            .map_or(0, |stage| stage.api_f_count);
        let stage1_lost = entry
            .api_stage1
            .as_ref()
            .map_or(0, |stage| stage.api_f_lostcount);
        let stage2_lost = entry
            .api_stage2
            .as_ref()
            .map_or(0, |stage| stage.api_f_lostcount);

        let edam_total =
            sum_edam(entry.api_stage3.as_ref()) + sum_edam(entry.api_stage3_combined.as_ref());
        let total_lost = stage1_lost + stage2_lost;

        let wave = {
            let entry = wave_index_for_base.entry(base_id).or_insert(0);
            *entry += 1;
            *entry
        };

        let base = match find_or_insert_base(&mut state.air_bases, current_area_id, base_id) {
            Some(b) => b,
            None => continue,
        };

        let per_squadron_lost = distribute_losses(&mut base.planes, total_lost);
        base.recent_attacks.push(models::AirBaseAttackWave {
            wave,
            disp_seiku,
            f_count,
            stage1_lost,
            stage2_lost,
            edam_total,
            per_squadron_lost,
        });
        changed = true;
    }

    changed
}

/// Distribute `total_lost` planes across squadrons proportionally to current
/// `count`. Mutates each squadron's `count` in place. Returns the per-squadron
/// loss in squadron_id order (1..=4).
fn distribute_losses(planes: &mut [models::AirBasePlane], total_lost: i32) -> Vec<i32> {
    let mut result = vec![0; 4];
    if total_lost <= 0 {
        return result;
    }
    let total_slots: i32 = planes
        .iter()
        .filter(|p| p.state != 0)
        .map(|p| p.count.max(0))
        .sum();
    if total_slots <= 0 {
        return result;
    }

    // First pass: floor of proportional share. Second pass: distribute remainder
    // largest-fractional-first so the sum equals total_lost exactly.
    let mut allocations: Vec<(usize, i32, f64)> = planes
        .iter()
        .enumerate()
        .filter(|(_, p)| p.state != 0 && p.count > 0)
        .map(|(idx, p)| {
            let exact = total_lost as f64 * p.count as f64 / total_slots as f64;
            let floor = exact.floor() as i32;
            let frac = exact - floor as f64;
            (idx, floor, frac)
        })
        .collect();

    let allocated: i32 = allocations.iter().map(|(_, f, _)| *f).sum();
    let mut remainder = total_lost - allocated;
    if remainder > 0 {
        // Sort by descending fractional part — assign one extra to top remainders.
        let mut order: Vec<usize> = (0..allocations.len()).collect();
        order.sort_by(|a, b| {
            allocations[*b]
                .2
                .partial_cmp(&allocations[*a].2)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        for &i in order.iter() {
            if remainder == 0 {
                break;
            }
            allocations[i].1 += 1;
            remainder -= 1;
        }
    }

    for (plane_idx, lost, _) in allocations {
        let plane = &mut planes[plane_idx];
        let lost = lost.min(plane.count);
        plane.count -= lost;
        if let Some(slot) = result.get_mut((plane.squadron_id as usize).saturating_sub(1)) {
            *slot = lost;
        }
    }
    result
}

fn sum_edam(stage3: Option<&dto::AirAttackDamage>) -> i32 {
    stage3
        .into_iter()
        .flat_map(|stage| stage.api_edam.iter().flatten())
        .copied()
        .map(|f| f.max(0.0) as i32)
        .sum()
}

/// Clear `recent_attacks` on every base. Called when a new sortie starts.
pub fn clear_recent_attacks(state: &mut models::GameStateInner) -> bool {
    let mut cleared = false;
    for base in state.air_bases.iter_mut() {
        if !base.recent_attacks.is_empty() {
            base.recent_attacks.clear();
            cleared = true;
        }
    }
    cleared
}

/// Emit `air-base-updated` to the frontend with the full current state.
pub fn emit_air_base_update(state: &models::GameStateInner, app: &AppHandle) {
    crate::action_log::log(
        "Event",
        "air-base-updated",
        &format!("{} bases", state.air_bases.len()),
    );
    if let Err(e) = app.emit(crate::events::AIR_BASE_UPDATED, &state.air_bases) {
        warn!("Failed to emit air-base-updated: {}", e);
    } else {
        info!("air-base-updated emitted: {} bases", state.air_bases.len());
    }
}
