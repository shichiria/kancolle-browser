use serde::Serialize;

// =============================================================================
// Land-Based Air Squadron (基地航空隊) state
// =============================================================================

/// Distance the base can strike (api_base + api_bonus from compounded radius equipment).
#[derive(Debug, Clone, Default, Serialize)]
pub struct AirBaseDistance {
    pub base: i32,
    pub bonus: i32,
}

/// One plane slot inside a base squadron.
/// `state == 0` means未配備 (no aircraft assigned). In that case `slotid` is 0
/// and `count` / `max_count` / `cond` are absent in the API.
#[derive(Debug, Clone, Default, Serialize)]
pub struct AirBasePlane {
    /// 1-4
    pub squadron_id: i32,
    /// Equipment instance ID (0 = empty slot)
    pub slotid: i32,
    /// 0=未配備, 1=配備済, 2=未補給/装備変更中
    pub state: i32,
    pub count: i32,
    pub max_count: i32,
    pub cond: i32,
    /// Master equipment name (e.g. "九六式陸攻")
    pub name: Option<String>,
    /// Master slotitem id (api_slotitem_id of the equipment)
    pub slotitem_id: Option<i32>,
    /// Improvement level (★ 0-10)
    pub level: Option<i32>,
    /// Aircraft proficiency (>> 0-7)
    pub alv: Option<i32>,
    /// Icon type from master api_type[3]
    pub icon_type: Option<i32>,
}

/// One LBAS attack wave parsed from `api_air_base_attack[]` of a battle response.
/// One sortie typically produces 2 waves per dispatched base.
#[derive(Debug, Clone, Default, Serialize)]
pub struct AirBaseAttackWave {
    /// 1-based wave number for this base in the current sortie
    pub wave: i32,
    /// Game-side enum: 0=均衡 / 1=確保 / 2=優勢 / 3=劣勢 / 4=喪失
    pub disp_seiku: i32,
    /// Sum of squadron plane counts that launched (e.g. 4×18 = 72)
    pub f_count: i32,
    /// Stage1 (制空戦) — total planes lost across all 4 squadrons
    pub stage1_lost: i32,
    /// Stage2 (敵対空) — total planes lost across all 4 squadrons
    pub stage2_lost: i32,
    /// Total damage dealt to enemy ships in stage3 (sum of api_edam)
    pub edam_total: i32,
    /// Per-squadron loss after distributing total_lost proportionally to slot.
    /// Length matches the 4 squadrons (squadron_id 1-4).
    pub per_squadron_lost: Vec<i32>,
}

/// One Land-Based Air Squadron base (一隊). Each base has 4 plane squadrons.
#[derive(Debug, Clone, Default, Serialize)]
pub struct AirBase {
    /// Base ID within an area (1, 2, 3)
    pub rid: i32,
    /// World/area ID (6 = 6-x maps, 7 = 7-x, 21+ = event maps)
    pub area_id: i32,
    /// Display name (e.g. "第一基地航空隊"). Editable via change_name API.
    pub name: String,
    /// 0=待機, 1=出撃, 2=防空, 3=退避, 4=休息
    pub action_kind: i32,
    pub distance: AirBaseDistance,
    /// 4 squadrons. Always exactly 4 entries even when some are未配備.
    pub planes: Vec<AirBasePlane>,
    /// Latest sortie's LBAS attack waves. Cleared on `api_req_map/start`,
    /// appended on each battle whose response contains `api_air_base_attack[]`.
    /// Plane counts are decremented in place using actual API losses distributed
    /// proportionally across the 4 squadrons (mean of the simulator distribution).
    pub recent_attacks: Vec<AirBaseAttackWave>,
}
