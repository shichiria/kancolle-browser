use chrono::{Datelike, FixedOffset, TimeZone, Timelike};
use log::{info, warn};
use std::path::Path;

use super::{jst, now_jst, SenkaData};

pub(super) fn current_ranking_month(now: &chrono::DateTime<FixedOffset>) -> String {
    let last_day = last_day_of_month(now.year(), now.month());
    if now.day() == last_day && now.hour() >= 22 {
        return if now.month() == 12 {
            format!("{}-01", now.year() + 1)
        } else {
            format!("{}-{:02}", now.year(), now.month() + 1)
        };
    }
    format!("{}-{:02}", now.year(), now.month())
}

pub(super) fn load_senka_data(path: &Path) -> SenkaData {
    match std::fs::read_to_string(path) {
        Ok(contents) => match serde_json::from_str::<SenkaData>(&contents) {
            Ok(data) => {
                let current_month = current_ranking_month(&now_jst());
                if data.month == current_month {
                    info!("Senka: loaded data for {current_month}");
                    data
                } else {
                    info!(
                        "Senka: saved data for {} but current month is {}, starting fresh",
                        data.month, current_month
                    );
                    SenkaData::default()
                }
            }
            Err(error) => {
                warn!("Senka: failed to parse senka_log.json: {error}");
                SenkaData::default()
            }
        },
        Err(_) => {
            info!("Senka: no existing senka_log.json, starting fresh");
            SenkaData::default()
        }
    }
}

pub(super) fn save_senka_data(path: &Path, data: &SenkaData) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match serde_json::to_string_pretty(data) {
        Ok(json) => {
            if let Err(error) = std::fs::write(path, json) {
                warn!("Senka: failed to write senka_log.json: {error}");
            }
        }
        Err(error) => warn!("Senka: failed to serialize senka data: {error}"),
    }
}

pub(super) fn get_recent_checkpoints(
    now: &chrono::DateTime<FixedOffset>,
) -> Vec<chrono::DateTime<FixedOffset>> {
    let mut checkpoints = Vec::new();
    let today_3 = jst()
        .with_ymd_and_hms(now.year(), now.month(), now.day(), 3, 0, 0)
        .single();
    let today_15 = jst()
        .with_ymd_and_hms(now.year(), now.month(), now.day(), 15, 0, 0)
        .single();
    if let Some(checkpoint) = today_3 {
        checkpoints.push(checkpoint);
    }
    if let Some(checkpoint) = today_15 {
        checkpoints.push(checkpoint);
    }

    let yesterday = *now - chrono::Duration::days(1);
    if let Some(checkpoint) = jst()
        .with_ymd_and_hms(
            yesterday.year(),
            yesterday.month(),
            yesterday.day(),
            15,
            0,
            0,
        )
        .single()
    {
        checkpoints.push(checkpoint);
    }
    checkpoints.sort();
    checkpoints
}

pub(super) fn next_checkpoint_iso() -> String {
    let now = now_jst();
    let today_3 = jst()
        .with_ymd_and_hms(now.year(), now.month(), now.day(), 3, 0, 0)
        .single();
    let today_15 = jst()
        .with_ymd_and_hms(now.year(), now.month(), now.day(), 15, 0, 0)
        .single();
    if let Some(checkpoint) = today_3 {
        if now < checkpoint {
            return checkpoint.to_rfc3339();
        }
    }
    if let Some(checkpoint) = today_15 {
        if now < checkpoint {
            return checkpoint.to_rfc3339();
        }
    }

    let tomorrow = now + chrono::Duration::days(1);
    jst()
        .with_ymd_and_hms(tomorrow.year(), tomorrow.month(), tomorrow.day(), 3, 0, 0)
        .single()
        .map(|checkpoint| checkpoint.to_rfc3339())
        .unwrap_or_default()
}

pub(super) fn ranking_data_cutoff(
    now: &chrono::DateTime<FixedOffset>,
) -> chrono::DateTime<FixedOffset> {
    if now.hour() >= 15 {
        jst()
            .with_ymd_and_hms(now.year(), now.month(), now.day(), 14, 0, 0)
            .single()
            .expect("valid JST ranking cutoff")
    } else if now.hour() >= 3 {
        jst()
            .with_ymd_and_hms(now.year(), now.month(), now.day(), 2, 0, 0)
            .single()
            .expect("valid JST ranking cutoff")
    } else {
        let yesterday = *now - chrono::Duration::days(1);
        jst()
            .with_ymd_and_hms(
                yesterday.year(),
                yesterday.month(),
                yesterday.day(),
                14,
                0,
                0,
            )
            .single()
            .expect("valid JST ranking cutoff")
    }
}

pub(super) fn is_eo_cutoff(now: &chrono::DateTime<FixedOffset>) -> bool {
    now.day() == last_day_of_month(now.year(), now.month()) && now.hour() >= 22
}

pub(super) fn is_quest_cutoff(now: &chrono::DateTime<FixedOffset>) -> bool {
    now.day() == last_day_of_month(now.year(), now.month()) && now.hour() >= 14
}

fn last_day_of_month(year: i32, month: u32) -> u32 {
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    chrono::NaiveDate::from_ymd_opt(next_year, next_month, 1)
        .and_then(|date| date.pred_opt())
        .map(|date| date.day())
        .unwrap_or(28)
}
