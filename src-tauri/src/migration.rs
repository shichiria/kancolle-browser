use log::info;

/// Migrate old flat data directory layout to new sync/ + local/ structure.
/// Idempotent: skips files that already exist at the new location.
pub(crate) fn migrate_data_dir(data_dir: &std::path::Path) {
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
