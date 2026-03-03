use chrono::{DateTime, Datelike, FixedOffset, TimeZone, Utc};
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::sortie_quest::SortieQuestDef;

// JST offset: +09:00
const JST_OFFSET: i32 = 9 * 3600;

fn jst() -> FixedOffset {
    FixedOffset::east_opt(JST_OFFSET).unwrap()
}

fn now_jst() -> DateTime<FixedOffset> {
    Utc::now().with_timezone(&jst())
}

// =============================================================================
// Data structures
// =============================================================================

/// Progress entry for a single quest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestProgressEntry {
    /// Game API quest ID (e.g. 226)
    pub quest_id: i32,
    /// Quest string ID (e.g. "Bd7")
    pub quest_id_str: String,
    /// LEGACY: per-area cleared state (kept for deserialization of old data)
    #[serde(default)]
    pub area_cleared: HashMap<String, bool>,
    /// Per-area clear counts for multi-area quests (area -> current count)
    #[serde(default)]
    pub area_counts: HashMap<String, i32>,
    /// Counter for single-area/exercise quests
    #[serde(default)]
    pub count: i32,
    /// Max count needed (for counter: total count, for area: per-area target)
    #[serde(default)]
    pub count_max: i32,
    /// Whether this quest is considered completed
    #[serde(default)]
    pub completed: bool,
    /// Last time this entry was updated
    pub last_updated: DateTime<FixedOffset>,
}

/// Full quest progress state
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QuestProgressState {
    /// quest_id (API) -> progress entry
    pub quests: HashMap<i32, QuestProgressEntry>,
    /// Last time global reset check was performed
    #[serde(default)]
    pub last_reset_check: Option<DateTime<FixedOffset>>,
}

/// Summary sent to frontend
#[derive(Debug, Clone, Serialize)]
pub struct QuestProgressSummary {
    pub quest_id: i32,
    pub quest_id_str: String,
    pub area_progress: Vec<AreaProgress>,
    pub count: i32,
    pub count_max: i32,
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AreaProgress {
    pub area: String,
    pub cleared: bool,
    pub count: i32,
    pub count_max: i32,
}

// =============================================================================
// Persistence
// =============================================================================

pub fn load_progress(path: &Path) -> QuestProgressState {
    match std::fs::read_to_string(path) {
        Ok(content) => match serde_json::from_str(&content) {
            Ok(state) => {
                info!("Loaded quest progress from {}", path.display());
                state
            }
            Err(e) => {
                warn!("Failed to parse quest progress: {}", e);
                QuestProgressState::default()
            }
        },
        Err(_) => {
            info!("No quest progress file found, starting fresh");
            QuestProgressState::default()
        }
    }
}

pub fn save_progress(path: &Path, state: &QuestProgressState) {
    if let Some(parent) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            error!("Failed to create quest progress dir: {}", e);
            return;
        }
    }
    match serde_json::to_string_pretty(state) {
        Ok(json) => {
            if let Err(e) = std::fs::write(path, json) {
                error!("Failed to save quest progress: {}", e);
            }
        }
        Err(e) => {
            error!("Failed to serialize quest progress: {}", e);
        }
    }
}

// =============================================================================
// Reset logic (JST 05:00 based)
// =============================================================================

/// Get the last reset boundary time for a given reset type.
/// Returns None if the type doesn't reset (once/limited).
fn last_reset_time(reset: &str, now: DateTime<FixedOffset>) -> Option<DateTime<FixedOffset>> {
    let today_5am = jst()
        .with_ymd_and_hms(now.year(), now.month(), now.day(), 5, 0, 0)
        .single()?;
    let boundary = if now < today_5am {
        today_5am - chrono::Duration::days(1)
    } else {
        today_5am
    };

    match reset {
        "daily" => Some(boundary),
        "weekly" => {
            // Monday 05:00 JST
            let weekday = boundary.weekday();
            let days_since_monday = weekday.num_days_from_monday() as i64;
            Some(boundary - chrono::Duration::days(days_since_monday))
        }
        "monthly" => {
            // 1st of month 05:00 JST
            jst()
                .with_ymd_and_hms(boundary.year(), boundary.month(), 1, 5, 0, 0)
                .single()
        }
        "quarterly" => {
            // 3/6/9/12 month 1st 05:00 JST
            let m = boundary.month();
            let q_month = match m {
                1..=3 => {
                    // Q4 of prev year (Dec) or Q1 (Mar)
                    if m < 3 || (m == 3 && boundary.day() == 1 && now < today_5am) {
                        12 // Previous year December
                    } else {
                        3
                    }
                }
                4..=6 => {
                    if m < 6 || (m == 6 && boundary.day() == 1 && now < today_5am) {
                        3
                    } else {
                        6
                    }
                }
                7..=9 => {
                    if m < 9 || (m == 9 && boundary.day() == 1 && now < today_5am) {
                        6
                    } else {
                        9
                    }
                }
                10..=12 => {
                    if m < 12 || (m == 12 && boundary.day() == 1 && now < today_5am) {
                        9
                    } else {
                        12
                    }
                }
                _ => 3,
            };
            let q_year = if q_month == 12 && m <= 2 {
                boundary.year() - 1
            } else {
                boundary.year()
            };
            jst()
                .with_ymd_and_hms(q_year, q_month, 1, 5, 0, 0)
                .single()
        }
        "yearly" => {
            // Simplified: April 1st 05:00 JST
            let y = boundary.year();
            let april = jst().with_ymd_and_hms(y, 4, 1, 5, 0, 0).single();
            if let Some(apr) = april {
                if boundary < apr {
                    // Before April this year -> use last year's April
                    jst().with_ymd_and_hms(y - 1, 4, 1, 5, 0, 0).single()
                } else {
                    Some(apr)
                }
            } else {
                None
            }
        }
        _ => None, // "once", "limited" - no reset
    }
}

/// Check and perform resets for all tracked quests.
pub fn check_resets(state: &mut QuestProgressState, quest_defs: &[SortieQuestDef], path: &Path) {
    let now = now_jst();
    let mut changed = false;

    // Build quest def lookup
    let def_by_id: HashMap<i32, &SortieQuestDef> = quest_defs.iter().map(|d| (d.id, d)).collect();

    let quest_ids: Vec<i32> = state.quests.keys().copied().collect();
    for quest_id in quest_ids {
        let quest_def = def_by_id.get(&quest_id);
        let reset_type = quest_def.map(|d| d.reset.as_str()).unwrap_or("once");
        let counter_reset = quest_def.and_then(|d| d.counter_reset.as_deref());

        if let Some(entry) = state.quests.get_mut(&quest_id) {
            // Primary reset: clear everything including completed status
            if let Some(reset_boundary) = last_reset_time(reset_type, now) {
                if entry.last_updated < reset_boundary {
                    info!(
                        "Resetting quest progress for {} ({}) - last_updated={}, boundary={}",
                        entry.quest_id_str, reset_type, entry.last_updated, reset_boundary
                    );
                    entry.count = 0;
                    entry.area_cleared.clear();
                    entry.area_counts.clear();
                    entry.completed = false;
                    entry.last_updated = now;
                    changed = true;
                    continue;
                }
            }

            // Counter reset: only reset progress counters if not yet completed
            if let Some(cr) = counter_reset {
                if !entry.completed {
                    if let Some(cr_boundary) = last_reset_time(cr, now) {
                        if entry.last_updated < cr_boundary {
                            info!(
                                "Counter-resetting quest progress for {} ({}) - last_updated={}, boundary={}",
                                entry.quest_id_str, cr, entry.last_updated, cr_boundary
                            );
                            entry.count = 0;
                            entry.area_counts.clear();
                            entry.last_updated = now;
                            changed = true;
                        }
                    }
                }
            }
        }
    }

    state.last_reset_check = Some(now);

    if changed {
        save_progress(path, state);
    }
}

// =============================================================================
// Battle/Exercise result processing
// =============================================================================

/// Check if an enemy stype matches the quest's enemy_type
fn match_enemy_type(enemy_type: &str, stype: i32) -> bool {
    match enemy_type {
        "carrier" => matches!(stype, 7 | 11 | 18),   // CVL, CV, CVB
        "transport" => stype == 15,                    // AP (補給艦)
        "submarine" => matches!(stype, 13 | 14),      // SS, SSV
        _ => false,
    }
}

/// Rank to numeric value for comparison
fn rank_value(rank: &str) -> i32 {
    match rank {
        "S" => 5,
        "A" => 4,
        "B" => 3,
        "C" => 2,
        "D" => 1,
        "E" => 0,
        _ => -1,
    }
}

/// Check if a map area string matches any of the given quest areas.
/// Handles gauge suffixes: "7-2(2nd)" matches "7-2(2nd)" exactly, or "7-2" as base area.
fn area_matches(map_area_str: &str, quest_areas: &[&str]) -> bool {
    if quest_areas.contains(&map_area_str) {
        return true;
    }
    // Also check base area (without gauge suffix) for quests that accept any gauge
    let base_area = map_area_str.split('(').next().unwrap_or(map_area_str);
    if base_area != map_area_str {
        return quest_areas.contains(&base_area);
    }
    false
}

/// Check if a battle result matches a quest's requirements
fn does_battle_match(
    quest: &SortieQuestDef,
    map_area_str: &str,
    rank: &str,
    is_boss: bool,
) -> bool {
    // Check boss requirement
    if quest.boss_only && !is_boss {
        return false;
    }

    // Check rank requirement
    if !quest.rank.is_empty() {
        let required = rank_value(&quest.rank);
        let actual = rank_value(rank);
        if actual < required {
            return false;
        }
    }

    // Check area match
    let area = &quest.area;
    if area == "任意" {
        return true;
    }
    if area == "演習" {
        return false; // Handled by exercise path
    }

    let areas: Vec<&str> = area.split('/').collect();
    area_matches(map_area_str, &areas)
}

/// Determine the progress pattern for a quest
/// Returns: "sub_goals" for multi-condition quests, "area" for specific area(s), "counter" for 任意/演習
fn quest_pattern(quest: &SortieQuestDef) -> &'static str {
    if !quest.sub_goals.is_empty() {
        "sub_goals"
    } else {
        let area = &quest.area;
        if area == "任意" || area == "演習" {
            "counter"
        } else {
            "area" // Single or multi-area: per-area count tracking
        }
    }
}

/// Ensure a progress entry exists for a quest, creating one if needed.
/// Also migrates old data (area_cleared -> area_counts) and updates count_max.
fn ensure_entry<'a>(
    state: &'a mut QuestProgressState,
    quest: &SortieQuestDef,
) -> &'a mut QuestProgressEntry {
    let entry = state.quests.entry(quest.id).or_insert_with(|| {
        let pattern = quest_pattern(quest);
        let mut area_counts = HashMap::new();
        if pattern == "area" {
            for a in quest.area.split('/') {
                area_counts.insert(a.to_string(), 0);
            }
        } else if pattern == "sub_goals" {
            for sg in &quest.sub_goals {
                area_counts.insert(sg.name.clone(), 0);
            }
        }
        QuestProgressEntry {
            quest_id: quest.id,
            quest_id_str: quest.quest_id.clone(),
            area_cleared: HashMap::new(),
            area_counts,
            count: 0,
            count_max: quest.count,
            completed: false,
            last_updated: now_jst(),
        }
    });

    // Migrate: populate area_counts from old area_cleared if needed
    let pattern = quest_pattern(quest);
    if pattern == "area" && entry.area_counts.is_empty() {
        for a in quest.area.split('/') {
            let old_val = entry.area_cleared.get(a).copied().unwrap_or(false);
            entry.area_counts.insert(
                a.to_string(),
                if old_val { quest.count.min(1) } else { 0 },
            );
        }
    }
    // Migrate: ensure sub_goals keys exist in area_counts
    if pattern == "sub_goals" {
        for sg in &quest.sub_goals {
            entry.area_counts.entry(sg.name.clone()).or_insert(0);
        }
    }
    // Always sync count_max with quest definition
    entry.count_max = quest.count;

    entry
}

/// Process a sortie battle result
pub fn on_battle_result(
    state: &mut QuestProgressState,
    map_area_str: &str,
    rank: &str,
    is_boss: bool,
    sunk_enemy_stypes: &[i32],
    active_quests: &std::collections::HashSet<i32>,
    quest_defs: &[SortieQuestDef],
    path: &Path,
) -> bool {
    let mut changed = false;
    let now = now_jst();

    // Build def lookup
    let def_by_id: HashMap<i32, &SortieQuestDef> = quest_defs.iter().map(|d| (d.id, d)).collect();

    for &quest_id in active_quests {
        let quest = match def_by_id.get(&quest_id) {
            Some(q) => *q,
            None => continue,
        };

        // Skip exercise quests
        if quest.area == "演習" {
            continue;
        }

        let pattern = quest_pattern(quest);
        let quest_quest_id = quest.quest_id.clone();

        // sub_goals pattern handles matching per sub-goal; others use does_battle_match
        if pattern != "sub_goals" && !does_battle_match(quest, map_area_str, rank, is_boss) {
            continue;
        }

        let entry = ensure_entry(state, quest);
        if entry.completed {
            continue;
        }

        match pattern {
            "sub_goals" => {
                // Each sub-goal is checked independently
                let actual_rank = rank_value(rank);
                for sg in &quest.sub_goals {
                    // Check area filter if specified
                    if let Some(ref sg_area) = sg.area {
                        let sg_areas: Vec<&str> = vec![sg_area.as_str()];
                        if !area_matches(map_area_str, &sg_areas) {
                            continue;
                        }
                    }
                    // Check boss_only
                    if sg.boss_only && !is_boss {
                        continue;
                    }
                    // Check rank
                    if !sg.rank.is_empty() {
                        let required = rank_value(&sg.rank);
                        if actual_rank < required {
                            continue;
                        }
                    }
                    // Increment this sub-goal
                    if let Some(ac) = entry.area_counts.get_mut(&sg.name) {
                        if *ac < sg.count {
                            *ac += 1;
                            entry.last_updated = now;
                            changed = true;
                            info!(
                                "Quest {} sub-goal {} progress: {}/{}",
                                quest_quest_id, sg.name, ac, sg.count
                            );
                        }
                    }
                }
                // Check if all sub-goals are met
                let all_met = quest.sub_goals.iter().all(|sg| {
                    entry.area_counts.get(&sg.name).copied().unwrap_or(0) >= sg.count
                });
                if all_met {
                    entry.completed = true;
                    info!("Quest {} completed (all sub-goals)", quest_quest_id);
                }
            }
            "area" => {
                // Increment per-area count
                // Try exact match first, then fall back to base area (without gauge suffix)
                // e.g., map_area_str="7-2(2nd)" matches area_counts key "7-2(2nd)" or "7-2"
                let area_key = if entry.area_counts.contains_key(map_area_str) {
                    Some(map_area_str.to_string())
                } else {
                    let base = map_area_str.split('(').next().unwrap_or(map_area_str);
                    if base != map_area_str && entry.area_counts.contains_key(base) {
                        Some(base.to_string())
                    } else {
                        None
                    }
                };
                if let Some(ref key) = area_key {
                    if let Some(ac) = entry.area_counts.get_mut(key.as_str()) {
                        if *ac < entry.count_max {
                            *ac += 1;
                            entry.last_updated = now;
                            changed = true;
                            info!(
                                "Quest {} area {} progress: {}/{}",
                                quest_quest_id, key, ac, entry.count_max
                            );
                        }
                    }
                }
                // Check if all areas reached target
                if entry.area_counts.values().all(|&v| v >= entry.count_max) {
                    entry.completed = true;
                    info!("Quest {} completed (all areas)", quest_quest_id);
                }
            }
            _ => {
                // Counter pattern
                let increment = if let Some(ref enemy_type) = quest.enemy_type {
                    // Count matching sunk enemy ships
                    sunk_enemy_stypes
                        .iter()
                        .filter(|&&stype| match_enemy_type(enemy_type, stype))
                        .count() as i32
                } else {
                    // No enemy_type: count battles
                    1
                };
                if increment > 0 {
                    entry.count = (entry.count + increment).min(entry.count_max);
                    entry.last_updated = now;
                    changed = true;
                    if entry.count >= entry.count_max {
                        entry.completed = true;
                        info!("Quest {} completed ({}/{})", quest_quest_id, entry.count, entry.count_max);
                    } else {
                        info!("Quest {} progress: {}/{}", quest_quest_id, entry.count, entry.count_max);
                    }
                }
            }
        }
    }

    if changed {
        save_progress(path, state);
    }
    changed
}

/// Process an exercise battle result
pub fn on_exercise_result(
    state: &mut QuestProgressState,
    rank: &str,
    active_quests: &std::collections::HashSet<i32>,
    quest_defs: &[SortieQuestDef],
    path: &Path,
) -> bool {
    let mut changed = false;
    let now = now_jst();

    let def_by_id: HashMap<i32, &SortieQuestDef> = quest_defs.iter().map(|d| (d.id, d)).collect();

    for &quest_id in active_quests {
        let quest = match def_by_id.get(&quest_id) {
            Some(q) => *q,
            None => continue,
        };

        // Only exercise quests
        if quest.area != "演習" {
            continue;
        }

        // Check rank
        if !quest.rank.is_empty() {
            let required = rank_value(&quest.rank);
            let actual = rank_value(rank);
            if actual < required {
                continue;
            }
        }

        let quest_quest_id = quest.quest_id.clone();
        let entry = ensure_entry(state, quest);
        if entry.completed {
            continue;
        }

        entry.count = (entry.count + 1).min(entry.count_max);
        entry.last_updated = now;
        changed = true;

        if entry.count >= entry.count_max {
            entry.completed = true;
            info!(
                "Exercise quest {} completed ({}/{})",
                quest_quest_id, entry.count, entry.count_max
            );
        } else {
            info!(
                "Exercise quest {} progress: {}/{}",
                quest_quest_id, entry.count, entry.count_max
            );
        }
    }

    if changed {
        save_progress(path, state);
    }
    changed
}

// =============================================================================
// Manual update
// =============================================================================

/// Manual update from frontend (toggle area checkbox or adjust count)
pub fn manual_update(
    state: &mut QuestProgressState,
    quest_id: i32,
    area: Option<String>,
    count: Option<i32>,
    quest_defs: &[SortieQuestDef],
    path: &Path,
) -> bool {
    let now = now_jst();

    // Get or create entry
    let quest = quest_defs.iter().find(|q| q.id == quest_id);
    if let Some(quest) = quest {
        let entry = ensure_entry(state, quest);

        let pattern = quest_pattern(quest);
        if let Some(area_key) = area {
            // Determine max for this specific key (sub_goals have per-key max)
            let key_max = if pattern == "sub_goals" {
                quest.sub_goals.iter().find(|sg| sg.name == area_key).map(|sg| sg.count).unwrap_or(entry.count_max)
            } else {
                entry.count_max
            };
            // Update per-area count
            if let Some(ac) = entry.area_counts.get_mut(&area_key) {
                if let Some(new_count) = count {
                    // Dropdown: set specific count
                    *ac = new_count.max(0).min(key_max);
                } else {
                    // Click: toggle (0 <-> max for max=1, else increment)
                    if key_max <= 1 {
                        *ac = if *ac > 0 { 0 } else { 1 };
                    } else {
                        *ac = if *ac >= key_max { 0 } else { *ac + 1 };
                    }
                }
                entry.last_updated = now;
                info!(
                    "Quest {} area {} manually set to {}/{}",
                    entry.quest_id_str, area_key, ac, key_max
                );
            }
            // Recheck completion
            if pattern == "sub_goals" {
                entry.completed = quest.sub_goals.iter().all(|sg| {
                    entry.area_counts.get(&sg.name).copied().unwrap_or(0) >= sg.count
                });
            } else {
                entry.completed = entry.area_counts.values().all(|&v| v >= entry.count_max);
            }
        } else if let Some(new_count) = count {
            // Set count
            entry.count = new_count.max(0).min(entry.count_max);
            entry.last_updated = now;
            entry.completed = entry.count >= entry.count_max;
            info!(
                "Quest {} count manually set to {}/{}",
                entry.quest_id_str, entry.count, entry.count_max
            );
        }

        save_progress(path, state);
        true
    } else {
        warn!("manual_update: quest {} not found in defs", quest_id);
        false
    }
}

// =============================================================================
// Frontend query
// =============================================================================

/// Get progress summaries for all active quests (migrates old data if needed)
pub fn get_active_progress(
    state: &mut QuestProgressState,
    active_quests: &std::collections::HashSet<i32>,
    quest_defs: &[SortieQuestDef],
    path: &Path,
) -> Vec<QuestProgressSummary> {
    let def_by_id: HashMap<i32, &SortieQuestDef> = quest_defs.iter().map(|d| (d.id, d)).collect();
    let mut result = Vec::new();
    let mut migrated = false;

    // Migrate old entries first
    let quest_ids: Vec<i32> = active_quests.iter().copied().collect();
    for &quest_id in &quest_ids {
        if let Some(quest) = def_by_id.get(&quest_id) {
            if quest.area.is_empty() {
                continue;
            }
            let pattern = quest_pattern(quest);
            if pattern == "area" {
                if let Some(entry) = state.quests.get_mut(&quest_id) {
                    if entry.area_counts.is_empty() {
                        for a in quest.area.split('/') {
                            let old_val = entry.area_cleared.get(a).copied().unwrap_or(false);
                            entry.area_counts.insert(
                                a.to_string(),
                                if old_val { quest.count.min(1) } else { 0 },
                            );
                        }
                        migrated = true;
                    }
                    if entry.count_max != quest.count {
                        entry.count_max = quest.count;
                        // Recheck completion with updated count_max
                        entry.completed = entry.area_counts.values().all(|&v| v >= quest.count);
                        migrated = true;
                    }
                }
            }
        }
    }
    if migrated {
        save_progress(path, state);
    }

    for &quest_id in active_quests {
        let quest = match def_by_id.get(&quest_id) {
            Some(q) => *q,
            None => continue,
        };

        // Only sortie/exercise quests (not composition quests)
        if quest.area.is_empty() {
            continue;
        }

        let pattern = quest_pattern(quest);
        let per_area_target = quest.count;

        if let Some(entry) = state.quests.get(&quest_id) {
            let area_progress = if pattern == "sub_goals" {
                quest.sub_goals.iter().map(|sg| {
                    let ac = *entry.area_counts.get(&sg.name).unwrap_or(&0);
                    AreaProgress {
                        area: sg.name.clone(),
                        cleared: ac >= sg.count,
                        count: ac,
                        count_max: sg.count,
                    }
                }).collect()
            } else if pattern == "area" {
                quest
                    .area
                    .split('/')
                    .map(|a| {
                        let ac = *entry.area_counts.get(a).unwrap_or(&0);
                        AreaProgress {
                            area: a.to_string(),
                            cleared: ac >= per_area_target,
                            count: ac,
                            count_max: per_area_target,
                        }
                    })
                    .collect()
            } else {
                Vec::new()
            };

            result.push(QuestProgressSummary {
                quest_id,
                quest_id_str: entry.quest_id_str.clone(),
                area_progress,
                count: entry.count,
                count_max: entry.count_max,
                completed: entry.completed,
            });
        } else {
            // No progress entry yet - return zeroed summary
            let area_progress = if pattern == "sub_goals" {
                quest.sub_goals.iter().map(|sg| AreaProgress {
                    area: sg.name.clone(),
                    cleared: false,
                    count: 0,
                    count_max: sg.count,
                }).collect()
            } else if pattern == "area" {
                quest
                    .area
                    .split('/')
                    .map(|a| AreaProgress {
                        area: a.to_string(),
                        cleared: false,
                        count: 0,
                        count_max: per_area_target,
                    })
                    .collect()
            } else {
                Vec::new()
            };

            result.push(QuestProgressSummary {
                quest_id,
                quest_id_str: quest.quest_id.clone(),
                area_progress,
                count: 0,
                count_max: quest.count,
                completed: false,
            });
        }
    }

    result
}
