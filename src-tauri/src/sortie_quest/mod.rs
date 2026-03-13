use serde::{Deserialize, Serialize};

// =============================================================================
// Data structures (JSON-deserializable)
// =============================================================================

/// A single condition from the JSON file
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum SortieQuestCondition {
    /// Minimum number of ships in fleet
    ShipCount { value: i32 },
    /// Minimum count of ships with specific stypes
    ShipTypeCount {
        ship_type: String,
        stypes: Vec<i32>,
        value: i32,
    },
    /// Flagship must be one of the given stypes
    FlagshipType {
        ship_type: String,
        stypes: Vec<i32>,
    },
    /// Fleet must contain `count` ships whose name starts with one of `names`
    ContainsShipName {
        names: Vec<String>,
        count: i32,
    },
    /// Fleet must contain `count` ships whose name starts with ANY of `names` (OR match)
    ContainsShipNameAny {
        names: Vec<String>,
        count: i32,
    },
    /// Fleet can only contain ships of these stypes
    OnlyShipTypes {
        desc: String,
        stypes: Vec<i32>,
    },
    /// Maximum count of ships with specific stypes (for routing: e.g. "戦艦+空母 <= 2")
    MaxShipTypeCount {
        ship_type: String,
        stypes: Vec<i32>,
        value: i32,
    },
    /// Any ONE of the alternative condition groups must be satisfied (OR logic)
    OrConditions {
        desc: String,
        alternatives: Vec<Vec<SortieQuestCondition>>,
    },
}

/// Per-map recommended fleet composition (used within sortie quests)
#[derive(Debug, Clone, Deserialize)]
pub struct MapRecommendation {
    pub area: String,
    pub fleet: Vec<SortieQuestCondition>,
}

// =============================================================================
// Map recommendation data structures (for normal sortie maps)
// =============================================================================

/// A single route recommendation for a map
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MapRecommendationRoute {
    pub desc: String,
    #[serde(skip_serializing)]
    pub fleet: Vec<SortieQuestCondition>,
}

/// Definition of map recommendations for one area
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MapRecommendationDef {
    pub area: String,
    pub name: String,
    #[serde(skip_serializing)]
    pub routes: Vec<MapRecommendationRoute>,
}

/// Result of checking one route against the current fleet
#[derive(Debug, Clone, Serialize)]
pub struct MapRouteCheckResult {
    pub desc: String,
    pub satisfied: bool,
    pub conditions: Vec<ConditionResult>,
}

/// Result of checking all routes for one map
#[derive(Debug, Clone, Serialize)]
pub struct MapRecommendationCheckResult {
    pub area: String,
    pub name: String,
    pub routes: Vec<MapRouteCheckResult>,
}

/// Sub-goal for quests with multiple independent conditions (e.g. あ号作戦)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SubGoal {
    pub name: String,
    pub count: i32,
    #[serde(default)]
    pub boss_only: bool,
    #[serde(default)]
    pub rank: String,
    /// Optional area filter for per-area sub-goals (e.g. Bq2: 6-4 requires S rank)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub area: Option<String>,
}

/// Definition of a single sortie quest (loaded from JSON)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SortieQuestDef {
    pub id: i32,
    pub quest_id: String,
    pub name: String,
    pub area: String,
    pub rank: String,
    pub boss_only: bool,
    pub count: i32,
    pub reset: String,
    /// true = confirmed no fleet conditions; false = conditions unknown or present
    #[serde(default)]
    pub no_conditions: bool,
    /// Counter reset override (e.g. "daily" for exercise quests that reset progress daily)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub counter_reset: Option<String>,
    /// Optional note shown to the user (e.g. "※第２艦隊で出撃")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Multiple independent sub-conditions (e.g. あ号作戦: 出撃/S勝利/ボス到達/ボス勝利)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sub_goals: Vec<SubGoal>,
    /// Enemy ship type to count for sinking quests (carrier/transport/submarine)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enemy_type: Option<String>,
    #[serde(skip_serializing)]
    pub conditions: Vec<SortieQuestCondition>,
    #[serde(default, skip_serializing)]
    pub recommended: Vec<MapRecommendation>,
}

/// Input data for one ship in the fleet being checked
#[derive(Debug, Clone)]
pub struct FleetShipData {
    pub name: String,
    pub ship_type: i32,
    pub level: i32,
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

/// Result of checking recommended fleet for one map
#[derive(Debug, Clone, Serialize)]
pub struct MapRecommendedResult {
    pub area: String,
    pub satisfied: bool,
    pub conditions: Vec<ConditionResult>,
}

/// Full result of checking a sortie quest against a fleet
#[derive(Debug, Clone, Serialize)]
pub struct SortieQuestCheckResult {
    pub quest_id: String,
    pub quest_name: String,
    pub area: String,
    pub rank: String,
    pub boss_only: bool,
    pub count: i32,
    pub no_conditions: bool,
    pub note: Option<String>,
    pub satisfied: bool,
    pub conditions: Vec<ConditionResult>,
    pub recommended: Vec<MapRecommendedResult>,
}

// =============================================================================
// Load sortie quest data from embedded JSON
// =============================================================================

const SORTIE_QUESTS_JSON: &str = include_str!("../../data/sortie_quests.json");

static SORTIE_QUESTS_DATA: std::sync::OnceLock<Vec<SortieQuestDef>> = std::sync::OnceLock::new();

/// Load all sortie quest definitions from the embedded JSON (cached after first call).
pub fn get_all_sortie_quests() -> Vec<SortieQuestDef> {
    SORTIE_QUESTS_DATA.get_or_init(|| {
        serde_json::from_str(SORTIE_QUESTS_JSON).expect("Failed to parse sortie_quests.json")
    }).clone()
}

// =============================================================================
// Condition checking
// =============================================================================

/// Check if a ship name starts with any of the given prefixes
fn name_matches(ship_name: &str, prefixes: &[String]) -> bool {
    prefixes.iter().any(|prefix| ship_name.starts_with(prefix))
}

fn check_condition(cond: &SortieQuestCondition, fleet: &FleetCheckData) -> ConditionResult {
    match cond {
        SortieQuestCondition::ShipCount { value } => {
            let current = fleet.ships.len() as i32;
            ConditionResult {
                condition: "艦数".into(),
                satisfied: current >= *value,
                current_value: format!("{}隻", current),
                required_value: format!("{}隻", value),
            }
        }
        SortieQuestCondition::ShipTypeCount {
            ship_type,
            stypes,
            value,
        } => {
            let current = fleet
                .ships
                .iter()
                .filter(|s| stypes.contains(&s.ship_type))
                .count() as i32;
            ConditionResult {
                condition: ship_type.clone(),
                satisfied: current >= *value,
                current_value: format!("{}隻", current),
                required_value: format!("{}隻", value),
            }
        }
        SortieQuestCondition::FlagshipType { ship_type, stypes } => {
            let flagship_stype = fleet.ships.first().map(|s| s.ship_type).unwrap_or(0);
            let satisfied = stypes.contains(&flagship_stype);
            ConditionResult {
                condition: format!("旗艦: {}", ship_type),
                satisfied,
                current_value: if satisfied {
                    "OK".into()
                } else {
                    format!("stype={}", flagship_stype)
                },
                required_value: ship_type.clone(),
            }
        }
        SortieQuestCondition::ContainsShipName { names, count } => {
            // Each name must be matched by a different ship (all required)
            let matched: Vec<&str> = fleet
                .ships
                .iter()
                .filter(|s| name_matches(&s.name, names))
                .map(|s| s.name.as_str())
                .collect();
            let current = matched.len() as i32;
            let display_names = names.join("・");
            ConditionResult {
                condition: display_names.clone(),
                satisfied: current >= *count,
                current_value: format!("{}隻", current),
                required_value: format!("{}隻", count),
            }
        }
        SortieQuestCondition::ContainsShipNameAny { names, count } => {
            // Count ships matching any of the names
            let current = fleet
                .ships
                .iter()
                .filter(|s| name_matches(&s.name, names))
                .count() as i32;
            let display_names = names.join("/");
            ConditionResult {
                condition: format!("{}から", display_names),
                satisfied: current >= *count,
                current_value: format!("{}隻", current),
                required_value: format!("{}隻", count),
            }
        }
        SortieQuestCondition::OnlyShipTypes { desc, stypes } => {
            let violators: Vec<&str> = fleet
                .ships
                .iter()
                .filter(|s| !stypes.contains(&s.ship_type))
                .map(|s| s.name.as_str())
                .collect();
            let satisfied = violators.is_empty();
            ConditionResult {
                condition: format!("{}のみ", desc),
                satisfied,
                current_value: if satisfied {
                    "OK".into()
                } else {
                    violators.join(",")
                },
                required_value: format!("{}のみ", desc),
            }
        }
        SortieQuestCondition::MaxShipTypeCount {
            ship_type,
            stypes,
            value,
        } => {
            let current = fleet
                .ships
                .iter()
                .filter(|s| stypes.contains(&s.ship_type))
                .count() as i32;
            ConditionResult {
                condition: format!("{}上限", ship_type),
                satisfied: current <= *value,
                current_value: format!("{}隻", current),
                required_value: format!("{}隻以下", value),
            }
        }
        SortieQuestCondition::OrConditions { desc, alternatives } => {
            let any_satisfied = alternatives.iter().any(|group| {
                group
                    .iter()
                    .all(|c| check_condition(c, fleet).satisfied)
            });
            ConditionResult {
                condition: desc.clone(),
                satisfied: any_satisfied,
                current_value: if any_satisfied {
                    "OK".into()
                } else {
                    "NG".into()
                },
                required_value: desc.clone(),
            }
        }
    }
}

// =============================================================================
// Main check function
// =============================================================================

pub fn check_sortie_quest(
    quest_id_str: &str,
    fleet: &FleetCheckData,
) -> SortieQuestCheckResult {
    let all = get_all_sortie_quests();
    let quest = match all.iter().find(|q| q.quest_id == quest_id_str) {
        Some(q) => q,
        None => {
            return SortieQuestCheckResult {
                quest_id: quest_id_str.to_string(),
                quest_name: format!("Unknown({})", quest_id_str),
                area: "?".into(),
                rank: "?".into(),
                boss_only: false,
                count: 0,
                no_conditions: false,
                note: None,
                satisfied: false,
                conditions: vec![ConditionResult {
                    condition: "任務データ".into(),
                    satisfied: false,
                    current_value: "不明".into(),
                    required_value: "有効な任務ID".into(),
                }],
                recommended: vec![],
            };
        }
    };

    let conditions: Vec<ConditionResult> = quest
        .conditions
        .iter()
        .map(|c| check_condition(c, fleet))
        .collect();
    // Satisfied when: confirmed no conditions, or conditions exist and all are met
    let satisfied =
        (quest.no_conditions || !conditions.is_empty()) && conditions.iter().all(|c| c.satisfied);

    let recommended: Vec<MapRecommendedResult> = quest
        .recommended
        .iter()
        .map(|rec| {
            let conds: Vec<ConditionResult> = rec
                .fleet
                .iter()
                .map(|c| check_condition(c, fleet))
                .collect();
            let sat = !conds.is_empty() && conds.iter().all(|c| c.satisfied);
            MapRecommendedResult {
                area: rec.area.clone(),
                satisfied: sat,
                conditions: conds,
            }
        })
        .collect();

    SortieQuestCheckResult {
        quest_id: quest.quest_id.clone(),
        quest_name: quest.name.clone(),
        area: quest.area.clone(),
        rank: quest.rank.clone(),
        boss_only: quest.boss_only,
        count: quest.count,
        no_conditions: quest.no_conditions,
        note: quest.note.clone(),
        satisfied,
        conditions,
        recommended,
    }
}

// =============================================================================
// Map recommendation functions
// =============================================================================

const MAP_RECOMMENDATIONS_JSON: &str = include_str!("../../data/map_recommendations.json");

static MAP_RECOMMENDATIONS_DATA: std::sync::OnceLock<Vec<MapRecommendationDef>> = std::sync::OnceLock::new();

/// Load all map recommendation definitions from the embedded JSON (cached after first call).
pub fn get_all_map_recommendations() -> Vec<MapRecommendationDef> {
    MAP_RECOMMENDATIONS_DATA.get_or_init(|| {
        serde_json::from_str(MAP_RECOMMENDATIONS_JSON).expect("Failed to parse map_recommendations.json")
    }).clone()
}

/// Check the current fleet against all routes for a specific map area.
pub fn check_map_recommendation(
    area: &str,
    fleet: &FleetCheckData,
) -> MapRecommendationCheckResult {
    let all = get_all_map_recommendations();
    let def = match all.iter().find(|d| d.area == area) {
        Some(d) => d,
        None => {
            return MapRecommendationCheckResult {
                area: area.to_string(),
                name: format!("Unknown({})", area),
                routes: vec![],
            };
        }
    };

    let routes: Vec<MapRouteCheckResult> = def
        .routes
        .iter()
        .map(|route| {
            let conditions: Vec<ConditionResult> = route
                .fleet
                .iter()
                .map(|c| check_condition(c, fleet))
                .collect();
            let satisfied = !conditions.is_empty() && conditions.iter().all(|c| c.satisfied);
            MapRouteCheckResult {
                desc: route.desc.clone(),
                satisfied,
                conditions,
            }
        })
        .collect();

    MapRecommendationCheckResult {
        area: def.area.clone(),
        name: def.name.clone(),
        routes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_loads() {
        let quests = get_all_sortie_quests();
        assert!(quests.len() > 200, "Expected 200+ quests, got {}", quests.len());
        let bm1 = quests.iter().find(|q| q.quest_id == "Bm1").unwrap();
        assert_eq!(bm1.name, "「第五戦隊」出撃せよ！");
        assert_eq!(bm1.area, "2-5");
        assert_eq!(bm1.reset, "monthly");

        // Check all reset types exist
        let daily = quests.iter().filter(|q| q.reset == "daily").count();
        let weekly = quests.iter().filter(|q| q.reset == "weekly").count();
        let monthly = quests.iter().filter(|q| q.reset == "monthly").count();
        let quarterly = quests.iter().filter(|q| q.reset == "quarterly").count();
        let yearly = quests.iter().filter(|q| q.reset == "yearly").count();
        let once = quests.iter().filter(|q| q.reset == "once").count();
        assert!(daily >= 5, "daily: {}", daily);
        assert!(weekly >= 8, "weekly: {}", weekly);
        assert!(monthly >= 6, "monthly: {}", monthly);
        assert!(quarterly >= 10, "quarterly: {}", quarterly);
        assert!(yearly >= 10, "yearly: {}", yearly);
        assert!(once >= 100, "once: {}", once);
    }

    #[test]
    fn test_ship_name_match() {
        let fleet = FleetCheckData {
            ships: vec![
                FleetShipData {
                    name: "那智改二".into(),
                    ship_type: 5,
                    level: 80,
                },
                FleetShipData {
                    name: "妙高改二".into(),
                    ship_type: 5,
                    level: 75,
                },
                FleetShipData {
                    name: "羽黒改二".into(),
                    ship_type: 5,
                    level: 70,
                },
                FleetShipData {
                    name: "島風".into(),
                    ship_type: 2,
                    level: 60,
                },
            ],
        };
        let result = check_sortie_quest("Bm1", &fleet);
        assert!(result.satisfied);
    }

    #[test]
    fn test_bm4_battleship_condition() {
        // Bm4: 大和型/長門型/伊勢型/扶桑型 3隻 + 軽巡1, other BBs prohibited
        let valid_fleet = FleetCheckData {
            ships: vec![
                FleetShipData { name: "大和改二重".into(), ship_type: 9, level: 99 },
                FleetShipData { name: "長門改二".into(), ship_type: 9, level: 90 },
                FleetShipData { name: "扶桑改二".into(), ship_type: 10, level: 85 },
                FleetShipData { name: "阿武隈改二".into(), ship_type: 3, level: 75 },
                FleetShipData { name: "島風".into(), ship_type: 2, level: 60 },
                FleetShipData { name: "雪風改二".into(), ship_type: 2, level: 70 },
            ],
        };
        let result = check_sortie_quest("Bm4", &valid_fleet);
        assert!(result.satisfied, "Valid Bm4 fleet should pass");

        // Invalid: 金剛型 (stype 8) should NOT count
        let invalid_fleet = FleetCheckData {
            ships: vec![
                FleetShipData { name: "金剛改二丙".into(), ship_type: 8, level: 99 },
                FleetShipData { name: "榛名改二".into(), ship_type: 8, level: 90 },
                FleetShipData { name: "霧島改二".into(), ship_type: 8, level: 85 },
                FleetShipData { name: "阿武隈改二".into(), ship_type: 3, level: 75 },
                FleetShipData { name: "島風".into(), ship_type: 2, level: 60 },
                FleetShipData { name: "雪風改二".into(), ship_type: 2, level: 70 },
            ],
        };
        let result = check_sortie_quest("Bm4", &invalid_fleet);
        assert!(!result.satisfied, "Kongou-class fleet should NOT pass Bm4");

        // Invalid: 4 BBs (exceeds MaxShipTypeCount of 3)
        let too_many_bbs = FleetCheckData {
            ships: vec![
                FleetShipData { name: "大和改二重".into(), ship_type: 9, level: 99 },
                FleetShipData { name: "武蔵改二".into(), ship_type: 9, level: 99 },
                FleetShipData { name: "長門改二".into(), ship_type: 9, level: 90 },
                FleetShipData { name: "金剛改二丙".into(), ship_type: 8, level: 85 },
                FleetShipData { name: "阿武隈改二".into(), ship_type: 3, level: 75 },
                FleetShipData { name: "島風".into(), ship_type: 2, level: 60 },
            ],
        };
        let result = check_sortie_quest("Bm4", &too_many_bbs);
        assert!(!result.satisfied, "4 BBs should NOT pass Bm4 (max 3)");
    }

    #[test]
    fn test_bq13_or_conditions() {
        // Bq13: 旗艦夕張改二 + (六水戦DD×2 OR 由良改二)

        // Option A: 夕張改二 + 睦月 + 如月
        let option_a = FleetCheckData {
            ships: vec![
                FleetShipData { name: "夕張改二特".into(), ship_type: 3, level: 90 },
                FleetShipData { name: "睦月改二".into(), ship_type: 2, level: 70 },
                FleetShipData { name: "如月改二".into(), ship_type: 2, level: 70 },
                FleetShipData { name: "島風".into(), ship_type: 2, level: 60 },
                FleetShipData { name: "雪風改二".into(), ship_type: 2, level: 70 },
                FleetShipData { name: "時雨改三".into(), ship_type: 2, level: 80 },
            ],
        };
        let result = check_sortie_quest("Bq13", &option_a);
        assert!(result.satisfied, "Bq13 Option A (六水戦DD) should pass");

        // Option B: 夕張改二 + 由良改二
        let option_b = FleetCheckData {
            ships: vec![
                FleetShipData { name: "夕張改二".into(), ship_type: 3, level: 90 },
                FleetShipData { name: "由良改二".into(), ship_type: 3, level: 80 },
                FleetShipData { name: "島風".into(), ship_type: 2, level: 60 },
                FleetShipData { name: "雪風改二".into(), ship_type: 2, level: 70 },
                FleetShipData { name: "時雨改三".into(), ship_type: 2, level: 80 },
                FleetShipData { name: "秋月改".into(), ship_type: 2, level: 75 },
            ],
        };
        let result = check_sortie_quest("Bq13", &option_b);
        assert!(result.satisfied, "Bq13 Option B (由良改二) should pass");

        // Invalid: 夕張改二 but only random DDs (no 六水戦DD, no 由良改二)
        let invalid = FleetCheckData {
            ships: vec![
                FleetShipData { name: "夕張改二丁".into(), ship_type: 3, level: 90 },
                FleetShipData { name: "島風".into(), ship_type: 2, level: 60 },
                FleetShipData { name: "雪風改二".into(), ship_type: 2, level: 70 },
                FleetShipData { name: "時雨改三".into(), ship_type: 2, level: 80 },
                FleetShipData { name: "秋月改".into(), ship_type: 2, level: 75 },
                FleetShipData { name: "涼月改".into(), ship_type: 2, level: 70 },
            ],
        };
        let result = check_sortie_quest("Bq13", &invalid);
        assert!(!result.satisfied, "Bq13 with random DDs should NOT pass");
    }

    #[test]
    fn test_bq2_sub_goals() {
        let quests = get_all_sortie_quests();
        let bq2 = quests.iter().find(|q| q.quest_id == "Bq2").unwrap();
        assert_eq!(bq2.sub_goals.len(), 4, "Bq2 should have 4 sub_goals");
        // 6-4 requires S rank
        let sg_64 = bq2.sub_goals.iter().find(|sg| sg.name == "6-4").unwrap();
        assert_eq!(sg_64.rank, "S");
        assert_eq!(sg_64.area.as_deref(), Some("6-4"));
        // Others require A rank
        let sg_24 = bq2.sub_goals.iter().find(|sg| sg.name == "2-4").unwrap();
        assert_eq!(sg_24.rank, "A");
    }

    #[test]
    fn test_c23_c27_once() {
        let quests = get_all_sortie_quests();
        let c23 = quests.iter().find(|q| q.quest_id == "C23").unwrap();
        assert_eq!(c23.reset, "once", "C23 should be a one-time quest");
        let c27 = quests.iter().find(|q| q.quest_id == "C27").unwrap();
        assert_eq!(c27.reset, "once", "C27 should be a one-time quest");
    }

    #[test]
    fn test_exercise_counter_reset() {
        let quests = get_all_sortie_quests();
        let ids = ["Cm1", "Cq1", "Cq2", "Cq3", "Cq4"];
        for id in ids {
            let q = quests.iter().find(|q| q.quest_id == id).unwrap();
            assert_eq!(
                q.counter_reset.as_deref(),
                Some("daily"),
                "{} should have counter_reset=daily",
                id
            );
        }
    }

    #[test]
    fn test_existing_conditions_unchanged() {
        let quests = get_all_sortie_quests();

        // Bm1: 那智+妙高+羽黒 should still work
        let bm1 = quests.iter().find(|q| q.quest_id == "Bm1").unwrap();
        assert_eq!(bm1.conditions.len(), 1);
        assert_eq!(bm1.area, "2-5");
        assert_eq!(bm1.rank, "S");

        // Bm3: 旗艦軽巡 + 軽巡駆逐のみ
        let bm3 = quests.iter().find(|q| q.quest_id == "Bm3").unwrap();
        assert_eq!(bm3.conditions.len(), 2);

        // Bm6: 空母2 + 駆逐2
        let bm6 = quests.iter().find(|q| q.quest_id == "Bm6").unwrap();
        assert_eq!(bm6.conditions.len(), 2);

        // Bm7: 旗艦駆逐 + 重巡1 + 軽巡1 + 駆逐4
        let bm7 = quests.iter().find(|q| q.quest_id == "Bm7").unwrap();
        assert_eq!(bm7.conditions.len(), 4);

        // Bq6: 長波改二 + 高波改/沖波改/朝霜改
        let bq6 = quests.iter().find(|q| q.quest_id == "Bq6").unwrap();
        assert_eq!(bq6.conditions.len(), 2);

        // Bq7: 三川艦隊 4隻
        let bq7 = quests.iter().find(|q| q.quest_id == "Bq7").unwrap();
        assert_eq!(bq7.conditions.len(), 1);
    }

    #[test]
    fn test_map_recommendations_json_loads() {
        let recs = get_all_map_recommendations();
        assert!(recs.len() >= 20, "Expected 20+ maps, got {}", recs.len());
        let map_1_1 = recs.iter().find(|r| r.area == "1-1").unwrap();
        assert_eq!(map_1_1.name, "鎮守府正面海域");
        assert!(!map_1_1.routes.is_empty());
    }

    #[test]
    fn test_map_recommendation_check() {
        // Fleet matching 2-5 second route: 6 ships, 3DD, 1CL, no BB/CV
        let fleet = FleetCheckData {
            ships: vec![
                FleetShipData { name: "那智改二".into(), ship_type: 5, level: 80 },
                FleetShipData { name: "妙高改二".into(), ship_type: 5, level: 75 },
                FleetShipData { name: "神通改二".into(), ship_type: 3, level: 70 },
                FleetShipData { name: "島風".into(), ship_type: 2, level: 60 },
                FleetShipData { name: "雪風".into(), ship_type: 2, level: 65 },
                FleetShipData { name: "時雨改二".into(), ship_type: 2, level: 70 },
            ],
        };
        let result = check_map_recommendation("2-5", &fleet);
        assert_eq!(result.area, "2-5");
        assert_eq!(result.name, "沖ノ島沖");
        assert!(result.routes.len() >= 2);
        // Second route (水上): 3DD + 1CL, no BB/CV -> satisfied
        assert!(result.routes[1].satisfied);
    }
}
