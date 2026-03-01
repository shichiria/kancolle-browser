use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct HenseiChangeReq {
    pub api_id: usize,
    pub api_ship_idx: i32,
    pub api_ship_id: i32,
}

#[derive(Debug, Deserialize)]
pub struct RemodelSlotReq {
    pub api_slot_id: i32,
    pub api_id: i32, // The master eq_id requested to remodel
}

#[derive(Debug, Deserialize)]
pub struct QuestReq {
    pub api_quest_id: i32,
}
