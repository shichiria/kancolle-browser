mod parser;
mod raw;
mod storage;

use chrono::{DateTime, Local};
use log::info;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::api::models::{PlayerSlotItem, ShipInfo};

pub(crate) use raw::save_to_disk as save_raw_api_to_disk;

// =============================================================================
// Data structures
// =============================================================================

/// HP state for a single ship (before/after battle, plus max)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HpState {
    pub before: i32,
    pub after: i32,
    pub max: i32,
}

/// Enemy ship info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnemyShip {
    /// Master ship ID
    pub ship_id: i32,
    /// Ship level
    pub level: i32,
    /// Ship name (from master data, if available)
    #[serde(default)]
    pub name: Option<String>,
    /// Enemy equipment IDs (master IDs)
    #[serde(default)]
    pub slots: Vec<i32>,
}

/// Air battle result from api_kouku.api_stage1/stage2
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AirBattleResult {
    /// Air superiority state (api_disp_seiku): 0=parity, 1=supremacy, 2=superiority, 3=denial, 4=incapability
    pub air_superiority: Option<i32>,
    /// Friendly plane count [total, lost]
    pub friendly_plane_count: Option<[i32; 2]>,
    /// Enemy plane count [total, lost]
    pub enemy_plane_count: Option<[i32; 2]>,
}

/// Land-base air defense result included in api_req_map/next.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseAirDefenseResult {
    /// Detection time of the air raid
    pub occurred_at: DateTime<Local>,
    /// Air superiority state (api_disp_seiku): 0=parity, 1=supremacy, 2=superiority, 3=denial, 4=incapability
    pub air_superiority: Option<i32>,
    /// Friendly plane count [total, lost]
    pub friendly_plane_count: Option<[i32; 2]>,
    /// Enemy plane count [total, lost]
    pub enemy_plane_count: Option<[i32; 2]>,
    /// Air-base damage category (api_lost_kind)
    pub lost_kind: Option<i32>,
}

/// Detailed battle information for a combat node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BattleDetail {
    /// Battle rank (S/A/B/C/D/E)
    pub rank: String,
    /// Enemy fleet name
    pub enemy_name: String,
    /// Enemy fleet composition
    pub enemy_ships: Vec<EnemyShip>,
    /// Formation [friendly, enemy, engagement_form]
    pub formation: [i32; 3],
    /// Air battle results
    #[serde(default)]
    pub air_battle: Option<AirBattleResult>,
    /// Friendly fleet HP states (before/after/max for each ship)
    pub friendly_hp: Vec<HpState>,
    /// Enemy fleet HP states (before/after/max for each ship)
    pub enemy_hp: Vec<HpState>,
    /// Dropped ship name (if any)
    pub drop_ship: Option<String>,
    /// Dropped ship ID (master)
    pub drop_ship_id: Option<i32>,
    /// MVP ship index (1-based)
    pub mvp: Option<i32>,
    /// Base experience gained
    pub base_exp: Option<i32>,
    /// Per-ship experience gained
    #[serde(default)]
    pub ship_exp: Vec<i32>,
    /// Whether night battle occurred (or was available)
    #[serde(default)]
    pub night_battle: bool,
    /// Raw battle API response JSON (for future analysis)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_battle: Option<serde_json::Value>,
    /// Raw battle result API response JSON
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_result: Option<serde_json::Value>,
}

impl BattleDetail {
    /// Copy without the bulky raw API payloads.
    /// Raw JSON stays in the on-disk battle log only.
    pub fn without_raw(&self) -> Self {
        Self {
            raw_battle: None,
            raw_result: None,
            ..self.clone()
        }
    }
}

/// A single battle node (cell) within a sortie
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BattleNode {
    /// Cell number on the map
    pub cell_no: i32,
    /// Event type (from api_color_no or api_event_id)
    pub event_kind: i32,
    /// Event ID from api_event_id (5 = boss node)
    #[serde(default)]
    pub event_id: i32,
    /// Battle detail (None if no combat at this cell)
    #[serde(default)]
    pub battle: Option<BattleDetail>,
    /// Land-base air defense result, when an air raid occurred while moving to this cell
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_air_defense: Option<BaseAirDefenseResult>,

    // --- Legacy fields for backward compatibility when loading old records ---
    // These are kept so that old saved JSON files can still be deserialized.
    // New records will always use the `battle` field instead.
    #[serde(default, skip_serializing)]
    pub rank: Option<String>,
    #[serde(default, skip_serializing)]
    pub enemy_name: Option<String>,
    #[serde(default, skip_serializing)]
    pub drop_ship: Option<String>,
    #[serde(default, skip_serializing)]
    pub drop_ship_id: Option<i32>,
    #[serde(default, skip_serializing)]
    pub mvp: Option<i32>,
    #[serde(default, skip_serializing)]
    pub base_exp: Option<i32>,
    #[serde(default, skip_serializing)]
    pub friendly_hp_before: Option<Vec<(i32, i32)>>,
    #[serde(default, skip_serializing)]
    pub friendly_hp_after: Option<Vec<(i32, i32)>>,
    #[serde(default, skip_serializing)]
    pub formation: Option<Vec<i32>>,
}

impl BattleNode {
    /// Copy for frontend transfer without the raw API payloads.
    pub fn without_raw(&self) -> Self {
        Self {
            battle: self.battle.as_ref().map(BattleDetail::without_raw),
            ..self.clone()
        }
    }

    /// Create a new empty node (no battle yet)
    fn new(cell_no: i32, event_kind: i32, event_id: i32) -> Self {
        Self {
            cell_no,
            event_kind,
            event_id,
            battle: None,
            base_air_defense: None,
            // Legacy fields - always None for new records
            rank: None,
            enemy_name: None,
            drop_ship: None,
            drop_ship_id: None,
            mvp: None,
            base_exp: None,
            friendly_hp_before: None,
            friendly_hp_after: None,
            formation: None,
        }
    }

    /// Migrate legacy data into BattleDetail if the `battle` field is None
    /// but legacy fields have data. Called after deserialization of old records.
    pub fn migrate_legacy(&mut self) {
        if self.battle.is_some() {
            return;
        }
        // Only migrate if there's actually battle data (rank is the key indicator)
        if let Some(rank) = self.rank.take() {
            let friendly_hp = match (&self.friendly_hp_before, &self.friendly_hp_after) {
                (Some(before), Some(after)) => before
                    .iter()
                    .zip(after.iter())
                    .map(|(&(bef_now, max), &(aft_now, _))| HpState {
                        before: bef_now,
                        after: aft_now,
                        max,
                    })
                    .collect(),
                _ => Vec::new(),
            };
            let formation = self
                .formation
                .as_ref()
                .map(|f| {
                    [
                        f.first().copied().unwrap_or(0),
                        f.get(1).copied().unwrap_or(0),
                        f.get(2).copied().unwrap_or(0),
                    ]
                })
                .unwrap_or([0, 0, 0]);

            self.battle = Some(BattleDetail {
                rank,
                enemy_name: self.enemy_name.take().unwrap_or_default(),
                enemy_ships: Vec::new(),
                formation,
                air_battle: None,
                friendly_hp,
                enemy_hp: Vec::new(),
                drop_ship: self.drop_ship.take(),
                drop_ship_id: self.drop_ship_id.take(),
                mvp: self.mvp.take(),
                base_exp: self.base_exp.take(),
                ship_exp: Vec::new(),
                night_battle: false,
                raw_battle: None,
                raw_result: None,
            });
        }
    }
}

/// Equipment snapshot for a single slot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotItemSnapshot {
    /// Master equipment ID
    pub id: i32,
    /// Improvement level (0-10, ★)
    #[serde(default, skip_serializing_if = "is_zero")]
    pub rf: i32,
    /// Aircraft proficiency (0-7, >>)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mas: Option<i32>,
}

fn is_zero(v: &i32) -> bool {
    *v == 0
}

/// Ship snapshot at sortie start
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortieShip {
    pub name: String,
    pub ship_id: i32,
    pub lv: i32,
    pub stype: i32,
    /// Equipment in each slot
    #[serde(default)]
    pub slots: Vec<SlotItemSnapshot>,
    /// Reinforcement expansion equipment (if any)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slot_ex: Option<SlotItemSnapshot>,
}

/// A complete sortie record (start to return)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortieRecord {
    /// Unique ID (timestamp-based)
    pub id: String,
    /// Fleet ID (1-based, from api_deck_id)
    pub fleet_id: i32,
    /// Map area ID (e.g., 1)
    pub map_area: i32,
    /// Map info number (e.g., 1 for 1-1)
    pub map_no: i32,
    /// Display string like "1-1"
    pub map_display: String,
    /// Ships in the fleet at sortie start
    pub ships: Vec<SortieShip>,
    /// Battle nodes visited
    pub nodes: Vec<BattleNode>,
    /// Sortie start time
    pub start_time: DateTime<Local>,
    /// Sortie end time (when port is reached)
    pub end_time: Option<DateTime<Local>>,
    /// Whether this is a combined fleet sortie
    pub is_combined: bool,
    /// Gauge number for multi-gauge maps (e.g., 7-2 has gauge 1 and 2)
    /// From api_eventmap.api_gauge_num in api_req_map/start response
    #[serde(default)]
    pub gauge_num: Option<i32>,
}

/// Summary sent to frontend
#[derive(Debug, Clone, Serialize)]
pub struct SortieRecordSummary {
    pub id: String,
    pub fleet_id: i32,
    pub map_area: i32,
    pub map_no: i32,
    pub map_display: String,
    pub gauge_num: Option<i32>,
    pub ships: Vec<SortieShip>,
    pub nodes: Vec<BattleNode>,
    pub start_time: String,
    pub end_time: Option<String>,
}

impl From<&SortieRecord> for SortieRecordSummary {
    fn from(r: &SortieRecord) -> Self {
        Self {
            id: r.id.clone(),
            fleet_id: r.fleet_id,
            map_area: r.map_area,
            map_no: r.map_no,
            map_display: r.map_display.clone(),
            gauge_num: r.gauge_num,
            ships: r.ships.clone(),
            nodes: r.nodes.iter().map(BattleNode::without_raw).collect(),
            start_time: r.start_time.format("%Y-%m-%d %H:%M:%S").to_string(),
            end_time: r
                .end_time
                .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string()),
        }
    }
}

// =============================================================================
// Temporary battle state accumulated during a battle sequence
// =============================================================================

/// Intermediate state accumulated from battle API calls before the result arrives
#[derive(Debug, Default)]
pub(super) struct PendingBattle {
    /// Friendly fleet HP before battle
    pub(super) friendly_hp_before: Vec<(i32, i32)>, // (now, max)
    /// Friendly fleet HP after all phases
    pub(super) friendly_hp_after: Vec<(i32, i32)>, // (now, max)
    /// Enemy fleet HP before battle
    pub(super) enemy_hp_before: Vec<(i32, i32)>, // (now, max)
    /// Enemy fleet HP after all phases
    pub(super) enemy_hp_after: Vec<(i32, i32)>, // (now, max)
    /// Formation [friendly, enemy, engagement]
    pub(super) formation: [i32; 3],
    /// Enemy ship IDs (master)
    pub(super) enemy_ship_ids: Vec<i32>,
    /// Enemy ship levels
    pub(super) enemy_ship_levels: Vec<i32>,
    /// Enemy ship equipment (per ship, each is a Vec of master equip IDs)
    pub(super) enemy_ship_slots: Vec<Vec<i32>>,
    /// Air battle result
    pub(super) air_battle: Option<AirBattleResult>,
    /// Whether midnight battle flag was set
    pub(super) midnight_flag: bool,
    /// Whether a night battle actually occurred
    pub(super) had_night_battle: bool,
    /// Raw battle API data (entire api_data)
    pub(super) raw_battle_json: Option<serde_json::Value>,
    /// Raw midnight battle API data
    pub(super) raw_midnight_json: Option<serde_json::Value>,
}

// =============================================================================
// BattleLogger - tracks active sortie and saves completed ones
// =============================================================================

#[derive(Debug, Default)]
pub struct BattleLogger {
    /// Currently active sortie (None if not in sortie)
    pub(super) active_sortie: Option<SortieRecord>,
    /// Pending battle data being accumulated
    pub(super) pending_battle: Option<PendingBattle>,
    /// Completed sortie records (newest first, kept in memory)
    pub(super) completed: Vec<SortieRecord>,
    /// Directory for persistent storage (completed records)
    pub(super) save_dir: Option<PathBuf>,
    /// Directory for raw API dumps
    raw_dir: Option<PathBuf>,
    /// Path storing the persistent developer-option state
    raw_enabled_path: Option<PathBuf>,
    /// Whether complete raw API saving is enabled
    raw_enabled: bool,
    /// Counter for raw dump ordering within a sortie
    raw_seq: u32,
}

impl BattleLogger {
    pub fn new(save_dir: PathBuf, raw_dir: PathBuf, raw_enabled_path: PathBuf) -> Self {
        // Load existing records from disk
        let completed = Self::load_from_disk(&save_dir);
        let raw_enabled = std::fs::read_to_string(&raw_enabled_path)
            .map(|value| value.trim() == "true")
            .unwrap_or(false);
        info!(
            "BattleLogger initialized with {} saved records (raw API: {})",
            completed.len(),
            if raw_enabled { "ON" } else { "OFF" }
        );
        let mut logger = Self {
            active_sortie: None,
            pending_battle: None,
            completed,
            save_dir: Some(save_dir),
            raw_dir: Some(raw_dir),
            raw_enabled_path: Some(raw_enabled_path),
            raw_enabled,
            raw_seq: 0,
        };
        logger.fix_interrupted_records();
        logger
    }

    /// Allocate raw API filename and increment sequence number.
    /// Returns (raw_dir, filename) without performing any file I/O.
    /// The actual file write should be done via `save_raw_api_to_disk` outside of any lock.
    pub fn allocate_raw_api_filename(&mut self, endpoint: &str) -> Option<(PathBuf, String)> {
        if !self.raw_enabled {
            return None;
        }
        let dir = match &self.raw_dir {
            Some(d) => d.clone(),
            None => return None,
        };

        let now = Local::now();
        let seq = self.raw_seq;
        self.raw_seq += 1;

        let clean_ep = endpoint.trim_start_matches("/kcsapi/").replace('/', "_");

        let filename = format!(
            "{}_{}_{:06}_{}.json",
            now.format("%Y%m%d_%H%M%S_%9f"),
            std::process::id(),
            seq,
            clean_ep
        );
        Some((dir, filename))
    }

    /// Handle sortie start (api_req_map/start)
    #[allow(clippy::too_many_arguments)]
    pub fn on_sortie_start(
        &mut self,
        json: &serde_json::Value,
        request_body: &str,
        fleets: &[Vec<i32>],
        player_ships: &HashMap<i32, ShipInfo>,
        player_slotitems: &HashMap<i32, PlayerSlotItem>,
        combined_flag: i32,
        mapinfo_gauges: &HashMap<i32, i32>,
    ) {
        // Parse map area and map no from request body
        let params = parse_form_body(request_body);
        let map_area = params
            .get("api_maparea_id")
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(0);
        let map_no = params
            .get("api_mapinfo_no")
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(0);
        let deck_id = params
            .get("api_deck_id")
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(1);

        let map_display = format!("{}-{}", map_area, map_no);

        // Get fleet ships
        let fleet_idx = (deck_id - 1) as usize;
        let ships = if fleet_idx < fleets.len() {
            fleets[fleet_idx]
                .iter()
                .filter_map(|&ship_id| {
                    player_ships.get(&ship_id).map(|info| {
                        // Snapshot regular equipment slots
                        let slots: Vec<SlotItemSnapshot> = info
                            .slot
                            .iter()
                            .filter(|&&slot_id| slot_id > 0)
                            .filter_map(|&slot_id| {
                                player_slotitems.get(&slot_id).map(|item| SlotItemSnapshot {
                                    id: item.slotitem_id,
                                    rf: item.level,
                                    mas: item.alv,
                                })
                            })
                            .collect();
                        // Snapshot reinforcement expansion slot
                        let slot_ex = if info.slot_ex > 0 {
                            player_slotitems
                                .get(&info.slot_ex)
                                .map(|item| SlotItemSnapshot {
                                    id: item.slotitem_id,
                                    rf: item.level,
                                    mas: item.alv,
                                })
                        } else {
                            None
                        };
                        SortieShip {
                            name: info.name.clone(),
                            ship_id: info.ship_id,
                            lv: info.lv,
                            stype: info.stype,
                            slots,
                            slot_ex,
                        }
                    })
                })
                .collect()
        } else {
            Vec::new()
        };

        // Parse first cell from response
        let api_data = json.get("api_data");
        let cell_no = api_data
            .and_then(|d| d.get("api_no"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;
        let event_kind = api_data
            .and_then(|d| d.get("api_color_no"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;
        let event_id = api_data
            .and_then(|d| d.get("api_event_id"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;

        // Extract gauge number for multi-gauge maps (e.g., 7-2, 7-3, 7-5)
        // First try api_eventmap (event maps), then fall back to cached mapinfo (regular maps)
        let gauge_num = api_data
            .and_then(|d| d.get("api_eventmap"))
            .and_then(|em| em.get("api_gauge_num"))
            .and_then(|v| v.as_i64())
            .map(|v| v as i32)
            .or_else(|| {
                let map_id = map_area * 10 + map_no;
                mapinfo_gauges.get(&map_id).copied()
            });

        let now = Local::now();
        let id = now.format("%Y%m%d_%H%M%S").to_string();

        let mut sortie = SortieRecord {
            id,
            fleet_id: deck_id,
            map_area,
            map_no,
            map_display: map_display.clone(),
            ships,
            nodes: Vec::new(),
            start_time: now,
            end_time: None,
            is_combined: combined_flag > 0,
            gauge_num,
        };

        // First node (map start always has a cell)
        if cell_no > 0 {
            sortie
                .nodes
                .push(BattleNode::new(cell_no, event_kind, event_id));
        }

        info!("Sortie started: {} (Fleet {})", map_display, deck_id);

        // Save initial sortie to disk immediately (crash recovery)
        self.save_to_disk(&sortie);

        self.active_sortie = Some(sortie);
        self.pending_battle = None;
    }

    /// Handle map next (api_req_map/next)
    pub fn on_map_next(
        &mut self,
        data: &crate::api::dto::battle::ApiMapNextResponse,
        json: &serde_json::Value,
    ) {
        let cell_no = data.api_no.unwrap_or(0);
        let event_kind = data.api_color_no.unwrap_or(0);
        let event_id = data.api_event_id.unwrap_or(0);
        let base_air_defense = parse_base_air_defense(json);

        let updated = if let Some(sortie) = &mut self.active_sortie {
            if cell_no <= 0 {
                false
            } else {
                let mut node = BattleNode::new(cell_no, event_kind, event_id);
                node.base_air_defense = base_air_defense;
                sortie.nodes.push(node);
                true
            }
        } else {
            return;
        };

        if updated {
            info!("Map next: cell {}", cell_no);
            if let Some(sortie) = &self.active_sortie {
                // Persist map movement immediately so an air-defense result is
                // not lost if the app exits before the next battle or port.
                self.save_to_disk(sortie);
            }
        }
    }

    /// Handle return to port (api_port/port) - finalize sortie
    pub fn on_port(&mut self) -> Option<SortieRecord> {
        let mut sortie = self.active_sortie.take()?;
        sortie.end_time = Some(Local::now());
        self.pending_battle = None;

        info!(
            "Sortie completed: {} ({} nodes)",
            sortie.map_display,
            sortie.nodes.len()
        );

        // Save to disk
        self.save_to_disk(&sortie);

        // Keep in memory (newest first)
        self.completed.insert(0, sortie.clone());

        // Keep at most 200 records in memory
        if self.completed.len() > 200 {
            self.completed.truncate(200);
        }

        Some(sortie)
    }

    /// Check if currently in a sortie
    pub fn is_in_sortie(&self) -> bool {
        self.active_sortie.is_some()
    }

    /// Get a reference to the active sortie (for reading map_area etc.)
    pub fn active_sortie_ref(&self) -> Option<&SortieRecord> {
        self.active_sortie.as_ref()
    }

    /// Get completed sortie records
    pub fn get_records(&self, limit: usize, offset: usize) -> Vec<SortieRecordSummary> {
        self.completed
            .iter()
            .skip(offset)
            .take(limit)
            .map(SortieRecordSummary::from)
            .collect()
    }

    /// Total number of completed records
    pub fn record_count(&self) -> usize {
        self.completed.len()
    }

    /// Clear all completed records (memory + disk)
    pub fn clear_records(&mut self) {
        self.completed.clear();
        if let Some(dir) = &self.save_dir {
            if dir.exists() {
                let _ = std::fs::remove_dir_all(dir);
                let _ = std::fs::create_dir_all(dir);
            }
        }
    }

    /// Clear raw API dumps on disk
    pub fn set_raw_enabled(&mut self, enabled: bool) -> Result<(), String> {
        if let Some(path) = &self.raw_enabled_path {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|error| {
                    format!("全ログ保存の設定フォルダを作成できません: {error}")
                })?;
            }
            std::fs::write(path, if enabled { "true" } else { "false" })
                .map_err(|error| format!("全ログ保存の設定を保存できません: {error}"))?;
        }
        self.raw_enabled = enabled;
        Ok(())
    }

    pub fn is_raw_enabled(&self) -> bool {
        self.raw_enabled
    }

    pub fn clear_raw_api(&self) {
        if let Some(dir) = &self.raw_dir {
            if dir.exists() {
                let _ = std::fs::remove_dir_all(dir);
                let _ = std::fs::create_dir_all(dir);
            }
        }
    }
}

// =============================================================================
// Helper functions
// =============================================================================

/// Parse URL-encoded form body into key-value pairs
fn parse_form_body(body: &str) -> HashMap<String, String> {
    body.split('&')
        .filter_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            let key = parts.next()?;
            let value = parts.next().unwrap_or("");
            Some((key.to_string(), value.to_string()))
        })
        .collect()
}

/// Extract an air-defense result from api_req_map/next.
///
/// The game has used both object and array containers around
/// api_air_base_attack, so locate the stage1 object defensively within
/// api_destruction_battle while keeping the full response in raw_api.
fn parse_base_air_defense(json: &serde_json::Value) -> Option<BaseAirDefenseResult> {
    let destruction = json
        .get("api_data")
        .and_then(|data| data.get("api_destruction_battle"))?;
    let stage1 = find_air_stage1(destruction)?;

    Some(BaseAirDefenseResult {
        occurred_at: Local::now(),
        air_superiority: json_i32(stage1, "api_disp_seiku"),
        friendly_plane_count: plane_counts(stage1, "api_f_count", "api_f_lostcount"),
        enemy_plane_count: plane_counts(stage1, "api_e_count", "api_e_lostcount"),
        lost_kind: json_i32(destruction, "api_lost_kind"),
    })
}

fn find_air_stage1(value: &serde_json::Value) -> Option<&serde_json::Value> {
    match value {
        serde_json::Value::Object(object) => {
            if let Some(stage1) = object.get("api_stage1") {
                if stage1.get("api_disp_seiku").is_some() {
                    return Some(stage1);
                }
            }
            object.values().find_map(find_air_stage1)
        }
        serde_json::Value::Array(values) => values.iter().find_map(find_air_stage1),
        _ => None,
    }
}

fn json_i32(value: &serde_json::Value, key: &str) -> Option<i32> {
    value
        .get(key)
        .and_then(serde_json::Value::as_i64)
        .and_then(|number| i32::try_from(number).ok())
}

fn plane_counts(value: &serde_json::Value, total_key: &str, lost_key: &str) -> Option<[i32; 2]> {
    Some([json_i32(value, total_key)?, json_i32(value, lost_key)?])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "kancolle-browser-{name}-{}-{}",
            std::process::id(),
            Local::now().timestamp_nanos_opt().unwrap_or_default()
        ))
    }

    #[test]
    fn parses_base_air_defense_from_map_next() {
        let json = serde_json::json!({
            "api_data": {
                "api_no": 42,
                "api_destruction_battle": {
                    "api_air_base_attack": {
                        "api_stage1": {
                            "api_disp_seiku": 2,
                            "api_f_count": 71,
                            "api_f_lostcount": 3,
                            "api_e_count": 48,
                            "api_e_lostcount": 19
                        }
                    },
                    "api_lost_kind": 1
                }
            }
        });

        let result = parse_base_air_defense(&json).unwrap();
        assert_eq!(result.air_superiority, Some(2));
        assert_eq!(result.friendly_plane_count, Some([71, 3]));
        assert_eq!(result.enemy_plane_count, Some([48, 19]));
        assert_eq!(result.lost_kind, Some(1));
    }

    #[test]
    fn raw_api_setting_survives_restart() {
        let root = test_path("raw-setting");
        let save_dir = root.join("battle_logs");
        let raw_dir = root.join("raw_api");
        let setting = root.join("local").join("raw_api_enabled");

        let mut logger = BattleLogger::new(save_dir.clone(), raw_dir.clone(), setting.clone());
        assert!(!logger.is_raw_enabled());
        logger.set_raw_enabled(true).unwrap();

        let restored = BattleLogger::new(save_dir, raw_dir, setting);
        assert!(restored.is_raw_enabled());

        let _ = std::fs::remove_dir_all(root);
    }
}
