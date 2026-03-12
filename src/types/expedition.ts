// Expedition and condition check related types

import type { ConditionResult } from "./common";

export interface ExpeditionDef {
  id: number;
  display_id: string;
  name: string;
  great_success_type: "Regular" | "Drum" | "Level";
  duration_minutes: number;
}

export interface ExpeditionCheckResult {
  expedition_id: number;
  expedition_name: string;
  display_id: string;
  result: "Failure" | "Success" | "GreatSuccess";
  conditions: ConditionResult[];
}

export interface MapRecommendedResult {
  area: string;
  satisfied: boolean;
  conditions: ConditionResult[];
}

export interface MapRecommendationDef {
  area: string;
  name: string;
}

export interface MapRouteCheckResult {
  desc: string;
  satisfied: boolean;
  conditions: ConditionResult[];
}

export interface MapRecommendationCheckResult {
  area: string;
  name: string;
  routes: MapRouteCheckResult[];
}
