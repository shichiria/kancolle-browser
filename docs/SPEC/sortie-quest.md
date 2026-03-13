<!-- AUTO-GENERATED from source code -->

# 出撃任務（Sortie Quest）詳細設計

## 1. 概要

出撃任務チェッカーは、受注中の出撃/演習/編成任務に対して現在の艦隊が編成条件を満たすかどうかを判定する機能。海域別の推奨編成（recommended）と汎用のマップルート推奨機能も含む。

## 2. 出撃任務定義データ

### 2.1 ファイル

- **パス**: `src-tauri/data/sortie_quests.json`
- **読み込み**: `include_str!` でバイナリに埋め込み
- **件数**: 200件以上（daily/weekly/monthly/quarterly/yearly/once）

### 2.2 SortieQuestDef 構造

```rust
pub struct SortieQuestDef {
    pub id: i32,                    // API任務番号（例: 226）
    pub quest_id: String,           // 任務文字列ID（例: "Bm1"）
    pub name: String,               // 任務名
    pub area: String,               // 対象海域（"2-5", "1-1/1-2", "任意", "演習"）
    pub rank: String,               // 要求ランク（"S", "A", "B", ""）
    pub boss_only: bool,            // ボス戦限定
    pub count: i32,                 // 達成必要回数
    pub reset: String,              // リセットタイプ
    pub no_conditions: bool,        // true=編成条件確認済みでなし
    pub counter_reset: Option<String>,  // カウンタリセット周期
    pub note: Option<String>,       // 補足情報（例: "※第２艦隊で出撃"）
    pub sub_goals: Vec<SubGoal>,    // 複合条件サブゴール
    pub enemy_type: Option<String>, // 撃沈対象艦種
    pub conditions: Vec<SortieQuestCondition>,  // 編成条件
    pub recommended: Vec<MapRecommendation>,    // 海域別推奨編成
}
```

### 2.3 JSON 例

```json
{
  "id": 264,
  "quest_id": "Bm1",
  "name": "「第五戦隊」出撃せよ！",
  "area": "2-5",
  "rank": "S",
  "boss_only": true,
  "count": 1,
  "reset": "monthly",
  "conditions": [
    {
      "type": "ContainsShipName",
      "names": ["那智", "妙高", "羽黒"],
      "count": 3
    }
  ],
  "recommended": [
    {
      "area": "2-5",
      "fleet": [
        { "type": "ShipCount", "value": 6 },
        { "type": "ShipTypeCount", "ship_type": "駆逐", "stypes": [2], "value": 3 }
      ]
    }
  ]
}
```

### 2.4 reset タイプ一覧

| reset | 説明 | 件数目安 |
|-------|------|---------|
| `daily` | 毎日リセット | 5+ |
| `weekly` | 毎週月曜リセット | 8+ |
| `monthly` | 毎月1日リセット | 6+ |
| `quarterly` | 3/6/9/12月1日リセット | 10+ |
| `yearly` | 4月1日リセット | 10+ |
| `once` | 単発（リセットなし） | 100+ |

### 2.5 SubGoal（複合サブゴール）

```rust
pub struct SubGoal {
    pub name: String,         // サブゴール名（例: "ボス勝利"）
    pub count: i32,           // 必要回数
    pub boss_only: bool,      // ボス戦限定
    pub rank: String,         // 要求ランク
    pub area: Option<String>, // 海域フィルタ（省略時は全海域）
}
```

例: あ号作戦(Bw1) → 出撃36回/S勝利6回/ボス到達24回/ボス勝利12回
例: Bq2 → 2-4(A), 6-1(A), 6-3(A), 6-4(S) の各1回

## 3. 編成条件タイプ一覧

### 3.1 SortieQuestCondition

| type | パラメータ | 判定内容 |
|------|-----------|---------|
| `ShipCount` | `value` | 艦数 >= value |
| `ShipTypeCount` | `ship_type`, `stypes[]`, `value` | 指定stype艦数 >= value |
| `FlagshipType` | `ship_type`, `stypes[]` | 旗艦が指定stypeに含まれる |
| `ContainsShipName` | `names[]`, `count` | names で始まる艦名の艦が count 隻以上（AND） |
| `ContainsShipNameAny` | `names[]`, `count` | names のいずれかで始まる艦名の艦が count 隻以上（OR） |
| `OnlyShipTypes` | `desc`, `stypes[]` | 全艦が指定stypeに含まれる |
| `MaxShipTypeCount` | `ship_type`, `stypes[]`, `value` | 指定stype艦数 <= value（上限制約） |
| `OrConditions` | `desc`, `alternatives[][]` | alternative グループのいずれかが全条件充足 |

### 3.2 ContainsShipName の艦名マッチ

```rust
fn name_matches(ship_name: &str, prefixes: &[String]) -> bool {
    prefixes.iter().any(|prefix| ship_name.starts_with(prefix))
}
```

- `starts_with` で前方一致 → 改造状態を問わずマッチ
- 例: `"那智"` は "那智", "那智改", "那智改二" すべてにマッチ

### 3.3 OrConditions の例

```json
{
  "type": "OrConditions",
  "desc": "六水戦DD×2 or 由良改二",
  "alternatives": [
    [
      { "type": "ContainsShipNameAny", "names": ["睦月","如月","弥生","卯月","水無月","文月"], "count": 2 }
    ],
    [
      { "type": "ContainsShipName", "names": ["由良改二"], "count": 1 }
    ]
  ]
}
```

## 4. 条件チェックロジック

### 4.1 FleetShipData（チェック入力）

```rust
pub struct FleetShipData {
    pub name: String,       // 艦名（改造名含む）
    pub ship_type: i32,     // 艦種stype
    pub level: i32,         // レベル
}
```

遠征チェッカーと異なり、火力/対空/対潜/索敵/コンディション/ドラム缶は不要。

### 4.2 check_sortie_quest フロー

```
1. quest_id_str で定義を検索（なければ Unknown 返却）
2. 全 conditions を check_condition で評価 → ConditionResult[]
3. satisfied 判定:
   - no_conditions=true かつ conditions 空 → satisfied=true
   - conditions 非空 かつ 全充足 → satisfied=true
   - それ以外 → satisfied=false
4. recommended の各海域も同様にチェック → MapRecommendedResult[]
5. SortieQuestCheckResult を返却
```

### 4.3 Tauri コマンド

| コマンド | 引数 | 戻り値 |
|---------|------|--------|
| `get_sortie_quests` | なし | `Vec<SortieQuestDef>` |
| `check_sortie_quest_cmd` | `fleet_index`, `quest_id` | `SortieQuestCheckResult` |
| `get_active_quest_ids` | なし | `Vec<ActiveQuestDetail>` |

## 5. 海域ルート推奨

### 5.1 ファイル

- **パス**: `src-tauri/data/map_recommendations.json`
- **読み込み**: `include_str!` でバイナリに埋め込み
- **件数**: 20+海域

### 5.2 MapRecommendationDef 構造

```rust
pub struct MapRecommendationDef {
    pub area: String,       // 海域（"1-1", "2-5" など）
    pub name: String,       // 海域名（"鎮守府正面海域" など）
    pub routes: Vec<MapRecommendationRoute>,
}

pub struct MapRecommendationRoute {
    pub desc: String,       // ルート説明（"A→E(ボス) (駆逐/海防4隻)"）
    pub fleet: Vec<SortieQuestCondition>,  // 編成条件（出撃任務と同じ型）
}
```

### 5.3 JSON 例

```json
{
  "area": "2-5",
  "name": "沖ノ島沖",
  "routes": [
    {
      "desc": "上ルート B→E→I→O(ボス) (重巡系3+駆逐1)",
      "fleet": [
        { "type": "ShipCount", "value": 6 },
        { "type": "ShipTypeCount", "ship_type": "重巡系", "stypes": [5, 6], "value": 3 },
        { "type": "ShipTypeCount", "ship_type": "駆逐", "stypes": [2], "value": 1 }
      ]
    },
    {
      "desc": "水上 B→F→J→O(ボス) (駆逐3+軽巡1, 戦艦/空母なし)",
      "fleet": [
        { "type": "ShipCount", "value": 6 },
        { "type": "ShipTypeCount", "ship_type": "駆逐", "stypes": [2], "value": 3 },
        { "type": "ShipTypeCount", "ship_type": "軽巡", "stypes": [3], "value": 1 },
        { "type": "MaxShipTypeCount", "ship_type": "戦艦系", "stypes": [8,9,10,12], "value": 0 },
        { "type": "MaxShipTypeCount", "ship_type": "空母系", "stypes": [7,11,18], "value": 0 }
      ]
    }
  ]
}
```

### 5.4 check_map_recommendation フロー

```
1. area で定義を検索
2. 各 route の fleet 条件を全チェック
3. MapRecommendationCheckResult を返却
   - routes[]: { desc, satisfied, conditions[] }
```

### 5.5 Tauri コマンド

| コマンド | 引数 | 戻り値 |
|---------|------|--------|
| `get_map_recommendations` | なし | `Vec<MapRecommendationDef>` |
| `check_map_recommendation_cmd` | `fleet_index`, `area` | `MapRecommendationCheckResult` |

## 6. フロントエンド表示

### 6.1 SortieQuestChecker

#### ファイル

- `src/components/homeport/SortieQuestChecker.tsx`
- `src/components/homeport/SortieQuestChecker.css`

#### Props

```typescript
{
  fleetIndex: number;
  sortieQuests: SortieQuestDef[];
  portDataVersion: number;
  activeQuests: ActiveQuestDetail[];
  questProgress: Map<number, QuestProgressSummary>;
}
```

#### 表示内容

1. **任務選択ドロップダウン**:
   - 受注中任務のみ表示（`activeQuests` から取得）
   - カテゴリ別グループ: 出撃(2,8,9,10) / 演習(3) / 編成(1)
   - JSON定義がない任務も `api_no:XXX` キーで表示可能

2. **任務情報**:
   - 海域、ボス/ランク、必要回数
   - 編成OK/NG/条件不明/条件なし/データなし

3. **編成条件一覧**: 各条件の充足状況

4. **海域別推奨編成 + 進捗**:
   - 推奨編成の条件チェック結果
   - 海域別進捗カウント（ドロップダウンで手動変更可能）
   - 達成済み海域は条件非表示 + "達成済" バッジ

5. **QuestProgressDisplay** 統合:
   - sub_goals 任務: サブゴール別の進捗表示
   - counter 任務: カウンタ進捗表示
   - area 任務の推奨未カバー海域: インラインで進捗表示

#### 自動選択

- `quest-started` イベント受信時: 新規受注任務を自動選択
- `quest-stopped` イベント受信時: 選択解除
- 選択中の任務がアクティブリストから消えた場合: 自動解除

#### 状態永続化

- 選択した任務IDは `localStorage` に保存（キー: `sortie_quest_fleet_{fleetIndex}`）
- `portDataVersion` 変更時に自動再チェック

### 6.2 MapRecommendationChecker

#### ファイル

- `src/components/homeport/MapRecommendationChecker.tsx`
- `src/components/homeport/MapRecommendationChecker.css`

#### Props

```typescript
{
  mapRecommendations: MapRecommendationDef[];
  portDataVersion: number;
}
```

#### 表示内容

1. **海域選択ドロップダウン**:
   - 海域番号でグループ化（第1海域, 第2海域, ...）
   - `area + name` を表示

2. **ルート別判定結果**:
   - ルート説明（`desc`）
   - 編成OK/NG
   - 各条件の充足状況

#### 状態永続化

- 選択した海域は `localStorage` に保存（キー: `map_rec_area`）

## 7. TypeScript 型定義

### 7.1 SortieQuestCheckResult

```typescript
interface SortieQuestCheckResult {
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
```

### 7.2 MapRecommendationCheckResult

```typescript
interface MapRecommendationCheckResult {
  area: string;
  name: string;
  routes: MapRouteCheckResult[];
}

interface MapRouteCheckResult {
  desc: string;
  satisfied: boolean;
  conditions: ConditionResult[];
}
```

## 8. データフロー図

```
[sortie_quests.json] --include_str!--> [get_all_sortie_quests()]
[map_recommendations.json] --include_str!--> [get_all_map_recommendations()]

[GameState (ships)] ──→ [check_sortie_quest_cmd]
                    │       ├─ conditions check
                    │       └─ recommended check per area
                    │
                    └──→ [check_map_recommendation_cmd]
                            └─ route conditions check

[API: api_get_member/questlist]
    └─ active_quest_details → フロントエンドのドロップダウンに反映

[quest-started event] → auto-select quest
[quest-stopped event] → clear selection
[portDataVersion change] → re-check conditions
```

## 9. 出撃任務と推奨編成の関係

```
SortieQuestDef.recommended[]     ← 任務固有の推奨編成（任務達成に最適化）
MapRecommendationDef.routes[]    ← 汎用マップルート推奨（ルーティング最適化）

SortieQuestChecker:
  ├─ 任務条件チェック（conditions）
  ├─ 任務推奨編成チェック（recommended）+ 海域別進捗表示
  └─ QuestProgressDisplay（sub_goals/counter 進捗）

MapRecommendationChecker:
  └─ 汎用ルート条件チェック（任務とは独立）
```

両者は `SortieQuestCondition` 型を共有しており、同じ `check_condition` 関数でチェックされる。
