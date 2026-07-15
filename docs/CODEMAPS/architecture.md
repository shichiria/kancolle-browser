<!-- Generated: 2026-03-22 | Updated: 2026-07-11 | Token estimate: ~1000 -->
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
├── lib.rs             (480L)   Tauri setup, invoke_handler registration, proxy launch, ExitRequested cookie save
├── commands.rs       (1030L)   Tauri commands: data queries, cache, drive sync, raw API, air bases
├── game_window/                Game window common flow + Windows/macOS WebView integration
├── overlay.rs         (441L)   Overlay: minimap, formation hint, taiha alert, expedition notif, battle info
├── cookie.rs          (240L)   DMM cookie collect/save/restore (macOS: native set_cookie)
├── ca.rs              (126L)   CA certificate install/check
├── migration.rs        (81L)   Data directory migration (flat → sync/local)
├── kantai.rs           (~45L)  Kantai window show/hide/toggle
├── management.rs       (~45L)  Management window show/hide/toggle
├── quests.rs           (~45L)  Quests window show/hide/toggle
├── mouse_hook.rs      (585L)   Win-only click hook → screen/fleet/quest tracking + screenshots
├── action_log.rs      (~180L)  Dev-only JSONL action log
├── ui_event/          (960L)   Screen enum + click coordinate → semantic event (incl. tests)
├── api/
│   ├── mod.rs        (~1650L)  API interceptor — process_api(), Category A+B handlers
│   ├── models.rs      (720L)   GameState, data structures (Ship, Fleet, Port, AirBase...)
│   ├── battle.rs      (588L)   Battle/sortie/practice API handlers
│   ├── air_corps.rs   (649L)   基地航空隊追跡 — parse/apply, 損失比例配分, air-base-updated
│   ├── battle_info.rs (287L)   戦闘情報オーバーレイ — 交戦形態/制空/LBAS波ラベル
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
├── App.tsx             (439L)  Root component — VIEW_MODE (window label) dispatch, event listeners
├── App.css             (231L)  Root layout styles (toolbar, tabs)
├── constants.ts                Storage key constants + air superiority labels
├── types/                      TypeScript type definitions (11 files, incl. airbase.ts)
├── utils/              (396L)  Formatting, color, map utilities (4+2 test files)
└── components/                  Feature-based component modules
    ├── common/         (540L)  Shared UI: HpBar, BattleHpBar, ClearButton, DateRangePicker, ListTable
    ├── homeport/      (1961L)  Fleet panels, expedition/quest checkers
    ├── battle/        (1280L)  Battle log viewer, map route, node detail
    ├── ships/          (191L)  Ship list with sort/filter
    ├── equips/         (126L)  Equipment list
    ├── improvement/    (386L)  Equipment improvement tracking
    ├── settings/       (463L)  App config, drive sync, cache controls
    ├── kantai/         (~460L) KantaiView (fleet window) + AirBaseTab (基地航空隊)
    ├── quests/         (~290L) QuestTab (quests window)
    └── debug/          (~280L) DebugTab (click/screen/API monitor)
```

## Totals
- Rust: ~13,646 lines (34 files) — incl. 1,764L tests
- Frontend: ~6,501 lines (59 files) — incl. 183L tests
- Deps: Tauri 2, hudsucker, serde, tokio, google-drive3, chrono, image
