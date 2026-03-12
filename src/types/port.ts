// Port / Homeport related types

export interface SpecialEquip {
  name: string;
  icon_type: number;
}

export interface ShipData {
  name: string;
  lv: number;
  hp: number;
  maxhp: number;
  cond: number;
  fuel: number;
  bull: number;
  damecon_name?: string | null;
  command_facility_name?: string | null;
  special_equips: SpecialEquip[];
  can_opening_asw?: boolean;
  soku: number;
}

export interface FleetExpedition {
  mission_name: string;
  return_time: number; // unix timestamp in ms, 0 = not on expedition
}

export interface FleetData {
  id: number;
  name: string;
  expedition?: FleetExpedition | null;
  ships: ShipData[];
  // Legacy fields
  ship_ids?: number[];
  mission?: unknown[];
}

export interface NdockData {
  id: number;
  state: number;
  ship_name?: string;
  ship_id?: number;
  complete_time: number;
}

export interface PortData {
  admiral_name: string;
  admiral_level: number;
  admiral_rank?: number;
  ship_count: number;
  ship_capacity?: number;
  // Resources
  fuel: number;
  ammo: number;
  steel: number;
  bauxite: number;
  instant_repair?: number;
  instant_build?: number;
  dev_material?: number;
  improvement_material?: number;
  // Fleets
  fleets: FleetData[];
  // Repair docks
  ndock: NdockData[];
}

export interface ApiLogEntry {
  time: string;
  endpoint: string;
}
