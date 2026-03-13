use log::info;
use std::collections::HashMap;

use crate::api::models::MasterShipInfo;

use super::{
    AirBattleResult, BattleDetail, BattleLogger, EnemyShip, HpState, PendingBattle,
};

impl BattleLogger {
    pub fn on_battle(
        &mut self,
        data: &crate::api::dto::battle::ApiBattleResponse,
        raw_json: &serde_json::Value,
    ) {
        if self.active_sortie.is_none() {
            return;
        }

        let mut pending = PendingBattle::default();

        // --- Formation ---
        if let Some(arr) = &data.api_formation {
            let vals = arr;
            pending.formation = [
                vals.first().copied().unwrap_or(0),
                vals.get(1).copied().unwrap_or(0),
                vals.get(2).copied().unwrap_or(0),
            ];
        }

        // --- Friendly HP before ---
        if let (Some(now_arr), Some(max_arr)) = (&data.api_f_nowhps, &data.api_f_maxhps) {
            pending.friendly_hp_before = now_arr
                .iter()
                .zip(max_arr.iter())
                .map(|(&n, &m)| (n, m))
                .collect();
        }

        // --- Enemy HP before ---
        if let (Some(now_arr), Some(max_arr)) = (&data.api_e_nowhps, &data.api_e_maxhps) {
            pending.enemy_hp_before = now_arr
                .iter()
                .zip(max_arr.iter())
                .map(|(&n, &m)| (n, m))
                .collect();
        }

        // --- Enemy ship IDs and levels ---
        if let Some(arr) = &data.api_ship_ke {
            pending.enemy_ship_ids = arr.iter().copied().filter(|&id| id > 0).collect();
        }
        if let Some(arr) = &data.api_ship_lv {
            pending.enemy_ship_levels = arr.iter().copied().collect();
        }

        // --- Enemy equipment (api_eSlot) ---
        if let Some(arr) = &data.api_e_slot {
            pending.enemy_ship_slots = arr
                .iter()
                .map(|ship_slots: &Vec<i32>| ship_slots.iter().copied().filter(|&id| id > 0).collect())
                .collect();
        }

        let api_data_raw = raw_json.get("api_data").unwrap_or(raw_json);

        // --- Air battle (api_kouku) ---
        pending.air_battle = extract_air_battle(api_data_raw);

        // --- Midnight flag ---
        pending.midnight_flag = data.api_midnight_flag.unwrap_or(0) == 1;

        // --- Calculate HP after battle (friendly) ---
        let f_hp_now: Vec<i32> = pending.friendly_hp_before.iter().map(|&(n, _)| n).collect();
        let f_hp_max: Vec<i32> = pending.friendly_hp_before.iter().map(|&(_, m)| m).collect();
        let f_hp_after = calculate_hp_after_from_start(&f_hp_now, api_data_raw);
        pending.friendly_hp_after = f_hp_after
            .iter()
            .zip(f_hp_max.iter())
            .map(|(&n, &m)| (n.max(0), m))
            .collect();

        // --- Calculate HP after battle (enemy) ---
        let e_hp_now: Vec<i32> = pending.enemy_hp_before.iter().map(|&(n, _)| n).collect();
        let e_hp_max: Vec<i32> = pending.enemy_hp_before.iter().map(|&(_, m)| m).collect();
        let e_hp_after = calculate_enemy_hp_after(&e_hp_now, api_data_raw);
        pending.enemy_hp_after = e_hp_after
            .iter()
            .zip(e_hp_max.iter())
            .map(|(&n, &m)| (n.max(0), m))
            .collect();

        // --- Store raw battle JSON ---
        pending.raw_battle_json = Some(api_data_raw.clone());

        self.pending_battle = Some(pending);
    }

    /// Handle midnight battle (continuation, adds to same node)
    pub fn on_midnight_battle(
        &mut self,
        data: &crate::api::dto::battle::ApiBattleResponse,
        raw_json: &serde_json::Value,
    ) {
        if self.active_sortie.is_none() {
            return;
        }

        let api_data_raw = raw_json.get("api_data").unwrap_or(raw_json);

        // If no pending battle exists (e.g. sp_midnight which starts at night),
        // create a new one from the midnight data
        if self.pending_battle.is_none() {
            let mut pending = PendingBattle::default();

            // Formation
            if let Some(arr) = &data.api_formation {
                let vals = arr;
                pending.formation = [
                    vals.first().copied().unwrap_or(0),
                    vals.get(1).copied().unwrap_or(0),
                    vals.get(2).copied().unwrap_or(0),
                ];
            }

            // Friendly HP before
            if let (Some(now_arr), Some(max_arr)) = (&data.api_f_nowhps, &data.api_f_maxhps) {
                pending.friendly_hp_before = now_arr
                    .iter()
                    .zip(max_arr.iter())
                    .map(|(&n, &m)| (n, m))
                    .collect();
            }

            // Enemy HP before
            if let (Some(now_arr), Some(max_arr)) = (&data.api_e_nowhps, &data.api_e_maxhps) {
                pending.enemy_hp_before = now_arr
                    .iter()
                    .zip(max_arr.iter())
                    .map(|(&n, &m)| (n, m))
                    .collect();
            }

            // Enemy ships
            if let Some(arr) = &data.api_ship_ke {
                pending.enemy_ship_ids = arr.iter().copied().filter(|&id| id > 0).collect();
            }
            if let Some(arr) = &data.api_ship_lv {
                pending.enemy_ship_levels = arr.iter().copied().collect();
            }

            pending.raw_battle_json = Some(api_data_raw.clone());
            self.pending_battle = Some(pending);
        }

        let pending = self.pending_battle.as_mut().unwrap();
        pending.had_night_battle = true;

        // Update friendly HP after with midnight battle results
        // Midnight battle starts from the HP at end of day battle (api_f_nowhps in midnight response)
        if let (Some(now_arr), Some(max_arr)) = (&data.api_f_nowhps, &data.api_f_maxhps) {
            let mut hp: Vec<i32> = now_arr.clone();
            let max_hp: Vec<i32> = max_arr.clone();

            // Apply midnight hougeki
            apply_hougeki_damage(&mut hp, api_data_raw, "api_hougeki");

            pending.friendly_hp_after = hp
                .iter()
                .zip(max_hp.iter())
                .map(|(&n, &m)| (n.max(0), m))
                .collect();
        }

        // Update enemy HP after with midnight battle results
        if let (Some(now_arr), Some(max_arr)) = (&data.api_e_nowhps, &data.api_e_maxhps) {
            let mut hp: Vec<i32> = now_arr.clone();
            let max_hp: Vec<i32> = max_arr.clone();

            // Apply midnight hougeki to enemy
            apply_hougeki_damage_enemy(&mut hp, api_data_raw, "api_hougeki");

            pending.enemy_hp_after = hp
                .iter()
                .zip(max_hp.iter())
                .map(|(&n, &m)| (n.max(0), m))
                .collect();
        }

        // Store raw midnight JSON
        pending.raw_midnight_json = Some(api_data_raw.clone());
    }

    /// Handle battle result (api_req_sortie/battleresult, api_req_combined_battle/battleresult)
    pub fn on_battle_result(
        &mut self,
        data: &crate::api::dto::battle::ApiBattleResultResponse,
        json: &serde_json::Value,
        master_ships: &HashMap<i32, MasterShipInfo>,
    ) {
        let sortie = match &mut self.active_sortie {
            Some(s) => s,
            None => return,
        };

        let pending = self.pending_battle.take().unwrap_or_default();

        // --- Basic result fields ---
        let rank = data.api_win_rank.clone().unwrap_or_else(|| "-".to_string());

        let enemy_name = data
            .api_enemy_info
            .as_ref()
            .and_then(|e| e.api_deck_name.clone())
            .unwrap_or_default();

        let mvp = data.api_mvp;
        let base_exp = data.api_get_base_exp;

        // --- Per-ship exp ---
        let ship_exp = data.api_get_ship_exp.clone().unwrap_or_default();

        // --- Ship drop ---
        let has_ship_drop = data
            .api_get_flag
            .as_ref()
            .and_then(|a| a.get(1))
            .copied()
            .unwrap_or(0)
            == 1;

        let (drop_ship, drop_ship_id) = if has_ship_drop {
            let ship_id = data.api_get_ship.as_ref().and_then(|s| s.api_ship_id);
            let ship_name = data
                .api_get_ship
                .as_ref()
                .and_then(|s| s.api_ship_name.clone())
                .or_else(|| ship_id.and_then(|id| master_ships.get(&id).map(|m| m.name.clone())));
            (ship_name, ship_id)
        } else {
            (None, None)
        };

        // --- Build enemy ships list ---
        let enemy_ships: Vec<EnemyShip> = {
            let ids = &pending.enemy_ship_ids;
            let levels = &pending.enemy_ship_levels;
            // enemy_ship_levels often has a leading dummy element that aligns
            // with ship_ke's -1 padding. Offset accordingly.
            let level_offset = if levels.len() > ids.len() {
                levels.len() - ids.len()
            } else {
                0
            };
            let eslots = &pending.enemy_ship_slots;
            // api_eSlot may include slots for padding ships; offset like levels
            let slot_offset = if eslots.len() > ids.len() {
                eslots.len() - ids.len()
            } else {
                0
            };
            ids.iter()
                .enumerate()
                .map(|(i, &ship_id)| {
                    let level = levels.get(i + level_offset).copied().unwrap_or(0);
                    let name = master_ships.get(&ship_id).map(|m| m.name.clone());
                    let slots = eslots.get(i + slot_offset).cloned().unwrap_or_default();
                    EnemyShip {
                        ship_id,
                        level,
                        name,
                        slots,
                    }
                })
                .collect()
        };

        // --- Build HP states ---
        let friendly_hp: Vec<HpState> = pending
            .friendly_hp_before
            .iter()
            .zip(pending.friendly_hp_after.iter())
            .map(|(&(bef, max), &(aft, _))| HpState {
                before: bef,
                after: aft,
                max,
            })
            .collect();

        let enemy_hp: Vec<HpState> = pending
            .enemy_hp_before
            .iter()
            .zip(pending.enemy_hp_after.iter())
            .map(|(&(bef, max), &(aft, _))| HpState {
                before: bef,
                after: aft,
                max,
            })
            .collect();

        // --- Combine raw JSONs ---
        let raw_battle = match (&pending.raw_battle_json, &pending.raw_midnight_json) {
            (Some(day), Some(night)) => Some(serde_json::json!({
                "day": day,
                "night": night,
            })),
            (Some(day), None) => Some(day.clone()),
            (None, Some(night)) => Some(night.clone()),
            (None, None) => None,
        };

        let night_battle = pending.had_night_battle || pending.midnight_flag;

        // --- Assemble BattleDetail ---
        let detail = BattleDetail {
            rank: rank.clone(),
            enemy_name: enemy_name.clone(),
            enemy_ships,
            formation: pending.formation,
            air_battle: pending.air_battle,
            friendly_hp,
            enemy_hp,
            drop_ship: drop_ship.clone(),
            drop_ship_id,
            mvp,
            base_exp,
            ship_exp,
            night_battle,
            raw_battle,
            raw_result: json.get("api_data").cloned(),
        };

        // Update the last node with battle detail
        if let Some(node) = sortie.nodes.last_mut() {
            node.battle = Some(detail);
        }

        info!(
            "Battle result: rank={} enemy={} drop={:?}",
            rank, enemy_name, drop_ship,
        );

        // Save partial sortie to disk after each battle (crash recovery)
        if let Some(ref sortie) = self.active_sortie {
            self.save_to_disk(sortie);
        }
    }
}

// =============================================================================
// HP calculation helper functions
// =============================================================================

/// Extract air battle results from api_kouku
fn extract_air_battle(api_data: &serde_json::Value) -> Option<AirBattleResult> {
    let kouku = api_data.get("api_kouku")?;
    if kouku.is_null() {
        return None;
    }

    let stage1 = kouku.get("api_stage1");

    let air_superiority = stage1
        .and_then(|s| s.get("api_disp_seiku"))
        .and_then(|v| v.as_i64())
        .map(|n| n as i32);

    let friendly_plane_count = stage1.and_then(|s| {
        let count = s.get("api_f_count")?.as_i64()? as i32;
        let lost = s.get("api_f_lostcount")?.as_i64()? as i32;
        Some([count, lost])
    });

    let enemy_plane_count = stage1.and_then(|s| {
        let count = s.get("api_e_count")?.as_i64()? as i32;
        let lost = s.get("api_e_lostcount")?.as_i64()? as i32;
        Some([count, lost])
    });

    Some(AirBattleResult {
        air_superiority,
        friendly_plane_count,
        enemy_plane_count,
    })
}

/// Calculate friendly HP after all day battle phases from starting HP
fn calculate_hp_after_from_start(start_hp: &[i32], api_data: &serde_json::Value) -> Vec<i32> {
    let mut hp = start_hp.to_vec();

    // Apply damage from various phases in order
    apply_kouku_damage(&mut hp, api_data, "api_kouku");
    apply_raigeki_damage(&mut hp, api_data, "api_opening_atack");
    apply_hougeki_damage(&mut hp, api_data, "api_opening_taisen");
    apply_hougeki_damage(&mut hp, api_data, "api_hougeki1");
    apply_hougeki_damage(&mut hp, api_data, "api_hougeki2");
    apply_hougeki_damage(&mut hp, api_data, "api_hougeki3");
    apply_raigeki_damage(&mut hp, api_data, "api_raigeki");

    hp
}

/// Calculate enemy HP after all day battle phases
fn calculate_enemy_hp_after(start_hp: &[i32], api_data: &serde_json::Value) -> Vec<i32> {
    let mut hp = start_hp.to_vec();

    // Apply damage to enemy from various phases
    apply_kouku_damage_enemy(&mut hp, api_data, "api_kouku");
    apply_raigeki_damage_enemy(&mut hp, api_data, "api_opening_atack");
    apply_hougeki_damage_enemy(&mut hp, api_data, "api_opening_taisen");
    apply_hougeki_damage_enemy(&mut hp, api_data, "api_hougeki1");
    apply_hougeki_damage_enemy(&mut hp, api_data, "api_hougeki2");
    apply_hougeki_damage_enemy(&mut hp, api_data, "api_hougeki3");
    apply_raigeki_damage_enemy(&mut hp, api_data, "api_raigeki");

    hp
}

/// Apply shelling damage to friendly fleet HP.
/// Uses `api_at_eflag` to determine attack direction:
///   eflag=1 means enemy attacking -> target index is a friendly ship (0-based)
fn apply_hougeki_damage(hp: &mut [i32], api_data: &serde_json::Value, key: &str) {
    let hougeki = match api_data.get(key) {
        Some(v) if !v.is_null() => v,
        _ => return,
    };
    let eflag_list = hougeki.get("api_at_eflag").and_then(|v| v.as_array());
    let df_list = hougeki.get("api_df_list").and_then(|v| v.as_array());
    let damage_list = hougeki.get("api_damage").and_then(|v| v.as_array());

    if let (Some(eflag), Some(df), Some(dmg)) = (eflag_list, df_list, damage_list) {
        for ((targets, damages), ef) in df.iter().zip(dmg.iter()).zip(eflag.iter()) {
            let is_enemy_attacking = ef.as_i64().unwrap_or(0) == 1;
            if !is_enemy_attacking {
                continue; // Friendly attacking enemy -- skip for friendly HP calc
            }
            let target_arr = targets.as_array();
            let damage_arr = damages.as_array();
            if let (Some(ts), Some(ds)) = (target_arr, damage_arr) {
                for (t, d) in ts.iter().zip(ds.iter()) {
                    let target_idx = t.as_i64().unwrap_or(-1) as i32;
                    let damage = d.as_f64().unwrap_or(0.0) as i32;
                    // Target is 0-based friendly ship index
                    if target_idx >= 0 && (target_idx as usize) < hp.len() {
                        hp[target_idx as usize] -= damage;
                    }
                }
            }
        }
    } else {
        // Fallback for older format without api_at_eflag (1-6 = friendly, 7-12 = enemy)
        let df_list = hougeki.get("api_df_list").and_then(|v| v.as_array());
        let damage_list = hougeki.get("api_damage").and_then(|v| v.as_array());
        if let (Some(df), Some(dmg)) = (df_list, damage_list) {
            for (targets, damages) in df.iter().zip(dmg.iter()) {
                let target_arr = targets.as_array();
                let damage_arr = damages.as_array();
                if let (Some(ts), Some(ds)) = (target_arr, damage_arr) {
                    for (t, d) in ts.iter().zip(ds.iter()) {
                        let target_idx = t.as_i64().unwrap_or(-1) as i32;
                        let damage = d.as_f64().unwrap_or(0.0) as i32;
                        if target_idx >= 1 && target_idx <= 6 && (target_idx as usize) <= hp.len() {
                            hp[(target_idx - 1) as usize] -= damage;
                        }
                    }
                }
            }
        }
    }
}

/// Apply shelling damage to enemy fleet HP.
/// Uses `api_at_eflag` to determine attack direction:
///   eflag=0 means friendly attacking -> target index is an enemy ship (0-based)
fn apply_hougeki_damage_enemy(hp: &mut [i32], api_data: &serde_json::Value, key: &str) {
    let hougeki = match api_data.get(key) {
        Some(v) if !v.is_null() => v,
        _ => return,
    };
    let eflag_list = hougeki.get("api_at_eflag").and_then(|v| v.as_array());
    let df_list = hougeki.get("api_df_list").and_then(|v| v.as_array());
    let damage_list = hougeki.get("api_damage").and_then(|v| v.as_array());

    if let (Some(eflag), Some(df), Some(dmg)) = (eflag_list, df_list, damage_list) {
        for ((targets, damages), ef) in df.iter().zip(dmg.iter()).zip(eflag.iter()) {
            let is_friendly_attacking = ef.as_i64().unwrap_or(0) == 0;
            if !is_friendly_attacking {
                continue; // Enemy attacking friendly -- skip for enemy HP calc
            }
            let target_arr = targets.as_array();
            let damage_arr = damages.as_array();
            if let (Some(ts), Some(ds)) = (target_arr, damage_arr) {
                for (t, d) in ts.iter().zip(ds.iter()) {
                    let target_idx = t.as_i64().unwrap_or(-1) as i32;
                    let damage = d.as_f64().unwrap_or(0.0) as i32;
                    // Target is 0-based enemy ship index
                    if target_idx >= 0 && (target_idx as usize) < hp.len() {
                        hp[target_idx as usize] -= damage;
                    }
                }
            }
        }
    } else {
        // Fallback for older format without api_at_eflag (7-12 = enemy, 1-based)
        let df_list = hougeki.get("api_df_list").and_then(|v| v.as_array());
        let damage_list = hougeki.get("api_damage").and_then(|v| v.as_array());
        if let (Some(df), Some(dmg)) = (df_list, damage_list) {
            for (targets, damages) in df.iter().zip(dmg.iter()) {
                let target_arr = targets.as_array();
                let damage_arr = damages.as_array();
                if let (Some(ts), Some(ds)) = (target_arr, damage_arr) {
                    for (t, d) in ts.iter().zip(ds.iter()) {
                        let target_idx = t.as_i64().unwrap_or(-1) as i32;
                        let damage = d.as_f64().unwrap_or(0.0) as i32;
                        if target_idx >= 7 && target_idx <= 12 {
                            let idx = (target_idx - 7) as usize;
                            if idx < hp.len() {
                                hp[idx] -= damage;
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Apply torpedo/opening attack damage to friendly fleet.
/// `api_fdam` is indexed by ship position (0-based). May have trailing extra elements.
fn apply_raigeki_damage(hp: &mut [i32], api_data: &serde_json::Value, key: &str) {
    let raigeki = match api_data.get(key) {
        Some(v) if !v.is_null() => v,
        _ => return,
    };
    if let Some(fdam) = raigeki.get("api_fdam").and_then(|v| v.as_array()) {
        for (i, d) in fdam.iter().enumerate() {
            if i >= hp.len() {
                break;
            }
            let damage = d.as_f64().unwrap_or(0.0) as i32;
            hp[i] -= damage;
        }
    }
}

/// Apply torpedo/opening attack damage to enemy fleet.
/// `api_edam` is indexed by ship position (0-based). May have trailing extra elements.
fn apply_raigeki_damage_enemy(hp: &mut [i32], api_data: &serde_json::Value, key: &str) {
    let raigeki = match api_data.get(key) {
        Some(v) if !v.is_null() => v,
        _ => return,
    };
    if let Some(edam) = raigeki.get("api_edam").and_then(|v| v.as_array()) {
        for (i, d) in edam.iter().enumerate() {
            if i >= hp.len() {
                break;
            }
            let damage = d.as_f64().unwrap_or(0.0) as i32;
            hp[i] -= damage;
        }
    }
}

/// Apply air battle damage to friendly fleet
fn apply_kouku_damage(hp: &mut [i32], api_data: &serde_json::Value, key: &str) {
    let kouku = match api_data.get(key) {
        Some(v) if !v.is_null() => v,
        _ => return,
    };
    // api_stage3.api_fdam = bomb damage to friendly fleet
    if let Some(stage3) = kouku.get("api_stage3") {
        if let Some(fdam) = stage3.get("api_fdam").and_then(|v| v.as_array()) {
            for (i, d) in fdam.iter().enumerate() {
                let damage = d.as_f64().unwrap_or(0.0) as i32;
                if i < hp.len() {
                    hp[i] -= damage;
                }
            }
        }
    }
}

/// Apply air battle damage to enemy fleet
fn apply_kouku_damage_enemy(hp: &mut [i32], api_data: &serde_json::Value, key: &str) {
    let kouku = match api_data.get(key) {
        Some(v) if !v.is_null() => v,
        _ => return,
    };
    // api_stage3.api_edam = bomb damage to enemy fleet
    if let Some(stage3) = kouku.get("api_stage3") {
        if let Some(edam) = stage3.get("api_edam").and_then(|v| v.as_array()) {
            for (i, d) in edam.iter().enumerate() {
                let damage = d.as_f64().unwrap_or(0.0) as i32;
                if i < hp.len() {
                    hp[i] -= damage;
                }
            }
        }
    }
}
