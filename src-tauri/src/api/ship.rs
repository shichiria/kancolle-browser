use log::{error, info, warn};
use tauri::AppHandle;

use super::models;
use super::models::ShipInfo;
use super::fleet::emit_fleet_update;

// Ship type (stype) ID constants
pub(super) const STYPE_DE: i32 = 1;    // 海防艦
pub(super) const STYPE_DD: i32 = 2;    // 駆逐艦
pub(super) const STYPE_CL: i32 = 3;    // 軽巡洋艦
pub(super) const STYPE_CLT: i32 = 4;   // 重雷装巡洋艦
pub(super) const STYPE_CVL: i32 = 7;   // 軽空母
pub(super) const STYPE_BBV: i32 = 10;  // 航空戦艦
pub(super) const STYPE_CT: i32 = 21;   // 練習巡洋艦
pub(super) const STYPE_AO: i32 = 22;   // 補給艦

/// Extract stat value from api_karyoku / api_taiku / etc.
/// These are arrays where index 0 is the equipped total value.
pub(super) fn extract_stat_value(val: &serde_json::Value) -> i32 {
    if let Some(arr) = val.as_array() {
        arr.first().and_then(|v| v.as_i64()).unwrap_or(0) as i32
    } else {
        val.as_i64().unwrap_or(0) as i32
    }
}

/// Extract slot IDs from api_slot value (array of equipment instance IDs, -1 = empty)
pub(super) fn extract_slot_ids(val: &serde_json::Value) -> Vec<i32> {
    if let Some(arr) = val.as_array() {
        arr.iter()
            .map(|v| v.as_i64().unwrap_or(-1) as i32)
            .collect()
    } else {
        Vec::new()
    }
}

/// Build a ShipInfo from a PlayerShip and optional MasterShip data.
/// Used by process_port, process_ship3, and process_slot_deprive.
pub(super) fn build_ship_info(ship: &models::PlayerShip, master: Option<&models::MasterShipInfo>) -> ShipInfo {
    let name = master
        .map(|m| m.name.clone())
        .unwrap_or_else(|| format!("Unknown({})", ship.api_ship_id));
    let stype = master.map(|m| m.stype).unwrap_or(0);

    ShipInfo {
        ship_id: ship.api_ship_id,
        name,
        stype,
        lv: ship.api_lv,
        hp: ship.api_nowhp,
        maxhp: ship.api_maxhp,
        cond: ship.api_cond,
        fuel: ship.api_fuel,
        bull: ship.api_bull,
        firepower: extract_stat_value(&ship.api_karyoku),
        torpedo: extract_stat_value(&ship.api_raisou),
        aa: extract_stat_value(&ship.api_taiku),
        armor: extract_stat_value(&ship.api_soukou),
        asw: extract_stat_value(&ship.api_taisen),
        evasion: extract_stat_value(&ship.api_kaihi),
        los: extract_stat_value(&ship.api_sakuteki),
        luck: extract_stat_value(&ship.api_lucky),
        locked: ship.api_locked == 1,
        slot: extract_slot_ids(&ship.api_slot),
        slot_ex: ship.api_slot_ex,
        soku: ship.api_soku,
    }
}

/// Process api_get_member/ship3 - update ship slot data after equipment changes
pub(super) fn process_ship3(
    state: &mut models::GameStateInner,
    api_data: &serde_json::Value,
    app: &AppHandle,
) {
    // Update ships from api_ship_data
    if let Some(ships) = api_data.get("api_ship_data") {
        if let Ok(ship_list) = serde_json::from_value::<Vec<models::PlayerShip>>(ships.clone()) {
            for ship in &ship_list {
                let master = state.master.ships.get(&ship.api_ship_id);
                state.profile.ships.insert(
                    ship.api_id,
                    build_ship_info(ship, master),
                );
            }
            info!("ship3: updated {} ships", ship_list.len());
        }
    }

    // Update fleet compositions from api_deck_data
    if let Some(decks) = api_data.get("api_deck_data") {
        if let Ok(deck_list) = serde_json::from_value::<Vec<models::Fleet>>(decks.clone()) {
            for fleet in &deck_list {
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
        }
    }

    emit_fleet_update(state, app);
}

/// Process api_req_kaisou/slot_deprive - equipment transfer between ships
/// Response contains api_ship_data with api_set_ship and api_unset_ship
pub(super) fn process_slot_deprive(
    state: &mut models::GameStateInner,
    api_data: &serde_json::Value,
    app: &AppHandle,
) {
    let ship_data = match api_data.get("api_ship_data") {
        Some(sd) => sd,
        None => {
            warn!("slot_deprive: no api_ship_data found");
            return;
        }
    };

    let mut updated = 0;
    for key in &["api_set_ship", "api_unset_ship"] {
        if let Some(ship_val) = ship_data.get(*key) {
            match serde_json::from_value::<models::PlayerShip>(ship_val.clone()) {
                Ok(ship) => {
                    let master = state.master.ships.get(&ship.api_ship_id);
                    state.profile.ships.insert(
                        ship.api_id,
                        build_ship_info(&ship, master),
                    );
                    updated += 1;
                }
                Err(e) => {
                    error!("slot_deprive: failed to parse {}: {}", key, e);
                }
            }
        }
    }
    info!("slot_deprive: updated {} ships", updated);

    emit_fleet_update(state, app);
}

/// Check if the flagship has a command facility whose activation conditions are met,
/// and set `command_facility_name` on the flagship's ShipSummary if so.
///
/// Activation conditions:
/// - 艦隊司令部施設 (107): combined fleet, fleet 1 flagship
/// - 精鋭水雷戦隊 司令部 (413): not combined, flagship is CL(3)/DD(2), all escorts are DD(2)/CLT(4)
/// - 遊撃部隊 艦隊司令部 (272): fleet 3, 7 ships
pub(super) fn resolve_command_facility(
    ships: &mut [models::ShipSummary],
    fleet_id: i32,
    combined_flag: i32,
    profile: &models::UserProfile,
    master_slotitems: &std::collections::HashMap<i32, models::MasterSlotItemInfo>,
) {
    if ships.is_empty() {
        return;
    }
    let flagship_instance_id = ships[0].id;
    let Some(flagship_info) = profile.ships.get(&flagship_instance_id) else {
        return;
    };

    // Scan flagship equipment for command facility
    for &slot_id in flagship_info
        .slot
        .iter()
        .chain(std::iter::once(&flagship_info.slot_ex))
    {
        if slot_id <= 0 {
            continue;
        }
        let Some(player_item) = profile.slotitems.get(&slot_id) else {
            continue;
        };
        let sid = player_item.slotitem_id;
        let activated = match sid {
            // 艦隊司令部施設: 連合艦隊の第1艦隊旗艦
            107 => combined_flag > 0 && fleet_id == 1,
            // 精鋭水雷戦隊 司令部: 非連合、旗艦CL/DD、随伴全員DD/CLT
            413 => {
                if combined_flag > 0 {
                    false
                } else {
                    let fs_ok = flagship_info.stype == STYPE_DD || flagship_info.stype == STYPE_CL;
                    let escorts_ok = ships[1..].iter().all(|s| {
                        profile
                            .ships
                            .get(&s.id)
                            .map(|info| info.stype == STYPE_DD || info.stype == STYPE_CLT)
                            .unwrap_or(false)
                    });
                    fs_ok && escorts_ok
                }
            }
            // 遊撃部隊 艦隊司令部: 第3艦隊の7隻編成
            272 => fleet_id == 3 && ships.len() == 7,
            _ => continue,
        };
        if activated {
            if let Some(master) = master_slotitems.get(&sid) {
                ships[0].command_facility_name = Some(master.name.clone());
            }
            return;
        }
    }
}

/// Collected marks/indicators for a ship (damecon, special equips, opening ASW)
pub(super) struct ShipMarks {
    pub damecon_name: Option<String>,
    pub special_equips: Vec<models::SpecialEquip>,
    pub can_opening_asw: bool,
}

/// Collect all ship marks in a single equipment loop: damecon, special equips, and opening ASW
pub(super) fn collect_ship_marks(
    ship: &models::ShipInfo,
    player_slotitems: &std::collections::HashMap<i32, models::PlayerSlotItem>,
    master_slotitems: &std::collections::HashMap<i32, models::MasterSlotItemInfo>,
) -> ShipMarks {
    let mut damecon_name: Option<String> = None;
    let mut special_equips: Vec<models::SpecialEquip> = Vec::new();
    // Opening ASW detection data
    let mut has_sonar = false;
    let mut has_large_sonar = false;
    let mut has_asw7_aircraft = false; // 対潜7以上の艦攻/回転翼/哨戒機
    let mut has_asw1_bomber = false; // 対潜1以上の艦攻/艦爆
    let mut has_asw1_aircraft = false; // 対潜1以上の艦攻/艦爆/回転翼/哨戒機
    let mut equip_asw_total: i32 = 0;
    let mut rotorcraft_count = 0; // 回転翼機 (item_type=25)
    let mut s51j_count = 0; // S-51J系 (item_type=26, 対潜哨戒機)
    let mut has_seaplane_bomber = false; // 水爆 (item_type=11)
    let mut has_depth_charge_projector = false; // 爆雷投射機 (item_type=15)
    let mut has_depth_charge = false; // 爆雷 (item_type=17)

    for &slot_id in ship.slot.iter().chain(std::iter::once(&ship.slot_ex)) {
        if slot_id <= 0 {
            continue;
        }
        let Some(player) = player_slotitems.get(&slot_id) else {
            continue;
        };
        let Some(master) = master_slotitems.get(&player.slotitem_id) else {
            continue;
        };

        // Damecon (icon_type=14) — take first only
        if master.icon_type == 14 && damecon_name.is_none() {
            damecon_name = Some(master.name.clone());
        }

        // Special equips: landing craft (20), drum canister (25)
        if master.icon_type == 20 || master.icon_type == 25 {
            special_equips.push(models::SpecialEquip {
                name: master.name.clone(),
                icon_type: master.icon_type,
            });
        }

        // Sonar detection
        if master.icon_type == 17 {
            has_sonar = true; // small sonar
        }
        if master.icon_type == 18 {
            has_sonar = true; // large sonar counts as sonar too
            has_large_sonar = true;
        }

        // Equipment ASW total
        equip_asw_total += master.asw;

        // Aircraft type checks
        let it = master.item_type;
        // 艦攻(8), 回転翼(25), 対潜哨戒機(26): ASW>=7 check
        if (it == 8 || it == 25 || it == 26) && master.asw >= 7 {
            has_asw7_aircraft = true;
        }
        // 艦攻(8), 艦爆(7): ASW>=1 check
        if (it == 8 || it == 7) && master.asw >= 1 {
            has_asw1_bomber = true;
        }
        // 艦攻(8), 艦爆(7), 回転翼(25), 対潜哨戒機(26): ASW>=1 check
        if (it == 8 || it == 7 || it == 25 || it == 26) && master.asw >= 1 {
            has_asw1_aircraft = true;
        }
        // Rotorcraft count (item_type=25)
        if it == 25 {
            rotorcraft_count += 1;
        }
        // Patrol aircraft count (item_type=26) for S-51J series
        if it == 26 {
            s51j_count += 1;
        }
        // Seaplane bomber (item_type=11)
        if it == 11 {
            has_seaplane_bomber = true;
        }
        // Depth charge projector (item_type=15)
        if it == 15 {
            has_depth_charge_projector = true;
        }
        // Depth charge (item_type=17)
        if it == 17 {
            has_depth_charge = true;
        }
    }

    if !special_equips.is_empty() {
        info!(
            "Ship {} has {} special equips: {:?}",
            ship.name,
            special_equips.len(),
            special_equips
                .iter()
                .map(|e| format!("{}(icon={})", e.name, e.icon_type))
                .collect::<Vec<_>>()
        );
    }

    let can_opening_asw = check_opening_asw(
        ship,
        has_sonar,
        has_large_sonar,
        has_asw7_aircraft,
        has_asw1_bomber,
        has_asw1_aircraft,
        equip_asw_total,
        rotorcraft_count,
        s51j_count,
        has_seaplane_bomber,
        has_depth_charge_projector,
        has_depth_charge,
    );

    ShipMarks {
        damecon_name,
        special_equips,
        can_opening_asw,
    }
}

/// Determine if a ship can perform opening ASW
fn check_opening_asw(
    ship: &models::ShipInfo,
    has_sonar: bool,
    _has_large_sonar: bool,
    has_asw7_aircraft: bool,
    has_asw1_bomber: bool,
    has_asw1_aircraft: bool,
    equip_asw_total: i32,
    rotorcraft_count: i32,
    s51j_count: i32,
    has_seaplane_bomber: bool,
    has_depth_charge_projector: bool,
    has_depth_charge: bool,
) -> bool {
    let asw = ship.asw; // equipped total ASW
    let stype = ship.stype;
    let sid = ship.ship_id;

    // 1. Unconditional ships (always OASW regardless of equipment)
    const UNCONDITIONAL: &[i32] = &[
        141,  // 五十鈴改二
        478,  // 龍田改二
        624,  // 夕張改二丁
        394,  // Jervis改
        893,  // Jervis Mk.II
        681,  // Janus改
        875,  // Janus Mk.II
        562,  // Fletcher
        596,  // Fletcher改 Mod.2
        628,  // Fletcher Mk.II
        629,  // Fletcher Mk.II (extra)
        563,  // Johnston
        597,  // Johnston改
        692,  // Johnston Mk.II
        700,  // Samuel B.Roberts Mk.II
        911,  // Heywood L.Edwards改
        916,  // Richard P.Leary改
    ];
    if UNCONDITIONAL.contains(&sid) {
        return true;
    }

    // 2. Escort carriers (大鷹型改/改二, 加賀改二護, Gambier Bay Mk.II)
    const ESCORT_CVE: &[i32] = &[
        529, // 大鷹改
        536, // 大鷹改二
        380, // 神鷹改
        521, // 神鷹改二
        381, // 雲鷹改
        539, // 雲鷹改二
        546, // 加賀改二護
        396, // 春日丸
        557, // Gambier Bay Mk.II
    ];
    if ESCORT_CVE.contains(&sid) {
        return has_asw1_aircraft;
    }

    // 3. 海防艦 (stype=1): ASW>=60+sonar OR ASW>=75+equip_asw>=4
    if stype == STYPE_DE {
        return (asw >= 60 && has_sonar) || (asw >= 75 && equip_asw_total >= 4);
    }

    // 4. 護衛空母/軽空母 (stype=7)
    //    鈴谷航改二(503), 熊野航改二(504)は除外
    if stype == STYPE_CVL && sid != 503 && sid != 504 {
        // Pattern A: ASW>=50 + sonar + ASW>=7 aircraft
        if asw >= 50 && has_sonar && has_asw7_aircraft {
            return true;
        }
        // Pattern B: ASW>=65 + ASW>=7 aircraft
        if asw >= 65 && has_asw7_aircraft {
            return true;
        }
        // Pattern C: ASW>=100 + sonar + ASW>=1 bomber (艦攻/艦爆)
        if asw >= 100 && has_sonar && has_asw1_bomber {
            return true;
        }
        return false;
    }

    // 5. 日向改二 (ship_id=554): S-51J系1+ OR 回転翼(Ka号/O号)2+
    if sid == 554 {
        return s51j_count >= 1 || rotorcraft_count >= 2;
    }

    // 6. 航空戦艦 (stype=10): ASW>=100 + sonar + (水爆/回転翼/哨戒機/爆雷投射機/爆雷)
    if stype == STYPE_BBV {
        let has_asw_equip = has_seaplane_bomber
            || rotorcraft_count > 0
            || s51j_count > 0
            || has_depth_charge_projector
            || has_depth_charge;
        return asw >= 100 && has_sonar && has_asw_equip;
    }

    // 7. General ships: DD(2), CL(3), CLT(4), CT(21), AO(22): ASW>=100 + sonar
    if stype == STYPE_DD || stype == STYPE_CL || stype == STYPE_CLT || stype == STYPE_CT || stype == STYPE_AO {
        return asw >= 100 && has_sonar;
    }

    false
}
