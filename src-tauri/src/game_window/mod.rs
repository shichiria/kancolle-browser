mod platform;
pub(crate) mod screenshot;
#[cfg(target_os = "windows")]
mod webview_diagnostics;

use log::info;
use std::sync::atomic::{AtomicU64, Ordering};
use tauri::webview::{NewWindowFeatures, NewWindowResponse};
use tauri::{
    Manager, State, Webview, WebviewBuilder, WebviewUrl, WebviewWindowBuilder, Window,
    WindowBuilder, Wry,
};
use url::Url;

use crate::AppState;

const GAME_INIT_SCRIPT_TEMPLATE: &str = include_str!("../game_init.js");
const GAME_URL: &str = "https://play.games.dmm.com/game/kancolle";
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/132.0.0.0 Safari/537.36 Edg/132.0.0.0";
static POPUP_SEQUENCE: AtomicU64 = AtomicU64::new(1);

pub(crate) const GAME_WIDTH: f64 = 1200.0;
pub(crate) const GAME_HEIGHT: f64 = 720.0;
pub(crate) const CONTROL_BAR_HEIGHT: f64 = 28.0;
pub(crate) const MACOS_TITLEBAR_HEIGHT: f64 = platform::TITLEBAR_HEIGHT;

fn build_game_init_script() -> String {
    GAME_INIT_SCRIPT_TEMPLATE
        .replace("__KC_GAME_WIDTH__", &GAME_WIDTH.to_string())
        .replace("__KC_GAME_HEIGHT__", &GAME_HEIGHT.to_string())
        .replace("__KC_CONTROL_BAR_HEIGHT__", &CONTROL_BAR_HEIGHT.to_string())
        .replace(
            "__KC_LAYOUT_DIAGNOSTICS__",
            if cfg!(debug_assertions) {
                "true"
            } else {
                "false"
            },
        )
}

fn focus_existing_game(app: &tauri::AppHandle) -> Result<bool, String> {
    let Some(window) = app.get_window("game") else {
        return Ok(false);
    };
    window.set_focus().map_err(|error| error.to_string())?;
    Ok(true)
}

fn proxy_port(app: &tauri::AppHandle) -> Result<u16, String> {
    let state = app.state::<AppState>();
    let port = *crate::lock_or_recover(&state.runtime.proxy_port, "proxy_port");
    if port == 0 {
        Err("Proxy is not ready yet. Please wait and try again.".to_string())
    } else {
        Ok(port)
    }
}

fn create_game_window(app: &tauri::AppHandle) -> Result<Window<Wry>, String> {
    let width = GAME_WIDTH;
    let height = GAME_HEIGHT + CONTROL_BAR_HEIGHT + MACOS_TITLEBAR_HEIGHT;
    WindowBuilder::new(app, "game")
        .title("KanColle")
        .inner_size(width, height)
        .min_inner_size(
            GAME_WIDTH * 0.5,
            GAME_HEIGHT * 0.5 + CONTROL_BAR_HEIGHT + MACOS_TITLEBAR_HEIGHT,
        )
        .build()
        .map_err(|error| error.to_string())
}

fn is_allowed_popup_url(url: &Url) -> bool {
    url.scheme() == "https"
}

fn is_point_charge_return_url(url: &Url) -> bool {
    if url.scheme() != "https" || url.host_str() != Some("point.dmm.com") {
        return false;
    }

    match url.path() {
        "/window/close" | "/close.html" => true,
        "/close" => url
            .query_pairs()
            .any(|(key, value)| key == "basket_service_type" && value == "freegame"),
        _ => false,
    }
}

fn open_requested_window(
    app: &tauri::AppHandle,
    url: Url,
    features: NewWindowFeatures,
) -> NewWindowResponse<Wry> {
    if !is_allowed_popup_url(&url) {
        log::warn!("Denied game popup with unsupported URL scheme: {url}");
        return NewWindowResponse::Deny;
    }

    let sequence = POPUP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let label = format!("game-popup-{sequence}");
    info!("Game popup requested: label={label}, url={url}");

    let blank_url = match Url::parse("about:blank") {
        Ok(url) => url,
        Err(error) => {
            log::error!("Failed to prepare game popup URL: {error}");
            return NewWindowResponse::Deny;
        }
    };
    let popup_url = url.clone();
    let builder = WebviewWindowBuilder::new(app, label.as_str(), WebviewUrl::External(blank_url))
        .title("DMM")
        .inner_size(1000.0, 800.0)
        // Carries the opener's WebView2 environment on Windows, so DMM cookies
        // and the configured proxy remain available to the point-charge page.
        .window_features(features)
        .on_navigation(move |navigated_url| {
            info!("Game popup navigation: {navigated_url}");
            navigated_url.scheme() == "https" || navigated_url.as_str() == "about:blank"
        })
        .on_page_load(|window, payload| {
            if is_point_charge_return_url(payload.url()) {
                info!(
                    "Point charge returned to service; closing popup: {}",
                    payload.url()
                );
                if let Err(error) = window.close() {
                    log::warn!("Failed to close completed point-charge popup: {error}");
                }
            }
        })
        .on_document_title_changed(|window, title| {
            if let Err(error) = window.set_title(&title) {
                log::warn!("Failed to update game popup title: {error}");
            }
        });

    match builder.build() {
        Ok(window) => {
            info!("Game popup created: label={label}, url={popup_url}");
            NewWindowResponse::Create { window }
        }
        Err(error) => {
            log::error!("Failed to create game popup for {popup_url}: {error}");
            NewWindowResponse::Deny
        }
    }
}

async fn create_game_webview(
    app: &tauri::AppHandle,
    game_window: &Window<Wry>,
    proxy_port: u16,
) -> Result<Webview<Wry>, String> {
    let blank_url = Url::parse("about:blank").map_err(|error| error.to_string())?;
    let init_script = platform::initialization_script(app, build_game_init_script()).await;
    let navigation_app = app.clone();
    let popup_app = app.clone();
    let builder = WebviewBuilder::new("game-content", WebviewUrl::External(blank_url))
        .user_agent(USER_AGENT)
        .initialization_script(&init_script)
        .on_navigation(move |url| {
            info!("Game navigation: {url}");
            if url.as_str().contains("dmm.com") {
                schedule_cookie_save(navigation_app.clone());
            }
            true
        })
        .on_new_window(move |url, features| open_requested_window(&popup_app, url, features));
    let builder = platform::configure_webview(builder, app, proxy_port)?;

    let webview = game_window
        .add_child(
            builder,
            tauri::LogicalPosition::new(0.0, 0.0),
            tauri::LogicalSize::new(
                GAME_WIDTH,
                GAME_HEIGHT + CONTROL_BAR_HEIGHT + MACOS_TITLEBAR_HEIGHT,
            ),
        )
        .map_err(|error| error.to_string())?;
    platform::install_diagnostics(&webview)?;
    Ok(webview)
}

fn schedule_cookie_save(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        match crate::cookie::save_game_cookies(app).await {
            Ok(count) => info!("Auto-saved {count} cookies after navigation"),
            Err(error) => log::warn!("Failed to auto-save cookies: {error}"),
        }
    });
}

fn add_overlay_webview(game_window: &Window<Wry>) -> Result<(), String> {
    game_window
        .add_child(
            WebviewBuilder::new("game-overlay", WebviewUrl::App("overlay.html".into()))
                .transparent(true),
            tauri::LogicalPosition::new(0.0, 0.0),
            tauri::LogicalSize::new(1.0, 1.0),
        )
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn create_clickthrough_window(
    app: &tauri::AppHandle,
    window_label: &str,
    webview_label: &str,
    page: &str,
    width: f64,
    height: f64,
) -> Result<(), String> {
    info!("Creating {window_label} window");
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
    info!("{window_label} window created");
    Ok(())
}

fn create_auxiliary_windows(app: &tauri::AppHandle) -> Result<(), String> {
    for (window, webview, page, width, height) in [
        (
            "formation-hint",
            "formation-hint-content",
            "formation-hint.html",
            200.0,
            170.0,
        ),
        (
            "battle-info",
            "battle-info-content",
            "battle-info.html",
            520.0,
            140.0,
        ),
        (
            "expedition-notify",
            "expedition-notify-content",
            "expedition-notify.html",
            250.0,
            100.0,
        ),
        (
            "exercise-notify",
            "exercise-notify-content",
            "exercise-notify.html",
            360.0,
            70.0,
        ),
        (
            "nozaki-timer",
            "nozaki-timer-content",
            "nozaki-timer.html",
            220.0,
            44.0,
        ),
    ] {
        create_clickthrough_window(app, window, webview, page, width, height)?;
    }
    Ok(())
}

fn register_window_events(app: &tauri::AppHandle, game_window: &Window<Wry>) {
    let event_app = app.clone();
    game_window.on_window_event(move |event| match event {
        tauri::WindowEvent::Resized(size) => {
            if let Some(webview) = event_app.get_webview("game-content") {
                let _ = webview.set_size(*size);
            }
            crate::overlay::reposition_formation_hint(&event_app);
            if event_app
                .state::<AppState>()
                .prefs
                .minimap_enabled
                .load(Ordering::Relaxed)
            {
                let _ = crate::overlay::show_minimap_overlay(&event_app);
            }
            crate::overlay::reposition_expedition_notification(&event_app);
            crate::overlay::reposition_exercise_notification(&event_app);
            crate::overlay::reposition_battle_info(&event_app);
            crate::nozaki_timer::reposition_overlay(&event_app);
        }
        tauri::WindowEvent::Moved(_) => {
            crate::overlay::reposition_formation_hint(&event_app);
            crate::overlay::reposition_expedition_notification(&event_app);
            crate::overlay::reposition_exercise_notification(&event_app);
            crate::overlay::reposition_battle_info(&event_app);
            crate::nozaki_timer::reposition_overlay(&event_app);
        }
        tauri::WindowEvent::CloseRequested { .. } => {
            info!("Game window close requested -> exiting app");
            event_app.exit(0);
        }
        _ => {}
    });
}

fn navigate_to_game(app: tauri::AppHandle, webview: Webview<Wry>) {
    tauri::async_runtime::spawn(async move {
        platform::prepare_navigation(&app, &webview).await;
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        let url = match Url::parse(GAME_URL) {
            Ok(url) => url,
            Err(error) => {
                log::error!("Invalid game URL: {error}");
                return;
            }
        };
        if let Err(error) = webview.navigate(url) {
            log::error!("Failed to navigate to DMM: {error}");
        }
    });
}

/// Open the KanColle game in a separate window with proxy configured.
#[tauri::command]
pub(crate) async fn open_game_window(app: tauri::AppHandle) -> Result<(), String> {
    crate::action_log::record("Command", "open_game_window", None);
    if focus_existing_game(&app)? {
        return Ok(());
    }

    let port = proxy_port(&app)?;
    info!("Opening game window with proxy: http://127.0.0.1:{port}");
    let game_window = create_game_window(&app)?;
    let game_webview = create_game_webview(&app, &game_window, port).await?;
    add_overlay_webview(&game_window)?;
    create_auxiliary_windows(&app)?;
    register_window_events(&app, &game_window);
    navigate_to_game(app.clone(), game_webview);
    platform::install_input_tracking(&app, &game_window)?;
    info!("Game window opened with proxy on port {port}");
    Ok(())
}

#[tauri::command]
pub(crate) async fn close_game_window(app: tauri::AppHandle) -> Result<(), String> {
    crate::action_log::record("Command", "close_game_window", None);
    crate::mouse_hook::uninstall();
    info!("Closing game window and child windows");
    for label in [
        "formation-hint",
        "battle-info",
        "expedition-notify",
        "exercise-notify",
        "nozaki-timer",
    ] {
        if let Some(window) = app.get_window(label) {
            if let Err(error) = window.close() {
                log::warn!("Failed to close {label}: {error}");
            }
        }
    }
    if let Some(window) = app.get_window("game") {
        match crate::cookie::save_game_cookies(app.clone()).await {
            Ok(count) => info!("Saved {count} cookies on explicit close"),
            Err(error) => log::warn!("Failed to save cookies on close: {error}"),
        }
        window.close().map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub(crate) fn set_game_zoom(app: tauri::AppHandle, zoom: f64) -> Result<(), String> {
    let game_webview = app
        .get_webview("game-content")
        .ok_or("Game webview not found")?;
    let window = app.get_window("game").ok_or("Game window not found")?;
    if let Some(state) = app.try_state::<AppState>() {
        *crate::lock_or_recover(&state.overlay.game_zoom, "game_zoom") = zoom;
    }

    game_webview
        .set_zoom(zoom)
        .map_err(|error| error.to_string())?;
    let width = GAME_WIDTH * zoom;
    let height = GAME_HEIGHT * zoom + CONTROL_BAR_HEIGHT + MACOS_TITLEBAR_HEIGHT;
    window
        .set_size(tauri::LogicalSize::new(width, height))
        .map_err(|error| error.to_string())?;
    let _ = game_webview.set_size(tauri::LogicalSize::new(width, height));

    if app
        .state::<AppState>()
        .prefs
        .minimap_enabled
        .load(Ordering::Relaxed)
    {
        let _ = crate::overlay::show_minimap_overlay(&app);
    }
    crate::nozaki_timer::reposition_overlay(&app);
    info!(
        "Game zoom set to {}% ({}x{})",
        (zoom * 100.0) as i32,
        width as i32,
        height as i32
    );
    Ok(())
}

#[tauri::command]
pub(crate) fn toggle_game_mute(
    app: tauri::AppHandle,
    state: State<AppState>,
    muted: bool,
) -> Result<(), String> {
    state.prefs.game_muted.store(muted, Ordering::Relaxed);
    crate::settings::persist_flag(&app, crate::settings::GAME_MUTED, muted)
        .map_err(|error| format!("Failed to persist game mute setting: {error}"))?;
    let webview = app
        .get_webview("game-content")
        .ok_or("Game webview not found")?;
    platform::set_muted(&webview, muted)?;
    info!("Game mute set to {muted}");
    Ok(())
}

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

    #[test]
    fn game_init_script_exits_child_frames_before_touching_browser_state() {
        let script = build_game_init_script();
        let child_frame_guard = script
            .find("if (!isTop) return;")
            .expect("initialization script must exit in child frames");
        let user_agent_override = script
            .find("Object.defineProperty(navigator, 'userAgentData'")
            .expect("user agent override must remain available to the DMM host page");
        let stylesheet_definition = script
            .find("var COMMON_CSS")
            .expect("DMM host-page stylesheet must remain available");

        assert!(child_frame_guard < user_agent_override);
        assert!(child_frame_guard < stylesheet_definition);
    }

    #[test]
    fn game_layout_keeps_dmm_purchase_dialog_above_the_game() {
        let script = build_game_init_script();

        assert!(script.contains("#game_frame"));
        assert!(script.contains("z-index: 1 !important"));
        assert!(!script.contains(
            "if (sibling !== node) sibling.style.setProperty('display', 'none', 'important')"
        ));
    }

    #[test]
    fn game_iframe_disables_its_own_scrollbars() {
        let script = build_game_init_script();

        assert!(script.contains("frame.setAttribute('scrolling', 'no')"));
    }

    #[test]
    fn game_control_bar_has_reload_button() {
        let script = build_game_init_script();

        assert!(script.contains("id=\"kc-reload\""));
        assert!(script.contains("window.location.reload()"));
    }

    #[test]
    fn game_control_bar_has_screenshot_button() {
        let script = build_game_init_script();

        assert!(script.contains("id=\"kc-screenshot\""));
        assert!(script.contains("invoke('take_game_screenshot')"));
    }

    #[test]
    fn game_control_bar_opens_event_window() {
        let script = build_game_init_script();

        assert!(script.contains("id=\"kc-event\""));
        assert!(script.contains("invoke('toggle_event_window')"));
    }

    #[test]
    fn event_map_shows_unsupplied_airbase_warning() {
        let script = build_game_init_script();

        assert!(script.contains("id = 'kc-airbase-supply-warning'"));
        assert!(script.contains("invoke('get_air_bases')"));
        assert!(script.contains("plane.count) < Number(plane.max_count"));
        assert!(script.contains("screen === 'SortieSelectEvent' && needsSupply"));
        assert!(script.contains("kc-airbase-supply-blink"));
    }

    #[test]
    fn game_window_has_exercise_refresh_warning() {
        let module = include_str!("mod.rs");
        let page = include_str!("../../../public/exercise-notify.html");

        assert!(module.contains("\"exercise-notify\""));
        assert!(module.contains("reposition_exercise_notification"));
        assert!(page.contains("window.showExerciseAlert"));
        assert!(page.contains("\\u6F14\\u7FD2\\u304C\\u672A\\u5B9F\\u65BD"));
    }

    #[test]
    fn game_window_has_support_timers() {
        let module = include_str!("mod.rs");
        let page = include_str!("../../../public/nozaki-timer.html");

        assert!(module.contains("\"nozaki-timer\""));
        assert!(module.contains("nozaki_timer::reposition_overlay"));
        assert!(page.contains("window.showSupportTimers"));
        assert!(page.contains("母港更新待ち"));
    }

    #[test]
    fn game_popups_only_accept_https_urls() {
        assert!(is_allowed_popup_url(
            &Url::parse("https://point.dmm.com/choice/pay").unwrap()
        ));
        assert!(!is_allowed_popup_url(
            &Url::parse("javascript:alert(1)").unwrap()
        ));
        assert!(!is_allowed_popup_url(
            &Url::parse("data:text/html,unsafe").unwrap()
        ));
    }

    #[test]
    fn point_charge_return_urls_close_the_popup() {
        for url in [
            "https://point.dmm.com/close?basket_service_type=freegame",
            "https://point.dmm.com/window/close",
            "https://point.dmm.com/close.html",
        ] {
            assert!(is_point_charge_return_url(&Url::parse(url).unwrap()));
        }

        assert!(!is_point_charge_return_url(
            &Url::parse(
                "https://point.dmm.com/choice/pay?back_url=https%3A%2F%2Fpoint.dmm.com%2Fwindow%2Fclose"
            )
            .unwrap()
        ));
        assert!(!is_point_charge_return_url(
            &Url::parse("https://example.com/close?basket_service_type=freegame").unwrap()
        ));
    }
}
