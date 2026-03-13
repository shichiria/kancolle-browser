mod parser;
mod storage;

use chrono::{DateTime, Local};
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::api::models::{PlayerSlotItem, ShipInfo};

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
    /// Air superiority state (api_disp_seiku): 0=denial, 1=superiority, 2=supremacy, 3=parity, 4=incapability
    pub air_superiority: Option<i32>,
    /// Friendly plane count [total, lost]
    pub friendly_plane_count: Option<[i32; 2]>,
    /// Enemy plane count [total, lost]
    pub enemy_plane_count: Option<[i32; 2]>,
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
    /// Create a new empty node (no battle yet)
    fn new(cell_no: i32, event_kind: i32, event_id: i32) -> Self {
        Self {
            cell_no,
            event_kind,
            event_id,
            battle: None,
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
    /// Fleet index (0-based)
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
    pub map_display: String,
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
            map_display: r.map_display.clone(),
            ships: r.ships.clone(),
            nodes: r.nodes.clone(),
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

/// Write a raw API dump to disk. This function performs file I/O and should be
/// called OUTSIDE of any GameState lock to avoid blocking state updates.
pub fn save_raw_api_to_disk(
    dir: &PathBuf,
    filename: &str,
    endpoint: &str,
    request_body: &str,
    response_body: &str,
) -> bool {
    if let Err(e) = fs::create_dir_all(dir) {
        error!("Failed to create raw API dir: {}", e);
        return false;
    }

    let path = dir.join(filename);
    let dump = serde_json::json!({
        "endpoint": endpoint,
        "timestamp": Local::now().to_rfc3339(),
        "request_body": request_body,
        "response_body_length": response_body.len(),
        "response_body": serde_json::from_str::<serde_json::Value>(response_body)
            .unwrap_or_else(|_| serde_json::Value::String(response_body.to_string())),
    });

    match serde_json::to_string_pretty(&dump) {
        Ok(json) => {
            if let Err(e) = fs::write(&path, json) {
                error!("Failed to write raw API dump {}: {}", filename, e);
                false
            } else {
                info!("Raw API saved: {}", filename);
                true
            }
        }
        Err(e) => {
            error!("Failed to serialize raw API dump: {}", e);
            false
        }
    }
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
    /// Whether raw API saving is enabled (developer option, default OFF)
    raw_enabled: bool,
    /// Counter for raw dump ordering within a sortie
    raw_seq: u32,
}

impl BattleLogger {
    pub fn new(save_dir: PathBuf, raw_dir: PathBuf) -> Self {
        // Load existing records from disk
        let completed = Self::load_from_disk(&save_dir);
        info!(
            "BattleLogger initialized with {} saved records",
            completed.len()
        );
        let mut logger = Self {
            active_sortie: None,
            pending_battle: None,
            completed,
            save_dir: Some(save_dir),
            raw_dir: Some(raw_dir),
            raw_enabled: false,
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
            "{}_{:03}_{}.json",
            now.format("%Y%m%d_%H%M%S"),
            seq,
            clean_ep
        );
        Some((dir, filename))
    }

    /// Handle sortie start (api_req_map/start)
    pub fn on_sortie_start(
        &mut self,
        json: &serde_json::Value,
        request_body: &str,
        fleets: &[Vec<i32>],
        player_ships: &HashMap<i32, ShipInfo>,
        player_slotitems: &HashMap<i32, PlayerSlotItem>,
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
        let gauge_num = api_data
            .and_then(|d| d.get("api_eventmap"))
            .and_then(|em| em.get("api_gauge_num"))
            .and_then(|v| v.as_i64())
            .map(|v| v as i32);

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
            is_combined: false,
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
        _json: &serde_json::Value,
    ) {
        let sortie = match &mut self.active_sortie {
            Some(s) => s,
            None => return,
        };

        let cell_no = data.api_no.unwrap_or(0);
        let event_kind = data.api_color_no.unwrap_or(0);
        let event_id = data.api_event_id.unwrap_or(0);

        if cell_no > 0 {
            sortie
                .nodes
                .push(BattleNode::new(cell_no, event_kind, event_id));
            info!("Map next: cell {}", cell_no);
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
    pub fn set_raw_enabled(&mut self, enabled: bool) {
        self.raw_enabled = enabled;
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
