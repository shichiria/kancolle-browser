use super::{battle, dto, models};
use dto::request::{HenseiChangeReq, QuestReq, RemodelSlotReq};
use log::{error, info, warn};
use serde::de::DeserializeOwned;

/// Pre-parsed API data to pass into the single async task
// One transient instance exists at a time; stack size is not a concern here.
#[allow(clippy::large_enum_variant)]
pub(super) enum ParsedApi {
    Start2(Box<models::ApiStart2>),
    Port(Box<models::ApiPort>),
    SlotItem(Vec<models::PlayerSlotItemApi>),
    QuestList(dto::member::ApiQuestListResponse),
    Battle(serde_json::Value),
    ExerciseResult(dto::member::ApiExerciseResultResponse),
    HenseiChange {
        fleet_id: usize,
        ship_idx: i32,
        ship_id: i32,
    },
    HenseiPresetSelect(dto::member::ApiHenseiPresetSelectResponse),
    RemodelSlot {
        slot_id: i32,
        success: bool,
        eq_id: i32,
    },
    QuestStart {
        quest_id: i32,
    },
    QuestStop {
        quest_id: i32,
    },
    QuestClear {
        quest_id: i32,
        senka_bonus: i64,
    },
    Ship3(dto::member::ApiShip3Response),
    SlotDeprive(dto::member::ApiSlotDepriveResponse),
    Charge(dto::member::ApiChargeResponse),
    Ranking(dto::ranking::ApiRankingResponse),
    // Category B: Ship/Equipment updates
    Powerup(dto::member::ApiPowerupResponse),
    SlotExchange(dto::member::ApiSlotExchangeResponse),
    GetShip(dto::member::ApiGetShipResponse),
    // Category B: Removal operations
    DestroyItem2 {
        item_ids: Vec<i32>,
    },
    DestroyShip {
        ship_id: i32,
    },
    CreateItem(dto::member::ApiCreateItemResponse),
    // Category B: Resource/Info refreshes
    MemberMaterial(Vec<models::Material>),
    MemberNDock(Vec<models::RepairDock>),
    MemberDeck(Vec<models::Fleet>),
    // Category B: Mission
    MissionResult(dto::member::ApiMissionResultResponse),
    // Category B: mapinfo — gauge cache + air-base state
    MapInfoData {
        gauges: std::collections::HashMap<i32, i32>,
        air_bases: dto::air_corps::MapInfoAirBases,
    },
    // Category B: 基地航空隊 — full state (api_get_member/base_air_corps)
    BaseAirCorps(Vec<dto::air_corps::AirBase>),
    // Category B: 基地航空隊 incremental updates
    AirCorpsSetPlane {
        request_body: String,
        api_data: dto::air_corps::PlaneUpdate,
    },
    AirCorpsSetAction {
        request_body: String,
    },
    AirCorpsSupply {
        request_body: String,
        api_data: dto::air_corps::PlaneUpdate,
    },
    AirCorpsChangeName {
        request_body: String,
    },
    AirCorpsChangeDeployment {
        request_body: String,
        api_data: dto::air_corps::DeploymentUpdate,
    },
    AirCorpsCondRecovery {
        request_body: String,
        api_data: dto::air_corps::PlaneUpdate,
    },
    // Category B: Log-only (null response or follow-up refresh)
    LogOnly,
    Other,
}

fn parse_typed<T, F>(
    json_str: &str,
    endpoint_name: &str,
    missing_data: ParsedApi,
    constructor: F,
) -> ParsedApi
where
    T: DeserializeOwned,
    F: FnOnce(T) -> ParsedApi,
{
    match serde_json::from_str::<models::ApiResponse<T>>(json_str) {
        Ok(response) => response.api_data.map(constructor).unwrap_or(missing_data),
        Err(error) => {
            error!("Failed to parse {endpoint_name}: {error}");
            ParsedApi::Other
        }
    }
}

pub(super) fn parse(endpoint: &str, json_str: &str, request_body: &str) -> ParsedApi {
    let parsed = match endpoint {
        "/kcsapi/api_start2/getData" => {
            info!("Processing api_start2/getData (master data)");
            parse_typed(json_str, "api_start2", ParsedApi::Other, |data| {
                ParsedApi::Start2(Box::new(data))
            })
        }
        "/kcsapi/api_port/port" => {
            info!("Processing api_port/port (home screen)");
            parse_typed(json_str, "api_port", ParsedApi::Other, |data| {
                ParsedApi::Port(Box::new(data))
            })
        }
        "/kcsapi/api_get_member/slot_item" => {
            info!("Processing api_get_member/slot_item (player equipment)");
            parse_typed(json_str, "slot_item", ParsedApi::Other, ParsedApi::SlotItem)
        }
        "/kcsapi/api_get_member/require_info" => {
            info!("Processing api_get_member/require_info (includes slot_item)");
            // require_info contains api_slot_item in the same format as api_get_member/slot_item
            match serde_json::from_str::<models::ApiResponse<serde_json::Value>>(json_str) {
                Ok(data) => {
                    if let Some(api_data) = data.api_data {
                        if let Some(items_val) = api_data.get("api_slot_item") {
                            match serde_json::from_value::<Vec<models::PlayerSlotItemApi>>(
                                items_val.clone(),
                            ) {
                                Ok(items) => ParsedApi::SlotItem(items),
                                Err(e) => {
                                    error!("Failed to parse require_info slot_item: {}", e);
                                    ParsedApi::Other
                                }
                            }
                        } else {
                            ParsedApi::Other
                        }
                    } else {
                        ParsedApi::Other
                    }
                }
                Err(e) => {
                    error!("Failed to parse require_info: {}", e);
                    ParsedApi::Other
                }
            }
        }
        "/kcsapi/api_get_member/questlist" => {
            info!("Processing api_get_member/questlist");
            parse_typed(
                json_str,
                "questlist",
                ParsedApi::Other,
                ParsedApi::QuestList,
            )
        }
        "/kcsapi/api_req_hensei/change" => {
            info!("Processing api_req_hensei/change (fleet composition change)");
            match serde_urlencoded::from_str::<HenseiChangeReq>(request_body) {
                Ok(req) => ParsedApi::HenseiChange {
                    fleet_id: req.api_id,
                    ship_idx: req.api_ship_idx,
                    ship_id: req.api_ship_id,
                },
                Err(e) => {
                    error!("Failed to parse hensei/change req: {}", e);
                    ParsedApi::Other
                }
            }
        }
        "/kcsapi/api_req_hensei/preset_select" => {
            info!("Processing api_req_hensei/preset_select (preset fleet load)");
            parse_typed(
                json_str,
                "preset_select",
                ParsedApi::Other,
                ParsedApi::HenseiPresetSelect,
            )
        }
        "/kcsapi/api_req_kousyou/remodel_slot" => {
            info!("Processing api_req_kousyou/remodel_slot (equipment improvement)");
            let req = serde_urlencoded::from_str::<RemodelSlotReq>(request_body).ok();
            let slot_id = req.as_ref().map(|r| r.api_slot_id).unwrap_or(-1);
            let req_eq_id = req.as_ref().map(|r| r.api_id).unwrap_or(-1);

            // Extract eq_id + success from response
            let (success, resp_eq_id) = match serde_json::from_str::<
                models::ApiResponse<dto::member::ApiRemodelSlotResponse>,
            >(json_str)
            {
                Ok(data) => {
                    let api_data = &data.api_data;
                    let flag = api_data.as_ref().and_then(|d| d.api_remodel_flag);
                    // Get master eq_id from api_after_slot.api_slotitem_id in response
                    let mut eq_id = api_data
                        .as_ref()
                        .and_then(|d| d.api_after_slot.as_ref())
                        .and_then(|s| s.api_slotitem_id)
                        .unwrap_or(-1);

                    if eq_id <= 0 {
                        eq_id = req_eq_id; // Fallback to request body's api_id
                    }
                    info!(
                        "remodel_slot: slot_id={}, resp_eq_id={}, flag={:?}",
                        slot_id, eq_id, flag
                    );
                    (flag.map(|f| f == 1).unwrap_or(false), eq_id)
                }
                Err(e) => {
                    error!("Failed to parse remodel_slot response: {}", e);
                    (false, -1)
                }
            };
            ParsedApi::RemodelSlot {
                slot_id,
                success,
                eq_id: resp_eq_id,
            }
        }
        "/kcsapi/api_req_quest/start" => {
            info!("Processing {} (quest started)", endpoint);
            let req = serde_urlencoded::from_str::<QuestReq>(request_body).ok();
            let quest_id = req.map(|r| r.api_quest_id).unwrap_or(0);
            ParsedApi::QuestStart { quest_id }
        }
        "/kcsapi/api_req_quest/stop" => {
            info!("Processing {} (quest cancelled)", endpoint);
            let req = serde_urlencoded::from_str::<QuestReq>(request_body).ok();
            let quest_id = req.map(|r| r.api_quest_id).unwrap_or(0);
            ParsedApi::QuestStop { quest_id }
        }
        "/kcsapi/api_req_quest/clearitemget" => {
            info!("Processing {} (quest completed)", endpoint);
            let req = serde_urlencoded::from_str::<QuestReq>(request_body).ok();
            let quest_id = req.map(|r| r.api_quest_id).unwrap_or(0);
            // Parse response to extract senka bonus from api_bounus
            let senka_bonus = extract_senka_from_clearitemget(json_str);
            ParsedApi::QuestClear {
                quest_id,
                senka_bonus,
            }
        }
        "/kcsapi/api_req_practice/battle_result" => {
            info!("Processing api_req_practice/battle_result (exercise result)");
            parse_typed(
                json_str,
                "exercise battle_result",
                ParsedApi::Other,
                ParsedApi::ExerciseResult,
            )
        }
        "/kcsapi/api_get_member/ship3" => {
            info!("Processing api_get_member/ship3 (ship data after equipment change)");
            parse_typed(json_str, "ship3", ParsedApi::Other, ParsedApi::Ship3)
        }
        "/kcsapi/api_req_kaisou/slot_deprive" => {
            info!("Processing api_req_kaisou/slot_deprive (equipment transfer between ships)");
            parse_typed(
                json_str,
                "slot_deprive",
                ParsedApi::Other,
                ParsedApi::SlotDeprive,
            )
        }
        "/kcsapi/api_req_hokyu/charge" => {
            info!("Processing api_req_hokyu/charge (resupply)");
            parse_typed(
                json_str,
                "hokyu/charge",
                ParsedApi::Other,
                ParsedApi::Charge,
            )
        }
        "/kcsapi/api_req_ranking/mxltvkpyuklh" => {
            info!("Processing api_req_ranking/mxltvkpyuklh (ranking data)");
            parse_typed(json_str, "ranking", ParsedApi::Other, ParsedApi::Ranking)
        }
        // --- Category B: Ship/Equipment updates ---
        "/kcsapi/api_get_member/ship_deck" => {
            info!("Processing api_get_member/ship_deck");
            // Same structure as ship3
            parse_typed(json_str, "ship_deck", ParsedApi::Other, ParsedApi::Ship3)
        }
        "/kcsapi/api_req_kaisou/powerup" => {
            info!("Processing api_req_kaisou/powerup (modernization)");
            parse_typed(json_str, "powerup", ParsedApi::Other, ParsedApi::Powerup)
        }
        "/kcsapi/api_req_kaisou/slot_exchange_index" => {
            info!("Processing api_req_kaisou/slot_exchange_index (swap equipment slots)");
            parse_typed(
                json_str,
                "slot_exchange_index",
                ParsedApi::Other,
                ParsedApi::SlotExchange,
            )
        }
        "/kcsapi/api_req_kousyou/getship" => {
            info!("Processing api_req_kousyou/getship (construction complete)");
            parse_typed(json_str, "getship", ParsedApi::Other, ParsedApi::GetShip)
        }
        // --- Category B: Removal operations ---
        "/kcsapi/api_req_kousyou/destroyitem2" => {
            info!("Processing api_req_kousyou/destroyitem2 (scrap equipment)");
            let item_ids: Vec<i32> =
                serde_urlencoded::from_str::<dto::member::DestroyItem2Req>(request_body)
                    .map(|req| {
                        req.api_slotitem_ids
                            .split(',')
                            .filter_map(|s| s.parse().ok())
                            .collect()
                    })
                    .unwrap_or_default();
            ParsedApi::DestroyItem2 { item_ids }
        }
        "/kcsapi/api_req_kousyou/destroyship" => {
            info!("Processing api_req_kousyou/destroyship (scrap ship)");
            let ship_id = serde_urlencoded::from_str::<dto::member::DestroyShipReq>(request_body)
                .map(|req| {
                    req.api_ship_id
                        .split(',')
                        .next()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0)
                })
                .unwrap_or(0);
            ParsedApi::DestroyShip { ship_id }
        }
        "/kcsapi/api_req_kousyou/createitem" => {
            info!("Processing api_req_kousyou/createitem (develop equipment)");
            parse_typed(
                json_str,
                "createitem",
                ParsedApi::Other,
                ParsedApi::CreateItem,
            )
        }
        // --- Category B: Resource/Info refreshes ---
        "/kcsapi/api_get_member/material" => {
            info!("Processing api_get_member/material");
            parse_typed(
                json_str,
                "material",
                ParsedApi::Other,
                ParsedApi::MemberMaterial,
            )
        }
        "/kcsapi/api_get_member/ndock" => {
            info!("Processing api_get_member/ndock");
            parse_typed(json_str, "ndock", ParsedApi::Other, ParsedApi::MemberNDock)
        }
        "/kcsapi/api_get_member/deck" => {
            info!("Processing api_get_member/deck");
            parse_typed(json_str, "deck", ParsedApi::Other, ParsedApi::MemberDeck)
        }
        // --- Category B: Mission ---
        "/kcsapi/api_req_mission/result" => {
            info!("Processing api_req_mission/result (expedition result)");
            parse_typed(
                json_str,
                "mission/result",
                ParsedApi::Other,
                ParsedApi::MissionResult,
            )
        }
        // --- Category B: mapinfo (gauge cache + 基地航空隊 state) ---
        "/kcsapi/api_get_member/mapinfo" => {
            info!("Processing api_get_member/mapinfo");
            match serde_json::from_str::<models::ApiResponse<serde_json::Value>>(json_str) {
                Ok(data) => match data.api_data {
                    Some(api_data) => {
                        let mut gauges = std::collections::HashMap::new();
                        if let Some(map_info) =
                            api_data.get("api_map_info").and_then(|v| v.as_array())
                        {
                            for m in map_info {
                                let map_id =
                                    m.get("api_id").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                                if let Some(gauge) = m.get("api_gauge_num").and_then(|v| v.as_i64())
                                {
                                    if gauge > 0 {
                                        gauges.insert(map_id, gauge as i32);
                                    }
                                }
                            }
                        }
                        let air_bases = serde_json::from_value(api_data).unwrap_or_else(|error| {
                            warn!("Failed to parse mapinfo air bases: {}", error);
                            dto::air_corps::MapInfoAirBases::default()
                        });
                        ParsedApi::MapInfoData { gauges, air_bases }
                    }
                    None => ParsedApi::LogOnly,
                },
                Err(_) => ParsedApi::LogOnly,
            }
        }
        "/kcsapi/api_req_air_corps/set_plane" => {
            info!("Processing api_req_air_corps/set_plane");
            parse_typed(
                json_str,
                "air_corps/set_plane",
                ParsedApi::LogOnly,
                |api_data| ParsedApi::AirCorpsSetPlane {
                    request_body: request_body.to_string(),
                    api_data,
                },
            )
        }
        "/kcsapi/api_req_air_corps/set_action" => {
            info!("Processing api_req_air_corps/set_action");
            ParsedApi::AirCorpsSetAction {
                request_body: request_body.to_string(),
            }
        }
        "/kcsapi/api_req_air_corps/supply" => {
            info!("Processing api_req_air_corps/supply");
            parse_typed(
                json_str,
                "air_corps/supply",
                ParsedApi::LogOnly,
                |api_data| ParsedApi::AirCorpsSupply {
                    request_body: request_body.to_string(),
                    api_data,
                },
            )
        }
        "/kcsapi/api_get_member/base_air_corps" => {
            info!("Processing api_get_member/base_air_corps");
            parse_typed(
                json_str,
                "base_air_corps",
                ParsedApi::LogOnly,
                ParsedApi::BaseAirCorps,
            )
        }
        "/kcsapi/api_req_air_corps/change_name" => {
            info!("Processing api_req_air_corps/change_name");
            ParsedApi::AirCorpsChangeName {
                request_body: request_body.to_string(),
            }
        }
        "/kcsapi/api_req_air_corps/change_deployment_base" => {
            info!("Processing api_req_air_corps/change_deployment_base");
            parse_typed(
                json_str,
                "change_deployment_base",
                ParsedApi::LogOnly,
                |api_data| ParsedApi::AirCorpsChangeDeployment {
                    request_body: request_body.to_string(),
                    api_data,
                },
            )
        }
        "/kcsapi/api_port/airCorpsCondRecoveryWithTimer" => {
            info!("Processing api_port/airCorpsCondRecoveryWithTimer");
            parse_typed(
                json_str,
                "airCorpsCondRecoveryWithTimer",
                ParsedApi::LogOnly,
                |api_data| ParsedApi::AirCorpsCondRecovery {
                    request_body: request_body.to_string(),
                    api_data,
                },
            )
        }
        "/kcsapi/api_req_kaisou/slotset"
        | "/kcsapi/api_req_kaisou/slotset_ex"
        | "/kcsapi/api_req_kaisou/unsetslot_all"
        | "/kcsapi/api_req_kaisou/preset_slot_select"
        | "/kcsapi/api_req_kaisou/remodeling"
        | "/kcsapi/api_req_kousyou/createship"
        | "/kcsapi/api_req_kousyou/createship_speedchange"
        | "/kcsapi/api_req_mission/start" => {
            info!(
                "Processing {} (log only, state refreshed by follow-up API)",
                endpoint
            );
            ParsedApi::LogOnly
        }
        // --- Category B: Practice battles ---
        // Log-only: exercise tracking uses battle_result (process_exercise_result);
        // the battle payloads themselves have no handler yet. If practice support
        // is added to the battle-info overlay, parse these as ParsedApi::Battle
        // and add arms in battle::process_battle.
        "/kcsapi/api_req_practice/battle" | "/kcsapi/api_req_practice/midnight_battle" => {
            info!("Processing {} (log only, practice battle)", endpoint);
            ParsedApi::LogOnly
        }
        ep if battle::is_battle_endpoint(ep) => {
            match serde_json::from_str::<serde_json::Value>(json_str) {
                Ok(v) => ParsedApi::Battle(v),
                Err(e) => {
                    let preview: String = json_str.chars().take(200).collect();
                    error!(
                        "Failed to parse battle API JSON for {}: {} (len={}, first 200: {:?})",
                        ep,
                        e,
                        json_str.len(),
                        preview
                    );
                    ParsedApi::Other
                }
            }
        }
        _ => {
            info!("Unhandled API endpoint: {}", endpoint);
            ParsedApi::Other
        }
    };

    parsed
}

/// Extract senka bonus from clearitemget response's api_bounus array
pub(super) fn extract_senka_from_clearitemget(json_str: &str) -> i64 {
    let parsed: Result<models::ApiResponse<serde_json::Value>, _> = serde_json::from_str(json_str);
    let api_data = match parsed {
        Ok(resp) => match resp.api_data {
            Some(d) => d,
            None => return 0,
        },
        Err(_) => return 0,
    };

    let bounus = match api_data.get("api_bounus").and_then(|v| v.as_array()) {
        Some(arr) => arr,
        None => return 0,
    };

    let mut total_bonus = 0i64;
    for item in bounus {
        if item.is_null() {
            continue;
        }
        let api_type = item.get("api_type").and_then(|v| v.as_i64()).unwrap_or(0);
        if api_type == 18 {
            // Ranking points bonus
            let api_count = item.get("api_count").and_then(|v| v.as_i64()).unwrap_or(1);
            let api_id = item
                .get("api_item")
                .and_then(|i| i.get("api_id"))
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let bonus_per = crate::senka::senka_item_bonus(api_id);
            total_bonus += bonus_per * api_count;
            info!(
                "clearitemget: senka bonus detected: api_id={}, count={}, bonus={}",
                api_id,
                api_count,
                bonus_per * api_count
            );
        }
    }
    total_bonus
}
