//! OS-level mouse hook for tracking clicks on the game window.
//!
//! Uses `SetWindowsHookEx(WH_MOUSE_LL)` to passively monitor mouse events
//! without injecting anything into the game's JS environment.
//! The hook runs on a dedicated thread with its own message pump, because
//! WH_MOUSE_LL callbacks require a Windows message loop to fire.
//!
//! Windows only — the entire module is gated behind `cfg(target_os = "windows")`.

const CANONICAL_GAME_WIDTH: i32 = 1200;
const CANONICAL_GAME_HEIGHT: i32 = 720;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GameContentRect {
    top: i32,
    width: i32,
    height: i32,
}

/// Locate the 1200x720 game canvas inside the window client area.
/// The control bar occupies the remaining space above it.
fn game_content_rect(client_width: i32, client_height: i32) -> Option<GameContentRect> {
    if client_width <= 0 || client_height <= 0 {
        return None;
    }
    let expected_height =
        ((client_width as i64 * CANONICAL_GAME_HEIGHT as i64) / CANONICAL_GAME_WIDTH as i64) as i32;
    let height = expected_height.min(client_height);
    if height <= 0 {
        return None;
    }
    Some(GameContentRect {
        top: client_height - height,
        width: client_width,
        height,
    })
}

/// Convert a physical client-area click to canonical 1200x720 game coordinates.
/// Clicks on the Windows title bar, app control bar, or unused margins are rejected.
fn normalize_game_click(
    client_width: i32,
    client_height: i32,
    client_x: i32,
    client_y: i32,
) -> Option<(i32, i32)> {
    let game = game_content_rect(client_width, client_height)?;
    if client_x < 0
        || client_x >= game.width
        || client_y < game.top
        || client_y >= game.top + game.height
    {
        return None;
    }
    Some((
        (client_x as i64 * CANONICAL_GAME_WIDTH as i64 / game.width as i64) as i32,
        ((client_y - game.top) as i64 * CANONICAL_GAME_HEIGHT as i64 / game.height as i64) as i32,
    ))
}

#[cfg(target_os = "windows")]
mod inner {
    use log::info;
    use std::sync::atomic::{AtomicPtr, Ordering};
    use std::sync::Mutex;
    use tokio::sync::mpsc;
    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
    use windows_sys::Win32::Graphics::Gdi::ClientToScreen;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, GetClientRect, GetMessageW, SetWindowsHookExW, UnhookWindowsHookEx, HHOOK,
        MSG, MSLLHOOKSTRUCT, WH_MOUSE_LL, WM_LBUTTONDOWN, WM_QUIT,
    };

    /// A click event detected within the game window.
    #[derive(Debug, Clone)]
    pub struct GameClick {
        /// Relative x within game window client area (physical pixels)
        pub x: i32,
        /// Relative y within game window client area (physical pixels)
        pub y: i32,
    }

    /// Game window HWND to check bounds against.
    pub(super) static GAME_HWND: AtomicPtr<std::ffi::c_void> = AtomicPtr::new(std::ptr::null_mut());

    /// Hook thread handle for joining on uninstall.
    static HOOK_THREAD_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

    /// Hook handle (stored so the callback can pass it to CallNextHookEx).
    static HOOK_HANDLE: AtomicPtr<std::ffi::c_void> = AtomicPtr::new(std::ptr::null_mut());

    /// Channel sender for click events.
    static CLICK_TX: Mutex<Option<mpsc::UnboundedSender<GameClick>>> = Mutex::new(None);

    /// Debounce: last click timestamp (ms).
    static LAST_CLICK_MS: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);

    const DEBOUNCE_MS: i64 = 500;
    /// Low-level mouse hook callback. Must return quickly.
    unsafe extern "system" fn mouse_hook_proc(
        n_code: i32,
        w_param: WPARAM,
        l_param: LPARAM,
    ) -> LRESULT {
        if n_code >= 0 && w_param == WM_LBUTTONDOWN as usize {
            let ptr = l_param as *const MSLLHOOKSTRUCT;
            if ptr.is_null() {
                let hook = HOOK_HANDLE.load(Ordering::Relaxed) as HHOOK;
                return CallNextHookEx(hook, n_code, w_param, l_param);
            }
            let hook_info = &*ptr;
            let pt = hook_info.pt;

            let hwnd = GAME_HWND.load(Ordering::Relaxed) as HWND;
            if !hwnd.is_null() {
                let mut client_rect: RECT = std::mem::zeroed();
                let mut client_origin = POINT { x: 0, y: 0 };
                if GetClientRect(hwnd, &mut client_rect) != 0
                    && ClientToScreen(hwnd, &mut client_origin) != 0
                {
                    let client_x = pt.x - client_origin.x;
                    let client_y = pt.y - client_origin.y;
                    let client_width = client_rect.right - client_rect.left;
                    let client_height = client_rect.bottom - client_rect.top;
                    let Some((game_x, game_y)) = super::normalize_game_click(
                        client_width,
                        client_height,
                        client_x,
                        client_y,
                    ) else {
                        let hook = HOOK_HANDLE.load(Ordering::Relaxed) as HHOOK;
                        return CallNextHookEx(hook, n_code, w_param, l_param);
                    };

                    let now_ms = chrono::Utc::now().timestamp_millis();
                    let last = LAST_CLICK_MS.load(Ordering::Relaxed);
                    if now_ms - last >= DEBOUNCE_MS {
                        LAST_CLICK_MS.store(now_ms, Ordering::Relaxed);

                        let click = GameClick {
                            x: game_x,
                            y: game_y,
                        };

                        if let Ok(guard) = CLICK_TX.try_lock() {
                            if let Some(ref tx) = *guard {
                                let _ = tx.send(click);
                            }
                        }
                    }
                }
            }
        }

        let hook = HOOK_HANDLE.load(Ordering::Relaxed) as HHOOK;
        CallNextHookEx(hook, n_code, w_param, l_param)
    }

    /// Install the mouse hook on a dedicated thread with a message pump.
    /// Returns a receiver for click events.
    pub fn install(game_hwnd: isize) -> Result<mpsc::UnboundedReceiver<GameClick>, String> {
        uninstall();

        let (tx, rx) = mpsc::unbounded_channel();
        {
            let mut guard = CLICK_TX.lock().map_err(|e| e.to_string())?;
            *guard = Some(tx);
        }
        GAME_HWND.store(game_hwnd as *mut std::ffi::c_void, Ordering::Relaxed);

        // Spawn a dedicated thread with a Windows message pump
        let (ready_tx, ready_rx) = std::sync::mpsc::channel::<Result<(), String>>();

        std::thread::spawn(move || unsafe {
            let hook =
                SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook_proc), std::ptr::null_mut(), 0);

            if hook.is_null() {
                let _ = ready_tx.send(Err("SetWindowsHookExW failed".to_string()));
                return;
            }

            HOOK_HANDLE.store(hook, Ordering::Relaxed);

            // Store this thread's ID so we can post WM_QUIT from uninstall()
            let thread_id = windows_sys::Win32::System::Threading::GetCurrentThreadId();
            HOOK_THREAD_ID.store(thread_id, Ordering::Relaxed);

            info!(
                "[MouseHook] Installed on dedicated thread (HWND={})",
                game_hwnd
            );
            let _ = ready_tx.send(Ok(()));

            // Message pump — keeps the hook alive
            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                // Just pump; we don't translate/dispatch since we only care about the hook
            }

            // Cleanup after WM_QUIT
            UnhookWindowsHookEx(hook);
            HOOK_HANDLE.store(std::ptr::null_mut(), Ordering::Relaxed);
            HOOK_THREAD_ID.store(0, Ordering::Relaxed);
            info!("[MouseHook] Thread exiting");
        });

        // Wait for hook installation result
        ready_rx
            .recv()
            .map_err(|_| "Hook thread died before ready".to_string())??;

        Ok(rx)
    }

    /// Uninstall the mouse hook by posting WM_QUIT to the hook thread.
    pub fn uninstall() {
        let thread_id = HOOK_THREAD_ID.swap(0, Ordering::Relaxed);
        if thread_id != 0 {
            unsafe {
                windows_sys::Win32::UI::WindowsAndMessaging::PostThreadMessageW(
                    thread_id, WM_QUIT, 0, 0,
                );
            }
            info!("[MouseHook] Sent WM_QUIT to hook thread");
        }

        if let Ok(mut guard) = CLICK_TX.lock() {
            *guard = None;
        }
        GAME_HWND.store(std::ptr::null_mut(), Ordering::Relaxed);
    }
}

#[cfg(target_os = "windows")]
pub use inner::{install, uninstall, GameClick};

#[derive(Debug, Clone)]
struct ScreenTransition {
    screen: crate::ui_event::Screen,
    label: String,
}

pub(crate) fn debug_screen_name(screen: crate::ui_event::Screen) -> String {
    if screen == crate::ui_event::Screen::Unknown {
        "???".to_string()
    } else {
        format!("{:?}", screen)
    }
}

/// Classify clicks which are expected to replace the current game screen.
/// Unknown destinations deliberately become `Screen::Unknown` so the Debug UI
/// shows `???` until an API endpoint identifies the new screen.
#[cfg(target_os = "windows")]
fn screen_transition_from_event(event: &crate::ui_event::UiEvent) -> Option<ScreenTransition> {
    use crate::ui_event::{Screen, UiEvent};
    let (screen, label) = match event {
        UiEvent::StartGame => (Screen::Homeport, "母港".to_string()),
        UiEvent::Navigate { target } | UiEvent::SideMenuClick { target } => {
            let screen = match target.as_str() {
                "編成" => Screen::FleetComposition,
                "改装" => Screen::Remodel,
                "補給" => Screen::Resupply,
                "出撃" => Screen::SortieMenu,
                "入渠" => Screen::RepairDockSelect,
                "工廠" => Screen::Factory,
                _ => Screen::Unknown,
            };
            (screen, target.clone())
        }
        UiEvent::SelectMode { mode } => {
            let screen = match mode.as_str() {
                "出撃" => Screen::SortieSelectChinjufu,
                "遠征" => Screen::ExpeditionSelect,
                // Practice selection has not been modeled yet.
                "演習" => Screen::Unknown,
                _ => Screen::Unknown,
            };
            (screen, mode.clone())
        }
        UiEvent::SortieAreaSelect { area } => {
            let screen = match area.as_str() {
                "鎮守府海域" => Screen::SortieSelectChinjufu,
                "南西諸島海域" => Screen::SortieSelectSouthwestIslands,
                "北方海域" => Screen::SortieSelectNorthern,
                "南西海域" => Screen::SortieSelectSouthwestern,
                "西方海域" => Screen::SortieSelectWestern,
                "南方海域" => Screen::SortieSelectSouthern,
                "中部海域" => Screen::SortieSelectCentral,
                "期間限定海域" => Screen::SortieSelectEvent,
                _ => Screen::SortieSelect,
            };
            (screen, format!("海域選択-{}", area))
        }
        UiEvent::OpenAirBaseSupply => (
            Screen::AirBaseSupply1,
            "基地航空隊-第一基地航空隊".to_string(),
        ),
        UiEvent::AirBaseSelect { base } => {
            let screen = match base {
                1 => Screen::AirBaseSupply1,
                2 => Screen::AirBaseSupply2,
                3 => Screen::AirBaseSupply3,
                _ => Screen::AirBaseSupply,
            };
            (screen, format!("基地航空隊-第{}基地航空隊", base))
        }
        UiEvent::ExpeditionAction { action } => match action.as_str() {
            // 決定 button on ExpeditionSelect transitions to fleet-select sub-screen.
            "決定" => (Screen::ExpeditionFleetSelect, "遠征-艦隊選択".to_string()),
            _ => return None,
        },
        UiEvent::FleetChangeStart { .. } => (Screen::ShipSelection, "編成-艦船選択".to_string()),
        UiEvent::ShipSelect { .. } => (Screen::ShipChangeConfirm, "編成-変更確認".to_string()),
        UiEvent::FleetChangeConfirm => (Screen::FleetComposition, "編成".to_string()),
        UiEvent::RemodelEquipmentSlot { .. } => {
            (Screen::RemodelEquipmentSelect, "改装-装備選択".to_string())
        }
        UiEvent::RemodelEquipmentFilterOpen => (
            Screen::RemodelEquipmentFilter,
            "改装-装備種別選択".to_string(),
        ),
        UiEvent::RemodelEquipmentCategorySelect => {
            (Screen::RemodelEquipmentSelect, "改装-装備選択".to_string())
        }
        UiEvent::RemodelEquipmentSelect { .. } => (
            Screen::RemodelEquipmentConfirm,
            "改装-装備変更確認".to_string(),
        ),
        UiEvent::RemodelEquipmentChangeConfirm => (Screen::Remodel, "改装".to_string()),
        UiEvent::RepairDockSelect { .. } => (Screen::RepairShipSelect, "入渠-艦船選択".to_string()),
        UiEvent::FactorySelect { mode } => {
            let screen = if mode == "開発" {
                Screen::FactoryDevelop
            } else {
                Screen::Unknown
            };
            (screen, mode.clone())
        }
        UiEvent::TopMenuClick { target } => {
            let screen = match target.as_str() {
                "任務" => Screen::QuestList,
                "図鑑表示" => Screen::Encyclopedia,
                "アイテム" => Screen::ItemListHeld,
                "模様替え" => Screen::FurnitureChange,
                _ => Screen::Unknown,
            };
            (screen, target.clone())
        }
        UiEvent::ItemMenuSelect { target } => {
            let screen = match target.as_str() {
                "アイテム一覧" => Screen::ItemListHeld,
                "アイテム屋" => Screen::ItemShopRegular,
                "家具屋" => Screen::FurnitureShopCategory,
                _ => Screen::Unknown,
            };
            let label = if target == "アイテム屋" {
                "アイテム屋-レギュラーコーナー".to_string()
            } else {
                target.clone()
            };
            (screen, label)
        }
        UiEvent::ItemInventoryTab { tab } => {
            let screen = match tab.as_str() {
                "保有アイテム" => Screen::ItemListHeld,
                "購入済みアイテム" => Screen::ItemListPurchased,
                _ => Screen::Unknown,
            };
            (screen, format!("アイテム一覧-{}", tab))
        }
        UiEvent::ItemExpansionOpen => (
            Screen::ItemListExpansion,
            "アイテム一覧-拡張アイテム".to_string(),
        ),
        UiEvent::ItemShopCornerSwitch { corner } => {
            let screen = match corner.as_str() {
                "レギュラーコーナー" => Screen::ItemShopRegular,
                "特選コーナー" => Screen::ItemShopSpecial,
                _ => Screen::Unknown,
            };
            (screen, format!("アイテム屋-{}", corner))
        }
        UiEvent::FurnitureChangeOpenShop => (Screen::FurnitureShopCategory, "家具屋".to_string()),
        UiEvent::FurnitureChangeReturnHomeport => (Screen::Homeport, "母港".to_string()),
        UiEvent::FurnitureCategorySelect { category } => {
            (Screen::FurnitureShopList, format!("家具一覧-{}", category))
        }
        UiEvent::FurnitureListBack => (Screen::FurnitureShopCategory, "家具屋".to_string()),
        UiEvent::ItemReturnHomeport => (Screen::Homeport, "母港".to_string()),
        UiEvent::GetScreenDismiss => (Screen::Unknown, "GET画面を閉じる".to_string()),
        _ => return None,
    };
    Some(ScreenTransition { screen, label })
}

/// Extract (period, category) updates from a click event on the QuestList.
#[cfg(target_os = "windows")]
fn quest_filter_from_event(
    event: &crate::ui_event::UiEvent,
) -> Option<(Option<&str>, Option<&str>)> {
    use crate::ui_event::UiEvent;
    match event {
        UiEvent::QuestFilter { filter } => Some((Some(filter.as_str()), None)),
        UiEvent::QuestCategoryFilter { category } => Some((None, Some(category.as_str()))),
        _ => None,
    }
}

/// Extract a fleet number (1-4) from fleet-tab click events.
#[cfg(target_os = "windows")]
fn fleet_from_event(event: &crate::ui_event::UiEvent) -> Option<u32> {
    use crate::ui_event::UiEvent;
    match event {
        UiEvent::FleetSelect { fleet } | UiEvent::SupplyFleetSelect { fleet } => Some(*fleet),
        _ => None,
    }
}

#[cfg(all(target_os = "windows", debug_assertions))]
#[derive(Debug, Clone, serde::Serialize)]
struct ClickCaptureMetadata {
    id: String,
    timestamp: String,
    x: i32,
    y: i32,
    screen_before: String,
    screen_after: String,
    screen_change_click: bool,
    target_screen: Option<String>,
    event: serde_json::Value,
}

#[cfg(all(target_os = "windows", debug_assertions))]
struct CapturedScreen {
    png: Vec<u8>,
    path: std::path::PathBuf,
    sample_kind: &'static str,
}

/// Consume click events from the hook and log them.
/// In dev builds, also captures the complete 1200x720 game canvas.
#[cfg(target_os = "windows")]
pub async fn consume_clicks(
    mut rx: tokio::sync::mpsc::UnboundedReceiver<GameClick>,
    app: tauri::AppHandle,
    _data_dir: std::path::PathBuf,
) {
    use log::info;
    use tauri::{Emitter, Manager};

    #[cfg(debug_assertions)]
    let screenshot_dir = {
        let dir = _data_dir
            .join("local")
            .join("action_logs")
            .join("screenshots");
        let _ = std::fs::create_dir_all(&dir);
        dir
    };

    log::info!(
        "[MouseHook] Click consumer started (screen tracked via Navigate / SideMenuClick events)"
    );

    while let Some(click) = rx.recv().await {
        // Read current screen from app state
        let current_screen = *crate::lock_or_recover(
            &app.state::<crate::AppState>().navigation.current_screen,
            "current_screen",
        );

        // Single timestamp for this click — shared by click-event payload and
        // the click-screenshot event so the Debug UI can correlate them.
        let click_now = chrono::Local::now();
        let click_ts = click_now.format("%H:%M:%S%.3f").to_string();
        let click_id = click_now.format("%Y%m%d_%H%M%S_%3f").to_string();

        // Detect semantic UI event from screen + coordinates
        let event = crate::ui_event::detect_event(current_screen, click.x, click.y);
        let event_json = serde_json::to_string(&event).unwrap_or_default();
        let transition = screen_transition_from_event(&event);
        let screen_before = debug_screen_name(current_screen);
        let screen_after = transition
            .as_ref()
            .map(|change| debug_screen_name(change.screen))
            .unwrap_or_else(|| screen_before.clone());
        let target_screen = transition.as_ref().map(|change| change.label.clone());

        crate::action_log::log(
            "Click",
            "game_canvas",
            &format!(
                "x={} y={} screen={} screen_change={} target={} event={}",
                click.x,
                click.y,
                screen_before,
                transition.is_some(),
                target_screen.as_deref().unwrap_or("-"),
                event_json
            ),
        );

        // Update tracked screen when user navigates (button click in Homeport
        // returns `Navigate`, side-menu click in any screen returns `SideMenuClick`).
        if let Some(new_screen) = transition.as_ref().map(|change| change.screen) {
            let state = app.state::<crate::AppState>();
            let mut guard =
                crate::lock_or_recover(&state.navigation.current_screen, "current_screen");
            if *guard != new_screen {
                let prev = *guard;
                info!("[MouseHook] Screen changed: {:?} -> {:?}", prev, new_screen);
                crate::action_log::log(
                    "Screen",
                    "click",
                    &format!("{:?} -> {:?}", prev, new_screen),
                );
                *guard = new_screen;
                drop(guard);
                let _ = app.emit(crate::events::SCREEN_CHANGED, debug_screen_name(new_screen));

                // Composition starts on fleet 1; compatible sub-screens keep
                // the current selection, and unrelated screens clear it.
                let (next_fleet, fleet_changed) =
                    crate::api::screen::update_fleet_after_screen_change(
                        &state.navigation.current_fleet,
                        prev,
                        new_screen,
                    );
                if fleet_changed {
                    let _ = app.emit(crate::events::FLEET_VIEW_CHANGED, next_fleet);
                }

                // Clear quest sub-state when leaving QuestList.
                if new_screen != crate::ui_event::Screen::QuestList {
                    let mut p = crate::lock_or_recover(
                        &state.navigation.current_quest_period,
                        "current_quest_period",
                    );
                    let mut c = crate::lock_or_recover(
                        &state.navigation.current_quest_category,
                        "current_quest_category",
                    );
                    if p.is_some() || c.is_some() {
                        *p = None;
                        *c = None;
                        drop(p);
                        drop(c);
                        let _ = app.emit(
                            crate::events::QUEST_FILTERS_CHANGED,
                            serde_json::json!({"period": null, "category": null}),
                        );
                    }
                }
                crate::nozaki_timer::refresh_overlay(&app);
            }
        }

        // Always emit the raw debug event so the Debug tab can show what was
        // detected for each click. Payload includes coords + JSON-encoded event.
        let _ = app.emit(
            crate::events::CLICK_EVENT,
            serde_json::json!({
                "ts": click_ts.clone(),
                "id": click_id.clone(),
                "x": click.x,
                "y": click.y,
                "screen": screen_before.clone(),
                "screenAfter": screen_after.clone(),
                "screenChangeClick": transition.is_some(),
                "targetScreen": target_screen.clone(),
                "event": event.clone(),
            }),
        );

        // Track current fleet selection and emit fleet-view-changed for kantai.
        if let Some(fleet) = fleet_from_event(&event) {
            let state = app.state::<crate::AppState>();
            let mut guard =
                crate::lock_or_recover(&state.navigation.current_fleet, "current_fleet");
            let prev = *guard;
            if prev != Some(fleet) {
                info!("[MouseHook] Fleet changed: {:?} -> Some({})", prev, fleet);
                crate::action_log::log("Fleet", "click", &format!("{:?} -> Some({})", prev, fleet));
                *guard = Some(fleet);
            }
            drop(guard);
            // Emit unconditionally so the kantai window stays in sync even on
            // repeated clicks of the same tab.
            let _ = app.emit(crate::events::FLEET_VIEW_CHANGED, fleet);
            crate::nozaki_timer::refresh_overlay(&app);
        }

        // Track QuestList sub-screen filters (period × category).
        if let Some((period, category)) = quest_filter_from_event(&event) {
            let state = app.state::<crate::AppState>();
            if let Some(p) = period {
                let mut guard = crate::lock_or_recover(
                    &state.navigation.current_quest_period,
                    "current_quest_period",
                );
                if guard.as_deref() != Some(p) {
                    info!("[MouseHook] Quest period: {:?} -> Some({:?})", *guard, p);
                    crate::action_log::log(
                        "Quest",
                        "period",
                        &format!("{:?} -> Some({:?})", *guard, p),
                    );
                    *guard = Some(p.to_string());
                }
            }
            if let Some(c) = category {
                let mut guard = crate::lock_or_recover(
                    &state.navigation.current_quest_category,
                    "current_quest_category",
                );
                if guard.as_deref() != Some(c) {
                    info!("[MouseHook] Quest category: {:?} -> Some({:?})", *guard, c);
                    crate::action_log::log(
                        "Quest",
                        "category",
                        &format!("{:?} -> Some({:?})", *guard, c),
                    );
                    *guard = Some(c.to_string());
                }
            }
            let snap_period = crate::lock_or_recover(
                &state.navigation.current_quest_period,
                "current_quest_period",
            )
            .clone();
            let snap_category = crate::lock_or_recover(
                &state.navigation.current_quest_category,
                "current_quest_category",
            )
            .clone();
            let _ = app.emit(
                crate::events::QUEST_FILTERS_CHANGED,
                serde_json::json!({
                    "period": snap_period,
                    "category": snap_category,
                }),
            );
        }

        #[cfg(debug_assertions)]
        {
            let metadata = ClickCaptureMetadata {
                id: click_id.clone(),
                timestamp: click_ts.clone(),
                x: click.x,
                y: click.y,
                screen_before,
                screen_after,
                screen_change_click: transition.is_some(),
                target_screen,
                event: serde_json::to_value(&event).unwrap_or_default(),
            };
            let ss_dir = screenshot_dir.clone();
            match tokio::task::spawn_blocking(move || capture_full_screen(&ss_dir, &metadata)).await
            {
                Ok(Ok(captured)) => {
                    use base64::Engine;
                    let b64 = base64::engine::general_purpose::STANDARD.encode(&captured.png);
                    let _ = app.emit(
                        crate::events::CLICK_SCREENSHOT,
                        serde_json::json!({
                            "ts": click_ts,
                            "id": click_id,
                            "x": click.x,
                            "y": click.y,
                            "image": format!("data:image/png;base64,{}", b64),
                            "imagePath": captured.path.to_string_lossy(),
                            "sampleKind": captured.sample_kind,
                            "width": CANONICAL_GAME_WIDTH,
                            "height": CANONICAL_GAME_HEIGHT,
                        }),
                    );
                }
                Ok(Err(error)) => log::warn!("[MouseHook] Screenshot failed: {}", error),
                Err(error) => log::warn!("[MouseHook] Screenshot task failed: {}", error),
            }
        }
    }

    info!("[MouseHook] Click consumer task ended");
}

/// Capture the complete game canvas using PrintWindow.
/// The Windows title bar and the app's control bar are excluded, and every
/// sample is normalized to 1200x720 for future screen-recognition work.
#[cfg(all(target_os = "windows", debug_assertions))]
fn capture_full_screen(
    screenshot_dir: &std::path::Path,
    metadata: &ClickCaptureMetadata,
) -> Result<CapturedScreen, String> {
    use windows_sys::Win32::Foundation::{HWND, POINT};
    use windows_sys::Win32::Graphics::Gdi::{
        ClientToScreen, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC,
        GetDIBits, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
    };
    use windows_sys::Win32::Storage::Xps::PrintWindow;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetClientRect, GetWindowRect, PW_RENDERFULLCONTENT,
    };

    let hwnd = inner::GAME_HWND.load(std::sync::atomic::Ordering::Relaxed) as HWND;
    if hwnd.is_null() {
        return Err("No game window".to_string());
    }

    unsafe {
        let mut window_rect: windows_sys::Win32::Foundation::RECT = std::mem::zeroed();
        let mut client_rect: windows_sys::Win32::Foundation::RECT = std::mem::zeroed();
        if GetWindowRect(hwnd, &mut window_rect) == 0 {
            return Err("GetWindowRect failed".to_string());
        }
        if GetClientRect(hwnd, &mut client_rect) == 0 {
            return Err("GetClientRect failed".to_string());
        }
        let outer_width = window_rect.right - window_rect.left;
        let outer_height = window_rect.bottom - window_rect.top;
        let client_width = client_rect.right - client_rect.left;
        let client_height = client_rect.bottom - client_rect.top;
        if outer_width <= 0 || outer_height <= 0 || client_width <= 0 || client_height <= 0 {
            return Err("Invalid window size".to_string());
        }
        let mut client_origin = POINT { x: 0, y: 0 };
        if ClientToScreen(hwnd, &mut client_origin) == 0 {
            return Err("ClientToScreen failed".to_string());
        }
        let game_rect =
            game_content_rect(client_width, client_height).ok_or("Invalid game canvas size")?;

        let hdc_window = GetDC(hwnd);
        if hdc_window.is_null() {
            return Err("GetDC failed".to_string());
        }

        let hdc_mem = CreateCompatibleDC(hdc_window);
        if hdc_mem.is_null() {
            ReleaseDC(hwnd, hdc_window);
            return Err("CreateCompatibleDC failed".to_string());
        }
        let hbm = CreateCompatibleBitmap(hdc_window, outer_width, outer_height);
        if hbm.is_null() {
            DeleteDC(hdc_mem);
            ReleaseDC(hwnd, hdc_window);
            return Err("CreateCompatibleBitmap failed".to_string());
        }
        let old_bm = SelectObject(hdc_mem, hbm);

        // PrintWindow with PW_RENDERFULLCONTENT captures WebView2 GPU content
        if PrintWindow(hwnd, hdc_mem, PW_RENDERFULLCONTENT) == 0 {
            SelectObject(hdc_mem, old_bm);
            DeleteObject(hbm);
            DeleteDC(hdc_mem);
            ReleaseDC(hwnd, hdc_window);
            return Err("PrintWindow failed".to_string());
        }

        let mut bmi: BITMAPINFO = std::mem::zeroed();
        bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = outer_width;
        bmi.bmiHeader.biHeight = -outer_height; // top-down
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB;

        let pixel_count = (outer_width * outer_height) as usize;
        let mut pixels: Vec<u8> = vec![0u8; pixel_count * 4];

        let copied = GetDIBits(
            hdc_mem,
            hbm,
            0,
            outer_height as u32,
            pixels.as_mut_ptr() as *mut _,
            &mut bmi,
            DIB_RGB_COLORS,
        );

        SelectObject(hdc_mem, old_bm);
        DeleteObject(hbm);
        DeleteDC(hdc_mem);
        ReleaseDC(hwnd, hdc_window);
        if copied == 0 {
            return Err("GetDIBits failed".to_string());
        }

        // BGRA → RGBA
        for chunk in pixels.chunks_exact_mut(4) {
            chunk.swap(0, 2);
        }

        let image: image::RgbaImage =
            image::ImageBuffer::from_raw(outer_width as u32, outer_height as u32, pixels)
                .ok_or("Failed to create image buffer")?;

        let client_left = (client_origin.x - window_rect.left).max(0) as u32;
        let client_top = (client_origin.y - window_rect.top).max(0) as u32;
        let game = image::DynamicImage::ImageRgba8(image)
            .crop_imm(
                client_left,
                client_top + game_rect.top as u32,
                game_rect.width as u32,
                game_rect.height as u32,
            )
            .resize_exact(
                CANONICAL_GAME_WIDTH as u32,
                CANONICAL_GAME_HEIGHT as u32,
                image::imageops::FilterType::Lanczos3,
            );

        let sample_kind = if metadata.screen_before == "???" {
            "unknown"
        } else {
            "known"
        };
        let sample_dir = screenshot_dir.join(sample_kind);
        std::fs::create_dir_all(&sample_dir)
            .map_err(|error| format!("Create sample directory: {error}"))?;
        let click_kind = if metadata.screen_change_click {
            "transition"
        } else {
            "action"
        };
        let screen_name = metadata.screen_before.replace('?', "unknown");
        let stem = format!(
            "{}_{}_{}_x{}_y{}",
            metadata.id, screen_name, click_kind, metadata.x, metadata.y
        );
        let image_path = sample_dir.join(format!("{stem}.png"));

        let mut png = Vec::new();
        game.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .map_err(|error| format!("Encode PNG: {error}"))?;
        std::fs::write(&image_path, &png).map_err(|error| format!("Save PNG: {error}"))?;

        let mut metadata_json =
            serde_json::to_value(metadata).map_err(|error| format!("Metadata JSON: {error}"))?;
        metadata_json["image"] = serde_json::Value::String(
            image_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
        );
        let metadata_path = sample_dir.join(format!("{stem}.json"));
        let metadata_bytes = serde_json::to_vec_pretty(&metadata_json)
            .map_err(|error| format!("Encode metadata: {error}"))?;
        std::fs::write(&metadata_path, metadata_bytes)
            .map_err(|error| format!("Save metadata: {error}"))?;

        log::info!(
            "[MouseHook] Full screen sample: {} ({} click, screen={})",
            image_path.display(),
            click_kind,
            metadata.screen_before
        );
        return Ok(CapturedScreen {
            png,
            path: image_path,
            sample_kind,
        });
    }
}

pub(crate) fn screen_sample_summary(data_dir: &std::path::Path) -> serde_json::Value {
    let root = data_dir
        .join("local")
        .join("action_logs")
        .join("screenshots");
    let count = |kind: &str| {
        std::fs::read_dir(root.join(kind))
            .ok()
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .filter(|entry| {
                entry.path().extension().and_then(|value| value.to_str()) == Some("png")
            })
            .count()
    };
    serde_json::json!({
        "known": count("known"),
        "unknown": count("unknown"),
        "directory": root.to_string_lossy(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui_event::{Screen, UiEvent};

    #[test]
    fn control_bar_and_outside_clicks_are_rejected() {
        assert_eq!(game_content_rect(1200, 748).unwrap().top, 28);
        assert_eq!(normalize_game_click(1200, 748, 600, 27), None);
        assert_eq!(normalize_game_click(1200, 748, -1, 100), None);
        assert_eq!(normalize_game_click(1200, 748, 1200, 100), None);
    }

    #[test]
    fn game_clicks_are_normalized_at_any_zoom() {
        assert_eq!(normalize_game_click(1200, 748, 600, 388), Some((600, 360)));
        assert_eq!(normalize_game_click(600, 388, 300, 208), Some((600, 360)));
        assert_eq!(normalize_game_click(600, 388, 599, 387), Some((1198, 718)));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn screen_change_clicks_are_classified_separately() {
        let change = screen_transition_from_event(&UiEvent::FleetChangeStart { slot: 2 }).unwrap();
        assert_eq!(change.screen, Screen::ShipSelection);
        assert_eq!(change.label, "編成-艦船選択");
        assert!(screen_transition_from_event(&UiEvent::UnknownClick { x: 10, y: 10 }).is_none());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn sortie_area_clicks_transition_to_distinct_screens() {
        let samples = [
            ("鎮守府海域", Screen::SortieSelectChinjufu),
            ("南西諸島海域", Screen::SortieSelectSouthwestIslands),
            ("北方海域", Screen::SortieSelectNorthern),
            ("南西海域", Screen::SortieSelectSouthwestern),
            ("西方海域", Screen::SortieSelectWestern),
            ("南方海域", Screen::SortieSelectSouthern),
            ("中部海域", Screen::SortieSelectCentral),
            ("期間限定海域", Screen::SortieSelectEvent),
        ];

        for (area, expected) in samples {
            let change = screen_transition_from_event(&UiEvent::SortieAreaSelect {
                area: area.to_string(),
            })
            .unwrap();
            assert_eq!(change.screen, expected, "{area}");
            assert_eq!(change.label, format!("海域選択-{area}"));
        }
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn airbase_panel_opens_on_first_base_and_tabs_are_distinct_screens() {
        let opened = screen_transition_from_event(&UiEvent::OpenAirBaseSupply).unwrap();
        assert_eq!(opened.screen, Screen::AirBaseSupply1);

        let samples = [
            (1, Screen::AirBaseSupply1),
            (2, Screen::AirBaseSupply2),
            (3, Screen::AirBaseSupply3),
        ];
        for (base, expected) in samples {
            let change = screen_transition_from_event(&UiEvent::AirBaseSelect { base }).unwrap();
            assert_eq!(change.screen, expected, "base {base}");
        }
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn captured_port_screen_flows_have_known_destinations() {
        assert_eq!(
            screen_transition_from_event(&UiEvent::StartGame)
                .unwrap()
                .screen,
            Screen::Homeport
        );
        assert_eq!(
            screen_transition_from_event(&UiEvent::TopMenuClick {
                target: "アイテム".to_string(),
            })
            .unwrap()
            .screen,
            Screen::ItemListHeld
        );
        assert_eq!(
            screen_transition_from_event(&UiEvent::TopMenuClick {
                target: "図鑑表示".to_string(),
            })
            .unwrap()
            .screen,
            Screen::Encyclopedia
        );
        assert_eq!(
            screen_transition_from_event(&UiEvent::TopMenuClick {
                target: "模様替え".to_string(),
            })
            .unwrap()
            .screen,
            Screen::FurnitureChange
        );
        assert_eq!(
            screen_transition_from_event(&UiEvent::ItemMenuSelect {
                target: "家具屋".to_string(),
            })
            .unwrap()
            .screen,
            Screen::FurnitureShopCategory
        );
        assert_eq!(
            screen_transition_from_event(&UiEvent::RemodelEquipmentSlot { slot: 2 })
                .unwrap()
                .screen,
            Screen::RemodelEquipmentSelect
        );
        assert_eq!(
            screen_transition_from_event(&UiEvent::RemodelEquipmentSelect { row: 6 })
                .unwrap()
                .screen,
            Screen::RemodelEquipmentConfirm
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn replays_captured_non_sortie_unknown_sequences_without_losing_screen() {
        fn apply_click(screen: &mut Screen, x: i32, y: i32) {
            let event = crate::ui_event::detect_event(*screen, x, y);
            if let Some(change) = screen_transition_from_event(&event) {
                *screen = change.screen;
            }
            assert_ne!(
                *screen,
                Screen::Unknown,
                "screen became unknown after ({x}, {y}): {event:?}"
            );
        }

        // Item inventory -> item shop -> furniture categories/lists -> port.
        let mut item_screen = Screen::ItemListHeld;
        for (x, y) in [
            (409, 196),
            (85, 283),
            (350, 241),
            (967, 685),
            (489, 698),
            (107, 333),
            (334, 274),
            (347, 686),
            (381, 346),
            (397, 661),
            (408, 414),
            (351, 689),
            (374, 547),
            (343, 694),
            (341, 700),
            (98, 680),
        ] {
            apply_click(&mut item_screen, x, y);
        }
        assert_eq!(item_screen, Screen::Homeport);

        // Resupply side menu -> remodel -> equipment filtering/selection.
        let mut remodel_screen = Screen::Resupply;
        for (x, y) in [
            (30, 390),
            (302, 175),
            (314, 402),
            (402, 325),
            (361, 249),
            (325, 512),
            (328, 645),
            (324, 504),
            (323, 420),
            (353, 314),
            (570, 309),
            (793, 169),
            (833, 285),
            (803, 446),
            (1134, 667),
        ] {
            apply_click(&mut remodel_screen, x, y);
        }
        assert_eq!(remodel_screen, Screen::Remodel);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn replays_194238_furniture_and_item_sequence_without_false_fleet_screen() {
        fn apply_click(screen: &mut Screen, x: i32, y: i32) {
            let event = crate::ui_event::detect_event(*screen, x, y);
            if let Some(change) = screen_transition_from_event(&event) {
                *screen = change.screen;
            }
            assert_ne!(
                *screen,
                Screen::Unknown,
                "screen became unknown after ({x}, {y}): {event:?}"
            );
            assert_ne!(
                *screen,
                Screen::FleetComposition,
                "false fleet transition after ({x}, {y}): {event:?}"
            );
        }

        let mut screen = Screen::Factory;
        for (x, y) in [
            (742, 87),
            (172, 60),
            (142, 128),
            (93, 196),
            (98, 247),
            (134, 312),
            (119, 387),
            (771, 1),
            (108, 553),
            (317, 339),
            (412, 703),
            (400, 553),
            (339, 698),
            (73, 229),
            (816, 230),
            (222, 227),
            (465, 168),
            (50, 274),
            (236, 281),
            (1005, 667),
            (763, 662),
            (301, 690),
            (301, 687),
            (92, 328),
            (703, 383),
        ] {
            apply_click(&mut screen, x, y);
        }
        assert_eq!(screen, Screen::FurnitureShopCategory);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn replays_1955_item_shop_corner_switches_as_separate_screens() {
        fn apply_click(screen: &mut Screen, x: i32, y: i32) {
            let event = crate::ui_event::detect_event(*screen, x, y);
            if let Some(change) = screen_transition_from_event(&event) {
                *screen = change.screen;
            }
            assert_ne!(*screen, Screen::Unknown);
        }

        let mut screen = Screen::ItemShopRegular;
        apply_click(&mut screen, 329, 326);
        apply_click(&mut screen, 1055, 689);
        assert_eq!(screen, Screen::ItemShopSpecial);

        apply_click(&mut screen, 740, 689);
        apply_click(&mut screen, 309, 672);
        assert_eq!(screen, Screen::ItemShopRegular);

        // This click was visually covered and did not change the captured page.
        apply_click(&mut screen, 857, 693);
        assert_eq!(screen, Screen::ItemShopRegular);

        apply_click(&mut screen, 1065, 688);
        assert_eq!(screen, Screen::ItemShopSpecial);
        apply_click(&mut screen, 281, 684);
        assert_eq!(screen, Screen::ItemShopRegular);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn replays_2012_item_inventory_as_three_separate_screens() {
        fn apply_click(screen: &mut Screen, x: i32, y: i32) {
            let event = crate::ui_event::detect_event(*screen, x, y);
            if let Some(change) = screen_transition_from_event(&event) {
                *screen = change.screen;
            }
            assert_ne!(*screen, Screen::Unknown);
        }

        let mut screen = Screen::ItemListHeld;
        apply_click(&mut screen, 767, 214);
        assert_eq!(screen, Screen::ItemListExpansion);
        apply_click(&mut screen, 276, 215);
        assert_eq!(screen, Screen::ItemListHeld);

        apply_click(&mut screen, 789, 217);
        assert_eq!(screen, Screen::ItemListExpansion);
        apply_click(&mut screen, 274, 169);
        assert_eq!(screen, Screen::ItemListHeld);

        apply_click(&mut screen, 454, 164);
        assert_eq!(screen, Screen::ItemListPurchased);
        apply_click(&mut screen, 309, 181);
        assert_eq!(screen, Screen::ItemListHeld);

        apply_click(&mut screen, 825, 218);
        assert_eq!(screen, Screen::ItemListExpansion);
        apply_click(&mut screen, 258, 232);
        assert_eq!(screen, Screen::ItemListHeld);

        apply_click(&mut screen, 430, 165);
        assert_eq!(screen, Screen::ItemListPurchased);
        apply_click(&mut screen, 297, 163);
        assert_eq!(screen, Screen::ItemListHeld);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn unmodeled_destination_becomes_unknown() {
        let change = screen_transition_from_event(&UiEvent::SelectMode {
            mode: "演習".to_string(),
        })
        .unwrap();
        assert_eq!(change.screen, Screen::Unknown);
        assert_eq!(debug_screen_name(change.screen), "???");
    }
}

// No-op stubs for non-Windows
#[cfg(not(target_os = "windows"))]
pub fn uninstall() {}
