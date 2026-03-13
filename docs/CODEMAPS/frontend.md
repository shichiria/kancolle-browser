<!-- Generated: 2026-03-13 | Files scanned: 57 | Token estimate: ~950 -->
# Frontend (React/TS)

## Structure
Modular SPA: App.tsx (457L) orchestrates tab components.
Types (395L), utils (213L), constants (15L), and 7 component modules.
No routing library, no external state management.

## Component Hierarchy
```
App (457L) — state, event listeners, tab switching
├── HomeportTab (215L)
│   ├── FleetPanel (142L) — per-fleet ship/equip display
│   │   ├── HpBar (common)
│   │   ├── ExpeditionChecker (111L)
│   │   ├── MapRecommendationChecker (115L)
│   │   └── SortieQuestChecker (366L)
│   └── QuestProgressDisplay (99L)
├── BattleTab (127L)
│   ├── DateRangePicker (138L, common)
│   └── BattleDetailView (121L)
│       ├── MapRouteView (260L)
│       └── BattleNodeDetail (152L)
│           └── BattleHpBar (49L, common)
├── ShipListTab (180L)
├── EquipListTab (125L)
├── ImprovementTab (166L)
└── SettingsTab (229L)
    └── ClearButton (47L, common)
```

## State (React useState in App.tsx)
Key state variables:
- portData: PortData (ships, fleets, materials, docks, quests)
- battleLogs: SortieRecord[] + battleDateFrom/To filters
- driveStatus: DriveStatus (loggedIn, syncing, lastSync, error)
- activeTab: TabId
- gameOpen: boolean
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

## Utils (src/utils/, 4 modules)
format.ts — getRankName, formatRemaining, fmtDate, toDateStr, daysInMonth
color.ts — hpColor, condColor, condBgClass
map.ts — getNodeLabel, buildPredeckUrl, CELL_COLORS
index.ts — barrel re-export

## CSS Organization
Each component has a paired CSS file (1:1). Total ~2,350 lines.
App.css (231L) — root layout only. Component styles colocated.
ListTable.css (151L) — shared table styles (common/).
