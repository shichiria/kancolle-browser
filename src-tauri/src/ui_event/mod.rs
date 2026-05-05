//! Game UI event detection from click coordinates and screen state.
//!
//! Maps `(screen, click_x, click_y)` → semantic `UiEvent`.
//! The screen is identified separately (e.g., by header pixel matching);
//! this module handles the coordinate → event mapping.

use serde::Serialize;

/// Known game screens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[allow(dead_code)]
pub enum Screen {
    /// 母港 — main hub with navigation buttons
    Homeport,
    /// 出撃選択 — sortie/exercise/expedition selection
    SortieSelect,
    /// 遠征選択 — expedition list with area tabs
    ExpeditionSelect,
    /// 編成 — fleet composition (6 ship slots)
    FleetComposition,
    /// 編成 - 艦船選択 — ship selection list
    ShipSelection,
    /// 編成 - 変更確認 — ship change confirmation
    ShipChangeConfirm,
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
    /// Select quest filter
    QuestFilter { filter: String },
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
        Screen::SortieSelect => detect_sortie_select(x, y),
        Screen::ExpeditionSelect => detect_expedition_select(x, y),
        Screen::FleetComposition => detect_fleet_composition(x, y),
        Screen::ShipSelection => detect_ship_selection(x, y),
        Screen::ShipChangeConfirm => detect_ship_change_confirm(x, y),
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
    if y < 55 || y > 70 {
        return None;
    }
    // Top menu items are roughly evenly spaced
    let target = if x >= 200 && x <= 280 {
        "戦績表示"
    } else if x >= 310 && x <= 390 {
        "友軍艦隊"
    } else if x >= 420 && x <= 500 {
        "図鑑表示"
    } else if x >= 530 && x <= 590 {
        "アイテム"
    } else if x >= 620 && x <= 700 {
        "模様替え"
    } else if x >= 730 && x <= 790 {
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
    let target = if HitRegion::new(200, 100, 350, 160).contains(x, y) {
        "編成"
    } else if HitRegion::new(100, 180, 230, 270).contains(x, y) {
        "補給"
    } else if HitRegion::new(370, 180, 520, 270).contains(x, y) {
        "改装"
    } else if HitRegion::new(230, 270, 430, 370).contains(x, y) {
        "出撃"
    } else if HitRegion::new(100, 370, 230, 470).contains(x, y) {
        "入渠"
    } else if HitRegion::new(370, 370, 520, 470).contains(x, y) {
        "工廠"
    } else {
        return UiEvent::UnknownClick { x, y };
    };
    UiEvent::Navigate {
        target: target.to_string(),
    }
}

fn detect_sortie_select(x: i32, y: i32) -> UiEvent {
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
    // Area tabs at bottom (y≈695-720)
    if y >= 690 {
        let area = if x >= 170 && x < 250 {
            "鎮守府海域"
        } else if x >= 250 && x < 320 {
            "南西諸島海域"
        } else if x >= 320 && x < 380 {
            "北方海域"
        } else if x >= 380 && x < 440 {
            "西方海域"
        } else if x >= 440 && x < 510 {
            "南方海域"
        } else if x >= 510 && x < 560 {
            "トラック泊地沖"
        } else if x >= 560 && x < 620 {
            "中部海域"
        } else {
            return UiEvent::UnknownClick { x, y };
        };
        return UiEvent::ExpeditionTab {
            area: area.to_string(),
        };
    }

    // Expedition list rows (y≈170..530, 8 rows, ~45px each)
    if x >= 100 && x <= 850 && y >= 170 && y <= 530 {
        let row = ((y - 170) / 45) as u32 + 1;
        return UiEvent::ExpeditionSelect { row: row.min(8) };
    }

    // Right panel buttons
    if x >= 900 && x <= 1100 {
        if y >= 200 && y <= 230 {
            return UiEvent::ExpeditionAction {
                action: "開始".to_string(),
            };
        }
        if y >= 650 && y <= 700 {
            return UiEvent::ExpeditionAction {
                action: "中止/帰還".to_string(),
            };
        }
    }

    UiEvent::UnknownClick { x, y }
}

fn detect_fleet_composition(x: i32, y: i32) -> UiEvent {
    // Fleet tabs (y≈120-140)
    if y >= 120 && y <= 145 {
        if x >= 70 && x < 95 {
            return UiEvent::FleetSelect { fleet: 1 };
        } else if x >= 95 && x < 120 {
            return UiEvent::FleetSelect { fleet: 2 };
        } else if x >= 120 && x < 145 {
            return UiEvent::FleetSelect { fleet: 3 };
        } else if x >= 145 && x < 170 {
            return UiEvent::FleetSelect { fleet: 4 };
        }
    }

    // Ship slots: 2 columns x 3 rows
    // Left: x≈80-530, Right: x≈540-1000
    // Row1: y≈160-290, Row2: y≈300-430, Row3: y≈440-570
    let col = if x >= 80 && x <= 530 {
        Some(0)
    } else if x >= 540 && x <= 1000 {
        Some(1)
    } else {
        None
    };
    let row = if y >= 160 && y <= 290 {
        Some(0)
    } else if y >= 300 && y <= 430 {
        Some(1)
    } else if y >= 440 && y <= 570 {
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
    if x >= 1060 && x <= 1140 {
        let slot = if y >= 160 && y <= 290 {
            Some(2)
        } else if y >= 300 && y <= 430 {
            Some(4)
        } else if y >= 440 && y <= 570 {
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
    if x >= 1100 && y >= 150 && y <= 220 {
        return UiEvent::ShipFilterChange;
    }

    // Sort tabs in list header (x≈530-1100, y≈120-140)
    if x >= 530 && x <= 1100 && y >= 120 && y <= 140 {
        return UiEvent::ShipSort;
    }

    // Ship list rows (x≈530-1100, y≈140-680, ~30px per row)
    if x >= 530 && x <= 1100 && y >= 140 && y <= 680 {
        let row = ((y - 140) / 30) as u32 + 1;
        return UiEvent::ShipSelect { row };
    }

    // Page navigation (y≈700, x≈700-850)
    if y >= 690 && x >= 600 && x <= 900 {
        // Rough page number detection
        let page = ((x - 600) / 30) as u32 + 1;
        return UiEvent::ShipListPage { page };
    }

    // "変更" button on right side (x≈1060-1160, y≈340-380)
    if x >= 1060 && y >= 320 && y <= 400 {
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

fn detect_resupply(x: i32, y: i32) -> UiEvent {
    // Fleet tabs (y≈100-120)
    if y >= 95 && y <= 125 {
        let fleet = if x >= 80 && x < 110 {
            1
        } else if x >= 110 && x < 140 {
            2
        } else if x >= 140 && x < 170 {
            3
        } else if x >= 170 && x < 200 {
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
    if x >= 100 && x <= 800 && y >= 140 && y <= 600 {
        let row = ((y - 140) / 80) as u32 + 1;
        return UiEvent::SupplyShipToggle { row };
    }

    UiEvent::UnknownClick { x, y }
}

fn detect_repair_dock(x: i32, y: i32) -> UiEvent {
    // 4 docks (y≈130..690, ~140px each)
    if x >= 100 && x <= 1100 && y >= 130 && y <= 690 {
        let dock = ((y - 130) / 140) as u32 + 1;
        return UiEvent::RepairDockSelect {
            dock: dock.min(4),
        };
    }
    UiEvent::UnknownClick { x, y }
}

fn detect_factory(x: i32, y: i32) -> UiEvent {
    // Left menu
    if x >= 100 && x <= 350 {
        let mode = if y >= 130 && y <= 200 {
            "建造"
        } else if y >= 210 && y <= 280 {
            "解体"
        } else if y >= 290 && y <= 360 {
            "開発"
        } else if y >= 370 && y <= 440 {
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
        let dock = if y >= 130 && y <= 250 {
            1
        } else if y >= 260 && y <= 380 {
            2
        } else if y >= 390 && y <= 510 {
            3
        } else if y >= 520 && y <= 640 {
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
    // Left filter buttons (x≈70-160)
    if x >= 70 && x <= 160 {
        let filter = if y >= 125 && y <= 145 {
            "全"
        } else if y >= 150 && y <= 170 {
            "遂行中"
        } else if y >= 180 && y <= 200 {
            "Daily"
        } else if y >= 210 && y <= 230 {
            "Weekly"
        } else if y >= 240 && y <= 260 {
            "Monthly"
        } else if y >= 270 && y <= 290 {
            "単"
        } else if y >= 320 && y <= 345 {
            "他"
        } else {
            return UiEvent::UnknownClick { x, y };
        };
        return UiEvent::QuestFilter {
            filter: filter.to_string(),
        };
    }

    // Quest rows (x≈200-1100, y≈120-680, ~100px per row)
    if x >= 200 && x <= 1100 && y >= 120 && y <= 680 {
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
