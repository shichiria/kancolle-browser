mod air_corps;
mod battle;
pub(crate) mod battle_info;
mod dispatch;
pub mod dto;
mod fleet;
pub(crate) mod formation;
pub(crate) mod minimap;
pub mod models;
mod parse;
mod port;
mod quest;
pub(crate) mod screen;
mod ship;

#[cfg(test)]
mod tests;
use tauri::{AppHandle, Manager};

use models::GameState;

// Re-export public functions used by other crates
pub use formation::hide_formation_hint;
pub use minimap::send_minimap_data;

#[cfg(test)]
use parse::extract_senka_from_clearitemget;
#[cfg(test)]
use port::get_material;

/// Send sync notification for changed files.
pub(super) fn notify_sync(state: &models::GameStateInner, paths: Vec<&str>) {
    if let Some(tx) = &state.sync_notifier {
        let _ = tx.try_send(crate::drive_sync::SyncCommand::UploadChanged(
            paths.into_iter().map(|s| s.to_string()).collect(),
        ));
    }
}

/// Process intercepted KanColle API data.
/// All state updates happen in a SINGLE async task to guarantee ordering.
pub fn process_api(app_handle: &AppHandle, endpoint: &str, json_str: &str, request_body: &str) {
    // Raw endpoint traces are high-volume; semantic Command/Event/State action
    // logs remain available in release builds.
    #[cfg(debug_assertions)]
    crate::action_log::log("API", endpoint, &format!("body_len={}", json_str.len()));

    // Update the tracked screen state from this API endpoint, if known.
    screen::update_from_api(app_handle, endpoint);

    let game_state = app_handle.state::<GameState>();

    // Parse data on the calling thread (sync) to avoid cloning large json_str
    let parsed = parse::parse(endpoint, json_str, request_body);

    // Single async task: raw save + state update (guarantees ordering)
    let inner = game_state.inner.clone();
    let endpoint = endpoint.to_string();
    let request_body = request_body.to_string();
    let json_str = json_str.to_string();
    let app = app_handle.clone();

    tauri::async_runtime::spawn(async move {
        // Step 1: Briefly lock to allocate filename + seq number (no I/O)
        let raw_info = {
            let mut state = inner.write().await;
            state
                .sortie
                .battle_logger
                .allocate_raw_api_filename(&endpoint)
        };

        // Step 2: Write raw API dump to disk OUTSIDE the lock
        let raw_filename = if let Some((dir, filename)) = raw_info {
            if crate::battle_log::save_raw_api_to_disk(
                &dir,
                &filename,
                &endpoint,
                &request_body,
                &json_str,
            ) {
                Some(filename)
            } else {
                None
            }
        } else {
            None
        };

        // Step 3: Re-acquire lock for state updates
        let mut state = inner.write().await;

        // Notify sync engine about new raw API file
        if let (Some(filename), Some(tx)) = (&raw_filename, &state.sync_notifier) {
            let path = format!("raw_api/{}", filename);
            let _ = tx.try_send(crate::drive_sync::SyncCommand::UploadChanged(vec![path]));
        }

        dispatch::apply(&mut state, parsed, &endpoint, &request_body, &app);
    });
}
