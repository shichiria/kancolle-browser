<!-- Generated: 2026-03-22 | Files scanned: 59+34 | Token estimate: ~950 -->
# Architecture

## Overview
Tauri v2 desktop app — Rust backend + React/TS frontend (modular SPA).
Intercepts KanColle game API via HTTP proxy, provides fleet/battle/quest tracking UI.

## Data Flow
```
Browser (WebView2/WKWebView)
  │ HTTP request
  ▼
Proxy (hudsucker, macOS:19080)
  │ intercept /kcsapi/*
  ├──► api::process_api() ──► GameState (Arc<RwLock<GameStateInner>>)
  │                               │ emit events
  │                               ▼
  │                           React Frontend (listen → state update → render)
  │
  └──► Cache (/kcs2/* → local/cache/)
```

## Module Map
```
src-tauri/src/
├── main.rs              (6L)   Entry point
├── lib.rs             (300L)   Tauri setup, invoke_handler registration, proxy launch
├── commands.rs        (974L)   Tauri commands: data queries, cache, drive sync, raw API
├── game_window.rs     (347L)   Game window open/close, zoom, mute
├── overlay.rs         (345L)   Overlay: minimap, formation hint, taiha alert, expedition notif
├── cookie.rs          (149L)   Cookie save/load/clear
├── ca.rs              (126L)   CA certificate install/check
├── migration.rs        (81L)   Data directory migration (flat → sync/local)
├── api/
│   ├── mod.rs        (1272L)   API interceptor — process_api(), Category A+B handlers
│   ├── models.rs      (638L)   GameState, data structures (Ship, Fleet, Port...)
│   ├── battle.rs      (512L)   Battle/sortie/practice API handlers
│   ├── ship.rs        (532L)   Ship slot/equip update handlers
│   ├── fleet.rs       (212L)   Fleet composition change handlers
│   ├── formation.rs   (132L)   Formation hint overlay logic
│   ├── minimap.rs      (47L)   Minimap data sender
│   ├── dto/
│   │   ├── mod.rs       (4L)   DTO module re-exports
│   │   ├── battle.rs   (82L)   Battle/quest/remodel response structs
│   │   ├── member.rs  (186L)   Port/slot_item/ndock typed DTOs
│   │   ├── ranking.rs  (25L)   Ranking API response structs
│   │   └── request.rs  (19L)   Hensei/remodel/quest request structs
│   └── tests.rs      (1764L)   API handler tests (comprehensive)
├── proxy/mod.rs       (429L)   Hudsucker proxy setup, CA cert, per-conn isolation
├── battle_log/
│   ├── mod.rs         (690L)   BattleLogger — sortie tracking, result processing
│   ├── parser.rs      (634L)   Battle data parsing (damage, formation, drops)
│   └── storage.rs     (167L)   Battle log file I/O
├── expedition/mod.rs  (505L)   Expedition definitions & great-success check
├── sortie_quest/mod.rs(723L)   Sortie quest definitions, map recommendations
├── quest_progress/mod.rs(841L) Quest progress tracking, reset logic
├── senka/mod.rs       (822L)   Ranking/senka calculation & tracking
├── improvement/mod.rs (338L)   Equipment improvement list
└── drive_sync/                  Google Drive sync
    ├── mod.rs         (112L)   SyncManifest, load/save
    ├── auth.rs        (134L)   OAuth2 flow
    ├── engine.rs      (525L)   Sync engine (tokio task + mpsc)
    └── files.rs       (281L)   GDrive file operations

src/
├── main.tsx              (9L)  React entry
├── App.tsx             (458L)  Root component — tab orchestration, event listeners
├── App.css             (231L)  Root layout styles (toolbar, tabs)
├── constants.ts         (15L)  Storage key constants
├── types/              (395L)  TypeScript type definitions (10 files)
├── utils/              (396L)  Formatting, color, map utilities (4+2 test files)
└── components/                  Feature-based component modules
    ├── common/         (540L)  Shared UI: HpBar, BattleHpBar, ClearButton, DateRangePicker, ListTable
    ├── homeport/      (1961L)  Fleet panels, expedition/quest checkers
    ├── battle/        (1280L)  Battle log viewer, map route, node detail
    ├── ships/          (191L)  Ship list with sort/filter
    ├── equips/         (126L)  Equipment list
    ├── improvement/    (386L)  Equipment improvement tracking
    └── settings/       (463L)  App config, drive sync, cache controls
```

## Totals
- Rust: ~13,646 lines (34 files) — incl. 1,764L tests
- Frontend: ~6,501 lines (59 files) — incl. 183L tests
- Deps: Tauri 2, hudsucker, serde, tokio, google-drive3, chrono, image
