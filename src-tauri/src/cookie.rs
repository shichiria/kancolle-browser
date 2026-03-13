use log::info;
use std::path::PathBuf;
use url::Url;

/// Generate a JavaScript snippet that restores the saved DMM session cookies
/// directly via `document.cookie`. This bypasses strict native API validations
/// (e.g. WebView2 dropping SameSite=None cookies on domains with a dot prefix).
pub(crate) async fn build_cookie_restore_script(app: &tauri::AppHandle) -> String {
    let path = cookie_file_path(app);
    let raw_cookies = match tokio::fs::read_to_string(&path).await {
        Ok(content) => match serde_json::from_str::<Vec<serde_json::Value>>(&content) {
            Ok(v) => v,
            Err(_) => return String::new(),
        },
        Err(_) => return String::new(),
    };

    // Only restore cookies on about:blank (initial page before DMM navigation).
    // Running on other pages would overwrite fresh session cookies set by the login flow.
    let mut script = String::from("(function() {\n  if (window.location.href !== 'about:blank') return;\n");
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
pub(crate) fn cookie_file_path(app: &tauri::AppHandle) -> PathBuf {
    use tauri::Manager;
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
pub(crate) async fn save_game_cookies(app: tauri::AppHandle) -> Result<usize, String> {
    use tauri::Manager;

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

/// Clear saved cookies
#[tauri::command]
pub(crate) fn clear_cookies(app: tauri::AppHandle) -> Result<(), String> {
    let path = cookie_file_path(&app);
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    info!("Cleared saved cookies");
    Ok(())
}
