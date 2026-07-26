use super::*;

// ── Homeport ──────────────────────────────────────────────────────────

// Calibrated from observed user clicks (2026-05-05):
//   編成 (258, 249), 改装 (493, 358), 入渠 (199, 558), 工廠 (394, 590)
#[test]
fn homeport_click_hensei_button() {
    let event = detect_event(Screen::Homeport, 258, 249);
    assert_eq!(
        event,
        UiEvent::Navigate {
            target: "編成".to_string()
        }
    );
}

#[test]
fn homeport_click_kaisou_button() {
    let event = detect_event(Screen::Homeport, 493, 358);
    assert_eq!(
        event,
        UiEvent::Navigate {
            target: "改装".to_string()
        }
    );
}

#[test]
fn homeport_click_nyukyo_button() {
    let event = detect_event(Screen::Homeport, 199, 558);
    assert_eq!(
        event,
        UiEvent::Navigate {
            target: "入渠".to_string()
        }
    );
}

#[test]
fn homeport_click_factory_button() {
    let event = detect_event(Screen::Homeport, 394, 590);
    assert_eq!(
        event,
        UiEvent::Navigate {
            target: "工廠".to_string()
        }
    );
}

#[test]
fn homeport_click_secretary_area_is_unknown() {
    let event = detect_event(Screen::Homeport, 800, 400);
    assert_eq!(event, UiEvent::UnknownClick { x: 800, y: 400 });
}

// ── Side menu (common) ───────────────────────────────────────────────

#[test]
fn side_menu_hensei() {
    let event = detect_event(Screen::Resupply, 30, 150);
    assert_eq!(
        event,
        UiEvent::SideMenuClick {
            target: "編成".to_string()
        }
    );
}

#[test]
fn side_menu_supply() {
    let event = detect_event(Screen::FleetComposition, 30, 210);
    assert_eq!(
        event,
        UiEvent::SideMenuClick {
            target: "補給".to_string()
        }
    );
}

#[test]
fn side_menu_not_triggered_when_x_too_large() {
    let event = detect_event(Screen::FleetComposition, 100, 150);
    // x=100 is outside side menu (max 65), so should NOT be side menu
    assert_ne!(
        event,
        UiEvent::SideMenuClick {
            target: "編成".to_string()
        }
    );
}

// ── Expedition ────────────────────────────────────────────────────────

#[test]
fn expedition_tab_chinjufu() {
    let event = detect_event(Screen::ExpeditionSelect, 216, 713);
    assert_eq!(
        event,
        UiEvent::ExpeditionTab {
            area: "鎮守府海域".to_string()
        }
    );
}

#[test]
fn expedition_tab_nansei() {
    let event = detect_event(Screen::ExpeditionSelect, 287, 706);
    assert_eq!(
        event,
        UiEvent::ExpeditionTab {
            area: "南西諸島海域".to_string()
        }
    );
}

#[test]
fn expedition_tab_hokutou() {
    let event = detect_event(Screen::ExpeditionSelect, 339, 702);
    assert_eq!(
        event,
        UiEvent::ExpeditionTab {
            area: "北方海域".to_string()
        }
    );
}

#[test]
fn expedition_tab_nanpou() {
    let event = detect_event(Screen::ExpeditionSelect, 466, 706);
    assert_eq!(
        event,
        UiEvent::ExpeditionTab {
            area: "南方海域".to_string()
        }
    );
}

#[test]
fn expedition_tab_chubu() {
    let event = detect_event(Screen::ExpeditionSelect, 577, 704);
    assert_eq!(
        event,
        UiEvent::ExpeditionTab {
            area: "中部海域".to_string()
        }
    );
}

#[test]
fn expedition_select_row_1() {
    let event = detect_event(Screen::ExpeditionSelect, 400, 185);
    assert_eq!(event, UiEvent::ExpeditionSelect { row: 1 });
}

#[test]
fn expedition_select_row_5() {
    let event = detect_event(Screen::ExpeditionSelect, 400, 355);
    assert_eq!(event, UiEvent::ExpeditionSelect { row: 5 });
}

// ── Fleet Composition ────────────────────────────────────────────────

// Calibrated from user-confirmed clicks (2026-05-05):
//   第2 (244, 208), 第3 (291, 202), 第4 (336, 204) → fleet tabs at y≈200
#[test]
fn fleet_select_tab_1() {
    let event = detect_event(Screen::FleetComposition, 197, 200);
    assert_eq!(event, UiEvent::FleetSelect { fleet: 1 });
}

#[test]
fn fleet_select_tab_3() {
    let event = detect_event(Screen::FleetComposition, 291, 202);
    assert_eq!(event, UiEvent::FleetSelect { fleet: 3 });
}

#[test]
fn fleet_change_start_slot_2_right_side() {
    // Slot 2 = right column, row 1 → change button on right half
    let event = detect_event(Screen::FleetComposition, 900, 200);
    assert_eq!(event, UiEvent::FleetChangeStart { slot: 2 });
}

#[test]
fn fleet_ship_detail_slot_1() {
    // Slot 1 = left column, row 1, left half. y must be below the fleet-tab
    // strip (≤235) but within row 1 (160-290) and x outside fleet-tab x-range.
    let event = detect_event(Screen::FleetComposition, 200, 260);
    assert_eq!(event, UiEvent::ShipDetail { slot: 1 });
}

#[test]
fn fleet_change_empty_slot() {
    // Empty slot "変更" button right edge
    let event = detect_event(Screen::FleetComposition, 1100, 360);
    assert_eq!(event, UiEvent::FleetChangeStart { slot: 4 });
}

// ── Ship Selection ───────────────────────────────────────────────────

#[test]
fn ship_filter_change() {
    let event = detect_event(Screen::ShipSelection, 1137, 195);
    assert_eq!(event, UiEvent::ShipFilterChange);
}

#[test]
fn ship_select_from_list() {
    let event = detect_event(Screen::ShipSelection, 700, 300);
    assert_eq!(event, UiEvent::ShipSelect { row: 6 });
}

#[test]
fn ship_change_confirm_button() {
    let event = detect_event(Screen::ShipChangeConfirm, 1066, 708);
    assert_eq!(event, UiEvent::FleetChangeConfirm);
}

// ── Resupply ─────────────────────────────────────────────────────────

#[test]
fn supply_fleet_select_1() {
    let event = detect_event(Screen::Resupply, 90, 110);
    assert_eq!(event, UiEvent::SupplyFleetSelect { fleet: 1 });
}

#[test]
fn supply_execute() {
    let event = detect_event(Screen::Resupply, 1050, 700);
    assert_eq!(event, UiEvent::SupplyExecute);
}

// ── Factory ──────────────────────────────────────────────────────────

#[test]
fn factory_select_develop() {
    let event = detect_event(Screen::Factory, 200, 330);
    assert_eq!(
        event,
        UiEvent::FactorySelect {
            mode: "開発".to_string()
        }
    );
}

#[test]
fn factory_develop_start() {
    let event = detect_event(Screen::FactoryDevelop, 1100, 660);
    assert_eq!(event, UiEvent::DevelopStart);
}

// ── Quest ────────────────────────────────────────────────────────────

#[test]
fn quest_filter_daily() {
    // Calibrated 2026-05-05: Daily filter at y≈330 (was y=180-200 in stale docs)
    let event = detect_event(Screen::QuestList, 100, 330);
    assert_eq!(
        event,
        UiEvent::QuestFilter {
            filter: "Daily".to_string()
        }
    );
}

#[test]
fn quest_select_row_2() {
    let event = detect_event(Screen::QuestList, 600, 250);
    assert_eq!(event, UiEvent::QuestSelect { row: 2 });
}

// ── GET Screen ───────────────────────────────────────────────────────

#[test]
fn get_screen_dismiss() {
    let event = detect_event(Screen::GetScreen, 1130, 670);
    assert_eq!(event, UiEvent::GetScreenDismiss);
}

// ── Real click log validation ────────────────────────────────────────
// Tests using actual click coordinates from the user's session

#[test]
fn real_session_homeport_to_hensei() {
    // 2026-05-05 user-confirmed: x=258 y=249 — clicked 編成 on homeport
    // (older docs at y≈115 were inaccurate for the user's actual UI)
    let event = detect_event(Screen::Homeport, 258, 249);
    assert_eq!(
        event,
        UiEvent::Navigate {
            target: "編成".to_string()
        }
    );
}

#[test]
fn real_session_fleet_change_start() {
    // 15:16:46 x=1101 y=360 — clicked 変更 on empty slot in fleet composition
    let event = detect_event(Screen::FleetComposition, 1101, 360);
    assert_eq!(event, UiEvent::FleetChangeStart { slot: 4 });
}

#[test]
fn real_session_ship_filter() {
    // 15:16:49 x=1137 y=195 — clicked filter tab in ship selection
    let event = detect_event(Screen::ShipSelection, 1137, 195);
    assert_eq!(event, UiEvent::ShipFilterChange);
}

#[test]
fn real_session_ship_select() {
    // 15:17:29 x=788 y=558 — clicked a ship in the list (綾波改二)
    let event = detect_event(Screen::ShipSelection, 788, 558);
    assert_eq!(event, UiEvent::ShipSelect { row: 14 });
}

#[test]
fn real_session_ship_change_confirm() {
    // 15:18:21 x=1066 y=708 — clicked 変更 button to confirm
    let event = detect_event(Screen::ShipChangeConfirm, 1066, 708);
    assert_eq!(event, UiEvent::FleetChangeConfirm);
}

#[test]
fn real_session_expedition_tab_chinjufu() {
    // 14:59:42 x=287 y=706 — clicked 南西諸島 tab (from 鎮守府)
    let event = detect_event(Screen::ExpeditionSelect, 287, 706);
    assert_eq!(
        event,
        UiEvent::ExpeditionTab {
            area: "南西諸島海域".to_string()
        }
    );
}

#[test]
fn event_map_airbase_button_leaves_map_selection() {
    let event = detect_event(Screen::SortieSelect, 350, 610);
    assert_eq!(event, UiEvent::OpenAirBaseSupply);
}

#[test]
fn sortie_mode_menu_does_not_mistake_lower_left_for_airbase_button() {
    let event = detect_event(Screen::SortieMenu, 350, 610);
    assert_eq!(event, UiEvent::UnknownClick { x: 350, y: 610 });
}
