// Quest related types

import type { ConditionResult } from "./common";
import type { MapRecommendedResult } from "./expedition";

export interface SortieQuestDef {
  id: number;
  quest_id: string;
  name: string;
  area: string;
  rank: string;
  boss_only: boolean;
  count: number;
  reset: string;
  no_conditions: boolean;
  sub_goals?: { name: string; count: number; boss_only: boolean; rank: string }[];
}

export interface ActiveQuestDetail {
  id: number;
  title: string;
  category: number;
}

export interface SortieQuestCheckResult {
  quest_id: string;
  quest_name: string;
  area: string;
  rank: string;
  boss_only: boolean;
  count: number;
  no_conditions: boolean;
  note: string | null;
  satisfied: boolean;
  conditions: ConditionResult[];
  recommended: MapRecommendedResult[];
}

export interface QuestProgressSummary {
  quest_id: number;
  quest_id_str: string;
  area_progress: { area: string; cleared: boolean; count: number; count_max: number }[];
  count: number;
  count_max: number;
  completed: boolean;
}

export interface DropdownQuest {
  key: string;       // quest_id (from JSON) or api_no as string
  label: string;     // display name
  category: number;
  hasData: boolean;   // true if conditions data exists in sortie_quests.json
}
