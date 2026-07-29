use chrono::{Datelike, FixedOffset, TimeZone, Timelike, Utc};
use std::sync::atomic::Ordering;
use tauri::Manager;

use crate::AppState;

const JST_OFFSET_SECONDS: i32 = 9 * 60 * 60;
const CHECK_INTERVAL_SECONDS: u64 = 15;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AlertStatus {
    minutes_remaining: i64,
}

fn alert_status(now_ms: i64, last_exercise_at_ms: i64) -> Option<AlertStatus> {
    let jst = FixedOffset::east_opt(JST_OFFSET_SECONDS)?;
    let now = Utc
        .timestamp_millis_opt(now_ms)
        .single()?
        .with_timezone(&jst);

    let end_hour = match (now.hour(), now.minute()) {
        (2, 45..=59) => 3,
        (14, 45..=59) => 15,
        _ => return None,
    };
    let end = jst
        .with_ymd_and_hms(now.year(), now.month(), now.day(), end_hour, 0, 0)
        .single()?;
    let start = end - chrono::Duration::hours(12);
    if last_exercise_at_ms >= start.timestamp_millis() && last_exercise_at_ms <= now_ms {
        return None;
    }

    let remaining_ms = end.timestamp_millis() - now_ms;
    Some(AlertStatus {
        minutes_remaining: ((remaining_ms + 59_999) / 60_000).clamp(1, 15),
    })
}

fn refresh(app: &tauri::AppHandle) {
    let last_exercise_at_ms = app
        .state::<AppState>()
        .overlay
        .last_exercise_at_ms
        .load(Ordering::Relaxed);
    match alert_status(Utc::now().timestamp_millis(), last_exercise_at_ms) {
        Some(status) => {
            if let Err(error) =
                crate::overlay::show_exercise_notification(app, status.minutes_remaining)
            {
                log::debug!("Exercise alert is not ready: {error}");
            }
        }
        None => crate::overlay::hide_exercise_notification(app),
    }
}

pub(crate) fn start(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut interval =
            tokio::time::interval(tokio::time::Duration::from_secs(CHECK_INTERVAL_SECONDS));
        loop {
            interval.tick().await;
            refresh(&app);
        }
    });
}

pub(crate) fn record_exercise(app: &tauri::AppHandle) {
    let completed_at_ms = Utc::now().timestamp_millis();
    app.state::<AppState>()
        .overlay
        .last_exercise_at_ms
        .store(completed_at_ms, Ordering::Relaxed);
    if let Err(error) =
        crate::settings::persist_json(app, crate::settings::LAST_EXERCISE_AT_MS, &completed_at_ms)
    {
        log::warn!("Failed to persist last exercise time: {error}");
    }
    crate::overlay::hide_exercise_notification(app);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jst_ms(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> i64 {
        FixedOffset::east_opt(JST_OFFSET_SECONDS)
            .unwrap()
            .with_ymd_and_hms(year, month, day, hour, minute, 0)
            .single()
            .unwrap()
            .timestamp_millis()
    }

    #[test]
    fn alerts_during_last_fifteen_minutes_before_three() {
        let status = alert_status(jst_ms(2026, 7, 27, 2, 45), 0).unwrap();
        assert_eq!(status.minutes_remaining, 15);
        let status = alert_status(jst_ms(2026, 7, 27, 2, 59), 0).unwrap();
        assert_eq!(status.minutes_remaining, 1);
    }

    #[test]
    fn alerts_during_last_fifteen_minutes_before_fifteen() {
        let status = alert_status(jst_ms(2026, 7, 27, 14, 45), 0).unwrap();
        assert_eq!(status.minutes_remaining, 15);
        assert!(alert_status(jst_ms(2026, 7, 27, 15, 0), 0).is_none());
    }

    #[test]
    fn does_not_alert_outside_warning_window() {
        assert!(alert_status(jst_ms(2026, 7, 27, 2, 44), 0).is_none());
        assert!(alert_status(jst_ms(2026, 7, 27, 14, 44), 0).is_none());
    }

    #[test]
    fn exercise_in_current_slot_suppresses_alert() {
        let morning_exercise = jst_ms(2026, 7, 27, 3, 5);
        assert!(alert_status(jst_ms(2026, 7, 27, 14, 50), morning_exercise).is_none());

        let evening_exercise = jst_ms(2026, 7, 26, 15, 5);
        assert!(alert_status(jst_ms(2026, 7, 27, 2, 50), evening_exercise).is_none());
    }

    #[test]
    fn exercise_before_current_slot_does_not_suppress_alert() {
        let stale_exercise = jst_ms(2026, 7, 27, 2, 59);
        assert!(alert_status(jst_ms(2026, 7, 27, 14, 50), stale_exercise).is_some());
    }
}
