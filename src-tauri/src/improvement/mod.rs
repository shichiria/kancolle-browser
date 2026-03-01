use serde::{Deserialize, Serialize};
use std::collections::HashSet;
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
    #[serde(default)]
    convert_to: serde_json::Value,
    #[serde(default)]
    upgrade_for: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct ImprovementPath {
    helpers: Vec<ImprovementHelper>,
    #[serde(default)]
    convert: serde_json::Value,
    #[serde(default)]
    costs: serde_json::Value,
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
    pub eq_type: i32,
    pub type_name: String,
    pub sort_value: i32,
    pub available_today: bool,
    pub today_helpers: Vec<String>,
    pub matches_secretary: bool,
    pub previously_improved: bool,
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
            serde_json::from_str::<Vec<i32>>(&content)
                .unwrap_or_default()
                .into_iter()
                .collect()
        }
        Err(_) => HashSet::new(),
    }
}

pub fn save_improved_history(path: &Path, history: &HashSet<i32>) {
    let ids: Vec<i32> = history.iter().copied().collect();
    if let Ok(json) = serde_json::to_string(&ids) {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, json);
    }
}

// =============================================================================
// Build improvement list
// =============================================================================

/// Get current JST day of week (0=Sun..6=Sat)
fn jst_day_of_week() -> i32 {
    use chrono::{Datelike, Utc, FixedOffset};
    let jst = FixedOffset::east_opt(9 * 3600).unwrap();
    let now_jst = Utc::now().with_timezone(&jst);
    now_jst.weekday().num_days_from_sunday() as i32
}

pub fn build_improvement_list(state: &GameStateInner) -> ImprovementListResponse {
    let upgrade_data = get_upgrade_data();
    let day_of_week = jst_day_of_week();

    // Get 2nd ship in fleet 1 (the helper ship for Akashi's improvement arsenal)
    // In KanColle, the 2nd ship determines which improvements are available
    let second_ship_master_id = state
        .profile.fleets
        .first()
        .and_then(|f| f.get(1))
        .and_then(|&id| state.profile.ships.get(&id))
        .map(|s| s.ship_id)
        .unwrap_or(0);

    let second_ship_name = state
        .profile.fleets
        .first()
        .and_then(|f| f.get(1))
        .and_then(|&id| state.profile.ships.get(&id))
        .map(|s| s.name.clone())
        .unwrap_or_default();

    let mut items = Vec::new();

    for entry in upgrade_data {
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
                    if helper.ship_ids.contains(&second_ship_master_id) {
                        matches_secretary = true;
                    }
                    for &ship_id in &helper.ship_ids {
                        let ship_name = state
                            .master.ships
                            .get(&ship_id)
                            .map(|s| s.name.clone())
                            .unwrap_or_else(|| format!("ID:{}", ship_id));
                        if !today_helpers.contains(&ship_name) {
                            today_helpers.push(ship_name);
                        }
                    }
                }
            }
        }

        let previously_improved = state.history.improved_equipment.contains(&entry.eq_id);

        items.push(ImprovementItem {
            eq_id: entry.eq_id,
            name: master_info.name.clone(),
            eq_type,
            type_name,
            sort_value,
            available_today,
            today_helpers,
            matches_secretary,
            previously_improved,
        });
    }

    ImprovementListResponse {
        items,
        day_of_week,
        secretary_ship: second_ship_name,
    }
}
