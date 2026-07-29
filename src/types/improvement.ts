// Improvement related types

export interface ConsumedEquipInfo {
  eq_id: number;
  name: string;
  counts: [number, number, number]; // [p1(★0-5), p2(★6-9), conv(更新)]
  owned: number; // total count, including locked equipment
}

/** A ship that can act as today's 担当艦 (2nd-slot helper) for an improvement. */
export interface ImprovementHelperShip {
  name: string;
  /** Highest level among the owned copies; null if not owned. */
  level: number | null;
}

export interface ImprovementItem {
  eq_id: number;
  name: string;
  owned_count: number;
  owned_levels: Array<[number, number]>;
  equipment_ready: boolean;
  can_improve_now: boolean;
  eq_type: number;
  type_name: string;
  sort_value: number;
  available_today: boolean;
  today_helpers: ImprovementHelperShip[];
  matches_secretary: boolean;
  previously_improved: boolean;
  consumed_equips: ConsumedEquipInfo[];
}

export interface ImprovementListResponse {
  items: ImprovementItem[];
  day_of_week: number;
  secretary_ship: string;
}
