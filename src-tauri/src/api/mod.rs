pub mod dto;
pub mod models;
use dto::request::{HenseiChangeReq, QuestReq, RemodelSlotReq};

#[cfg(test)]
mod tests;
use log::{error, info, warn};
use tauri::{AppHandle, Emitter, Manager};

use models::{GameState, ShipInfo};

/// Send sync notification for changed files.
fn notify_sync(state: &models::GameStateInner, paths: Vec<&str>) {
    if let Some(tx) = &state.sync_notifier {
        let _ = tx.try_send(crate::drive_sync::SyncCommand::UploadChanged(
            paths.into_iter().map(|s| s.to_string()).collect(),
        ));
    }
}

/// Get Japanese name for a formation ID
fn formation_name(id: i32) -> &'static str {
    match id {
        1 => "単縦陣",
        2 => "複縦陣",
        3 => "輪形陣",
        4 => "梯形陣",
        5 => "単横陣",
        6 => "警戒陣",
        11 => "第一警戒航行序列(対潜警戒)",
        12 => "第二警戒航行序列(前方警戒)",
        13 => "第三警戒航行序列(輪形陣)",
        14 => "第四警戒航行序列(戦闘隊形)",
        _ => "不明",
    }
}

/// Formation button label rect in game canvas (1200x720) coordinates: (x, y, w, h)
/// Positions derived from sprite atlas (sally_jin.json: label=150x48) and kcauto reference data.
/// Grid layout: 3 columns (x=668,866,1064) × 2 rows (y=278,517), same for all ship counts.
fn get_formation_button_rect(formation: i32, _ship_count: usize) -> Option<(f64, f64, f64, f64)> {
    // Label sprite is 150x48 in sally_jin atlas; yellow border is slightly wider
    const BW: f64 = 154.0;
    const BH: f64 = 48.0;

    // Button center positions in game canvas coordinates
    // Column X: derived from kcauto search regions + fine-tuned (left+7, right-3 → cx-5)
    // Row Y: derived from kcauto search regions (row1: 278, row2: 517)
    let (cx, cy) = match formation {
        1 => (663.0, 278.0),   // 単縦陣 col1 row1
        2 => (858.0, 278.0),   // 複縦陣 col2 row1
        3 => (1056.0, 278.0),  // 輪形陣 col3 row1
        4 => (766.0, 517.0),   // 梯形陣 col1 row2
        5 => (960.0, 517.0),   // 単横陣 col2 row2
        6 => (1048.0, 517.0),  // 警戒陣 col3 row2
        // Combined fleet formations (from kcauto CF regions, same cx-5 adjustment)
        11 => (743.0, 263.0),  // 第一警戒航行序列
        12 => (993.0, 263.0),  // 第二警戒航行序列
        13 => (743.0, 468.0),  // 第三警戒航行序列
        14 => (993.0, 468.0),  // 第四警戒航行序列
        _ => return None,
    };

    Some((cx - BW / 2.0, cy - BH / 2.0, BW, BH))
}

/// Show formation highlight using the click-through formation-hint window
fn show_formation_hint(app: &AppHandle, formation: i32, ship_count: usize) {
    // Check if formation hint is enabled
    if let Some(state) = app.try_state::<crate::AppState>() {
        if !state.formation_hint_enabled.load(std::sync::atomic::Ordering::Relaxed) {
            return;
        }
    }

    let game_win = match app.get_window("game") {
        Some(w) => w,
        None => return,
    };
    let hint_win = match app.get_window("formation-hint") {
        Some(w) => w,
        None => return,
    };

    let (bx, by, bw, bh) = match get_formation_button_rect(formation, ship_count) {
        Some(r) => r,
        None => return,
    };

    let inner_pos = match game_win.inner_position() {
        Ok(p) => p,
        Err(_) => return,
    };
    let scale = game_win.scale_factor().unwrap_or(1.0);

    // Get current game zoom level
    let zoom = app.try_state::<crate::AppState>()
        .map(|s| *s.game_zoom.lock().unwrap())
        .unwrap_or(1.0);

    // Control bar is 28 CSS pixels, scaled by zoom and DPI
    // Game coordinates are also scaled by zoom
    let mut dx = (bx * zoom * scale) as i32;
    let mut dy = ((28.0 + by) * zoom * scale) as i32;

    // macOS: adjust for platform-specific coordinate offset
    #[cfg(target_os = "macos")]
    {
        dx += (6.0 * scale) as i32;
        dy += (30.0 * scale) as i32;
    }
    let phys_w = (bw * zoom * scale) as u32;
    let phys_h = (bh * zoom * scale) as u32;

    // Save offset in AppState for window-move tracking
    if let Some(app_state) = app.try_state::<crate::AppState>() {
        let mut rect = app_state.formation_hint_rect.lock().unwrap();
        rect.dx = dx;
        rect.dy = dy;
        rect.w = phys_w;
        rect.h = phys_h;
        rect.visible = true;
    }

    let screen_x = inner_pos.x + dx;
    let screen_y = inner_pos.y + dy;

    // Also check outer_position and game webview position for debugging
    let outer_pos = game_win.outer_position().ok();
    let win_size = game_win.inner_size().ok();
    log::info!(
        "FormationHint: formation={}, ship_count={}, scale={}, inner_pos=({},{}), outer_pos={:?}, win_size={:?}, dx={}, dy={}, screen=({},{}), rect={}x{}",
        formation, ship_count, scale, inner_pos.x, inner_pos.y, outer_pos, win_size, dx, dy, screen_x, screen_y, phys_w, phys_h
    );

    let _ = hint_win.set_size(tauri::PhysicalSize::new(phys_w, phys_h));
    if let Some(wv) = app.get_webview("formation-hint-content") {
        let _ = wv.set_size(tauri::PhysicalSize::new(phys_w, phys_h));
    }
    let _ = hint_win.set_position(tauri::PhysicalPosition::new(screen_x, screen_y));
    let _ = hint_win.show();
}

/// Hide formation hint window
pub fn hide_formation_hint(app: &AppHandle) {
    if let Some(app_state) = app.try_state::<crate::AppState>() {
        app_state.formation_hint_rect.lock().unwrap().visible = false;
    }
    if let Some(hint_win) = app.get_window("formation-hint") {
        let _ = hint_win.hide();
    }
}

/// Helper to get a material value by api_id from the material array
fn get_material(materials: &[models::Material], id: i32) -> i32 {
    materials
        .iter()
        .find(|m| m.api_id == id)
        .map(|m| m.api_value)
        .unwrap_or(0)
}

/// Extract stat value from api_karyoku / api_taiku / etc.
/// These are arrays where index 0 is the equipped total value.
fn extract_stat_value(val: &serde_json::Value) -> i32 {
    if let Some(arr) = val.as_array() {
        arr.first().and_then(|v| v.as_i64()).unwrap_or(0) as i32
    } else {
        val.as_i64().unwrap_or(0) as i32
    }
}

/// Extract slot IDs from api_slot value (array of equipment instance IDs, -1 = empty)
fn extract_slot_ids(val: &serde_json::Value) -> Vec<i32> {
    if let Some(arr) = val.as_array() {
        arr.iter()
            .map(|v| v.as_i64().unwrap_or(-1) as i32)
            .collect()
    } else {
        Vec::new()
    }
}

/// Pre-parsed API data to pass into the single async task
enum ParsedApi {
    Start2(Box<models::ApiStart2>),
    Port(Box<models::ApiPort>),
    SlotItem(Vec<models::PlayerSlotItemApi>),
    QuestList(crate::api::dto::battle::ApiQuestListResponse),
    Battle(serde_json::Value),
    ExerciseResult(serde_json::Value),
    HenseiChange {
        fleet_id: usize,
        ship_idx: i32,
        ship_id: i32,
    },
    HenseiPresetSelect(crate::api::dto::battle::ApiHenseiPresetSelectResponse),
    RemodelSlot {
        slot_id: i32,
        success: bool,
        eq_id: i32,
    },
    QuestStart {
        quest_id: i32,
    },
    QuestStop {
        quest_id: i32,
    },
    QuestClear {
        quest_id: i32,
        senka_bonus: i64,
    },
    Ship3(serde_json::Value),
    SlotDeprive(serde_json::Value),
    Ranking(String), // raw JSON string for ranking decryption (needs admiral name from state)
    Other,
}

/// Process intercepted KanColle API data.
/// All state updates happen in a SINGLE async task to guarantee ordering.
pub fn process_api(app_handle: &AppHandle, endpoint: &str, json_str: &str, request_body: &str) {
    let game_state = app_handle.state::<GameState>();

    // Parse data on the calling thread (sync) to avoid cloning large json_str
    let parsed = match endpoint {
        "/kcsapi/api_start2/getData" => {
            info!("Processing api_start2/getData (master data)");
            match serde_json::from_str::<models::ApiResponse<models::ApiStart2>>(json_str) {
                Ok(data) => match data.api_data {
                    Some(api_data) => ParsedApi::Start2(Box::new(api_data)),
                    None => ParsedApi::Other,
                },
                Err(e) => {
                    error!("Failed to parse api_start2: {}", e);
                    ParsedApi::Other
                }
            }
        }
        "/kcsapi/api_port/port" => {
            info!("Processing api_port/port (home screen)");
            match serde_json::from_str::<models::ApiResponse<models::ApiPort>>(json_str) {
                Ok(data) => match data.api_data {
                    Some(api_data) => ParsedApi::Port(Box::new(api_data)),
                    None => ParsedApi::Other,
                },
                Err(e) => {
                    error!("Failed to parse api_port: {}", e);
                    ParsedApi::Other
                }
            }
        }
        "/kcsapi/api_get_member/slot_item" => {
            info!("Processing api_get_member/slot_item (player equipment)");
            match serde_json::from_str::<models::ApiResponse<Vec<models::PlayerSlotItemApi>>>(
                json_str,
            ) {
                Ok(data) => match data.api_data {
                    Some(items) => ParsedApi::SlotItem(items),
                    None => ParsedApi::Other,
                },
                Err(e) => {
                    error!("Failed to parse slot_item: {}", e);
                    ParsedApi::Other
                }
            }
        }
        "/kcsapi/api_get_member/require_info" => {
            info!("Processing api_get_member/require_info (includes slot_item)");
            // require_info contains api_slot_item in the same format as api_get_member/slot_item
            match serde_json::from_str::<models::ApiResponse<serde_json::Value>>(json_str) {
                Ok(data) => {
                    if let Some(api_data) = data.api_data {
                        if let Some(items_val) = api_data.get("api_slot_item") {
                            match serde_json::from_value::<Vec<models::PlayerSlotItemApi>>(
                                items_val.clone(),
                            ) {
                                Ok(items) => ParsedApi::SlotItem(items),
                                Err(e) => {
                                    error!("Failed to parse require_info slot_item: {}", e);
                                    ParsedApi::Other
                                }
                            }
                        } else {
                            ParsedApi::Other
                        }
                    } else {
                        ParsedApi::Other
                    }
                }
                Err(e) => {
                    error!("Failed to parse require_info: {}", e);
                    ParsedApi::Other
                }
            }
        }
        "/kcsapi/api_get_member/questlist" => {
            info!("Processing api_get_member/questlist");
            match serde_json::from_str::<
                models::ApiResponse<crate::api::dto::battle::ApiQuestListResponse>,
            >(json_str)
            {
                Ok(data) => match data.api_data {
                    Some(api_data) => ParsedApi::QuestList(api_data),
                    None => ParsedApi::Other,
                },
                Err(e) => {
                    error!("Failed to parse questlist: {}", e);
                    ParsedApi::Other
                }
            }
        }
        "/kcsapi/api_req_hensei/change" => {
            info!("Processing api_req_hensei/change (fleet composition change)");
            match serde_urlencoded::from_str::<HenseiChangeReq>(request_body) {
                Ok(req) => ParsedApi::HenseiChange {
                    fleet_id: req.api_id,
                    ship_idx: req.api_ship_idx,
                    ship_id: req.api_ship_id,
                },
                Err(e) => {
                    error!("Failed to parse hensei/change req: {}", e);
                    ParsedApi::Other
                }
            }
        }
        "/kcsapi/api_req_hensei/preset_select" => {
            info!("Processing api_req_hensei/preset_select (preset fleet load)");
            match serde_json::from_str::<
                models::ApiResponse<crate::api::dto::battle::ApiHenseiPresetSelectResponse>,
            >(json_str)
            {
                Ok(data) => match data.api_data {
                    Some(api_data) => ParsedApi::HenseiPresetSelect(api_data),
                    None => ParsedApi::Other,
                },
                Err(e) => {
                    error!("Failed to parse preset_select: {}", e);
                    ParsedApi::Other
                }
            }
        }
        "/kcsapi/api_req_kousyou/remodel_slot" => {
            info!("Processing api_req_kousyou/remodel_slot (equipment improvement)");
            let req = serde_urlencoded::from_str::<RemodelSlotReq>(request_body).ok();
            let slot_id = req.as_ref().map(|r| r.api_slot_id).unwrap_or(-1);
            let req_eq_id = req.as_ref().map(|r| r.api_id).unwrap_or(-1);

            // Extract eq_id + success from response
            let (success, resp_eq_id) = match serde_json::from_str::<
                models::ApiResponse<crate::api::dto::battle::ApiRemodelSlotResponse>,
            >(json_str)
            {
                Ok(data) => {
                    let api_data = &data.api_data;
                    let flag = api_data.as_ref().and_then(|d| d.api_remodel_flag);
                    // Get master eq_id from api_after_slot.api_slotitem_id in response
                    let mut eq_id = api_data
                        .as_ref()
                        .and_then(|d| d.api_after_slot.as_ref())
                        .and_then(|s| s.api_slotitem_id)
                        .unwrap_or(-1) as i32;

                    if eq_id <= 0 {
                        eq_id = req_eq_id; // Fallback to request body's api_id
                    }
                    info!(
                        "remodel_slot: slot_id={}, resp_eq_id={}, flag={:?}",
                        slot_id, eq_id, flag
                    );
                    (flag.map(|f| f == 1).unwrap_or(false), eq_id)
                }
                Err(e) => {
                    error!("Failed to parse remodel_slot response: {}", e);
                    (false, -1)
                }
            };
            ParsedApi::RemodelSlot {
                slot_id,
                success,
                eq_id: resp_eq_id,
            }
        }
        "/kcsapi/api_req_quest/start" => {
            info!("Processing {} (quest started)", endpoint);
            let req = serde_urlencoded::from_str::<QuestReq>(request_body).ok();
            let quest_id = req.map(|r| r.api_quest_id).unwrap_or(0);
            ParsedApi::QuestStart { quest_id }
        }
        "/kcsapi/api_req_quest/stop" => {
            info!("Processing {} (quest cancelled)", endpoint);
            let req = serde_urlencoded::from_str::<QuestReq>(request_body).ok();
            let quest_id = req.map(|r| r.api_quest_id).unwrap_or(0);
            ParsedApi::QuestStop { quest_id }
        }
        "/kcsapi/api_req_quest/clearitemget" => {
            info!("Processing {} (quest completed)", endpoint);
            let req = serde_urlencoded::from_str::<QuestReq>(request_body).ok();
            let quest_id = req.map(|r| r.api_quest_id).unwrap_or(0);
            // Parse response to extract senka bonus from api_bounus
            let senka_bonus = extract_senka_from_clearitemget(json_str);
            ParsedApi::QuestClear {
                quest_id,
                senka_bonus,
            }
        }
        "/kcsapi/api_req_practice/battle_result" => {
            info!("Processing api_req_practice/battle_result (exercise result)");
            match serde_json::from_str::<serde_json::Value>(json_str) {
                Ok(v) => ParsedApi::ExerciseResult(v),
                Err(e) => {
                    error!("Failed to parse exercise battle_result: {}", e);
                    ParsedApi::Other
                }
            }
        }
        "/kcsapi/api_get_member/ship3" => {
            info!("Processing api_get_member/ship3 (ship data after equipment change)");
            match serde_json::from_str::<models::ApiResponse<serde_json::Value>>(json_str) {
                Ok(data) => match data.api_data {
                    Some(api_data) => ParsedApi::Ship3(api_data),
                    None => ParsedApi::Other,
                },
                Err(e) => {
                    error!("Failed to parse ship3: {}", e);
                    ParsedApi::Other
                }
            }
        }
        "/kcsapi/api_req_kaisou/slot_deprive" => {
            info!("Processing api_req_kaisou/slot_deprive (equipment transfer between ships)");
            match serde_json::from_str::<models::ApiResponse<serde_json::Value>>(json_str) {
                Ok(data) => match data.api_data {
                    Some(api_data) => ParsedApi::SlotDeprive(api_data),
                    None => ParsedApi::Other,
                },
                Err(e) => {
                    error!("Failed to parse slot_deprive: {}", e);
                    ParsedApi::Other
                }
            }
        }
        "/kcsapi/api_req_ranking/mxltvkpyuklh" => {
            info!("Processing api_req_ranking/mxltvkpyuklh (ranking data)");
            ParsedApi::Ranking(json_str.to_string())
        }
        ep if is_battle_endpoint(ep) => match serde_json::from_str::<serde_json::Value>(json_str) {
            Ok(v) => ParsedApi::Battle(v),
            Err(e) => {
                let preview: String = json_str.chars().take(200).collect();
                error!(
                    "Failed to parse battle API JSON for {}: {} (len={}, first 200: {:?})",
                    ep,
                    e,
                    json_str.len(),
                    preview
                );
                ParsedApi::Other
            }
        },
        _ => {
            info!("Unhandled API endpoint: {}", endpoint);
            ParsedApi::Other
        }
    };

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
            state.sortie.battle_logger.allocate_raw_api_filename(&endpoint)
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

        match parsed {
            ParsedApi::Start2(api_data) => {
                process_start2(&mut state, &api_data, &app);
            }
            ParsedApi::Port(api_data) => {
                process_port(&mut state, &api_data, &app);
            }
            ParsedApi::SlotItem(items) => {
                let count = items.len();
                state.profile.slotitems.clear();
                for item in &items {
                    state.profile.slotitems.insert(
                        item.api_id,
                        models::PlayerSlotItem {
                            item_id: item.api_id,
                            slotitem_id: item.api_slotitem_id,
                            level: item.api_level,
                            alv: item.api_alv,
                            locked: item.api_locked == 1,
                        },
                    );
                }
                info!("GameState updated: {} player slot items", count);
            }
            ParsedApi::QuestList(json) => {
                process_questlist(&mut state, &json, &app);
            }
            ParsedApi::Battle(json) => {
                process_battle(&mut state, &endpoint, &request_body, &json, &app);
            }
            ParsedApi::ExerciseResult(json) => {
                process_exercise_result(&mut state, &json, &app);
            }
            ParsedApi::HenseiChange {
                fleet_id,
                ship_idx,
                ship_id,
            } => {
                process_hensei_change(&mut state, fleet_id, ship_idx, ship_id, &app);
            }
            ParsedApi::HenseiPresetSelect(json) => {
                process_hensei_preset_select(&mut state, &json, &app);
            }
            ParsedApi::RemodelSlot {
                slot_id,
                success,
                eq_id,
            } => {
                if success {
                    // Use eq_id from request body (api_id param), fallback to player_slotitems lookup
                    let resolved_eq_id = if eq_id > 0 {
                        eq_id
                    } else if slot_id > 0 {
                        state
                            .profile.slotitems
                            .get(&slot_id)
                            .map(|item| item.slotitem_id)
                            .unwrap_or(-1)
                    } else {
                        -1
                    };
                    if resolved_eq_id > 0 {
                        state.history.improved_equipment.insert(resolved_eq_id);
                        crate::improvement::save_improved_history(
                            &state.improved_equipment_path,
                            &state.history.improved_equipment,
                        );
                        notify_sync(&state, vec!["improved_equipment.json"]);
                        info!(
                            "Equipment improved: eq_id={} (instance={})",
                            resolved_eq_id, slot_id
                        );
                    } else {
                        warn!(
                            "remodel_slot success but could not resolve eq_id: slot_id={}, req_eq_id={}",
                            slot_id, eq_id
                        );
                    }
                }
            }
            ParsedApi::QuestStart { quest_id } => {
                if quest_id > 0 {
                    state.history.active_quests.insert(quest_id);
                    info!("Quest {} started", quest_id);
                    let _ = app.emit("quest-started", quest_id);
                }
            }
            ParsedApi::QuestStop { quest_id } => {
                if quest_id > 0 {
                    state.history.active_quests.remove(&quest_id);
                    state.history.active_quest_details.remove(&quest_id);
                    let details: Vec<&models::ActiveQuestDetail> =
                        state.history.active_quest_details.values().collect();
                    info!(
                        "Quest {} cancelled, {} active quests remaining",
                        quest_id,
                        details.len()
                    );
                    let _ = app.emit("quest-list-updated", &details);
                    let _ = app.emit("quest-stopped", quest_id);
                }
            }
            ParsedApi::QuestClear {
                quest_id,
                senka_bonus,
            } => {
                if quest_id > 0 {
                    state.history.active_quests.remove(&quest_id);
                    state.history.active_quest_details.remove(&quest_id);
                    let details: Vec<&models::ActiveQuestDetail> =
                        state.history.active_quest_details.values().collect();
                    info!(
                        "Quest {} completed (senka bonus: {}), {} active quests remaining",
                        quest_id,
                        senka_bonus,
                        details.len()
                    );
                    let _ = app.emit("quest-list-updated", &details);
                    let _ = app.emit("quest-stopped", quest_id);

                    // Add senka bonus if present
                    if senka_bonus > 0 {
                        state.senka.add_quest_bonus(senka_bonus, quest_id);
                        let summary = state.senka.summary();
                        let _ = app.emit("senka-updated", &summary);
                        notify_sync(
                            &state,
                            vec![crate::senka::SenkaTracker::sync_path()],
                        );
                    }
                }
            }
            ParsedApi::Ship3(api_data) => {
                process_ship3(&mut state, &api_data, &app);
            }
            ParsedApi::SlotDeprive(api_data) => {
                process_slot_deprive(&mut state, &api_data, &app);
            }
            ParsedApi::Ranking(raw_json) => {
                // Get admiral name from cached port data
                let admiral_name = state
                    .sortie
                    .last_port_summary
                    .as_ref()
                    .map(|p| p.admiral_name.clone())
                    .unwrap_or_default();

                if admiral_name.is_empty() {
                    warn!("Ranking: admiral name not available, skipping decryption");
                } else {
                    let (entries, own_senka) =
                        crate::senka::decrypt_ranking(&raw_json, &admiral_name);

                    if let Some(senka) = own_senka {
                        state.senka.confirm_ranking(senka);
                        let summary = state.senka.summary();
                        let _ = app.emit("senka-updated", &summary);
                        notify_sync(
                            &state,
                            vec![crate::senka::SenkaTracker::sync_path()],
                        );
                    } else if !entries.is_empty() {
                        info!(
                            "Ranking: decoded {} entries but own admiral '{}' not found in this page",
                            entries.len(),
                            admiral_name
                        );
                    }
                }
            }
            ParsedApi::Other => {}
        }
    });
}

/// Extract senka bonus from clearitemget response's api_bounus array
fn extract_senka_from_clearitemget(json_str: &str) -> i64 {
    let parsed: Result<models::ApiResponse<serde_json::Value>, _> =
        serde_json::from_str(json_str);
    let api_data = match parsed {
        Ok(resp) => match resp.api_data {
            Some(d) => d,
            None => return 0,
        },
        Err(_) => return 0,
    };

    let bounus = match api_data.get("api_bounus").and_then(|v| v.as_array()) {
        Some(arr) => arr,
        None => return 0,
    };

    let mut total_bonus = 0i64;
    for item in bounus {
        if item.is_null() {
            continue;
        }
        let api_type = item.get("api_type").and_then(|v| v.as_i64()).unwrap_or(0);
        if api_type == 18 {
            // Ranking points bonus
            let api_count = item.get("api_count").and_then(|v| v.as_i64()).unwrap_or(1);
            let api_id = item
                .get("api_item")
                .and_then(|i| i.get("api_id"))
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let bonus_per = crate::senka::senka_item_bonus(api_id);
            total_bonus += bonus_per * api_count;
            info!(
                "clearitemget: senka bonus detected: api_id={}, count={}, bonus={}",
                api_id,
                api_count,
                bonus_per * api_count
            );
        }
    }
    total_bonus
}

fn is_battle_endpoint(ep: &str) -> bool {
    ep.starts_with("/kcsapi/api_req_map/")
        || ep.starts_with("/kcsapi/api_req_sortie/")
        || ep.starts_with("/kcsapi/api_req_battle_midnight/")
        || ep.starts_with("/kcsapi/api_req_combined_battle/")
}

/// Process api_start2 master data
fn process_start2(
    state: &mut models::GameStateInner,
    api_data: &models::ApiStart2,
    app: &AppHandle,
) {
    // Populate master ships (name + stype)
    state.master.ships.clear();
    for ship in &api_data.api_mst_ship {
        state.master.ships.insert(
            ship.api_id,
            models::MasterShipInfo {
                name: ship.api_name.clone(),
                stype: ship.api_stype,
            },
        );
    }

    // Populate master stypes
    state.master.stypes.clear();
    for stype in &api_data.api_mst_stype {
        state
            .master.stypes
            .insert(stype.api_id, stype.api_name.clone());
    }

    // Populate master missions
    state.master.missions.clear();
    for mission in &api_data.api_mst_mission {
        state.master.missions.insert(
            mission.api_id,
            models::MissionInfo {
                name: mission.api_name.clone(),
                time: mission.api_time,
            },
        );
    }

    // Populate master slotitems
    state.master.slotitems.clear();
    for item in &api_data.api_mst_slotitem {
        let type_arr = item.api_type.as_array();
        let item_type = type_arr
            .and_then(|arr| arr.get(2))
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;
        let icon_type = type_arr
            .and_then(|arr| arr.get(3))
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;
        state.master.slotitems.insert(
            item.api_id,
            models::MasterSlotItemInfo {
                name: item.api_name.clone(),
                item_type,
                icon_type,
                firepower: item.api_houg,
                torpedo: item.api_raig,
                bombing: item.api_baku,
                aa: item.api_tyku,
                asw: item.api_tais,
                los: item.api_saku,
            },
        );
    }

    // Populate master equip types
    state.master.equip_types.clear();
    for et in &api_data.api_mst_slotitem_equiptype {
        state
            .master.equip_types
            .insert(et.api_id, et.api_name.clone());
    }

    info!(
        "GameState updated: {} master ships, {} stypes, {} missions, {} slotitems, {} equip_types",
        state.master.ships.len(),
        state.master.stypes.len(),
        state.master.missions.len(),
        state.master.slotitems.len(),
        state.master.equip_types.len(),
    );

    let _ = app.emit(
        "master-data-loaded",
        serde_json::json!({
            "shipCount": state.master.ships.len(),
            "stypeCount": state.master.stypes.len(),
            "missionCount": state.master.missions.len(),
            "equipCount": state.master.slotitems.len(),
        }),
    );
}

/// Process api_port data
fn process_port(state: &mut models::GameStateInner, api_data: &models::ApiPort, app: &AppHandle) {
    // Finalize active sortie if any
    if state.sortie.battle_logger.is_in_sortie() {
        if let Some(record) = state.sortie.battle_logger.on_port() {
            let filename = format!("battle_logs/{}.json", record.id);
            notify_sync(state, vec![&filename]);
            let summary = crate::battle_log::SortieRecordSummary::from(&record);
            let _ = app.emit("sortie-complete", &summary);
        }
    }

    // Check quest progress resets on returning to port
    crate::quest_progress::check_resets(
        &mut state.history.quest_progress,
        &state.history.sortie_quest_defs,
        &state.quest_progress_path,
    );

    // Update player ships from port data
    state.profile.ships.clear();
    for ship in &api_data.api_ship {
        let master = state.master.ships.get(&ship.api_ship_id);
        let name = master
            .map(|m| m.name.clone())
            .unwrap_or_else(|| format!("Unknown({})", ship.api_ship_id));
        let stype = master.map(|m| m.stype).unwrap_or(0);

        let firepower = extract_stat_value(&ship.api_karyoku);
        let torpedo = extract_stat_value(&ship.api_raisou);
        let aa = extract_stat_value(&ship.api_taiku);
        let armor = extract_stat_value(&ship.api_soukou);
        let asw = extract_stat_value(&ship.api_taisen);
        let evasion = extract_stat_value(&ship.api_kaihi);
        let los = extract_stat_value(&ship.api_sakuteki);
        let luck = extract_stat_value(&ship.api_lucky);
        let slot = extract_slot_ids(&ship.api_slot);

        state.profile.ships.insert(
            ship.api_id,
            ShipInfo {
                ship_id: ship.api_ship_id,
                name,
                stype,
                lv: ship.api_lv,
                hp: ship.api_nowhp,
                maxhp: ship.api_maxhp,
                cond: ship.api_cond,
                fuel: ship.api_fuel,
                bull: ship.api_bull,
                firepower,
                torpedo,
                aa,
                armor,
                asw,
                evasion,
                los,
                luck,
                locked: ship.api_locked == 1,
                slot,
                slot_ex: ship.api_slot_ex,
                soku: ship.api_soku,
            },
        );
    }

    // Update fleet compositions
    state.profile.fleets.clear();
    for fleet in &api_data.api_deck_port {
        let ship_ids: Vec<i32> = fleet
            .api_ship
            .iter()
            .filter(|&&id| id > 0)
            .copied()
            .collect();
        while state.profile.fleets.len() < fleet.api_id as usize {
            state.profile.fleets.push(Vec::new());
        }
        state.profile.fleets[fleet.api_id as usize - 1] = ship_ids;
    }

    info!(
        "GameState updated: {} player ships, {} slotitems in memory",
        state.profile.ships.len(),
        state.profile.slotitems.len(),
    );

    // Build enriched fleet summaries
    let fleets: Vec<models::FleetSummary> = api_data
        .api_deck_port
        .iter()
        .map(|fleet| {
            let ships: Vec<models::ShipSummary> = fleet
                .api_ship
                .iter()
                .filter(|&&id| id > 0)
                .filter_map(|&id| {
                    state.profile.ships.get(&id).map(|info| {
                        let marks = collect_ship_marks(
                            info,
                            &state.profile.slotitems,
                            &state.master.slotitems,
                        );
                        models::ShipSummary {
                            id,
                            name: info.name.clone(),
                            lv: info.lv,
                            hp: info.hp,
                            maxhp: info.maxhp,
                            cond: info.cond,
                            fuel: info.fuel,
                            bull: info.bull,
                            damecon_name: marks.damecon_name,
                            special_equips: marks.special_equips,
                            can_opening_asw: marks.can_opening_asw,
                            soku: info.soku,
                        }
                    })
                })
                .collect();

            let expedition = parse_expedition(&fleet.api_mission, &state.master.missions);

            models::FleetSummary {
                id: fleet.api_id,
                name: fleet.api_name.clone(),
                ships,
                expedition,
            }
        })
        .collect();

    // Build enriched dock summaries
    let ndock: Vec<models::DockSummary> = api_data
        .api_ndock
        .iter()
        .map(|dock| {
            let ship_name = if dock.api_ship_id > 0 {
                state
                    .profile.ships
                    .get(&dock.api_ship_id)
                    .map(|info| info.name.clone())
                    .unwrap_or_else(|| format!("Unknown({})", dock.api_ship_id))
            } else {
                String::new()
            };

            models::DockSummary {
                id: dock.api_id,
                state: dock.api_state,
                ship_id: dock.api_ship_id,
                ship_name,
                complete_time: dock.api_complete_time,
            }
        })
        .collect();

    let port_data = models::PortSummary {
        admiral_name: api_data.api_basic.api_nickname.clone(),
        admiral_level: api_data.api_basic.api_level,
        admiral_rank: api_data.api_basic.api_rank,
        ship_count: api_data.api_ship.len(),
        ship_capacity: api_data.api_basic.api_max_chara,
        fuel: get_material(&api_data.api_material, 1),
        ammo: get_material(&api_data.api_material, 2),
        steel: get_material(&api_data.api_material, 3),
        bauxite: get_material(&api_data.api_material, 4),
        instant_repair: get_material(&api_data.api_material, 5),
        instant_build: get_material(&api_data.api_material, 6),
        dev_material: get_material(&api_data.api_material, 7),
        improvement_material: get_material(&api_data.api_material, 8),
        fleets,
        ndock,
    };

    info!(
        "Port data: Admiral {} Lv.{}, {} ships",
        port_data.admiral_name, port_data.admiral_level, port_data.ship_count
    );

    // Cache for re-emitting during sortie
    state.sortie.last_port_summary = Some(port_data.clone());

    match app.emit("port-data", &port_data) {
        Ok(_) => info!("port-data event emitted successfully"),
        Err(e) => error!("Failed to emit port-data: {}", e),
    }

    // Update senka tracker with HQ experience
    let hq_exp = match &api_data.api_basic.api_experience {
        serde_json::Value::Number(n) => n.as_i64().unwrap_or(0),
        serde_json::Value::Array(arr) => arr.first().and_then(|v| v.as_i64()).unwrap_or(0),
        _ => 0,
    };
    if hq_exp > 0 {
        let (changed, checkpoint_crossed) = state.senka.update_experience(hq_exp);
        let summary = state.senka.summary_with_checkpoint(checkpoint_crossed);
        let _ = app.emit("senka-updated", &summary);
        if changed || checkpoint_crossed {
            notify_sync(state, vec![crate::senka::SenkaTracker::sync_path()]);
        }
    }
}

/// Process api_get_member/questlist data
/// api_list contains quest objects (with api_no, api_state) or 0 values (gaps).
/// api_state: 1=not accepted, 2=accepted/in progress, 3=completed
fn process_questlist(
    state: &mut models::GameStateInner,
    data: &crate::api::dto::battle::ApiQuestListResponse,
    app: &AppHandle,
) {
    if let Some(api_list) = data.api_list.as_ref() {
        for item in api_list {
            let api_no = match item.get("api_no").and_then(|v| v.as_i64()) {
                Some(n) => n as i32,
                None => continue, // skip 0 / null entries
            };
            let api_state = item.get("api_state").and_then(|v| v.as_i64()).unwrap_or(0) as i32;

            match api_state {
                2 | 3 => {
                    // Accepted or completed → add to active set
                    state.history.active_quests.insert(api_no);
                    let title = item
                        .get("api_title")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let category = item
                        .get("api_category")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0) as i32;
                    state.history.active_quest_details.insert(
                        api_no,
                        models::ActiveQuestDetail {
                            id: api_no,
                            title,
                            category,
                        },
                    );
                }
                1 => {
                    // Not accepted → remove from active set
                    state.history.active_quests.remove(&api_no);
                    state.history.active_quest_details.remove(&api_no);
                }
                _ => {}
            }
        }

        let details: Vec<&models::ActiveQuestDetail> =
            state.history.active_quest_details.values().collect();
        info!("Active quests updated: {} quests", details.len());
        let _ = app.emit("quest-list-updated", &details);
    }
}

/// Process battle-related API endpoints
fn process_battle(
    state: &mut models::GameStateInner,
    endpoint: &str,
    request_body: &str,
    json: &serde_json::Value,
    app: &AppHandle,
) {
    info!(
        "Battle API: {} (req_body len={})",
        endpoint,
        request_body.len(),
    );

    match endpoint {
        "/kcsapi/api_req_map/start" => {
            let fleets = state.profile.fleets.clone();
            let player_ships = state.profile.ships.clone();
            let player_slotitems = state.profile.slotitems.clone();
            info!(
                "Sortie start: {} ships in fleet, {} slotitems available",
                player_ships.len(),
                player_slotitems.len(),
            );
            state.sortie.battle_logger.on_sortie_start(
                json,
                request_body,
                &fleets,
                &player_ships,
                &player_slotitems,
            );
            let _ = app.emit(
                "sortie-start",
                serde_json::json!({
                    "in_sortie": true,
                }),
            );
            // Emit sortie-update with the initial sortie record
            if let Some(sortie) = state.sortie.battle_logger.active_sortie_ref() {
                let summary = crate::battle_log::SortieRecordSummary::from(sortie);
                let _ = app.emit("sortie-update", &summary);

                // Show formation hint for first cell if previously visited
                if let Some(node) = sortie.nodes.last() {
                    let key = format!("{}-{}-{}", sortie.map_area, sortie.map_no, node.cell_no);
                    if let Some(&formation) = state.formation_memory.get(&key) {
                        let ship_count = sortie.ships.len();
                        info!("Formation hint: {} -> {} (ships={})", key, formation_name(formation), ship_count);
                        show_formation_hint(app, formation, ship_count);
                    }
                }
            }
        }
        "/kcsapi/api_req_map/next" => {
            // Check for taiha (大破) ships advancing — warn the player
            let mut taiha_shown = false;
            let taiha_enabled = app.try_state::<crate::AppState>()
                .map(|s| s.taiha_alert_enabled.load(std::sync::atomic::Ordering::Relaxed))
                .unwrap_or(true);
            if taiha_enabled {
            if let Some(sortie) = state.sortie.battle_logger.active_sortie_ref() {
                let fleet_id = sortie.fleet_id as usize;
                let fleet_idx = fleet_id.saturating_sub(1);
                if fleet_idx < state.profile.fleets.len() {
                    let ship_ids = &state.profile.fleets[fleet_idx];
                    let mut taiha_names: Vec<String> = Vec::new();
                    for (i, &ship_id) in ship_ids.iter().enumerate() {
                        if let Some(ship) = state.profile.ships.get(&ship_id) {
                            if ship.maxhp > 0 && ship.hp as f64 / ship.maxhp as f64 <= 0.25 && ship.hp > 0 {
                                // Check if damage control is equipped
                                let has_damecon = ship.slot.iter()
                                    .chain(std::iter::once(&ship.slot_ex))
                                    .any(|&slot_id| {
                                        slot_id > 0 && state.profile.slotitems.get(&slot_id)
                                            .and_then(|p| state.master.slotitems.get(&p.slotitem_id))
                                            .map(|m| m.icon_type == 14)
                                            .unwrap_or(false)
                                    });
                                if has_damecon {
                                    info!("Ship {} ({}) is taiha but has damecon — skipping warning", ship.name, i);
                                } else {
                                    warn!("Ship {} ({}) is taiha (HP {}/{}) and advancing without damecon!", ship.name, i, ship.hp, ship.maxhp);
                                    taiha_names.push(ship.name.clone());
                                }
                            }
                        }
                    }
                    if !taiha_names.is_empty() {
                        taiha_shown = true;
                        warn!("TAIHA ADVANCE WARNING: {} ships critically damaged: {:?}", taiha_names.len(), taiha_names);
                        // Show overlay directly via eval (more reliable than event system in multi-webview)
                        if let Some(overlay) = app.get_webview("game-overlay") {
                            if let Some(win) = app.get_window("game") {
                                if let Ok(size) = win.inner_size() {
                                    let _ = overlay.set_position(tauri::LogicalPosition::new(0.0, 0.0));
                                    let _ = overlay.set_size(size);
                                }
                            }
                            let ships_json = serde_json::to_string(&taiha_names).unwrap_or_else(|_| "[]".to_string());
                            let _ = overlay.eval(&format!("window.showTaihaWarning({})", ships_json));
                        }
                    }
                }
            }
            } // taiha_enabled

            match serde_json::from_value::<
                models::ApiResponse<crate::api::dto::battle::ApiMapNextResponse>,
            >(json.clone())
            {
                Ok(resp) => {
                    if let Some(data) = resp.api_data {
                        state.sortie.battle_logger.on_map_next(&data, json);

                        // Show formation hint for new cell (skip if taiha warning is active)
                        if !taiha_shown {
                            if let Some(sortie) = state.sortie.battle_logger.active_sortie_ref() {
                                if let Some(node) = sortie.nodes.last() {
                                    let key = format!("{}-{}-{}", sortie.map_area, sortie.map_no, node.cell_no);
                                    if let Some(&formation) = state.formation_memory.get(&key) {
                                        let ship_count = sortie.ships.len();
                                        info!("Formation hint: {} -> {} (ships={})", key, formation_name(formation), ship_count);
                                        show_formation_hint(app, formation, ship_count);
                                    }
                                }
                            }
                        }

                        // 1-6 goal node detection: event_id 9 = goal reached
                        // 1-6 has no boss battle so EO bonus comes from reaching the goal
                        if data.api_event_id == Some(9) {
                            if let Some(sortie) =
                                state.sortie.battle_logger.active_sortie_ref()
                            {
                                if sortie.map_area == 1 && sortie.map_no == 6 {
                                    let bonus =
                                        crate::senka::eo_bonus_for_map(1, 6);
                                    if bonus > 0 {
                                        info!("1-6 goal reached, EO bonus: {}", bonus);
                                        state.senka.add_eo_bonus(bonus, "1-6");
                                        let summary = state.senka.summary();
                                        let _ = app.emit("senka-updated", &summary);
                                        notify_sync(
                                            &state,
                                            vec![crate::senka::SenkaTracker::sync_path()],
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => error!("Failed to parse map/next response: {}", e),
            }
        }
        // Day battles
        "/kcsapi/api_req_sortie/battle"
        | "/kcsapi/api_req_sortie/airbattle"
        | "/kcsapi/api_req_sortie/ld_airbattle"
        | "/kcsapi/api_req_sortie/ld_shooting"
        | "/kcsapi/api_req_sortie/night_to_day"
        | "/kcsapi/api_req_combined_battle/battle"
        | "/kcsapi/api_req_combined_battle/battle_water"
        | "/kcsapi/api_req_combined_battle/each_battle"
        | "/kcsapi/api_req_combined_battle/each_battle_water"
        | "/kcsapi/api_req_combined_battle/ec_battle"
        | "/kcsapi/api_req_combined_battle/ld_airbattle"
        | "/kcsapi/api_req_combined_battle/ld_shooting" => {
            match serde_json::from_value::<
                models::ApiResponse<crate::api::dto::battle::ApiBattleResponse>,
            >(json.clone())
            {
                Ok(resp) => {
                    if let Some(data) = resp.api_data {
                        state.sortie.battle_logger.on_battle(&data, json);

                        // Save formation to memory and hide hint
                        if let Some(arr) = &data.api_formation {
                            let friendly_formation = arr.first().copied().unwrap_or(0);
                            if friendly_formation > 0 {
                                if let Some(sortie) = state.sortie.battle_logger.active_sortie_ref() {
                                    if let Some(node) = sortie.nodes.last() {
                                        let key = format!("{}-{}-{}", sortie.map_area, sortie.map_no, node.cell_no);
                                        info!("Formation saved: {} = {} ({})", key, friendly_formation, formation_name(friendly_formation));
                                        state.formation_memory.insert(key, friendly_formation);
                                        models::save_formation_memory(&state.formation_memory_path, &state.formation_memory);
                                        notify_sync(&state, vec!["formation_memory.json"]);
                                    }
                                }
                            }
                        }
                        hide_formation_hint(app);
                    }
                }
                Err(e) => error!("Failed to parse battle response: {}", e),
            }
        }
        // Night battles
        "/kcsapi/api_req_battle_midnight/battle"
        | "/kcsapi/api_req_battle_midnight/sp_midnight"
        | "/kcsapi/api_req_combined_battle/midnight_battle"
        | "/kcsapi/api_req_combined_battle/sp_midnight"
        | "/kcsapi/api_req_combined_battle/ec_midnight_battle"
        | "/kcsapi/api_req_combined_battle/ec_night_to_day" => {
            match serde_json::from_value::<
                models::ApiResponse<crate::api::dto::battle::ApiBattleResponse>,
            >(json.clone())
            {
                Ok(resp) => {
                    if let Some(data) = resp.api_data {
                        state.sortie.battle_logger.on_midnight_battle(&data, json);
                    }
                }
                Err(e) => error!("Failed to parse midnight battle response: {}", e),
            }
        }
        // Battle results
        "/kcsapi/api_req_sortie/battleresult" | "/kcsapi/api_req_combined_battle/battleresult" => {
            let master_ships = state.master.ships.clone();

            match serde_json::from_value::<
                models::ApiResponse<crate::api::dto::battle::ApiBattleResultResponse>,
            >(json.clone())
            {
                Ok(resp) => {
                    if let Some(data) = resp.api_data {
                        state
                            .sortie.battle_logger
                            .on_battle_result(&data, json, &master_ships);
                    }
                }
                Err(e) => error!("Failed to parse battleresult response: {}", e),
            }

            // Update player ships HP from battle result and re-emit port-data
            if let Some(sortie) = state.sortie.battle_logger.active_sortie_ref() {
                let fleet_id = sortie.fleet_id as usize;
                let fleet_idx = fleet_id.saturating_sub(1);
                // Get friendly HP after battle from the last node's battle detail
                let hp_after: Option<Vec<crate::battle_log::HpState>> = sortie
                    .nodes
                    .last()
                    .and_then(|n| n.battle.as_ref())
                    .map(|b| b.friendly_hp.clone());

                if let Some(hp_states) = &hp_after {
                    if fleet_idx < state.profile.fleets.len() {
                        let ship_ids = state.profile.fleets[fleet_idx].clone();
                        for (i, &ship_id) in ship_ids.iter().enumerate() {
                            if let (Some(hp_state), Some(ship_info)) =
                                (hp_states.get(i), state.profile.ships.get_mut(&ship_id))
                            {
                                ship_info.hp = hp_state.after.max(0);
                            }
                        }
                        info!(
                            "Updated fleet {} ship HP from battle result ({} ships)",
                            fleet_id,
                            ship_ids.len().min(hp_states.len()),
                        );
                    }
                }

                // Re-emit port-data with updated HP
                if let Some(ref mut cached) = state.sortie.last_port_summary {
                    // Update fleet ship HP in cached summary
                    if fleet_idx < cached.fleets.len() {
                        if let Some(hp_states) = &hp_after {
                            for (i, ship) in cached.fleets[fleet_idx].ships.iter_mut().enumerate() {
                                if let Some(hp_state) = hp_states.get(i) {
                                    ship.hp = hp_state.after.max(0);
                                }
                            }
                        }
                    }
                    let _ = app.emit("port-data", &*cached);
                    info!("Re-emitted port-data with updated battle HP");
                }

                // Quest progress: extract map area, rank, boss from active sortie
                let gauge_suffix = match sortie.gauge_num {
                    Some(1) => "(1st)",
                    Some(2) => "(2nd)",
                    Some(3) => "(3rd)",
                    _ => "",
                };
                let map_area_str = format!("{}-{}{}", sortie.map_area, sortie.map_no, gauge_suffix);
                let last_node = sortie.nodes.last();
                let is_boss = last_node.map(|n| n.event_id == 5).unwrap_or(false);
                let rank = last_node
                    .and_then(|n| n.battle.as_ref())
                    .map(|b| b.rank.clone())
                    .unwrap_or_default();

                if !rank.is_empty() {
                    // Extract sunk enemy ship stypes for sinking quests
                    let sunk_enemy_stypes: Vec<i32> = last_node
                        .and_then(|n| n.battle.as_ref())
                        .map(|b| {
                            b.enemy_ships
                                .iter()
                                .zip(b.enemy_hp.iter())
                                .filter(|(_, hp)| hp.after <= 0)
                                .filter_map(|(ship, _)| {
                                    master_ships.get(&ship.ship_id).map(|m| m.stype)
                                })
                                .collect()
                        })
                        .unwrap_or_default();

                    let changed = crate::quest_progress::on_battle_result(
                        &mut state.history.quest_progress,
                        &map_area_str,
                        &rank,
                        is_boss,
                        &sunk_enemy_stypes,
                        &state.history.active_quests,
                        &state.history.sortie_quest_defs,
                        &state.quest_progress_path,
                    );
                    if changed {
                        notify_sync(state, vec!["quest_progress.json"]);
                        let path = state.quest_progress_path.clone();
                        let defs = state.history.sortie_quest_defs.clone();
                        let aq = state.history.active_quests.clone();
                        let progress = crate::quest_progress::get_active_progress(
                            &mut state.history.quest_progress,
                            &aq,
                            &defs,
                            &path,
                        );
                        let _ = app.emit("quest-progress-updated", &progress);
                    }
                }
            }

            // Record per-battle HQ exp (api_get_exp) and check for EO bonus
            if let Some(api_data) = json.get("api_data") {
                let mut senka_changed = false;

                // Record HQ exp from this battle
                let hq_exp = api_data
                    .get("api_get_exp")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                if hq_exp > 0 {
                    let map_display = state
                        .sortie
                        .battle_logger
                        .active_sortie_ref()
                        .map(|s| format!("{}-{}", s.map_area, s.map_no))
                        .unwrap_or_default();
                    state.senka.add_battle_exp(hq_exp, &map_display);
                    senka_changed = true;
                }

                // Check for EO ranking bonus (api_get_exmap_rate)
                if let Some(exmap_rate) = api_data.get("api_get_exmap_rate") {
                    let rate = exmap_rate.as_i64().unwrap_or(0);
                    if rate > 0 {
                        let map_display = state
                            .sortie
                            .battle_logger
                            .active_sortie_ref()
                            .map(|s| format!("{}-{}", s.map_area, s.map_no))
                            .unwrap_or_default();
                        state.senka.add_eo_bonus(rate, &map_display);
                        senka_changed = true;
                    }
                }

                if senka_changed {
                    let summary = state.senka.summary();
                    let _ = app.emit("senka-updated", &summary);
                    notify_sync(
                        state,
                        vec![crate::senka::SenkaTracker::sync_path()],
                    );
                }
            }

            // Emit sortie-update event for real-time frontend updates
            if let Some(sortie) = state.sortie.battle_logger.active_sortie_ref() {
                let summary = crate::battle_log::SortieRecordSummary::from(sortie);
                let _ = app.emit("sortie-update", &summary);
            }
        }
        _ => {
            info!("Unhandled battle endpoint: {}", endpoint);
        }
    }
}

/// Process exercise battle result (api_req_practice/battle_result)
fn process_exercise_result(
    state: &mut models::GameStateInner,
    json: &serde_json::Value,
    app: &AppHandle,
) {
    let api_data = match json.get("api_data") {
        Some(d) => d,
        None => return,
    };

    let rank = api_data
        .get("api_win_rank")
        .and_then(|v| v.as_str())
        .unwrap_or("-")
        .to_string();

    info!("Exercise result: rank={}", rank);

    // Record HQ exp from exercise
    let hq_exp = api_data
        .get("api_get_exp")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    if hq_exp > 0 {
        state.senka.add_battle_exp(hq_exp, "演習");
        let summary = state.senka.summary();
        let _ = app.emit("senka-updated", &summary);
        notify_sync(state, vec![crate::senka::SenkaTracker::sync_path()]);
    }

    let changed = crate::quest_progress::on_exercise_result(
        &mut state.history.quest_progress,
        &rank,
        &state.history.active_quests,
        &state.history.sortie_quest_defs,
        &state.quest_progress_path,
    );
    if changed {
        notify_sync(state, vec!["quest_progress.json"]);
        let path = state.quest_progress_path.clone();
        let defs = state.history.sortie_quest_defs.clone();
        let aq = state.history.active_quests.clone();
        let progress = crate::quest_progress::get_active_progress(
            &mut state.history.quest_progress,
            &aq,
            &defs,
            &path,
        );
        let _ = app.emit("quest-progress-updated", &progress);
    }
}

/// Process api_req_hensei/change - fleet composition change
/// request body has: api_id (fleet 1-4), api_ship_idx (position 0-5), api_ship_id (ship instance ID, -1=remove, -2=remove all except flagship)
fn process_hensei_change(
    state: &mut models::GameStateInner,
    fleet_id: usize,
    ship_idx: i32,
    ship_id: i32,
    app: &AppHandle,
) {
    if fleet_id == 0 || fleet_id > state.profile.fleets.len() {
        warn!("hensei/change: invalid fleet_id {}", fleet_id);
        return;
    }
    let fidx = fleet_id - 1;

    if ship_id == -2 {
        // Remove all except flagship
        let flagship = if !state.profile.fleets[fidx].is_empty() {
            state.profile.fleets[fidx][0]
        } else {
            return;
        };
        state.profile.fleets[fidx] = vec![flagship];
        info!(
            "Fleet {} cleared (flagship {} retained)",
            fleet_id, flagship
        );
    } else if ship_id == -1 {
        // Remove ship at index
        let idx = ship_idx as usize;
        if idx < state.profile.fleets[fidx].len() {
            state.profile.fleets[fidx].remove(idx);
            info!("Fleet {} removed ship at index {}", fleet_id, idx);
        }
    } else {
        // Add/swap ship
        let idx = ship_idx as usize;

        // Find the ship being replaced at this position (if any)
        let replaced_id = if idx < state.profile.fleets[fidx].len() {
            state.profile.fleets[fidx][idx]
        } else {
            -1
        };

        // Check if this ship is already in another fleet (or same fleet different position) and swap
        for fi in 0..state.profile.fleets.len() {
            for si in 0..state.profile.fleets[fi].len() {
                if state.profile.fleets[fi][si] == ship_id && !(fi == fidx && si == idx) {
                    // Found the ship in another position — swap or remove
                    if replaced_id > 0 {
                        state.profile.fleets[fi][si] = replaced_id;
                    } else {
                        state.profile.fleets[fi].remove(si);
                    }
                    break;
                }
            }
        }

        // Place the ship at the target position
        while state.profile.fleets[fidx].len() <= idx {
            state.profile.fleets[fidx].push(-1);
        }
        state.profile.fleets[fidx][idx] = ship_id;

        // Remove any -1 gaps
        state.profile.fleets[fidx].retain(|&id| id > 0);

        info!("Fleet {} set index {} to ship {}", fleet_id, idx, ship_id);
    }

    emit_fleet_update(state, app);
}

/// Process api_req_hensei/preset_select - load preset fleet
fn process_hensei_preset_select(
    state: &mut models::GameStateInner,
    data: &crate::api::dto::battle::ApiHenseiPresetSelectResponse,
    app: &AppHandle,
) {
    let fleet_id = data
        .api_fleet
        .as_ref()
        .and_then(|f| f.get("api_id"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0) as usize;
    if fleet_id == 0 || fleet_id > state.profile.fleets.len() {
        warn!("preset_select: invalid fleet_id {}", fleet_id);
        return;
    }
    let fidx = fleet_id - 1;

    if let Some(api_ship) = data
        .api_fleet
        .as_ref()
        .and_then(|f| f.get("api_ship"))
        .and_then(|v| v.as_array())
    {
        let ship_ids: Vec<i32> = api_ship
            .iter()
            .filter_map(|v| v.as_i64().map(|id| id as i32))
            .filter(|&id| id > 0)
            .collect();

        // Remove these ships from other fleets (preset load can pull ships)
        for fi in 0..state.profile.fleets.len() {
            if fi == fidx {
                continue;
            }
            state.profile.fleets[fi].retain(|id| !ship_ids.contains(id));
        }

        state.profile.fleets[fidx] = ship_ids;
        info!(
            "Fleet {} loaded from preset: {} ships",
            fleet_id,
            state.profile.fleets[fidx].len()
        );
    }

    emit_fleet_update(state, app);
}

/// Process api_get_member/ship3 - update ship slot data after equipment changes
fn process_ship3(
    state: &mut models::GameStateInner,
    api_data: &serde_json::Value,
    app: &AppHandle,
) {
    // Update ships from api_ship_data
    if let Some(ships) = api_data.get("api_ship_data") {
        if let Ok(ship_list) = serde_json::from_value::<Vec<models::PlayerShip>>(ships.clone()) {
            for ship in &ship_list {
                let master = state.master.ships.get(&ship.api_ship_id);
                let name = master
                    .map(|m| m.name.clone())
                    .unwrap_or_else(|| format!("Unknown({})", ship.api_ship_id));
                let stype = master.map(|m| m.stype).unwrap_or(0);
                let firepower = extract_stat_value(&ship.api_karyoku);
                let torpedo = extract_stat_value(&ship.api_raisou);
                let aa = extract_stat_value(&ship.api_taiku);
                let armor = extract_stat_value(&ship.api_soukou);
                let asw = extract_stat_value(&ship.api_taisen);
                let evasion = extract_stat_value(&ship.api_kaihi);
                let los = extract_stat_value(&ship.api_sakuteki);
                let luck = extract_stat_value(&ship.api_lucky);
                let slot = extract_slot_ids(&ship.api_slot);

                state.profile.ships.insert(
                    ship.api_id,
                    ShipInfo {
                        ship_id: ship.api_ship_id,
                        name,
                        stype,
                        lv: ship.api_lv,
                        hp: ship.api_nowhp,
                        maxhp: ship.api_maxhp,
                        cond: ship.api_cond,
                        fuel: ship.api_fuel,
                        bull: ship.api_bull,
                        firepower,
                        torpedo,
                        aa,
                        armor,
                        asw,
                        evasion,
                        los,
                        luck,
                        locked: ship.api_locked == 1,
                        slot,
                        slot_ex: ship.api_slot_ex,
                        soku: ship.api_soku,
                    },
                );
            }
            info!("ship3: updated {} ships", ship_list.len());
        }
    }

    // Update fleet compositions from api_deck_data
    if let Some(decks) = api_data.get("api_deck_data") {
        if let Ok(deck_list) = serde_json::from_value::<Vec<models::Fleet>>(decks.clone()) {
            for fleet in &deck_list {
                let ship_ids: Vec<i32> = fleet
                    .api_ship
                    .iter()
                    .filter(|&&id| id > 0)
                    .copied()
                    .collect();
                let fidx = fleet.api_id as usize;
                while state.profile.fleets.len() < fidx {
                    state.profile.fleets.push(Vec::new());
                }
                if fidx > 0 {
                    state.profile.fleets[fidx - 1] = ship_ids;
                }
            }
        }
    }

    emit_fleet_update(state, app);
}

/// Process api_req_kaisou/slot_deprive - equipment transfer between ships
/// Response contains api_ship_data with api_set_ship and api_unset_ship
fn process_slot_deprive(
    state: &mut models::GameStateInner,
    api_data: &serde_json::Value,
    app: &AppHandle,
) {
    let ship_data = match api_data.get("api_ship_data") {
        Some(sd) => sd,
        None => {
            warn!("slot_deprive: no api_ship_data found");
            return;
        }
    };

    let mut updated = 0;
    for key in &["api_set_ship", "api_unset_ship"] {
        if let Some(ship_val) = ship_data.get(*key) {
            match serde_json::from_value::<models::PlayerShip>(ship_val.clone()) {
                Ok(ship) => {
                    let master = state.master.ships.get(&ship.api_ship_id);
                    let name = master
                        .map(|m| m.name.clone())
                        .unwrap_or_else(|| format!("Unknown({})", ship.api_ship_id));
                    let stype = master.map(|m| m.stype).unwrap_or(0);
                    let slot = extract_slot_ids(&ship.api_slot);

                    state.profile.ships.insert(
                        ship.api_id,
                        ShipInfo {
                            ship_id: ship.api_ship_id,
                            name,
                            stype,
                            lv: ship.api_lv,
                            hp: ship.api_nowhp,
                            maxhp: ship.api_maxhp,
                            cond: ship.api_cond,
                            fuel: ship.api_fuel,
                            bull: ship.api_bull,
                            firepower: extract_stat_value(&ship.api_karyoku),
                            torpedo: extract_stat_value(&ship.api_raisou),
                            aa: extract_stat_value(&ship.api_taiku),
                            armor: extract_stat_value(&ship.api_soukou),
                            asw: extract_stat_value(&ship.api_taisen),
                            evasion: extract_stat_value(&ship.api_kaihi),
                            los: extract_stat_value(&ship.api_sakuteki),
                            luck: extract_stat_value(&ship.api_lucky),
                            locked: ship.api_locked == 1,
                            slot,
                            slot_ex: ship.api_slot_ex,
                            soku: ship.api_soku,
                        },
                    );
                    updated += 1;
                }
                Err(e) => {
                    error!("slot_deprive: failed to parse {}: {}", key, e);
                }
            }
        }
    }
    info!("slot_deprive: updated {} ships", updated);

    emit_fleet_update(state, app);
}

/// Collected marks/indicators for a ship (damecon, special equips, opening ASW)
struct ShipMarks {
    damecon_name: Option<String>,
    special_equips: Vec<models::SpecialEquip>,
    can_opening_asw: bool,
}

/// Collect all ship marks in a single equipment loop: damecon, special equips, and opening ASW
fn collect_ship_marks(
    ship: &models::ShipInfo,
    player_slotitems: &std::collections::HashMap<i32, models::PlayerSlotItem>,
    master_slotitems: &std::collections::HashMap<i32, models::MasterSlotItemInfo>,
) -> ShipMarks {
    let mut damecon_name: Option<String> = None;
    let mut special_equips: Vec<models::SpecialEquip> = Vec::new();
    // Opening ASW detection data
    let mut has_sonar = false;
    let mut has_large_sonar = false;
    let mut has_asw7_aircraft = false; // 対潜7以上の艦攻/回転翼/哨戒機
    let mut has_asw1_bomber = false; // 対潜1以上の艦攻/艦爆
    let mut has_asw1_aircraft = false; // 対潜1以上の艦攻/艦爆/回転翼/哨戒機
    let mut equip_asw_total: i32 = 0;
    let mut rotorcraft_count = 0; // 回転翼機 (item_type=25)
    let mut s51j_count = 0; // S-51J系 (item_type=26, 対潜哨戒機)
    let mut has_seaplane_bomber = false; // 水爆 (item_type=11)
    let mut has_depth_charge_projector = false; // 爆雷投射機 (item_type=15)
    let mut has_depth_charge = false; // 爆雷 (item_type=17)

    for &slot_id in ship.slot.iter().chain(std::iter::once(&ship.slot_ex)) {
        if slot_id <= 0 {
            continue;
        }
        let Some(player) = player_slotitems.get(&slot_id) else {
            continue;
        };
        let Some(master) = master_slotitems.get(&player.slotitem_id) else {
            continue;
        };

        // Damecon (icon_type=14) — take first only
        if master.icon_type == 14 && damecon_name.is_none() {
            damecon_name = Some(master.name.clone());
        }

        // Special equips: landing craft (20), drum canister (25)
        if master.icon_type == 20 || master.icon_type == 25 {
            special_equips.push(models::SpecialEquip {
                name: master.name.clone(),
                icon_type: master.icon_type,
            });
        }

        // Sonar detection
        if master.icon_type == 17 {
            has_sonar = true; // small sonar
        }
        if master.icon_type == 18 {
            has_sonar = true; // large sonar counts as sonar too
            has_large_sonar = true;
        }

        // Equipment ASW total
        equip_asw_total += master.asw;

        // Aircraft type checks
        let it = master.item_type;
        // 艦攻(8), 回転翼(25), 対潜哨戒機(26): ASW>=7 check
        if (it == 8 || it == 25 || it == 26) && master.asw >= 7 {
            has_asw7_aircraft = true;
        }
        // 艦攻(8), 艦爆(7): ASW>=1 check
        if (it == 8 || it == 7) && master.asw >= 1 {
            has_asw1_bomber = true;
        }
        // 艦攻(8), 艦爆(7), 回転翼(25), 対潜哨戒機(26): ASW>=1 check
        if (it == 8 || it == 7 || it == 25 || it == 26) && master.asw >= 1 {
            has_asw1_aircraft = true;
        }
        // Rotorcraft count (item_type=25)
        if it == 25 {
            rotorcraft_count += 1;
        }
        // Patrol aircraft count (item_type=26) for S-51J series
        if it == 26 {
            s51j_count += 1;
        }
        // Seaplane bomber (item_type=11)
        if it == 11 {
            has_seaplane_bomber = true;
        }
        // Depth charge projector (item_type=15)
        if it == 15 {
            has_depth_charge_projector = true;
        }
        // Depth charge (item_type=17)
        if it == 17 {
            has_depth_charge = true;
        }
    }

    if !special_equips.is_empty() {
        info!(
            "Ship {} has {} special equips: {:?}",
            ship.name,
            special_equips.len(),
            special_equips
                .iter()
                .map(|e| format!("{}(icon={})", e.name, e.icon_type))
                .collect::<Vec<_>>()
        );
    }

    let can_opening_asw = check_opening_asw(
        ship,
        has_sonar,
        has_large_sonar,
        has_asw7_aircraft,
        has_asw1_bomber,
        has_asw1_aircraft,
        equip_asw_total,
        rotorcraft_count,
        s51j_count,
        has_seaplane_bomber,
        has_depth_charge_projector,
        has_depth_charge,
    );

    ShipMarks {
        damecon_name,
        special_equips,
        can_opening_asw,
    }
}

/// Determine if a ship can perform opening ASW
fn check_opening_asw(
    ship: &models::ShipInfo,
    has_sonar: bool,
    _has_large_sonar: bool,
    has_asw7_aircraft: bool,
    has_asw1_bomber: bool,
    has_asw1_aircraft: bool,
    equip_asw_total: i32,
    rotorcraft_count: i32,
    s51j_count: i32,
    has_seaplane_bomber: bool,
    has_depth_charge_projector: bool,
    has_depth_charge: bool,
) -> bool {
    let asw = ship.asw; // equipped total ASW
    let stype = ship.stype;
    let sid = ship.ship_id;

    // 1. Unconditional ships (always OASW regardless of equipment)
    const UNCONDITIONAL: &[i32] = &[
        141,  // 五十鈴改二
        478,  // 龍田改二
        624,  // 夕張改二丁
        394,  // Jervis改
        893,  // Jervis Mk.II
        681,  // Janus改
        875,  // Janus Mk.II
        562,  // Fletcher
        596,  // Fletcher改 Mod.2
        628,  // Fletcher Mk.II
        629,  // Fletcher Mk.II (extra)
        563,  // Johnston
        597,  // Johnston改
        692,  // Johnston Mk.II
        700,  // Samuel B.Roberts Mk.II
        911,  // Heywood L.Edwards改
        916,  // Richard P.Leary改
    ];
    if UNCONDITIONAL.contains(&sid) {
        return true;
    }

    // 2. Escort carriers (大鷹型改/改二, 加賀改二護, Gambier Bay Mk.II)
    //    Need ASW>=1 aircraft (艦攻/艦爆/回転翼/哨戒機)
    const ESCORT_CVE: &[i32] = &[
        529, // 大鷹改
        536, // 大鷹改二
        380, // 神鷹改
        521, // 神鷹改二
        381, // 雲鷹改
        539, // 雲鷹改二
        546, // 加賀改二護
        396, // 春日丸 → not escort carrier
        557, // Gambier Bay Mk.II
    ];
    if ESCORT_CVE.contains(&sid) {
        return has_asw1_aircraft;
    }

    // 3. 海防艦 (stype=1): ASW>=60+sonar OR ASW>=75+equip_asw>=4
    if stype == 1 {
        return (asw >= 60 && has_sonar) || (asw >= 75 && equip_asw_total >= 4);
    }

    // 4. 護衛空母/軽空母 (stype=7)
    //    鈴谷航改二(503), 熊野航改二(504)は除外
    if stype == 7 && sid != 503 && sid != 504 {
        // Pattern A: ASW>=50 + sonar + ASW>=7 aircraft
        if asw >= 50 && has_sonar && has_asw7_aircraft {
            return true;
        }
        // Pattern B: ASW>=65 + ASW>=7 aircraft
        if asw >= 65 && has_asw7_aircraft {
            return true;
        }
        // Pattern C: ASW>=100 + sonar + ASW>=1 bomber (艦攻/艦爆)
        if asw >= 100 && has_sonar && has_asw1_bomber {
            return true;
        }
        return false;
    }

    // 5. 日向改二 (ship_id=554): S-51J系1+ OR 回転翼(Ka号/O号)2+
    if sid == 554 {
        return s51j_count >= 1 || rotorcraft_count >= 2;
    }

    // 6. 航空戦艦 (stype=10): ASW>=100 + sonar + (水爆/回転翼/哨戒機/爆雷投射機/爆雷)
    if stype == 10 {
        let has_asw_equip = has_seaplane_bomber
            || rotorcraft_count > 0
            || s51j_count > 0
            || has_depth_charge_projector
            || has_depth_charge;
        return asw >= 100 && has_sonar && has_asw_equip;
    }

    // 7. General ships: DD(2), CL(3), CLT(4), CT(21), AO(22): ASW>=100 + sonar
    if stype == 2 || stype == 3 || stype == 4 || stype == 21 || stype == 22 {
        return asw >= 100 && has_sonar;
    }

    false
}

/// Build and emit fleet summaries to the frontend
fn emit_fleet_update(state: &models::GameStateInner, app: &AppHandle) {
    let fleets: Vec<models::FleetSummary> = state
        .profile.fleets
        .iter()
        .enumerate()
        .map(|(i, ship_ids)| {
            let ships: Vec<models::ShipSummary> = ship_ids
                .iter()
                .filter_map(|&id| {
                    state.profile.ships.get(&id).map(|info| {
                        let marks = collect_ship_marks(
                            info,
                            &state.profile.slotitems,
                            &state.master.slotitems,
                        );
                        models::ShipSummary {
                            id,
                            name: info.name.clone(),
                            lv: info.lv,
                            hp: info.hp,
                            maxhp: info.maxhp,
                            cond: info.cond,
                            fuel: info.fuel,
                            bull: info.bull,
                            damecon_name: marks.damecon_name,
                            special_equips: marks.special_equips,
                            can_opening_asw: marks.can_opening_asw,
                            soku: info.soku,
                        }
                    })
                })
                .collect();

            models::FleetSummary {
                id: (i + 1) as i32,
                name: format!("第{}艦隊", i + 1),
                ships,
                expedition: None, // Expedition info not available from hensei APIs
            }
        })
        .collect();

    match app.emit("fleet-updated", &fleets) {
        Ok(_) => info!("fleet-updated event emitted: {} fleets", fleets.len()),
        Err(e) => error!("Failed to emit fleet-updated: {}", e),
    }
}

/// Parse expedition info from a fleet's api_mission array.
fn parse_expedition(
    mission: &[serde_json::Value],
    master_missions: &std::collections::HashMap<i32, models::MissionInfo>,
) -> Option<models::ExpeditionInfo> {
    if mission.len() < 4 {
        return None;
    }

    let mission_type = mission[0].as_i64().unwrap_or(0);
    if mission_type == 0 {
        return None;
    }

    let mission_id = mission[1].as_i64().unwrap_or(0) as i32;
    // api_mission format: [type, mission_id, return_time, ?]
    let return_time = mission[2].as_i64().unwrap_or(0);

    let mission_name = master_missions
        .get(&mission_id)
        .map(|m| m.name.clone())
        .unwrap_or_else(|| {
            warn!("Unknown mission ID: {}", mission_id);
            format!("Mission {}", mission_id)
        });

    Some(models::ExpeditionInfo {
        mission_id,
        mission_name,
        return_time,
    })
}
