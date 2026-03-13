<!-- AUTO-GENERATED from source code -->

# 遠征（Expedition）詳細設計

## 1. 概要

遠征チェッカーは、選択された遠征に対して現在の艦隊が出撃条件を満たすかどうかを判定し、大成功可能かどうかも表示する機能。遠征帰還タイマーと通知ウィンドウも含む。

## 2. 遠征定義データ

### 2.1 ファイル

- **パス**: `src-tauri/data/expeditions.json`
- **読み込み**: `include_str!` でバイナリに埋め込み、起動時にパース
- **件数**: 72件（01〜46, A1〜A6, B1〜B6, D1〜D3, E1〜E2）

### 2.2 ExpeditionDef 構造

```rust
pub struct ExpeditionDef {
    pub id: i32,              // API遠征ID（1〜46, 100〜115, 131〜142）
    pub display_id: String,   // 表示用ID（"01", "A1", "B6" など）
    pub name: String,         // 遠征名（"練習航海" など）
    pub great_success_type: GreatSuccessType,  // 大成功判定タイプ
    pub duration_minutes: i32, // 所要時間（分）
    pub conditions: Vec<ExpeditionCondition>,   // 成功条件リスト
}
```

### 2.3 JSON 例

```json
{
  "id": 37,
  "display_id": "37",
  "name": "東京急行",
  "great_success_type": "Drum",
  "duration_minutes": 165,
  "conditions": [
    { "type": "FlagshipLevel", "value": 50 },
    { "type": "LevelSum", "value": 200 },
    { "type": "ShipTypeCount", "ship_type": "軽巡", "stypes": [3], "value": 1 },
    { "type": "ShipTypeCount", "ship_type": "駆逐", "stypes": [2], "value": 5 },
    { "type": "DrumShipCount", "value": 3 },
    { "type": "DrumTotal", "value": 4 }
  ]
}
```

### 2.4 条件タイプ一覧

| type | パラメータ | 判定内容 |
|------|-----------|---------|
| `FlagshipLevel` | `value` | 旗艦レベル >= value |
| `LevelSum` | `value` | 艦隊合計レベル >= value |
| `ShipCount` | `value` | 艦数 >= value |
| `SmallShipCount` | `value` | 駆逐(2)+海防(1) >= value |
| `ShipTypeCount` | `ship_type`, `stypes[]`, `value` | 指定stype艦数 >= value |
| `FlagshipType` | `ship_type`, `stypes[]` | 旗艦が指定stypeに含まれる |
| `SubmarineCount` | `value` | 潜水艦(SS=13/SSV=14) >= value |
| `AircraftCarrierCount` | `value` | 空母(CV=11/CVL=7/CVB=18) >= value |
| `AircraftCarrierOrAVCount` | `value` | 空母+水母(AV=16) >= value |
| `EscortFleet` | なし | 護衛艦隊編成パターン成立 |
| `EscortFleetDD3` | なし | 護衛艦隊 + DD>=3 |
| `EscortFleetDD4` | なし | 護衛艦隊 + DD>=4 |
| `EscortFleetSmall3` | なし | 護衛艦隊 + (DD+DE)>=3 |
| `MiConvoyEscort2` | なし | ミ船団護衛(二号船団)パターン |
| `DrumShipCount` | `value` | ドラム缶搭載艦数 >= value |
| `DrumTotal` | `value` | ドラム缶合計個数 >= value |
| `Firepower` | `value` | 火力合計 >= value |
| `AA` | `value` | 対空合計 >= value |
| `ASW` | `value` | 対潜合計 >= value |
| `LOS` | `value` | 索敵合計 >= value |

## 3. 護衛艦隊判定

### 3.1 基本パターン（6パターンのOR）

```
(CL>=1 AND DD>=2)
OR (CL>=1 AND DE>=2)
OR (CVE>=1 AND DD>=2)
OR (CVE>=1 AND DE>=2)
OR (DD>=1 AND DE>=3)
OR (CT>=1 AND DE>=2)
```

- **CVE（護衛空母）**: stype=CVL かつ ship_id が `CVE_SHIP_IDS` に含まれる
  - 鳳翔改二/改二戦, 龍鳳改二/改二戊, 瑞鳳改二乙, 大鷹系, 雲鷹系, 神鷹系, Langley系, Gambier Bay系

### 3.2 ミ船団護衛(二号船団)

```
パターンA: CVE旗艦 + (DD>=2 OR DE>=2)
パターンB: CVL旗艦 + CL>=1 + DD>=4
```

## 4. 大成功判定ロジック

### 4.1 GreatSuccessType

| タイプ | 判定条件 |
|--------|---------|
| `Regular` | 全艦キラキラ（cond>=50） |
| `Drum` | キラキラ4隻以上 OR 全艦キラキラ |
| `Level` | キラキラ4隻以上 OR 全艦キラキラ |

### 4.2 判定フロー

```
1. 全条件を個別チェック → ConditionResult[] 生成
2. 全条件充足? → No → Failure
3. Yes → 大成功判定
   ├─ Regular: 全艦 cond>=50 → GreatSuccess
   ├─ Drum/Level: sparkled>=4 OR 全艦sparkled → GreatSuccess
   └─ それ以外 → Success
```

> **注意**: Drum/Level タイプの正式な大成功条件（ドラム缶数・合計レベル閾値等）はここでは簡略化されており、キラキラ艦数のみで判定している。

## 5. バックエンド実装

### 5.1 FleetShipData（チェック入力）

```rust
pub struct FleetShipData {
    pub ship_type: i32,     // 艦種stype
    pub ship_id: i32,       // マスターship_id（CVE判定用）
    pub level: i32,
    pub firepower: i32,
    pub aa: i32,
    pub asw: i32,
    pub los: i32,
    pub cond: i32,          // コンディション値
    pub has_drum: bool,     // ドラム缶搭載有無
    pub drum_count: i32,    // ドラム缶個数
}
```

### 5.2 Tauri コマンド

| コマンド | 引数 | 戻り値 |
|---------|------|--------|
| `get_expeditions` | なし | `Vec<ExpeditionDef>` |
| `check_expedition_cmd` | `fleet_index`, `expedition_id` | `ExpeditionCheckResult` |

### 5.3 ドラム缶判定

`check_expedition_cmd` 内でドラム缶を判定:
- プレイヤー装備の `slotitem_id` → マスター装備 → `item_type == 30`（輸送機材カテゴリ）

## 6. フロントエンド表示（ExpeditionChecker）

### 6.1 ファイル

- `src/components/homeport/ExpeditionChecker.tsx`
- `src/components/homeport/ExpeditionChecker.css`

### 6.2 Props

```typescript
{
  fleetIndex: number;           // 第何艦隊（0-based）
  expeditions: ExpeditionDef[]; // 遠征定義リスト
  portDataVersion: number;      // 母港データ更新トリガー
  currentExpedition?: FleetExpedition | null;  // 現在出撃中の遠征
  now: number;                  // 現在時刻(ms)
}
```

### 6.3 表示内容

1. **遠征選択ドロップダウン**: `display_id + name + (所要時間)`
2. **遠征タイマー**: 帰還までの残り時間 + 帰還予定時刻
3. **判定結果ラベル**:
   - `GreatSuccess` → "大成功"（緑系）
   - `Success` → "成功"
   - `Failure` → "失敗"（赤系）
4. **条件一覧**: 各条件の充足/未充足を色分け表示
   - `condition`: 条件名
   - `current_value / required_value`

### 6.4 状態管理

- 選択した遠征IDは `localStorage` に永続化（キー: `expedition_fleet_{fleetIndex}`）
- `portDataVersion` 変更時に自動再チェック

## 7. 遠征帰還通知

### 7.1 タイマー監視（App.tsx）

```typescript
// 60秒間隔でポーリング
for (const fleet of portData.fleets) {
  if (fleet.expedition.return_time - now <= 60000) {
    ready.push({ fleet_id, mission_name });
  }
}
```

- 帰還まで残り60秒以内の艦隊を検出
- 前回と同じ通知キーなら再通知しない（`prevNotifyKeyRef`）

### 7.2 通知ウィンドウ（overlay.rs）

| コマンド | 説明 |
|---------|------|
| `show_expedition_notification` | ゲームウィンドウ右上に通知表示 |
| `hide_expedition_notification` | 通知非表示 |
| `reposition_expedition_notification` | ゲームウィンドウ移動時に追従 |

- 専用ウィンドウ `expedition-notify` をゲームウィンドウにオーバーレイ
- `window.showNotifications(json)` で WebView に通知データを渡す

### 7.3 ExpeditionInfo（API由来データ）

```rust
pub struct ExpeditionInfo {
    pub mission_id: i32,
    pub mission_name: String,
    pub return_time: i64,  // ミリ秒エポック
}
```

- API `api_mission` フィールド `[type, mission_id, return_time, ?]` から取得
- `mission_type == 0` の場合は遠征なし

## 8. データフロー図

```
[expeditions.json] --include_str!--> [get_all_expeditions()]
                                           |
[GameState (ships, slotitems)] --> [check_expedition_cmd] --> [ExpeditionCheckResult]
                                           |
                                    [FleetCheckData]
                                           |
                                    [check_expedition()]
                                      ├─ conditions check
                                      └─ great success check

[API: api_mission] --> [ExpeditionInfo] --> FleetExpedition
                                                |
[App.tsx timer] --> show_expedition_notification --> overlay window
```
