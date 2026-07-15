mod action_log;
mod api;
mod battle_log;
mod ca;
mod commands;
mod cookie;
mod drive_sync;
mod diagnostics;
mod events;
mod expedition;
mod game_window;
mod improvement;
mod log_io;
mod migration;
mod mouse_hook;
mod overlay;
mod proxy;
mod quest_progress;
mod senka;
mod sensitive;
mod settings;
mod sortie_quest;
mod ui_event;
mod window_toggle;

use log::info;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard};
use tauri::{Emitter, Manager};
#[cfg(any(target_os = "macos", target_os = "windows"))]
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

use api::models::GameState;

/// Recover UI state from a poisoned mutex instead of turning one panic into a
/// permanent crash loop. The original panic is still present in diagnostics.
pub(crate) fn lock_or_recover<'a, T>(mutex: &'a Mutex<T>, name: &str) -> MutexGuard<'a, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            log::error!("Recovering poisoned mutex: {}", name);
            poisoned.into_inner()
        }
    }
}

/// Formation hint window offset from game window inner position (physical pixels)
#[derive(Debug, Default, Clone, Copy)]
pub struct FormationHintRect {
    pub dx: i32,
    pub dy: i32,
    pub w: u32,
    pub h: u32,
    pub visible: bool,
}

pub struct RuntimeState {
    pub proxy_port: Mutex<u16>,
}

pub struct OverlayPrefs {
    pub game_muted: AtomicBool,
    pub formation_hint_enabled: AtomicBool,
    pub taiha_alert_enabled: AtomicBool,
    pub minimap_enabled: AtomicBool,
    pub battle_info_enabled: AtomicBool,
}

pub struct OverlayState {
    /// Last battle info data for re-display on toggle re-enable
    pub last_battle_info: Mutex<Option<crate::api::battle_info::BattleInfoData>>,
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

pub struct NavigationState {
    /// Currently displayed game screen, inferred from click navigation events.
    /// Used by the mouse hook to dispatch coordinate-based UI event detection.
    pub current_screen: Mutex<ui_event::Screen>,
    /// Currently selected fleet (1-4) within fleet-compatible screens
    /// (編成 / 補給 / 改装). `None` when on a screen without fleet tabs.
    pub current_fleet: Mutex<Option<u32>>,
    /// QuestList left-side period filter (全 / 遂行中 / Daily / ...).
    pub current_quest_period: Mutex<Option<String>>,
    /// QuestList top-row category filter (出撃 / 演習 / 遠征 / 編成 / その他).
    pub current_quest_category: Mutex<Option<String>>,
}

/// Application state shared across the app, grouped by responsibility.
pub struct AppState {
    pub runtime: RuntimeState,
    pub prefs: OverlayPrefs,
    pub overlay: OverlayState,
    pub navigation: NavigationState,
}

/// Verify the CA certificate is installed; if not, prompt the user.
///
/// Returns `true` if the CA is (now) installed and the caller should proceed.
/// Returns `false` after `app.exit()` was already called (user cancelled or
/// install failed), in which case the caller should abort.
#[cfg(any(target_os = "macos", target_os = "windows"))]
async fn ensure_ca_installed(app: &tauri::AppHandle) -> bool {
    if ca::is_ca_installed() {
        return true;
    }

    info!("CA not installed — prompting user");
    let confirmed = ask_dialog(
        app,
        "CA証明書のインストール",
        "DMM 通信のためにCA証明書のインストールが必要です。\n\
         インストールしますか?\n\n\
         キャンセルするとアプリを終了します。",
        MessageDialogKind::Warning,
    )
    .await;

    if !confirmed {
        info!("User declined CA install — exiting");
        app.exit(0);
        return false;
    }

    // install_ca_cert is blocking (spawns elevated process); offload it.
    let install_app = app.clone();
    let install_result = tokio::task::spawn_blocking(move || ca::install_ca_cert(install_app))
        .await
        .unwrap_or_else(|e| Err(format!("install task panicked: {}", e)));

    match install_result {
        Ok(()) => {
            info!("CA installed at startup");
            true
        }
        Err(e) => {
            log::error!("CA install failed: {}", e);
            show_error_dialog(
                app,
                "CA証明書インストール失敗",
                &format!("インストールに失敗しました:\n{}\n\nアプリを終了します。", e),
            )
            .await;
            app.exit(1);
            false
        }
    }
}

/// Show a Yes/No dialog and await the user's choice (true = Yes).
#[cfg(any(target_os = "macos", target_os = "windows"))]
async fn ask_dialog(
    app: &tauri::AppHandle,
    title: &str,
    message: &str,
    kind: MessageDialogKind,
) -> bool {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .message(message)
        .title(title)
        .kind(kind)
        .buttons(MessageDialogButtons::YesNo)
        .show(move |answer| {
            let _ = tx.send(answer);
        });
    rx.await.unwrap_or(false)
}

/// Show a blocking error dialog (Ok button only) and wait for dismissal.
#[cfg(any(target_os = "macos", target_os = "windows"))]
async fn show_error_dialog(app: &tauri::AppHandle, title: &str, message: &str) {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .message(message)
        .title(title)
        .kind(MessageDialogKind::Error)
        .buttons(MessageDialogButtons::Ok)
        .show(move |_| {
            let _ = tx.send(());
        });
    let _ = rx.await;
}

/// On unsupported platforms, skip CA enforcement (proxy itself won't work).
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
async fn ensure_ca_installed(_app: &tauri::AppHandle) -> bool {
    true
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    diagnostics::init();

    // Static Tauri windows begin loading their React apps while `build()` is
    // running, before the setup hook. Resolve the same app-local-data path as
    // Tauri and register GameState first so startup invokes cannot race it.
    let context = tauri::generate_context!();
    let startup_data_dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(&context.config().identifier);

    if let Err(e) = diagnostics::attach(&startup_data_dir) {
        eprintln!("Failed to attach persistent session log: {e}");
    }
    migration::migrate_data_dir(&startup_data_dir);
    action_log::init(&startup_data_dir);
    action_log::log(
        "Session",
        "start",
        &format!("session_id={}", diagnostics::session_id()),
    );
    let game_state = GameState::new(startup_data_dir.clone());

    // Install rustls CryptoProvider globally (needed by hyper-rustls for Drive API)
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(game_state)
        .manage(AppState {
            runtime: RuntimeState {
                proxy_port: Mutex::new(0),
            },
            prefs: OverlayPrefs {
                game_muted: AtomicBool::new(false),
                formation_hint_enabled: AtomicBool::new(true),
                taiha_alert_enabled: AtomicBool::new(true),
                minimap_enabled: AtomicBool::new(true),
                battle_info_enabled: AtomicBool::new(true),
            },
            overlay: OverlayState {
                last_battle_info: Mutex::new(None),
                expedition_notify_visible: AtomicBool::new(false),
                formation_hint_rect: Mutex::new(FormationHintRect::default()),
                game_zoom: Mutex::new(1.0),
                minimap_position: Mutex::new(None),
                minimap_size: Mutex::new((
                    overlay::MINIMAP_DEFAULT_W,
                    overlay::MINIMAP_DEFAULT_H,
                )),
            },
            navigation: NavigationState {
                // Default to Unknown until the first port API arrives.
                current_screen: Mutex::new(ui_event::Screen::Unknown),
                current_fleet: Mutex::new(None),
                current_quest_period: Mutex::new(None),
                current_quest_category: Mutex::new(None),
            },
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_proxy_port,
            ca::is_ca_installed,
            ca::install_ca_cert,
            game_window::open_game_window,
            game_window::close_game_window,
            window_toggle::show_management_window,
            window_toggle::hide_management_window,
            window_toggle::toggle_management_window,
            window_toggle::show_kantai_window,
            window_toggle::hide_kantai_window,
            window_toggle::toggle_kantai_window,
            window_toggle::show_quests_window,
            window_toggle::hide_quests_window,
            window_toggle::toggle_quests_window,
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
            commands::cache::reset_browser_data,
            commands::cache::get_cached_resource,
            commands::cache::get_map_sprite,
            commands::cache::clear_resource_cache,
            commands::cache::clear_browser_cache,
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
            overlay::set_battle_info_enabled,
            overlay::get_battle_info_enabled,
            commands::get_quest_progress,
            commands::update_quest_progress,
            commands::clear_quest_progress,
            commands::drive_login,
            commands::drive_logout,
            commands::get_drive_status,
            commands::drive_force_sync,
            commands::get_action_log,
            commands::get_current_screen,
            commands::get_current_fleet,
            commands::get_quest_filters,
            commands::get_air_bases,
            commands::log_frontend_event,
            commands::log_frontend_events
        ])
        .setup(move |app| {
            let data_dir = app
                .path()
                .app_local_data_dir()
                .unwrap_or_else(|_| PathBuf::from("."));

            if data_dir != startup_data_dir {
                log::warn!(
                    "Pre-resolved app data dir differs from Tauri: startup={} tauri={}",
                    startup_data_dir.display(),
                    data_dir.display()
                );
            }

            let sync_dir = data_dir.join("sync");
            info!("Sync dir: {}", sync_dir.display());

            let state = app.state::<AppState>();
            state.prefs.game_muted.store(
                settings::restore_flag(&data_dir, settings::GAME_MUTED, false),
                Ordering::Relaxed,
            );
            state.prefs.formation_hint_enabled.store(
                settings::restore_flag(&data_dir, settings::FORMATION_HINT_ENABLED, true),
                Ordering::Relaxed,
            );
            state.prefs.taiha_alert_enabled.store(
                settings::restore_flag(&data_dir, settings::TAIHA_ALERT_ENABLED, true),
                Ordering::Relaxed,
            );
            state.prefs.minimap_enabled.store(
                settings::restore_flag(&data_dir, settings::MINIMAP_ENABLED, true),
                Ordering::Relaxed,
            );
            state.prefs.battle_info_enabled.store(
                settings::restore_flag(&data_dir, settings::BATTLE_INFO_ENABLED, true),
                Ordering::Relaxed,
            );
            if let Some(position) = settings::restore_json(&data_dir, settings::MINIMAP_POSITION) {
                *lock_or_recover(&state.overlay.minimap_position, "minimap_position") =
                    Some(position);
            }
            if let Some(size) = settings::restore_json(&data_dir, settings::MINIMAP_SIZE) {
                *lock_or_recover(&state.overlay.minimap_size, "minimap_size") = size;
            }

            // Create cache directory for proxy resource caching
            let cache_dir = data_dir.join("local").join("cache");
            let _ = std::fs::create_dir_all(&cache_dir);

            window_toggle::intercept_close_as_hide(app.handle());

            let handle = app.handle().clone();
            let proxy_data_dir = data_dir.clone();

            tauri::async_runtime::spawn(async move {
                match proxy::start_proxy(handle.clone(), cache_dir, proxy_data_dir).await {
                    Ok(port) => {
                        info!("Proxy server started on port {}", port);
                        crate::action_log::log("Event", "proxy-ready", &format!("port={}", port));
                        let state = handle.state::<AppState>();
                        *lock_or_recover(&state.runtime.proxy_port, "proxy_port") = port;
                        let _ = handle.emit(events::PROXY_READY, port);

                        // CA check before opening the game window. Without a trusted
                        // CA the proxy can't intercept HTTPS, so DMM ends up in a
                        // login loop on Windows + WebView2.
                        if !ensure_ca_installed(&handle).await {
                            return;
                        }

                        // Auto-open game window once proxy is ready and CA is OK.
                        if let Err(e) = game_window::open_game_window(handle.clone()).await {
                            log::error!("Failed to auto-open game window: {}", e);
                        }
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
        .build(context)
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::ExitRequested { .. } = &event {
                crate::action_log::log("Session", "exit-requested", "");
                // Uninstall mouse hook before exit
                mouse_hook::uninstall();

                // Save DMM cookies before the app exits so login persists across restarts
                if let Some(game_wv) = app_handle.get_webview("game-content") {
                    let all_cookies = cookie::collect_dmm_cookies(&game_wv);
                    if !all_cookies.is_empty() {
                        match cookie::write_cookie_file(app_handle, &all_cookies) {
                            Ok(n) => info!("Saved {} cookies on app exit", n),
                            Err(e) => log::warn!("Failed to save cookies on exit: {}", e),
                        }
                    }
                }
                diagnostics::shutdown();
            }
        });
}
