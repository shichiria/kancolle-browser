use serde::{Deserialize, Serialize};

// =============================================================================
// Ship type constants
// =============================================================================

const STYPE_DE: i32 = 1;
const STYPE_DD: i32 = 2;
const STYPE_CL: i32 = 3;
const STYPE_CVL: i32 = 7;
const STYPE_CT: i32 = 21;

// =============================================================================
// Data structures (JSON-deserializable)
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GreatSuccessType {
    Regular,
    Drum,
    Level,
}

/// A single condition from the JSON file
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum ExpeditionCondition {
    FlagshipLevel { value: i32 },
    LevelSum { value: i32 },
    ShipCount { value: i32 },
    SmallShipCount { value: i32 },
    ShipTypeCount { ship_type: String, stypes: Vec<i32>, value: i32 },
    FlagshipType { ship_type: String, stypes: Vec<i32> },
    SubmarineCount { value: i32 },
    AircraftCarrierCount { value: i32 },
    EscortFleet,
    EscortFleetDD3,
    EscortFleetDD4,
    DrumShipCount { value: i32 },
    DrumTotal { value: i32 },
    Firepower { value: i32 },
    AA { value: i32 },
    ASW { value: i32 },
    LOS { value: i32 },
}

/// Definition of a single expedition (loaded from JSON)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExpeditionDef {
    pub id: i32,
    pub display_id: String,
    pub name: String,
    pub great_success_type: GreatSuccessType,
    pub duration_minutes: i32,
    #[serde(skip_serializing)]
    pub conditions: Vec<ExpeditionCondition>,
}

/// Input data for one ship in the fleet being checked
#[derive(Debug, Clone)]
pub struct FleetShipData {
    pub ship_type: i32,
    pub level: i32,
    pub firepower: i32,
    pub aa: i32,
    pub asw: i32,
    pub los: i32,
    pub cond: i32,
    pub has_drum: bool,
    pub drum_count: i32,
}

/// Input data for a whole fleet
#[derive(Debug, Clone)]
pub struct FleetCheckData {
    pub ships: Vec<FleetShipData>,
}

/// Result of checking a single condition
#[derive(Debug, Clone, Serialize)]
pub struct ConditionResult {
    pub condition: String,
    pub satisfied: bool,
    pub current_value: String,
    pub required_value: String,
}

#[derive(Debug, Clone, Serialize)]
pub enum ExpeditionResultType {
    Failure,
    Success,
    GreatSuccess,
}

/// Full result of checking an expedition against a fleet
#[derive(Debug, Clone, Serialize)]
pub struct ExpeditionCheckResult {
    pub expedition_id: i32,
    pub expedition_name: String,
    pub display_id: String,
    pub result: ExpeditionResultType,
    pub conditions: Vec<ConditionResult>,
}

// =============================================================================
// Ship type helpers
// =============================================================================

fn is_small_ship(stype: i32) -> bool {
    stype == STYPE_DD || stype == STYPE_DE
}

fn is_submarine(stype: i32) -> bool {
    stype == 13 || stype == 14 // SS, SSV
}

fn is_aircraft_carrier_no_av(stype: i32) -> bool {
    stype == 11 || stype == STYPE_CVL || stype == 18 // CV, CVL, CVB
}

fn count_by_stypes(ships: &[FleetShipData], stypes: &[i32]) -> i32 {
    ships.iter().filter(|s| stypes.contains(&s.ship_type)).count() as i32
}

/// Escort fleet: (CL≥1 AND DD+DE≥2) OR (CVL≥1 AND DD≥2) OR (DD≥1 AND DE≥3) OR (CT≥1 AND DE≥2)
fn check_escort_fleet(ships: &[FleetShipData]) -> bool {
    let cl = count_by_stypes(ships, &[STYPE_CL]);
    let dd = count_by_stypes(ships, &[STYPE_DD]);
    let de = count_by_stypes(ships, &[STYPE_DE]);
    let cvl = count_by_stypes(ships, &[STYPE_CVL]);
    let ct = count_by_stypes(ships, &[STYPE_CT]);

    (cl >= 1 && (dd + de) >= 2) || (cvl >= 1 && dd >= 2) || (dd >= 1 && de >= 3) || (ct >= 1 && de >= 2)
}

fn check_escort_fleet_dd(ships: &[FleetShipData], min_dd: i32) -> bool {
    let dd = count_by_stypes(ships, &[STYPE_DD]);
    check_escort_fleet(ships) && dd >= min_dd
}

// =============================================================================
// Load expedition data from embedded JSON
// =============================================================================

const EXPEDITIONS_JSON: &str = include_str!("../../data/expeditions.json");

/// Load all expedition definitions from the embedded JSON.
pub fn get_all_expeditions() -> Vec<ExpeditionDef> {
    serde_json::from_str(EXPEDITIONS_JSON).expect("Failed to parse expeditions.json")
}

// =============================================================================
// Condition checking
// =============================================================================

fn check_condition(cond: &ExpeditionCondition, fleet: &FleetCheckData) -> ConditionResult {
    match cond {
        ExpeditionCondition::FlagshipLevel { value } => {
            let current = fleet.ships.first().map(|s| s.level).unwrap_or(0);
            ConditionResult {
                condition: "旗艦レベル".into(),
                satisfied: current >= *value,
                current_value: format!("Lv.{}", current),
                required_value: format!("Lv.{}", value),
            }
        }
        ExpeditionCondition::LevelSum { value } => {
            let current: i32 = fleet.ships.iter().map(|s| s.level).sum();
            ConditionResult {
                condition: "合計レベル".into(),
                satisfied: current >= *value,
                current_value: format!("{}", current),
                required_value: format!("{}", value),
            }
        }
        ExpeditionCondition::ShipCount { value } => {
            let current = fleet.ships.len() as i32;
            ConditionResult {
                condition: "艦数".into(),
                satisfied: current >= *value,
                current_value: format!("{}隻", current),
                required_value: format!("{}隻", value),
            }
        }
        ExpeditionCondition::SmallShipCount { value } => {
            let current = fleet.ships.iter().filter(|s| is_small_ship(s.ship_type)).count() as i32;
            ConditionResult {
                condition: "駆逐/海防".into(),
                satisfied: current >= *value,
                current_value: format!("{}隻", current),
                required_value: format!("{}隻", value),
            }
        }
        ExpeditionCondition::ShipTypeCount { ship_type, stypes, value } => {
            let current = count_by_stypes(&fleet.ships, stypes);
            ConditionResult {
                condition: ship_type.clone(),
                satisfied: current >= *value,
                current_value: format!("{}隻", current),
                required_value: format!("{}隻", value),
            }
        }
        ExpeditionCondition::FlagshipType { ship_type, stypes } => {
            let flagship_stype = fleet.ships.first().map(|s| s.ship_type).unwrap_or(0);
            let satisfied = stypes.contains(&flagship_stype);
            ConditionResult {
                condition: format!("旗艦艦種: {}", ship_type),
                satisfied,
                current_value: format!("stype={}", flagship_stype),
                required_value: ship_type.clone(),
            }
        }
        ExpeditionCondition::SubmarineCount { value } => {
            let current = fleet.ships.iter().filter(|s| is_submarine(s.ship_type)).count() as i32;
            ConditionResult {
                condition: "潜水艦".into(),
                satisfied: current >= *value,
                current_value: format!("{}隻", current),
                required_value: format!("{}隻", value),
            }
        }
        ExpeditionCondition::AircraftCarrierCount { value } => {
            let current = fleet.ships.iter().filter(|s| is_aircraft_carrier_no_av(s.ship_type)).count() as i32;
            ConditionResult {
                condition: "空母".into(),
                satisfied: current >= *value,
                current_value: format!("{}隻", current),
                required_value: format!("{}隻", value),
            }
        }
        ExpeditionCondition::EscortFleet => {
            let satisfied = check_escort_fleet(&fleet.ships);
            ConditionResult {
                condition: "護衛艦隊編成".into(),
                satisfied,
                current_value: if satisfied { "OK" } else { "NG" }.into(),
                required_value: "護衛編成".into(),
            }
        }
        ExpeditionCondition::EscortFleetDD3 => {
            let satisfied = check_escort_fleet_dd(&fleet.ships, 3);
            ConditionResult {
                condition: "護衛艦隊(DD3+)".into(),
                satisfied,
                current_value: if satisfied { "OK" } else { "NG" }.into(),
                required_value: "護衛編成+DD3".into(),
            }
        }
        ExpeditionCondition::EscortFleetDD4 => {
            let satisfied = check_escort_fleet_dd(&fleet.ships, 4);
            ConditionResult {
                condition: "護衛艦隊(DD4+)".into(),
                satisfied,
                current_value: if satisfied { "OK" } else { "NG" }.into(),
                required_value: "護衛編成+DD4".into(),
            }
        }
        ExpeditionCondition::DrumShipCount { value } => {
            let current = fleet.ships.iter().filter(|s| s.has_drum).count() as i32;
            ConditionResult {
                condition: "ドラム缶搭載艦".into(),
                satisfied: current >= *value,
                current_value: format!("{}隻", current),
                required_value: format!("{}隻", value),
            }
        }
        ExpeditionCondition::DrumTotal { value } => {
            let current: i32 = fleet.ships.iter().map(|s| s.drum_count).sum();
            ConditionResult {
                condition: "ドラム缶合計".into(),
                satisfied: current >= *value,
                current_value: format!("{}個", current),
                required_value: format!("{}個", value),
            }
        }
        ExpeditionCondition::Firepower { value } => {
            let current: i32 = fleet.ships.iter().map(|s| s.firepower).sum();
            ConditionResult {
                condition: "火力合計".into(),
                satisfied: current >= *value,
                current_value: format!("{}", current),
                required_value: format!("{}", value),
            }
        }
        ExpeditionCondition::AA { value } => {
            let current: i32 = fleet.ships.iter().map(|s| s.aa).sum();
            ConditionResult {
                condition: "対空合計".into(),
                satisfied: current >= *value,
                current_value: format!("{}", current),
                required_value: format!("{}", value),
            }
        }
        ExpeditionCondition::ASW { value } => {
            let current: i32 = fleet.ships.iter().map(|s| s.asw).sum();
            ConditionResult {
                condition: "対潜合計".into(),
                satisfied: current >= *value,
                current_value: format!("{}", current),
                required_value: format!("{}", value),
            }
        }
        ExpeditionCondition::LOS { value } => {
            let current: i32 = fleet.ships.iter().map(|s| s.los).sum();
            ConditionResult {
                condition: "索敵合計".into(),
                satisfied: current >= *value,
                current_value: format!("{}", current),
                required_value: format!("{}", value),
            }
        }
    }
}

// =============================================================================
// Main check function
// =============================================================================

pub fn check_expedition(expedition_id: i32, fleet: &FleetCheckData) -> ExpeditionCheckResult {
    let all = get_all_expeditions();
    let exp = match all.iter().find(|e| e.id == expedition_id) {
        Some(e) => e,
        None => {
            return ExpeditionCheckResult {
                expedition_id,
                expedition_name: format!("Unknown({})", expedition_id),
                display_id: "??".into(),
                result: ExpeditionResultType::Failure,
                conditions: vec![ConditionResult {
                    condition: "遠征データ".into(),
                    satisfied: false,
                    current_value: "不明".into(),
                    required_value: "有効な遠征ID".into(),
                }],
            };
        }
    };

    let conditions: Vec<ConditionResult> = exp.conditions.iter().map(|c| check_condition(c, fleet)).collect();
    let all_satisfied = conditions.iter().all(|c| c.satisfied);

    let result = if !all_satisfied {
        ExpeditionResultType::Failure
    } else if fleet.ships.iter().all(|s| s.cond >= 50) {
        ExpeditionResultType::GreatSuccess
    } else {
        ExpeditionResultType::Success
    };

    ExpeditionCheckResult {
        expedition_id: exp.id,
        expedition_name: exp.name.clone(),
        display_id: exp.display_id.clone(),
        result,
        conditions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_loads() {
        let exps = get_all_expeditions();
        assert!(exps.len() > 50);
        let exp01 = exps.iter().find(|e| e.id == 1).unwrap();
        assert_eq!(exp01.display_id, "01");
        assert_eq!(exp01.name, "練習航海");
    }

    #[test]
    fn test_simple_check() {
        let fleet = FleetCheckData {
            ships: vec![
                FleetShipData { ship_type: 2, level: 50, firepower: 30, aa: 30, asw: 50, los: 10, cond: 49, has_drum: false, drum_count: 0 },
                FleetShipData { ship_type: 2, level: 30, firepower: 25, aa: 25, asw: 40, los: 8, cond: 49, has_drum: false, drum_count: 0 },
            ],
        };
        let result = check_expedition(1, &fleet);
        assert!(matches!(result.result, ExpeditionResultType::Success));
    }

    #[test]
    fn test_great_success() {
        let fleet = FleetCheckData {
            ships: vec![
                FleetShipData { ship_type: 2, level: 50, firepower: 30, aa: 30, asw: 50, los: 10, cond: 53, has_drum: false, drum_count: 0 },
                FleetShipData { ship_type: 2, level: 30, firepower: 25, aa: 25, asw: 40, los: 8, cond: 50, has_drum: false, drum_count: 0 },
            ],
        };
        let result = check_expedition(1, &fleet);
        assert!(matches!(result.result, ExpeditionResultType::GreatSuccess));
    }
}
