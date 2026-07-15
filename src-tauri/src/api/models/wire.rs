use serde::Deserialize;

/// Generic KanColle API response wrapper
/// All API responses follow: { "api_result": 1, "api_result_msg": "成功", "api_data": {...} }
#[derive(Debug, Deserialize)]
pub struct ApiResponse<T> {
    #[allow(dead_code)] // kept for API schema completeness
    pub api_result: i32,
    #[allow(dead_code)] // kept for API schema completeness
    pub api_result_msg: Option<String>,
    pub api_data: Option<T>,
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
    #[serde(default)]
    pub api_combined_flag: i32,
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
#[derive(Debug, Deserialize, Clone)]
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

#[derive(Debug, Deserialize, Clone)]
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

#[derive(Debug, Deserialize, Clone)]
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
