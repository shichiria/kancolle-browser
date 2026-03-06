mod api;
mod battle_log;
mod drive_sync;
mod expedition;
mod improvement;
mod proxy;
mod quest_progress;
mod senka;
mod sortie_quest;

use base64::Engine;
use log::{info, warn};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{Emitter, Manager, State, WebviewBuilder, WebviewUrl, WindowBuilder};
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
    /// Formation hint window offset relative to game window inner position
    pub formation_hint_rect: Mutex<FormationHintRect>,
    /// Current game zoom level (1.0 = 100%)
    pub game_zoom: Mutex<f64>,
}

/// Get the proxy port for the frontend
#[tauri::command]
fn get_proxy_port(state: State<AppState>) -> u16 {
    *state.proxy_port.lock().unwrap()
}

/// Check if the CA certificate is installed in the system trust store
#[tauri::command]
fn is_ca_installed() -> bool {
    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("security")
            .args(["find-certificate", "-c", "KanColle Browser CA"])
            .output();
        match output {
            Ok(o) => o.status.success(),
            Err(_) => false,
        }
    }
    #[cfg(target_os = "windows")]
    {
        let output = std::process::Command::new("certutil")
            .args(["-verifystore", "Root", "KanColle Browser CA"])
            .output();
        match output {
            Ok(o) => o.status.success(),
            Err(_) => false,
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        false
    }
}

/// Install the CA certificate into the system trust store.
#[tauri::command]
fn install_ca_cert() -> Result<(), String> {
    let pem_path = proxy::ca_pem_path();

    if !pem_path.exists() {
        return Err("CA certificate file not found. Proxy may not have started yet.".to_string());
    }

    let pem_str = pem_path.to_str().unwrap();
    info!("Installing CA certificate from: {}", pem_path.display());

    #[cfg(target_os = "macos")]
    {
        let keychain = format!(
            "{}/Library/Keychains/login.keychain-db",
            std::env::var("HOME").unwrap_or_default()
        );

        // Step 1: Import certificate to login keychain
        let import_output = std::process::Command::new("security")
            .args(["import", pem_str, "-k", &keychain, "-t", "cert"])
            .output()
            .map_err(|e| format!("Failed to run security import: {}", e))?;

        if !import_output.status.success() {
            let stderr = String::from_utf8_lossy(&import_output.stderr);
            if !stderr.contains("already exists") {
                return Err(format!("Failed to import certificate: {}", stderr));
            }
            info!("CA certificate already in keychain, updating trust...");
        } else {
            info!("CA certificate imported to keychain");
        }

        // Step 2: Set trust as root CA (triggers macOS password dialog)
        let trust_output = std::process::Command::new("security")
            .args([
                "add-trusted-cert",
                "-d",
                "-r",
                "trustRoot",
                "-k",
                &keychain,
                pem_str,
            ])
            .output()
            .map_err(|e| format!("Failed to run security add-trusted-cert: {}", e))?;

        if trust_output.status.success() {
            info!("CA certificate trusted for SSL successfully");
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&trust_output.stderr);
            Err(format!("Failed to set certificate trust: {}", stderr))
        }
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        // Use PowerShell Start-Process -Verb RunAs to trigger UAC elevation dialog
        // This allows certificate installation without running the app as administrator
        let escaped_path = pem_str.replace('\'', "''");
        let script = format!(
            "try {{ $p = Start-Process -FilePath 'certutil.exe' -ArgumentList '-addstore','Root','\"{}\"' -Verb RunAs -Wait -PassThru; exit $p.ExitCode }} catch {{ Write-Error $_.Exception.Message; exit 1 }}",
            escaped_path
        );

        let output = std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-Command", &script])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| format!("Failed to run certutil: {}", e))?;

        if output.status.success() {
            info!("CA certificate installed to Windows trust store");
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("canceled") || stderr.contains("cancelled") {
                Err("Certificate installation was cancelled by user.".to_string())
            } else {
                Err(format!("Failed to install certificate: {}", stderr))
            }
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err("Certificate installation not supported on this platform".to_string())
    }
}

/// Generate a JavaScript snippet that restores the saved DMM session cookies
/// directly via `document.cookie`. This bypasses strict native API validations
/// (e.g. WebView2 dropping SameSite=None cookies on domains with a dot prefix).
async fn build_cookie_restore_script(app: &tauri::AppHandle) -> String {
    let path = cookie_file_path(app);
    let raw_cookies = match tokio::fs::read_to_string(&path).await {
        Ok(content) => match serde_json::from_str::<Vec<serde_json::Value>>(&content) {
            Ok(v) => v,
            Err(_) => return String::new(),
        },
        Err(_) => return String::new(),
    };

    let mut script = String::from("(function() {\n");
    let expires = (chrono::Utc::now() + chrono::Duration::days(365))
        .format("%a, %d %b %Y %H:%M:%S GMT")
        .to_string();

    let mut count = 0;
    for c in &raw_cookies {
        let name = match c.get("name").and_then(|v| v.as_str()) {
            Some(n) => n,
            None => continue,
        };
        let value = match c.get("value").and_then(|v| v.as_str()) {
            Some(v) => v,
            None => continue,
        };
        let domain = c.get("domain").and_then(|v| v.as_str()).unwrap_or("");

        // Ensure domain cookies apply to subdomains by prepending a dot
        let mut domain_str = domain.to_string();
        if !domain_str.starts_with('.') && domain_str.contains('.') {
            domain_str = format!(".{}", domain);
        }

        let cookie_path = c.get("path").and_then(|v| v.as_str()).unwrap_or("/");
        let http_only = c
            .get("http_only")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // httpOnly cookies cannot be set via document.cookie. However, DMM's critical session
        // identifier (login_secure_id / secid) is sometimes marked httpOnly. Since we are injecting
        // into about:blank before navigation, it's safer to just set them normally so the browser
        // attaches them. The browser will protect them on subsequent HTTP requests.
        let _ = http_only; // Ignore http_only flag for JS injection

        // Build the cookie string
        let cookie_str = format!(
            "{}={}; domain={}; path={}; expires={}; secure; samesite=none",
            name, value, domain_str, cookie_path, expires
        );

        script.push_str(&format!("  document.cookie = {:?};\n", cookie_str));
        count += 1;
    }
    script.push_str("})();\n");

    info!("Generated JS script to restore {} cookies.", count);
    script
}

/// Cookie persistence file path
fn cookie_file_path(app: &tauri::AppHandle) -> PathBuf {
    app.path()
        .app_local_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("local")
        .join("dmm_cookies.json")
}

/// Save cookies from the game window to a file
/// DMM uses Session cookies which are deleted when the Webview process dies.
/// We must manually extract and save them to JSON, then restore with Expiration+365 days.
#[tauri::command]
async fn save_game_cookies(app: tauri::AppHandle) -> Result<usize, String> {
    let game_wv = app
        .get_webview("game-content")
        .ok_or("Game webview not found")?;

    let urls = [
        "https://www.dmm.com",
        "https://accounts.dmm.com",
        "https://play.games.dmm.com",
        "https://osapi.dmm.com",
    ];

    let mut all_cookies: Vec<serde_json::Value> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for url_str in &urls {
        let url: Url = url_str.parse().unwrap();
        match game_wv.cookies_for_url(url) {
            Ok(cookies) => {
                for cookie in cookies {
                    let key = format!("{}={}", cookie.name(), cookie.domain().unwrap_or(""));
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
            Err(e) => {
                log::warn!("Failed to get cookies for {}: {}", url_str, e);
            }
        }
    }

    let count = all_cookies.len();
    if count == 0 {
        return Ok(0);
    }

    let path = cookie_file_path(&app);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(&all_cookies).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    info!("Saved {} DMM cookies to {}", count, path.display());
    Ok(count)
}

/// JavaScript injection: hide DMM UI, show game frame, add control bar overlay
const GAME_INIT_SCRIPT: &str = r#"
(function() {
    // --- CSS applied to ALL frames (including cross-origin game iframes) ---
    // This removes scrollbars everywhere in the WebView2 window.
    var COMMON_CSS = `
        html, body {
            margin: 0 !important;
            padding: 0 !important;
            overflow: hidden !important;
        }
        * {
            scrollbar-width: none !important;
            -ms-overflow-style: none !important;
        }
        *::-webkit-scrollbar { display: none !important; }
    `;

    // --- CSS applied only to the top-level DMM frame ---
    var TOP_CSS = `
        html, body {
            background-color: black !important;
            width: 100% !important;
            height: 100% !important;
        }
        .dmm-ntgnavi, .area-naviapp, #ntg-recommend,
        #foot, #foot+img,
        .gamesResetStyle > header,
        .gamesResetStyle > footer,
        .gamesResetStyle > aside,
        #page header, #page footer, .nav_area,
        .area-biling, .peri-header, .peri-footer {
            display: none !important;
        }
        #w, #main-ntg, #page {
            margin: 0 !important;
            padding: 0 !important;
            width: 100% !important;
            background: none !important;
            overflow: hidden !important;
        }
        #main-ntg {
            margin: 0 !important;
            position: static !important;
        }
        #area-game {
            margin: 0 !important;
            padding: 0 !important;
            width: 1200px !important;
            height: 720px !important;
            position: relative !important;
            overflow: hidden !important;
        }
        #game_frame {
            position: fixed !important;
            top: 28px !important;
            left: 0 !important;
            z-index: 10000 !important;
            width: 1200px !important;
            height: 720px !important;
            border: none !important;
            overflow: hidden !important;
        }
        /* Control bar */
        #kc-control-bar {
            position: fixed;
            top: 0;
            left: 0;
            right: 0;
            height: 28px;
            z-index: 99999;
            background: #16213e;
            display: flex;
            align-items: center;
            padding: 0 8px;
            gap: 8px;
            font-family: -apple-system, BlinkMacSystemFont, sans-serif;
            font-size: 11px;
            color: #e0e0e0;
            border-bottom: 1px solid #0f3460;
            user-select: none;
            -webkit-user-select: none;
        }
        #kc-control-bar select {
            font-size: 11px;
            padding: 1px 4px;
            background: #0f3460;
            color: #e0e0e0;
            border: 1px solid #1a4080;
            border-radius: 3px;
            outline: none;
            cursor: pointer;
        }
        #kc-control-bar select:hover { background: #1a4080; }
        #kc-control-bar button {
            font-size: 12px;
            padding: 1px 8px;
            background: #0f3460;
            color: #e0e0e0;
            border: 1px solid #1a4080;
            border-radius: 3px;
            cursor: pointer;
            line-height: 1.4;
        }
        #kc-control-bar button:hover { background: #1a4080; }
        #kc-control-bar button.muted {
            background: rgba(233,69,96,0.2);
            border-color: rgba(233,69,96,0.4);
        }
        #kc-control-bar .spacer { flex: 1; }
        #kc-control-bar .label { font-size: 10px; color: #666; }
    `;

    var isTop = false;
    try { isTop = (window.self === window.top); } catch(e) {}

    var cssText = isTop ? (COMMON_CSS + TOP_CSS) : COMMON_CSS;

    // Inject style — use MutationObserver on document for WebView2 compatibility
    function injectStyle() {
        if (document.getElementById('kc-browser-style')) return true;
        var target = document.head || document.documentElement;
        if (!target) return false;
        var style = document.createElement('style');
        style.id = 'kc-browser-style';
        style.textContent = cssText;
        target.appendChild(style);
        return true;
    }

    if (!injectStyle()) {
        var obs = new MutationObserver(function(mutations, observer) {
            if (injectStyle()) observer.disconnect();
        });
        obs.observe(document, { childList: true, subtree: true });
    }
    document.addEventListener('DOMContentLoaded', function() { injectStyle(); });

    // Control bar — top frame only
    if (!isTop) return;

    function addControlBar() {
        if (document.getElementById('kc-control-bar')) return;
        var parent = document.body || document.documentElement;
        if (!parent) return;
        var bar = document.createElement('div');
        bar.id = 'kc-control-bar';
        bar.innerHTML = '<select id="kc-zoom">'
            + '<option value="0.5">50%</option>'
            + '<option value="0.67">67%</option>'
            + '<option value="0.75">75%</option>'
            + '<option value="1">100%</option>'
            + '<option value="1.25">125%</option>'
            + '<option value="1.5">150%</option>'
            + '<option value="2">200%</option>'
            + '</select>'
            + '<button id="kc-mute">\u{1f50a}</button>'
            + '<button id="kc-formation" title="\u{9663}\u{5F62}\u{8A18}\u{61B6}">\u{9663}\u{5F62}</button>'
            + '<button id="kc-taiha" title="\u{5927}\u{7834}\u{8B66}\u{544A}">\u{26A0}\u{5927}\u{7834}</button>'
            + '<span class="spacer"></span>'
            + '<span class="label">KanColle Browser</span>';
        parent.appendChild(bar);

        // Restore saved zoom
        var saved = localStorage.getItem('kc-game-zoom');
        if (saved) {
            document.getElementById('kc-zoom').value = saved;
            var z = parseFloat(saved);
            if (z && z !== 1 && window.__TAURI_INTERNALS__) {
                window.__TAURI_INTERNALS__.invoke('set_game_zoom', { zoom: z });
            }
        }

        document.getElementById('kc-zoom').addEventListener('change', function() {
            var z = parseFloat(this.value);
            localStorage.setItem('kc-game-zoom', String(z));
            window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke('set_game_zoom', { zoom: z });
        });

        // Restore mute state from backend
        var muted = false;
        var muteBtn = document.getElementById('kc-mute');
        if (window.__TAURI_INTERNALS__) {
            window.__TAURI_INTERNALS__.invoke('get_game_mute').then(function(m) {
                muted = !!m;
                muteBtn.textContent = muted ? '\u{1f507}' : '\u{1f50a}';
                muteBtn.className = muted ? 'muted' : '';
                if (muted) {
                    window.__TAURI_INTERNALS__.invoke('toggle_game_mute', { muted: true });
                }
            }).catch(function() {});
        }
        muteBtn.addEventListener('click', function() {
            muted = !muted;
            this.textContent = muted ? '\u{1f507}' : '\u{1f50a}';
            this.className = muted ? 'muted' : '';
            window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke('toggle_game_mute', { muted: muted });
        });

        // Formation hint toggle
        var fmtEnabled = true;
        var fmtBtn = document.getElementById('kc-formation');
        if (window.__TAURI_INTERNALS__) {
            window.__TAURI_INTERNALS__.invoke('get_formation_hint_enabled').then(function(e) {
                fmtEnabled = !!e;
                fmtBtn.className = fmtEnabled ? '' : 'muted';
                fmtBtn.title = fmtEnabled ? '\u{9663}\u{5F62}\u{8A18}\u{61B6} ON' : '\u{9663}\u{5F62}\u{8A18}\u{61B6} OFF';
            }).catch(function() {});
        }
        fmtBtn.addEventListener('click', function() {
            fmtEnabled = !fmtEnabled;
            this.className = fmtEnabled ? '' : 'muted';
            this.title = fmtEnabled ? '\u{9663}\u{5F62}\u{8A18}\u{61B6} ON' : '\u{9663}\u{5F62}\u{8A18}\u{61B6} OFF';
            window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke('set_formation_hint_enabled', { enabled: fmtEnabled });
        });

        // Taiha alert toggle
        var taihaEnabled = true;
        var taihaBtn = document.getElementById('kc-taiha');
        if (window.__TAURI_INTERNALS__) {
            window.__TAURI_INTERNALS__.invoke('get_taiha_alert_enabled').then(function(e) {
                taihaEnabled = !!e;
                taihaBtn.className = taihaEnabled ? '' : 'muted';
                taihaBtn.title = taihaEnabled ? '\u{5927}\u{7834}\u{8B66}\u{544A} ON' : '\u{5927}\u{7834}\u{8B66}\u{544A} OFF';
            }).catch(function() {});
        }
        taihaBtn.addEventListener('click', function() {
            taihaEnabled = !taihaEnabled;
            this.className = taihaEnabled ? '' : 'muted';
            this.title = taihaEnabled ? '\u{5927}\u{7834}\u{8B66}\u{544A} ON' : '\u{5927}\u{7834}\u{8B66}\u{544A} OFF';
            window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke('set_taiha_alert_enabled', { enabled: taihaEnabled });
        });
    }

    if (document.body) addControlBar();
    else document.addEventListener('DOMContentLoaded', addControlBar);
})();
"#;

/// Open the KanColle game in a separate window with proxy configured.
/// Uses multi-webview: game-content (game) + game-overlay (transparent overlay).
#[tauri::command]
async fn open_game_window(app: tauri::AppHandle) -> Result<(), String> {
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
    let restore_script = build_cookie_restore_script(&app).await;
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
                        match save_game_cookies(handle).await {
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

    // Sync game webview on resize, reposition formation hint on move/resize
    let resize_app = app.clone();
    game_window.on_window_event(move |event| {
        match event {
            tauri::WindowEvent::Resized(size) => {
                if let Some(wv) = resize_app.get_webview("game-content") {
                    let _ = wv.set_size(*size);
                }
                // Reposition formation hint
                reposition_formation_hint(&resize_app);
            }
            tauri::WindowEvent::Moved(_) => {
                reposition_formation_hint(&resize_app);
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

/// Get all expedition definitions for the frontend
#[tauri::command]
fn get_expeditions() -> Vec<expedition::ExpeditionDef> {
    expedition::get_all_expeditions()
}

/// Get all sortie quest definitions for the frontend
#[tauri::command]
fn get_sortie_quests() -> Vec<sortie_quest::SortieQuestDef> {
    sortie_quest::get_all_sortie_quests()
}

/// Get currently active (accepted/completed) quest details
#[tauri::command]
async fn get_active_quest_ids(
    state: tauri::State<'_, api::models::GameState>,
) -> Result<Vec<api::models::ActiveQuestDetail>, String> {
    let inner = state.inner.read().await;
    Ok(inner.history.active_quest_details.values().cloned().collect())
}

/// Check if a fleet meets the conditions for a specific sortie quest
#[tauri::command]
async fn check_sortie_quest_cmd(
    fleet_index: usize,
    quest_id: String,
    state: tauri::State<'_, api::models::GameState>,
) -> Result<sortie_quest::SortieQuestCheckResult, String> {
    let inner = state.inner.read().await;

    if fleet_index >= inner.profile.fleets.len() {
        return Err(format!(
            "Invalid fleet index: {} (have {} fleets)",
            fleet_index,
            inner.profile.fleets.len()
        ));
    }

    let fleet_ship_ids = &inner.profile.fleets[fleet_index];
    if fleet_ship_ids.is_empty() {
        return Err("Fleet is empty".to_string());
    }

    let mut ships = Vec::new();
    for &ship_id in fleet_ship_ids {
        if let Some(info) = inner.profile.ships.get(&ship_id) {
            ships.push(sortie_quest::FleetShipData {
                name: info.name.clone(),
                ship_type: info.stype,
                level: info.lv,
            });
        }
    }

    let fleet_data = sortie_quest::FleetCheckData { ships };
    Ok(sortie_quest::check_sortie_quest(&quest_id, &fleet_data))
}

/// Get all map recommendation definitions for the frontend
#[tauri::command]
fn get_map_recommendations() -> Vec<sortie_quest::MapRecommendationDef> {
    sortie_quest::get_all_map_recommendations()
}

/// Check if a fleet meets the route conditions for a specific map
#[tauri::command]
async fn check_map_recommendation_cmd(
    fleet_index: usize,
    area: String,
    state: tauri::State<'_, api::models::GameState>,
) -> Result<sortie_quest::MapRecommendationCheckResult, String> {
    let inner = state.inner.read().await;

    if fleet_index >= inner.profile.fleets.len() {
        return Err(format!(
            "Invalid fleet index: {} (have {} fleets)",
            fleet_index,
            inner.profile.fleets.len()
        ));
    }

    let fleet_ship_ids = &inner.profile.fleets[fleet_index];
    if fleet_ship_ids.is_empty() {
        return Err("Fleet is empty".to_string());
    }

    let mut ships = Vec::new();
    for &ship_id in fleet_ship_ids {
        if let Some(info) = inner.profile.ships.get(&ship_id) {
            ships.push(sortie_quest::FleetShipData {
                name: info.name.clone(),
                ship_type: info.stype,
                level: info.lv,
            });
        }
    }

    let fleet_data = sortie_quest::FleetCheckData { ships };
    Ok(sortie_quest::check_map_recommendation(&area, &fleet_data))
}

/// Check if a fleet meets the conditions for a specific expedition
#[tauri::command]
async fn check_expedition_cmd(
    fleet_index: usize,
    expedition_id: i32,
    state: tauri::State<'_, api::models::GameState>,
) -> Result<expedition::ExpeditionCheckResult, String> {
    let inner = state.inner.read().await;

    // Validate fleet index
    if fleet_index >= inner.profile.fleets.len() {
        return Err(format!(
            "Invalid fleet index: {} (have {} fleets)",
            fleet_index,
            inner.profile.fleets.len()
        ));
    }

    let fleet_ship_ids = &inner.profile.fleets[fleet_index];
    if fleet_ship_ids.is_empty() {
        return Err("Fleet is empty".to_string());
    }

    // Drum canister: master slotitem category (api_type[2]) == 24
    const DRUM_CATEGORY: i32 = 24;

    // Build FleetCheckData from GameState
    let mut ships = Vec::new();
    for &ship_id in fleet_ship_ids {
        if let Some(info) = inner.profile.ships.get(&ship_id) {
            // Count drums on this ship
            let mut drum_count = 0i32;
            for &slot_id in &info.slot {
                if slot_id <= 0 {
                    continue;
                }
                if let Some(player_item) = inner.profile.slotitems.get(&slot_id) {
                    if let Some(master_item) = inner.master.slotitems.get(&player_item.slotitem_id)
                    {
                        if master_item.item_type == DRUM_CATEGORY {
                            drum_count += 1;
                        }
                    }
                }
            }

            ships.push(expedition::FleetShipData {
                ship_type: info.stype,
                ship_id: info.ship_id,
                level: info.lv,
                firepower: info.firepower,
                aa: info.aa,
                asw: info.asw,
                los: info.los,
                cond: info.cond,
                has_drum: drum_count > 0,
                drum_count,
            });
        }
    }

    let fleet_data = expedition::FleetCheckData { ships };
    Ok(expedition::check_expedition(expedition_id, &fleet_data))
}

/// Get improvement list for the improvement tab
#[tauri::command]
async fn get_improvement_list(
    state: tauri::State<'_, api::models::GameState>,
) -> Result<improvement::ImprovementListResponse, String> {
    let inner = state.inner.read().await;
    Ok(improvement::build_improvement_list(&inner))
}

/// Get all player ships for the ship list tab
#[tauri::command]
async fn get_ship_list(
    state: tauri::State<'_, api::models::GameState>,
) -> Result<api::models::ShipListResponse, String> {
    let inner = state.inner.read().await;
    let mut ships: Vec<api::models::ShipListItem> = inner
        .profile
        .ships
        .iter()
        .map(|(&id, info)| {
            let stype_name = inner
                .master
                .stypes
                .get(&info.stype)
                .cloned()
                .unwrap_or_default();
            api::models::ShipListItem {
                id,
                ship_id: info.ship_id,
                name: info.name.clone(),
                stype: info.stype,
                stype_name,
                lv: info.lv,
                hp: info.hp,
                maxhp: info.maxhp,
                cond: info.cond,
                firepower: info.firepower,
                torpedo: info.torpedo,
                aa: info.aa,
                armor: info.armor,
                asw: info.asw,
                evasion: info.evasion,
                los: info.los,
                luck: info.luck,
                locked: info.locked,
            }
        })
        .collect();
    ships.sort_by(|a, b| b.lv.cmp(&a.lv).then(a.ship_id.cmp(&b.ship_id)));

    let mut stypes: Vec<(i32, String)> = inner
        .master
        .stypes
        .iter()
        .map(|(&id, name)| (id, name.clone()))
        .collect();
    stypes.sort_by_key(|(id, _)| *id);

    Ok(api::models::ShipListResponse { ships, stypes })
}

/// Get all player equipment grouped by master ID for the equipment list tab
#[tauri::command]
async fn get_equipment_list(
    state: tauri::State<'_, api::models::GameState>,
) -> Result<api::models::EquipListResponse, String> {
    use std::collections::BTreeMap;

    let inner = state.inner.read().await;

    // Group player items by master slotitem_id
    let mut groups: std::collections::HashMap<i32, Vec<&api::models::PlayerSlotItem>> =
        std::collections::HashMap::new();
    for item in inner.profile.slotitems.values() {
        groups.entry(item.slotitem_id).or_default().push(item);
    }

    let mut items: Vec<api::models::EquipListItem> = groups
        .into_iter()
        .filter_map(|(master_id, player_items)| {
            let master = inner.master.slotitems.get(&master_id)?;
            let type_name = inner
                .master
                .equip_types
                .get(&master.item_type)
                .cloned()
                .unwrap_or_default();

            let total_count = player_items.len() as i32;
            let locked_count = player_items.iter().filter(|i| i.locked).count() as i32;

            // Count by improvement level
            let mut level_counts: BTreeMap<i32, i32> = BTreeMap::new();
            for item in &player_items {
                *level_counts.entry(item.level).or_insert(0) += 1;
            }
            let improvements: Vec<(i32, i32)> = level_counts.into_iter().collect();

            Some(api::models::EquipListItem {
                master_id,
                name: master.name.clone(),
                type_id: master.item_type,
                type_name,
                icon_type: master.icon_type,
                total_count,
                locked_count,
                improvements,
            })
        })
        .collect();

    items.sort_by(|a, b| a.type_id.cmp(&b.type_id).then(a.name.cmp(&b.name)));

    // Build equip type filter list (only types that exist in player's equipment)
    let mut used_types: std::collections::HashSet<i32> = std::collections::HashSet::new();
    for item in &items {
        used_types.insert(item.type_id);
    }
    let mut equip_types: Vec<(i32, String)> = inner
        .master
        .equip_types
        .iter()
        .filter(|(id, _)| used_types.contains(id))
        .map(|(&id, name)| (id, name.clone()))
        .collect();
    equip_types.sort_by_key(|(id, _)| *id);

    Ok(api::models::EquipListResponse { items, equip_types })
}

/// Clear improved equipment history
#[tauri::command]
async fn clear_improved_history(
    state: tauri::State<'_, api::models::GameState>,
) -> Result<(), String> {
    let mut inner = state.inner.write().await;
    inner.history.improved_equipment.clear();
    improvement::save_improved_history(&inner.improved_equipment_path, &inner.history.improved_equipment);
    info!("Cleared improved equipment history");
    Ok(())
}

/// Clear battle log records
#[tauri::command]
async fn clear_battle_logs(state: tauri::State<'_, api::models::GameState>) -> Result<(), String> {
    let mut inner = state.inner.write().await;
    inner.sortie.battle_logger.clear_records();
    info!("Cleared battle logs");
    Ok(())
}

/// Clear raw API dumps
#[tauri::command]
async fn clear_raw_api(state: tauri::State<'_, api::models::GameState>) -> Result<(), String> {
    let inner = state.inner.read().await;
    inner.sortie.battle_logger.clear_raw_api();
    info!("Cleared raw API dumps");
    Ok(())
}

/// Toggle raw API log saving (developer option)
#[tauri::command]
async fn set_raw_api_enabled(
    state: tauri::State<'_, api::models::GameState>,
    enabled: bool,
) -> Result<(), String> {
    let mut inner = state.inner.write().await;
    inner.sortie.battle_logger.set_raw_enabled(enabled);
    info!("Raw API saving: {}", if enabled { "ON" } else { "OFF" });
    Ok(())
}

/// Get raw API log saving state
#[tauri::command]
async fn get_raw_api_enabled(
    state: tauri::State<'_, api::models::GameState>,
) -> Result<bool, String> {
    let inner = state.inner.read().await;
    Ok(inner.sortie.battle_logger.is_raw_enabled())
}

/// Clear saved cookies
#[tauri::command]
fn clear_cookies(app: tauri::AppHandle) -> Result<(), String> {
    let path = cookie_file_path(&app);
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    info!("Cleared saved cookies");
    Ok(())
}

/// Get a cached game resource (image or JSON) from the local cache.
/// For images, returns a data URI (data:image/png;base64,...).
/// For JSON/text files, returns the raw content string.
/// Returns empty string if the file is not cached.
#[tauri::command]
async fn get_cached_resource(app: tauri::AppHandle, path: String) -> Result<String, String> {
    let cache_dir = app
        .path()
        .app_local_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("local")
        .join("cache");

    let file_path = cache_dir.join(&path);

    // Security: ensure the resolved path stays within cache_dir
    let canonical_cache = cache_dir
        .canonicalize()
        .unwrap_or_else(|_| cache_dir.clone());
    if let Ok(canonical_file) = file_path.canonicalize() {
        if !canonical_file.starts_with(&canonical_cache) {
            return Err("Invalid path".to_string());
        }
    }

    if !file_path.exists() {
        return Ok(String::new());
    }

    let ext = file_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "json" | "js" | "css" | "txt" | "html" => tokio::fs::read_to_string(&file_path)
            .await
            .map_err(|e| format!("Failed to read {}: {}", path, e)),
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" => {
            let mime = match ext.as_str() {
                "png" => "image/png",
                "jpg" | "jpeg" => "image/jpeg",
                "gif" => "image/gif",
                "webp" => "image/webp",
                "svg" => "image/svg+xml",
                _ => "application/octet-stream",
            };
            let data = tokio::fs::read(&file_path)
                .await
                .map_err(|e| format!("Failed to read {}: {}", path, e))?;
            let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
            Ok(format!("data:{};base64,{}", mime, b64))
        }
        _ => {
            // Binary fallback: return base64 with generic MIME
            let data = tokio::fs::read(&file_path)
                .await
                .map_err(|e| format!("Failed to read {}: {}", path, e))?;
            let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
            Ok(format!("data:application/octet-stream;base64,{}", b64))
        }
    }
}

/// Clear the proxy resource cache directory (game images, JSON, etc.).
#[tauri::command]
async fn clear_resource_cache(app: tauri::AppHandle) -> Result<String, String> {
    let cache_dir = app
        .path()
        .app_local_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("local")
        .join("cache");

    if !cache_dir.exists() {
        return Ok("保存リソースはありません".to_string());
    }

    fn count_files(dir: &std::path::Path) -> u64 {
        let mut count = 0u64;
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    count += count_files(&path);
                } else {
                    count += 1;
                }
            }
        }
        count
    }
    let count = count_files(&cache_dir);

    std::fs::remove_dir_all(&cache_dir).map_err(|e| format!("削除失敗: {}", e))?;
    std::fs::create_dir_all(&cache_dir).map_err(|e| format!("ディレクトリ再作成失敗: {}", e))?;

    info!("Resource cache cleared: {} files deleted", count);
    Ok(format!("保存リソースを削除しました（{}ファイル）", count))
}

/// Clear the browser cache (HTTP cache, code cache, GPU cache, etc.).
/// If the game webview is open, uses the WebView API (clear_all_browsing_data).
/// If the game webview is closed, falls back to file-system deletion.
#[tauri::command]
async fn clear_browser_cache(app: tauri::AppHandle) -> Result<String, String> {
    // If game webview is open, use the WebView API to clear browsing data
    if let Some(game_wv) = app.get_webview("game-content") {
        game_wv
            .clear_all_browsing_data()
            .map_err(|e| e.to_string())?;
        info!("Browser cache cleared via WebView API");
        return Ok("ブラウザキャッシュを削除しました".to_string());
    }

    // Game webview is closed — fall back to file-system deletion
    let mut deleted = 0u64;

    #[cfg(not(target_os = "macos"))]
    {
        let webview_dir = app
            .path()
            .app_local_data_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("local")
            .join("game-webview")
            .join("EBWebView");

        if webview_dir.exists() {
            let cache_dirs = [
                "Default/Cache",
                "Default/Code Cache",
                "Default/GPUCache",
                "Default/DawnGraphiteCache",
                "Default/DawnWebGPUCache",
                "ShaderCache",
                "GrShaderCache",
                "GraphiteDawnCache",
            ];

            for dir_name in &cache_dirs {
                let dir_path = webview_dir.join(dir_name);
                if dir_path.exists() {
                    if let Ok(_) = std::fs::remove_dir_all(&dir_path) {
                        deleted += 1;
                        info!("Deleted browser cache: {}", dir_name);
                    }
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        // WKWebView stores NetworkCache under ~/Library/Caches/<app-name>/WebKit/
        if let Some(home) = dirs::home_dir() {
            let caches_dir = home.join("Library/Caches");
            let app_names = ["kancolle-browser", "com.eo.kancolle-browser"];

            for app_name in &app_names {
                let webkit_dir = caches_dir.join(app_name).join("WebKit");
                if webkit_dir.exists() {
                    match std::fs::remove_dir_all(&webkit_dir) {
                        Ok(_) => {
                            deleted += 1;
                            info!("Deleted WKWebView cache: {}/WebKit", app_name);
                        }
                        Err(e) => {
                            log::warn!("Failed to delete WebKit cache for {}: {}", app_name, e);
                        }
                    }
                }
            }
        }
    }

    if deleted == 0 {
        return Ok("ブラウザキャッシュはありません".to_string());
    }

    info!(
        "Browser cache cleared: {} directories/caches deleted",
        deleted
    );
    Ok(format!(
        "ブラウザキャッシュを削除しました（{}箇所）",
        deleted
    ))
}

/// Extract a sprite from a map sprite sheet and return as base64 data URI.
/// `map_display` is e.g. "1-1", `frame_name` is e.g. "map1-1" (from _info.json bg[0]).
#[tauri::command]
async fn get_map_sprite(
    app: tauri::AppHandle,
    map_display: String,
    frame_name: String,
    #[allow(unused)] tint_cyan: Option<bool>,
    route_idx: Option<i32>,
    spot_no: Option<i32>,
) -> Result<String, String> {
    info!(
        "get_map_sprite: map={}, frame={}, route_idx={:?}, spot_no={:?}, tint_cyan={:?}",
        map_display, frame_name, route_idx, spot_no, tint_cyan
    );
    let cache_dir = app
        .path()
        .app_local_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("local")
        .join("cache");

    let parts: Vec<&str> = map_display.split('-').collect();
    let area = format!(
        "{:03}",
        parts
            .first()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0)
    );
    let map = format!(
        "{:02}",
        parts
            .get(1)
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0)
    );

    let atlas_path = cache_dir.join(format!("kcs2/resources/map/{}/{}_image.json", area, map));
    let image_path = cache_dir.join(format!("kcs2/resources/map/{}/{}_image.png", area, map));

    if !atlas_path.exists() || !image_path.exists() {
        return Ok(String::new());
    }

    // Read and parse the atlas JSON
    let atlas_bytes = tokio::fs::read(&atlas_path)
        .await
        .map_err(|e| format!("Failed to read atlas: {}", e))?;

    // The atlas might be brotli-compressed (if cached before the brotli fix)
    let atlas_str = if atlas_bytes.starts_with(b"{") {
        String::from_utf8(atlas_bytes).map_err(|e| format!("Invalid atlas UTF-8: {}", e))?
    } else {
        // Try brotli decompression for old cached files
        let mut decoder = brotli::Decompressor::new(atlas_bytes.as_slice(), 4096);
        let mut decompressed = Vec::new();
        std::io::Read::read_to_end(&mut decoder, &mut decompressed)
            .map_err(|e| format!("Failed to decompress atlas: {}", e))?;
        String::from_utf8(decompressed)
            .map_err(|e| format!("Invalid decompressed atlas UTF-8: {}", e))?
    };

    let atlas: serde_json::Value = serde_json::from_str(&atlas_str)
        .map_err(|e| format!("Failed to parse atlas JSON: {}", e))?;

    // Build the full frame name: map{area}{map}_{frame_name}
    let full_frame_name = format!("map{}{}_{}", area, map, frame_name);

    let frame = atlas
        .get("frames")
        .and_then(|f| f.get(&full_frame_name))
        .and_then(|f| f.get("frame"))
        .ok_or_else(|| format!("Frame '{}' not found in atlas", full_frame_name))?;

    let fx = frame.get("x").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    let fy = frame.get("y").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    let fw = frame.get("w").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    let fh = frame.get("h").and_then(|v| v.as_u64()).unwrap_or(0) as u32;

    if fw == 0 || fh == 0 {
        return Err("Invalid frame dimensions".to_string());
    }

    // Read the sprite sheet and crop - do heavy work in blocking thread
    let image_path_clone = image_path.clone();
    let apply_tint = tint_cyan.unwrap_or(false);
    let result = tokio::task::spawn_blocking(move || -> Result<String, String> {
        let img = image::open(&image_path_clone)
            .map_err(|e| format!("Failed to open sprite sheet: {}", e))?;
        let cropped = img.crop_imm(fx, fy, fw, fh);

        // Apply cyan tint if requested: replace RGB with cyan, preserve alpha
        let output = if apply_tint {
            let mut rgba = cropped.to_rgba8();
            for pixel in rgba.pixels_mut() {
                if pixel[3] == 0 {
                    continue;
                } // skip fully transparent
                  // Original pixel luminescence (0.0 - 1.0)
                let lum =
                    (pixel[0] as f32 * 0.299 + pixel[1] as f32 * 0.587 + pixel[2] as f32 * 0.114)
                        / 255.0;

                // For white dotted lines, lum is high. Map brightness to cyan.
                // Pure white -> Cyan (#26c6da or similar bright color)
                pixel[0] = (38.0 * lum) as u8; // R
                pixel[1] = (198.0 * lum) as u8; // G
                pixel[2] = (218.0 * lum) as u8; // B
                                                // Keep original alpha (pixel[3]) to preserve anti-aliasing edges
            }
            image::DynamicImage::ImageRgba8(rgba)
        } else {
            cropped
        };

        let mut buf = std::io::Cursor::new(Vec::new());
        output
            .write_to(&mut buf, image::ImageFormat::Png)
            .map_err(|e| format!("Failed to encode cropped sprite: {}", e))?;

        let b64 = base64::engine::general_purpose::STANDARD.encode(buf.into_inner());
        Ok(format!("data:image/png;base64,{}", b64))
    })
    .await
    .map_err(|e| format!("Spawn blocking failed: {}", e))?;

    result
}

/// Reposition the formation hint window to follow the game window
fn reposition_formation_hint(app: &tauri::AppHandle) {
    let state = app.state::<AppState>();
    let rect = *state.formation_hint_rect.lock().unwrap();
    if !rect.visible {
        return;
    }
    let game_win = match app.get_window("game") {
        Some(w) => w,
        None => return,
    };
    let hint_win = match app.get_window("formation-hint") {
        Some(w) => w,
        None => return,
    };
    let inner_pos = match game_win.inner_position() {
        Ok(p) => p,
        Err(_) => return,
    };
    let screen_x = inner_pos.x + rect.dx;
    let screen_y = inner_pos.y + rect.dy;
    let _ = hint_win.set_position(tauri::PhysicalPosition::new(screen_x, screen_y));
}

/// Get battle log records
#[tauri::command]
async fn get_battle_logs(
    limit: Option<usize>,
    offset: Option<usize>,
    date_from: Option<String>,
    date_to: Option<String>,
    state: tauri::State<'_, api::models::GameState>,
) -> Result<serde_json::Value, String> {
    let inner = state.inner.read().await;
    if let (Some(from), Some(to)) = (&date_from, &date_to) {
        let records = inner.sortie.battle_logger.get_records_by_date_range(from, to);
        let total = records.len();
        Ok(serde_json::json!({
            "records": records,
            "total": total,
        }))
    } else {
        let limit = limit.unwrap_or(50);
        let offset = offset.unwrap_or(0);
        let records = inner.sortie.battle_logger.get_records(limit, offset);
        let total = inner.sortie.battle_logger.record_count();
        Ok(serde_json::json!({
            "records": records,
            "total": total,
        }))
    }
}

/// Close the game window
#[tauri::command]
async fn close_game_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(hint_win) = app.get_window("formation-hint") {
        let _ = hint_win.close();
    }
    if let Some(win) = app.get_window("game") {
        // Force save cookies immediately before closing
        match save_game_cookies(app.clone()).await {
            Ok(n) => info!("Saved {} cookies on explicit close", n),
            Err(e) => log::warn!("Failed to save cookies on close: {}", e),
        }
        win.close().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// KanColle game native resolution
const GAME_WIDTH: f64 = 1200.0;
const GAME_HEIGHT: f64 = 720.0;
/// Height of the injected control bar (pixels, not scaled by zoom)
const CONTROL_BAR_HEIGHT: f64 = 28.0;
/// macOS title bar height — tao/tauri includes titlebar in inner_size on macOS (tauri-apps/tauri#6333)
#[cfg(target_os = "macos")]
const MACOS_TITLEBAR_HEIGHT: f64 = 28.0;
#[cfg(not(target_os = "macos"))]
const MACOS_TITLEBAR_HEIGHT: f64 = 0.0;

/// Set zoom level for the game window and resize the window accordingly
#[tauri::command]
fn set_game_zoom(app: tauri::AppHandle, zoom: f64) -> Result<(), String> {
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
fn toggle_game_mute(
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
fn get_game_mute(state: State<AppState>) -> bool {
    state.game_muted.load(Ordering::Relaxed)
}

#[tauri::command]
fn set_formation_hint_enabled(
    app: tauri::AppHandle,
    state: State<AppState>,
    enabled: bool,
) -> Result<(), String> {
    state
        .formation_hint_enabled
        .store(enabled, Ordering::Relaxed);

    // Persist to disk
    if let Ok(dir) = app.path().app_local_data_dir() {
        let _ = std::fs::write(
            dir.join("local").join("formation_hint_enabled"),
            if enabled { "1" } else { "0" },
        );
    }

    // Hide hint window immediately when disabled
    if !enabled {
        crate::api::hide_formation_hint(&app);
    }

    info!("Formation hint set to {}", if enabled { "enabled" } else { "disabled" });
    Ok(())
}

#[tauri::command]
fn get_formation_hint_enabled(state: State<AppState>) -> bool {
    state.formation_hint_enabled.load(Ordering::Relaxed)
}

#[tauri::command]
fn set_taiha_alert_enabled(
    app: tauri::AppHandle,
    state: State<AppState>,
    enabled: bool,
) -> Result<(), String> {
    state.taiha_alert_enabled.store(enabled, Ordering::Relaxed);

    if let Ok(dir) = app.path().app_local_data_dir() {
        let _ = std::fs::write(
            dir.join("local").join("taiha_alert_enabled"),
            if enabled { "1" } else { "0" },
        );
    }

    info!("Taiha alert set to {}", if enabled { "enabled" } else { "disabled" });
    Ok(())
}

#[tauri::command]
fn get_taiha_alert_enabled(state: State<AppState>) -> bool {
    state.taiha_alert_enabled.load(Ordering::Relaxed)
}

/// Show or hide the overlay webview (called from overlay JS).
#[tauri::command]
fn set_overlay_visible(app: tauri::AppHandle, visible: bool) -> Result<(), String> {
    let overlay = app
        .get_webview("game-overlay")
        .ok_or("Overlay not found")?;
    if visible {
        let win = app.get_window("game").ok_or("Game window not found")?;
        let size = win.inner_size().map_err(|e| e.to_string())?;
        overlay
            .set_position(tauri::LogicalPosition::new(0.0, 0.0))
            .map_err(|e| e.to_string())?;
        overlay.set_size(size).map_err(|e| e.to_string())?;
    } else {
        overlay
            .set_size(tauri::LogicalSize::new(1.0, 1.0))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Get quest progress for active quests
#[tauri::command]
async fn get_quest_progress(
    state: tauri::State<'_, api::models::GameState>,
) -> Result<Vec<quest_progress::QuestProgressSummary>, String> {
    let mut inner = state.inner.write().await;
    let path = inner.quest_progress_path.clone();
    let defs = inner.history.sortie_quest_defs.clone();
    let aq = inner.history.active_quests.clone();
    Ok(quest_progress::get_active_progress(
        &mut inner.history.quest_progress,
        &aq,
        &defs,
        &path,
    ))
}

/// Manually update quest progress (toggle area or set count)
#[tauri::command]
async fn update_quest_progress(
    quest_id: i32,
    area: Option<String>,
    count: Option<i32>,
    state: tauri::State<'_, api::models::GameState>,
    app: tauri::AppHandle,
) -> Result<bool, String> {
    let mut inner = state.inner.write().await;
    let path = inner.quest_progress_path.clone();
    let defs = inner.history.sortie_quest_defs.clone();
    let changed = quest_progress::manual_update(
        &mut inner.history.quest_progress,
        quest_id,
        area,
        count,
        &defs,
        &path,
    );
    if changed {
        let aq = inner.history.active_quests.clone();
        let progress =
            quest_progress::get_active_progress(&mut inner.history.quest_progress, &aq, &defs, &path);
        let _ = app.emit("quest-progress-updated", &progress);
    }
    Ok(changed)
}

/// Clear all quest progress data
#[tauri::command]
async fn clear_quest_progress(
    state: tauri::State<'_, api::models::GameState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let mut inner = state.inner.write().await;
    inner.history.quest_progress = quest_progress::QuestProgressState::default();
    quest_progress::save_progress(&inner.quest_progress_path, &inner.history.quest_progress);
    info!("Cleared quest progress");
    let progress: Vec<quest_progress::QuestProgressSummary> = Vec::new();
    let _ = app.emit("quest-progress-updated", &progress);
    Ok(())
}

// =============================================================================
// Google Drive Sync Commands
// =============================================================================

/// Start Google Drive OAuth login flow (opens browser)
#[tauri::command]
async fn drive_login(
    app: tauri::AppHandle,
    state: tauri::State<'_, GameState>,
) -> Result<(), String> {
    info!("drive_login: started");
    let inner = state.inner.read().await;
    let data_dir = inner.data_dir.clone();
    drop(inner);

    let (client_id, client_secret) = drive_sync::auth::client_credentials()
        .ok_or("Google Drive sync is not available in this build.")?;

    let auth = drive_sync::auth::authenticate(client_id, client_secret, &data_dir).await?;

    // Start sync engine
    let sync_tx = drive_sync::engine::start_sync_engine(app.clone(), data_dir, auth).await;

    // Store notifier in GameState
    let mut inner = state.inner.write().await;
    inner.sync_notifier = Some(sync_tx);

    info!("Google Drive sync started");
    Ok(())
}

/// Log out from Google Drive
#[tauri::command]
async fn drive_logout(state: tauri::State<'_, GameState>) -> Result<(), String> {
    let mut inner = state.inner.write().await;

    // Shut down sync engine
    if let Some(tx) = inner.sync_notifier.take() {
        let _ = tx.send(drive_sync::SyncCommand::Shutdown).await;
    }

    drive_sync::auth::logout(&inner.data_dir);
    info!("Google Drive logged out");
    Ok(())
}

/// Get Google Drive sync status
#[tauri::command]
async fn get_drive_status(
    state: tauri::State<'_, GameState>,
) -> Result<drive_sync::SyncStatus, String> {
    let inner = state.inner.read().await;
    let has_notifier = inner.sync_notifier.is_some();

    let manifest = drive_sync::load_manifest(&inner.data_dir);
    let last_sync = manifest.last_full_sync.map(|t| t.to_rfc3339());

    Ok(drive_sync::SyncStatus {
        authenticated: has_notifier,
        email: None,
        syncing: false,
        last_sync,
        error: None,
    })
}

/// Force a full sync with Google Drive
#[tauri::command]
async fn drive_force_sync(state: tauri::State<'_, GameState>) -> Result<(), String> {
    let inner = state.inner.read().await;
    let tx = inner
        .sync_notifier
        .as_ref()
        .ok_or("Not connected to Google Drive")?;
    tx.send(drive_sync::SyncCommand::FullSync)
        .await
        .map_err(|e| format!("Failed to send sync command: {}", e))?;
    Ok(())
}

/// Migrate old flat data directory layout to new sync/ + local/ structure.
/// Idempotent: skips files that already exist at the new location.
fn migrate_data_dir(data_dir: &std::path::Path) {
    use std::fs;

    let sync_dir = data_dir.join("sync");
    let local_dir = data_dir.join("local");

    // Create target directories
    let _ = fs::create_dir_all(sync_dir.join("battle_logs"));
    let _ = fs::create_dir_all(sync_dir.join("raw_api"));
    let _ = fs::create_dir_all(&local_dir);

    // Sync targets: move files/dirs into sync/
    let sync_moves: &[(&str, &str)] = &[
        ("quest_progress.json", "sync/quest_progress.json"),
        ("improved_equipment.json", "sync/improved_equipment.json"),
    ];
    for &(old, new) in sync_moves {
        let old_path = data_dir.join(old);
        let new_path = data_dir.join(new);
        if old_path.exists() && !new_path.exists() {
            match fs::rename(&old_path, &new_path) {
                Ok(_) => info!("Migrated {} -> {}", old, new),
                Err(e) => log::warn!("Failed to migrate {} -> {}: {}", old, new, e),
            }
        }
    }

    // Sync directories: move contents (not the dir itself, since we already created them)
    let sync_dir_moves: &[(&str, &str)] = &[
        ("battle_logs", "sync/battle_logs"),
        ("raw_api", "sync/raw_api"),
    ];
    for &(old, new) in sync_dir_moves {
        let old_dir = data_dir.join(old);
        let new_dir = data_dir.join(new);
        if old_dir.is_dir() && old_dir != new_dir {
            if let Ok(entries) = fs::read_dir(&old_dir) {
                for entry in entries.flatten() {
                    let dest = new_dir.join(entry.file_name());
                    if !dest.exists() {
                        let _ = fs::rename(entry.path(), &dest);
                    }
                }
            }
            // Remove old dir if empty
            let _ = fs::remove_dir(&old_dir);
        }
    }

    // Local targets: move to local/
    let local_moves: &[(&str, &str)] = &[
        ("dmm_cookies.json", "local/dmm_cookies.json"),
        ("game_muted", "local/game_muted"),
    ];
    for &(old, new) in local_moves {
        let old_path = data_dir.join(old);
        let new_path = data_dir.join(new);
        if old_path.exists() && !new_path.exists() {
            match fs::rename(&old_path, &new_path) {
                Ok(_) => info!("Migrated {} -> {}", old, new),
                Err(e) => log::warn!("Failed to migrate {} -> {}: {}", old, new, e),
            }
        }
    }

    // Migrate game-webview directory
    let old_webview = data_dir.join("game-webview");
    let new_webview = data_dir.join("local").join("game-webview");
    if old_webview.is_dir() && !new_webview.exists() {
        match fs::rename(&old_webview, &new_webview) {
            Ok(_) => info!("Migrated game-webview -> local/game-webview"),
            Err(e) => log::warn!("Failed to migrate game-webview: {}", e),
        }
    }

    info!("Data directory migration check complete");
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
            formation_hint_rect: Mutex::new(FormationHintRect::default()),
            game_zoom: Mutex::new(1.0),
        })
        .invoke_handler(tauri::generate_handler![
            get_proxy_port,
            is_ca_installed,
            install_ca_cert,
            open_game_window,
            close_game_window,
            get_expeditions,
            check_expedition_cmd,
            get_sortie_quests,
            get_active_quest_ids,
            check_sortie_quest_cmd,
            get_map_recommendations,
            check_map_recommendation_cmd,
            get_battle_logs,
            get_improvement_list,
            get_ship_list,
            get_equipment_list,
            save_game_cookies,
            clear_improved_history,
            clear_battle_logs,
            clear_raw_api,
            set_raw_api_enabled,
            get_raw_api_enabled,
            clear_cookies,
            get_cached_resource,
            get_map_sprite,
            clear_resource_cache,
            clear_browser_cache,
            set_game_zoom,
            toggle_game_mute,
            get_game_mute,
            set_overlay_visible,
            set_formation_hint_enabled,
            get_formation_hint_enabled,
            set_taiha_alert_enabled,
            get_taiha_alert_enabled,
            get_quest_progress,
            update_quest_progress,
            clear_quest_progress,
            drive_login,
            drive_logout,
            get_drive_status,
            drive_force_sync
        ])
        .setup(|app| {
            let data_dir = app
                .path()
                .app_local_data_dir()
                .unwrap_or_else(|_| PathBuf::from("."));

            // Migrate old flat layout to sync/ + local/ structure
            migrate_data_dir(&data_dir);

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
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
