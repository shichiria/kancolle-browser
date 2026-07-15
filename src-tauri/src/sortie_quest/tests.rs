use super::*;

#[test]
fn test_json_loads() {
    let quests = get_all_sortie_quests();
    assert!(
        quests.len() > 200,
        "Expected 200+ quests, got {}",
        quests.len()
    );
    let bm1 = quests.iter().find(|q| q.quest_id == "Bm1").unwrap();
    assert_eq!(bm1.name, "「第五戦隊」出撃せよ！");
    assert_eq!(bm1.area, "2-5");
    assert_eq!(bm1.reset, "monthly");

    // Check all reset types exist
    let daily = quests.iter().filter(|q| q.reset == "daily").count();
    let weekly = quests.iter().filter(|q| q.reset == "weekly").count();
    let monthly = quests.iter().filter(|q| q.reset == "monthly").count();
    let quarterly = quests.iter().filter(|q| q.reset == "quarterly").count();
    let yearly = quests.iter().filter(|q| q.reset == "yearly").count();
    let once = quests.iter().filter(|q| q.reset == "once").count();
    assert!(daily >= 5, "daily: {}", daily);
    assert!(weekly >= 8, "weekly: {}", weekly);
    assert!(monthly >= 6, "monthly: {}", monthly);
    assert!(quarterly >= 10, "quarterly: {}", quarterly);
    assert!(yearly >= 10, "yearly: {}", yearly);
    assert!(once >= 100, "once: {}", once);
}

#[test]
fn test_ship_name_match() {
    let fleet = FleetCheckData {
        ships: vec![
            FleetShipData {
                name: "那智改二".into(),
                ship_type: 5,
                level: 80,
            },
            FleetShipData {
                name: "妙高改二".into(),
                ship_type: 5,
                level: 75,
            },
            FleetShipData {
                name: "羽黒改二".into(),
                ship_type: 5,
                level: 70,
            },
            FleetShipData {
                name: "島風".into(),
                ship_type: 2,
                level: 60,
            },
        ],
    };
    let result = check_sortie_quest("Bm1", &fleet);
    assert!(result.satisfied);
}

#[test]
fn test_bm4_battleship_condition() {
    // Bm4: 大和型/長門型/伊勢型/扶桑型 3隻 + 軽巡1, other BBs prohibited
    let valid_fleet = FleetCheckData {
        ships: vec![
            FleetShipData {
                name: "大和改二重".into(),
                ship_type: 9,
                level: 99,
            },
            FleetShipData {
                name: "長門改二".into(),
                ship_type: 9,
                level: 90,
            },
            FleetShipData {
                name: "扶桑改二".into(),
                ship_type: 10,
                level: 85,
            },
            FleetShipData {
                name: "阿武隈改二".into(),
                ship_type: 3,
                level: 75,
            },
            FleetShipData {
                name: "島風".into(),
                ship_type: 2,
                level: 60,
            },
            FleetShipData {
                name: "雪風改二".into(),
                ship_type: 2,
                level: 70,
            },
        ],
    };
    let result = check_sortie_quest("Bm4", &valid_fleet);
    assert!(result.satisfied, "Valid Bm4 fleet should pass");

    // Invalid: 金剛型 (stype 8) should NOT count
    let invalid_fleet = FleetCheckData {
        ships: vec![
            FleetShipData {
                name: "金剛改二丙".into(),
                ship_type: 8,
                level: 99,
            },
            FleetShipData {
                name: "榛名改二".into(),
                ship_type: 8,
                level: 90,
            },
            FleetShipData {
                name: "霧島改二".into(),
                ship_type: 8,
                level: 85,
            },
            FleetShipData {
                name: "阿武隈改二".into(),
                ship_type: 3,
                level: 75,
            },
            FleetShipData {
                name: "島風".into(),
                ship_type: 2,
                level: 60,
            },
            FleetShipData {
                name: "雪風改二".into(),
                ship_type: 2,
                level: 70,
            },
        ],
    };
    let result = check_sortie_quest("Bm4", &invalid_fleet);
    assert!(!result.satisfied, "Kongou-class fleet should NOT pass Bm4");

    // Invalid: 4 BBs (exceeds MaxShipTypeCount of 3)
    let too_many_bbs = FleetCheckData {
        ships: vec![
            FleetShipData {
                name: "大和改二重".into(),
                ship_type: 9,
                level: 99,
            },
            FleetShipData {
                name: "武蔵改二".into(),
                ship_type: 9,
                level: 99,
            },
            FleetShipData {
                name: "長門改二".into(),
                ship_type: 9,
                level: 90,
            },
            FleetShipData {
                name: "金剛改二丙".into(),
                ship_type: 8,
                level: 85,
            },
            FleetShipData {
                name: "阿武隈改二".into(),
                ship_type: 3,
                level: 75,
            },
            FleetShipData {
                name: "島風".into(),
                ship_type: 2,
                level: 60,
            },
        ],
    };
    let result = check_sortie_quest("Bm4", &too_many_bbs);
    assert!(!result.satisfied, "4 BBs should NOT pass Bm4 (max 3)");
}

#[test]
fn test_bq13_or_conditions() {
    // Bq13: 旗艦夕張改二 + (六水戦DD×2 OR 由良改二)

    // Option A: 夕張改二 + 睦月 + 如月
    let option_a = FleetCheckData {
        ships: vec![
            FleetShipData {
                name: "夕張改二特".into(),
                ship_type: 3,
                level: 90,
            },
            FleetShipData {
                name: "睦月改二".into(),
                ship_type: 2,
                level: 70,
            },
            FleetShipData {
                name: "如月改二".into(),
                ship_type: 2,
                level: 70,
            },
            FleetShipData {
                name: "島風".into(),
                ship_type: 2,
                level: 60,
            },
            FleetShipData {
                name: "雪風改二".into(),
                ship_type: 2,
                level: 70,
            },
            FleetShipData {
                name: "時雨改三".into(),
                ship_type: 2,
                level: 80,
            },
        ],
    };
    let result = check_sortie_quest("Bq13", &option_a);
    assert!(result.satisfied, "Bq13 Option A (六水戦DD) should pass");

    // Option B: 夕張改二 + 由良改二
    let option_b = FleetCheckData {
        ships: vec![
            FleetShipData {
                name: "夕張改二".into(),
                ship_type: 3,
                level: 90,
            },
            FleetShipData {
                name: "由良改二".into(),
                ship_type: 3,
                level: 80,
            },
            FleetShipData {
                name: "島風".into(),
                ship_type: 2,
                level: 60,
            },
            FleetShipData {
                name: "雪風改二".into(),
                ship_type: 2,
                level: 70,
            },
            FleetShipData {
                name: "時雨改三".into(),
                ship_type: 2,
                level: 80,
            },
            FleetShipData {
                name: "秋月改".into(),
                ship_type: 2,
                level: 75,
            },
        ],
    };
    let result = check_sortie_quest("Bq13", &option_b);
    assert!(result.satisfied, "Bq13 Option B (由良改二) should pass");

    // Invalid: 夕張改二 but only random DDs (no 六水戦DD, no 由良改二)
    let invalid = FleetCheckData {
        ships: vec![
            FleetShipData {
                name: "夕張改二丁".into(),
                ship_type: 3,
                level: 90,
            },
            FleetShipData {
                name: "島風".into(),
                ship_type: 2,
                level: 60,
            },
            FleetShipData {
                name: "雪風改二".into(),
                ship_type: 2,
                level: 70,
            },
            FleetShipData {
                name: "時雨改三".into(),
                ship_type: 2,
                level: 80,
            },
            FleetShipData {
                name: "秋月改".into(),
                ship_type: 2,
                level: 75,
            },
            FleetShipData {
                name: "涼月改".into(),
                ship_type: 2,
                level: 70,
            },
        ],
    };
    let result = check_sortie_quest("Bq13", &invalid);
    assert!(!result.satisfied, "Bq13 with random DDs should NOT pass");
}

#[test]
fn test_bq2_sub_goals() {
    let quests = get_all_sortie_quests();
    let bq2 = quests.iter().find(|q| q.quest_id == "Bq2").unwrap();
    assert_eq!(bq2.sub_goals.len(), 4, "Bq2 should have 4 sub_goals");
    // 6-4 requires S rank
    let sg_64 = bq2.sub_goals.iter().find(|sg| sg.name == "6-4").unwrap();
    assert_eq!(sg_64.rank, "S");
    assert_eq!(sg_64.area.as_deref(), Some("6-4"));
    // Others require A rank
    let sg_24 = bq2.sub_goals.iter().find(|sg| sg.name == "2-4").unwrap();
    assert_eq!(sg_24.rank, "A");
}

#[test]
fn test_c23_c27_once() {
    let quests = get_all_sortie_quests();
    let c23 = quests.iter().find(|q| q.quest_id == "C23").unwrap();
    assert_eq!(c23.reset, "once", "C23 should be a one-time quest");
    let c27 = quests.iter().find(|q| q.quest_id == "C27").unwrap();
    assert_eq!(c27.reset, "once", "C27 should be a one-time quest");
}

#[test]
fn test_exercise_counter_reset() {
    let quests = get_all_sortie_quests();
    let ids = ["Cm1", "Cq1", "Cq2", "Cq3", "Cq4"];
    for id in ids {
        let q = quests.iter().find(|q| q.quest_id == id).unwrap();
        assert_eq!(
            q.counter_reset.as_deref(),
            Some("daily"),
            "{} should have counter_reset=daily",
            id
        );
    }
}

#[test]
fn test_existing_conditions_unchanged() {
    let quests = get_all_sortie_quests();

    // Bm1: 那智+妙高+羽黒 should still work
    let bm1 = quests.iter().find(|q| q.quest_id == "Bm1").unwrap();
    assert_eq!(bm1.conditions.len(), 1);
    assert_eq!(bm1.area, "2-5");
    assert_eq!(bm1.rank, "S");

    // Bm3: 旗艦軽巡 + 軽巡駆逐のみ
    let bm3 = quests.iter().find(|q| q.quest_id == "Bm3").unwrap();
    assert_eq!(bm3.conditions.len(), 2);

    // Bm6: 空母2 + 駆逐2
    let bm6 = quests.iter().find(|q| q.quest_id == "Bm6").unwrap();
    assert_eq!(bm6.conditions.len(), 2);

    // Bm7: 旗艦駆逐 + 重巡1 + 軽巡1 + 駆逐4
    let bm7 = quests.iter().find(|q| q.quest_id == "Bm7").unwrap();
    assert_eq!(bm7.conditions.len(), 4);

    // Bq6: 長波改二 + 高波改/沖波改/朝霜改
    let bq6 = quests.iter().find(|q| q.quest_id == "Bq6").unwrap();
    assert_eq!(bq6.conditions.len(), 2);

    // Bq7: 三川艦隊 4隻
    let bq7 = quests.iter().find(|q| q.quest_id == "Bq7").unwrap();
    assert_eq!(bq7.conditions.len(), 1);
}

#[test]
fn test_map_recommendations_json_loads() {
    let recs = get_all_map_recommendations();
    assert!(recs.len() >= 20, "Expected 20+ maps, got {}", recs.len());
    let map_1_1 = recs.iter().find(|r| r.area == "1-1").unwrap();
    assert_eq!(map_1_1.name, "鎮守府正面海域");
    assert!(!map_1_1.routes.is_empty());
}

#[test]
fn test_map_recommendation_check() {
    // Fleet matching 2-5 second route: 6 ships, 3DD, 1CL, no BB/CV
    let fleet = FleetCheckData {
        ships: vec![
            FleetShipData {
                name: "那智改二".into(),
                ship_type: 5,
                level: 80,
            },
            FleetShipData {
                name: "妙高改二".into(),
                ship_type: 5,
                level: 75,
            },
            FleetShipData {
                name: "神通改二".into(),
                ship_type: 3,
                level: 70,
            },
            FleetShipData {
                name: "島風".into(),
                ship_type: 2,
                level: 60,
            },
            FleetShipData {
                name: "雪風".into(),
                ship_type: 2,
                level: 65,
            },
            FleetShipData {
                name: "時雨改二".into(),
                ship_type: 2,
                level: 70,
            },
        ],
    };
    let result = check_map_recommendation("2-5", &fleet);
    assert_eq!(result.area, "2-5");
    assert_eq!(result.name, "沖ノ島沖");
    assert!(result.routes.len() >= 2);
    // Second route (水上): 3DD + 1CL, no BB/CV -> satisfied
    assert!(result.routes[1].satisfied);
}
