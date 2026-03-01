use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct ApiBattleResponse {
    pub api_formation: Option<Vec<i32>>,
    pub api_ship_ke: Option<Vec<i32>>,
    pub api_ship_lv: Option<Vec<i32>>,
    #[serde(rename = "api_eSlot")]
    pub api_e_slot: Option<Vec<Vec<i32>>>,
    pub api_f_nowhps: Option<Vec<i32>>,
    pub api_f_maxhps: Option<Vec<i32>>,
    pub api_e_nowhps: Option<Vec<i32>>,
    pub api_e_maxhps: Option<Vec<i32>>,
    pub api_midnight_flag: Option<i32>,
    pub api_kouku: Option<ApiKouku>,
    pub api_opening_atack: Option<ApiRaigeki>,
    pub api_opening_taisen: Option<ApiHougeki>,
    pub api_hougeki1: Option<ApiHougeki>,
    pub api_hougeki2: Option<ApiHougeki>,
    pub api_hougeki3: Option<ApiHougeki>,
    pub api_raigeki: Option<ApiRaigeki>,
    // Midnight specific
    pub api_hougeki: Option<ApiHougeki>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ApiKouku {
    pub api_stage1: Option<ApiKoukuStage1>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ApiKoukuStage1 {
    pub api_disp_seiku: Option<i32>,
    pub api_f_count: Option<i32>,
    pub api_f_lostcount: Option<i32>,
    pub api_e_count: Option<i32>,
    pub api_e_lostcount: Option<i32>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ApiHougeki {
    pub api_at_eflag: Option<Vec<i32>>,
    pub api_df_list: Option<Vec<serde_json::Value>>,
    pub api_damage: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ApiRaigeki {
    pub api_fdam: Option<Vec<f64>>,
    pub api_edam: Option<Vec<f64>>,
}

// Below are mappings for previously raw JSON structures
#[derive(Debug, Deserialize, Clone)]
pub struct ApiMapNextResponse {
    pub api_no: Option<i32>,
    pub api_color_no: Option<i32>,
    pub api_event_id: Option<i32>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ApiBattleResultResponse {
    pub api_win_rank: Option<String>,
    pub api_get_ship: Option<ApiGetShip>,
    pub api_mvp: Option<i32>,
    pub api_get_base_exp: Option<i32>,
    pub api_get_ship_exp: Option<Vec<i32>>,
    pub api_get_flag: Option<Vec<i32>>,
    pub api_enemy_info: Option<ApiEnemyInfo>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ApiGetShip {
    pub api_ship_id: Option<i32>,
    pub api_ship_name: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ApiEnemyInfo {
    pub api_deck_name: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ApiQuestListResponse {
    pub api_list: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ApiRemodelSlotResponse {
    pub api_remodel_flag: Option<i32>,
    pub api_after_slot: Option<ApiAfterSlot>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ApiAfterSlot {
    pub api_slotitem_id: Option<i32>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ApiHenseiPresetSelectResponse {
    pub api_fleet: Option<serde_json::Value>,
}
