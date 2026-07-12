use serde::Deserialize;

use crate::api::models::{Fleet, PlayerShip, PlayerSlotItemApi};

// =============================================================================
// Category A DTOs (処理済み)
// =============================================================================

/// Response for api_get_member/ship3
/// Contains updated ship data and fleet compositions after equipment changes
#[derive(Debug, Deserialize, Clone)]
pub struct ApiShip3Response {
    pub api_ship_data: Vec<PlayerShip>,
    #[serde(default)]
    pub api_deck_data: Vec<Fleet>,
}

/// Inner ship data for slot_deprive response
#[derive(Debug, Deserialize, Clone)]
pub struct ApiSlotDepriveShipData {
    pub api_set_ship: PlayerShip,
    pub api_unset_ship: PlayerShip,
}

/// Response for api_req_kaisou/slot_deprive
#[derive(Debug, Deserialize, Clone)]
pub struct ApiSlotDepriveResponse {
    pub api_ship_data: ApiSlotDepriveShipData,
}

/// Response for api_req_practice/battle_result
#[derive(Debug, Deserialize, Clone)]
pub struct ApiExerciseResultResponse {
    pub api_win_rank: String,
    #[serde(default)]
    pub api_get_exp: i64,
}

/// Response for api_get_member/questlist
#[derive(Debug, Deserialize, Clone)]
pub struct ApiQuestListResponse {
    pub api_list: Option<Vec<serde_json::Value>>,
}

/// Response for api_req_hensei/preset_select
/// The api_data IS a Fleet object directly (api_id, api_name, api_ship, etc.)
#[derive(Debug, Deserialize, Clone)]
pub struct ApiHenseiPresetSelectResponse {
    pub api_id: i32,
    #[allow(dead_code)] // kept for API schema completeness
    #[serde(default)]
    pub api_name: String,
    #[serde(default)]
    pub api_ship: Vec<i32>,
    #[serde(flatten)]
    _extra: serde_json::Value,
}

/// Single ship entry in api_req_hokyu/charge response
#[derive(Debug, Deserialize, Clone)]
pub struct ApiChargeShip {
    pub api_id: i32,
    pub api_fuel: i32,
    pub api_bull: i32,
    #[allow(dead_code)] // kept for API schema completeness
    #[serde(default)]
    pub api_onslot: Vec<i32>,
}

/// Response for api_req_hokyu/charge (resupply)
#[derive(Debug, Deserialize, Clone)]
pub struct ApiChargeResponse {
    pub api_ship: Vec<ApiChargeShip>,
    #[allow(dead_code)] // kept for API schema completeness
    pub api_material: Vec<i64>,
    #[allow(dead_code)] // kept for API schema completeness
    #[serde(default)]
    pub api_use_bou: i32,
}

/// Response for api_req_kousyou/remodel_slot
#[derive(Debug, Deserialize, Clone)]
pub struct ApiRemodelSlotResponse {
    pub api_remodel_flag: Option<i32>,
    pub api_after_slot: Option<ApiAfterSlot>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ApiAfterSlot {
    pub api_slotitem_id: Option<i32>,
}

// =============================================================================
// Category B DTOs (新規実装)
// =============================================================================

// --- Group 2: Ship/Equipment state updates ---

/// Response for api_req_kaisou/powerup (近代化改修)
#[derive(Debug, Deserialize, Clone)]
pub struct ApiPowerupResponse {
    pub api_powerup_flag: i32,
    pub api_ship: PlayerShip,
    #[serde(default)]
    pub api_deck: Vec<Fleet>,
    #[serde(flatten)]
    _extra: serde_json::Value,
}

/// Response for api_req_kaisou/slot_exchange_index (装備入替)
#[derive(Debug, Deserialize, Clone)]
pub struct ApiSlotExchangeResponse {
    pub api_ship_data: PlayerShip,
    #[serde(flatten)]
    _extra: serde_json::Value,
}

/// Response for api_req_kousyou/getship (建造完了)
#[derive(Debug, Deserialize, Clone)]
pub struct ApiGetShipResponse {
    pub api_ship: PlayerShip,
    #[serde(default)]
    pub api_slotitem: Vec<PlayerSlotItemApi>,
    #[allow(dead_code)] // kept for API schema completeness
    #[serde(default)]
    pub api_kdock: Vec<serde_json::Value>,
    #[serde(flatten)]
    _extra: serde_json::Value,
}

// --- Group 3: Removal operations ---
// destroyitem2 / destroyship use only the request DTOs (DestroyItem2Req /
// DestroyShipReq); material refresh arrives via follow-up api_get_member/material.

/// Response for api_req_kousyou/createitem (開発)
#[derive(Debug, Deserialize, Clone)]
pub struct ApiCreateItemResponse {
    #[serde(default)]
    pub api_create_flag: i32,
    #[serde(default)]
    pub api_get_items: Vec<serde_json::Value>,
    #[allow(dead_code)] // kept for API schema completeness
    #[serde(default)]
    pub api_material: Vec<i64>,
    #[serde(flatten)]
    _extra: serde_json::Value,
}

// --- Group 5: Mission ---

/// Response for api_req_mission/result (遠征結果)
#[derive(Debug, Deserialize, Clone)]
pub struct ApiMissionResultResponse {
    #[serde(default)]
    pub api_clear_result: i32,
    #[serde(default)]
    pub api_get_material: Vec<i64>,
    #[serde(default)]
    pub api_get_exp: i64,
    #[serde(flatten)]
    _extra: serde_json::Value,
}

// --- Request DTOs for null-response endpoints ---

/// Request for api_req_kousyou/destroyship
#[derive(Debug, Deserialize)]
pub struct DestroyShipReq {
    pub api_ship_id: String, // can be comma-separated for batch
}

/// Request for api_req_kousyou/destroyitem2
#[derive(Debug, Deserialize)]
pub struct DestroyItem2Req {
    pub api_slotitem_ids: String, // comma-separated item instance IDs
}
