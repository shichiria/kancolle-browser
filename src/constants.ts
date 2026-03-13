/** Centralized localStorage key definitions */
export const STORAGE_KEYS = {
  UI_ZOOM: "ui-zoom",
  SHOW_API_LOG: "show-api-log",
  RAW_API_ENABLED: "raw-api-enabled",
  SHIP_STYPE_FILTERS: "ship-stype-filters",
  EQUIP_TYPE_FILTERS: "equip-type-filters",
  IMPROVEMENT_TYPE_FILTERS: "improvement-type-filters",
  MAP_REC_AREA: "map-rec-area",
  expeditionFleet: (index: number) => `expedition-fleet-${index}`,
  sortieQuestFleet: (index: number) => `sortie-quest-fleet-${index}`,
} as const;

/** Prefix for quests loaded from API without JSON definition data */
export const API_QUEST_PREFIX = "api_";
