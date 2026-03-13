<!-- Generated: 2026-03-13 | Files scanned: 57+32 | Token estimate: ~950 -->
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
│   ├── mod.rs         (917L)   API interceptor — process_api(), port/quest/senka/remodel
│   ├── models.rs      (636L)   GameState, data structures (Ship, Fleet, Port...)
│   ├── battle.rs      (474L)   Battle/sortie/practice API handlers
│   ├── ship.rs        (458L)   Ship slot/equip update handlers
│   ├── fleet.rs       (221L)   Fleet composition change handlers
│   ├── formation.rs   (132L)   Formation hint overlay logic
│   ├── minimap.rs      (47L)   Minimap data sender
│   ├── dto/
│   │   ├── mod.rs       (2L)   DTO module re-exports
│   │   ├── battle.rs  (102L)   Battle/quest/remodel response structs
│   │   └── request.rs  (19L)   Hensei/remodel/quest request structs
│   └── tests.rs        (84L)   API handler tests
├── proxy/mod.rs       (429L)   Hudsucker proxy setup, CA cert, per-conn isolation
├── battle_log/
│   ├── mod.rs         (683L)   BattleLogger — sortie tracking, result processing
│   ├── parser.rs      (601L)   Battle data parsing (damage, formation, drops)
│   └── storage.rs     (167L)   Battle log file I/O
├── expedition/mod.rs  (501L)   Expedition definitions & great-success check
├── sortie_quest/mod.rs(715L)   Sortie quest definitions, map recommendations
├── quest_progress/mod.rs(841L) Quest progress tracking, reset logic
├── senka/mod.rs       (846L)   Ranking/senka calculation & tracking
├── improvement/mod.rs (338L)   Equipment improvement list
└── drive_sync/                  Google Drive sync
    ├── mod.rs         (112L)   SyncManifest, load/save
    ├── auth.rs        (134L)   OAuth2 flow
    ├── engine.rs      (525L)   Sync engine (tokio task + mpsc)
    └── files.rs       (281L)   GDrive file operations

src/
├── main.tsx              (9L)  React entry
├── App.tsx             (457L)  Root component — tab orchestration, event listeners
├── App.css             (231L)  Root layout styles (toolbar, tabs)
├── constants.ts         (15L)  Storage key constants
├── types/              (395L)  TypeScript type definitions (10 files)
├── utils/              (213L)  Formatting, color, map utilities (4 files)
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
- Rust: ~11,593 lines (32 files)
- Frontend: ~6,530 lines (57 files)
- Deps: Tauri 2, hudsucker, serde, tokio, google-drive3, chrono, image
