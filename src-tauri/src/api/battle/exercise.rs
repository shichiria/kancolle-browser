use log::info;
use tauri::{AppHandle, Emitter};

use crate::api::{models, notify_sync};

pub(crate) fn process_exercise_result(
    state: &mut models::GameStateInner,
    data: &crate::api::dto::member::ApiExerciseResultResponse,
    app: &AppHandle,
) {
    info!("Exercise result: rank={}", data.api_win_rank);
    crate::practice_alert::record_exercise(app);
    if data.api_get_exp > 0 {
        state.senka.add_battle_exp(data.api_get_exp, "演習");
        let summary = state.senka.summary();
        let _ = app.emit(crate::events::SENKA_UPDATED, &summary);
        notify_sync(state, vec![crate::senka::SenkaTracker::sync_path()]);
    }

    let changed = crate::quest_progress::on_exercise_result(
        &mut state.history.quest_progress,
        &data.api_win_rank,
        &state.history.active_quests,
        &state.history.sortie_quest_defs,
        &state.quest_progress_path,
    );
    if changed {
        notify_sync(state, vec!["quest_progress.json"]);
        let path = state.quest_progress_path.clone();
        let definitions = state.history.sortie_quest_defs.clone();
        let active_quests = state.history.active_quests.clone();
        let progress = crate::quest_progress::get_active_progress(
            &mut state.history.quest_progress,
            &active_quests,
            &definitions,
            &path,
        );
        let _ = app.emit(crate::events::QUEST_PROGRESS_UPDATED, &progress);
    }
}
