// Ship list related types

export interface ShipListItem {
  id: number;
  ship_id: number;
  name: string;
  stype: number;
  stype_name: string;
  lv: number;
  hp: number;
  maxhp: number;
  cond: number;
  firepower: number;
  torpedo: number;
  aa: number;
  armor: number;
  asw: number;
  evasion: number;
  los: number;
  luck: number;
  locked: boolean;
  /** 出撃札 (api_sally_area): 0 = 札なし, N = 札N */
  sally_area: number;
}

export interface ShipListResponse {
  ships: ShipListItem[];
  stypes: [number, string][];
}

export type ShipSortKey = "lv" | "name" | "stype" | "firepower" | "torpedo" | "aa" | "armor" | "asw" | "evasion" | "los" | "luck" | "cond" | "locked" | "sally";
