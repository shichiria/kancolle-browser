use super::*;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;

#[cfg(test)]
mod parsing_tests {
    use super::models::{ApiPort, ApiResponse};
    use serde_json::from_str;

    #[test]
    fn test_parse_api_port() {
        let json_str = include_str!("../../tests/fixtures/api_port_port.json");

        // The raw_api JSON stored by our app includes a wrapper:
        // { "endpoint": "...", "request_body": "...", "response_body": { "api_result": 1, ... } }
        let wrapper: serde_json::Value =
            serde_json::from_str(json_str).expect("Failed to parse wrapper JSON");
        let response_body = wrapper
            .get("response_body")
            .expect("Missing response_body in wrapper");

        let parsed: Result<ApiResponse<ApiPort>, _> = serde_json::from_value(response_body.clone());
        assert!(
            parsed.is_ok(),
            "Failed to parse API Port JSON: {:?}",
            parsed.err()
        );

        let api_response = parsed.unwrap();
        assert_eq!(
            api_response.api_result, 1,
            "api_result should be 1 (success)"
        );

        let api_data = api_response.api_data.expect("api_data should not be null");
        assert!(api_data.api_ship.len() > 0, "Should have at least one ship");
        assert!(
            api_data.api_deck_port.len() > 0,
            "Should have at least one fleet"
        );
        assert!(api_data.api_material.len() > 0, "Should have material data");
        assert_eq!(api_data.api_ndock.len(), 4, "Should have 4 docks");
    }

    #[test]
    fn test_parse_questlist() {
        let json_str = include_str!("../../tests/fixtures/api_get_member_questlist.json");
        let wrapper: serde_json::Value =
            serde_json::from_str(json_str).expect("Failed to parse wrapper JSON");
        let api_data = wrapper
            .get("response_body")
            .and_then(|b| b.get("api_data"))
            .expect("Missing api_data in questlist");

        let api_list = api_data
            .get("api_list")
            .and_then(|l| l.as_array())
            .expect("api_list should be an array");
        assert!(api_list.len() > 0, "Quest list should not be empty");

        let has_quest = api_list.iter().any(|item| item.get("api_no").is_some());
        assert!(has_quest, "Should have at least one active quest object");
    }

    #[test]
    fn test_parse_sortie_battle() {
        let json_str = include_str!("../../tests/fixtures/api_req_sortie_battle.json");
        let wrapper: serde_json::Value =
            serde_json::from_str(json_str).expect("Failed to parse wrapper JSON");
        let api_data = wrapper
            .get("response_body")
            .and_then(|b| b.get("api_data"))
            .expect("Missing api_data in battle");

        let api_ship_ke = api_data
            .get("api_ship_ke")
            .and_then(|s| s.as_array())
            .expect("api_ship_ke should be an array");
        assert!(
            api_ship_ke.len() > 0,
            "Enemy ships should exist in battle data"
        );
    }
}
