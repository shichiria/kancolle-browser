use super::{dto, models};
use log::info;
use tauri::{AppHandle, Emitter};

pub(super) fn process_questlist(
    state: &mut models::GameStateInner,
    data: &dto::member::ApiQuestListResponse,
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
                    // Accepted or completed -> add to active set
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
                    // Not accepted -> remove from active set
                    state.history.active_quests.remove(&api_no);
                    state.history.active_quest_details.remove(&api_no);
                }
                _ => {}
            }
        }

        let details: Vec<&models::ActiveQuestDetail> =
            state.history.active_quest_details.values().collect();
        info!("Active quests updated: {} quests", details.len());
        let _ = app.emit(crate::events::QUEST_LIST_UPDATED, &details);
    }
}
