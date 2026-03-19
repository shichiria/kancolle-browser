use super::*;

/// Helper: load fixture from samples/ directory and extract response_body
fn load_fixture(filename: &str) -> serde_json::Value {
    let path = format!("tests/fixtures/samples/{}", filename);
    let json_str =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("Failed to read {}: {}", path, e));
    serde_json::from_str(&json_str).expect("Failed to parse fixture JSON")
}

/// Helper: extract response_body from fixture wrapper
fn response_body(fixture: &serde_json::Value) -> &serde_json::Value {
    fixture
        .get("response_body")
        .expect("Missing response_body in fixture")
}

/// Helper: extract api_data from response_body
fn api_data(fixture: &serde_json::Value) -> &serde_json::Value {
    response_body(fixture)
        .get("api_data")
        .expect("Missing api_data in response_body")
}

/// Helper: extract request_body string from fixture wrapper
fn request_body_str(fixture: &serde_json::Value) -> &str {
    fixture
        .get("request_body")
        .and_then(|v| v.as_str())
        .unwrap_or("")
}

// =============================================================================
// Suite 1: API Deserialization Tests — Category A (処理済みAPI)
// =============================================================================

#[cfg(test)]
mod a01_start2 {
    use super::*;

    #[test]
    fn test_parse_api_start2() {
        let fixture = load_fixture("api_start2_getData.json");
        let parsed: models::ApiResponse<models::ApiStart2> =
            serde_json::from_value(response_body(&fixture).clone())
                .expect("Failed to parse ApiStart2");

        assert_eq!(parsed.api_result, 1);
        let data = parsed.api_data.expect("api_data should exist");

        assert!(
            data.api_mst_ship.len() > 100,
            "Should have many master ships, got {}",
            data.api_mst_ship.len()
        );
        assert!(
            data.api_mst_slotitem.len() > 100,
            "Should have many master slot items, got {}",
            data.api_mst_slotitem.len()
        );
        assert!(
            data.api_mst_stype.len() > 10,
            "Should have many ship types, got {}",
            data.api_mst_stype.len()
        );
        assert!(
            data.api_mst_mission.len() > 10,
            "Should have many missions, got {}",
            data.api_mst_mission.len()
        );
        assert!(
            data.api_mst_slotitem_equiptype.len() > 10,
            "Should have equip types, got {}",
            data.api_mst_slotitem_equiptype.len()
        );

        // Verify a ship has required fields
        let ship = &data.api_mst_ship[0];
        assert!(ship.api_id > 0, "Ship should have positive ID");
        assert!(!ship.api_name.is_empty(), "Ship should have a name");
        assert!(ship.api_stype > 0, "Ship should have a stype");

        // Verify a slot item has required fields
        let item = &data.api_mst_slotitem[0];
        assert!(item.api_id > 0, "SlotItem should have positive ID");
        assert!(!item.api_name.is_empty(), "SlotItem should have a name");
    }
}

#[cfg(test)]
mod a02_port {
    use super::*;

    #[test]
    fn test_parse_api_port() {
        let fixture = load_fixture("api_port_port.json");
        let parsed: models::ApiResponse<models::ApiPort> =
            serde_json::from_value(response_body(&fixture).clone())
                .expect("Failed to parse ApiPort");

        assert_eq!(parsed.api_result, 1);
        let data = parsed.api_data.expect("api_data should exist");

        // Ships
        assert!(
            data.api_ship.len() > 0,
            "Should have at least one ship"
        );
        let ship = &data.api_ship[0];
        assert!(ship.api_id > 0, "Ship instance ID should be positive");
        assert!(ship.api_ship_id > 0, "Ship master ID should be positive");
        assert!(ship.api_lv > 0, "Ship level should be positive");
        assert!(ship.api_maxhp > 0, "Ship maxhp should be positive");

        // Fleets
        assert!(
            data.api_deck_port.len() >= 1,
            "Should have at least one fleet"
        );
        let fleet = &data.api_deck_port[0];
        assert_eq!(fleet.api_id, 1, "First fleet should have id=1");
        assert!(!fleet.api_name.is_empty(), "Fleet should have a name");
        assert!(fleet.api_ship.len() > 0, "Fleet should have ships");

        // Materials
        assert!(
            data.api_material.len() >= 4,
            "Should have at least 4 materials"
        );
        for id in 1..=4 {
            assert!(
                data.api_material.iter().any(|m| m.api_id == id),
                "Material ID {} should exist",
                id
            );
        }

        // Repair docks
        assert_eq!(data.api_ndock.len(), 4, "Should have 4 repair docks");

        // Admiral basic
        assert!(data.api_basic.api_level > 0, "Admiral level should be positive");
    }

    #[test]
    fn test_get_material_helper() {
        let materials: Vec<models::Material> = serde_json::from_value(serde_json::json!([
            {"api_id": 1, "api_value": 100},
            {"api_id": 2, "api_value": 200}
        ]))
        .expect("Failed to create test materials");

        assert_eq!(get_material(&materials, 1), 100);
        assert_eq!(get_material(&materials, 2), 200);
        assert_eq!(get_material(&materials, 99), 0, "Missing material should return 0");
    }
}

#[cfg(test)]
mod a03_slot_item {
    use super::*;

    #[test]
    fn test_parse_slot_item() {
        let fixture = load_fixture("api_get_member_slot_item.json");
        let parsed: models::ApiResponse<Vec<models::PlayerSlotItemApi>> =
            serde_json::from_value(response_body(&fixture).clone())
                .expect("Failed to parse slot_item");

        assert_eq!(parsed.api_result, 1);
        let items = parsed.api_data.expect("api_data should exist");

        assert!(
            items.len() > 100,
            "Should have many equipment items, got {}",
            items.len()
        );

        let item = &items[0];
        assert!(item.api_id > 0, "Item instance ID should be positive");
        assert!(
            item.api_slotitem_id > 0,
            "Item master ID should be positive"
        );
        assert!(item.api_level >= 0, "Level should be non-negative");
    }
}

#[cfg(test)]
mod a04_require_info {
    use super::*;

    #[test]
    fn test_parse_require_info_slot_item() {
        let fixture = load_fixture("api_get_member_require_info.json");
        let parsed: models::ApiResponse<serde_json::Value> =
            serde_json::from_value(response_body(&fixture).clone())
                .expect("Failed to parse require_info");

        let data = parsed.api_data.expect("api_data should exist");
        let slot_item_val = data
            .get("api_slot_item")
            .expect("require_info should contain api_slot_item");

        let items: Vec<models::PlayerSlotItemApi> =
            serde_json::from_value(slot_item_val.clone())
                .expect("Failed to parse api_slot_item from require_info");

        assert!(
            items.len() > 100,
            "Should have many equipment items, got {}",
            items.len()
        );

        let item = &items[0];
        assert!(item.api_id > 0);
        assert!(item.api_slotitem_id > 0);
    }
}

#[cfg(test)]
mod a05_questlist {
    use super::*;

    #[test]
    fn test_parse_questlist() {
        let fixture = load_fixture("api_get_member_questlist.json");
        let parsed: models::ApiResponse<dto::member::ApiQuestListResponse> =
            serde_json::from_value(response_body(&fixture).clone())
                .expect("Failed to parse questlist");

        assert_eq!(parsed.api_result, 1);
        let data = parsed.api_data.expect("api_data should exist");
        let list = data.api_list.expect("api_list should exist");

        assert!(!list.is_empty(), "Quest list should not be empty");

        // At least one real quest (not null/0)
        let quest_count = list
            .iter()
            .filter(|item| item.get("api_no").is_some())
            .count();
        assert!(quest_count > 0, "Should have at least one quest object");

        // Verify quest structure
        let quest = list
            .iter()
            .find(|item| item.get("api_no").is_some())
            .expect("Should have a quest");
        assert!(quest.get("api_state").is_some(), "Quest should have api_state");
        assert!(quest.get("api_title").is_some(), "Quest should have api_title");
        assert!(quest.get("api_category").is_some(), "Quest should have api_category");
    }
}

#[cfg(test)]
mod a06_ship3 {
    use super::*;

    #[test]
    fn test_parse_ship3() {
        let fixture = load_fixture("api_get_member_ship3.json");
        let parsed: models::ApiResponse<dto::member::ApiShip3Response> =
            serde_json::from_value(response_body(&fixture).clone())
                .expect("Failed to parse ship3");

        assert_eq!(parsed.api_result, 1);
        let data = parsed.api_data.expect("api_data should exist");

        assert!(
            !data.api_ship_data.is_empty(),
            "ship_data should have at least one ship"
        );
        assert!(
            !data.api_deck_data.is_empty(),
            "deck_data should have at least one fleet"
        );

        // Verify ship_data has expected fields
        let ship = &data.api_ship_data[0];
        assert!(ship.api_id > 0, "Ship should have positive api_id");
        assert!(ship.api_ship_id > 0, "Ship should have positive api_ship_id");
        assert!(ship.api_lv > 0, "Ship should have positive api_lv");

        // Verify deck_data has expected fields
        let deck = &data.api_deck_data[0];
        assert!(deck.api_id > 0, "Deck should have positive api_id");
        assert!(!deck.api_ship.is_empty(), "Deck should have ships");
    }
}

#[cfg(test)]
mod a07_hensei_change {
    use super::*;

    #[test]
    fn test_parse_hensei_change_request() {
        let fixture = load_fixture("api_req_hensei_change.json");
        let req_body = request_body_str(&fixture);

        let req: dto::request::HenseiChangeReq =
            serde_urlencoded::from_str(req_body).expect("Failed to parse hensei change request");

        assert!(req.api_id >= 1 && req.api_id <= 4, "Fleet ID should be 1-4, got {}", req.api_id);
        assert!(req.api_ship_idx >= -1, "Ship index should be >= -1");
    }
}

#[cfg(test)]
mod a08_preset_select {
    use super::*;

    #[test]
    fn test_parse_preset_select() {
        let fixture = load_fixture("api_req_hensei_preset_select.json");
        let parsed: models::ApiResponse<dto::member::ApiHenseiPresetSelectResponse> =
            serde_json::from_value(response_body(&fixture).clone())
                .expect("Failed to parse preset_select");

        assert_eq!(parsed.api_result, 1);
        let data = parsed.api_data.expect("api_data should exist");
        assert!(data.api_id >= 1, "Fleet ID should be >= 1");
    }

    #[test]
    fn test_preset_select_has_ship_array() {
        let fixture = load_fixture("api_req_hensei_preset_select.json");
        let parsed: models::ApiResponse<dto::member::ApiHenseiPresetSelectResponse> =
            serde_json::from_value(response_body(&fixture).clone())
                .expect("Failed to parse preset_select");

        let data = parsed.api_data.expect("api_data should exist");
        assert!(
            !data.api_ship.is_empty(),
            "Preset should have at least one ship"
        );
    }
}

#[cfg(test)]
mod a09_remodel_slot {
    use super::*;

    #[test]
    fn test_parse_remodel_slot_response() {
        let fixture = load_fixture("api_req_kousyou_remodel_slot.json");
        let parsed: models::ApiResponse<dto::member::ApiRemodelSlotResponse> =
            serde_json::from_value(response_body(&fixture).clone())
                .expect("Failed to parse remodel_slot");

        assert_eq!(parsed.api_result, 1);
        let data = parsed.api_data.expect("api_data should exist");

        let flag = data.api_remodel_flag.expect("Should have remodel_flag");
        assert!(flag == 0 || flag == 1, "remodel_flag should be 0 or 1, got {}", flag);

        if flag == 1 {
            let after_slot = data.api_after_slot.as_ref().expect("Success should have api_after_slot");
            assert!(
                after_slot.api_slotitem_id.is_some(),
                "api_after_slot should have api_slotitem_id"
            );
        }
    }

    #[test]
    fn test_parse_remodel_slot_request() {
        let fixture = load_fixture("api_req_kousyou_remodel_slot.json");
        let req_body = request_body_str(&fixture);

        let req: dto::request::RemodelSlotReq =
            serde_urlencoded::from_str(req_body).expect("Failed to parse remodel request");

        assert!(req.api_slot_id > 0, "Slot ID should be positive");
        assert!(req.api_id > 0, "Equipment master ID should be positive");
    }
}

#[cfg(test)]
mod a10_quest_start_stop {
    use super::*;

    #[test]
    fn test_parse_quest_start() {
        let fixture = load_fixture("api_req_quest_start.json");
        let req_body = request_body_str(&fixture);

        let req: dto::request::QuestReq =
            serde_urlencoded::from_str(req_body).expect("Failed to parse quest start request");

        assert!(req.api_quest_id > 0, "Quest ID should be positive");
    }

    #[test]
    fn test_parse_quest_stop() {
        let fixture = load_fixture("api_req_quest_stop.json");
        let req_body = request_body_str(&fixture);

        let req: dto::request::QuestReq =
            serde_urlencoded::from_str(req_body).expect("Failed to parse quest stop request");

        assert!(req.api_quest_id > 0, "Quest ID should be positive");
    }
}

#[cfg(test)]
mod a12_quest_clearitemget {
    use super::*;

    #[test]
    fn test_parse_clearitemget() {
        let fixture = load_fixture("api_req_quest_clearitemget.json");
        let req_body = request_body_str(&fixture);

        let req: dto::request::QuestReq =
            serde_urlencoded::from_str(req_body).expect("Failed to parse clearitemget request");
        assert!(req.api_quest_id > 0, "Quest ID should be positive");

        let data = api_data(&fixture);
        assert!(
            data.get("api_bounus").is_some(),
            "clearitemget should have api_bounus"
        );
    }

    #[test]
    fn test_extract_senka_from_clearitemget() {
        let fixture = load_fixture("api_req_quest_clearitemget.json");
        let response = serde_json::to_string(response_body(&fixture))
            .expect("Failed to serialize response_body");

        let bonus = extract_senka_from_clearitemget(&response);
        assert!(bonus >= 0, "Senka bonus should be non-negative, got {}", bonus);
    }
}

#[cfg(test)]
mod a13_practice_result {
    use super::*;

    #[test]
    fn test_parse_practice_battle_result() {
        let fixture = load_fixture("api_req_practice_battle_result.json");
        let parsed: models::ApiResponse<dto::member::ApiExerciseResultResponse> =
            serde_json::from_value(response_body(&fixture).clone())
                .expect("Failed to parse practice battle result");

        assert_eq!(parsed.api_result, 1);
        let data = parsed.api_data.expect("api_data should exist");

        assert!(
            ["S", "A", "B", "C", "D", "E"].contains(&data.api_win_rank.as_str()),
            "Invalid rank: {}",
            data.api_win_rank
        );
        assert!(data.api_get_exp >= 0, "Experience should be non-negative");
    }
}

#[cfg(test)]
mod a14_slot_deprive {
    use super::*;

    #[test]
    fn test_parse_slot_deprive() {
        let fixture = load_fixture("api_req_kaisou_slot_deprive.json");
        let parsed: models::ApiResponse<dto::member::ApiSlotDepriveResponse> =
            serde_json::from_value(response_body(&fixture).clone())
                .expect("Failed to parse slot_deprive");

        assert_eq!(parsed.api_result, 1);
        let data = parsed.api_data.expect("api_data should exist");

        assert!(
            data.api_ship_data.api_set_ship.api_id > 0,
            "Set ship should have valid id"
        );
        assert!(
            data.api_ship_data.api_unset_ship.api_id > 0,
            "Unset ship should have valid id"
        );
    }
}

#[cfg(test)]
mod a15_ranking {
    use super::*;

    #[test]
    fn test_parse_ranking() {
        let fixture = load_fixture("api_req_ranking_mxltvkpyuklh.json");
        let parsed: models::ApiResponse<dto::ranking::ApiRankingResponse> =
            serde_json::from_value(response_body(&fixture).clone())
                .expect("Failed to parse ranking");

        assert_eq!(parsed.api_result, 1);
        let data = parsed.api_data.expect("api_data should exist");

        assert!(
            data.api_count.unwrap_or(0) > 0,
            "Ranking count should be positive"
        );
        assert!(!data.api_list.is_empty(), "Ranking list should not be empty");

        // Verify entry has encrypted fields
        let entry = &data.api_list[0];
        assert!(
            entry.api_mxltvkpyuklh.is_some(),
            "Entry should have position field"
        );
    }
}

// =============================================================================
// Battle API Tests (A16-A35)
// =============================================================================

#[cfg(test)]
mod a16_sortie_battle {
    use super::*;

    #[test]
    fn test_parse_day_battle() {
        let fixture = load_fixture("api_req_sortie_battle.json");
        let parsed: models::ApiResponse<dto::battle::ApiBattleResponse> =
            serde_json::from_value(response_body(&fixture).clone())
                .expect("Failed to parse sortie battle");

        assert_eq!(parsed.api_result, 1);
        let data = parsed.api_data.expect("api_data should exist");

        // Formation
        let formation = data.api_formation.as_ref().expect("Should have formation");
        assert_eq!(formation.len(), 3, "Formation should have 3 elements [friend, enemy, engagement]");

        // Enemy ships
        let enemy_ships = data.api_ship_ke.as_ref().expect("Should have api_ship_ke");
        assert!(!enemy_ships.is_empty(), "Should have enemy ships");

        // HP arrays
        let f_nowhps = data.api_f_nowhps.as_ref().expect("Should have api_f_nowhps");
        let f_maxhps = data.api_f_maxhps.as_ref().expect("Should have api_f_maxhps");
        let e_nowhps = data.api_e_nowhps.as_ref().expect("Should have api_e_nowhps");
        let e_maxhps = data.api_e_maxhps.as_ref().expect("Should have api_e_maxhps");

        assert!(!f_nowhps.is_empty(), "Friend HP should not be empty");
        assert_eq!(f_nowhps.len(), f_maxhps.len(), "Friend HP arrays should match length");
        assert_eq!(e_nowhps.len(), e_maxhps.len(), "Enemy HP arrays should match length");
    }

    #[test]
    fn test_battle_hougeki_structure() {
        let fixture = load_fixture("api_req_sortie_battle.json");
        let data = api_data(&fixture);

        if let Some(hougeki) = data.get("api_hougeki1") {
            assert!(
                hougeki.get("api_at_eflag").is_some(),
                "Hougeki should have api_at_eflag"
            );
            assert!(
                hougeki.get("api_df_list").is_some(),
                "Hougeki should have api_df_list"
            );
            assert!(
                hougeki.get("api_damage").is_some(),
                "Hougeki should have api_damage"
            );
        }
    }
}

#[cfg(test)]
mod a17_airbattle {
    use super::*;

    #[test]
    fn test_parse_airbattle() {
        let fixture = load_fixture("api_req_sortie_airbattle.json");
        let parsed: models::ApiResponse<dto::battle::ApiBattleResponse> =
            serde_json::from_value(response_body(&fixture).clone())
                .expect("Failed to parse airbattle");

        assert_eq!(parsed.api_result, 1);
        let data = parsed.api_data.expect("api_data should exist");

        let kouku = data.api_kouku.as_ref().expect("Airbattle should have api_kouku");
        assert!(
            kouku.api_stage1.is_some(),
            "Kouku should have stage1 (air superiority)"
        );
    }
}

#[cfg(test)]
mod a18_ld_airbattle {
    use super::*;

    #[test]
    fn test_parse_ld_airbattle() {
        let fixture = load_fixture("api_req_sortie_ld_airbattle.json");
        let parsed: models::ApiResponse<dto::battle::ApiBattleResponse> =
            serde_json::from_value(response_body(&fixture).clone())
                .expect("Failed to parse ld_airbattle");

        assert_eq!(parsed.api_result, 1);
        let data = parsed.api_data.expect("api_data should exist");

        assert!(data.api_formation.is_some(), "Should have formation");
        assert!(data.api_ship_ke.is_some(), "Should have enemy ships");
        assert!(data.api_kouku.is_some(), "Should have air combat data");
    }
}

#[cfg(test)]
mod a28_midnight_battle {
    use super::*;

    #[test]
    fn test_parse_midnight_battle() {
        let fixture = load_fixture("api_req_battle_midnight_battle.json");
        let parsed: models::ApiResponse<dto::battle::ApiBattleResponse> =
            serde_json::from_value(response_body(&fixture).clone())
                .expect("Failed to parse midnight battle");

        assert_eq!(parsed.api_result, 1);
        let data = parsed.api_data.expect("api_data should exist");

        assert!(
            data.api_hougeki.is_some(),
            "Midnight battle should have api_hougeki"
        );
        assert!(data.api_formation.is_some(), "Should have formation");
        assert!(data.api_ship_ke.is_some(), "Should have enemy ships");
        assert!(data.api_f_nowhps.is_some(), "Should have friendly HP");
        assert!(data.api_e_nowhps.is_some(), "Should have enemy HP");
    }
}

#[cfg(test)]
mod a29_sp_midnight {
    use super::*;

    #[test]
    fn test_parse_sp_midnight() {
        let fixture = load_fixture("api_req_battle_midnight_sp_midnight.json");
        let parsed: models::ApiResponse<dto::battle::ApiBattleResponse> =
            serde_json::from_value(response_body(&fixture).clone())
                .expect("Failed to parse sp_midnight");

        assert_eq!(parsed.api_result, 1);
        let data = parsed.api_data.expect("api_data should exist");

        assert!(
            data.api_hougeki.is_some(),
            "SP midnight should have api_hougeki"
        );
        assert!(data.api_ship_ke.is_some(), "Should have enemy ships");
    }
}

#[cfg(test)]
mod a34_battleresult {
    use super::*;

    #[test]
    fn test_parse_battleresult() {
        let fixture = load_fixture("api_req_sortie_battleresult.json");
        let parsed: models::ApiResponse<dto::battle::ApiBattleResultResponse> =
            serde_json::from_value(response_body(&fixture).clone())
                .expect("Failed to parse battleresult");

        assert_eq!(parsed.api_result, 1);
        let data = parsed.api_data.expect("api_data should exist");

        let rank = data.api_win_rank.as_ref().expect("Should have win_rank");
        assert!(
            ["S", "A", "B", "C", "D", "E"].contains(&rank.as_str()),
            "Invalid rank: {}",
            rank
        );

        let mvp = data.api_mvp.expect("Should have MVP");
        assert!(mvp >= 1, "MVP index should be >= 1");

        assert!(data.api_get_base_exp.is_some(), "Should have base experience");
    }

    #[test]
    fn test_battleresult_drop_ship() {
        let fixture = load_fixture("api_req_sortie_battleresult.json");
        let data = api_data(&fixture);

        if let Some(get_ship) = data.get("api_get_ship") {
            let ship_id = get_ship
                .get("api_ship_id")
                .and_then(|v| v.as_i64());
            let ship_name = get_ship
                .get("api_ship_name")
                .and_then(|v| v.as_str());

            if let Some(id) = ship_id {
                assert!(id > 0, "Dropped ship ID should be positive");
            }
            if let Some(name) = ship_name {
                assert!(!name.is_empty(), "Dropped ship name should not be empty");
            }
        }
    }

    #[test]
    fn test_battleresult_enemy_info() {
        let fixture = load_fixture("api_req_sortie_battleresult.json");
        let parsed: models::ApiResponse<dto::battle::ApiBattleResultResponse> =
            serde_json::from_value(response_body(&fixture).clone())
                .expect("Failed to parse battleresult");

        let data = parsed.api_data.expect("api_data should exist");

        if let Some(enemy_info) = &data.api_enemy_info {
            if let Some(name) = &enemy_info.api_deck_name {
                assert!(!name.is_empty(), "Enemy deck name should not be empty");
            }
        }
    }
}

// =============================================================================
// Map API Tests (A36-A37)
// =============================================================================

#[cfg(test)]
mod a36_map_start {
    use super::*;

    #[test]
    fn test_parse_map_start_request() {
        let fixture = load_fixture("api_req_map_start.json");
        let req_body = request_body_str(&fixture);

        assert!(
            req_body.contains("api_maparea_id="),
            "Should have maparea_id"
        );
        assert!(
            req_body.contains("api_mapinfo_no="),
            "Should have mapinfo_no"
        );
        assert!(
            req_body.contains("api_deck_id="),
            "Should have deck_id"
        );

        let params: std::collections::HashMap<String, String> =
            serde_urlencoded::from_str(req_body).expect("Failed to parse map_start request");

        let maparea: i32 = params["api_maparea_id"].parse().expect("maparea should be int");
        let mapinfo: i32 = params["api_mapinfo_no"].parse().expect("mapinfo should be int");
        let deck: i32 = params["api_deck_id"].parse().expect("deck_id should be int");

        assert!(maparea >= 1, "Map area should be >= 1");
        assert!(mapinfo >= 1, "Map info should be >= 1");
        assert!(deck >= 1 && deck <= 4, "Deck ID should be 1-4");
    }

    #[test]
    fn test_parse_map_start_response() {
        let fixture = load_fixture("api_req_map_start.json");
        let data = api_data(&fixture);

        assert!(
            data.get("api_bosscell_no").is_some(),
            "Should have api_bosscell_no"
        );

        let no = data
            .get("api_no")
            .and_then(|v| v.as_i64())
            .expect("Should have api_no (first cell)");
        assert!(no > 0, "First cell number should be positive");
    }
}

#[cfg(test)]
mod a37_map_next {
    use super::*;

    #[test]
    fn test_parse_map_next() {
        let fixture = load_fixture("api_req_map_next.json");
        let parsed: models::ApiResponse<dto::battle::ApiMapNextResponse> =
            serde_json::from_value(response_body(&fixture).clone())
                .expect("Failed to parse map_next");

        assert_eq!(parsed.api_result, 1);
        let data = parsed.api_data.expect("api_data should exist");

        let no = data.api_no.expect("Should have api_no (cell number)");
        assert!(no > 0, "Cell number should be positive");

        let event_id = data.api_event_id.expect("Should have api_event_id");
        assert!(event_id >= 0, "Event ID should be non-negative");

        assert!(data.api_color_no.is_some(), "Should have api_color_no");
    }
}

// =============================================================================
// Cross-cutting deserialization tests
// =============================================================================

#[cfg(test)]
mod cross_cutting {
    use super::*;

    /// Verify all battle endpoint fixtures can be parsed with ApiBattleResponse DTO
    #[test]
    fn test_all_battle_fixtures_parse() {
        let battle_fixtures = [
            "api_req_sortie_battle.json",
            "api_req_sortie_airbattle.json",
            "api_req_sortie_ld_airbattle.json",
            "api_req_battle_midnight_battle.json",
            "api_req_battle_midnight_sp_midnight.json",
            "api_req_practice_battle.json",
            "api_req_practice_midnight_battle.json",
        ];

        for filename in &battle_fixtures {
            let fixture = load_fixture(filename);
            let result: Result<
                models::ApiResponse<dto::battle::ApiBattleResponse>,
                _,
            > = serde_json::from_value(response_body(&fixture).clone());

            assert!(
                result.is_ok(),
                "Failed to parse {} as ApiBattleResponse: {:?}",
                filename,
                result.err()
            );

            let data = result.unwrap().api_data;
            assert!(
                data.is_some(),
                "{}: api_data should not be None",
                filename
            );
        }
    }

    /// Verify all fixtures at least parse as valid JSON with api_result
    #[test]
    fn test_all_a_fixtures_have_api_result() {
        let fixtures = [
            "api_start2_getData.json",
            "api_port_port.json",
            "api_get_member_slot_item.json",
            "api_get_member_require_info.json",
            "api_get_member_questlist.json",
            "api_get_member_ship3.json",
            "api_req_hensei_preset_select.json",
            "api_req_kousyou_remodel_slot.json",
            "api_req_quest_clearitemget.json",
            "api_req_practice_battle_result.json",
            "api_req_kaisou_slot_deprive.json",
            "api_req_ranking_mxltvkpyuklh.json",
            "api_req_sortie_battle.json",
            "api_req_sortie_airbattle.json",
            "api_req_sortie_ld_airbattle.json",
            "api_req_battle_midnight_battle.json",
            "api_req_battle_midnight_sp_midnight.json",
            "api_req_sortie_battleresult.json",
            "api_req_map_start.json",
            "api_req_map_next.json",
        ];

        for filename in &fixtures {
            let fixture = load_fixture(filename);
            let resp = response_body(&fixture);

            let api_result = resp
                .get("api_result")
                .and_then(|v| v.as_i64())
                .unwrap_or_else(|| panic!("{}: Missing api_result", filename));

            assert_eq!(
                api_result, 1,
                "{}: api_result should be 1 (success)",
                filename
            );
        }
    }

    /// Verify request_body-only endpoints return success without api_data
    #[test]
    fn test_request_only_endpoints() {
        let fixtures = [
            "api_req_hensei_change.json",
            "api_req_quest_start.json",
            "api_req_quest_stop.json",
        ];

        for filename in &fixtures {
            let fixture = load_fixture(filename);
            let resp = response_body(&fixture);

            let api_result = resp
                .get("api_result")
                .and_then(|v| v.as_i64())
                .unwrap_or_else(|| panic!("{}: Missing api_result", filename));
            assert_eq!(api_result, 1, "{}: should succeed", filename);

            let req_body = request_body_str(&fixture);
            assert!(!req_body.is_empty(), "{}: Should have request_body", filename);
        }
    }

    /// Test that practice battle fixtures can be parsed
    #[test]
    fn test_practice_battle_parses() {
        let fixture = load_fixture("api_req_practice_battle.json");
        let result: Result<
            models::ApiResponse<dto::battle::ApiBattleResponse>,
            _,
        > = serde_json::from_value(response_body(&fixture).clone());

        assert!(
            result.is_ok(),
            "Practice battle should parse as ApiBattleResponse: {:?}",
            result.err()
        );
    }

    /// Integration: verify extract_senka_from_clearitemget handles edge cases
    #[test]
    fn test_senka_extraction_no_panic() {
        let fixture = load_fixture("api_req_quest_clearitemget.json");
        let response = serde_json::to_string(response_body(&fixture)).unwrap();
        let bonus = extract_senka_from_clearitemget(&response);
        assert!(bonus >= 0);

        // Edge cases
        assert_eq!(extract_senka_from_clearitemget("{}"), 0);
        assert_eq!(extract_senka_from_clearitemget("invalid"), 0);
    }
}

// =============================================================================
// Suite B: Category B API Tests (未処理API)
// =============================================================================

#[cfg(test)]
mod b01_charge {
    use super::*;

    #[test]
    fn test_parse_charge_response() {
        let fixture = load_fixture("api_req_hokyu_charge.json");
        let parsed: models::ApiResponse<dto::member::ApiChargeResponse> =
            serde_json::from_value(response_body(&fixture).clone())
                .expect("Failed to parse ApiChargeResponse");

        assert_eq!(parsed.api_result, 1);
        let data = parsed.api_data.expect("api_data should exist");

        assert!(!data.api_ship.is_empty(), "Should have resupplied ships");

        // Verify first ship has valid fields
        let first = &data.api_ship[0];
        assert!(first.api_id > 0, "Ship id should be positive");
        assert!(first.api_fuel >= 0, "Fuel should be non-negative");
        assert!(first.api_bull >= 0, "Bull should be non-negative");

        // api_material is flat [fuel, ammo, steel, bauxite]
        assert_eq!(data.api_material.len(), 4, "Should have 4 material values");
    }

    #[test]
    fn test_charge_ship_fields_all_valid() {
        let fixture = load_fixture("api_req_hokyu_charge.json");
        let parsed: models::ApiResponse<dto::member::ApiChargeResponse> =
            serde_json::from_value(response_body(&fixture).clone())
                .expect("Failed to parse ApiChargeResponse");

        let data = parsed.api_data.expect("api_data should exist");

        for ship in &data.api_ship {
            assert!(ship.api_id > 0, "Ship id should be positive, got {}", ship.api_id);
            assert!(ship.api_fuel >= 0, "Fuel should be non-negative");
            assert!(ship.api_bull >= 0, "Bull should be non-negative");
        }
    }

    #[test]
    fn test_charge_material_values() {
        let fixture = load_fixture("api_req_hokyu_charge.json");
        let parsed: models::ApiResponse<dto::member::ApiChargeResponse> =
            serde_json::from_value(response_body(&fixture).clone())
                .expect("Failed to parse ApiChargeResponse");

        let data = parsed.api_data.expect("api_data should exist");

        assert_eq!(data.api_material.len(), 4, "Must have exactly 4 material values");
        for (i, &val) in data.api_material.iter().enumerate() {
            assert!(val >= 0, "Material[{}] should be non-negative, got {}", i, val);
        }
    }
}

#[cfg(test)]
mod b02_ship_deck {
    use super::*;

    #[test]
    fn test_parse_ship_deck() {
        // ship_deck uses same DTO as ship3
        let fixture = load_fixture("api_get_member_ship_deck.json");
        let parsed: models::ApiResponse<dto::member::ApiShip3Response> =
            serde_json::from_value(response_body(&fixture).clone())
                .expect("Failed to parse ship_deck");

        assert_eq!(parsed.api_result, 1);
        let data = parsed.api_data.expect("api_data should exist");
        assert!(!data.api_ship_data.is_empty(), "Should have ship data");
        assert!(!data.api_deck_data.is_empty(), "Should have deck data");
    }
}

#[cfg(test)]
mod b03_powerup {
    use super::*;

    #[test]
    fn test_parse_powerup() {
        let fixture = load_fixture("api_req_kaisou_powerup.json");
        let parsed: models::ApiResponse<dto::member::ApiPowerupResponse> =
            serde_json::from_value(response_body(&fixture).clone())
                .expect("Failed to parse powerup");

        assert_eq!(parsed.api_result, 1);
        let data = parsed.api_data.expect("api_data should exist");
        assert!(data.api_ship.api_id > 0, "Ship should have valid id");
        assert!(data.api_powerup_flag == 0 || data.api_powerup_flag == 1, "Flag should be 0 or 1");
    }
}

#[cfg(test)]
mod b04_slot_exchange {
    use super::*;

    #[test]
    fn test_parse_slot_exchange() {
        let fixture = load_fixture("api_req_kaisou_slot_exchange_index.json");
        let parsed: models::ApiResponse<dto::member::ApiSlotExchangeResponse> =
            serde_json::from_value(response_body(&fixture).clone())
                .expect("Failed to parse slot_exchange_index");

        assert_eq!(parsed.api_result, 1);
        let data = parsed.api_data.expect("api_data should exist");
        assert!(data.api_ship_data.api_id > 0, "Ship should have valid id");
    }
}

#[cfg(test)]
mod b05_getship {
    use super::*;

    #[test]
    fn test_parse_getship() {
        let fixture = load_fixture("api_req_kousyou_getship.json");
        let parsed: models::ApiResponse<dto::member::ApiGetShipResponse> =
            serde_json::from_value(response_body(&fixture).clone())
                .expect("Failed to parse getship");

        assert_eq!(parsed.api_result, 1);
        let data = parsed.api_data.expect("api_data should exist");
        assert!(data.api_ship.api_id > 0, "New ship should have valid id");
        assert!(data.api_ship.api_ship_id > 0, "New ship should have valid master id");
    }
}

#[cfg(test)]
mod b06_destroyitem2 {
    use super::*;

    #[test]
    fn test_parse_destroyitem2_request() {
        let fixture = load_fixture("api_req_kousyou_destroyitem2.json");
        let req_body = request_body_str(&fixture);

        let req: dto::member::DestroyItem2Req =
            serde_urlencoded::from_str(req_body).expect("Failed to parse destroyitem2 request");

        let ids: Vec<i32> = req.api_slotitem_ids.split(',').filter_map(|s| s.parse().ok()).collect();
        assert!(!ids.is_empty(), "Should have at least one item ID to destroy");
        for &id in &ids {
            assert!(id > 0, "Item ID should be positive");
        }
    }
}

#[cfg(test)]
mod b07_destroyship {
    use super::*;

    #[test]
    fn test_parse_destroyship_request() {
        let fixture = load_fixture("api_req_kousyou_destroyship.json");
        let req_body = request_body_str(&fixture);

        let req: dto::member::DestroyShipReq =
            serde_urlencoded::from_str(req_body).expect("Failed to parse destroyship request");

        let ship_id: i32 = req.api_ship_id.split(',').next()
            .and_then(|s| s.parse().ok()).unwrap_or(0);
        assert!(ship_id > 0, "Ship ID should be positive");
    }
}

#[cfg(test)]
mod b08_createitem {
    use super::*;

    #[test]
    fn test_parse_createitem() {
        let fixture = load_fixture("api_req_kousyou_createitem.json");
        let parsed: models::ApiResponse<dto::member::ApiCreateItemResponse> =
            serde_json::from_value(response_body(&fixture).clone())
                .expect("Failed to parse createitem");

        assert_eq!(parsed.api_result, 1);
        let data = parsed.api_data.expect("api_data should exist");
        assert!(data.api_create_flag == 0 || data.api_create_flag == 1, "Flag should be 0 or 1");
    }
}

#[cfg(test)]
mod b09_material {
    use super::*;

    #[test]
    fn test_parse_member_material() {
        let fixture = load_fixture("api_get_member_material.json");
        let parsed: models::ApiResponse<Vec<models::Material>> =
            serde_json::from_value(response_body(&fixture).clone())
                .expect("Failed to parse material");

        assert_eq!(parsed.api_result, 1);
        let materials = parsed.api_data.expect("api_data should exist");
        assert_eq!(materials.len(), 8, "Should have 8 material entries");

        // Verify IDs 1-8 exist
        for id in 1..=8 {
            assert!(materials.iter().any(|m| m.api_id == id), "Material ID {} missing", id);
        }
    }
}

#[cfg(test)]
mod b10_ndock {
    use super::*;

    #[test]
    fn test_parse_member_ndock() {
        let fixture = load_fixture("api_get_member_ndock.json");
        let parsed: models::ApiResponse<Vec<models::RepairDock>> =
            serde_json::from_value(response_body(&fixture).clone())
                .expect("Failed to parse ndock");

        assert_eq!(parsed.api_result, 1);
        let ndock = parsed.api_data.expect("api_data should exist");
        assert_eq!(ndock.len(), 4, "Should have 4 repair docks");

        for dock in &ndock {
            assert!(dock.api_id >= 1 && dock.api_id <= 4, "Dock ID should be 1-4");
        }
    }
}

#[cfg(test)]
mod b11_deck {
    use super::*;

    #[test]
    fn test_parse_member_deck() {
        let fixture = load_fixture("api_get_member_deck.json");
        let parsed: models::ApiResponse<Vec<models::Fleet>> =
            serde_json::from_value(response_body(&fixture).clone())
                .expect("Failed to parse deck");

        assert_eq!(parsed.api_result, 1);
        let decks = parsed.api_data.expect("api_data should exist");
        assert!(!decks.is_empty(), "Should have at least one fleet");

        let first = &decks[0];
        assert_eq!(first.api_id, 1, "First fleet should have id=1");
        assert!(!first.api_ship.is_empty(), "Fleet should have ships");
    }
}

#[cfg(test)]
mod b12_mission_result {
    use super::*;

    #[test]
    fn test_parse_mission_result() {
        let fixture = load_fixture("api_req_mission_result.json");
        let parsed: models::ApiResponse<dto::member::ApiMissionResultResponse> =
            serde_json::from_value(response_body(&fixture).clone())
                .expect("Failed to parse mission_result");

        assert_eq!(parsed.api_result, 1);
        let data = parsed.api_data.expect("api_data should exist");
        assert!(data.api_clear_result >= 0, "Clear result should be non-negative");
    }
}

#[cfg(test)]
mod b13_practice_battles {
    use super::*;

    #[test]
    fn test_practice_battle_parses_as_battle() {
        let fixture = load_fixture("api_req_practice_battle.json");
        let parsed: models::ApiResponse<dto::battle::ApiBattleResponse> =
            serde_json::from_value(response_body(&fixture).clone())
                .expect("Failed to parse practice battle");

        assert_eq!(parsed.api_result, 1);
        let data = parsed.api_data.expect("api_data should exist");
        assert!(data.api_ship_ke.is_some(), "Should have enemy ships");
    }

    #[test]
    fn test_practice_midnight_parses_as_battle() {
        let fixture = load_fixture("api_req_practice_midnight_battle.json");
        let parsed: models::ApiResponse<dto::battle::ApiBattleResponse> =
            serde_json::from_value(response_body(&fixture).clone())
                .expect("Failed to parse practice midnight battle");

        assert_eq!(parsed.api_result, 1);
        let data = parsed.api_data.expect("api_data should exist");
        assert!(data.api_hougeki.is_some(), "Should have night battle hougeki");
    }
}

#[cfg(test)]
mod b14_log_only {
    use super::*;

    #[test]
    fn test_log_only_fixtures_valid() {
        let fixtures = [
            "api_req_kaisou_slotset.json",
            "api_req_kaisou_slotset_ex.json",
            "api_req_kaisou_unsetslot_all.json",
            "api_req_kaisou_preset_slot_select.json",
            "api_req_kaisou_remodeling.json",
            "api_req_kousyou_createship.json",
            "api_req_kousyou_createship_speedchange.json",
            "api_req_mission_start.json",
            "api_get_member_mapinfo.json",
        ];

        for filename in &fixtures {
            let fixture = load_fixture(filename);
            let resp = response_body(&fixture);

            let api_result = resp
                .get("api_result")
                .and_then(|v| v.as_i64())
                .unwrap_or_else(|| panic!("{}: Missing api_result", filename));

            assert_eq!(api_result, 1, "{}: api_result should be 1", filename);
        }
    }
}

// =============================================================================
// Suite 2: Data Transformation Tests
// =============================================================================

#[cfg(test)]
mod suite2_data_transformation {
    use super::*;

    #[test]
    fn test_extract_stat_value_array() {
        let val = serde_json::json!([65, 50]);
        assert_eq!(ship::extract_stat_value(&val), 65);
    }

    #[test]
    fn test_extract_stat_value_scalar() {
        let val = serde_json::json!(42);
        assert_eq!(ship::extract_stat_value(&val), 42);
    }

    #[test]
    fn test_extract_stat_value_null() {
        let val = serde_json::Value::Null;
        assert_eq!(ship::extract_stat_value(&val), 0);
    }

    #[test]
    fn test_extract_stat_value_empty_array() {
        let val = serde_json::json!([]);
        assert_eq!(ship::extract_stat_value(&val), 0);
    }

    #[test]
    fn test_extract_slot_ids_normal() {
        let val = serde_json::json!([-1, 100, 200, -1]);
        assert_eq!(ship::extract_slot_ids(&val), vec![-1, 100, 200, -1]);
    }

    #[test]
    fn test_extract_slot_ids_all_empty() {
        let val = serde_json::json!([-1, -1, -1, -1, -1]);
        assert_eq!(ship::extract_slot_ids(&val), vec![-1, -1, -1, -1, -1]);
    }

    #[test]
    fn test_extract_slot_ids_null() {
        let val = serde_json::Value::Null;
        assert_eq!(ship::extract_slot_ids(&val), Vec::<i32>::new());
    }

    #[test]
    fn test_build_ship_info_with_master() {
        let ship_json = serde_json::json!({
            "api_id": 12345,
            "api_ship_id": 100,
            "api_lv": 99,
            "api_nowhp": 50,
            "api_maxhp": 63,
            "api_cond": 49,
            "api_fuel": 75,
            "api_bull": 80,
            "api_karyoku": [72, 60],
            "api_raisou": [88, 70],
            "api_taiku": [68, 50],
            "api_soukou": [55, 45],
            "api_taisen": [40, 30],
            "api_kaihi": [65, 55],
            "api_sakuteki": [30, 20],
            "api_lucky": [15, 12],
            "api_locked": 1,
            "api_slot": [1001, 1002, -1, -1, -1],
            "api_slot_ex": 2001,
            "api_soku": 10
        });
        let player_ship: models::PlayerShip =
            serde_json::from_value(ship_json).expect("Failed to parse PlayerShip");
        let master = models::MasterShipInfo {
            name: "島風".to_string(),
            stype: 2,
        };

        let info = ship::build_ship_info(&player_ship, Some(&master));

        assert_eq!(info.ship_id, 100);
        assert_eq!(info.name, "島風");
        assert_eq!(info.stype, 2);
        assert_eq!(info.lv, 99);
        assert_eq!(info.hp, 50);
        assert_eq!(info.maxhp, 63);
        assert_eq!(info.cond, 49);
        assert_eq!(info.fuel, 75);
        assert_eq!(info.bull, 80);
        assert_eq!(info.firepower, 72);
        assert_eq!(info.torpedo, 88);
        assert_eq!(info.aa, 68);
        assert_eq!(info.armor, 55);
        assert_eq!(info.asw, 40);
        assert_eq!(info.evasion, 65);
        assert_eq!(info.los, 30);
        assert_eq!(info.luck, 15);
        assert!(info.locked);
        assert_eq!(info.slot, vec![1001, 1002, -1, -1, -1]);
        assert_eq!(info.slot_ex, 2001);
        assert_eq!(info.soku, 10);
    }

    #[test]
    fn test_build_ship_info_without_master() {
        let ship_json = serde_json::json!({
            "api_id": 999,
            "api_ship_id": 777,
            "api_lv": 1
        });
        let player_ship: models::PlayerShip =
            serde_json::from_value(ship_json).expect("Failed to parse PlayerShip");

        let info = ship::build_ship_info(&player_ship, None);

        assert_eq!(info.name, "Unknown(777)");
        assert_eq!(info.stype, 0);
    }

    #[test]
    fn test_build_ship_info_from_port_fixture() {
        let fixture = load_fixture("api_port_port.json");
        let parsed: models::ApiResponse<models::ApiPort> =
            serde_json::from_value(response_body(&fixture).clone())
                .expect("Failed to parse ApiPort");

        let port = parsed.api_data.expect("api_data should exist");
        let first_ship = &port.api_ship[0];

        let info = ship::build_ship_info(first_ship, None);

        assert!(info.ship_id > 0);
        assert!(info.lv > 0);
        assert!(info.maxhp > 0);
        assert!(!info.slot.is_empty());
    }
}

// =============================================================================
// Suite 5: Quest Progress Tests
// =============================================================================

#[cfg(test)]
mod suite5_quest_progress {
    use crate::quest_progress::*;
    use chrono::{FixedOffset, TimeZone};

    fn jst() -> FixedOffset {
        FixedOffset::east_opt(9 * 3600).unwrap()
    }

    fn make_entry(quest_id: i32, count: i32, count_max: i32, last: chrono::DateTime<FixedOffset>) -> QuestProgressEntry {
        QuestProgressEntry {
            quest_id,
            quest_id_str: format!("test_{}", quest_id),
            area_cleared: Default::default(),
            area_counts: Default::default(),
            count,
            count_max,
            completed: false,
            last_updated: last,
        }
    }

    #[test]
    fn test_quest_progress_serialization_roundtrip() {
        let mut state = QuestProgressState::default();
        let now = jst().with_ymd_and_hms(2026, 3, 18, 12, 0, 0).unwrap();
        state.quests.insert(226, make_entry(226, 3, 5, now));
        state.last_reset_check = Some(now);

        let json = serde_json::to_string(&state).expect("serialize");
        let restored: QuestProgressState = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(restored.quests.len(), 1);
        let entry = restored.quests.get(&226).unwrap();
        assert_eq!(entry.count, 3);
        assert_eq!(entry.count_max, 5);
        assert!(!entry.completed);
    }

    #[test]
    fn test_quest_progress_entry_fields() {
        let now = jst().with_ymd_and_hms(2026, 3, 18, 12, 0, 0).unwrap();
        let mut entry = make_entry(226, 0, 5, now);

        entry.count += 1;
        assert_eq!(entry.count, 1);

        entry.count = 5;
        entry.completed = entry.count >= entry.count_max;
        assert!(entry.completed);
    }

    #[test]
    fn test_quest_progress_area_counts() {
        let now = jst().with_ymd_and_hms(2026, 3, 18, 12, 0, 0).unwrap();
        let mut entry = make_entry(226, 0, 1, now);

        *entry.area_counts.entry("7-2".to_string()).or_insert(0) += 1;
        *entry.area_counts.entry("7-5".to_string()).or_insert(0) += 1;
        *entry.area_counts.entry("7-2".to_string()).or_insert(0) += 1;

        assert_eq!(entry.area_counts["7-2"], 2);
        assert_eq!(entry.area_counts["7-5"], 1);
    }

    #[test]
    fn test_quest_progress_multi_gauge_area_key() {
        // Multi-gauge maps: 7-2 has G(1st boss) and M(2nd boss)
        // 7-5 has K(1st), Q(2nd), T(3rd boss)
        let now = jst().with_ymd_and_hms(2026, 3, 18, 12, 0, 0).unwrap();
        let mut entry = make_entry(888, 0, 3, now);

        *entry.area_counts.entry("7-2(1st)".to_string()).or_insert(0) += 1;
        *entry.area_counts.entry("7-2(2nd)".to_string()).or_insert(0) += 1;
        *entry.area_counts.entry("7-5(1st)".to_string()).or_insert(0) += 1;
        *entry.area_counts.entry("7-5(2nd)".to_string()).or_insert(0) += 1;
        *entry.area_counts.entry("7-5(3rd)".to_string()).or_insert(0) += 1;

        assert_eq!(entry.area_counts.len(), 5);
        assert_eq!(entry.area_counts["7-2(1st)"], 1);
        assert_eq!(entry.area_counts["7-5(3rd)"], 1);
    }

    #[test]
    fn test_load_progress_empty_file() {
        let state = load_progress(std::path::Path::new("/tmp/nonexistent_quest_progress.json"));
        assert!(state.quests.is_empty());
    }
}

// =============================================================================
// Suite 4: Sortie Sequence Fixtures Validation
// =============================================================================

#[cfg(test)]
mod suite4_sortie_sequences {
    use super::*;

    fn load_sequence_file(seq_dir: &str, filename: &str) -> serde_json::Value {
        let path = format!("tests/fixtures/sequences/{}/{}", seq_dir, filename);
        let json_str = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", path, e));
        serde_json::from_str(&json_str).expect("Failed to parse sequence fixture")
    }

    fn list_sequence_files(seq_dir: &str) -> Vec<String> {
        let path = format!("tests/fixtures/sequences/{}", seq_dir);
        let mut files: Vec<String> = std::fs::read_dir(&path)
            .unwrap_or_else(|e| panic!("Failed to read dir {}: {}", path, e))
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|f| f.ends_with(".json") && !f.starts_with('_') && f != "mapinfo.json")
            .collect();
        files.sort();
        files
    }

    fn verify_sequence(seq_dir: &str, expected_area: i32, expected_no: i32) {
        let files = list_sequence_files(seq_dir);
        assert!(!files.is_empty(), "{}: should have files", seq_dir);

        for f in &files {
            let fixture = load_sequence_file(seq_dir, f);
            assert!(fixture.get("endpoint").is_some(), "{}/{}: missing endpoint", seq_dir, f);
        }

        // First file = map_start
        assert!(files[0].contains("map_start"), "{}: first file should be map_start", seq_dir);
        let start = load_sequence_file(seq_dir, &files[0]);
        let req = start.get("request_body").and_then(|v| v.as_str()).unwrap_or("");
        assert!(req.contains(&format!("api_maparea_id={}", expected_area)), "{}: wrong area", seq_dir);
        assert!(req.contains(&format!("api_mapinfo_no={}", expected_no)), "{}: wrong map", seq_dir);

        // Last file = port
        assert!(files.last().unwrap().contains("port"), "{}: last should be port", seq_dir);

        // Has battleresult
        assert!(files.iter().any(|f| f.contains("battleresult")), "{}: needs battleresult", seq_dir);
    }

    fn get_gauge_num(seq_dir: &str) -> Option<i64> {
        let files = list_sequence_files(seq_dir);
        let start = load_sequence_file(seq_dir, &files[0]);
        start.get("_test_meta").and_then(|m| m.get("gauge_num")).and_then(|v| v.as_i64())
    }

    /// Get gauge_num from mapinfo.json for a specific map (e.g. map_id=72 for 7-2)
    fn get_gauge_from_mapinfo(seq_dir: &str, map_id: i64) -> Option<i64> {
        let mapinfo = load_sequence_file(seq_dir, "mapinfo.json");
        let map_info = mapinfo.get("response_body")?
            .get("api_data")?
            .get("api_map_info")?
            .as_array()?;
        for m in map_info {
            if m.get("api_id").and_then(|v| v.as_i64()) == Some(map_id) {
                return m.get("api_gauge_num").and_then(|v| v.as_i64());
            }
        }
        None
    }

    // --- 7-2 (2 gauges: G=1st boss, M=2nd boss) ---

    #[test]
    fn test_7_2_gauge1_mapinfo() {
        // Gauge 1: no sortie data, but mapinfo confirms gauge_num=1
        let gauge = get_gauge_from_mapinfo("sortie_7-2_gauge1", 72);
        assert_eq!(gauge, Some(1), "mapinfo should show gauge 1 for 7-2");
    }

    #[test]
    fn test_7_2_gauge2_sequence() {
        verify_sequence("sortie_7-2_gauge2", 7, 2);
        assert_eq!(get_gauge_num("sortie_7-2_gauge2"), Some(2));
        assert_eq!(get_gauge_from_mapinfo("sortie_7-2_gauge2", 72), Some(2));
    }

    #[test]
    fn test_7_2_both_gauges() {
        let g1 = get_gauge_from_mapinfo("sortie_7-2_gauge1", 72).unwrap();
        let g2 = get_gauge_from_mapinfo("sortie_7-2_gauge2", 72).unwrap();
        assert_eq!((g1, g2), (1, 2), "7-2 should have gauges 1 and 2");
    }

    // --- 7-5 (3 gauges: K=1st, Q=2nd, T=3rd boss) ---

    #[test]
    fn test_7_5_gauge1_sequence() {
        verify_sequence("sortie_7-5_gauge1", 7, 5);
        assert_eq!(get_gauge_num("sortie_7-5_gauge1"), Some(1));
        assert_eq!(get_gauge_from_mapinfo("sortie_7-5_gauge1", 75), Some(1));

        let files = list_sequence_files("sortie_7-5_gauge1");
        assert!(!files.iter().any(|f| f.contains("midnight")), "K boss: no midnight");
    }

    #[test]
    fn test_7_5_gauge2_sequence() {
        verify_sequence("sortie_7-5_gauge2", 7, 5);
        assert_eq!(get_gauge_num("sortie_7-5_gauge2"), Some(2));
        assert_eq!(get_gauge_from_mapinfo("sortie_7-5_gauge2", 75), Some(2));

        let files = list_sequence_files("sortie_7-5_gauge2");
        assert!(files.iter().any(|f| f.contains("midnight")), "Q boss: has midnight");
    }

    #[test]
    fn test_7_5_gauge3_sequence() {
        verify_sequence("sortie_7-5_gauge3", 7, 5);
        assert_eq!(get_gauge_num("sortie_7-5_gauge3"), Some(3));
        assert_eq!(get_gauge_from_mapinfo("sortie_7-5_gauge3", 75), Some(3));
    }

    #[test]
    fn test_7_5_all_gauges_distinct() {
        let g1 = get_gauge_from_mapinfo("sortie_7-5_gauge1", 75).unwrap();
        let g2 = get_gauge_from_mapinfo("sortie_7-5_gauge2", 75).unwrap();
        let g3 = get_gauge_from_mapinfo("sortie_7-5_gauge3", 75).unwrap();
        assert_eq!((g1, g2, g3), (1, 2, 3), "All 3 gauges should be distinct");
    }

    #[test]
    fn test_all_boss_results_parseable() {
        for seq_dir in &[
            "sortie_7-2_gauge2", "sortie_7-5_gauge1",
            "sortie_7-5_gauge2", "sortie_7-5_gauge3",
        ] {
            let files = list_sequence_files(seq_dir);
            let boss_file = files.iter().rev()
                .find(|f| f.contains("battleresult"))
                .unwrap_or_else(|| panic!("{}: no battleresult", seq_dir));

            let fixture = load_sequence_file(seq_dir, boss_file);
            let resp = response_body(&fixture);
            let rank = resp.get("api_data")
                .and_then(|d| d.get("api_win_rank"))
                .and_then(|v| v.as_str());
            assert!(rank.is_some(), "{}: boss result should have rank", seq_dir);
        }
    }

    // --- Original multi-battle ---

    // --- Multi-gauge quest progress bug reproduction ---

    #[test]
    fn test_gauge_num_from_mapinfo_for_sortie() {
        // BUG: sortie to 7-2 gauge 2 produces area key "7-2" instead of "7-2(2nd)"
        // EXPECTED: mapinfo_gauges cache provides gauge_num when api_eventmap is absent

        // Load mapinfo for 7-2 gauge 2
        let mapinfo = load_sequence_file("sortie_7-2_gauge2", "mapinfo.json");
        let map_info_arr = mapinfo.get("response_body").unwrap()
            .get("api_data").unwrap()
            .get("api_map_info").unwrap()
            .as_array().unwrap();

        // Build mapinfo_gauges cache (simulating what process_api should do)
        let mut mapinfo_gauges: std::collections::HashMap<i32, i32> = std::collections::HashMap::new();
        for m in map_info_arr {
            let map_id = m.get("api_id").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            if let Some(gauge) = m.get("api_gauge_num").and_then(|v| v.as_i64()) {
                if gauge > 0 {
                    mapinfo_gauges.insert(map_id, gauge as i32);
                }
            }
        }

        // 7-2 (map_id=72) should have gauge_num=2
        assert_eq!(mapinfo_gauges.get(&72), Some(&2), "7-2 should have gauge 2 from mapinfo");

        // Simulate gauge suffix generation (this is what battle.rs does)
        let map_area = 7;
        let map_no = 2;
        let map_id = map_area * 10 + map_no;

        // Current bug: api_eventmap is None for regular maps, so gauge_num = None
        let gauge_from_eventmap: Option<i32> = None;
        // Fix: fall back to mapinfo cache
        let gauge_num = gauge_from_eventmap.or_else(|| mapinfo_gauges.get(&map_id).copied());

        let gauge_suffix = match gauge_num {
            Some(1) => "(1st)",
            Some(2) => "(2nd)",
            Some(3) => "(3rd)",
            _ => "",
        };
        let map_area_str = format!("{}-{}{}", map_area, map_no, gauge_suffix);

        assert_eq!(map_area_str, "7-2(2nd)", "Area key should include gauge suffix");
    }

    #[test]
    fn test_gauge_num_7_5_all_gauges() {
        // Test all 3 gauges of 7-5 produce correct area keys
        let expected = [
            ("sortie_7-5_gauge1", 75, "7-5(1st)"),
            ("sortie_7-5_gauge2", 75, "7-5(2nd)"),
            ("sortie_7-5_gauge3", 75, "7-5(3rd)"),
        ];

        for (seq_dir, map_id, expected_key) in &expected {
            let mapinfo = load_sequence_file(seq_dir, "mapinfo.json");
            let map_info_arr = mapinfo.get("response_body").unwrap()
                .get("api_data").unwrap()
                .get("api_map_info").unwrap()
                .as_array().unwrap();

            let mut gauges: std::collections::HashMap<i32, i32> = std::collections::HashMap::new();
            for m in map_info_arr {
                let mid = m.get("api_id").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                if let Some(g) = m.get("api_gauge_num").and_then(|v| v.as_i64()) {
                    if g > 0 { gauges.insert(mid, g as i32); }
                }
            }

            let gauge_num = gauges.get(map_id).copied();
            let suffix = match gauge_num {
                Some(1) => "(1st)",
                Some(2) => "(2nd)",
                Some(3) => "(3rd)",
                _ => "",
            };
            let key = format!("7-5{}", suffix);
            assert_eq!(&key, expected_key, "{}: area key mismatch", seq_dir);
        }
    }

    // --- Original multi-battle ---

    #[test]
    fn test_multi_battle_sequence() {
        let files = list_sequence_files("sortie_multi_battle");
        assert!(!files.is_empty());
        for f in &files {
            let fixture = load_sequence_file("sortie_multi_battle", f);
            assert!(fixture.get("endpoint").is_some());
        }
    }
}
