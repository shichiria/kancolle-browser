use log::{error, info, warn};
use md5::{Digest, Md5};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;

use tauri::Manager;

use super::auth::DriveAuthenticator;
use super::files::{self, Hub};
use super::{
    load_manifest, save_manifest, SyncCommand, SyncFileEntry, SyncManifest, SyncStatus,
    SYNC_TARGETS,
};

/// Polling interval for remote changes (5 minutes).
const POLL_INTERVAL: std::time::Duration = std::time::Duration::from_secs(300);

/// Compute MD5 hash of a file.
fn file_md5(path: &Path) -> Option<String> {
    let data = std::fs::read(path).ok()?;
    let mut hasher = Md5::new();
    hasher.update(&data);
    Some(format!("{:x}", hasher.finalize()))
}

/// Get local file modification time as DateTime<Utc>.
fn file_mtime(path: &Path) -> Option<chrono::DateTime<chrono::Utc>> {
    let meta = std::fs::metadata(path).ok()?;
    let st = meta.modified().ok()?;
    Some(chrono::DateTime::<chrono::Utc>::from(st))
}

/// Start the sync engine background task.
/// Returns an mpsc::Sender to send commands to the engine.
pub async fn start_sync_engine(
    app: AppHandle,
    data_dir: PathBuf,
    auth: DriveAuthenticator,
) -> mpsc::Sender<SyncCommand> {
    let (tx, rx) = mpsc::channel::<SyncCommand>(64);

    let hub = files::build_hub(auth);

    tokio::spawn(async move {
        run_sync_loop(app, data_dir, hub, rx).await;
    });

    tx
}

async fn run_sync_loop(
    app: AppHandle,
    data_dir: PathBuf,
    hub: Hub,
    mut rx: mpsc::Receiver<SyncCommand>,
) {
    let sync_dir = data_dir.join("sync");
    let mut manifest = load_manifest(&data_dir);

    // Ensure Drive folders exist
    emit_status(&app, true, None, None);
    match setup_drive_folders(&hub, &mut manifest).await {
        Ok(_) => {
            save_manifest(&data_dir, &manifest);
            info!("Drive folders ready");
        }
        Err(e) => {
            error!("Failed to setup Drive folders: {}", e);
            emit_status(&app, false, None, Some(&e));
            return;
        }
    }

    // Initial full sync (download remote changes)
    match full_sync(&hub, &sync_dir, &mut manifest, &app).await {
        Ok(_) => {
            manifest.last_full_sync = Some(chrono::Utc::now());
            save_manifest(&data_dir, &manifest);
            emit_status(&app, false, manifest.last_full_sync.as_ref(), None);
        }
        Err(e) => {
            warn!("Initial sync failed: {}", e);
            emit_status(&app, false, None, Some(&e));
        }
    }

    // Main loop: receive commands or poll on interval
    let mut interval = tokio::time::interval(POLL_INTERVAL);
    interval.tick().await; // Skip first immediate tick

    loop {
        tokio::select! {
            cmd = rx.recv() => {
                match cmd {
                    Some(SyncCommand::UploadChanged(paths)) => {
                        emit_status(&app, true, manifest.last_full_sync.as_ref(), None);
                        for rel_path in &paths {
                            if let Err(e) = upload_single_file(
                                &hub, &sync_dir, rel_path, &mut manifest,
                            ).await {
                                warn!("Upload failed for '{}': {}", rel_path, e);
                            }
                        }
                        save_manifest(&data_dir, &manifest);
                        emit_status(&app, false, manifest.last_full_sync.as_ref(), None);
                    }
                    Some(SyncCommand::FullSync) => {
                        emit_status(&app, true, manifest.last_full_sync.as_ref(), None);
                        match full_sync(&hub, &sync_dir, &mut manifest, &app).await {
                            Ok(_) => {
                                manifest.last_full_sync = Some(chrono::Utc::now());
                                save_manifest(&data_dir, &manifest);
                                emit_status(&app, false, manifest.last_full_sync.as_ref(), None);
                            }
                            Err(e) => {
                                warn!("Full sync failed: {}", e);
                                emit_status(&app, false, manifest.last_full_sync.as_ref(), Some(&e));
                            }
                        }
                    }
                    Some(SyncCommand::Shutdown) | None => {
                        info!("Sync engine shutting down");
                        break;
                    }
                }
            }
            _ = interval.tick() => {
                // Periodic poll for remote changes
                emit_status(&app, true, manifest.last_full_sync.as_ref(), None);
                match full_sync(&hub, &sync_dir, &mut manifest, &app).await {
                    Ok(_) => {
                        manifest.last_full_sync = Some(chrono::Utc::now());
                        save_manifest(&data_dir, &manifest);
                    }
                    Err(e) => {
                        warn!("Periodic sync failed: {}", e);
                    }
                }
                emit_status(&app, false, manifest.last_full_sync.as_ref(), None);
            }
        }
    }
}

/// Emit sync status to frontend.
fn emit_status(
    app: &AppHandle,
    syncing: bool,
    last_sync: Option<&chrono::DateTime<chrono::Utc>>,
    error: Option<&str>,
) {
    let status = SyncStatus {
        authenticated: true,
        email: None, // Drive API doesn't easily expose email; omit for now
        syncing,
        last_sync: last_sync.map(|t| t.to_rfc3339()),
        error: error.map(|s| s.to_string()),
    };
    let _ = app.emit("drive-sync-status", &status);
}

/// Setup Drive folder structure: root + subfolders.
async fn setup_drive_folders(hub: &Hub, manifest: &mut SyncManifest) -> Result<(), String> {
    // Ensure root folder
    let root_id = match &manifest.drive_folder_id {
        Some(id) => id.clone(),
        None => {
            let id = files::ensure_sync_folder(hub).await?;
            manifest.drive_folder_id = Some(id.clone());
            id
        }
    };

    // Ensure subfolders for directory targets
    for target in SYNC_TARGETS {
        if target.is_dir && !manifest.subfolder_ids.contains_key(target.relative) {
            let id = files::ensure_subfolder(hub, &root_id, target.relative).await?;
            manifest.subfolder_ids.insert(target.relative.to_string(), id);
        }
    }

    Ok(())
}

/// Upload a single file to Drive.
async fn upload_single_file(
    hub: &Hub,
    sync_dir: &Path,
    relative_path: &str,
    manifest: &mut SyncManifest,
) -> Result<(), String> {
    let local_path = sync_dir.join(relative_path);
    if !local_path.exists() {
        return Ok(());
    }

    let hash = file_md5(&local_path).unwrap_or_default();

    // Check if content actually changed
    if let Some(entry) = manifest.files.get(relative_path) {
        if entry.content_hash == hash {
            return Ok(()); // No change
        }
    }

    // Determine parent folder on Drive
    let parent_id = determine_parent_id(relative_path, manifest)?;
    let file_name = Path::new(relative_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(relative_path);

    let existing_id = manifest.files.get(relative_path).map(|e| e.drive_file_id.as_str());

    let (file_id, remote_modified) =
        files::upload_file(hub, &parent_id, file_name, &local_path, existing_id).await?;

    let local_modified = file_mtime(&local_path).unwrap_or_else(chrono::Utc::now);

    manifest.files.insert(
        relative_path.to_string(),
        SyncFileEntry {
            drive_file_id: file_id,
            remote_modified,
            local_modified,
            content_hash: hash,
        },
    );

    info!("Synced up: {}", relative_path);
    Ok(())
}

/// Determine the Drive parent folder ID for a relative path.
fn determine_parent_id(relative_path: &str, manifest: &SyncManifest) -> Result<String, String> {
    // Check if this path is inside a subfolder target
    for target in SYNC_TARGETS {
        if target.is_dir && relative_path.starts_with(target.relative) {
            return manifest
                .subfolder_ids
                .get(target.relative)
                .cloned()
                .ok_or_else(|| format!("Subfolder ID not found for '{}'", target.relative));
        }
    }

    // Top-level file goes to root folder
    manifest
        .drive_folder_id
        .clone()
        .ok_or_else(|| "Root folder ID not set".to_string())
}

/// Full sync: for each target, compare local and remote, download/upload as needed.
async fn full_sync(
    hub: &Hub,
    sync_dir: &Path,
    manifest: &mut SyncManifest,
    app: &AppHandle,
) -> Result<(), String> {
    let root_id = manifest
        .drive_folder_id
        .as_ref()
        .ok_or("Root folder ID not set")?
        .clone();

    for target in SYNC_TARGETS {
        info!("Syncing target: {} (dir={})", target.relative, target.is_dir);
        let changed = if target.is_dir {
            let folder_id = manifest
                .subfolder_ids
                .get(target.relative)
                .ok_or_else(|| format!("Subfolder ID not set for '{}'", target.relative))?
                .clone();

            sync_directory(hub, sync_dir, target.relative, &folder_id, manifest).await?
        } else {
            sync_single_file(hub, sync_dir, target.relative, &root_id, manifest).await?
        };
        info!("Done syncing target: {}", target.relative);

        // Reload in-memory state immediately per target so UI updates
        // even if later targets are slow (e.g. raw_api with many files).
        if changed {
            reload_game_state(app).await;
            let _ = app.emit("drive-data-updated", ());
        }
    }

    Ok(())
}

/// Sync a single file target. Returns true if local data was updated from remote.
async fn sync_single_file(
    hub: &Hub,
    sync_dir: &Path,
    relative: &str,
    parent_id: &str,
    manifest: &mut SyncManifest,
) -> Result<bool, String> {
    let local_path = sync_dir.join(relative);
    let file_name = Path::new(relative)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(relative);

    let local_exists = local_path.exists();
    let local_hash = if local_exists { file_md5(&local_path) } else { None };
    let local_mtime = if local_exists { file_mtime(&local_path) } else { None };

    // Check manifest entry
    let manifest_entry = manifest.files.get(relative).cloned();

    // Check remote
    let remote_files = files::list_files(hub, parent_id).await?;
    let remote_file = remote_files.iter().find(|f| f.name == file_name);

    match (local_exists, remote_file) {
        (true, Some(remote)) => {
            let local_changed = match &manifest_entry {
                Some(entry) => local_hash.as_deref() != Some(&entry.content_hash),
                None => true,
            };
            let remote_changed = match &manifest_entry {
                Some(entry) => remote.modified_time > entry.remote_modified,
                None => true,
            };

            if local_changed && remote_changed {
                // Conflict: timestamp wins
                let local_t = local_mtime.unwrap_or_else(chrono::Utc::now);
                if local_t > remote.modified_time {
                    // Local wins
                    let (file_id, remote_modified) = files::upload_file(
                        hub, parent_id, file_name, &local_path, Some(&remote.id),
                    ).await?;
                    update_manifest_entry(manifest, relative, &file_id, remote_modified, &local_path);
                    info!("Conflict resolved (local wins): {}", relative);
                    Ok(false)
                } else {
                    // Remote wins
                    files::download_file(hub, &remote.id, &local_path).await?;
                    update_manifest_entry(manifest, relative, &remote.id, remote.modified_time, &local_path);
                    info!("Conflict resolved (remote wins): {}", relative);
                    Ok(true)
                }
            } else if local_changed {
                // Upload local
                let (file_id, remote_modified) = files::upload_file(
                    hub, parent_id, file_name, &local_path, Some(&remote.id),
                ).await?;
                update_manifest_entry(manifest, relative, &file_id, remote_modified, &local_path);
                Ok(false)
            } else if remote_changed {
                // Download remote
                files::download_file(hub, &remote.id, &local_path).await?;
                update_manifest_entry(manifest, relative, &remote.id, remote.modified_time, &local_path);
                info!("Downloaded: {}", relative);
                Ok(true)
            } else {
                Ok(false) // No change
            }
        }
        (true, None) => {
            // Local only → upload
            let (file_id, remote_modified) = files::upload_file(
                hub, parent_id, file_name, &local_path, None,
            ).await?;
            update_manifest_entry(manifest, relative, &file_id, remote_modified, &local_path);
            Ok(false)
        }
        (false, Some(remote)) => {
            // Remote only → download
            files::download_file(hub, &remote.id, &local_path).await?;
            update_manifest_entry(manifest, relative, &remote.id, remote.modified_time, &local_path);
            info!("Downloaded new: {}", relative);
            Ok(true)
        }
        (false, None) => Ok(false), // Neither exists
    }
}

/// Sync a directory target. Returns true if any local files were updated.
async fn sync_directory(
    hub: &Hub,
    sync_dir: &Path,
    dir_relative: &str,
    folder_id: &str,
    manifest: &mut SyncManifest,
) -> Result<bool, String> {
    let local_dir = sync_dir.join(dir_relative);
    let _ = std::fs::create_dir_all(&local_dir);

    // Get local files
    let local_files: Vec<String> = std::fs::read_dir(&local_dir)
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
                .filter_map(|e| e.file_name().to_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    // Get remote files
    let remote_files = files::list_files(hub, folder_id).await?;

    let mut any_updated = false;

    // Build lookup maps
    let remote_by_name: std::collections::HashMap<&str, &files::RemoteFile> = remote_files
        .iter()
        .map(|f| (f.name.as_str(), f))
        .collect();

    // Process local files
    for name in &local_files {
        let relative = format!("{}/{}", dir_relative, name);
        let local_path = local_dir.join(name);

        if let Some(remote) = remote_by_name.get(name.as_str()) {
            // Both exist — check for changes
            let local_hash = file_md5(&local_path);
            let manifest_entry = manifest.files.get(&relative).cloned();

            let local_changed = match &manifest_entry {
                Some(entry) => local_hash.as_deref() != Some(&entry.content_hash),
                None => true,
            };
            let remote_changed = match &manifest_entry {
                Some(entry) => remote.modified_time > entry.remote_modified,
                None => true,
            };

            if local_changed && remote_changed {
                // Conflict: timestamp wins
                let local_t = file_mtime(&local_path).unwrap_or_else(chrono::Utc::now);
                if local_t > remote.modified_time {
                    let (fid, rm) = files::upload_file(
                        hub, folder_id, name, &local_path, Some(&remote.id),
                    ).await?;
                    update_manifest_entry(manifest, &relative, &fid, rm, &local_path);
                } else {
                    files::download_file(hub, &remote.id, &local_path).await?;
                    update_manifest_entry(manifest, &relative, &remote.id, remote.modified_time, &local_path);
                    any_updated = true;
                }
            } else if local_changed {
                let (fid, rm) = files::upload_file(
                    hub, folder_id, name, &local_path, Some(&remote.id),
                ).await?;
                update_manifest_entry(manifest, &relative, &fid, rm, &local_path);
            } else if remote_changed {
                files::download_file(hub, &remote.id, &local_path).await?;
                update_manifest_entry(manifest, &relative, &remote.id, remote.modified_time, &local_path);
                any_updated = true;
            }
        } else {
            // Local only → upload
            let (fid, rm) = files::upload_file(
                hub, folder_id, name, &local_path, None,
            ).await?;
            update_manifest_entry(manifest, &relative, &fid, rm, &local_path);
        }
    }

    // Remote files not in local → download
    let local_set: std::collections::HashSet<&str> =
        local_files.iter().map(|s| s.as_str()).collect();
    for remote in &remote_files {
        if !local_set.contains(remote.name.as_str()) {
            let relative = format!("{}/{}", dir_relative, remote.name);
            let local_path = local_dir.join(&remote.name);
            files::download_file(hub, &remote.id, &local_path).await?;
            update_manifest_entry(manifest, &relative, &remote.id, remote.modified_time, &local_path);
            info!("Downloaded new dir file: {}", relative);
            any_updated = true;
        }
    }

    Ok(any_updated)
}

/// Reload GameState in-memory data from disk after sync downloads.
/// This ensures that quest progress, improved equipment, and battle logs
/// from other devices are reflected in the running application.
async fn reload_game_state(app: &AppHandle) {
    let game_state = app.state::<crate::api::models::GameState>();
    let mut state = game_state.inner.write().await;

    // Reload quest progress
    let qp_path = state.quest_progress_path.clone();
    state.history.quest_progress = crate::quest_progress::load_progress(&qp_path);
    info!("Sync reload: quest progress");

    // Reload improved equipment history
    let ie_path = state.improved_equipment_path.clone();
    state.history.improved_equipment = crate::improvement::load_improved_history(&ie_path);
    info!("Sync reload: improved equipment ({} items)", state.history.improved_equipment.len());

    // Reload battle logs from disk
    state.sortie.battle_logger.reload_from_disk();
}

/// Update a manifest entry after a sync operation.
fn update_manifest_entry(
    manifest: &mut SyncManifest,
    relative: &str,
    drive_file_id: &str,
    remote_modified: chrono::DateTime<chrono::Utc>,
    local_path: &Path,
) {
    let local_modified = file_mtime(local_path).unwrap_or_else(chrono::Utc::now);
    let content_hash = file_md5(local_path).unwrap_or_default();
    manifest.files.insert(
        relative.to_string(),
        SyncFileEntry {
            drive_file_id: drive_file_id.to_string(),
            remote_modified,
            local_modified,
            content_hash,
        },
    );
}
