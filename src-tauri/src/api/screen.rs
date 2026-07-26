//! Screen inference driven by intercepted API endpoints.

use tauri::{Emitter, Manager};

/// Map a KanColle API endpoint to the game screen the user is currently on.
/// Ambiguous preload APIs are intentionally omitted to avoid screen flapping.
fn from_api(endpoint: &str) -> Option<crate::ui_event::Screen> {
    use crate::ui_event::Screen;
    match endpoint {
        "/kcsapi/api_start2/getData" => Some(Screen::Unknown),
        "/kcsapi/api_port/port" => Some(Screen::Homeport),

        "/kcsapi/api_req_hensei/change"
        | "/kcsapi/api_req_hensei/preset_select"
        | "/kcsapi/api_req_hensei/lock"
        | "/kcsapi/api_req_hensei/preset_register"
        | "/kcsapi/api_req_hensei/preset_delete"
        | "/kcsapi/api_req_hensei/combined" => Some(Screen::FleetComposition),

        "/kcsapi/api_req_kaisou/powerup"
        | "/kcsapi/api_req_kaisou/remodeling"
        | "/kcsapi/api_req_kaisou/slotset"
        | "/kcsapi/api_req_kaisou/slotset_ex"
        | "/kcsapi/api_req_kaisou/slot_deprive"
        | "/kcsapi/api_req_kaisou/slot_exchange_index"
        | "/kcsapi/api_req_kaisou/preset_slot_select"
        | "/kcsapi/api_req_kaisou/preset_slot_register"
        | "/kcsapi/api_req_kaisou/preset_slot_delete" => Some(Screen::Remodel),

        "/kcsapi/api_req_hokyu/charge" => Some(Screen::Resupply),
        "/kcsapi/api_req_nyukyo/start" | "/kcsapi/api_req_nyukyo/speedchange" => {
            Some(Screen::RepairDockSelect)
        }
        "/kcsapi/api_req_kousyou/createship"
        | "/kcsapi/api_req_kousyou/createship_speedchange"
        | "/kcsapi/api_req_kousyou/destroyship"
        | "/kcsapi/api_req_kousyou/destroyitem2"
        | "/kcsapi/api_req_kousyou/getship" => Some(Screen::Factory),
        "/kcsapi/api_req_kousyou/createitem" => Some(Screen::FactoryDevelop),
        "/kcsapi/api_get_member/questlist"
        | "/kcsapi/api_req_quest/start"
        | "/kcsapi/api_req_quest/stop"
        | "/kcsapi/api_req_quest/clearitemget" => Some(Screen::QuestList),
        "/kcsapi/api_get_member/mapinfo" => Some(Screen::SortieSelect),
        "/kcsapi/api_req_map/start"
        | "/kcsapi/api_req_map/next"
        | "/kcsapi/api_req_sortie/battle"
        | "/kcsapi/api_req_sortie/airbattle"
        | "/kcsapi/api_req_sortie/ld_airbattle"
        | "/kcsapi/api_req_sortie/night_to_day"
        | "/kcsapi/api_req_battle_midnight/battle"
        | "/kcsapi/api_req_battle_midnight/sp_midnight"
        | "/kcsapi/api_req_combined_battle/battle"
        | "/kcsapi/api_req_combined_battle/each_battle"
        | "/kcsapi/api_req_combined_battle/ld_airbattle"
        | "/kcsapi/api_req_combined_battle/each_battle_water"
        | "/kcsapi/api_req_combined_battle/midnight_battle"
        | "/kcsapi/api_req_combined_battle/sp_midnight"
        | "/kcsapi/api_req_sortie/battleresult"
        | "/kcsapi/api_req_combined_battle/battleresult" => Some(Screen::SortieInProgress),
        "/kcsapi/api_get_member/mission" => Some(Screen::ExpeditionSelect),
        _ => None,
    }
}

/// Whether a screen has the common fleet-tab UI.
pub(crate) fn has_fleet_tabs(screen: crate::ui_event::Screen) -> bool {
    use crate::ui_event::Screen;
    matches!(
        screen,
        Screen::FleetComposition
            | Screen::Resupply
            | Screen::Remodel
            | Screen::ExpeditionFleetSelect
    )
}

/// Update tracked screen state from an API URL and emit only actual changes.
pub(crate) fn update_from_api(app: &tauri::AppHandle, endpoint: &str) {
    let Some(new_screen) = from_api(endpoint) else {
        return;
    };
    let state = app.state::<crate::AppState>();
    let mut guard = crate::lock_or_recover(&state.navigation.current_screen, "current_screen");
    if *guard == new_screen {
        return;
    }

    let previous = *guard;
    log::info!(
        "[Screen] API '{}' -> {:?} (was {:?})",
        endpoint,
        new_screen,
        previous
    );
    crate::action_log::log(
        "Screen",
        "api",
        &format!("{:?} -> {:?} via {}", previous, new_screen, endpoint),
    );
    *guard = new_screen;
    drop(guard);
    let _ = app.emit(crate::events::SCREEN_CHANGED, format!("{:?}", new_screen));

    if !has_fleet_tabs(new_screen) {
        let mut fleet = crate::lock_or_recover(&state.navigation.current_fleet, "current_fleet");
        if fleet.is_some() {
            *fleet = None;
            drop(fleet);
            let _ = app.emit(crate::events::FLEET_VIEW_CHANGED, serde_json::Value::Null);
        }
    }

    if new_screen != crate::ui_event::Screen::QuestList {
        let mut period = crate::lock_or_recover(
            &state.navigation.current_quest_period,
            "current_quest_period",
        );
        let mut category = crate::lock_or_recover(
            &state.navigation.current_quest_category,
            "current_quest_category",
        );
        if period.is_some() || category.is_some() {
            *period = None;
            *category = None;
            drop(period);
            drop(category);
            let _ = app.emit(
                crate::events::QUEST_FILTERS_CHANGED,
                serde_json::json!({"period": null, "category": null}),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ambiguous_preload_apis_do_not_change_screen() {
        assert_eq!(from_api("/kcsapi/api_get_member/deck"), None);
        assert_eq!(from_api("/kcsapi/api_get_member/ndock"), None);
    }

    #[test]
    fn execution_apis_identify_their_screen() {
        assert_eq!(
            from_api("/kcsapi/api_req_hensei/change"),
            Some(crate::ui_event::Screen::FleetComposition)
        );
        assert_eq!(
            from_api("/kcsapi/api_req_kousyou/createitem"),
            Some(crate::ui_event::Screen::FactoryDevelop)
        );
    }

    #[test]
    fn sortie_start_leaves_map_selection_screen() {
        assert_eq!(
            from_api("/kcsapi/api_req_map/start"),
            Some(crate::ui_event::Screen::SortieInProgress)
        );
        assert_eq!(
            from_api("/kcsapi/api_req_sortie/battle"),
            Some(crate::ui_event::Screen::SortieInProgress)
        );
    }
}
