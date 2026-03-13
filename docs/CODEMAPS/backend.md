<!-- Generated: 2026-03-13 | Files scanned: 32 | Token estimate: ~950 -->
# Backend (Rust/Tauri)

## Modules (lib.rs → 15 modules)
api, battle_log, ca, commands, cookie, drive_sync, expedition,
game_window, improvement, migration, overlay, proxy, quest_progress, senka, sortie_quest

## Tauri Commands (lib.rs → invoke_handler)
### Game Window (game_window.rs)
open_game_window, close_game_window, set_game_zoom, toggle_game_mute, get_game_mute

### Proxy/Cert (commands.rs, ca.rs)
get_proxy_port, is_ca_installed, install_ca_cert

### Fleet/Ship Data (commands.rs)
get_ship_list, get_equipment_list

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

### Senka
(no direct command — tracked via API events, emits "senka-updated")

### Cache/Resource (commands.rs)
get_cached_resource, get_map_sprite, clear_resource_cache, clear_browser_cache

### Overlay (overlay.rs)
set_overlay_visible, dismiss_overlay, toggle_minimap, get_minimap_enabled
move_minimap, resize_minimap, set_formation_hint_enabled, get_formation_hint_enabled
set_taiha_alert_enabled, get_taiha_alert_enabled
show_expedition_notification, hide_expedition_notification

### Cookie/Browser (cookie.rs, commands.rs)
save_game_cookies, clear_cookies, reset_browser_data

### Drive Sync (commands.rs)
drive_login, drive_logout, get_drive_status, drive_force_sync

### Raw API (commands.rs)
set_raw_api_enabled, get_raw_api_enabled, clear_raw_api

## API Interceptor (api/mod.rs → process_api)
Routing: endpoint string → match block, delegates to sub-modules
Key endpoints:
- api_start2 → MasterData parse (ships, equip, missions)
- api_port → PortData build, fleet/quest/senka emit
- api_req_map/start → sortie start
- api_req_sortie/battle* → api::battle module
- api_req_battle_midnight/* → api::battle module
- api_req_sortie/battleresult → api::battle (result, quest progress)
- api_req_hensei/* → api::fleet (composition change)
- api_req_quest/* → quest start/stop/list
- api_req_kousyou/remodel_slot → improvement tracking
- api_req_practice/* → api::battle (exercise tracking)
- Ship slot/equip updates → api::ship module

## Events (backend → frontend)
port-data, fleet-updated, quest-list-updated, quest-progress-updated
sortie-complete, sortie-update, senka-updated, kancolle-api
proxy-ready, drive-sync-status, drive-data-updated
quest-started, quest-stopped

## State
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
