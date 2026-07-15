use super::{jst, now_jst, QuestProgressState};
use crate::sortie_quest::SortieQuestDef;
use chrono::{DateTime, Datelike, FixedOffset, TimeZone};
use log::{error, info, warn};
use std::collections::HashMap;
use std::path::Path;

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
            let q_year = if q_month == 12 && m <= 3 {
                boundary.year() - 1
            } else {
                boundary.year()
            };
            jst().with_ymd_and_hms(q_year, q_month, 1, 5, 0, 0).single()
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
