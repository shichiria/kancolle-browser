export interface AirBaseDistance {
  base: number;
  bonus: number;
}

export interface AirBasePlane {
  squadron_id: number;
  /// Equipment instance ID (0 = empty slot, no aircraft assigned)
  slotid: number;
  /// 0=未配備, 1=配備済, 2=未補給
  state: number;
  count: number;
  max_count: number;
  cond: number;
  /// Resolved master equipment name (e.g. "九六式陸攻")
  name?: string | null;
  /// Master slotitem id
  slotitem_id?: number | null;
  /// Improvement level (★ 0-10)
  level?: number | null;
  /// Aircraft proficiency (>> 0-7)
  alv?: number | null;
  /// Icon type from master api_type[3]
  icon_type?: number | null;
}

export interface AirBaseAttackWave {
  /// 1-based wave number for this base in the current sortie
  wave: number;
  /// Game enum: 0=劣勢, 1=優勢, 2=確保, 3=均衡, 4=喪失
  disp_seiku: number;
  /// Total planes that launched (e.g. 4×18 = 72)
  f_count: number;
  /// Planes lost in stage1 (制空戦)
  stage1_lost: number;
  /// Planes lost in stage2 (敵対空)
  stage2_lost: number;
  /// Damage dealt to enemy in stage3
  edam_total: number;
  /// Per-squadron loss (squadron_id 1..=4 order, length 4)
  per_squadron_lost: number[];
}

export interface AirBase {
  /// Base ID within an area (1, 2, 3)
  rid: number;
  /// World/area ID (6 = 6-x, 7 = 7-x, 21+ = event)
  area_id: number;
  name: string;
  /// 0=待機, 1=出撃, 2=防空, 3=退避, 4=休息
  action_kind: number;
  distance: AirBaseDistance;
  planes: AirBasePlane[];
  /// Latest sortie's wave-by-wave attack history. Cleared on next sortie start.
  recent_attacks: AirBaseAttackWave[];
}
