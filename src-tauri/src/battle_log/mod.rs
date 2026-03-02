use chrono::{DateTime, Local};
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::api::models::{MasterShipInfo, PlayerSlotItem, ShipInfo};

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
struct PendingBattle {
    /// Friendly fleet HP before battle
    friendly_hp_before: Vec<(i32, i32)>, // (now, max)
    /// Friendly fleet HP after all phases
    friendly_hp_after: Vec<(i32, i32)>, // (now, max)
    /// Enemy fleet HP before battle
    enemy_hp_before: Vec<(i32, i32)>, // (now, max)
    /// Enemy fleet HP after all phases
    enemy_hp_after: Vec<(i32, i32)>, // (now, max)
    /// Formation [friendly, enemy, engagement]
    formation: [i32; 3],
    /// Enemy ship IDs (master)
    enemy_ship_ids: Vec<i32>,
    /// Enemy ship levels
    enemy_ship_levels: Vec<i32>,
    /// Enemy ship equipment (per ship, each is a Vec of master equip IDs)
    enemy_ship_slots: Vec<Vec<i32>>,
    /// Air battle result
    air_battle: Option<AirBattleResult>,
    /// Whether midnight battle flag was set
    midnight_flag: bool,
    /// Whether a night battle actually occurred
    had_night_battle: bool,
    /// Raw battle API data (entire api_data)
    raw_battle_json: Option<serde_json::Value>,
    /// Raw midnight battle API data
    raw_midnight_json: Option<serde_json::Value>,
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
    active_sortie: Option<SortieRecord>,
    /// Pending battle data being accumulated
    pending_battle: Option<PendingBattle>,
    /// Completed sortie records (newest first, kept in memory)
    completed: Vec<SortieRecord>,
    /// Directory for persistent storage (completed records)
    save_dir: Option<PathBuf>,
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
        Self {
            active_sortie: None,
            pending_battle: None,
            completed,
            save_dir: Some(save_dir),
            raw_dir: Some(raw_dir),
            raw_enabled: false,
            raw_seq: 0,
        }
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
        json: &serde_json::Value,
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

    // --- Persistence ---

    fn save_to_disk(&self, record: &SortieRecord) {
        let dir = match &self.save_dir {
            Some(d) => d,
            None => return,
        };
        if let Err(e) = fs::create_dir_all(dir) {
            error!("Failed to create battle log dir: {}", e);
            return;
        }

        let filename = format!("{}.json", record.id);
        let path = dir.join(&filename);
        match serde_json::to_string_pretty(record) {
            Ok(json) => {
                if let Err(e) = fs::write(&path, json) {
                    error!("Failed to write battle log {}: {}", filename, e);
                } else {
                    info!("Battle log saved: {}", filename);
                }
            }
            Err(e) => {
                error!("Failed to serialize battle log: {}", e);
            }
        }
    }

    /// Reload completed records from disk (used after sync downloads new files).
    pub fn reload_from_disk(&mut self) {
        if let Some(dir) = &self.save_dir {
            self.completed = Self::load_from_disk(dir);
            info!("BattleLogger reloaded: {} records", self.completed.len());
        }
    }

    fn load_from_disk(dir: &PathBuf) -> Vec<SortieRecord> {
        let mut records = Vec::new();
        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return records, // Directory doesn't exist yet
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            match fs::read_to_string(&path) {
                Ok(content) => match serde_json::from_str::<SortieRecord>(&content) {
                    Ok(mut record) => {
                        // Migrate legacy BattleNode format to new BattleDetail format
                        for node in &mut record.nodes {
                            node.migrate_legacy();
                        }
                        records.push(record);
                    }
                    Err(e) => {
                        warn!("Failed to parse battle log {:?}: {}", path.file_name(), e);
                    }
                },
                Err(e) => {
                    warn!("Failed to read battle log {:?}: {}", path.file_name(), e);
                }
            }
        }

        // Sort by start_time descending (newest first)
        records.sort_by(|a, b| b.start_time.cmp(&a.start_time));

        // Keep at most 200
        records.truncate(200);

        records
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
///   eflag=1 means enemy attacking → target index is a friendly ship (0-based)
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
                continue; // Friendly attacking enemy — skip for friendly HP calc
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
///   eflag=0 means friendly attacking → target index is an enemy ship (0-based)
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
                continue; // Enemy attacking friendly — skip for enemy HP calc
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
