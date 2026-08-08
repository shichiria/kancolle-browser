use log::info;
use std::path::PathBuf;

/// Registrable DMM domains whose cookies are persisted for auto-login.
const DMM_DOMAIN_SUFFIXES: [&str; 2] = ["dmm.com", "dmm.co.jp"];

/// True if `domain` (with or without a leading dot) is a DMM domain or subdomain.
fn is_dmm_domain(domain: &str) -> bool {
    let d = domain.trim_start_matches('.');
    DMM_DOMAIN_SUFFIXES
        .iter()
        .any(|suffix| d == *suffix || d.ends_with(&format!(".{}", suffix)))
}

/// Collect all DMM-related cookies from the game webview.
///
/// Uses `cookies()` (full enumeration) with our own domain-suffix filter instead
/// of `cookies_for_url()`: wry 0.54's WKWebView backend filters `cookies_for_url`
/// by exact domain equality, which silently drops `.dmm.com`-scoped session
/// cookies on macOS — precisely the ones DMM login needs.
pub(crate) fn collect_dmm_cookies(game_wv: &tauri::Webview) -> Vec<serde_json::Value> {
    let cookies = match game_wv.cookies() {
        Ok(c) => c,
        Err(e) => {
            log::warn!("Failed to enumerate cookies: {}", e);
            return Vec::new();
        }
    };

    let mut all_cookies: Vec<serde_json::Value> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for cookie in cookies {
        let domain = cookie.domain().unwrap_or("");
        if !is_dmm_domain(domain) {
            continue;
        }
        let key = format!("{}={}", cookie.name(), domain);
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
    all_cookies
}

/// Write collected cookies to the persistence file. Returns the cookie count.
pub(crate) fn write_cookie_file(
    app: &tauri::AppHandle,
    cookies: &[serde_json::Value],
) -> Result<usize, String> {
    let path = cookie_file_path(app);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(cookies).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    info!("Saved {} DMM cookies to {}", cookies.len(), path.display());
    Ok(cookies.len())
}

/// Build `cookie::Cookie` values from the saved JSON entries, re-written with a
/// +365d expiry so DMM's session cookies survive across app restarts.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn build_restore_cookies(
    raw_cookies: &[serde_json::Value],
) -> Vec<tauri::webview::cookie::Cookie<'static>> {
    use tauri::webview::cookie::time::{Duration as CookieDuration, OffsetDateTime};
    use tauri::webview::cookie::Cookie;

    let expires = OffsetDateTime::now_utc() + CookieDuration::days(365);
    let mut cookies = Vec::new();
    for c in raw_cookies {
        let (Some(name), Some(value)) = (
            c.get("name").and_then(|v| v.as_str()),
            c.get("value").and_then(|v| v.as_str()),
        ) else {
            continue;
        };
        // A domain is required: wry's WKWebView backend panics when building an
        // NSHTTPCookie without one (cookieWithProperties returns nil).
        let Some(domain) = c
            .get("domain")
            .and_then(|v| v.as_str())
            .filter(|d| !d.is_empty())
        else {
            log::warn!("Skipping saved cookie {} without domain", name);
            continue;
        };
        let builder = Cookie::build((name.to_string(), value.to_string()))
            .domain(domain.to_string())
            .path(
                c.get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("/")
                    .to_string(),
            )
            .secure(c.get("secure").and_then(|v| v.as_bool()).unwrap_or(true))
            .http_only(
                c.get("http_only")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
            )
            .expires(expires);
        cookies.push(builder.build());
    }
    cookies
}

/// Restore saved DMM cookies directly into the WKWebView cookie store (macOS).
///
/// `document.cookie` injection cannot restore these cookies: the injection page
/// (`about:blank`) has an opaque origin, cross-domain cookies cannot be set from
/// JS, and httpOnly cookies are invisible to `document.cookie` entirely.
/// `set_cookie` (WKHTTPCookieStore) has none of those restrictions.
#[cfg(target_os = "macos")]
pub(crate) async fn restore_cookies_native(
    app: &tauri::AppHandle,
    game_wv: &tauri::Webview,
) -> usize {
    let path = cookie_file_path(app);
    let raw_cookies = match tokio::fs::read_to_string(&path).await {
        Ok(content) => match serde_json::from_str::<Vec<serde_json::Value>>(&content) {
            Ok(v) => v,
            Err(e) => {
                log::warn!("Cookie file parse failed: {}", e);
                return 0;
            }
        },
        Err(_) => return 0, // no saved cookies yet
    };

    let mut count = 0;
    for cookie in build_restore_cookies(&raw_cookies) {
        let name = cookie.name().to_string();
        match game_wv.set_cookie(cookie) {
            Ok(()) => count += 1,
            Err(e) => log::warn!("set_cookie failed for {}: {}", name, e),
        }
    }
    info!("Natively restored {} DMM cookies (macOS)", count);
    count
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

/// Save cookies from the game window to a file.
/// DMM uses session cookies which are deleted when the webview process dies;
/// we extract and save them to JSON so they can be restored on next launch.
#[tauri::command]
pub(crate) async fn save_game_cookies(app: tauri::AppHandle) -> Result<usize, String> {
    use tauri::Manager;

    let game_wv = app
        .get_webview("game-content")
        .ok_or("Game webview not found")?;

    let all_cookies = collect_dmm_cookies(&game_wv);
    if all_cookies.is_empty() {
        return Ok(0);
    }
    write_cookie_file(&app, &all_cookies)
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
