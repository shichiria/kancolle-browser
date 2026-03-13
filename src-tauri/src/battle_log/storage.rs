use chrono::{DateTime, Local};
use log::{error, info, warn};
use std::fs;
use std::path::PathBuf;

use super::{BattleLogger, SortieRecord, SortieRecordSummary};

impl BattleLogger {
    /// Save a sortie record to disk as JSON
    pub(super) fn save_to_disk(&self, record: &SortieRecord) {
        let dir = match &self.save_dir {
            Some(d) => d,
            None => return,
        };
        if let Err(e) = fs::create_dir_all(dir) {
            error!("Failed to create battle log dir: {}", e);
            return;
        }

        let filename = format!("{}.json", record.id);
        let path = dir.join(&filename);
        match serde_json::to_string_pretty(record) {
            Ok(json) => {
                if let Err(e) = fs::write(&path, json) {
                    error!("Failed to write battle log {}: {}", filename, e);
                } else {
                    info!("Battle log saved: {}", filename);
                }
            }
            Err(e) => {
                error!("Failed to serialize battle log: {}", e);
            }
        }
    }

    /// Load records from disk filtered by date range (file name based).
    /// date_from/date_to are in YYYYMMDD format. Both inclusive.
    pub fn get_records_by_date_range(
        &self,
        date_from: &str,
        date_to: &str,
    ) -> Vec<SortieRecordSummary> {
        let dir = match &self.save_dir {
            Some(d) => d,
            None => return Vec::new(),
        };
        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return Vec::new(),
        };

        // Collect and filter filenames first, then sort before reading
        let mut matching_paths: Vec<_> = entries
            .flatten()
            .filter_map(|entry| {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("json") {
                    return None;
                }
                let stem = path.file_stem()?.to_str()?.to_string();
                if stem.len() < 8 {
                    return None;
                }
                let file_date = &stem[..8];
                if file_date >= date_from && file_date <= date_to {
                    Some((stem, path))
                } else {
                    None
                }
            })
            .collect();

        // Sort by filename descending (newest first) - avoids parsing JSON for ordering
        matching_paths.sort_by(|a, b| b.0.cmp(&a.0));

        let mut records = Vec::new();
        for (_stem, path) in matching_paths {
            match fs::read_to_string(&path) {
                Ok(content) => match serde_json::from_str::<SortieRecord>(&content) {
                    Ok(mut record) => {
                        for node in &mut record.nodes {
                            node.migrate_legacy();
                        }
                        records.push(SortieRecordSummary::from(&record));
                    }
                    Err(e) => {
                        warn!("Failed to parse battle log {:?}: {}", path.file_name(), e);
                    }
                },
                Err(e) => {
                    warn!("Failed to read battle log {:?}: {}", path.file_name(), e);
                }
            }
        }

        records
    }

    /// Reload completed records from disk (used after sync downloads new files).
    pub fn reload_from_disk(&mut self) {
        if let Some(dir) = &self.save_dir {
            self.completed = Self::load_from_disk(dir);
            info!("BattleLogger reloaded: {} records", self.completed.len());
            self.fix_interrupted_records();
        }
    }

    /// Fix sortie records that were interrupted (crash, browser close, etc.)
    /// Records with end_time = None on disk are no longer active, so mark them completed.
    pub(super) fn fix_interrupted_records(&mut self) {
        let active_id = self.active_sortie.as_ref().map(|s| s.id.clone());
        let mut fixed_indices = Vec::new();

        for (i, record) in self.completed.iter_mut().enumerate() {
            if record.end_time.is_none() && active_id.as_deref() != Some(&record.id) {
                // Use the file's last modified time if available, otherwise use start_time
                let end_time = self.save_dir.as_ref()
                    .map(|dir| dir.join(format!("{}.json", record.id)))
                    .and_then(|path| path.metadata().ok())
                    .and_then(|meta| meta.modified().ok())
                    .map(|sys_time| DateTime::<Local>::from(sys_time))
                    .unwrap_or(record.start_time);
                record.end_time = Some(end_time);
                fixed_indices.push(i);
            }
        }

        if !fixed_indices.is_empty() {
            for &i in &fixed_indices {
                self.save_to_disk(&self.completed[i]);
            }
            info!("Fixed {} interrupted sortie records", fixed_indices.len());
        }
    }

    pub(super) fn load_from_disk(dir: &PathBuf) -> Vec<SortieRecord> {
        let mut records = Vec::new();
        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return records, // Directory doesn't exist yet
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            match fs::read_to_string(&path) {
                Ok(content) => match serde_json::from_str::<SortieRecord>(&content) {
                    Ok(mut record) => {
                        // Migrate legacy BattleNode format to new BattleDetail format
                        for node in &mut record.nodes {
                            node.migrate_legacy();
                        }
                        records.push(record);
                    }
                    Err(e) => {
                        warn!("Failed to parse battle log {:?}: {}", path.file_name(), e);
                    }
                },
                Err(e) => {
                    warn!("Failed to read battle log {:?}: {}", path.file_name(), e);
                }
            }
        }

        // Sort by start_time descending (newest first)
        records.sort_by(|a, b| b.start_time.cmp(&a.start_time));

        // Keep at most 200
        records.truncate(200);

        records
    }
}
