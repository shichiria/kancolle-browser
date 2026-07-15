//! Small typed persistence helpers for local UI preferences.

use serde::{de::DeserializeOwned, Serialize};
use std::path::{Path, PathBuf};
use tauri::Manager;

pub const GAME_MUTED: &str = "game_muted";
pub const FORMATION_HINT_ENABLED: &str = "formation_hint_enabled";
pub const TAIHA_ALERT_ENABLED: &str = "taiha_alert_enabled";
pub const MINIMAP_ENABLED: &str = "minimap_enabled";
pub const BATTLE_INFO_ENABLED: &str = "battle_info_enabled";
pub const MINIMAP_POSITION: &str = "minimap_position.json";
pub const MINIMAP_SIZE: &str = "minimap_size.json";

fn local_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_local_data_dir()
        .map(|path| path.join("local"))
        .map_err(|error| error.to_string())
}

fn local_path(data_dir: &Path, name: &str) -> PathBuf {
    data_dir.join("local").join(name)
}

pub fn persist_flag(app: &tauri::AppHandle, name: &str, enabled: bool) -> Result<(), String> {
    let dir = local_dir(app)?;
    std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    std::fs::write(dir.join(name), if enabled { "1" } else { "0" })
        .map_err(|error| error.to_string())
}

pub fn restore_flag(data_dir: &Path, name: &str, default: bool) -> bool {
    match std::fs::read_to_string(local_path(data_dir, name)) {
        Ok(value) => match value.trim() {
            "1" | "true" => true,
            "0" | "false" => false,
            unexpected => {
                log::warn!(
                    "Invalid persisted flag {}={:?}; using default",
                    name,
                    unexpected
                );
                default
            }
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => default,
        Err(error) => {
            log::warn!("Failed to restore {}: {}; using default", name, error);
            default
        }
    }
}

pub fn persist_json<T: Serialize>(
    app: &tauri::AppHandle,
    name: &str,
    value: &T,
) -> Result<(), String> {
    let dir = local_dir(app)?;
    std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    let json = serde_json::to_vec(value).map_err(|error| error.to_string())?;
    std::fs::write(dir.join(name), json).map_err(|error| error.to_string())
}

pub fn restore_json<T: DeserializeOwned>(data_dir: &Path, name: &str) -> Option<T> {
    let path = local_path(data_dir, name);
    match std::fs::read(&path) {
        Ok(bytes) => match serde_json::from_slice(&bytes) {
            Ok(value) => Some(value),
            Err(error) => {
                log::warn!("Failed to parse {}: {}", path.display(), error);
                None
            }
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => {
            log::warn!("Failed to restore {}: {}", path.display(), error);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_flag_uses_declared_default() {
        let missing = std::env::temp_dir().join("kancolle-settings-missing-test");
        assert!(restore_flag(&missing, FORMATION_HINT_ENABLED, true));
        assert!(!restore_flag(&missing, GAME_MUTED, false));
    }
}
