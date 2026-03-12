// Equipment list related types

export interface EquipListItem {
  master_id: number;
  name: string;
  type_id: number;
  type_name: string;
  icon_type: number;
  total_count: number;
  locked_count: number;
  improvements: [number, number][]; // [level, count]
}

export interface EquipListResponse {
  items: EquipListItem[];
  equip_types: [number, string][];
}
