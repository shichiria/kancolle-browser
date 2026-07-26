//! Game UI event detection from click coordinates and screen state.
//!
//! Maps `(screen, click_x, click_y)` → semantic `UiEvent`.
//! The screen is identified separately (e.g., by header pixel matching);
//! this module handles the coordinate → event mapping.

use serde::Serialize;

/// Known game screens.
// GetScreen intentionally ends with "Screen" — it names the in-game "GET画面".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[allow(dead_code, clippy::enum_variant_names)]
pub enum Screen {
    /// 母港 — main hub with navigation buttons
    Homeport,
    /// 出撃メニュー — sortie/exercise/expedition mode selection
    SortieMenu,
    /// 海域選択 — sortie map selection
    SortieSelect,
    /// 基地航空隊 — event-map air-base organization/supply panel
    AirBaseSupply,
    /// 出撃中 — map advance and battle screens
    SortieInProgress,
    /// 遠征選択 — expedition list with area tabs
    ExpeditionSelect,
    /// 編成 — fleet composition (6 ship slots)
    FleetComposition,
    /// 編成 - 艦船選択 — ship selection list
    ShipSelection,
    /// 編成 - 変更確認 — ship change confirmation
    ShipChangeConfirm,
    /// 遠征 - 艦隊選択 — fleet selection sub-screen of ExpeditionSelect.
    /// Has its own fleet-tab strip at x≈565-745, y≈175-220.
    ExpeditionFleetSelect,
    /// 改装 — modernization / equipment / remodel screen.
    /// Shares the fleet-tab area with 編成 (x≈70-170, y≈120-140) but has a
    /// distinct ship-list / equipment layout.
    Remodel,
    /// 補給 — resupply screen
    Resupply,
    /// 入渠 - ドック選択 — repair dock selection
    RepairDockSelect,
    /// 入渠 - 艦船選択 — repair ship selection
    RepairShipSelect,
    /// 工廠 — factory main menu
    Factory,
    /// 工廠 - 開発 — equipment development
    FactoryDevelop,
    /// 任務 — quest list
    QuestList,
    /// GET画面 — equipment/ship acquisition result
    GetScreen,
    /// 不明
    Unknown,
}

/// A semantic UI event derived from screen + click position.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[allow(dead_code)]
pub enum UiEvent {
    /// Navigate from homeport to another screen
    Navigate { target: String },
    /// Select sortie mode (出撃/演習/遠征)
    SelectMode { mode: String },
    /// Open the event-map air-base organization/supply panel
    OpenAirBaseSupply,
    /// Switch expedition area tab
    ExpeditionTab { area: String },
    /// Select an expedition from the list
    ExpeditionSelect { row: u32 },
    /// Start or recall expedition
    ExpeditionAction { action: String },
    /// Select fleet tab (1-4)
    FleetSelect { fleet: u32 },
    /// Start changing a ship in a fleet slot
    FleetChangeStart { slot: u32 },
    /// View ship detail in a fleet slot
    ShipDetail { slot: u32 },
    /// Change ship type filter in ship selection
    ShipFilterChange,
    /// Select a ship from the list
    ShipSelect { row: u32 },
    /// Change sort order in ship list
    ShipSort,
    /// Navigate ship list page
    ShipListPage { page: u32 },
    /// Confirm ship change
    FleetChangeConfirm,
    /// Select fleet tab in resupply screen
    SupplyFleetSelect { fleet: u32 },
    /// Toggle ship selection in resupply
    SupplyShipToggle { row: u32 },
    /// Execute resupply
    SupplyExecute,
    /// Select a repair dock
    RepairDockSelect { dock: u32 },
    /// Select a ship for repair
    RepairShipSelect { row: u32 },
    /// Use instant repair
    RepairInstant,
    /// Select factory mode (建造/解体/開発/廃棄)
    FactorySelect { mode: String },
    /// Start development
    DevelopStart,
    /// Use instant construction
    FactoryInstantBuild { dock: u32 },
    /// Select left-side quest period filter (全/遂行中/Daily/Weekly/Monthly/単/他/Others)
    QuestFilter { filter: String },
    /// Select top-row quest category filter (出撃/演習/遠征/編成/その他)
    QuestCategoryFilter { category: String },
    /// Select a quest
    QuestSelect { row: u32 },
    /// Dismiss GET screen
    GetScreenDismiss,
    /// Click on left side menu
    SideMenuClick { target: String },
    /// Click on top menu bar
    TopMenuClick { target: String },
    /// Unrecognized click
    UnknownClick { x: i32, y: i32 },
}

/// A rectangular hit region on the game canvas.
struct HitRegion {
    x_min: i32,
    x_max: i32,
    y_min: i32,
    y_max: i32,
}

impl HitRegion {
    const fn new(x_min: i32, y_min: i32, x_max: i32, y_max: i32) -> Self {
        Self {
            x_min,
            x_max,
            y_min,
            y_max,
        }
    }

    fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x_min && x <= self.x_max && y >= self.y_min && y <= self.y_max
    }
}

/// Detect a UI event from the current screen and click coordinates.
pub fn detect_event(screen: Screen, x: i32, y: i32) -> UiEvent {
    // Check common regions first (side menu, top menu)
    if let Some(event) = check_side_menu(x, y) {
        return event;
    }
    if let Some(event) = check_top_menu(x, y) {
        return event;
    }

    match screen {
        Screen::Homeport => detect_homeport(x, y),
        Screen::SortieMenu => detect_sortie_menu(x, y),
        Screen::SortieSelect => detect_sortie_select(x, y),
        Screen::ExpeditionSelect => detect_expedition_select(x, y),
        Screen::FleetComposition => detect_fleet_composition(x, y),
        Screen::ShipSelection => detect_ship_selection(x, y),
        Screen::ShipChangeConfirm => detect_ship_change_confirm(x, y),
        Screen::ExpeditionFleetSelect => detect_expedition_fleet_select(x, y),
        Screen::Remodel => detect_remodel(x, y),
        Screen::Resupply => detect_resupply(x, y),
        Screen::RepairDockSelect => detect_repair_dock(x, y),
        Screen::Factory => detect_factory(x, y),
        Screen::FactoryDevelop => detect_factory_develop(x, y),
        Screen::QuestList => detect_quest_list(x, y),
        Screen::GetScreen => detect_get_screen(x, y),
        _ => UiEvent::UnknownClick { x, y },
    }
}

// ── Common regions ────────────────────────────────────────────────────

fn check_side_menu(x: i32, y: i32) -> Option<UiEvent> {
    if x > 65 {
        return None;
    }
    let target = if HitRegion::new(10, 130, 65, 180).contains(x, y) {
        "編成"
    } else if HitRegion::new(10, 190, 65, 240).contains(x, y) {
        "補給"
    } else if HitRegion::new(10, 250, 65, 300).contains(x, y) {
        "改装"
    } else if HitRegion::new(10, 310, 65, 360).contains(x, y) {
        "入渠"
    } else if HitRegion::new(10, 370, 65, 420).contains(x, y) {
        "工廠"
    } else {
        return None;
    };
    Some(UiEvent::SideMenuClick {
        target: target.to_string(),
    })
}

fn check_top_menu(x: i32, y: i32) -> Option<UiEvent> {
    if !(55..=70).contains(&y) {
        return None;
    }
    // Top menu items are roughly evenly spaced
    let target = if (200..=280).contains(&x) {
        "戦績表示"
    } else if (310..=390).contains(&x) {
        "友軍艦隊"
    } else if (420..=500).contains(&x) {
        "図鑑表示"
    } else if (530..=590).contains(&x) {
        "アイテム"
    } else if (620..=700).contains(&x) {
        "模様替え"
    } else if (730..=790).contains(&x) {
        "任務"
    } else {
        return None;
    };
    Some(UiEvent::TopMenuClick {
        target: target.to_string(),
    })
}

// ── Screen-specific detectors ────────────────────────────────────────

fn detect_homeport(x: i32, y: i32) -> UiEvent {
    // Coordinates calibrated from observed clicks (2026-05-05):
    //   編成 (258, 249) — user-confirmed
    //   改装 (493, 358) — preceded api_req_kaisou/can_preset_slot_select
    //   入渠 (199, 558) — preceded api_get_member/ndock
    //   工廠 (394, 590) — preceded api_get_member/preset_dev_items
    // The previously documented coords (top-of-canvas) were way off — actual
    // buttons are 100-200px lower than ui-regions.md states.
    let target = if HitRegion::new(200, 200, 320, 300).contains(x, y) {
        "編成"
    } else if HitRegion::new(100, 320, 220, 440).contains(x, y) {
        "補給"
    } else if HitRegion::new(430, 300, 560, 420).contains(x, y) {
        "改装"
    } else if HitRegion::new(250, 380, 410, 510).contains(x, y) {
        "出撃"
    } else if HitRegion::new(130, 510, 280, 620).contains(x, y) {
        "入渠"
    } else if HitRegion::new(320, 530, 470, 650).contains(x, y) {
        "工廠"
    } else {
        return UiEvent::UnknownClick { x, y };
    };
    UiEvent::Navigate {
        target: target.to_string(),
    }
}

fn detect_sortie_select(x: i32, y: i32) -> UiEvent {
    // Event-map 「基地航空隊」 button at the lower left.
    if HitRegion::new(200, 560, 500, 660).contains(x, y) {
        UiEvent::OpenAirBaseSupply
    } else {
        detect_sortie_menu(x, y)
    }
}

fn detect_sortie_menu(x: i32, y: i32) -> UiEvent {
    if HitRegion::new(300, 80, 450, 120).contains(x, y) {
        UiEvent::SelectMode {
            mode: "出撃".to_string(),
        }
    } else if HitRegion::new(460, 80, 570, 120).contains(x, y) {
        UiEvent::SelectMode {
            mode: "演習".to_string(),
        }
    } else if HitRegion::new(580, 80, 700, 120).contains(x, y) {
        UiEvent::SelectMode {
            mode: "遠征".to_string(),
        }
    } else {
        UiEvent::UnknownClick { x, y }
    }
}

fn detect_expedition_select(x: i32, y: i32) -> UiEvent {
    // Right-panel buttons checked FIRST so they aren't shadowed by the
    // bottom area-tab branch (which spans y≥690 across narrow x).
    // 決定 (right-bottom) advances to ExpeditionFleetSelect via screen tracker.
    if (900..=1160).contains(&x) {
        if (200..=230).contains(&y) {
            return UiEvent::ExpeditionAction {
                action: "開始".to_string(),
            };
        }
        if (650..=720).contains(&y) {
            return UiEvent::ExpeditionAction {
                action: "決定".to_string(),
            };
        }
    }

    // Area tabs at bottom (y≈695-720, x≈170-620)
    if y >= 690 && x < 700 {
        let area = if (170..250).contains(&x) {
            "鎮守府海域"
        } else if (250..320).contains(&x) {
            "南西諸島海域"
        } else if (320..380).contains(&x) {
            "北方海域"
        } else if (380..440).contains(&x) {
            "西方海域"
        } else if (440..510).contains(&x) {
            "南方海域"
        } else if (510..560).contains(&x) {
            "トラック泊地沖"
        } else if (560..620).contains(&x) {
            "中部海域"
        } else {
            return UiEvent::UnknownClick { x, y };
        };
        return UiEvent::ExpeditionTab {
            area: area.to_string(),
        };
    }

    // Expedition list rows (y≈170..530, 8 rows, ~45px each)
    if (100..=850).contains(&x) && (170..=530).contains(&y) {
        let row = ((y - 170) / 45) as u32 + 1;
        return UiEvent::ExpeditionSelect { row: row.min(8) };
    }

    UiEvent::UnknownClick { x, y }
}

fn detect_expedition_fleet_select(x: i32, y: i32) -> UiEvent {
    // 遠征-艦隊選択 has only 3 tabs (第2/第3/第4) — 第1 is the main fleet
    // and cannot be sent on expeditions, so it isn't shown as a tab.
    // Calibrated from user clicks 2026-05-06:
    //   第2 cluster: 495, 544, 590
    //   第3 cluster: 633, 639, 642
    //   第4 cluster: 689, 693
    if (175..=225).contains(&y) {
        if (480..615).contains(&x) {
            return UiEvent::FleetSelect { fleet: 2 };
        } else if (615..665).contains(&x) {
            return UiEvent::FleetSelect { fleet: 3 };
        } else if (665..720).contains(&x) {
            return UiEvent::FleetSelect { fleet: 4 };
        }
    }
    // 「遠征開始！」ボタン (right-bottom of fleet-select sub-screen)
    if (950..=1160).contains(&x) && (700..=720).contains(&y) {
        return UiEvent::ExpeditionAction {
            action: "遠征開始".to_string(),
        };
    }
    UiEvent::UnknownClick { x, y }
}

fn detect_fleet_composition(x: i32, y: i32) -> UiEvent {
    // Fleet tabs — calibrated from user clicks (2026-05-05):
    //   第2 (244, 208), 第3 (291, 202), 第4 (336, 204).
    // Tab spacing ≈ 47px starting around x=197 for 第1.
    if (180..=235).contains(&y) {
        if (170..220).contains(&x) {
            return UiEvent::FleetSelect { fleet: 1 };
        } else if (220..270).contains(&x) {
            return UiEvent::FleetSelect { fleet: 2 };
        } else if (270..315).contains(&x) {
            return UiEvent::FleetSelect { fleet: 3 };
        } else if (315..365).contains(&x) {
            return UiEvent::FleetSelect { fleet: 4 };
        }
    }

    // Ship slots: 2 columns x 3 rows
    // Left: x≈80-530, Right: x≈540-1000
    // Row1: y≈160-290, Row2: y≈300-430, Row3: y≈440-570
    let col = if (80..=530).contains(&x) {
        Some(0)
    } else if (540..=1000).contains(&x) {
        Some(1)
    } else {
        None
    };
    let row = if (160..=290).contains(&y) {
        Some(0)
    } else if (300..=430).contains(&y) {
        Some(1)
    } else if (440..=570).contains(&y) {
        Some(2)
    } else {
        None
    };

    if let (Some(c), Some(r)) = (col, row) {
        let slot = (r * 2 + c) as u32 + 1;
        // Distinguish detail vs change button by x position within the slot
        // "変更" button is on the right side of each slot
        let is_right_half = (c == 0 && x > 400) || (c == 1 && x > 870);
        if is_right_half {
            return UiEvent::FleetChangeStart { slot };
        } else {
            return UiEvent::ShipDetail { slot };
        }
    }

    // Empty slot "変更" button (right edge)
    if (1060..=1140).contains(&x) {
        let slot = if (160..=290).contains(&y) {
            Some(2)
        } else if (300..=430).contains(&y) {
            Some(4)
        } else if (440..=570).contains(&y) {
            Some(6)
        } else {
            None
        };
        if let Some(s) = slot {
            return UiEvent::FleetChangeStart { slot: s };
        }
    }

    UiEvent::UnknownClick { x, y }
}

fn detect_ship_selection(x: i32, y: i32) -> UiEvent {
    // Ship type filter tab (right side, x≈1100-1160, y≈170-210)
    if x >= 1100 && (150..=220).contains(&y) {
        return UiEvent::ShipFilterChange;
    }

    // Sort tabs in list header (x≈530-1100, y≈120-140)
    if (530..=1100).contains(&x) && (120..=140).contains(&y) {
        return UiEvent::ShipSort;
    }

    // Ship list rows (x≈530-1100, y≈140-680, ~30px per row)
    if (530..=1100).contains(&x) && (140..=680).contains(&y) {
        let row = ((y - 140) / 30) as u32 + 1;
        return UiEvent::ShipSelect { row };
    }

    // Page navigation (y≈700, x≈700-850)
    if y >= 690 && (600..=900).contains(&x) {
        // Rough page number detection
        let page = ((x - 600) / 30) as u32 + 1;
        return UiEvent::ShipListPage { page };
    }

    // "変更" button on right side (x≈1060-1160, y≈340-380)
    if x >= 1060 && (320..=400).contains(&y) {
        return UiEvent::FleetChangeConfirm;
    }

    UiEvent::UnknownClick { x, y }
}

fn detect_ship_change_confirm(x: i32, y: i32) -> UiEvent {
    // "変更" button at bottom right
    if x >= 1020 && y >= 680 {
        return UiEvent::FleetChangeConfirm;
    }
    UiEvent::UnknownClick { x, y }
}

fn detect_remodel(x: i32, y: i32) -> UiEvent {
    // 改装 has 5 fleet tabs (第1/第2/第3/第4/他) at y≈180-235.
    // Calibrated from user clicks 2026-05-05:
    //   第2 (271, 214), 第3 (313, 216), 第4 (369, 192), 他 (399, 201)
    // Tabs spread over x≈200-420 at ~42px each.
    // 他 is encoded as fleet=5.
    if (180..=235).contains(&y) {
        if (200..242).contains(&x) {
            return UiEvent::FleetSelect { fleet: 1 };
        } else if (242..285).contains(&x) {
            return UiEvent::FleetSelect { fleet: 2 };
        } else if (285..330).contains(&x) {
            return UiEvent::FleetSelect { fleet: 3 };
        } else if (330..380).contains(&x) {
            return UiEvent::FleetSelect { fleet: 4 };
        } else if (380..425).contains(&x) {
            return UiEvent::FleetSelect { fleet: 5 };
        }
    }
    UiEvent::UnknownClick { x, y }
}

fn detect_resupply(x: i32, y: i32) -> UiEvent {
    // Fleet tabs (y≈100-120)
    if (95..=125).contains(&y) {
        let fleet = if (80..110).contains(&x) {
            1
        } else if (110..140).contains(&x) {
            2
        } else if (140..170).contains(&x) {
            3
        } else if (170..200).contains(&x) {
            4
        } else {
            return UiEvent::UnknownClick { x, y };
        };
        return UiEvent::SupplyFleetSelect { fleet };
    }

    // "まとめて補給" button (bottom right)
    if x >= 1000 && y >= 680 {
        return UiEvent::SupplyExecute;
    }

    // Ship rows (x≈100-800, y≈140-600, ~80px per row)
    if (100..=800).contains(&x) && (140..=600).contains(&y) {
        let row = ((y - 140) / 80) as u32 + 1;
        return UiEvent::SupplyShipToggle { row };
    }

    UiEvent::UnknownClick { x, y }
}

fn detect_repair_dock(x: i32, y: i32) -> UiEvent {
    // 4 docks (y≈130..690, ~140px each)
    if (100..=1100).contains(&x) && (130..=690).contains(&y) {
        let dock = ((y - 130) / 140) as u32 + 1;
        return UiEvent::RepairDockSelect {
            dock: dock.min(4),
        };
    }
    UiEvent::UnknownClick { x, y }
}

fn detect_factory(x: i32, y: i32) -> UiEvent {
    // Left menu
    if (100..=350).contains(&x) {
        let mode = if (130..=200).contains(&y) {
            "建造"
        } else if (210..=280).contains(&y) {
            "解体"
        } else if (290..=360).contains(&y) {
            "開発"
        } else if (370..=440).contains(&y) {
            "廃棄"
        } else {
            return UiEvent::UnknownClick { x, y };
        };
        return UiEvent::FactorySelect {
            mode: mode.to_string(),
        };
    }

    // Right panel - dock instant build buttons (x≈900-1100)
    if x >= 900 {
        let dock = if (130..=250).contains(&y) {
            1
        } else if (260..=380).contains(&y) {
            2
        } else if (390..=510).contains(&y) {
            3
        } else if (520..=640).contains(&y) {
            4
        } else {
            return UiEvent::UnknownClick { x, y };
        };
        return UiEvent::FactoryInstantBuild { dock };
    }

    UiEvent::UnknownClick { x, y }
}

fn detect_factory_develop(x: i32, y: i32) -> UiEvent {
    // "開発開始" button (bottom right)
    if x >= 1020 && y >= 620 {
        return UiEvent::DevelopStart;
    }
    UiEvent::UnknownClick { x, y }
}

fn detect_quest_list(x: i32, y: i32) -> UiEvent {
    // Left filter buttons calibrated from user clicks (2026-05-05):
    //   全 y≈247, 遂行中 y≈282, Daily y≈330, Weekly y≈374,
    //   Monthly y≈412, 単 y≈455, 他 y≈497, Others y≈540 (estimated).
    // ~40-45px spacing per row.
    if (60..=160).contains(&x) {
        let filter = if (225..270).contains(&y) {
            "全"
        } else if (270..310).contains(&y) {
            "遂行中"
        } else if (310..355).contains(&y) {
            "Daily"
        } else if (355..395).contains(&y) {
            "Weekly"
        } else if (395..435).contains(&y) {
            "Monthly"
        } else if (435..475).contains(&y) {
            "単"
        } else if (475..520).contains(&y) {
            "他"
        } else if (520..565).contains(&y) {
            "Others"
        } else {
            return UiEvent::UnknownClick { x, y };
        };
        return UiEvent::QuestFilter {
            filter: filter.to_string(),
        };
    }

    // Top category filter row (y≈145-175) calibrated from user clicks 2026-05-05:
    //   出撃 ≈743, 演習 ≈807, 遠征 ≈947-955, 編成 ≈1022, その他 ≈1129-1130
    if (140..=180).contains(&y) {
        let category = if (700..780).contains(&x) {
            "出撃"
        } else if (780..880).contains(&x) {
            "演習"
        } else if (880..1000).contains(&x) {
            "遠征"
        } else if (1000..1080).contains(&x) {
            "編成"
        } else if (1080..1170).contains(&x) {
            "その他"
        } else {
            ""
        };
        if !category.is_empty() {
            return UiEvent::QuestCategoryFilter {
                category: category.to_string(),
            };
        }
    }

    // Quest rows (x≈200-1100, y≈120-680, ~100px per row)
    if (200..=1100).contains(&x) && (120..=680).contains(&y) {
        let row = ((y - 120) / 100) as u32 + 1;
        return UiEvent::QuestSelect { row };
    }

    UiEvent::UnknownClick { x, y }
}

fn detect_get_screen(x: i32, y: i32) -> UiEvent {
    // "帰" button (bottom right)
    if x >= 1100 && y >= 650 {
        return UiEvent::GetScreenDismiss;
    }
    UiEvent::UnknownClick { x, y }
}

#[cfg(test)]
mod tests;
