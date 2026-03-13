mod api;
mod battle_log;
mod ca;
mod commands;
mod cookie;
mod drive_sync;
mod expedition;
mod game_window;
mod improvement;
mod migration;
mod overlay;
mod proxy;
mod quest_progress;
mod senka;
mod sortie_quest;

use log::info;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{Emitter, Manager};
use url::Url;

use api::models::GameState;

/// Formation hint window offset from game window inner position (physical pixels)
#[derive(Debug, Default, Clone, Copy)]
pub struct FormationHintRect {
    pub dx: i32,
    pub dy: i32,
    pub w: u32,
    pub h: u32,
    pub visible: bool,
}

/// Application state shared across the app
pub struct AppState {
    pub proxy_port: Mutex<u16>,
    pub game_muted: AtomicBool,
    pub formation_hint_enabled: AtomicBool,
    pub taiha_alert_enabled: AtomicBool,
    pub minimap_enabled: AtomicBool,
    pub expedition_notify_visible: AtomicBool,
    /// Formation hint window offset relative to game window inner position
    pub formation_hint_rect: Mutex<FormationHintRect>,
    /// Current game zoom level (1.0 = 100%)
    pub game_zoom: Mutex<f64>,
    /// Minimap position (logical x, y) — None means use default bottom-right
    pub minimap_position: Mutex<Option<(f64, f64)>>,
    /// Minimap size (logical w, h)
    pub minimap_size: Mutex<(f64, f64)>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Install rustls CryptoProvider globally (needed by hyper-rustls for Drive API)
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            proxy_port: Mutex::new(0),
            game_muted: AtomicBool::new(false),
            formation_hint_enabled: AtomicBool::new(true),
            taiha_alert_enabled: AtomicBool::new(true),
            minimap_enabled: AtomicBool::new(true),
            expedition_notify_visible: AtomicBool::new(false),
            formation_hint_rect: Mutex::new(FormationHintRect::default()),
            game_zoom: Mutex::new(1.0),
            minimap_position: Mutex::new(None),
            minimap_size: Mutex::new((overlay::MINIMAP_DEFAULT_W, overlay::MINIMAP_DEFAULT_H)),
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_proxy_port,
            ca::is_ca_installed,
            ca::install_ca_cert,
            game_window::open_game_window,
            game_window::close_game_window,
            commands::get_expeditions,
            commands::check_expedition_cmd,
            commands::get_sortie_quests,
            commands::get_active_quest_ids,
            commands::check_sortie_quest_cmd,
            commands::get_map_recommendations,
            commands::check_map_recommendation_cmd,
            commands::get_battle_logs,
            commands::get_improvement_list,
            commands::get_ship_list,
            commands::get_equipment_list,
            cookie::save_game_cookies,
            commands::clear_improved_history,
            commands::clear_battle_logs,
            commands::clear_raw_api,
            commands::set_raw_api_enabled,
            commands::get_raw_api_enabled,
            cookie::clear_cookies,
            commands::reset_browser_data,
            commands::get_cached_resource,
            commands::get_map_sprite,
            commands::clear_resource_cache,
            commands::clear_browser_cache,
            game_window::set_game_zoom,
            game_window::toggle_game_mute,
            game_window::get_game_mute,
            overlay::set_overlay_visible,
            overlay::dismiss_overlay,
            overlay::toggle_minimap,
            overlay::get_minimap_enabled,
            overlay::move_minimap,
            overlay::resize_minimap,
            overlay::set_formation_hint_enabled,
            overlay::get_formation_hint_enabled,
            overlay::show_expedition_notification,
            overlay::hide_expedition_notification,
            overlay::set_taiha_alert_enabled,
            overlay::get_taiha_alert_enabled,
            commands::get_quest_progress,
            commands::update_quest_progress,
            commands::clear_quest_progress,
            commands::drive_login,
            commands::drive_logout,
            commands::get_drive_status,
            commands::drive_force_sync
        ])
        .setup(|app| {
            let data_dir = app
                .path()
                .app_local_data_dir()
                .unwrap_or_else(|_| PathBuf::from("."));

            // Migrate old flat layout to sync/ + local/ structure
            migration::migrate_data_dir(&data_dir);

            // Initialize GameState
            let sync_dir = data_dir.join("sync");
            info!("Sync dir: {}", sync_dir.display());
            app.manage(GameState::new(data_dir.clone()));

            // Restore mute state from disk (new local/ path)
            let mute_file = data_dir.join("local").join("game_muted");
            if let Ok(content) = std::fs::read_to_string(&mute_file) {
                if content.trim() == "1" {
                    let state = app.state::<AppState>();
                    state.game_muted.store(true, Ordering::Relaxed);
                    info!("Restored mute state: muted");
                }
            }

            // Restore formation hint enabled state from disk (default: enabled)
            let hint_file = data_dir.join("local").join("formation_hint_enabled");
            if let Ok(content) = std::fs::read_to_string(&hint_file) {
                if content.trim() == "0" {
                    let state = app.state::<AppState>();
                    state.formation_hint_enabled.store(false, Ordering::Relaxed);
                    info!("Restored formation hint state: disabled");
                }
            }

            // Restore taiha alert enabled state from disk (default: enabled)
            let taiha_file = data_dir.join("local").join("taiha_alert_enabled");
            if let Ok(content) = std::fs::read_to_string(&taiha_file) {
                if content.trim() == "0" {
                    let state = app.state::<AppState>();
                    state.taiha_alert_enabled.store(false, Ordering::Relaxed);
                    info!("Restored taiha alert state: disabled");
                }
            }

            // Restore minimap enabled state from disk (default: enabled)
            let minimap_file = data_dir.join("local").join("minimap_enabled");
            if let Ok(content) = std::fs::read_to_string(&minimap_file) {
                if content.trim() == "0" {
                    let state = app.state::<AppState>();
                    state.minimap_enabled.store(false, Ordering::Relaxed);
                    info!("Restored minimap state: disabled");
                }
            }

            // Restore minimap position from disk
            let minimap_pos_file = data_dir.join("local").join("minimap_position.json");
            if let Ok(content) = std::fs::read_to_string(&minimap_pos_file) {
                if let Ok(pos) = serde_json::from_str::<(f64, f64)>(&content) {
                    let state = app.state::<AppState>();
                    *state.minimap_position.lock().unwrap() = Some(pos);
                    info!("Restored minimap position: ({}, {})", pos.0, pos.1);
                }
            }

            // Restore minimap size from disk
            let minimap_size_file = data_dir.join("local").join("minimap_size.json");
            if let Ok(content) = std::fs::read_to_string(&minimap_size_file) {
                if let Ok(size) = serde_json::from_str::<(f64, f64)>(&content) {
                    let state = app.state::<AppState>();
                    *state.minimap_size.lock().unwrap() = size;
                    info!("Restored minimap size: ({}, {})", size.0, size.1);
                }
            }

            // Create cache directory for proxy resource caching
            let cache_dir = data_dir.join("local").join("cache");
            let _ = std::fs::create_dir_all(&cache_dir);

            let handle = app.handle().clone();

            tauri::async_runtime::spawn(async move {
                match proxy::start_proxy(handle.clone(), cache_dir).await {
                    Ok(port) => {
                        info!("Proxy server started on port {}", port);
                        let state = handle.state::<AppState>();
                        *state.proxy_port.lock().unwrap() = port;
                        let _ = handle.emit("proxy-ready", port);
                    }
                    Err(e) => {
                        log::error!("Failed to start proxy server: {}", e);
                    }
                }
            });

            // Try to auto-restore Google Drive sync from cached token
            let sync_handle = app.handle().clone();
            let sync_data_dir = data_dir.clone();
            tauri::async_runtime::spawn(async move {
                if let Some((client_id, client_secret)) = drive_sync::auth::client_credentials() {
                    // Try to restore from cached token (non-interactive)
                    if let Some(auth) =
                        drive_sync::auth::try_restore_auth(client_id, client_secret, &sync_data_dir)
                            .await
                    {
                        let sync_tx = drive_sync::engine::start_sync_engine(
                            sync_handle.clone(),
                            sync_data_dir,
                            auth,
                        )
                        .await;

                        let game_state_ref = sync_handle.state::<GameState>();
                        let mut inner = game_state_ref.inner.write().await;
                        inner.sync_notifier = Some(sync_tx);
                        info!("Auto-restored Google Drive sync");
                    } else {
                        info!("No cached Google Drive token, sync not started");
                    }
                }
            });

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::ExitRequested { .. } = &event {
                // Save DMM cookies before the app exits so login persists across restarts
                if let Some(game_wv) = app_handle.get_webview("game-content") {
                    let urls = [
                        "https://www.dmm.com",
                        "https://accounts.dmm.com",
                        "https://play.games.dmm.com",
                        "https://osapi.dmm.com",
                    ];
                    let mut all_cookies: Vec<serde_json::Value> = Vec::new();
                    let mut seen = std::collections::HashSet::new();
                    for url_str in &urls {
                        if let Ok(url) = url_str.parse::<Url>() {
                            if let Ok(cookies) = game_wv.cookies_for_url(url) {
                                for cookie in cookies {
                                    let key = format!(
                                        "{}={}",
                                        cookie.name(),
                                        cookie.domain().unwrap_or("")
                                    );
                                    if seen.insert(key) {
                                        all_cookies.push(serde_json::json!({
                                            "name": cookie.name(),
                                            "value": cookie.value(),
                                            "domain": cookie.domain(),
                                            "path": cookie.path(),
                                            "http_only": cookie.http_only().unwrap_or(false),
                                            "secure": cookie.secure().unwrap_or(false),
                                        }));
                                    }
                                }
                            }
                        }
                    }
                    if !all_cookies.is_empty() {
                        let path = cookie::cookie_file_path(app_handle);
                        if let Some(parent) = path.parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }
                        if let Ok(json) = serde_json::to_string_pretty(&all_cookies) {
                            let _ = std::fs::write(&path, json);
                            info!("Saved {} cookies on app exit", all_cookies.len());
                        }
                    }
                }
            }
        });
}
