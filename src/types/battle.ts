// Battle related types

export interface HpState {
  before: number;
  after: number;
  max: number;
}

export interface SlotItemSnapshot {
  id: number;
  rf?: number;
  mas?: number;
}

export interface EnemyShip {
  ship_id: number;
  level: number;
  name?: string;
  slots?: number[];
}

export interface AirBattleResult {
  air_superiority?: number;
  friendly_plane_count?: [number, number];
  enemy_plane_count?: [number, number];
}

export interface BattleDetail {
  rank: string;
  enemy_name: string;
  enemy_ships: EnemyShip[];
  formation: [number, number, number];
  air_battle?: AirBattleResult;
  friendly_hp: HpState[];
  enemy_hp: HpState[];
  drop_ship?: string;
  drop_ship_id?: number;
  mvp?: number;
  base_exp?: number;
  ship_exp: number[];
  night_battle: boolean;
}

export interface BattleNode {
  cell_no: number;
  event_kind: number;
  event_id?: number;
  battle?: BattleDetail;
  // Legacy fields (from old saved records, migrated on load)
  rank?: string;
  enemy_name?: string;
  drop_ship?: string;
  drop_ship_id?: number;
  mvp?: number;
  base_exp?: number;
}

export interface SortieShip {
  name: string;
  ship_id: number;
  lv: number;
  stype: number;
  slots?: SlotItemSnapshot[];
  slot_ex?: SlotItemSnapshot;
}

export interface SortieRecord {
  id: string;
  fleet_id: number;
  map_display: string;
  ships: SortieShip[];
  nodes: BattleNode[];
  start_time: string;
  end_time?: string;
}

export interface BattleLogsResponse {
  records: SortieRecord[];
  total: number;
}

export interface MapSpot {
  no: number;
  x: number;
  y: number;
  line?: { x: number; y: number; img?: string };
}

export interface MapInfo {
  bg: string[];
  spots: MapSpot[];
}

export interface AtlasFrame {
  frame: { x: number; y: number; w: number; h: number };
}

export interface MapSprites {
  bg?: string;         // terrain background
  point?: string;      // cell markers overlay (red dots)
  routes: { uri: string; x: number; y: number; w: number; h: number; spotNo: number; isVisited?: boolean }[]; // route connection sprites
}
