//! OS-level mouse hook for tracking clicks on the game window.
//!
//! Uses `SetWindowsHookEx(WH_MOUSE_LL)` to passively monitor mouse events
//! without injecting anything into the game's JS environment.
//! The hook runs on a dedicated thread with its own message pump, because
//! WH_MOUSE_LL callbacks require a Windows message loop to fire.
//!
//! Windows only — the entire module is gated behind `cfg(target_os = "windows")`.

#[cfg(target_os = "windows")]
mod inner {
    use log::info;
    use std::sync::atomic::{AtomicPtr, Ordering};
    use std::sync::Mutex;
    use tokio::sync::mpsc;
    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, GetMessageW, GetWindowRect, SetWindowsHookExW, UnhookWindowsHookEx,
        HHOOK, MSLLHOOKSTRUCT, MSG, WH_MOUSE_LL, WM_LBUTTONDOWN, WM_QUIT,
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
    pub(super) static GAME_HWND: AtomicPtr<std::ffi::c_void> =
        AtomicPtr::new(std::ptr::null_mut());

    /// Hook thread handle for joining on uninstall.
    static HOOK_THREAD_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

    /// Hook handle (stored so the callback can pass it to CallNextHookEx).
    static HOOK_HANDLE: AtomicPtr<std::ffi::c_void> = AtomicPtr::new(std::ptr::null_mut());

    /// Channel sender for click events.
    static CLICK_TX: Mutex<Option<mpsc::UnboundedSender<GameClick>>> = Mutex::new(None);

    /// Debounce: last click timestamp (ms).
    static LAST_CLICK_MS: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);

    const DEBOUNCE_MS: i64 = 500;
    /// Tauri window title bar / control bar height in physical pixels.
    /// Subtracted from screen-relative Y to get game-canvas-relative Y.
    const CONTROL_BAR_HEIGHT: i32 = 28;

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
                let mut rect: RECT = std::mem::zeroed();
                if GetWindowRect(hwnd, &mut rect) != 0
                    && pt.x >= rect.left
                    && pt.x <= rect.right
                    && pt.y >= rect.top
                    && pt.y <= rect.bottom
                {
                    let now_ms = chrono::Utc::now().timestamp_millis();
                    let last = LAST_CLICK_MS.load(Ordering::Relaxed);
                    if now_ms - last >= DEBOUNCE_MS {
                        LAST_CLICK_MS.store(now_ms, Ordering::Relaxed);

                        let click = GameClick {
                            x: pt.x - rect.left,
                            y: pt.y - rect.top - CONTROL_BAR_HEIGHT,
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
            let hook = SetWindowsHookExW(
                WH_MOUSE_LL,
                Some(mouse_hook_proc),
                std::ptr::null_mut(),
                0,
            );

            if hook.is_null() {
                let _ = ready_tx.send(Err("SetWindowsHookExW failed".to_string()));
                return;
            }

            HOOK_HANDLE.store(hook, Ordering::Relaxed);

            // Store this thread's ID so we can post WM_QUIT from uninstall()
            let thread_id = windows_sys::Win32::System::Threading::GetCurrentThreadId();
            HOOK_THREAD_ID.store(thread_id, Ordering::Relaxed);

            info!("[MouseHook] Installed on dedicated thread (HWND={})", game_hwnd);
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

/// Map a Navigate / SideMenuClick / SelectMode / ExpeditionAction event to
/// the resulting Screen, if known.
#[cfg(target_os = "windows")]
fn screen_from_event(event: &crate::ui_event::UiEvent) -> Option<crate::ui_event::Screen> {
    use crate::ui_event::{Screen, UiEvent};
    match event {
        UiEvent::Navigate { target } | UiEvent::SideMenuClick { target } => {
            match target.as_str() {
                "編成" => Some(Screen::FleetComposition),
                "改装" => Some(Screen::Remodel),
                "補給" => Some(Screen::Resupply),
                "出撃" => Some(Screen::SortieMenu),
                "入渠" => Some(Screen::RepairDockSelect),
                "工廠" => Some(Screen::Factory),
                _ => None,
            }
        }
        UiEvent::SelectMode { mode } => match mode.as_str() {
            // SortieSelect tabs: 出撃/演習/遠征.
            // 遠征タブ → ExpeditionSelect screen.
            "遠征" => Some(Screen::ExpeditionSelect),
            _ => None,
        },
        UiEvent::OpenAirBaseSupply => Some(Screen::AirBaseSupply),
        UiEvent::ExpeditionAction { action } => match action.as_str() {
            // 決定 button on ExpeditionSelect transitions to fleet-select sub-screen.
            "決定" => Some(Screen::ExpeditionFleetSelect),
            _ => None,
        },
        _ => None,
    }
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

/// Consume click events from the hook and log them.
/// In dev builds, also captures screenshots via GDI and crops the click region.
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

    log::info!("[MouseHook] Click consumer started (screen tracked via Navigate / SideMenuClick events)");

    // Concurrent screenshot guard (max 1 capture in flight). The Debug UI needs
    // a screenshot for every click, so the prior 2-second time gate is removed.
    #[cfg(debug_assertions)]
    let screenshot_semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(1));

    while let Some(click) = rx.recv().await {
        // Read current screen from app state
        let current_screen = *crate::lock_or_recover(
            &app.state::<crate::AppState>().navigation.current_screen,
            "current_screen",
        );

        // Single timestamp for this click — shared by click-event payload and
        // the click-screenshot event so the Debug UI can correlate them.
        let click_ts = chrono::Local::now().format("%H:%M:%S%.3f").to_string();

        // Detect semantic UI event from screen + coordinates
        let event = crate::ui_event::detect_event(current_screen, click.x, click.y);
        let event_json = serde_json::to_string(&event).unwrap_or_default();

        crate::action_log::log(
            "Click",
            "game_canvas",
            &format!("x={} y={} event={}", click.x, click.y, event_json),
        );

        // Update tracked screen when user navigates (button click in Homeport
        // returns `Navigate`, side-menu click in any screen returns `SideMenuClick`).
        if let Some(new_screen) = screen_from_event(&event) {
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
                let _ = app.emit(crate::events::SCREEN_CHANGED, format!("{:?}", new_screen));

                // Clear fleet selection when leaving fleet-compatible screens.
                if !crate::api::screen::has_fleet_tabs(new_screen) {
                    let mut f_guard =
                        crate::lock_or_recover(&state.navigation.current_fleet, "current_fleet");
                    if f_guard.is_some() {
                        *f_guard = None;
                        drop(f_guard);
                        let _ = app.emit(crate::events::FLEET_VIEW_CHANGED, serde_json::Value::Null);
                    }
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
            }
        }

        // Always emit the raw debug event so the Debug tab can show what was
        // detected for each click. Payload includes coords + JSON-encoded event.
        let _ = app.emit(
            crate::events::CLICK_EVENT,
            serde_json::json!({
                "ts": click_ts.clone(),
                "x": click.x,
                "y": click.y,
                "screen": format!("{:?}", current_screen),
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
                crate::action_log::log(
                    "Fleet",
                    "click",
                    &format!("{:?} -> Some({})", prev, fleet),
                );
                *guard = Some(fleet);
            }
            drop(guard);
            // Emit unconditionally so the kantai window stays in sync even on
            // repeated clicks of the same tab.
            let _ = app.emit(crate::events::FLEET_VIEW_CHANGED, fleet);
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
            let snap_period =
                crate::lock_or_recover(
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
            if let Ok(permit) = screenshot_semaphore.clone().try_acquire_owned() {
                let ss_dir = screenshot_dir.clone();
                let app_for_ss = app.clone();
                let ts_for_ss = click_ts.clone();
                let cx = click.x;
                let cy = click.y;
                tokio::task::spawn_blocking(move || {
                    match capture_and_crop(&ss_dir, cx, cy) {
                        Ok(Some(bytes)) => {
                            use base64::Engine;
                            let b64 =
                                base64::engine::general_purpose::STANDARD.encode(&bytes);
                            let _ = app_for_ss.emit(
                                crate::events::CLICK_SCREENSHOT,
                                serde_json::json!({
                                    "ts": ts_for_ss,
                                    "x": cx,
                                    "y": cy,
                                    "image": format!("data:image/png;base64,{}", b64),
                                }),
                            );
                        }
                        Ok(None) => {
                            log::warn!(
                                "[MouseHook] No crop produced for click ({}, {})",
                                cx, cy
                            );
                        }
                        Err(e) => log::warn!("[MouseHook] Screenshot failed: {}", e),
                    }
                    drop(permit);
                });
            } else {
                log::debug!("[MouseHook] Screenshot in flight, skipping ({}, {})", click.x, click.y);
            }
        }
    }

    info!("[MouseHook] Click consumer task ended");
}

/// Capture game window screenshot using PrintWindow and crop around click point.
/// PrintWindow with PW_RENDERFULLCONTENT captures WebView2 GPU-rendered content.
/// Returns the cropped PNG bytes (None if the click was at a window edge with
/// no usable crop area).
#[cfg(all(target_os = "windows", debug_assertions))]
fn capture_and_crop(
    screenshot_dir: &std::path::Path,
    click_x: i32,
    click_y: i32,
) -> Result<Option<Vec<u8>>, String> {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::Graphics::Gdi::{
        CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, GetDC,
        ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
    };
    use windows_sys::Win32::Storage::Xps::PrintWindow;
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetClientRect, PW_RENDERFULLCONTENT};

    let hwnd = inner::GAME_HWND.load(std::sync::atomic::Ordering::Relaxed) as HWND;
    if hwnd.is_null() {
        return Err("No game window".to_string());
    }

    unsafe {
        let mut rect: windows_sys::Win32::Foundation::RECT = std::mem::zeroed();
        if GetClientRect(hwnd, &mut rect) == 0 {
            return Err("GetClientRect failed".to_string());
        }

        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;
        if width <= 0 || height <= 0 {
            return Err("Invalid window size".to_string());
        }

        let hdc_window = GetDC(hwnd);
        if hdc_window.is_null() {
            return Err("GetDC failed".to_string());
        }

        let hdc_mem = CreateCompatibleDC(hdc_window);
        let hbm = CreateCompatibleBitmap(hdc_window, width, height);
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
        bmi.bmiHeader.biWidth = width;
        bmi.bmiHeader.biHeight = -height; // top-down
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB;

        let pixel_count = (width * height) as usize;
        let mut pixels: Vec<u8> = vec![0u8; pixel_count * 4];

        GetDIBits(
            hdc_mem,
            hbm,
            0,
            height as u32,
            pixels.as_mut_ptr() as *mut _,
            &mut bmi,
            DIB_RGB_COLORS,
        );

        SelectObject(hdc_mem, old_bm);
        DeleteObject(hbm);
        DeleteDC(hdc_mem);
        ReleaseDC(hwnd, hdc_window);

        // BGRA → RGBA
        for chunk in pixels.chunks_exact_mut(4) {
            chunk.swap(0, 2);
        }

        let img: image::RgbaImage =
            image::ImageBuffer::from_raw(width as u32, height as u32, pixels)
                .ok_or("Failed to create image buffer")?;

        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S_%3f").to_string();

        // Full screenshot
        let full_path = screenshot_dir.join(format!("full_{}.png", timestamp));
        img.save(&full_path).map_err(|e| format!("Save full: {}", e))?;

        // Crop 100px region around click
        let margin = 100i32;
        let left = (click_x - margin).max(0) as u32;
        let top = (click_y - margin).max(0) as u32;
        let right = ((click_x + margin).min(width)) as u32;
        let bottom = ((click_y + margin).min(height)) as u32;
        let crop_w = right.saturating_sub(left);
        let crop_h = bottom.saturating_sub(top);

        if crop_w > 0 && crop_h > 0 {
            let cropped =
                image::DynamicImage::ImageRgba8(img).crop_imm(left, top, crop_w, crop_h);
            let crop_path = screenshot_dir.join(format!("crop_{}.png", timestamp));
            cropped
                .save(&crop_path)
                .map_err(|e| format!("Save crop: {}", e))?;

            log::info!(
                "[MouseHook] Screenshot: full_{}.png + crop_{}.png (click {}, {})",
                timestamp, timestamp, click_x, click_y
            );

            // Read back the cropped PNG bytes to forward to the Debug UI.
            let bytes = std::fs::read(&crop_path).map_err(|e| format!("Read crop: {}", e))?;
            return Ok(Some(bytes));
        }

        log::info!(
            "[MouseHook] Screenshot: full_{}.png (no crop, click {}, {})",
            timestamp, click_x, click_y
        );
    }

    Ok(None)
}

// No-op stubs for non-Windows
#[cfg(not(target_os = "windows"))]
pub fn uninstall() {}
