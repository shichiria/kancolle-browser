use super::*;

fn test_tracker(name: &str) -> SenkaTracker {
    let dir = std::env::temp_dir().join("kc-senka-tests").join(format!(
        "{}-{}",
        name,
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    let mut t = SenkaTracker::new(&dir);
    // Establish the monthly baseline (first port of the month)
    t.update_experience(10_000);
    t
}

fn exp_entries(t: &SenkaTracker) -> Vec<i64> {
    t.data
        .entries
        .iter()
        .filter(|e| e.entry_type == "exp")
        .filter_map(|e| e.exp_gain)
        .collect()
}

#[test]
fn battle_exp_fully_covered_by_port_delta_adds_no_extra_entry() {
    let mut t = test_tracker("battle-only");
    t.add_battle_exp(150, "1-5");
    t.update_experience(10_150);
    assert_eq!(exp_entries(&t), vec![150]);
    assert_eq!(t.data.pending_battle_exp, 0);
}

#[test]
fn non_battle_gain_is_attributed_at_port() {
    let mut t = test_tracker("mixed");
    t.add_battle_exp(150, "1-5");
    // Port shows +500: 150 from battle, 350 from expeditions etc.
    t.update_experience(10_500);
    assert_eq!(exp_entries(&t), vec![150, 350]);
    assert_eq!(t.data.pending_battle_exp, 0);
    // Last "exp" entry is the port-attributed non-battle gain
    // (a checkpoint entry may follow it in `entries`)
    let detail = t
        .data
        .entries
        .iter()
        .rev()
        .find(|e| e.entry_type == "exp")
        .and_then(|e| e.detail.clone())
        .unwrap();
    assert!(detail.contains("非戦闘"), "detail: {}", detail);
}

#[test]
fn port_only_gain_is_recorded_in_full() {
    let mut t = test_tracker("port-only");
    t.update_experience(10_400);
    assert_eq!(exp_entries(&t), vec![400]);
}

#[test]
fn pending_exceeding_delta_resets_without_negative_entry() {
    let mut t = test_tracker("over-pending");
    t.add_battle_exp(300, "1-5");
    // Port delta (+100) is smaller than recorded battle exp — no extra entry
    t.update_experience(10_100);
    assert_eq!(exp_entries(&t), vec![300]);
    assert_eq!(t.data.pending_battle_exp, 0);
}
