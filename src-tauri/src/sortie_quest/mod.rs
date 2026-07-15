use serde::{Deserialize, Serialize};

// =============================================================================
// Data structures (JSON-deserializable)
// =============================================================================

/// A single condition from the JSON file
#[derive(Debug, Clone, Deserialize, Serialize)]
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
    FlagshipType { ship_type: String, stypes: Vec<i32> },
    /// Fleet must contain `count` ships whose name starts with one of `names`
    ContainsShipName { names: Vec<String>, count: i32 },
    /// Fleet must contain `count` ships whose name starts with ANY of `names` (OR match)
    ContainsShipNameAny { names: Vec<String>, count: i32 },
    /// Fleet can only contain ships of these stypes
    OnlyShipTypes { desc: String, stypes: Vec<i32> },
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
    pub conditions: Vec<SortieQuestCondition>,
    #[serde(default, skip_serializing)]
    pub recommended: Vec<MapRecommendation>,
}

/// Input data for one ship in the fleet being checked
#[derive(Debug, Clone)]
pub struct FleetShipData {
    pub name: String,
    pub ship_type: i32,
    /// Part of the caller-facing input contract; no current condition reads it.
    #[allow(dead_code)]
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
    SORTIE_QUESTS_DATA
        .get_or_init(|| {
            serde_json::from_str(SORTIE_QUESTS_JSON).expect("Failed to parse sortie_quests.json")
        })
        .clone()
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
            let any_satisfied = alternatives
                .iter()
                .any(|group| group.iter().all(|c| check_condition(c, fleet).satisfied));
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

pub fn check_sortie_quest(quest_id_str: &str, fleet: &FleetCheckData) -> SortieQuestCheckResult {
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

static MAP_RECOMMENDATIONS_DATA: std::sync::OnceLock<Vec<MapRecommendationDef>> =
    std::sync::OnceLock::new();

/// Load all map recommendation definitions from the embedded JSON (cached after first call).
pub fn get_all_map_recommendations() -> Vec<MapRecommendationDef> {
    MAP_RECOMMENDATIONS_DATA
        .get_or_init(|| {
            serde_json::from_str(MAP_RECOMMENDATIONS_JSON)
                .expect("Failed to parse map_recommendations.json")
        })
        .clone()
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
mod tests;
