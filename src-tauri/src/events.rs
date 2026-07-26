//! Tauri event-name registry. Keep in sync with `src/constants.ts::EVENTS`.

pub const PROXY_READY: &str = "proxy-ready";
pub const KANCOLLE_API: &str = "kancolle-api";
pub const PORT_DATA: &str = "port-data";
pub const SORTIE_UPDATE: &str = "sortie-update";
pub const SORTIE_COMPLETE: &str = "sortie-complete";
pub const SENKA_UPDATED: &str = "senka-updated";
pub const FLEET_UPDATED: &str = "fleet-updated";
pub const QUEST_LIST_UPDATED: &str = "quest-list-updated";
pub const QUEST_STARTED: &str = "quest-started";
pub const QUEST_STOPPED: &str = "quest-stopped";
pub const QUEST_PROGRESS_UPDATED: &str = "quest-progress-updated";
pub const QUEST_FILTERS_CHANGED: &str = "quest-filters-changed";
pub const SCREEN_CHANGED: &str = "screen-changed";
pub const FLEET_VIEW_CHANGED: &str = "fleet-view-changed";
pub const AIR_BASE_UPDATED: &str = "air-base-updated";
pub const EVENT_MAP_UPDATED: &str = "event-map-updated";
pub const EVENT_WINDOW_OPENED: &str = "event-window-opened";
pub const DRIVE_SYNC_STATUS: &str = "drive-sync-status";
pub const DRIVE_DATA_UPDATED: &str = "drive-data-updated";
pub const CLICK_EVENT: &str = "click-event";
#[cfg(debug_assertions)]
pub const CLICK_SCREENSHOT: &str = "click-screenshot";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frontend_event_registry_stays_in_sync() {
        let frontend = include_str!("../../src/constants.ts");
        for event in [
            PROXY_READY,
            KANCOLLE_API,
            PORT_DATA,
            SORTIE_UPDATE,
            SORTIE_COMPLETE,
            SENKA_UPDATED,
            FLEET_UPDATED,
            QUEST_LIST_UPDATED,
            QUEST_STARTED,
            QUEST_STOPPED,
            QUEST_PROGRESS_UPDATED,
            QUEST_FILTERS_CHANGED,
            SCREEN_CHANGED,
            FLEET_VIEW_CHANGED,
            AIR_BASE_UPDATED,
            EVENT_MAP_UPDATED,
            EVENT_WINDOW_OPENED,
            DRIVE_SYNC_STATUS,
            DRIVE_DATA_UPDATED,
            CLICK_EVENT,
            CLICK_SCREENSHOT,
        ] {
            assert!(
                frontend.contains(&format!("\"{event}\"")),
                "frontend event registry is missing {event}"
            );
        }
    }
}
