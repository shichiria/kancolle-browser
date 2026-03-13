<!-- Generated: 2026-03-13 | Files scanned: 54+19 | Token estimate: ~900 -->
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
├── lib.rs             (2539L)  Tauri commands, setup, cookie/cache/overlay mgmt
├── api/
│   ├── mod.rs         (2271L)  API interceptor — process_api(), event emission
│   ├── models.rs       (636L)  GameState, data structures (Ship, Fleet, Port...)
│   ├── dto/                    API response/request DTOs
│   │   ├── battle.rs  (102L)   Battle/quest/remodel response structs
│   │   └── request.rs  (19L)   Hensei/remodel/quest request structs
│   └── tests.rs        (84L)   API handler tests
├── proxy/mod.rs        (429L)  Hudsucker proxy setup, CA cert, per-conn isolation
├── battle_log/mod.rs  (1428L)  BattleLogger — sortie tracking, battle parsing
├── expedition/mod.rs   (501L)  Expedition definitions & great-success check
├── sortie_quest/mod.rs (715L)  Sortie quest definitions, map recommendations
├── quest_progress/mod.rs(841L) Quest progress tracking, reset logic
├── senka/mod.rs        (846L)  Ranking/senka calculation & tracking
├── improvement/mod.rs  (338L)  Equipment improvement list
└── drive_sync/                 Google Drive sync
    ├── mod.rs          (113L)  SyncManifest, load/save
    ├── auth.rs         (134L)  OAuth2 flow
    ├── engine.rs       (525L)  Sync engine (tokio task + mpsc)
    └── files.rs        (281L)  GDrive file operations

src/
├── main.tsx              (9L)  React entry
├── App.tsx             (453L)  Root component — tab orchestration, event listeners
├── App.css             (231L)  Root layout styles (toolbar, tabs)
├── types/              (385L)  TypeScript type definitions (10 files)
├── utils/              (187L)  Formatting, color, map utilities (4 files)
└── components/                 Feature-based component modules
    ├── common/         (382L)  Shared UI: HpBar, BattleHpBar, ClearButton, DateRangePicker
    ├── homeport/      (1961L)  Fleet panels, expedition/quest checkers
    ├── battle/        (1270L)  Battle log viewer, map route, node detail
    ├── ships/          (326L)  Ship list with sort/filter
    ├── equips/         (124L)  Equipment list
    ├── improvement/    (386L)  Equipment improvement tracking
    └── settings/       (466L)  App config, drive sync, cache controls
```

## Totals
- Rust: ~11,810 lines (19 files)
- Frontend: ~6,247 lines (54 files)
- Deps: Tauri 2, hudsucker, serde, tokio, google-drive3, chrono, image
