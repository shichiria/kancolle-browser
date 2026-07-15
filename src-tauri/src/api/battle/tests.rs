use super::escape_ship_ids;
use serde_json::json;
use std::collections::HashSet;

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
