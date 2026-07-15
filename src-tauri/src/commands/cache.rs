use base64::Engine;
use log::info;
use std::path::PathBuf;
use tauri::Manager;

/// Reset all browsing data (cookies, session, cache, etc.).
/// If the game webview is open, uses the WebView API first, then deletes files.
#[tauri::command]
pub(crate) fn reset_browser_data(app: tauri::AppHandle) -> Result<String, String> {
    // Windows: require game window to be closed (EBWebView directory is locked)
    #[cfg(not(target_os = "macos"))]
    if app.get_window("game").is_some() {
        return Err("ゲーム画面を閉じてから実行してください".to_string());
    }

    let mut deleted = false;

    // macOS: if game webview is open, clear via API and close the window
    #[cfg(target_os = "macos")]
    {
        if let Some(game_wv) = app.get_webview("game-content") {
            if let Err(e) = game_wv.clear_all_browsing_data() {
                log::warn!("Failed to clear browsing data via API: {}", e);
            } else {
                info!("Cleared browsing data via WebView API");
                deleted = true;
            }
        }
        if let Some(win) = app.get_window("game") {
            let _ = win.close();
        }
    }

    // Windows: delete WebView2 user data
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
            if let Err(e) = std::fs::remove_dir_all(&webview_dir) {
                log::warn!("Failed to delete WebView2 data: {}", e);
            } else {
                info!("Deleted WebView2 data: {}", webview_dir.display());
                deleted = true;
            }
        }
    }

    // macOS: delete WKWebView caches and WKWebsiteDataStore data
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = dirs::home_dir() {
            let app_names = ["kancolle-browser", "com.eo.kancolle-browser"];

            // ~/Library/Caches/<app-name>/ (HTTP cache, WebKit cache)
            let caches_dir = home.join("Library/Caches");
            for app_name in &app_names {
                let app_cache = caches_dir.join(app_name);
                if app_cache.exists() {
                    if let Err(e) = std::fs::remove_dir_all(&app_cache) {
                        log::warn!("Failed to delete cache for {}: {}", app_name, e);
                    } else {
                        info!("Deleted WKWebView cache: {}", app_name);
                        deleted = true;
                    }
                }
            }

            // ~/Library/WebKit/<app-name>/ (WKWebsiteDataStore: cookies, local storage, etc.)
            let webkit_dir = home.join("Library/WebKit");
            for app_name in &app_names {
                let app_data = webkit_dir.join(app_name);
                if app_data.exists() {
                    if let Err(e) = std::fs::remove_dir_all(&app_data) {
                        log::warn!("Failed to delete WebKit data for {}: {}", app_name, e);
                    } else {
                        info!("Deleted WKWebsiteDataStore: {}", app_name);
                        deleted = true;
                    }
                }
            }

            // ~/Library/HTTPStorages/<app-name>/ (cookies and HTTP storage)
            let http_storages_dir = home.join("Library/HTTPStorages");
            for app_name in &app_names {
                let app_storage = http_storages_dir.join(app_name);
                if app_storage.exists() {
                    if let Err(e) = std::fs::remove_dir_all(&app_storage) {
                        log::warn!("Failed to delete HTTPStorages for {}: {}", app_name, e);
                    } else {
                        info!("Deleted HTTPStorages: {}", app_name);
                        deleted = true;
                    }
                }
            }
        }
    }

    // Delete saved cookies
    let cookie_path = crate::cookie::cookie_file_path(&app);
    if cookie_path.exists() {
        if let Err(e) = std::fs::remove_file(&cookie_path) {
            log::warn!("Failed to delete cookies: {}", e);
        } else {
            info!("Deleted saved cookies");
            deleted = true;
        }
    }

    if deleted {
        Ok(
            "ブラウザデータをリセットしました（次回ゲーム起動時に再ログインが必要です）"
                .to_string(),
        )
    } else {
        Ok("リセット対象のデータはありません".to_string())
    }
}

/// Get a cached game resource (image or JSON) from the local cache.
/// For images, returns a data URI (data:image/png;base64,...).
/// For JSON/text files, returns the raw content string.
/// Returns empty string if the file is not cached.
#[tauri::command]
pub(crate) async fn get_cached_resource(
    app: tauri::AppHandle,
    path: String,
) -> Result<String, String> {
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
pub(crate) async fn clear_resource_cache(app: tauri::AppHandle) -> Result<String, String> {
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
pub(crate) async fn clear_browser_cache(app: tauri::AppHandle) -> Result<String, String> {
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
                if dir_path.exists() && std::fs::remove_dir_all(&dir_path).is_ok() {
                    deleted += 1;
                    info!("Deleted browser cache: {}", dir_name);
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
pub(crate) async fn get_map_sprite(
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
