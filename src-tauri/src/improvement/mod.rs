use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::OnceLock;

use crate::api::models::{GameStateInner, MasterSlotItemInfo};

// =============================================================================
// Equipment upgrade data from EquipmentUpgrades.json
// =============================================================================

#[derive(Debug, Deserialize)]
struct EquipmentUpgradeEntry {
    eq_id: i32,
    improvement: Vec<ImprovementPath>,
    #[allow(dead_code)] // kept for JSON schema completeness
    #[serde(default)]
    convert_to: serde_json::Value,
    #[allow(dead_code)] // kept for JSON schema completeness
    #[serde(default)]
    upgrade_for: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct ImprovementPath {
    helpers: Vec<ImprovementHelper>,
    #[serde(default)]
    convert: serde_json::Value,
    #[serde(default)]
    costs: Option<ImprovementCosts>,
}

#[derive(Debug, Deserialize)]
struct ImprovementCosts {
    #[serde(default)]
    p1: Option<CostPhase>,
    #[serde(default)]
    p2: Option<CostPhase>,
    #[serde(default)]
    conv: Option<CostPhase>,
    #[serde(default)]
    extra: Vec<ExtraCost>,
}

#[derive(Debug, Deserialize)]
struct CostPhase {
    #[serde(default)]
    equips: Vec<CostEquip>,
}

#[derive(Debug, Deserialize)]
struct CostEquip {
    id: i32,
    eq_count: i32,
}

#[derive(Debug, Deserialize)]
struct ExtraCost {
    levels: Vec<i32>,
    #[serde(default)]
    equips: Vec<CostEquip>,
}

#[derive(Debug, Deserialize)]
struct ImprovementHelper {
    ship_ids: Vec<i32>,
    days: Vec<i32>,
}

// =============================================================================
// Response types sent to frontend
// =============================================================================

#[derive(Debug, Serialize)]
pub struct ImprovementListResponse {
    pub items: Vec<ImprovementItem>,
    pub day_of_week: i32,
    pub secretary_ship: String,
}

#[derive(Debug, Serialize)]
pub struct ImprovementItem {
    pub eq_id: i32,
    pub name: String,
    pub owned_count: i32,
    pub owned_levels: Vec<[i32; 2]>,
    pub equipment_ready: bool,
    pub can_improve_now: bool,
    pub eq_type: i32,
    pub type_name: String,
    pub sort_value: i32,
    pub available_today: bool,
    pub today_helpers: Vec<ImprovementHelperShip>,
    pub matches_secretary: bool,
    pub previously_improved: bool,
    pub consumed_equips: Vec<ConsumedEquipInfo>,
}

/// A ship that can act as today's 担当艦 (2nd-slot helper) for an improvement.
#[derive(Debug, Serialize)]
pub struct ImprovementHelperShip {
    pub name: String,
    /// Highest level among the owned copies of this ship; `None` if not owned.
    pub level: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct ConsumedEquipInfo {
    pub eq_id: i32,
    pub name: String,
    pub counts: [i32; 3], // [p1(★0-5), p2(★6-9), conv(更新)]
    pub owned: i32,       // total count, including locked equipment
}

// =============================================================================
// Static data
// =============================================================================

static UPGRADE_DATA: OnceLock<Vec<EquipmentUpgradeEntry>> = OnceLock::new();

fn get_upgrade_data() -> &'static [EquipmentUpgradeEntry] {
    UPGRADE_DATA.get_or_init(|| {
        let json_str = include_str!("../../data/equipment_upgrades.json");
        let json_str = json_str.strip_prefix('\u{feff}').unwrap_or(json_str);
        serde_json::from_str(json_str).expect("Failed to parse equipment_upgrades.json")
    })
}

// =============================================================================
// Equipment type helpers
// =============================================================================

fn get_type_name(eq_type: i32) -> &'static str {
    match eq_type {
        1 => "小口径主砲",
        2 => "中口径主砲",
        3 => "大口径主砲",
        4 => "副砲",
        5 => "魚雷",
        6 => "艦上戦闘機",
        7 => "艦上爆撃機",
        8 => "艦上攻撃機",
        9 => "艦上偵察機",
        10 => "水上偵察機",
        11 => "水上爆撃機",
        12 => "小型電探",
        13 => "大型電探",
        14 => "ソナー",
        15 => "爆雷",
        16 | 27 | 28 => "追加装甲",
        17 => "機関部強化",
        18 => "対空強化弾",
        19 => "対艦強化弾",
        21 => "対空機銃",
        22 => "特殊潜航艇",
        24 => "上陸用舟艇",
        25 => "オートジャイロ",
        26 => "対潜哨戒機",
        29 | 42 => "探照灯",
        32 => "潜水艦魚雷",
        33 => "照明弾",
        34 => "司令部施設",
        36 => "高射装置",
        37 => "対地装備",
        38 => "大口径主砲II",
        39 => "水上艦要員",
        40 => "大型ソナー",
        41 => "大型飛行艇",
        45 => "水上戦闘機",
        46 => "特型内火艇",
        47 => "陸上攻撃機",
        48 => "局地戦闘機",
        49 => "陸上偵察機",
        51 => "潜水艦装備",
        93 => "大型電探II",
        94 => "艦上偵察機II",
        95 => "副砲II",
        _ => "その他",
    }
}

/// Get primary stat value for sorting based on equipment type
fn get_primary_stat(eq_type: i32, info: &MasterSlotItemInfo) -> i32 {
    match eq_type {
        // Guns, AP shell, secondary guns, rockets
        1 | 2 | 3 | 4 | 19 | 37 | 38 | 95 => info.firepower,
        // Torpedoes, submarine torpedoes, midget subs
        5 | 22 | 32 => info.torpedo,
        // Torpedo bombers
        8 => info.torpedo,
        // Fighters, AA equipment
        6 | 18 | 21 | 36 | 45 | 48 => info.aa,
        // Bombers
        7 | 11 | 47 => info.bombing,
        // Recon, radar
        9 | 10 | 12 | 13 | 49 | 93 | 94 => info.los,
        // ASW
        14 | 15 | 25 | 26 | 40 | 41 => info.asw,
        // Others - use firepower as fallback
        _ => info.firepower,
    }
}

// =============================================================================
// Persistence for improved equipment history
// =============================================================================

pub fn load_improved_history(path: &Path) -> HashSet<i32> {
    match std::fs::read_to_string(path) {
        Ok(content) => {
            let set: HashSet<i32> = serde_json::from_str::<Vec<i32>>(&content)
                .unwrap_or_default()
                .into_iter()
                .collect();
            log::info!(
                "[Improvement] loaded {} improved equipment IDs from {}",
                set.len(),
                path.display()
            );
            set
        }
        Err(e) => {
            log::debug!(
                "[Improvement] no improved history at {}: {}",
                path.display(),
                e
            );
            HashSet::new()
        }
    }
}

pub fn save_improved_history(path: &Path, history: &HashSet<i32>) {
    let ids: Vec<i32> = history.iter().copied().collect();
    match serde_json::to_string(&ids) {
        Ok(json) => {
            if let Some(parent) = path.parent() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    log::warn!(
                        "[Improvement] failed to create dir {}: {}",
                        parent.display(),
                        e
                    );
                }
            }
            if let Err(e) = std::fs::write(path, json) {
                log::warn!(
                    "[Improvement] failed to save improved history to {}: {}",
                    path.display(),
                    e
                );
            }
        }
        Err(e) => {
            log::warn!("[Improvement] failed to serialize improved history: {}", e);
        }
    }
}

// =============================================================================
// Build improvement list
// =============================================================================

/// Get current JST day of week (0=Sun..6=Sat)
fn jst_day_of_week() -> i32 {
    use chrono::{Datelike, FixedOffset, Utc};
    let jst = FixedOffset::east_opt(9 * 3600).unwrap();
    let now_jst = Utc::now().with_timezone(&jst);
    now_jst.weekday().num_days_from_sunday() as i32
}

fn count_owned_equipment<I>(items: I) -> HashMap<i32, i32>
where
    I: IntoIterator<Item = (i32, bool)>,
{
    items
        .into_iter()
        .fold(HashMap::new(), |mut counts, (master_id, _locked)| {
            *counts.entry(master_id).or_insert(0) += 1;
            counts
        })
}

fn equipment_levels<I>(items: I) -> HashMap<i32, Vec<i32>>
where
    I: IntoIterator<Item = (i32, i32)>,
{
    items
        .into_iter()
        .fold(HashMap::new(), |mut levels, (master_id, level)| {
            levels.entry(master_id).or_default().push(level);
            levels
        })
}

fn equipment_requirements_met(
    phase: &CostPhase,
    extra_equips: &[&CostEquip],
    owned_counts: &HashMap<i32, i32>,
    source_eq_id: i32,
) -> bool {
    let mut requirements: HashMap<i32, i32> = HashMap::new();
    for equip in &phase.equips {
        *requirements.entry(equip.id).or_insert(0) += equip.eq_count;
    }
    for equip in extra_equips {
        *requirements.entry(equip.id).or_insert(0) += equip.eq_count;
    }
    // One copy is the item being improved and cannot simultaneously be
    // consumed as material when the recipe asks for the same master item.
    *requirements.entry(source_eq_id).or_insert(0) += 1;

    requirements
        .into_iter()
        .all(|(eq_id, required)| owned_counts.get(&eq_id).copied().unwrap_or(0) >= required)
}

fn path_equipment_ready(
    source_eq_id: i32,
    path: &ImprovementPath,
    source_levels: &[i32],
    owned_counts: &HashMap<i32, i32>,
) -> bool {
    let Some(costs) = path.costs.as_ref() else {
        return false;
    };

    source_levels.iter().any(|level| {
        let phase = match *level {
            0..=5 => costs.p1.as_ref(),
            6..=9 => costs.p2.as_ref(),
            10 if !path.convert.is_null() => costs.conv.as_ref(),
            _ => None,
        };
        let extra_equips: Vec<&CostEquip> = if *level < 10 {
            costs
                .extra
                .iter()
                .filter(|extra| extra.levels.contains(&(*level + 1)))
                .flat_map(|extra| extra.equips.iter())
                .collect()
        } else {
            Vec::new()
        };
        phase.is_some_and(|phase| {
            equipment_requirements_met(phase, &extra_equips, owned_counts, source_eq_id)
        })
    })
}

fn helper_matches(helper: &ImprovementHelper, day_of_week: i32, ship_id: i32) -> bool {
    helper.days.contains(&day_of_week)
        && (helper.ship_ids.is_empty() || helper.ship_ids.contains(&ship_id))
}

pub fn build_improvement_list(state: &GameStateInner) -> ImprovementListResponse {
    let upgrade_data = get_upgrade_data();
    let day_of_week = jst_day_of_week();

    // Get 2nd ship in fleet 1 (the helper ship for Akashi's improvement arsenal)
    // In KanColle, the 2nd ship determines which improvements are available
    let second_ship_master_id = state
        .profile
        .fleets
        .first()
        .and_then(|f| f.get(1))
        .and_then(|&id| state.profile.ships.get(&id))
        .map(|s| s.ship_id)
        .unwrap_or(0);

    let second_ship_name = state
        .profile
        .fleets
        .first()
        .and_then(|f| f.get(1))
        .and_then(|&id| state.profile.ships.get(&id))
        .map(|s| s.name.clone())
        .unwrap_or_default();

    // master ship_id → highest level among the owned copies.
    // Remodel forms are distinct master ids, so the recipe's ship_ids match directly.
    let owned_levels: HashMap<i32, i32> =
        state
            .profile
            .ships
            .values()
            .fold(HashMap::new(), |mut acc, ship| {
                acc.entry(ship.ship_id)
                    .and_modify(|lv| *lv = (*lv).max(ship.lv))
                    .or_insert(ship.lv);
                acc
            });
    let owned_equipment_counts = count_owned_equipment(
        state
            .profile
            .slotitems
            .values()
            .map(|item| (item.slotitem_id, item.locked)),
    );
    let owned_equipment_levels = equipment_levels(
        state
            .profile
            .slotitems
            .values()
            .map(|item| (item.slotitem_id, item.level)),
    );

    let mut items = Vec::new();

    for entry in upgrade_data {
        // Some source entries exist only to describe that the equipment is
        // consumed by another recipe. They are not themselves improvable.
        if entry.improvement.is_empty() {
            continue;
        }
        let master_info = match state.master.slotitems.get(&entry.eq_id) {
            Some(info) => info,
            None => continue,
        };

        let eq_type = master_info.item_type;
        let type_name = get_type_name(eq_type).to_string();
        let sort_value = get_primary_stat(eq_type, master_info);

        let mut available_today = false;
        let mut matches_secretary = false;
        let mut today_helpers = Vec::new();

        for imp in &entry.improvement {
            for helper in &imp.helpers {
                if helper.days.contains(&day_of_week) {
                    available_today = true;
                    if helper_matches(helper, day_of_week, second_ship_master_id) {
                        matches_secretary = true;
                    }
                    for &ship_id in &helper.ship_ids {
                        let name = state
                            .master
                            .ships
                            .get(&ship_id)
                            .map(|s| s.name.clone())
                            .unwrap_or_else(|| format!("ID:{}", ship_id));
                        if !today_helpers
                            .iter()
                            .any(|h: &ImprovementHelperShip| h.name == name)
                        {
                            today_helpers.push(ImprovementHelperShip {
                                name,
                                level: owned_levels.get(&ship_id).copied(),
                            });
                        }
                    }
                }
            }
        }

        let previously_improved = state.history.improved_equipment.contains(&entry.eq_id);
        let source_levels = owned_equipment_levels
            .get(&entry.eq_id)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let equipment_ready = entry.improvement.iter().any(|path| {
            path_equipment_ready(entry.eq_id, path, source_levels, &owned_equipment_counts)
        });
        let can_improve_now = entry.improvement.iter().any(|path| {
            path_equipment_ready(entry.eq_id, path, source_levels, &owned_equipment_counts)
                && path
                    .helpers
                    .iter()
                    .any(|helper| helper_matches(helper, day_of_week, second_ship_master_id))
        });
        let mut owned_level_counts: HashMap<i32, i32> = HashMap::new();
        for level in source_levels {
            *owned_level_counts.entry(*level).or_insert(0) += 1;
        }
        let mut owned_levels: Vec<[i32; 2]> = owned_level_counts
            .into_iter()
            .map(|(level, count)| [level, count])
            .collect();
        owned_levels.sort_by_key(|entry| entry[0]);

        // Collect consumed equips across all improvement paths
        let mut consumed_map: HashMap<i32, [i32; 3]> = HashMap::new();
        for imp in &entry.improvement {
            if let Some(ref costs) = imp.costs {
                let phases = [costs.p1.as_ref(), costs.p2.as_ref(), costs.conv.as_ref()];
                for (phase_idx, phase) in phases.iter().enumerate() {
                    if let Some(p) = phase {
                        for equip in &p.equips {
                            let e = consumed_map.entry(equip.id).or_insert([0, 0, 0]);
                            e[phase_idx] = e[phase_idx].max(equip.eq_count);
                        }
                    }
                }
                for extra in &costs.extra {
                    let phase_idx = if extra.levels.iter().any(|level| *level <= 6) {
                        0
                    } else {
                        1
                    };
                    for equip in &extra.equips {
                        let e = consumed_map.entry(equip.id).or_insert([0, 0, 0]);
                        e[phase_idx] = e[phase_idx].max(equip.eq_count);
                    }
                }
            }
        }
        let consumed_equips: Vec<ConsumedEquipInfo> = consumed_map
            .into_iter()
            .map(|(eq_id, counts)| {
                let name = state
                    .master
                    .slotitems
                    .get(&eq_id)
                    .map(|info| info.name.clone())
                    .unwrap_or_else(|| format!("装備ID:{}", eq_id));
                let owned = owned_equipment_counts.get(&eq_id).copied().unwrap_or(0);
                ConsumedEquipInfo {
                    eq_id,
                    name,
                    counts,
                    owned,
                }
            })
            .collect();

        items.push(ImprovementItem {
            eq_id: entry.eq_id,
            name: master_info.name.clone(),
            owned_count: owned_equipment_counts
                .get(&entry.eq_id)
                .copied()
                .unwrap_or(0),
            owned_levels,
            equipment_ready,
            can_improve_now,
            eq_type,
            type_name,
            sort_value,
            available_today,
            today_helpers,
            matches_secretary,
            previously_improved,
            consumed_equips,
        });
    }

    ImprovementListResponse {
        items,
        day_of_week,
        secretary_ship: second_ship_name,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{
        count_owned_equipment, equipment_requirements_met, path_equipment_ready, CostEquip,
        CostPhase, ImprovementCosts, ImprovementHelper, ImprovementPath,
    };

    #[test]
    fn owned_equipment_count_includes_locked_items() {
        let counts = count_owned_equipment([(28, false), (28, true), (28, true), (29, true)]);

        assert_eq!(counts.get(&28), Some(&3));
        assert_eq!(counts.get(&29), Some(&1));
    }

    #[test]
    fn readiness_uses_the_phase_for_the_owned_star_level() {
        let path = ImprovementPath {
            helpers: vec![],
            convert: serde_json::Value::Null,
            costs: Some(ImprovementCosts {
                p1: Some(CostPhase {
                    equips: vec![CostEquip {
                        id: 28,
                        eq_count: 2,
                    }],
                }),
                p2: Some(CostPhase {
                    equips: vec![CostEquip {
                        id: 29,
                        eq_count: 3,
                    }],
                }),
                conv: None,
                extra: vec![],
            }),
        };
        let counts = HashMap::from([(100, 1), (28, 2), (29, 2)]);

        assert!(path_equipment_ready(100, &path, &[5], &counts));
        assert!(!path_equipment_ready(100, &path, &[6], &counts));
        assert!(!path_equipment_ready(100, &path, &[10], &counts));
    }

    #[test]
    fn readiness_reserves_the_source_when_it_is_also_material() {
        let phase = CostPhase {
            equips: vec![CostEquip {
                id: 100,
                eq_count: 2,
            }],
        };

        assert!(!equipment_requirements_met(
            &phase,
            &[],
            &HashMap::from([(100, 2)]),
            100,
        ));
        assert!(equipment_requirements_met(
            &phase,
            &[],
            &HashMap::from([(100, 3)]),
            100,
        ));
    }

    #[test]
    fn readiness_includes_extra_equipment_for_the_next_star_level() {
        let path = ImprovementPath {
            helpers: vec![],
            convert: serde_json::Value::Null,
            costs: Some(ImprovementCosts {
                p1: None,
                p2: Some(CostPhase {
                    equips: vec![CostEquip {
                        id: 129,
                        eq_count: 3,
                    }],
                }),
                conv: None,
                extra: vec![super::ExtraCost {
                    levels: vec![8],
                    equips: vec![CostEquip {
                        id: 145,
                        eq_count: 4,
                    }],
                }],
            }),
        };
        let missing_extra = HashMap::from([(575, 1), (129, 3), (145, 3)]);
        let enough_extra = HashMap::from([(575, 1), (129, 3), (145, 4)]);

        assert!(!path_equipment_ready(575, &path, &[7], &missing_extra));
        assert!(path_equipment_ready(575, &path, &[7], &enough_extra));
        assert!(path_equipment_ready(575, &path, &[8], &missing_extra));
    }

    #[test]
    fn recipe_without_a_second_ship_requirement_matches_on_its_listed_days() {
        let helper = ImprovementHelper {
            ship_ids: vec![],
            days: vec![3, 4],
        };

        assert!(super::helper_matches(&helper, 3, 0));
        assert!(super::helper_matches(&helper, 4, 999));
        assert!(!super::helper_matches(&helper, 2, 999));
    }
}
