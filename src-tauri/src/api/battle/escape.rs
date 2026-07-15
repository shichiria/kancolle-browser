use std::collections::HashSet;

/// Resolve the 1-based fleet positions returned in battleresult.api_escape to
/// player ship instance IDs. Combined fleets reserve six API slots per fleet.
pub(super) fn ship_ids(
    fleets: &[Vec<i32>],
    fleet_indices: &[usize],
    api_data: &serde_json::Value,
) -> HashSet<i32> {
    let ordered_ship_ids: Vec<Option<i32>> = if fleet_indices.len() > 1 {
        fleet_indices
            .iter()
            .flat_map(|&index| {
                fleets
                    .get(index)
                    .into_iter()
                    .flatten()
                    .copied()
                    .map(Some)
                    .chain(std::iter::repeat(None))
                    .take(6)
            })
            .collect()
    } else {
        fleet_indices
            .iter()
            .filter_map(|&index| fleets.get(index))
            .flatten()
            .copied()
            .map(Some)
            .collect()
    };

    let mut result = HashSet::new();
    let Some(escape) = api_data.get("api_escape") else {
        return result;
    };
    for field in ["api_escape_idx", "api_tow_idx"] {
        let Some(indices) = escape.get(field).and_then(|value| value.as_array()) else {
            continue;
        };
        for index in indices {
            let Some(position) = index.as_u64().and_then(|value| usize::try_from(value).ok())
            else {
                continue;
            };
            if let Some(&ship_id) = position
                .checked_sub(1)
                .and_then(|index| ordered_ship_ids.get(index))
                .and_then(Option::as_ref)
            {
                result.insert(ship_id);
            }
        }
    }
    result
}
