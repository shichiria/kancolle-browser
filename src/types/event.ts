export interface EventMapStatus {
  map_id: number;
  gauge_num?: number;
  gauge_type?: number;
  current_hp?: number;
  max_hp?: number;
  selected_rank?: number;
  state?: number;
  cleared: boolean;
  provisional: boolean;
}
