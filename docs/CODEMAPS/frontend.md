<!-- Generated: 2026-03-13 | Files scanned: 54 | Token estimate: ~950 -->
# Frontend (React/TS)

## Structure
Modular SPA: App.tsx (453L) orchestrates tab components.
Types (385L), utils (187L), and 7 component modules.
No routing library, no external state management.

## Component Hierarchy
```
App (453L) — state, event listeners, tab switching
├── HomeportTab (213L)
│   ├── FleetPanel (142L) — per-fleet ship/equip display
│   │   ├── HpBar (common)
│   │   ├── ExpeditionChecker (106L)
│   │   ├── MapRecommendationChecker (110L)
│   │   └── SortieQuestChecker (370L)
│   └── QuestProgressDisplay (98L)
├── BattleTab (127L)
│   ├── DateRangePicker (138L, common)
│   └── BattleDetailView (121L)
│       ├── MapRouteView (260L)
│       └── BattleNodeDetail (152L)
│           └── BattleHpBar (49L, common)
├── ShipListTab (165L)
├── EquipListTab (123L)
├── ImprovementTab (165L)
└── SettingsTab (232L)
    └── ClearButton (47L, common)
```

## State (React useState in App.tsx)
Key state variables:
- portData: PortData (ships, fleets, materials, docks, quests)
- battleLogs: SortieRecord[] + battleDateFrom/To filters
- driveStatus: DriveStatus (loggedIn, syncing, lastSync, error)
- activeTab: TabId
- gameWindowOpen: boolean
- fleetData: FleetData[]
- questProgress: Map<number, QuestProgressSummary>
- senkaSummary: SenkaSummary

## Event Listeners (Tauri → React, in App.tsx)
proxy-ready → set proxy port
port-data → update portData
fleet-updated → update fleetData
sortie-complete → prepend to battleLogs
sortie-update → update current sortie
quest-list-updated → update quest display
quest-progress-updated → update progress map
senka-updated → update senka charts
drive-sync-status → update sync UI
drive-data-updated → reload game state (uses ref to avoid stale closure)
kancolle-api → debug log

## Types (src/types/, 10 files)
common.ts — ConditionResult, TabId, DriveStatus
port.ts — ShipData, FleetData, PortData, ApiLogEntry
battle.ts — BattleNode, SortieRecord, BattleLogsResponse, MapSprites
quest.ts — SortieQuestDef, ActiveQuestDetail, QuestProgressSummary
expedition.ts — ExpeditionDef, ExpeditionCheckResult, MapRecommendationDef
ship.ts — ShipListItem, ShipListResponse, ShipSortKey
equipment.ts — EquipListItem, EquipListResponse
improvement.ts — ImprovementItem, ImprovementListResponse
senka.ts — SenkaSummary

## Utils (src/utils/, 3 modules)
format.ts — getRankName, formatRemaining, fmtDate, toDateStr, daysInMonth
color.ts — hpColor, condColor, condBgClass
map.ts — getNodeLabel, buildPredeckUrl, CELL_COLORS

## CSS Organization
Each component has a paired CSS file (1:1). Total ~2,200 lines.
App.css (231L) — root layout only. Component styles colocated.
