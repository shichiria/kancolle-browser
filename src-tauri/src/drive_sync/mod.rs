pub mod auth;
pub mod engine;
pub mod files;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Defines a target for syncing between local sync/ directory and Google Drive.
pub struct SyncTarget {
    /// Relative path within sync/ directory
    pub relative: &'static str,
    /// Whether this is a directory (containing multiple files)
    pub is_dir: bool,
}

/// All sync targets — add a line here to sync a new file/directory.
pub const SYNC_TARGETS: &[SyncTarget] = &[
    SyncTarget {
        relative: "quest_progress.json",
        is_dir: false,
    },
    SyncTarget {
        relative: "improved_equipment.json",
        is_dir: false,
    },
    SyncTarget {
        relative: "battle_logs",
        is_dir: true,
    },
    SyncTarget {
        relative: "raw_api",
        is_dir: true,
    },
    SyncTarget {
        relative: "senka_log.json",
        is_dir: false,
    },
    SyncTarget {
        relative: "formation_memory.json",
        is_dir: false,
    },
];

/// Sync manifest persisted to disk to track Drive file metadata.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SyncManifest {
    /// Relative path -> file entry mapping
    pub files: HashMap<String, SyncFileEntry>,
    /// Google Drive folder ID for "KanColle Browser Sync"
    pub drive_folder_id: Option<String>,
    /// Subfolder IDs: "battle_logs" -> folder ID, "raw_api" -> folder ID
    pub subfolder_ids: HashMap<String, String>,
    /// Last time a full sync was completed
    pub last_full_sync: Option<DateTime<Utc>>,
}

/// Metadata about a single synced file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncFileEntry {
    /// Google Drive file ID
    pub drive_file_id: String,
    /// Last modified time on Drive
    pub remote_modified: DateTime<Utc>,
    /// Last modified time locally when we last synced
    pub local_modified: DateTime<Utc>,
    /// MD5 hash of file content at last sync
    pub content_hash: String,
}

/// Commands sent to the sync engine background task.
#[derive(Debug)]
pub enum SyncCommand {
    /// Upload specific changed files (relative paths within sync/)
    UploadChanged(Vec<String>),
    /// Perform a full sync (download + upload)
    FullSync,
    /// Shut down the sync engine
    Shutdown,
}

/// Status of the sync engine, emitted to frontend.
#[derive(Debug, Clone, Serialize)]
pub struct SyncStatus {
    /// Whether authenticated with Google
    pub authenticated: bool,
    /// User email (if authenticated)
    pub email: Option<String>,
    /// Whether currently syncing
    pub syncing: bool,
    /// Last sync time
    pub last_sync: Option<String>,
    /// Error message (if any)
    pub error: Option<String>,
}

/// Load sync manifest from disk.
pub fn load_manifest(data_dir: &std::path::Path) -> SyncManifest {
    let path = data_dir.join("sync_manifest.json");
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => SyncManifest::default(),
    }
}

/// Save sync manifest to disk.
pub fn save_manifest(data_dir: &std::path::Path, manifest: &SyncManifest) {
    let path = data_dir.join("sync_manifest.json");
    if let Ok(json) = serde_json::to_string_pretty(manifest) {
        let _ = std::fs::write(&path, json);
    }
}
