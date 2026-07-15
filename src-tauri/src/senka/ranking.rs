use log::{info, warn};
use serde::Serialize;

// =============================================================================
// Ranking API decryption (ported from EO SenkaLeaderboardViewModel.cs)
// =============================================================================

/// Key table for ranking decryption (13 values, indexed by rank % 13)
const POSSIBLE_RANK: [i64; 13] = [
    8931, 1201, 1156, 5061, 4569, 4732, 3779, 4568, 5695, 4619, 4912, 5669, 6586,
];

/// A decrypted ranking entry
#[derive(Debug, Clone, Serialize)]
pub struct RankingEntry {
    pub position: i32,
    pub admiral_name: String,
    pub senka: i64,
    pub medal_count: i32,
    pub comment: String,
}

/// Check if a candidate user_key produces an integer senka >= 0
fn check_rate(key: i64, user_key: i64, rate: f64) -> bool {
    let points = rate / (key as f64) / (user_key as f64) - 91.0;
    points >= 0.0 && (points - points.floor()).abs() < 1e-6
}

/// Decrypt ranking entries from the typed API response.
/// Returns (decoded entries, user's own senka if found).
pub fn decrypt_ranking(
    ranking_data: &crate::api::dto::ranking::ApiRankingResponse,
    admiral_name: &str,
) -> (Vec<RankingEntry>, Option<i64>) {
    let entries = &ranking_data.api_list;

    // Phase 1: Narrow down possible user keys using all entries
    let mut possible_user_keys: Vec<i64> = Vec::new();

    for entry in entries {
        let position = entry.api_mxltvkpyuklh.unwrap_or(0);
        let rate = entry.api_wuhnhojjxmke.unwrap_or(0.0);

        if position <= 0 || rate <= 0.0 {
            continue;
        }

        let key = POSSIBLE_RANK[(position % 13) as usize];

        if possible_user_keys.is_empty() {
            // First entry: try all keys 10-99
            for uk in 10..100 {
                if check_rate(key, uk, rate) {
                    possible_user_keys.push(uk);
                }
            }
        } else {
            // Subsequent entries: filter down
            possible_user_keys.retain(|&uk| check_rate(key, uk, rate));
        }
    }

    if possible_user_keys.is_empty() {
        warn!("Senka: could not determine user key for ranking decryption");
        return (vec![], None);
    }

    let user_key = *possible_user_keys.last().unwrap();
    info!(
        "Senka: ranking user_key determined: {} (from {} candidates)",
        user_key,
        possible_user_keys.len()
    );

    // Phase 2: Decrypt all entries
    let mut decoded = Vec::new();
    let mut own_senka = None;

    for entry in entries {
        let position = entry.api_mxltvkpyuklh.unwrap_or(0) as i32;
        let name = entry.api_mtjmdcwtvhdr.as_deref().unwrap_or("").to_string();
        let rate = entry.api_wuhnhojjxmke.unwrap_or(0.0);
        let medal_enc = entry.api_itslcqtmrxtf.unwrap_or(0);
        let comment = entry.api_itbrdpdbkynm.as_deref().unwrap_or("").to_string();

        if position <= 0 {
            continue;
        }

        let key = POSSIBLE_RANK[(position as i64 % 13) as usize];
        let senka = (rate / (key as f64) / (user_key as f64)).floor() as i64 - 91;
        let medal_count = (medal_enc / (key + 1853)) as i32 - 157;

        let re = RankingEntry {
            position,
            admiral_name: name.clone(),
            senka: senka.max(0),
            medal_count: medal_count.max(0),
            comment,
        };

        // Check if this is our admiral
        if name == admiral_name {
            info!(
                "Senka: found own entry at rank {} with senka {}",
                position, senka
            );
            own_senka = Some(senka.max(0));
        }

        decoded.push(re);
    }

    info!(
        "Senka: decoded {} ranking entries, own senka: {:?}",
        decoded.len(),
        own_senka
    );
    (decoded, own_senka)
}
