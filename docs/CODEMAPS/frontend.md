<!-- Generated: 2026-05-05 | Updated: 2026-07-11 | Token estimate: ~1150 -->
# Frontend (React/TS)

## Multi-window, single bundle
Single React entry (`main.tsx` → `App.tsx`) loaded by every Tauri window;
the App component branches on `getCurrentWindow().label` to pick the view:

| Window label | Visible by | Renders | Notes |
|--------------|-----------|---------|-------|
| `management` | 📊 button on game control bar | Full SPA (toolbar + tabs) | hide-on-close |
| `kantai` | ⚓ button on game control bar | `<KantaiView/>` (fleet + 🛩 air base) | hide-on-close, own UI zoom |
| `quests` | 📜 button on game control bar | `<QuestTab/>` (quest-only) | hide-on-close |

`VIEW_MODE` is decided once at startup (immutable for the window's lifetime).

## Component Hierarchy
```
App (~440L) — state, event listeners, view-mode dispatch
├── KantaiView (~180L) — fleet tabs + 🛩 AirBaseTab + FleetPanel + UI zoom slider
│   ├── FleetPanel (homeport, reused)
│   └── AirBaseTab (~270L) — 基地航空隊 (get_air_bases + air-base-updated)
├── QuestTab (~290L) — quests window view (カテゴリ/海域別、ピン留め pinned_quests)
└── (management mode)
    ├── HomeportTab
    │   ├── FleetPanel
    │   │   ├── HpBar, ExpeditionChecker,
    │   │   └── MapRecommendationChecker, SortieQuestChecker
    │   └── QuestProgressDisplay
    ├── BattleTab
    │   ├── DateRangePicker
    │   └── BattleDetailView
    │       ├── MapRouteView
    │       └── BattleNodeDetail → BattleHpBar
    ├── ShipListTab
    ├── EquipListTab
    ├── ImprovementTab
    ├── DebugTab — current_screen + recent clicks (with crops) + recent APIs
    └── SettingsTab → ClearButton
```

## State (React useState in App.tsx)
- portData: PortData (ships, fleets, materials, docks, quests)
- battleLogs: SortieRecord[] + battleDateFrom/To filters
- driveStatus: DriveStatus
- activeTab: TabId (now includes `"debug"`)
- fleetData: FleetData[]
- questProgress: Map<number, QuestProgressSummary>
- senkaSummary: SenkaSummary

## Event Listeners (Tauri → React, in App.tsx)
proxy-ready, port-data, fleet-updated, sortie-complete, sortie-update
quest-list-updated, quest-progress-updated, senka-updated
drive-sync-status, drive-data-updated, kancolle-api

### KantaiView additionally listens
- fleet-view-changed (number | null) — auto-switches selected fleet (1-4)
- air-base-updated (AirBase[]) — refreshes 🛩 AirBaseTab

### DebugTab additionally listens
- screen-changed (string)
- fleet-view-changed (number | null)
- quest-filters-changed ({period, category})
- click-event ({ts, x, y, screen, event})
- click-screenshot ({ts, x, y, image: data:image/png;base64,…})
- kancolle-api ({endpoint})

## Types (src/types/, 10 files)
common.ts — ConditionResult, **TabId** (`"homeport" | "battle" | "improvement" | "ships" | "equips" | "options" | "debug"`), DriveStatus
port.ts — ShipData, FleetData, PortData, ApiLogEntry
battle.ts — BattleNode, SortieRecord, BattleLogsResponse, MapSprites
quest.ts — SortieQuestDef, ActiveQuestDetail, QuestProgressSummary
expedition.ts — ExpeditionDef, ExpeditionCheckResult, MapRecommendationDef
ship.ts — ShipListItem, ShipListResponse, ShipSortKey
equipment.ts — EquipListItem, EquipListResponse
improvement.ts — ImprovementItem, ImprovementListResponse
senka.ts — SenkaSummary
airbase.ts — AirBase, AirBasePlane, AirBaseAttackWave, AirBaseDistance

## Component modules
- common/ — HpBar, BattleHpBar, ClearButton, DateRangePicker, ListTable
- homeport/ — FleetPanel, ExpeditionChecker, MapRecommendationChecker, SortieQuestChecker, QuestProgressDisplay, HomeportTab
- battle/ — BattleTab, BattleDetailView, MapRouteView, BattleNodeDetail
- ships/ — ShipListTab
- equips/ — EquipListTab
- improvement/ — ImprovementTab
- settings/ — SettingsTab
- **kantai/** — KantaiView (fleet tabs + FleetPanel + zoom slider, persisted via localStorage `kc-kantai-fleet-id` / `kc-kantai-ui-zoom`) + AirBaseTab
- **quests/** — QuestTab (quests window view; カテゴリ/海域別表示 + pinned_quests)
- **debug/** — DebugTab (current_screen card + clicks table with crops + API table; pause + clear)

## Utils (src/utils/, 4 modules + 2 test files)
format.ts — getRankName, formatRemaining, fmtDate, toDateStr, daysInMonth
color.ts — hpColor, condColor, condBgClass
map.ts — getNodeLabel, buildPredeckUrl, CELL_COLORS
index.ts — barrel re-export

## CSS
Each component has a paired CSS file. App.css holds root layout (toolbar, tabs).
KantaiView.css adds dark zoom-bar at the bottom (range slider + reset).
DebugTab.css uses fixed-table layout with hover-to-enlarge thumbnails.
