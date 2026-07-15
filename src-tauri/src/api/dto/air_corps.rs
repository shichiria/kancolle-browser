use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct AirBaseRequest {
    #[serde(default)]
    pub api_area_id: i32,
    #[serde(default)]
    pub api_base_id: i32,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SetActionRequest {
    #[serde(default)]
    pub api_area_id: i32,
    #[serde(default)]
    pub api_base_id: String,
    #[serde(default)]
    pub api_action_kind: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ChangeNameRequest {
    #[serde(default)]
    pub api_area_id: i32,
    #[serde(default)]
    pub api_base_id: i32,
    #[serde(default)]
    pub api_name: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct AirBaseDistance {
    #[serde(default)]
    pub api_base: i32,
    #[serde(default)]
    pub api_bonus: i32,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct AirBasePlane {
    #[serde(default)]
    pub api_squadron_id: i32,
    #[serde(default)]
    pub api_slotid: i32,
    #[serde(default)]
    pub api_state: i32,
    #[serde(default)]
    pub api_count: i32,
    #[serde(default)]
    pub api_max_count: i32,
    #[serde(default)]
    pub api_cond: i32,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct AirBase {
    #[serde(default)]
    pub api_rid: i32,
    #[serde(default)]
    pub api_area_id: i32,
    #[serde(default)]
    pub api_name: String,
    #[serde(default)]
    pub api_action_kind: i32,
    #[serde(default)]
    pub api_distance: AirBaseDistance,
    #[serde(default)]
    pub api_plane_info: Vec<AirBasePlane>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct MapInfoAirBases {
    #[serde(default)]
    pub api_air_base: Vec<AirBase>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct PlaneUpdate {
    #[serde(default)]
    pub api_distance: AirBaseDistance,
    #[serde(default)]
    pub api_plane_info: Vec<AirBasePlane>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct DeploymentBaseItem {
    #[serde(default)]
    pub api_rid: i32,
    #[serde(default)]
    pub api_distance: AirBaseDistance,
    #[serde(default)]
    pub api_plane_info: Vec<AirBasePlane>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct DeploymentUpdate {
    #[serde(default)]
    pub api_base_items: Vec<DeploymentBaseItem>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct AirAttackStage {
    #[serde(default)]
    pub api_disp_seiku: i32,
    #[serde(default)]
    pub api_f_count: i32,
    #[serde(default)]
    pub api_f_lostcount: i32,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct AirAttackDamage {
    #[serde(default)]
    pub api_edam: Vec<Option<f64>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct AirBaseAttack {
    #[serde(default)]
    pub api_base_id: i32,
    #[serde(default)]
    pub api_stage1: Option<AirAttackStage>,
    #[serde(default)]
    pub api_stage2: Option<AirAttackStage>,
    #[serde(default)]
    pub api_stage3: Option<AirAttackDamage>,
    #[serde(default)]
    pub api_stage3_combined: Option<AirAttackDamage>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct BattleAirBaseAttacks {
    #[serde(default)]
    pub api_air_base_attack: Vec<AirBaseAttack>,
}
