use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::battle_log::BattleLogger;
use crate::quest_progress::QuestProgressState;
use crate::senka::SenkaTracker;
use crate::sortie_quest::SortieQuestDef;

/// Generic KanColle API response wrapper
/// All API responses follow: { "api_result": 1, "api_result_msg": "成功", "api_data": {...} }
#[derive(Debug, Deserialize)]
pub struct ApiResponse<T> {
    pub api_result: i32,
    pub api_result_msg: Option<String>,
    pub api_data: Option<T>,
}

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
}

/// Sortie session and battle logging state
#[derive(Debug, Default)]
pub struct SortieState {
    /// Battle logger for tracking sorties
    pub battle_logger: BattleLogger,
    /// Cached last port summary for re-emitting during sortie
    pub last_port_summary: Option<PortSummary>,
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
        inner.history.improved_equipment = crate::improvement::load_improved_history(&improved_path);
        inner.improved_equipment_path = improved_path;

        // Load quest progress
        let quest_progress_path = sync_dir.join("quest_progress.json");
        inner.history.quest_progress = crate::quest_progress::load_progress(&quest_progress_path);
        inner.quest_progress_path = quest_progress_path;

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

// =============================================================================
// api_start2/getData - Master game data
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct ApiStart2 {
    pub api_mst_ship: Vec<MasterShip>,
    pub api_mst_slotitem: Vec<MasterSlotItem>,
    pub api_mst_stype: Vec<MasterShipType>,
    #[serde(default)]
    pub api_mst_mission: Vec<MasterMission>,
    #[serde(default)]
    pub api_mst_slotitem_equiptype: Vec<MasterEquipType>,
    #[serde(flatten)]
    _extra: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct MasterEquipType {
    pub api_id: i32,
    #[serde(default)]
    pub api_name: String,
    #[serde(flatten)]
    _extra: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct MasterShip {
    pub api_id: i32,
    #[serde(default)]
    pub api_name: String,
    #[serde(default)]
    pub api_stype: i32,
    #[serde(flatten)]
    _extra: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct MasterSlotItem {
    pub api_id: i32,
    #[serde(default)]
    pub api_name: String,
    /// Equipment type array: [0]=大分類, [1]=図鑑表示, [2]=カテゴリ, [3]=アイコン, [4]=航空機カテゴリ
    #[serde(default)]
    pub api_type: serde_json::Value,
    /// Equipment stats
    #[serde(default)]
    pub api_houg: i32,
    #[serde(default)]
    pub api_raig: i32,
    #[serde(default)]
    pub api_baku: i32,
    #[serde(default)]
    pub api_tyku: i32,
    #[serde(default)]
    pub api_tais: i32,
    #[serde(default)]
    pub api_saku: i32,
    #[serde(flatten)]
    _extra: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct MasterShipType {
    pub api_id: i32,
    #[serde(default)]
    pub api_name: String,
    #[serde(flatten)]
    _extra: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct MasterMission {
    pub api_id: i32,
    #[serde(default)]
    pub api_name: String,
    #[serde(default)]
    pub api_time: i32,
    #[serde(flatten)]
    _extra: serde_json::Value,
}

// =============================================================================
// api_port/port - Home screen data
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct ApiPort {
    pub api_basic: AdmiralBasic,
    pub api_ship: Vec<PlayerShip>,
    pub api_deck_port: Vec<Fleet>,
    pub api_ndock: Vec<RepairDock>,
    pub api_material: Vec<Material>,
    #[serde(flatten)]
    _extra: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct AdmiralBasic {
    #[serde(default)]
    pub api_nickname: String,
    #[serde(default)]
    pub api_level: i32,
    #[serde(default)]
    pub api_rank: i32,
    #[serde(default)]
    pub api_max_chara: i32,
    #[serde(default)]
    pub api_experience: serde_json::Value,
    #[serde(flatten)]
    _extra: serde_json::Value,
}

/// Player ship instance - only fields we actually use are strongly typed.
/// All other fields are ignored via `deny_unknown_fields` being absent (serde default).
#[derive(Debug, Deserialize)]
pub struct PlayerShip {
    pub api_id: i32,
    #[serde(default)]
    pub api_ship_id: i32,
    #[serde(default)]
    pub api_lv: i32,
    #[serde(default)]
    pub api_nowhp: i32,
    #[serde(default)]
    pub api_maxhp: i32,
    #[serde(default)]
    pub api_cond: i32,
    #[serde(default)]
    pub api_fuel: i32,
    #[serde(default)]
    pub api_bull: i32,
    /// Stats arrays: [equipped_value, base_value] - index 0 is total with equipment
    #[serde(default)]
    pub api_karyoku: serde_json::Value,
    #[serde(default)]
    pub api_raisou: serde_json::Value,
    #[serde(default)]
    pub api_taiku: serde_json::Value,
    #[serde(default)]
    pub api_soukou: serde_json::Value,
    #[serde(default)]
    pub api_taisen: serde_json::Value,
    #[serde(default)]
    pub api_kaihi: serde_json::Value,
    #[serde(default)]
    pub api_sakuteki: serde_json::Value,
    #[serde(default)]
    pub api_lucky: serde_json::Value,
    #[serde(default)]
    pub api_locked: i32,
    /// Equipment slot IDs (instance IDs, -1 = empty)
    #[serde(default)]
    pub api_slot: serde_json::Value,
    /// Reinforcement expansion slot (-1 = no slot, 0 = empty, >0 = equipped instance ID)
    #[serde(default)]
    pub api_slot_ex: i32,
    /// Speed: 5=低速, 10=高速, 15=高速+, 20=最速
    #[serde(default)]
    pub api_soku: i32,
    /// Capture all other fields without strongly typing them
    #[serde(flatten)]
    _extra: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct Fleet {
    pub api_id: i32,
    #[serde(default)]
    pub api_name: String,
    #[serde(default)]
    pub api_ship: Vec<i32>,
    #[serde(default)]
    pub api_mission: Vec<serde_json::Value>,
    #[serde(flatten)]
    _extra: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct RepairDock {
    pub api_id: i32,
    #[serde(default)]
    pub api_state: i32,
    #[serde(default)]
    pub api_ship_id: i32,
    #[serde(default)]
    pub api_complete_time: i64,
    #[serde(flatten)]
    _extra: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct Material {
    pub api_id: i32,
    #[serde(default)]
    pub api_value: i32,
    #[serde(flatten)]
    _extra: serde_json::Value,
}

// =============================================================================
// api_get_member/slot_item - Player equipment data
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct PlayerSlotItemApi {
    pub api_id: i32,
    #[serde(default)]
    pub api_slotitem_id: i32,
    /// Improvement/remodel level (0-10, ★)
    #[serde(default)]
    pub api_level: i32,
    /// Aircraft proficiency (0-7, >>)
    #[serde(default)]
    pub api_alv: Option<i32>,
    #[serde(default)]
    pub api_locked: i32,
    #[serde(flatten)]
    _extra: serde_json::Value,
}

// =============================================================================
// Enriched summary types sent to frontend
// =============================================================================

/// A single ship's summary for fleet display
#[derive(Debug, Serialize, Clone)]
pub struct ShipSummary {
    /// Ship instance ID
    pub id: i32,
    /// Ship name (resolved from master data)
    pub name: String,
    /// Ship level
    pub lv: i32,
    /// Current HP
    pub hp: i32,
    /// Maximum HP
    pub maxhp: i32,
    /// Morale/condition
    pub cond: i32,
    /// Current fuel
    pub fuel: i32,
    /// Current ammo
    pub bull: i32,
    /// Damage control item name if equipped (icon_type 14), e.g. "応急修理要員"
    pub damecon_name: Option<String>,
    /// Special equipment for expedition display (drums icon_type=25, landing craft icon_type=20)
    pub special_equips: Vec<SpecialEquip>,
    /// Whether this ship can perform opening ASW attack
    pub can_opening_asw: bool,
    /// Speed: 5=低速, 10=高速, 15=高速+, 20=最速
    pub soku: i32,
}

/// A special equipment item displayed as an icon in the fleet panel
#[derive(Debug, Serialize, Clone)]
pub struct SpecialEquip {
    /// Equipment name (e.g. "ドラム缶(輸送用)", "大発動艇")
    pub name: String,
    /// Icon type from api_type[3] (20=landing craft, 25=drum canister)
    pub icon_type: i32,
}

/// Expedition information for a fleet
#[derive(Debug, Serialize, Clone)]
pub struct ExpeditionInfo {
    /// Mission ID (0 = not on expedition)
    pub mission_id: i32,
    /// Mission name (resolved from master data)
    pub mission_name: String,
    /// Return timestamp (milliseconds since epoch)
    pub return_time: i64,
}

/// Enriched fleet summary with ship details and expedition info
#[derive(Debug, Serialize, Clone)]
pub struct FleetSummary {
    pub id: i32,
    pub name: String,
    /// Ships in this fleet with full details
    pub ships: Vec<ShipSummary>,
    /// Expedition info (None if not on expedition)
    pub expedition: Option<ExpeditionInfo>,
}

/// Enriched repair dock summary with ship name
#[derive(Debug, Serialize, Clone)]
pub struct DockSummary {
    pub id: i32,
    pub state: i32,
    pub ship_id: i32,
    /// Ship name (resolved from master/player data)
    pub ship_name: String,
    pub complete_time: i64,
}

/// Active quest detail from api_get_member/questlist
#[derive(Debug, Serialize, Clone)]
pub struct ActiveQuestDetail {
    pub id: i32,
    pub title: String,
    pub category: i32,
}

/// Enriched port summary sent to the frontend
#[derive(Debug, Serialize, Clone)]
pub struct PortSummary {
    pub admiral_name: String,
    pub admiral_level: i32,
    pub admiral_rank: i32,
    pub ship_count: usize,
    pub ship_capacity: i32,
    // Basic resources
    pub fuel: i32,
    pub ammo: i32,
    pub steel: i32,
    pub bauxite: i32,
    // Consumable resources
    pub instant_repair: i32,
    pub instant_build: i32,
    pub dev_material: i32,
    pub improvement_material: i32,
    // Enriched fleet data
    pub fleets: Vec<FleetSummary>,
    pub ndock: Vec<DockSummary>,
}

// =============================================================================
// Ship/Equipment list response types for frontend tabs
// =============================================================================

/// A single ship entry for the ship list tab
#[derive(Debug, Serialize)]
pub struct ShipListItem {
    pub id: i32,
    pub ship_id: i32,
    pub name: String,
    pub stype: i32,
    pub stype_name: String,
    pub lv: i32,
    pub hp: i32,
    pub maxhp: i32,
    pub cond: i32,
    pub firepower: i32,
    pub torpedo: i32,
    pub aa: i32,
    pub armor: i32,
    pub asw: i32,
    pub evasion: i32,
    pub los: i32,
    pub luck: i32,
    pub locked: bool,
}

/// Response for the ship list tab
#[derive(Debug, Serialize)]
pub struct ShipListResponse {
    pub ships: Vec<ShipListItem>,
    pub stypes: Vec<(i32, String)>,
}

/// A single equipment entry (grouped by master ID) for the equipment list tab
#[derive(Debug, Serialize)]
pub struct EquipListItem {
    pub master_id: i32,
    pub name: String,
    pub type_id: i32,
    pub type_name: String,
    pub icon_type: i32,
    pub total_count: i32,
    pub locked_count: i32,
    /// (improvement_level, count) sorted by level
    pub improvements: Vec<(i32, i32)>,
}

/// Response for the equipment list tab
#[derive(Debug, Serialize)]
pub struct EquipListResponse {
    pub items: Vec<EquipListItem>,
    pub equip_types: Vec<(i32, String)>,
}
