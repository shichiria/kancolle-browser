use serde::Serialize;

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
    /// Command facility name if activation conditions are met
    pub command_facility_name: Option<String>,
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
    /// 出撃札 (`api_sally_area`): 0 = 札なし, N = 札N
    pub sally_area: i32,
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
