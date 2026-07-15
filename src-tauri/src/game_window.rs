use log::info;
use std::sync::atomic::Ordering;
use tauri::{Manager, State, WebviewBuilder, WebviewUrl, WindowBuilder};
use url::Url;

use crate::AppState;

/// DMM host-page shim template: isolate the game frame and add our control bar.
/// Placeholders are expanded from the Rust layout constants below.
const GAME_INIT_SCRIPT_TEMPLATE: &str = include_str!("game_init.js");

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

fn build_game_init_script() -> String {
    GAME_INIT_SCRIPT_TEMPLATE
        .replace("__KC_GAME_WIDTH__", &GAME_WIDTH.to_string())
        .replace("__KC_GAME_HEIGHT__", &GAME_HEIGHT.to_string())
        .replace("__KC_CONTROL_BAR_HEIGHT__", &CONTROL_BAR_HEIGHT.to_string())
        .replace(
            "__KC_LAYOUT_DIAGNOSTICS__",
            if cfg!(debug_assertions) { "true" } else { "false" },
        )
}

fn create_clickthrough_window(
    app: &tauri::AppHandle,
    window_label: &str,
    webview_label: &str,
    page: &str,
    width: f64,
    height: f64,
) -> Result<(), String> {
    info!("Creating {} window", window_label);
    let window = WindowBuilder::new(app, window_label)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .visible(false)
        .skip_taskbar(true)
        .inner_size(width, height)
        .build()
        .map_err(|error| format!("Failed to create {window_label}: {error}"))?;

    window
        .set_ignore_cursor_events(true)
        .map_err(|error| format!("Failed to make {window_label} click-through: {error}"))?;
    window
        .add_child(
            WebviewBuilder::new(webview_label, WebviewUrl::App(page.into())).transparent(true),
            tauri::LogicalPosition::new(0.0, 0.0),
            tauri::LogicalSize::new(width, height),
        )
        .map_err(|error| format!("Failed to create {webview_label}: {error}"))?;
    info!("{} window created", window_label);
    Ok(())
}

/// Open the KanColle game in a separate window with proxy configured.
/// Uses multi-webview: game-content (game) + game-overlay (transparent overlay).
#[tauri::command]
pub(crate) async fn open_game_window(app: tauri::AppHandle) -> Result<(), String> {
    crate::action_log::record("Command", "open_game_window", None);
    // Check if game window already exists
    if app.get_window("game").is_some() {
        if let Some(win) = app.get_window("game") {
            win.set_focus().map_err(|e| e.to_string())?;
        }
        return Ok(());
    }

    // Get the proxy port from app state
    let state = app.state::<AppState>();
    let proxy_port = *crate::lock_or_recover(&state.runtime.proxy_port, "proxy_port");

    if proxy_port == 0 {
        return Err("Proxy is not ready yet. Please wait and try again.".to_string());
    }

    #[cfg(target_os = "macos")]
    let proxy_url =
        Url::parse(&format!("http://127.0.0.1:{}", proxy_port)).map_err(|e| e.to_string())?;

    info!("Opening game window with proxy: http://127.0.0.1:{}", proxy_port);

    // Use a persistent data store so cookies/sessions survive across app restarts.
    // Windows: data_directory (file-based WebView2 profile)
    // macOS: data_store_identifier (WKWebsiteDataStore, requires macOS >= 14)
    #[cfg(not(target_os = "macos"))]
    let data_dir = app
        .path()
        .app_local_data_dir()
        .map(|d| d.join("local").join("game-webview"))
        .map_err(|e| e.to_string())?;

    // Start with about:blank so cookies can be restored before DMM loads.
    let game_url: Url = "about:blank".parse().unwrap();
    let app_handle = app.clone();

    // Cookie restore strategy:
    // - Windows: WebView2's data_directory profile persists session cookies across
    //   restarts; the injected document.cookie script is kept as a best-effort fallback.
    // - macOS: WKWebView drops session cookies on exit and JS injection cannot set
    //   them (opaque origin / cross-domain / httpOnly), so saved cookies are written
    //   natively into WKHTTPCookieStore before navigating to DMM (see spawn below).
    let game_init_script = build_game_init_script();

    #[cfg(not(target_os = "macos"))]
    let final_init_script = {
        let restore_script = crate::cookie::build_cookie_restore_script(&app).await;
        format!("{}\n{}", game_init_script, restore_script)
    };
    #[cfg(target_os = "macos")]
    let final_init_script = game_init_script;

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
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/132.0.0.0 Safari/537.36 Edg/132.0.0.0")
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
        // wry overrides proxy_url when additional_browser_args is set, so we build args manually
        // to combine proxy + bypass list. DMM domains bypass the proxy because hudsucker tunneling
        // to play.games.dmm.com is unreliable; kancolle-server.com still goes through the proxy
        // for API intercept.
        let browser_args = format!(
            "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection \
             --proxy-server=http://127.0.0.1:{} \
             --proxy-bypass-list=*.dmm.com;*.dmm-corp.com;*.dmm.co.jp;*.dmmgames.com",
            proxy_port
        );
        game_wv_builder = game_wv_builder.additional_browser_args(&browser_args);
    }

    // macOS: use a fixed data_store_identifier for persistent WKWebsiteDataStore (macOS >= 14)
    // This persists cookies (including httpOnly), sessions, and cache natively.
    #[cfg(target_os = "macos")]
    {
        game_wv_builder = game_wv_builder.proxy_url(proxy_url);
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

    create_clickthrough_window(
        &app,
        "formation-hint",
        "formation-hint-content",
        "formation-hint.html",
        200.0,
        170.0,
    )?;
    create_clickthrough_window(
        &app,
        "battle-info",
        "battle-info-content",
        "battle-info.html",
        520.0,
        140.0,
    )?;
    create_clickthrough_window(
        &app,
        "expedition-notify",
        "expedition-notify-content",
        "expedition-notify.html",
        250.0,
        100.0,
    )?;

    // Sync game webview on resize, reposition formation hint on move/resize.
    // Closing the game window terminates the whole app — game window is the
    // primary surface; the management SPA is just an auxiliary panel.
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
                if resize_app
                    .state::<AppState>()
                    .prefs
                    .minimap_enabled
                    .load(Ordering::Relaxed)
                {
                    let _ = crate::overlay::show_minimap_overlay(&resize_app);
                }
                // Reposition expedition notification if visible
                crate::overlay::reposition_expedition_notification(&resize_app);
                // Reposition battle info overlay if visible
                crate::overlay::reposition_battle_info(&resize_app);
            }
            tauri::WindowEvent::Moved(_) => {
                crate::overlay::reposition_formation_hint(&resize_app);
                crate::overlay::reposition_expedition_notification(&resize_app);
                crate::overlay::reposition_battle_info(&resize_app);
            }
            tauri::WindowEvent::CloseRequested { .. } => {
                info!("Game window close requested -> exiting app");
                resize_app.exit(0);
            }
            _ => {}
        }
    });

    // Restore cookies (macOS: natively into WKHTTPCookieStore), give the Cookie
    // Manager time to settle, then navigate to DMM.
    let game_wv_clone = game_webview.clone();
    #[cfg(target_os = "macos")]
    let restore_app = app.clone();
    tauri::async_runtime::spawn(async move {
        #[cfg(target_os = "macos")]
        crate::cookie::restore_cookies_native(&restore_app, &game_wv_clone).await;
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        let actual_url: Url = "https://play.games.dmm.com/game/kancolle".parse().unwrap();
        if let Err(e) = game_wv_clone.navigate(actual_url) {
            log::error!("Failed to navigate to DMM: {}", e);
        }
    });

    // Install OS-level mouse hook for click tracking (Windows only)
    #[cfg(target_os = "windows")]
    {
        let hwnd = game_window.hwnd().map_err(|e| e.to_string())?;
        let data_dir = app
            .path()
            .app_local_data_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."));
        match crate::mouse_hook::install(hwnd.0 as isize) {
            Ok(rx) => {
                info!("Mouse hook installed for game window");
                // Spawn consumer task for click events
                let click_app = app.clone();
                tauri::async_runtime::spawn(async move {
                    crate::mouse_hook::consume_clicks(rx, click_app, data_dir).await;
                });
            }
            Err(e) => {
                log::warn!("Failed to install mouse hook: {}", e);
            }
        }
    }

    info!("Game window opened with proxy on port {}", proxy_port);
    Ok(())
}

/// Close the game window
#[tauri::command]
pub(crate) async fn close_game_window(app: tauri::AppHandle) -> Result<(), String> {
    crate::action_log::record("Command", "close_game_window", None);
    crate::mouse_hook::uninstall();
    info!("Closing game window and child windows");
    if let Some(hint_win) = app.get_window("formation-hint") {
        if let Err(e) = hint_win.close() {
            log::warn!("Failed to close formation-hint: {}", e);
        }
    }
    if let Some(battle_info_win) = app.get_window("battle-info") {
        if let Err(e) = battle_info_win.close() {
            log::warn!("Failed to close battle-info: {}", e);
        }
    }
    if let Some(notify_win) = app.get_window("expedition-notify") {
        if let Err(e) = notify_win.close() {
            log::warn!("Failed to close expedition-notify: {}", e);
        }
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
        *crate::lock_or_recover(&state.overlay.game_zoom, "game_zoom") = zoom;
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
    if app
        .state::<AppState>()
        .prefs
        .minimap_enabled
        .load(Ordering::Relaxed)
    {
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
    state.prefs.game_muted.store(muted, Ordering::Relaxed);

    crate::settings::persist_flag(&app, crate::settings::GAME_MUTED, muted)
        .map_err(|error| format!("Failed to persist game mute setting: {error}"))?;

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
    state.prefs.game_muted.load(Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_init_script_expands_rust_layout_constants() {
        let script = build_game_init_script();
        assert!(!script.contains("__KC_"));
        assert!(script.contains("width: 1200px"));
        assert!(script.contains("height: 720px"));
        assert!(script.contains("top: 28px"));
    }
}
