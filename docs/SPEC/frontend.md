<!-- AUTO-GENERATED from source code -->

# フロントエンド設計書

## 概要

React + TypeScript によるSPA。Tauri v2 の `invoke` / `listen` API を通じてRustバックエンドと通信する。メインウィンドウ (情報パネル) とゲームウィンドウ (別ウィンドウ) の2画面構成。

---

## React コンポーネント階層

```
App (src/App.tsx)
|
+-- Toolbar
|     +-- [CA Install Button]
|     +-- [Open/Close Game Button]
|     +-- [Status Indicators]
|
+-- TabBar
|     +-- TabButton x 6 (母港/戦闘/改修/艦娘/装備/設定)
|
+-- MainContent
      |
      +-- HomeportTab (src/components/homeport/HomeportTab.tsx)
      |     +-- TopBar
      |     |     +-- AdmiralSection (提督情報 + 戦果)
      |     |     +-- ResourcesSection (資源8種)
      |     |     +-- NdockSection (入渠ドック)
      |     +-- FleetsArea
      |     |     +-- FleetPanel x 4 (src/components/homeport/FleetPanel.tsx)
      |     |           +-- FleetHeader (艦隊名 + 速力タグ + 遠征状態)
      |     |           +-- ShipRow x N
      |     |           |     +-- HpBar (src/components/common/HpBar.tsx)
      |     |           |     +-- [ダメコンアイコン]
      |     |           |     +-- [司令部施設バッジ]
      |     |           |     +-- [特殊装備アイコン]
      |     |           |     +-- [先制対潜アイコン]
      |     |           +-- MapRecommendationChecker (第1艦隊のみ)
      |     |           |     (src/components/homeport/MapRecommendationChecker.tsx)
      |     |           +-- SortieQuestChecker (第1艦隊のみ)
      |     |           |     (src/components/homeport/SortieQuestChecker.tsx)
      |     |           |     +-- QuestProgressDisplay
      |     |           |           (src/components/homeport/QuestProgressDisplay.tsx)
      |     |           +-- ExpeditionChecker (第2-4艦隊)
      |     |                 (src/components/homeport/ExpeditionChecker.tsx)
      |     +-- ApiLogPanel (条件付き表示)
      |
      +-- BattleTab (src/components/battle/BattleTab.tsx)
      |     +-- FilterBar
      |     |     +-- DateRangePicker (src/components/common/DateRangePicker.tsx)
      |     |     +-- [プリセットボタン: 今日/今月/全て]
      |     |     +-- [マップフィルター]
      |     +-- BattleRecordList
      |     |     +-- BattleRecordRow x N
      |     +-- BattleDetailView (選択時, src/components/battle/BattleDetailView.tsx)
      |           +-- BattleDetailHeader
      |           +-- FleetCompSection
      |           +-- SplitContainer (上下分割, ドラッグリサイズ)
      |                 +-- MapRouteView (上, src/components/battle/MapRouteView.tsx)
      |                 |     +-- [マップ背景画像]
      |                 |     +-- [ルートスプライト]
      |                 |     +-- SVG (ノード円 + ラベル)
      |                 +-- BattleNodeDetail x N (下, src/components/battle/BattleNodeDetail.tsx)
      |                       +-- NodeHeader (セル名 + イベント + ランク + ドロップ)
      |                       +-- FormationRow
      |                       +-- AirBattleRow
      |                       +-- BattleFleetsRow
      |                             +-- BattleHpBar x N (src/components/common/BattleHpBar.tsx)
      |
      +-- ImprovementTab (src/components/improvement/ImprovementTab.tsx)
      |     +-- ImprovementHeader (曜日 + 2番艦 + 件数)
      |     +-- TypeFilters (装備種別トグル)
      |     +-- ImprovementList
      |           +-- ImpRow x N (装備名 + 消費装備 + 担当艦)
      |
      +-- ShipListTab (src/components/ships/ShipListTab.tsx)
      |     +-- ListHeader (艦数)
      |     +-- StypeFilters (艦種トグル)
      |     +-- ListTable (ソータブル13列テーブル)
      |
      +-- EquipListTab (src/components/equips/EquipListTab.tsx)
      |     +-- ListHeader (装備種数)
      |     +-- TypeFilters (装備種トグル)
      |     +-- ListTable (5列テーブル)
      |
      +-- SettingsTab (src/components/settings/SettingsTab.tsx)
            +-- DisplaySection (UIサイズスライダー)
            +-- GoogleDriveSection (ログイン/同期)
            +-- DeveloperSection (APIログ/全ログ保存)
            +-- DataClearSection
                  +-- ClearButton x 8 (src/components/common/ClearButton.tsx)
```

---

## 状態管理パターン

### App.tsx の状態 (useState)

全てのグローバル状態は `App` コンポーネントで管理し、子コンポーネントへ props として渡す。
グローバルな状態管理ライブラリ (Redux, Zustand等) は使用していない。

| 状態 | 型 | 初期値 | 用途 |
|------|-----|-------|------|
| `proxyPort` | `number` | `0` | プロキシポート番号 |
| `portData` | `PortData \| null` | `null` | 母港データ (提督/資源/艦隊/入渠) |
| `senkaData` | `SenkaSummary \| null` | `null` | 戦果データ |
| `senkaCheckpoint` | `boolean` | `false` | ランキング更新通過フラグ (10秒表示) |
| `apiLog` | `ApiLogEntry[]` | `[]` | API通信ログ (最大200件) |
| `gameOpen` | `boolean` | `false` | ゲームウィンドウ開閉状態 |
| `caInstalled` | `boolean \| null` | `null` | CA証明書インストール状態 |
| `caInstalling` | `boolean` | `false` | CA証明書インストール中フラグ |
| `now` | `number` | `Date.now()` | 1秒タイマー (カウントダウン用) |
| `expeditions` | `ExpeditionDef[]` | `[]` | 遠征定義一覧 |
| `sortieQuests` | `SortieQuestDef[]` | `[]` | 出撃任務定義一覧 |
| `mapRecommendations` | `MapRecommendationDef[]` | `[]` | 海域編成推奨定義一覧 |
| `activeQuests` | `ActiveQuestDetail[]` | `[]` | 現在受注中の任務 |
| `questProgress` | `Map<number, QuestProgressSummary>` | `new Map()` | 任務進捗 |
| `portDataVersion` | `number` | `0` | 母港データ更新カウンタ (子の再フェッチトリガー) |
| `battleLogs` | `SortieRecord[]` | `[]` | 戦闘ログ一覧 |
| `battleLogsTotal` | `number` | `0` | 戦闘ログ総件数 |
| `battleDateFrom` | `string` | 今月1日 | 戦闘ログ期間 (開始) |
| `battleDateTo` | `string` | 今月末日 | 戦闘ログ期間 (終了) |
| `activeTab` | `TabId` | `"homeport"` | 現在のタブ |
| `uiZoom` | `number` | `135` | UI表示倍率 (%, localStorage永続化) |
| `driveStatus` | `DriveStatus` | `{authenticated:false,syncing:false}` | Google Drive同期状態 |
| `driveLoading` | `boolean` | `false` | Drive操作中フラグ |
| `showApiLog` | `boolean` | localStorage | APIログパネル表示フラグ |
| `rawApiEnabled` | `boolean` | localStorage | 全APIログ保存フラグ |
| `weaponIconSheet` | `string \| null` | `null` | 装備アイコンスプライトシート (data URI) |

### Tauri イベントリスナー (useEffect)

App.tsx の初期化 `useEffect` で登録。全てアンマウント時に解除。

| イベント名 | ペイロード型 | ハンドラ |
|------------|-------------|---------|
| `proxy-ready` | `number` | `proxyPort` 設定 + CA確認 |
| `port-data` | `PortData` | `portData` 更新 + 装備アイコン初回読込 |
| `fleet-updated` | `FleetData[]` | `portData.fleets` を差分更新 |
| `sortie-complete` | `SortieRecord` | `battleLogs` にupsert + total更新 |
| `sortie-update` | `SortieRecord` | `battleLogs` にupsert (進行中更新) |
| `quest-list-updated` | `ActiveQuestDetail[]` | `activeQuests` 更新 + 進捗再取得 |
| `quest-progress-updated` | `QuestProgressSummary[]` | `questProgress` Map更新 |
| `senka-updated` | `SenkaSummary` | `senkaData` 更新 + チェックポイント通知 |
| `drive-sync-status` | `DriveStatus` | `driveStatus` 更新 |
| `drive-data-updated` | `void` | 進捗再取得 + 戦闘ログ再取得 + `portDataVersion` インクリメント |
| `kancolle-api` | `{endpoint:string}` | `apiLog` に追記 (最大200件) |

### 子コンポーネント固有イベント (SortieQuestChecker)

| イベント名 | ペイロード型 | ハンドラ |
|------------|-------------|---------|
| `quest-started` | `number` | 受注開始した任務を自動選択 |
| `quest-stopped` | `number` | 任務選択をクリア |

### 初期データロード (invoke)

App.tsx マウント時に以下のコマンドを並列呼び出し:

- `get_proxy_port` → ポート番号
- `get_expeditions` → 遠征定義
- `get_sortie_quests` → 出撃任務定義
- `get_map_recommendations` → 海域推奨定義
- `get_quest_progress` → 任務進捗
- `get_battle_logs` → 戦闘ログ
- `get_drive_status` → Drive同期状態

### portDataVersion パターン

子コンポーネント (`ShipListTab`, `EquipListTab`, `ImprovementTab`, `FleetPanel`) は `portDataVersion` をdependencyとする `useEffect` でバックエンドからデータを再取得する。親が `portDataVersion` をインクリメントすることで、全子コンポーネントに再フェッチを通知する。

---

## タブ切替方式

- `activeTab` 状態 (`TabId` 型) で現在のタブを管理
- 条件付きレンダリング (`{activeTab === "xxx" && <Component />}`) で表示切替
- タブ切替時にコンポーネントは毎回マウント/アンマウントされる
- 戦闘タブ選択時は `refreshBattleLogs` も呼出

```typescript
type TabId = "homeport" | "battle" | "improvement" | "ships" | "equips" | "options";
```

| タブ名 | TabId | コンポーネント |
|--------|-------|--------------|
| 母港 | `homeport` | `HomeportTab` |
| 戦闘 | `battle` | `BattleTab` |
| 改修 | `improvement` | `ImprovementTab` |
| 艦娘 | `ships` | `ShipListTab` |
| 装備 | `equips` | `EquipListTab` |
| 設定 | `options` | `SettingsTab` |

---

## 型定義一覧 (src/types/)

### src/types/common.ts

```typescript
export interface ConditionResult {
  condition: string;
  satisfied: boolean;
  current_value: string;
  required_value: string;
}

export type TabId = "homeport" | "battle" | "improvement" | "ships" | "equips" | "options";

export interface DriveStatus {
  authenticated: boolean;
  email?: string;
  syncing: boolean;
  last_sync?: string;
  error?: string;
}
```

### src/types/port.ts

```typescript
export interface SpecialEquip {
  name: string;
  icon_type: number;
}

export interface ShipData {
  name: string;
  lv: number;
  hp: number;
  maxhp: number;
  cond: number;
  fuel: number;
  bull: number;
  damecon_name?: string | null;
  command_facility_name?: string | null;
  special_equips: SpecialEquip[];
  can_opening_asw?: boolean;
  soku: number;
}

export interface FleetExpedition {
  mission_name: string;
  return_time: number;  // unix timestamp in ms, 0 = not on expedition
}

export interface FleetData {
  id: number;
  name: string;
  expedition?: FleetExpedition | null;
  ships: ShipData[];
  ship_ids?: number[];
  mission?: unknown[];
}

export interface NdockData {
  id: number;
  state: number;
  ship_name?: string;
  ship_id?: number;
  complete_time: number;
}

export interface PortData {
  admiral_name: string;
  admiral_level: number;
  admiral_rank?: number;
  ship_count: number;
  ship_capacity?: number;
  fuel: number;
  ammo: number;
  steel: number;
  bauxite: number;
  instant_repair?: number;
  instant_build?: number;
  dev_material?: number;
  improvement_material?: number;
  fleets: FleetData[];
  ndock: NdockData[];
}

export interface ApiLogEntry {
  time: string;
  endpoint: string;
}
```

### src/types/senka.ts

```typescript
export interface SenkaSummary {
  total: number;
  exp_senka: number;
  eo_bonus: number;
  quest_bonus: number;
  monthly_exp_gain: number;
  tracking_active: boolean;
  next_checkpoint: string;
  checkpoint_crossed: boolean;
  eo_cutoff_active: boolean;
  quest_cutoff_active: boolean;
  confirmed_senka: number | null;
  confirmed_cutoff: string | null;
  is_confirmed_base: boolean;
}
```

### src/types/expedition.ts

```typescript
export interface ExpeditionDef {
  id: number;
  display_id: string;
  name: string;
  great_success_type: "Regular" | "Drum" | "Level";
  duration_minutes: number;
}

export interface ExpeditionCheckResult {
  expedition_id: number;
  expedition_name: string;
  display_id: string;
  result: "Failure" | "Success" | "GreatSuccess";
  conditions: ConditionResult[];
}

export interface MapRecommendedResult {
  area: string;
  satisfied: boolean;
  conditions: ConditionResult[];
}

export interface MapRecommendationDef {
  area: string;
  name: string;
}

export interface MapRouteCheckResult {
  desc: string;
  satisfied: boolean;
  conditions: ConditionResult[];
}

export interface MapRecommendationCheckResult {
  area: string;
  name: string;
  routes: MapRouteCheckResult[];
}
```

### src/types/quest.ts

```typescript
export interface SortieQuestDef {
  id: number;
  quest_id: string;
  name: string;
  area: string;
  rank: string;
  boss_only: boolean;
  count: number;
  reset: string;
  no_conditions: boolean;
  sub_goals?: { name: string; count: number; boss_only: boolean; rank: string }[];
}

export interface ActiveQuestDetail {
  id: number;
  title: string;
  category: number;
}

export interface SortieQuestCheckResult {
  quest_id: string;
  quest_name: string;
  area: string;
  rank: string;
  boss_only: boolean;
  count: number;
  no_conditions: boolean;
  note: string | null;
  satisfied: boolean;
  conditions: ConditionResult[];
  recommended: MapRecommendedResult[];
}

export interface QuestProgressSummary {
  quest_id: number;
  quest_id_str: string;
  area_progress: { area: string; cleared: boolean; count: number; count_max: number }[];
  count: number;
  count_max: number;
  completed: boolean;
}

export interface DropdownQuest {
  key: string;
  label: string;
  category: number;
  hasData: boolean;
}
```

### src/types/battle.ts

```typescript
export interface HpState {
  before: number;
  after: number;
  max: number;
}

export interface SlotItemSnapshot {
  id: number;
  rf?: number;
  mas?: number;
}

export interface EnemyShip {
  ship_id: number;
  level: number;
  name?: string;
  slots?: number[];
}

export interface AirBattleResult {
  air_superiority?: number;
  friendly_plane_count?: [number, number];
  enemy_plane_count?: [number, number];
}

export interface BattleDetail {
  rank: string;
  enemy_name: string;
  enemy_ships: EnemyShip[];
  formation: [number, number, number];
  air_battle?: AirBattleResult;
  friendly_hp: HpState[];
  enemy_hp: HpState[];
  drop_ship?: string;
  drop_ship_id?: number;
  mvp?: number;
  base_exp?: number;
  ship_exp: number[];
  night_battle: boolean;
}

export interface BattleNode {
  cell_no: number;
  event_kind: number;
  event_id?: number;
  battle?: BattleDetail;
  rank?: string;
  enemy_name?: string;
  drop_ship?: string;
  drop_ship_id?: number;
  mvp?: number;
  base_exp?: number;
}

export interface SortieShip {
  name: string;
  ship_id: number;
  lv: number;
  stype: number;
  slots?: SlotItemSnapshot[];
  slot_ex?: SlotItemSnapshot;
}

export interface SortieRecord {
  id: string;
  fleet_id: number;
  map_display: string;
  ships: SortieShip[];
  nodes: BattleNode[];
  start_time: string;
  end_time?: string;
}

export interface BattleLogsResponse {
  records: SortieRecord[];
  total: number;
}

export interface MapSpot {
  no: number;
  x: number;
  y: number;
  line?: { x: number; y: number; img?: string };
}

export interface MapInfo {
  bg: string[];
  spots: MapSpot[];
}

export interface AtlasFrame {
  frame: { x: number; y: number; w: number; h: number };
}

export interface MapSprites {
  bg?: string;
  point?: string;
  routes: { uri: string; x: number; y: number; w: number; h: number; spotNo: number; isVisited?: boolean }[];
}
```

### src/types/ship.ts

```typescript
export interface ShipListItem {
  id: number;
  ship_id: number;
  name: string;
  stype: number;
  stype_name: string;
  lv: number;
  hp: number;
  maxhp: number;
  cond: number;
  firepower: number;
  torpedo: number;
  aa: number;
  armor: number;
  asw: number;
  evasion: number;
  los: number;
  luck: number;
  locked: boolean;
}

export interface ShipListResponse {
  ships: ShipListItem[];
  stypes: [number, string][];
}

export type ShipSortKey = "lv" | "name" | "stype" | "firepower" | "torpedo" | "aa"
  | "armor" | "asw" | "evasion" | "los" | "luck" | "cond" | "locked";
```

### src/types/equipment.ts

```typescript
export interface EquipListItem {
  master_id: number;
  name: string;
  type_id: number;
  type_name: string;
  icon_type: number;
  total_count: number;
  locked_count: number;
  improvements: [number, number][];  // [level, count]
}

export interface EquipListResponse {
  items: EquipListItem[];
  equip_types: [number, string][];
}
```

### src/types/improvement.ts

```typescript
export interface ConsumedEquipInfo {
  eq_id: number;
  name: string;
  counts: [number, number, number];  // [p1(★0-5), p2(★6-9), conv(更新)]
  owned: number;
}

export interface ImprovementItem {
  eq_id: number;
  name: string;
  eq_type: number;
  type_name: string;
  sort_value: number;
  available_today: boolean;
  today_helpers: string[];
  matches_secretary: boolean;
  previously_improved: boolean;
  consumed_equips: ConsumedEquipInfo[];
}

export interface ImprovementListResponse {
  items: ImprovementItem[];
  day_of_week: number;
  secretary_ship: string;
}
```

### src/types/index.ts (re-export)

全ての型を一箇所から `import type { ... } from "./types"` でインポート可能にする barrel ファイル。

---

## ユーティリティ関数 (src/utils/)

### src/utils/format.ts

| 関数 | シグネチャ | 説明 |
|------|-----------|------|
| `getRankName` | `(rank?: number) => string` | 提督階級名を返す (1=元帥 ~ 10=新米少佐) |
| `formatRemaining` | `(targetMs: number, now: number) => string` | 残り時間を `HH:MM:SS` / `MM:SS` / `"完了"` で表示 |
| `formatDuration` | `(minutes: number) => string` | 分を `Xh` / `XhYYm` / `Xm` 形式に変換 |
| `fmtDate` | `(d: string) => string` | `YYYY-MM-DD` を `YYYY/MM/DD` に変換 |
| `formatImprovements` | `(improvements: [number, number][]) => string` | 改修レベル内訳を `★X x N` 形式で表示 |
| `daysInMonth` | `(year: number, month: number) => number` | 月の日数 (0-indexed month) |
| `toDateStr` | `(y: number, m: number, d: number) => string` | `YYYY-MM-DD` 文字列を生成 (0-indexed month) |

### src/utils/color.ts

| 関数 | シグネチャ | 説明 |
|------|-----------|------|
| `hpColor` | `(hp: number, maxhp: number) => string` | HP割合に応じた色 (>75%=緑, >50%=黄, >25%=橙, <=25%=赤) |
| `condColor` | `(cond: number) => string` | コンディション値の文字色 (>=50=橙キラ, >=40=灰, >=30=黄, <30=赤) |
| `condBgClass` | `(cond: number) => string` | コンディション値のCSSクラス名 (`cond-sparkle` / `""` / `cond-tired` / `cond-red`) |

### src/utils/map.ts

| 関数/定数 | シグネチャ | 説明 |
|----------|-----------|------|
| `getNodeLabel` | `(mapDisplay: string, edgeId: number) => string \| null` | KC3Kai edges データからノードラベル (A, B, ...) を返す |
| `buildPredeckUrl` | `(record: SortieRecord) => string` | kc-web aircalc 用の predeck JSON URL を生成 |
| `CELL_COLORS` | `Record<number, string>` | マスイベント種別ごとの色マップ (0=始点緑, 4=戦闘赤, 5=ボス濃赤, ...) |

### src/utils/index.ts (re-export)

全ユーティリティを `import { ... } from "./utils"` でインポート可能にする barrel ファイル。

---

## CSS 設計方針

### 基本方針

- **コンポーネント1:1 CSS**: 各コンポーネントに対応する `.css` ファイルを配置
- **グローバルスタイル**: `App.css` にリセット、ツールバー、タブバー、スクロールバー等を定義
- **共有テーブルスタイル**: `ListTable.css` を艦娘・装備リストで共有
- **CSS Modules 不使用**: プレーンCSS、クラス名による名前空間分離

### カラーパレット

| 用途 | 色 | 値 |
|------|-----|-----|
| 背景 (メイン) | 濃紺 | `#1a1a2e` |
| 背景 (ヘッダー) | 暗紺 | `#16213e` |
| ボーダー | 青 | `#0f3460` |
| ボタン | 暗青 | `#0f3460` |
| ボタン hover | 中青 | `#1a4080` |
| アクセント | 赤 | `#e94560` |
| テキスト | 薄灰 | `#e0e0e0` |
| サブテキスト | 灰 | `#888` |

### フォント

```css
font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
font-size: 11px;  /* ベースサイズ */
```

### レイアウト

- `display: flex; flex-direction: column` でフルハイト構成
- `overflow: hidden` でコンテナ制御、子で `overflow-y: auto`
- UIズーム: `<div class="app" style={{ zoom: uiZoom / 100 }}>` でCSS `zoom` を適用

### CSS ファイル一覧

| ファイル | 対象コンポーネント |
|----------|------------------|
| `src/App.css` | グローバル (リセット、ツールバー、タブバー、スクロールバー) |
| `src/components/homeport/HomeportTab.css` | HomeportTab (提督情報、資源、入渠、APIログ) |
| `src/components/homeport/FleetPanel.css` | FleetPanel (艦隊パネル、艦娘行) |
| `src/components/homeport/ExpeditionChecker.css` | ExpeditionChecker (遠征条件チェック) |
| `src/components/homeport/MapRecommendationChecker.css` | MapRecommendationChecker (海域推奨) |
| `src/components/homeport/SortieQuestChecker.css` | SortieQuestChecker + QuestProgressDisplay |
| `src/components/battle/BattleTab.css` | BattleTab (フィルターバー、レコード一覧) |
| `src/components/battle/BattleDetailView.css` | BattleDetailView (詳細画面、スプリッター) |
| `src/components/common/DateRangePicker.css` | DateRangePicker (カレンダーUI) |
| `src/components/common/ListTable.css` | 共有テーブル (ShipList + EquipList 共用) |
| `src/components/ships/ShipListTab.css` | ShipListTab (フィルターボタン) |
| `src/components/improvement/ImprovementTab.css` | ImprovementTab (改修リスト) |
| `src/components/settings/SettingsTab.css` | SettingsTab (設定セクション) |

---

## 共通コンポーネント (src/components/common/)

### HpBar

**ファイル**: `src/components/common/HpBar.tsx`

HP残量をバーで表示する。FleetPanel の艦娘行で使用。

- Props: `{ hp: number; maxhp: number }`
- `hpColor()` ユーティリティで色を決定
- CSS: `hp-bar-container`, `hp-bar-fill`, `hp-bar-text`

### BattleHpBar

**ファイル**: `src/components/common/BattleHpBar.tsx`

戦闘詳細画面用のHP表示バー。戦闘前後のHP変動を視覚化する。

- Props: `{ before: number; after: number; max: number; shipName?: string }`
- ゴーストバー (戦闘前HP) + 実バー (戦闘後HP) の2層表示
- ダメージ量 `(-N)` を表示
- 撃沈時は艦名に `sunk` クラスを適用

### ClearButton

**ファイル**: `src/components/common/ClearButton.tsx`

確認付きデータクリアボタン。SettingsTab で使用。

- Props: `{ label: string; desc: string; command: string; onSuccess?: () => void }`
- 状態遷移: `idle` → `confirm` (実行/取消ボタン表示) → `busy` → `done`/`error` → 5秒後に `idle` へ戻る
- `invoke(command)` でバックエンドコマンドを実行

### DateRangePicker

**ファイル**: `src/components/common/DateRangePicker.tsx`

カレンダーUIによる日付範囲選択。BattleTab で使用。

- Props: `{ dateFrom: string; dateTo: string; onChange: (from, to) => void }`
- 2クリック方式: 1回目で開始日、2回目で終了日を選択 (自動swap)
- ホバー中の範囲プレビュー
- 外部クリックで閉じる (`useEffect` + `mousedown` リスナー)
- 今日の日付ハイライト (`drp-today`)

---

## 定数 (src/constants.ts)

```typescript
export const STORAGE_KEYS = {
  UI_ZOOM: "ui-zoom",
  SHOW_API_LOG: "show-api-log",
  RAW_API_ENABLED: "raw-api-enabled",
  SHIP_STYPE_FILTERS: "ship-stype-filters",
  EQUIP_TYPE_FILTERS: "equip-type-filters",
  IMPROVEMENT_TYPE_FILTERS: "improvement-type-filters",
  MAP_REC_AREA: "map-rec-area",
  expeditionFleet: (index: number) => `expedition-fleet-${index}`,
  sortieQuestFleet: (index: number) => `sortie-quest-fleet-${index}`,
} as const;

export const API_QUEST_PREFIX = "api_";
```

### 戦闘定数 (src/components/battle/constants.ts)

| 定数 | 型 | 内容 |
|------|-----|------|
| `FORMATION_NAMES` | `Record<number, string>` | 陣形ID→日本語名 (1=単縦陣 ~ 14=第四警戒) |
| `ENGAGEMENT_NAMES` | `Record<number, string>` | 交戦形態 (1=同航戦 ~ 4=T字不利) |
| `RANK_COLORS` | `Record<string, string>` | 勝敗ランク色 (S=金, A=赤, B=橙, C/D/E=灰) |
| `EVENT_LABELS` | `Record<number, string>` | マスイベント種別ラベル (event_kind) |
| `EVENT_ID_LABELS` | `Record<number, string>` | マスイベントIDラベル (event_id, 優先) |
| `AIR_SUPERIORITY_LABELS` | `Record<number, {text, color}>` | 制空状態 (0=劣勢 ~ 4=喪失) |

---

## localStorage 永続化項目

| キー | 値 | 用途 |
|------|-----|------|
| `ui-zoom` | `number` (文字列) | UIサイズ倍率 |
| `show-api-log` | `"true"/"false"` | APIログパネル表示 |
| `raw-api-enabled` | `"true"/"false"` | 全APIログ保存 |
| `ship-stype-filters` | `number[]` (JSON) | 艦種フィルター |
| `equip-type-filters` | `number[]` (JSON) | 装備種フィルター |
| `improvement-type-filters` | `number[]` (JSON) | 改修装備種フィルター |
| `map-rec-area` | `string` | 選択中の海域推奨マップ |
| `expedition-fleet-{N}` | `number` (文字列) | 各艦隊の選択遠征ID |
| `sortie-quest-fleet-{N}` | `string` | 各艦隊の選択任務ID |
