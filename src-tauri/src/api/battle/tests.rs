use super::{
    calculate_provisional_gauge_hp, escape_ship_ids, is_taiha_warning_exempt,
};
use serde_json::json;
use std::collections::HashSet;

#[test]
fn estimates_destruction_gauge_from_boss_damage() {
    assert_eq!(
        calculate_provisional_gauge_hp(Some(4410), Some(2), 5, 980, 0),
        Some(3430)
    );
    assert_eq!(
        calculate_provisional_gauge_hp(Some(4410), Some(3), 5, 980, 0),
        None,
        "transport gauges must wait for the authoritative API value"
    );
    assert_eq!(
        calculate_provisional_gauge_hp(Some(4410), Some(2), 4, 980, 0),
        None,
        "non-boss battles must not change the gauge"
    );
}

#[test]
fn resolves_retreat_ship_from_recorded_sortie_result() {
    let fleets = vec![
        vec![],
        vec![],
        vec![79689, 101, 138361, 44105, 7310, 8906, 21292],
    ];
    let api_data = json!({
        "api_escape": {
            "api_escape_idx": [4],
            "api_escape_type": 1
        }
    });

    assert_eq!(
        escape_ship_ids(&fleets, &[2], &api_data),
        HashSet::from([44105])
    );
}

#[test]
fn resolves_escape_and_tow_positions_across_combined_fleets() {
    let fleets = vec![vec![11, 12, 13], vec![21, 22, 23]];
    let api_data = json!({
        "api_escape": {
            "api_escape_idx": [2],
            "api_tow_idx": [8]
        }
    });

    assert_eq!(
        escape_ship_ids(&fleets, &[0, 1], &api_data),
        HashSet::from([12, 22])
    );
}

#[test]
fn ignores_missing_and_out_of_range_escape_positions() {
    let fleets = vec![vec![11, 12]];
    let api_data = json!({
        "api_escape": {
            "api_escape_idx": [0, 3]
        }
    });

    assert!(escape_ship_ids(&fleets, &[0], &api_data).is_empty());
    assert!(escape_ship_ids(&fleets, &[0], &json!({})).is_empty());
}

#[test]
fn combined_escort_flagship_is_exempt_from_taiha_warning() {
    assert!(is_taiha_warning_exempt(true, 1, 0));
}

#[test]
fn taiha_warning_still_applies_to_every_other_position() {
    assert!(!is_taiha_warning_exempt(true, 0, 0));
    assert!(!is_taiha_warning_exempt(true, 1, 1));
    assert!(!is_taiha_warning_exempt(false, 1, 0));
}
