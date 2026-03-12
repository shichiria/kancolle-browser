// Improvement related types

export interface ConsumedEquipInfo {
  eq_id: number;
  name: string;
  counts: [number, number, number]; // [p1(★0-5), p2(★6-9), conv(更新)]
  owned: number; // unlocked count
}

export interface ImprovementItem {
  eq_id: number;
  name: string;
  eq_type: number;
  type_name: string;
  sort_value: number;
  available_today: boolean;
  today_helpers: string[];
  matches_secretary: boolean;
  previously_improved: boolean;
  consumed_equips: ConsumedEquipInfo[];
}

export interface ImprovementListResponse {
  items: ImprovementItem[];
  day_of_week: number;
  secretary_ship: string;
}
