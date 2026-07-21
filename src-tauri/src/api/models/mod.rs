mod air_base;
mod summary;
mod wire;

pub use air_base::*;
pub use summary::*;
pub use wire::*;

use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::battle_log::BattleLogger;
use crate::quest_progress::QuestProgressState;
use crate::senka::SenkaTracker;
use crate::sortie_quest::SortieQuestDef;

// =============================================================================
// GameState - Persistent in-memory storage for parsed API data
// =============================================================================

/// Master ship data (name + stype)
#[derive(Debug, Clone, Serialize)]
pub struct MasterShipInfo {
    pub name: String,
    pub stype: i32,
}

/// Information about an expedition/mission from master data
#[derive(Debug, Clone, Serialize)]
pub struct MissionInfo {
    pub name: String,
    /// Duration in minutes
    pub time: i32,
}

/// Master slot item info for equipment lookup
#[derive(Debug, Clone, Serialize)]
pub struct MasterSlotItemInfo {
    pub name: String,
    pub item_type: i32,
    /// Icon type from api_type[3] (e.g. 14 = damage control)
    pub icon_type: i32,
    /// Equipment stats for sorting in improvement tab
    pub firepower: i32,
    pub torpedo: i32,
    pub bombing: i32,
    pub aa: i32,
    pub asw: i32,
    pub los: i32,
}

/// Information about a player's ship instance
#[derive(Debug, Clone, Serialize)]
pub struct ShipInfo {
    pub ship_id: i32,
    pub name: String,
    pub stype: i32,
    pub lv: i32,
    pub hp: i32,
    pub maxhp: i32,
    pub cond: i32,
    pub fuel: i32,
    pub bull: i32,
    /// Stats: [current_with_equip, base]. Index 0 = equipped value.
    pub firepower: i32,
    pub torpedo: i32,
    pub aa: i32,
    pub armor: i32,
    pub asw: i32,
    pub evasion: i32,
    pub los: i32,
    pub luck: i32,
    pub locked: bool,
    /// Equipment slot IDs (-1 = empty)
    pub slot: Vec<i32>,
    /// Reinforcement expansion slot ID (-1 = no slot, 0 = empty slot, >0 = equipped)
    pub slot_ex: i32,
    /// Speed: 5=低速, 10=高速, 15=高速+, 20=最速
    pub soku: i32,
    /// 出撃札 (`api_sally_area`): 0 = 札なし, N = 札N。
    /// イベント海域に出撃すると付与され、以降その札が許可された海域にしか出せない。
    /// 札の名称はAPIに含まれない (ゲーム側のUI素材のみ) ため番号で扱う。
    pub sally_area: i32,
}

/// Player equipment instance
#[derive(Debug, Clone, Serialize)]
pub struct PlayerSlotItem {
    pub item_id: i32,
    /// Master slotitem ID (type of equipment)
    pub slotitem_id: i32,
    /// Improvement/remodel level (0-10, ★)
    pub level: i32,
    /// Aircraft proficiency (0-7, >>)
    pub alv: Option<i32>,
    /// Whether this item is locked
    pub locked: bool,
}

/// Static master data from api_start2 (immutable during session)
#[derive(Debug, Default)]
pub struct MasterData {
    /// Master ship data: ship_id -> MasterShipInfo (name + stype)
    pub ships: HashMap<i32, MasterShipInfo>,
    /// Master ship type data: stype_id -> stype_name
    pub stypes: HashMap<i32, String>,
    /// Master mission data: mission_id -> MissionInfo
    pub missions: HashMap<i32, MissionInfo>,
    /// Master slot item data: slotitem_id -> MasterSlotItemInfo
    pub slotitems: HashMap<i32, MasterSlotItemInfo>,
    /// Master equip type data: equip_type_id -> name
    pub equip_types: HashMap<i32, String>,
}

/// Player's homeport assets and fleet compositions
#[derive(Debug, Default)]
pub struct UserProfile {
    /// Player ship instances: ship_instance_id -> ShipInfo
    pub ships: HashMap<i32, ShipInfo>,
    /// Player equipment instances: slot_item_instance_id -> PlayerSlotItem
    pub slotitems: HashMap<i32, PlayerSlotItem>,
    /// Fleet compositions: fleet_index (0-3) -> ship instance IDs
    pub fleets: Vec<Vec<i32>>,
    /// Combined fleet flag: 0=none, 1=carrier TF, 2=surface TF, 3=transport escort
    pub combined_flag: i32,
}

/// Sortie session and battle logging state
#[derive(Debug, Default)]
pub struct SortieState {
    /// Battle logger for tracking sorties
    pub battle_logger: BattleLogger,
    /// Cached last port summary for re-emitting during sortie
    pub last_port_summary: Option<PortSummary>,
    /// Ship instance IDs offered for retreat by the latest battle result.
    /// These are only promoted to `escaped_ship_ids` when goback_port confirms
    /// that the player accepted the retreat.
    pub pending_escape_ship_ids: HashSet<i32>,
    /// Ship instance IDs that have retreated during the active sortie.
    pub escaped_ship_ids: HashSet<i32>,
}

/// Player's accumulated activity records and quest tracking
#[derive(Debug, Default)]
pub struct UserHistory {
    /// Currently active (accepted/completed) quest IDs from api_get_member/questlist
    pub active_quests: HashSet<i32>,
    /// Active quest details (id -> ActiveQuestDetail) accumulated across pages
    pub active_quest_details: HashMap<i32, ActiveQuestDetail>,
    /// Cached sortie quest definitions (loaded once)
    pub sortie_quest_defs: Vec<SortieQuestDef>,
    /// Set of master equipment IDs that have been previously improved
    pub improved_equipment: std::collections::HashSet<i32>,
    /// Quest progress tracking state
    pub quest_progress: QuestProgressState,
}

/// Inner mutable state for game data
#[derive(Debug, Default)]
pub struct GameStateInner {
    /// Static master data (api_start2)
    pub master: MasterData,
    /// Player homeport data (ships, equipment, fleets)
    pub profile: UserProfile,
    /// Sortie session state (battle logger, port summary cache)
    pub sortie: SortieState,
    /// Player activity history (quests, improvements, progress)
    pub history: UserHistory,
    /// Path to improved equipment history file
    pub improved_equipment_path: std::path::PathBuf,
    /// Path to quest progress file
    pub quest_progress_path: std::path::PathBuf,
    /// Base data directory (app_local_data_dir)
    pub data_dir: std::path::PathBuf,
    /// Senka (ranking points) tracker
    pub senka: SenkaTracker,
    /// Sync notifier — sends SyncCommand to the background sync engine
    pub sync_notifier: Option<tokio::sync::mpsc::Sender<crate::drive_sync::SyncCommand>>,
    /// Cached gauge numbers from mapinfo: map_id (area*10+no) -> gauge_num
    pub mapinfo_gauges: HashMap<i32, i32>,
    /// Formation memory: "{map_area}-{map_no}-{cell_no}" -> formation_id
    pub formation_memory: HashMap<String, i32>,
    /// Path to formation memory file
    pub formation_memory_path: std::path::PathBuf,
    /// Land-Based Air Squadron (基地航空隊) state from mapinfo / air_corps APIs.
    /// Keyed by (area_id, rid). 4 squadrons per base.
    pub air_bases: Vec<AirBase>,
}

/// Thread-safe game state accessible via Tauri managed state
#[derive(Debug, Clone)]
pub struct GameState {
    pub inner: Arc<RwLock<GameStateInner>>,
}

impl GameState {
    pub fn new(data_dir: PathBuf) -> Self {
        let sync_dir = data_dir.join("sync");
        let mut inner = GameStateInner::default();
        inner.sortie.battle_logger =
            BattleLogger::new(sync_dir.join("battle_logs"), sync_dir.join("raw_api"));

        // Load improved equipment history
        let improved_path = sync_dir.join("improved_equipment.json");
        inner.history.improved_equipment =
            crate::improvement::load_improved_history(&improved_path);
        inner.improved_equipment_path = improved_path;

        // Load quest progress
        let quest_progress_path = sync_dir.join("quest_progress.json");
        inner.history.quest_progress = crate::quest_progress::load_progress(&quest_progress_path);
        inner.quest_progress_path = quest_progress_path;

        // Load formation memory
        let formation_memory_path = sync_dir.join("formation_memory.json");
        inner.formation_memory = super::formation::load_memory(&formation_memory_path);
        inner.formation_memory_path = formation_memory_path;

        // Initialize senka tracker
        inner.senka = SenkaTracker::new(&data_dir);

        // Store data_dir for sync module access
        inner.data_dir = data_dir;

        // Load sortie quest definitions (cached for progress tracking)
        inner.history.sortie_quest_defs = crate::sortie_quest::get_all_sortie_quests();

        // Initial reset check
        crate::quest_progress::check_resets(
            &mut inner.history.quest_progress,
            &inner.history.sortie_quest_defs,
            &inner.quest_progress_path,
        );

        Self {
            inner: Arc::new(RwLock::new(inner)),
        }
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            inner: Arc::new(RwLock::new(GameStateInner::default())),
        }
    }
}
