/** Centralized localStorage key definitions */
export const STORAGE_KEYS = {
  UI_ZOOM: "ui-zoom",
  SHOW_API_LOG: "show-api-log",
  RAW_API_ENABLED: "raw-api-enabled",
  SHIP_STYPE_FILTERS: "ship-stype-filters",
  EQUIP_TYPE_FILTERS: "equip-type-filters",
  IMPROVEMENT_TYPE_FILTERS: "improvement-type-filters",
  MAP_REC_AREA: "map-rec-area",
  EVENT_E3_PROGRESS: "event-e3-progress-2026-summer",
  expeditionFleet: (index: number) => `expedition-fleet-${index}`,
  sortieQuestFleet: (index: number) => `sortie-quest-fleet-${index}`,
} as const;

/** Tauri event names. Keep in sync with `src-tauri/src/events.rs`. */
export const EVENTS = {
  PROXY_READY: "proxy-ready",
  KANCOLLE_API: "kancolle-api",
  PORT_DATA: "port-data",
  SORTIE_UPDATE: "sortie-update",
  SORTIE_COMPLETE: "sortie-complete",
  SENKA_UPDATED: "senka-updated",
  FLEET_UPDATED: "fleet-updated",
  QUEST_LIST_UPDATED: "quest-list-updated",
  QUEST_STARTED: "quest-started",
  QUEST_STOPPED: "quest-stopped",
  QUEST_PROGRESS_UPDATED: "quest-progress-updated",
  QUEST_FILTERS_CHANGED: "quest-filters-changed",
  SCREEN_CHANGED: "screen-changed",
  FLEET_VIEW_CHANGED: "fleet-view-changed",
  AIR_BASE_UPDATED: "air-base-updated",
  EVENT_MAP_UPDATED: "event-map-updated",
  EVENT_WINDOW_OPENED: "event-window-opened",
  DRIVE_SYNC_STATUS: "drive-sync-status",
  DRIVE_DATA_UPDATED: "drive-data-updated",
  CLICK_EVENT: "click-event",
  CLICK_SCREENSHOT: "click-screenshot",
} as const;

/** Prefix for quests loaded from API without JSON definition data */
export const API_QUEST_PREFIX = "api_";
