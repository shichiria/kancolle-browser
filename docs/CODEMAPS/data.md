<!-- Generated: 2026-03-13 | Files scanned: 10 | Token estimate: ~800 -->
# Data Layer

## Static Data (bundled in build)
```
src-tauri/data/
├── equipment_upgrades.json  (619KB) Improvement recipes & helpers
├── expeditions.json         (21KB)  Expedition definitions & conditions
├── map_recommendations.json (28KB)  Map route recommendations
└── sortie_quests.json       (744KB) Sortie quest definitions & conditions

src/data/
└── edges.json               Map routing data (KC3Kai format, node labels)
```

## Runtime Storage (No database — file-based JSON)
AppData: com.eo.kancolle-browser
```
<app_local_data>/
├── sync/                         (Google Drive synced)
│   ├── improved_equipment.json   Equipment improvement history
│   ├── quest_progress.json       Quest progress state
│   ├── senka_log.json            Senka tracking data
│   ├── formation_memory.json     Map cell → formation ID memory
│   ├── battle_logs/              Battle log JSON files (directory)
│   └── raw_api/                  Raw API response logs (directory)
├── local/                        (Device-local only)
│   ├── cache/                    /kcs2/* resource cache (img, json)
│   ├── game_muted                Mute state flag
│   ├── formation_hint_enabled    Hint toggle flag
│   ├── taiha_alert_enabled       Taiha alert toggle flag
│   ├── minimap_enabled           Minimap toggle flag
│   ├── minimap_position.json     Minimap [x, y] position
│   ├── minimap_size.json         Minimap [w, h] size
│   ├── dmm_cookies.json          DMM session cookies
│   └── game-webview/             WebView persistent data
├── sync_manifest.json            Drive sync state
└── google_drive_token.json       OAuth2 token
```

## Data Migration (migration.rs)
On startup: migrates old flat layout → sync/ + local/ structure.

## Google Drive Sync
Engine: tokio task + mpsc channel (5-min polling)
Folder: "KanColle Browser Sync" on user's Drive
Synced: improved_equipment.json, quest_progress.json, senka_log.json,
        formation_memory.json, battle_logs/, raw_api/
Conflict resolution: newer modified_time wins
Events: drive-sync-status, drive-data-updated

## Key Data Flows
1. API intercept → parse → GameStateInner (in-memory)
2. Port API → PortSummary → emit "port-data" → React state
3. Battle result → BattleLogger → save to sync/battle_logs/
4. Quest progress → save to sync/quest_progress.json → Drive sync
5. Senka → SenkaTracker → save to sync/senka_log.json → Drive sync

## Cache
/kcs2/* resources cached via proxy to local/cache/
get_cached_resource: JSON→string, image→base64 data URI
Map sprites: TexturePacker sheet + _info.json coordinates
