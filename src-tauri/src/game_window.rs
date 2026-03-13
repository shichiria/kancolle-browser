use log::info;
use std::sync::atomic::Ordering;
use tauri::{Manager, State, WebviewBuilder, WebviewUrl, WindowBuilder};
use url::Url;

use crate::AppState;

/// JavaScript injection: hide DMM UI, show game frame, add control bar overlay
const GAME_INIT_SCRIPT: &str = include_str!("game_init.js");

/// KanColle game native resolution
pub(crate) const GAME_WIDTH: f64 = 1200.0;
pub(crate) const GAME_HEIGHT: f64 = 720.0;
/// Height of the injected control bar (pixels, not scaled by zoom)
pub(crate) const CONTROL_BAR_HEIGHT: f64 = 28.0;
/// macOS title bar height — tao/tauri includes titlebar in inner_size on macOS (tauri-apps/tauri#6333)
#[cfg(target_os = "macos")]
pub(crate) const MACOS_TITLEBAR_HEIGHT: f64 = 28.0;
#[cfg(not(target_os = "macos"))]
pub(crate) const MACOS_TITLEBAR_HEIGHT: f64 = 0.0;

/// Open the KanColle game in a separate window with proxy configured.
/// Uses multi-webview: game-content (game) + game-overlay (transparent overlay).
#[tauri::command]
pub(crate) async fn open_game_window(app: tauri::AppHandle) -> Result<(), String> {
    // Check if game window already exists
    if app.get_window("game").is_some() {
        if let Some(win) = app.get_window("game") {
            win.set_focus().map_err(|e| e.to_string())?;
        }
        return Ok(());
    }

    // Get the proxy port from app state
    let state = app.state::<AppState>();
    let proxy_port = *state.proxy_port.lock().unwrap();

    if proxy_port == 0 {
        return Err("Proxy is not ready yet. Please wait and try again.".to_string());
    }

    let proxy_url =
        Url::parse(&format!("http://127.0.0.1:{}", proxy_port)).map_err(|e| e.to_string())?;

    info!("Opening game window with proxy: {}", proxy_url);

    // Use a persistent data store so cookies/sessions survive across app restarts.
    // Windows: data_directory (file-based WebView2 profile)
    // macOS: data_store_identifier (WKWebsiteDataStore, requires macOS >= 14)
    #[cfg(not(target_os = "macos"))]
    let data_dir = app
        .path()
        .app_local_data_dir()
        .map(|d| d.join("local").join("game-webview"))
        .map_err(|e| e.to_string())?;

    // Start with about:blank so we can inject cookies into the global Cookie Manager before DMM loads.
    let game_url: Url = "about:blank".parse().unwrap();
    let app_handle = app.clone();

    // Append the cookie restoration script to the default initialization script
    let restore_script = crate::cookie::build_cookie_restore_script(&app).await;
    let final_init_script = format!("{}\n{}", GAME_INIT_SCRIPT, restore_script);

    let win_width = GAME_WIDTH;
    let win_height = GAME_HEIGHT + CONTROL_BAR_HEIGHT + MACOS_TITLEBAR_HEIGHT;

    // Create the window (without a built-in webview)
    let game_window = WindowBuilder::new(&app, "game")
        .title("KanColle")
        .inner_size(win_width, win_height)
        .min_inner_size(GAME_WIDTH * 0.5, GAME_HEIGHT * 0.5 + CONTROL_BAR_HEIGHT + MACOS_TITLEBAR_HEIGHT)
        .build()
        .map_err(|e| e.to_string())?;

    // Add game webview (bottom layer)
    let mut game_wv_builder =
        WebviewBuilder::new("game-content", WebviewUrl::External(game_url))
            .proxy_url(proxy_url)
            .initialization_script(&final_init_script)
            .on_navigation(move |nav_url| {
                let url_str = nav_url.to_string();
                info!("Game navigation: {}", url_str);
                if url_str.contains("dmm.com") {
                    let handle = app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                        match crate::cookie::save_game_cookies(handle).await {
                            Ok(n) => info!("Auto-saved {} cookies after navigation", n),
                            Err(e) => log::warn!("Failed to auto-save cookies: {}", e),
                        }
                    });
                }
                true
            });

    #[cfg(not(target_os = "macos"))]
    {
        game_wv_builder = game_wv_builder.data_directory(data_dir);
    }

    // macOS: use a fixed data_store_identifier for persistent WKWebsiteDataStore (macOS >= 14)
    // This persists cookies (including httpOnly), sessions, and cache natively.
    #[cfg(target_os = "macos")]
    {
        // Fixed UUID: "kancolle-browser-game" as deterministic 16-byte identifier
        const GAME_DATA_STORE_ID: [u8; 16] = [
            0x6b, 0x61, 0x6e, 0x63, 0x6f, 0x6c, 0x6c, 0x65, // "kancolle"
            0x2d, 0x62, 0x72, 0x6f, 0x77, 0x73, 0x65, 0x72, // "-browser"
        ];
        game_wv_builder = game_wv_builder.data_store_identifier(GAME_DATA_STORE_ID);
    }

    let game_webview = game_window
        .add_child(
            game_wv_builder,
            tauri::LogicalPosition::new(0.0, 0.0),
            tauri::LogicalSize::new(win_width, win_height),
        )
        .map_err(|e| e.to_string())?;

    // Add overlay webview (top layer, transparent, hidden by default via 1x1 size)
    let _overlay = game_window
        .add_child(
            WebviewBuilder::new("game-overlay", WebviewUrl::App("overlay.html".into()))
                .transparent(true),
            tauri::LogicalPosition::new(0.0, 0.0),
            tauri::LogicalSize::new(1.0, 1.0),
        )
        .map_err(|e| e.to_string())?;

    // Create click-through formation hint window (separate window so it doesn't block game input)
    let hint_win = WindowBuilder::new(&app, "formation-hint")
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .visible(false)
        .skip_taskbar(true)
        .inner_size(200.0, 170.0)
        .build()
        .map_err(|e| e.to_string())?;

    hint_win
        .set_ignore_cursor_events(true)
        .map_err(|e| e.to_string())?;

    let _hint_wv = hint_win
        .add_child(
            WebviewBuilder::new(
                "formation-hint-content",
                WebviewUrl::App("formation-hint.html".into()),
            )
            .transparent(true),
            tauri::LogicalPosition::new(0.0, 0.0),
            tauri::LogicalSize::new(200.0, 170.0),
        )
        .map_err(|e| e.to_string())?;

    // Create expedition notification window (click-through, transparent)
    let notify_win = WindowBuilder::new(&app, "expedition-notify")
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .visible(false)
        .skip_taskbar(true)
        .inner_size(250.0, 100.0)
        .build()
        .map_err(|e| e.to_string())?;

    notify_win
        .set_ignore_cursor_events(true)
        .map_err(|e| e.to_string())?;

    let _notify_wv = notify_win
        .add_child(
            WebviewBuilder::new(
                "expedition-notify-content",
                WebviewUrl::App("expedition-notify.html".into()),
            )
            .transparent(true),
            tauri::LogicalPosition::new(0.0, 0.0),
            tauri::LogicalSize::new(250.0, 100.0),
        )
        .map_err(|e| e.to_string())?;

    // Sync game webview on resize, reposition formation hint on move/resize
    let resize_app = app.clone();
    game_window.on_window_event(move |event| {
        match event {
            tauri::WindowEvent::Resized(size) => {
                if let Some(wv) = resize_app.get_webview("game-content") {
                    let _ = wv.set_size(*size);
                }
                // Reposition formation hint
                crate::overlay::reposition_formation_hint(&resize_app);
                // Reposition minimap if enabled
                if resize_app.state::<AppState>().minimap_enabled.load(Ordering::Relaxed) {
                    let _ = crate::overlay::show_minimap_overlay(&resize_app);
                }
                // Reposition expedition notification if visible
                crate::overlay::reposition_expedition_notification(&resize_app);
            }
            tauri::WindowEvent::Moved(_) => {
                crate::overlay::reposition_formation_hint(&resize_app);
                crate::overlay::reposition_expedition_notification(&resize_app);
            }
            _ => {}
        }
    });

    // Give the Cookie Manager time to process injected cookies, then navigate to DMM
    let game_wv_clone = game_webview.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        let actual_url: Url = "https://play.games.dmm.com/game/kancolle".parse().unwrap();
        if let Err(e) = game_wv_clone.navigate(actual_url) {
            log::error!("Failed to navigate to DMM: {}", e);
        }
    });

    info!("Game window opened with proxy on port {}", proxy_port);
    Ok(())
}

/// Close the game window
#[tauri::command]
pub(crate) async fn close_game_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(hint_win) = app.get_window("formation-hint") {
        let _ = hint_win.close();
    }
    if let Some(notify_win) = app.get_window("expedition-notify") {
        let _ = notify_win.close();
    }
    if let Some(win) = app.get_window("game") {
        // Force save cookies immediately before closing
        match crate::cookie::save_game_cookies(app.clone()).await {
            Ok(n) => info!("Saved {} cookies on explicit close", n),
            Err(e) => log::warn!("Failed to save cookies on close: {}", e),
        }
        win.close().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Set zoom level for the game window and resize the window accordingly
#[tauri::command]
pub(crate) fn set_game_zoom(app: tauri::AppHandle, zoom: f64) -> Result<(), String> {
    let game_wv = app
        .get_webview("game-content")
        .ok_or("Game webview not found")?;
    let win = app
        .get_window("game")
        .ok_or("Game window not found")?;

    // Save zoom level to AppState
    if let Some(state) = app.try_state::<AppState>() {
        *state.game_zoom.lock().unwrap() = zoom;
    }

    // Set webview zoom
    game_wv.set_zoom(zoom).map_err(|e| e.to_string())?;

    // Resize the window to fit the zoomed game + control bar + macOS titlebar compensation
    let new_width = GAME_WIDTH * zoom;
    let new_height = GAME_HEIGHT * zoom + CONTROL_BAR_HEIGHT + MACOS_TITLEBAR_HEIGHT;
    let size = tauri::LogicalSize::new(new_width, new_height);
    win.set_size(size).map_err(|e| e.to_string())?;

    // Resize game webview to match (on_window_event also handles this)
    // NOTE: Do NOT resize overlay here — overlay is 1x1 when hidden and only
    // expanded by set_overlay_visible(). Expanding it here blocks game clicks.
    let wv_size = tauri::LogicalSize::new(new_width, new_height);
    let _ = game_wv.set_size(wv_size);

    // Reposition minimap if enabled
    if app.state::<AppState>().minimap_enabled.load(Ordering::Relaxed) {
        let _ = crate::overlay::show_minimap_overlay(&app);
    }

    info!(
        "Game zoom set to {}% ({}x{})",
        (zoom * 100.0) as i32,
        new_width as i32,
        new_height as i32
    );
    Ok(())
}

/// Toggle mute on the game window using native WebView API
#[tauri::command]
pub(crate) fn toggle_game_mute(
    app: tauri::AppHandle,
    state: State<AppState>,
    muted: bool,
) -> Result<(), String> {
    state.game_muted.store(muted, Ordering::Relaxed);

    // Persist to disk so mute survives app restart
    if let Ok(dir) = app.path().app_local_data_dir() {
        let _ = std::fs::write(
            dir.join("local").join("game_muted"),
            if muted { "1" } else { "0" },
        );
    }

    let game_wv = app
        .get_webview("game-content")
        .ok_or("Game webview not found")?;

    #[cfg(target_os = "macos")]
    {
        use objc2::msg_send;
        use objc2::runtime::AnyObject;

        let muted_state: u64 = if muted { 1 } else { 0 }; // _WKMediaAudioMuted = 1 << 0
        game_wv.with_webview(move |webview| unsafe {
            let wk: *mut AnyObject = webview.inner().cast();
            let _: () = msg_send![wk, _setPageMuted: muted_state];
        })
        .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "windows")]
    {
        use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2_8;
        use windows_core::Interface;

        game_wv.with_webview(move |webview| unsafe {
            let controller = webview.controller();
            if let Ok(core) = controller.CoreWebView2() {
                if let Ok(core8) = core.cast::<ICoreWebView2_8>() {
                    let _ = core8.SetIsMuted(muted);
                }
            }
        })
        .map_err(|e| e.to_string())?;
    }

    info!("Game mute set to {}", muted);
    Ok(())
}

/// Get the current mute state (for init script to restore UI)
#[tauri::command]
pub(crate) fn get_game_mute(state: State<AppState>) -> bool {
    state.game_muted.load(Ordering::Relaxed)
}
