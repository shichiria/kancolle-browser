use std::path::{Path, PathBuf};

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use tauri::Manager;
use tauri_plugin_dialog::DialogExt;

use super::platform;

const DEFAULT_FILENAME_PATTERN: &str = "kancolle_{timestamp}.png";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ScreenshotSettings {
    pub directory: String,
    pub filename_pattern: String,
}

impl Default for ScreenshotSettings {
    fn default() -> Self {
        Self {
            directory: default_screenshot_directory()
                .to_string_lossy()
                .into_owned(),
            filename_pattern: DEFAULT_FILENAME_PATTERN.to_string(),
        }
    }
}

fn application_directory() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."))
}

fn default_screenshot_directory() -> PathBuf {
    application_directory().join("screenshot")
}

fn load_settings(app: &tauri::AppHandle) -> Result<ScreenshotSettings, String> {
    let data_dir = app
        .path()
        .app_local_data_dir()
        .map_err(|error| error.to_string())?;
    let settings = crate::settings::restore_json(&data_dir, crate::settings::SCREENSHOT_SETTINGS)
        .unwrap_or_default();
    normalize_settings(settings)
}

fn normalize_settings(mut settings: ScreenshotSettings) -> Result<ScreenshotSettings, String> {
    let directory = settings.directory.trim();
    if directory.is_empty() {
        return Err("保存フォルダを指定してください".to_string());
    }
    let directory = PathBuf::from(directory);
    settings.directory = if directory.is_absolute() {
        directory
    } else {
        application_directory().join(directory)
    }
    .to_string_lossy()
    .into_owned();

    settings.filename_pattern = settings.filename_pattern.trim().to_string();
    if settings.filename_pattern.is_empty() {
        return Err("ファイル名を指定してください".to_string());
    }
    if settings
        .filename_pattern
        .chars()
        .any(|character| character.is_control() || r#"<>:"/\|?*"#.contains(character))
    {
        return Err("ファイル名に使用できない文字が含まれています".to_string());
    }
    if matches!(settings.filename_pattern.as_str(), "." | "..") {
        return Err("このファイル名は使用できません".to_string());
    }
    if !settings
        .filename_pattern
        .to_ascii_lowercase()
        .ends_with(".png")
    {
        settings.filename_pattern.push_str(".png");
    }
    Ok(settings)
}

fn render_filename(pattern: &str, now: DateTime<Local>) -> String {
    pattern
        .replace("{timestamp}", &now.format("%Y%m%d_%H%M%S_%3f").to_string())
        .replace("{date}", &now.format("%Y%m%d").to_string())
        .replace("{time}", &now.format("%H%M%S").to_string())
}

fn available_path(directory: &Path, filename: &str) -> PathBuf {
    let requested = directory.join(filename);
    if !requested.exists() {
        return requested;
    }

    let file = Path::new(filename);
    let stem = file
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("kancolle");
    let extension = file
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("png");
    for sequence in 1..=9999 {
        let candidate = directory.join(format!("{stem}_{sequence}.{extension}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    directory.join(format!(
        "{stem}_{}.{}",
        Local::now().timestamp_millis(),
        extension
    ))
}

#[tauri::command]
pub(crate) fn get_screenshot_settings(app: tauri::AppHandle) -> Result<ScreenshotSettings, String> {
    load_settings(&app)
}

#[tauri::command]
pub(crate) fn get_default_screenshot_settings() -> ScreenshotSettings {
    ScreenshotSettings::default()
}

#[tauri::command]
pub(crate) fn set_screenshot_settings(
    app: tauri::AppHandle,
    settings: ScreenshotSettings,
) -> Result<ScreenshotSettings, String> {
    let settings = normalize_settings(settings)?;
    std::fs::create_dir_all(&settings.directory).map_err(|error| {
        format!(
            "保存フォルダを作成できません: {} ({error})",
            settings.directory
        )
    })?;
    crate::settings::persist_json(&app, crate::settings::SCREENSHOT_SETTINGS, &settings)?;
    log::info!(
        "Screenshot settings saved: directory={}, filename_pattern={}",
        settings.directory,
        settings.filename_pattern
    );
    Ok(settings)
}

#[tauri::command]
pub(crate) async fn choose_screenshot_directory(
    app: tauri::AppHandle,
) -> Result<Option<String>, String> {
    let settings = load_settings(&app)?;
    let requested = PathBuf::from(&settings.directory);
    let starting_directory = if requested.is_dir() {
        requested
    } else {
        application_directory()
    };

    let (sender, receiver) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title("スクリーンショット保存フォルダ")
        .set_directory(starting_directory)
        .pick_folder(move |folder| {
            let selected = folder
                .and_then(|path| path.into_path().ok())
                .map(|path| path.to_string_lossy().into_owned());
            let _ = sender.send(selected);
        });
    receiver
        .await
        .map_err(|_| "フォルダ選択ダイアログが終了しました".to_string())
}

#[tauri::command]
pub(crate) async fn take_game_screenshot(app: tauri::AppHandle) -> Result<String, String> {
    let settings = load_settings(&app)?;
    let directory = PathBuf::from(&settings.directory);
    std::fs::create_dir_all(&directory).map_err(|error| {
        format!(
            "保存フォルダを作成できません: {} ({error})",
            directory.display()
        )
    })?;
    let filename = render_filename(&settings.filename_pattern, Local::now());
    let path = available_path(&directory, &filename);
    let game_window = app
        .get_window("game")
        .ok_or_else(|| "ゲームウィンドウが見つかりません".to_string())?;
    let save_path = path.clone();

    tokio::task::spawn_blocking(move || platform::save_screenshot(&game_window, &save_path))
        .await
        .map_err(|error| format!("スクリーンショット処理に失敗しました: {error}"))??;

    let saved = path.to_string_lossy().into_owned();
    log::info!("Game screenshot saved: {saved}");
    crate::action_log::log("Command", "take_game_screenshot", &saved);
    Ok(saved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn default_directory_is_screenshot_beside_executable() {
        assert_eq!(
            default_screenshot_directory(),
            application_directory().join("screenshot")
        );
    }

    #[test]
    fn filename_placeholders_render_as_png() {
        let now = Local
            .with_ymd_and_hms(2026, 7, 24, 3, 4, 5)
            .single()
            .unwrap();
        assert_eq!(
            render_filename("kc_{date}_{time}_{timestamp}.png", now),
            "kc_20260724_030405_20260724_030405_000.png"
        );
    }

    #[test]
    fn filename_is_normalized_and_paths_are_rejected() {
        let normalized = normalize_settings(ScreenshotSettings {
            directory: "screenshot".to_string(),
            filename_pattern: "capture_{timestamp}".to_string(),
        })
        .unwrap();
        assert!(normalized.directory.ends_with("screenshot"));
        assert_eq!(normalized.filename_pattern, "capture_{timestamp}.png");

        assert!(normalize_settings(ScreenshotSettings {
            directory: "screenshot".to_string(),
            filename_pattern: "../escape.png".to_string(),
        })
        .is_err());
    }
}
