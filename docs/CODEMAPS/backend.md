<!-- Generated: 2026-05-05 | Updated: 2026-07-11 | Token estimate: ~1150 -->
# Backend (Rust/Tauri)

## Modules (lib.rs → 20 modules)
action_log, api, battle_log, ca, commands, cookie, drive_sync, expedition,
game_window, improvement, kantai, management, migration, mouse_hook,
overlay, proxy, quest_progress, quests, senka, sortie_quest, ui_event

api/ サブモジュール: mod, models, battle, ship, fleet, formation, minimap,
air_corps (基地航空隊), battle_info (戦闘情報オーバーレイ), dto/, tests

## Tauri Commands (lib.rs → invoke_handler)
### Game Window (`game_window/{mod,windows,macos}.rs`)
open_game_window, close_game_window, set_game_zoom, toggle_game_mute, get_game_mute

### Management Window (management.rs) — React SPA hide/show
show_management_window, hide_management_window, toggle_management_window

### Kantai Window (kantai.rs) — Fleet panel hide/show
show_kantai_window, hide_kantai_window, toggle_kantai_window

### Quests Window (quests.rs) — Quest view hide/show
show_quests_window, hide_quests_window, toggle_quests_window

### Proxy/Cert (commands.rs, ca.rs)
get_proxy_port, is_ca_installed, install_ca_cert

### Debug (commands.rs)
get_action_log, get_current_screen, get_current_fleet, get_quest_filters

### Fleet/Ship Data (commands.rs)
get_ship_list, get_equipment_list, get_air_bases

### Expedition (commands.rs)
get_expeditions → expedition::get_all_expeditions()
check_expedition_cmd → expedition::check_expedition()

### Sortie Quest (commands.rs)
get_sortie_quests, check_sortie_quest_cmd, get_active_quest_ids
get_map_recommendations, check_map_recommendation_cmd

### Quest Progress (commands.rs)
get_quest_progress, update_quest_progress, clear_quest_progress

### Battle Log (commands.rs)
get_battle_logs, clear_battle_logs

### Improvement (commands.rs)
get_improvement_list, clear_improved_history

### Cache/Resource (commands.rs)
get_cached_resource, get_map_sprite, clear_resource_cache, clear_browser_cache

### Overlay (overlay.rs)
set_overlay_visible, dismiss_overlay, toggle_minimap, get_minimap_enabled
move_minimap, resize_minimap, set_formation_hint_enabled, get_formation_hint_enabled
set_taiha_alert_enabled, get_taiha_alert_enabled
set_battle_info_enabled, get_battle_info_enabled
show_expedition_notification, hide_expedition_notification

### Cookie/Browser (cookie.rs, commands.rs)
save_game_cookies, clear_cookies, reset_browser_data

### Drive Sync (commands.rs)
drive_login, drive_logout, get_drive_status, drive_force_sync

### Raw API (commands.rs)
set_raw_api_enabled, get_raw_api_enabled, clear_raw_api

## Click / Screen Tracking (mouse_hook.rs + ui_event/)
- `mouse_hook` (Win-only): SetWindowsHookExW(WH_MOUSE_LL) on dedicated thread,
  emits `GameClick { x, y }` for left-clicks within game window canvas
- `consume_clicks`: reads `AppState.current_screen`, calls `ui_event::detect_event`,
  updates current_screen / current_fleet / current_quest_period / current_quest_category
  on Navigate / SideMenuClick / FleetSelect / QuestFilter / QuestCategoryFilter,
  emits `screen-changed` / `fleet-view-changed` / `quest-filters-changed` / `click-event` / `click-screenshot`
- `ui_event::detect_event(screen, x, y) -> UiEvent`: coordinate → semantic mapping
  per known game screen (calibrated to user's actual UI as of 2026-05-05;
  see [docs/KNOWLEDGE/ui-regions.md](../KNOWLEDGE/ui-regions.md))

## API Interceptor (api/mod.rs → process_api)
Routing: endpoint string → ParsedApi enum, delegates to sub-modules.
Two-tier: Category A (stateful — port, battle, fleet) + Category B (info-only — ranking, useitem).

API → Screen mapping (`screen_from_api`):
- api_port/port → Homeport
- api_start2/getData → Unknown (game-restart reset)
- api_req_hensei/* → FleetComposition
- api_req_kaisou/* → Remodel
- api_req_hokyu/charge → Resupply
- api_req_nyukyo/{start,speedchange} → RepairDockSelect
- api_req_kousyou/{createship,destroyship,destroyitem2,getship,…} → Factory
- api_req_kousyou/createitem → FactoryDevelop
- api_get_member/questlist, api_req_quest/* → QuestList
- api_get_member/mapinfo → SortieSelect

Other key endpoints:
- api_start2 → MasterData parse (ships, equip, missions)
- api_port → PortData build (typed DTOs: dto::member), fleet/quest/senka emit
- api_req_map/start → sortie start
- api_req_sortie/battle* → api::battle module
- api_req_battle_midnight/* → api::battle module
- api_req_sortie/battleresult → api::battle (result, quest progress)
- api_req_hensei/* → api::fleet (composition change)
- api_req_quest/* → quest start/stop/list
- api_req_kousyou/remodel_slot → improvement tracking
- api_req_practice/* → api::battle (exercise tracking)
- api_req_ranking/mxltvkpyuklh → ranking decryption (dto::ranking)
- Ship slot/equip updates → api::ship module

## DTOs (api/dto/)
- battle.rs — Battle/quest/remodel response structs
- member.rs — Typed port/slot_item/ndock API response parsing
- ranking.rs — Ranking API entry structs for senka decryption
- request.rs — Hensei/remodel/quest request body structs

## Events (backend → frontend)
### Game state
port-data, fleet-updated, quest-list-updated, quest-progress-updated
sortie-complete, sortie-update, senka-updated, kancolle-api
proxy-ready, drive-sync-status, drive-data-updated
quest-started, quest-stopped, air-base-updated

### Click / Screen tracking
screen-changed (payload: `"Homeport"` etc.)
fleet-view-changed (payload: 1-4 / 5=他 / null)
quest-filters-changed (payload: `{period, category}`)
click-event (payload: `{ts, x, y, screen, event}`)
click-screenshot (payload: `{ts, x, y, image: data:image/png;base64,…}`)

## State (AppState)
- proxy_port, game_muted
- formation_hint_enabled, taiha_alert_enabled, minimap_enabled, battle_info_enabled
- expedition_notify_visible
- formation_hint_rect, game_zoom, minimap_position, minimap_size
- last_battle_info
- **current_screen** (`ui_event::Screen`) — inferred game screen
- **current_fleet** (`Option<u32>`) — selected fleet (1-4, 5=他) on fleet-bearing screens
- **current_quest_period** (`Option<String>`) — QuestList left filter
- **current_quest_category** (`Option<String>`) — QuestList top filter

GameState: Arc<RwLock<GameStateInner>>
├── master: MasterData (ship/equip/mission definitions)
├── ships: HashMap<i64, ShipInfo>
├── slot_items: HashMap<i64, PlayerSlotItem>
├── fleets: Vec<Fleet>
├── profile: UserProfile (admiral name, level, exp)
├── battle_logger: BattleLogger
├── senka_tracker: SenkaTracker
├── quest_progress: QuestProgressState
└── sortie: SortieState

## Action Log (action_log.rs, dev only)
JSONL ring buffer + file output at `%LOCALAPPDATA%\com.eo.kancolle-browser\local\action_logs\actions_YYYYMMDD.jsonl`.
Categories: `API` / `API_PARSED` / `Event` / `State` / `Command` / `Click` / `Screen` / `Fleet` / `Quest`.
Mouse-hook clicks also save full + cropped PNG screenshots in `screenshots/` (rate-limited to 1 in flight).

## Tests (api/tests.rs — 1,764L + ui_event/tests.rs)
- API handler tests covering port parsing, fleet updates, battle processing,
  quest progress, senka tracking, DTO deserialization
- ui_event coordinate tests calibrated to actual user clicks (see
  `real_session_*` tests for ground-truth assertions)
