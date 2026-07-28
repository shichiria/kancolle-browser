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
    let event = detect_event(Screen::Resupply, 30, 225);
    assert_eq!(
        event,
        UiEvent::SideMenuClick {
            target: "編成".to_string()
        }
    );
}

#[test]
fn side_menu_supply() {
    let event = detect_event(Screen::FleetComposition, 30, 315);
    assert_eq!(
        event,
        UiEvent::SideMenuClick {
            target: "補給".to_string()
        }
    );
}

#[test]
fn side_menu_not_triggered_when_x_too_large() {
    let event = detect_event(Screen::FleetComposition, 100, 225);
    // x=100 is outside side menu (max 75), so should NOT be side menu
    assert_ne!(
        event,
        UiEvent::SideMenuClick {
            target: "編成".to_string()
        }
    );
}

#[test]
fn real_fullscreen_side_menu_positions() {
    assert_eq!(
        detect_event(Screen::Resupply, 30, 390),
        UiEvent::SideMenuClick {
            target: "改装".to_string()
        }
    );
    assert_eq!(
        detect_event(Screen::Remodel, 30, 480),
        UiEvent::SideMenuClick {
            target: "入渠".to_string()
        }
    );
    assert_eq!(
        detect_event(Screen::RepairDockSelect, 30, 565),
        UiEvent::SideMenuClick {
            target: "工廠".to_string()
        }
    );
}

#[test]
fn top_menu_quest_uses_full_width_position() {
    assert_eq!(
        detect_event(Screen::Homeport, 817, 81),
        UiEvent::TopMenuClick {
            target: "任務".to_string()
        }
    );
}

// ── GAME START / item and furniture screens ─────────────────────────

#[test]
fn game_start_button_enters_port() {
    assert_eq!(INITIAL_SCREEN, Screen::GameStart);
    assert_eq!(
        detect_event(Screen::GameStart, 972, 607),
        UiEvent::StartGame
    );
    assert_eq!(
        detect_event(Screen::GameStart, 100, 50),
        UiEvent::UnknownClick { x: 100, y: 50 }
    );
}

#[test]
fn observed_item_screen_flow() {
    assert_eq!(
        detect_event(Screen::ItemListHeld, 409, 196),
        UiEvent::ItemInventoryTab {
            tab: "購入済みアイテム".to_string()
        }
    );
    assert_eq!(
        detect_event(Screen::ItemListHeld, 85, 283),
        UiEvent::ItemMenuSelect {
            target: "アイテム屋".to_string()
        }
    );
    assert_eq!(
        detect_event(Screen::ItemShopRegular, 107, 333),
        UiEvent::ItemMenuSelect {
            target: "家具屋".to_string()
        }
    );
    assert_eq!(
        detect_event(Screen::FurnitureShopCategory, 98, 680),
        UiEvent::ItemReturnHomeport
    );
}

#[test]
fn observed_item_inventory_subscreens() {
    assert_eq!(
        detect_event(Screen::ItemListHeld, 767, 214),
        UiEvent::ItemExpansionOpen
    );
    assert_eq!(
        detect_event(Screen::ItemListExpansion, 276, 215),
        UiEvent::ItemInventoryTab {
            tab: "保有アイテム".to_string()
        }
    );
    assert_eq!(
        detect_event(Screen::ItemListHeld, 454, 164),
        UiEvent::ItemInventoryTab {
            tab: "購入済みアイテム".to_string()
        }
    );
    assert_eq!(
        detect_event(Screen::ItemListPurchased, 309, 181),
        UiEvent::ItemInventoryTab {
            tab: "保有アイテム".to_string()
        }
    );
}

#[test]
fn observed_item_shop_corner_switches() {
    assert_eq!(
        detect_event(Screen::ItemShopRegular, 1065, 688),
        UiEvent::ItemShopCornerSwitch {
            corner: "特選コーナー".to_string()
        }
    );
    assert_eq!(
        detect_event(Screen::ItemShopSpecial, 281, 684),
        UiEvent::ItemShopCornerSwitch {
            corner: "レギュラーコーナー".to_string()
        }
    );
    assert_eq!(
        detect_event(Screen::ItemShopRegular, 740, 689),
        UiEvent::UnknownClick { x: 740, y: 689 }
    );
}

#[test]
fn observed_furniture_category_flow() {
    assert_eq!(
        detect_event(Screen::FurnitureShopCategory, 334, 274),
        UiEvent::FurnitureCategorySelect {
            category: "壁紙".to_string()
        }
    );
    assert_eq!(
        detect_event(Screen::FurnitureShopCategory, 408, 414),
        UiEvent::FurnitureCategorySelect {
            category: "椅子+机".to_string()
        }
    );
    assert_eq!(
        detect_event(Screen::FurnitureShopList, 347, 686),
        UiEvent::FurnitureListBack
    );
}

#[test]
fn observed_furniture_change_flow() {
    assert_eq!(
        detect_event(Screen::FurnitureChange, 172, 60),
        UiEvent::FurnitureChangeCategory {
            category: "壁紙".to_string()
        }
    );
    assert_eq!(
        detect_event(Screen::FurnitureChange, 142, 128),
        UiEvent::FurnitureChangeCategory {
            category: "床".to_string()
        }
    );
    assert_eq!(
        detect_event(Screen::FurnitureChange, 98, 247),
        UiEvent::FurnitureChangeCategory {
            category: "窓枠+カーテン".to_string()
        }
    );
    assert_eq!(
        detect_event(Screen::FurnitureChange, 108, 553),
        UiEvent::FurnitureChangeOpenShop
    );
    assert_eq!(
        detect_event(Screen::FurnitureChange, 100, 680),
        UiEvent::FurnitureChangeReturnHomeport
    );
}

#[test]
fn observed_remodel_equipment_flow() {
    assert_eq!(
        detect_event(Screen::Remodel, 570, 309),
        UiEvent::RemodelEquipmentSlot { slot: 2 }
    );
    assert_eq!(
        detect_event(Screen::RemodelEquipmentSelect, 793, 169),
        UiEvent::RemodelEquipmentFilterOpen
    );
    assert_eq!(
        detect_event(Screen::RemodelEquipmentFilter, 833, 285),
        UiEvent::RemodelEquipmentCategorySelect
    );
    assert_eq!(
        detect_event(Screen::RemodelEquipmentSelect, 803, 446),
        UiEvent::RemodelEquipmentSelect { row: 6 }
    );
    assert_eq!(
        detect_event(Screen::RemodelEquipmentConfirm, 1134, 667),
        UiEvent::RemodelEquipmentChangeConfirm
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
    // 2026-07-28 23:23:04.384 — observed click that opened the panel.
    let event = detect_event(Screen::SortieSelectEvent, 377, 579);
    assert_eq!(event, UiEvent::OpenAirBaseSupply);
}

#[test]
fn observed_airbase_tabs_select_all_three_bases() {
    let samples = [(930, 190, 1), (1041, 192, 2), (1154, 185, 3)];

    for (x, y, base) in samples {
        assert_eq!(
            detect_event(Screen::AirBaseSupply1, x, y),
            UiEvent::AirBaseSelect { base },
            "base {base} at ({x}, {y})"
        );
    }
}

#[test]
fn sortie_mode_menu_does_not_mistake_lower_left_for_airbase_button() {
    let event = detect_event(Screen::SortieMenu, 350, 610);
    assert_eq!(event, UiEvent::UnknownClick { x: 350, y: 610 });
}

#[test]
fn observed_sortie_menu_button_opens_map_selection() {
    assert_eq!(
        detect_event(Screen::SortieMenu, 354, 414),
        UiEvent::SelectMode {
            mode: "出撃".to_string()
        }
    );
}

#[test]
fn observed_sortie_area_tabs_are_distinct() {
    let samples = [
        (1098, 666, "期間限定海域"),
        (806, 662, "中部海域"),
        (692, 676, "南方海域"),
        (585, 677, "西方海域"),
        (476, 658, "南西海域"),
        (387, 659, "北方海域"),
        (279, 681, "南西諸島海域"),
        (227, 673, "鎮守府海域"),
    ];

    for (x, y, area) in samples {
        assert_eq!(
            detect_event(Screen::SortieSelectChinjufu, x, y),
            UiEvent::SortieAreaSelect {
                area: area.to_string()
            },
            "sample ({x}, {y})"
        );
    }
}

#[test]
fn southwest_area_tab_is_not_airbase_supply() {
    assert_eq!(
        detect_event(Screen::SortieSelectWestern, 476, 658),
        UiEvent::SortieAreaSelect {
            area: "南西海域".to_string()
        }
    );
    assert_eq!(
        detect_event(Screen::SortieSelectWestern, 350, 610),
        UiEvent::UnknownClick { x: 350, y: 610 }
    );
}
