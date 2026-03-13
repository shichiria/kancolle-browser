<!-- AUTO-GENERATED from source code -->

# 任務進捗（Quest Progress）詳細設計

## 1. 概要

出撃・演習任務の進捗を自動追跡し、手動更新も可能とする機能。戦闘結果APIから自動でカウントし、JST 05:00 基準のリセットロジックにより日次/週次/月次/四半期/年次/単発の任務サイクルを管理する。

## 2. データ構造

### 2.1 QuestProgressState（全体状態）

```rust
pub struct QuestProgressState {
    /// quest_id (API番号) -> 進捗エントリ
    pub quests: HashMap<i32, QuestProgressEntry>,
    /// 最終リセットチェック日時
    pub last_reset_check: Option<DateTime<FixedOffset>>,
}
```

### 2.2 QuestProgressEntry（個別任務進捗）

```rust
pub struct QuestProgressEntry {
    pub quest_id: i32,              // ゲームAPI任務ID（例: 226）
    pub quest_id_str: String,       // 任務文字列ID（例: "Bd7"）
    pub area_cleared: HashMap<String, bool>,  // [LEGACY] 旧データ互換
    pub area_counts: HashMap<String, i32>,    // 海域別/サブゴール別カウント
    pub count: i32,                 // カウンタ値（任意/演習用）
    pub count_max: i32,             // 目標カウント
    pub completed: bool,            // 達成フラグ
    pub last_updated: DateTime<FixedOffset>,  // 最終更新日時(JST)
}
```

### 2.3 進捗パターン（quest_pattern）

任務の種類に応じて3つの進捗パターンが存在する:

| パターン | 条件 | 管理方法 | 例 |
|---------|------|---------|-----|
| `sub_goals` | `sub_goals` が非空 | `area_counts[sub_goal.name]` で各サブゴール独立管理 | あ号作戦(Bw1): 出撃36/S勝利6/ボス到達24/ボス勝利12 |
| `area` | area が "任意"/"演習" 以外 | `area_counts[area]` で海域別カウント | Bm1: 2-5でS勝利1回 |
| `counter` | area が "任意" or "演習" | `count` で単純カウント | Bd1: 任意の戦闘1回 |

## 3. 永続化

### 3.1 保存先

- **ファイル**: `quest_progress.json`（`quest_progress_path` で管理）
- **フォーマット**: JSON（`serde_json::to_string_pretty`）

### 3.2 保存タイミング

| トリガー | 関数 |
|---------|------|
| 戦闘結果処理後（変更あり時） | `on_battle_result` → `save_progress` |
| 演習結果処理後（変更あり時） | `on_exercise_result` → `save_progress` |
| 手動更新時 | `manual_update` → `save_progress` |
| リセット実行時（変更あり時） | `check_resets` → `save_progress` |

### 3.3 読み込み

```rust
pub fn load_progress(path: &Path) -> QuestProgressState
```
- ファイルが存在しない場合 → `QuestProgressState::default()`
- パースに失敗した場合 → 警告ログ + デフォルト値

### 3.4 Google Drive 同期

- 変更時に `notify_sync(state, vec!["quest_progress.json"])` で同期通知
- `drive-data-updated` イベント受信時に `load_progress` で再読み込み

## 4. リセットロジック

### 4.1 基準時刻

すべてのリセットは **JST 05:00** を境界とする。

```rust
let today_5am = JST.ymd(now.year(), now.month(), now.day()).and_hms(5, 0, 0);
let boundary = if now < today_5am {
    today_5am - 1日
} else {
    today_5am
};
```

### 4.2 リセットタイプ別境界

| reset | 境界計算 | 説明 |
|-------|---------|------|
| `daily` | 当日/前日 05:00 JST | 毎日リセット |
| `weekly` | 直前の月曜日 05:00 JST | 毎週月曜リセット |
| `monthly` | 当月1日 05:00 JST | 毎月1日リセット |
| `quarterly` | 直前の 3/6/9/12月1日 05:00 JST | 四半期リセット |
| `yearly` | 4月1日 05:00 JST | 年度リセット |
| `once` / `limited` | リセットなし | 単発/限定任務 |

### 4.3 リセット処理フロー

```
check_resets(state, quest_defs, path):
  for each tracked quest:
    1. quest_def から reset type を取得
    2. primary reset 判定:
       if last_updated < reset_boundary:
         → count=0, area_counts=clear, completed=false（全リセット）
         → continue
    3. counter_reset 判定（completed でない場合のみ）:
       if last_updated < counter_reset_boundary:
         → count=0, area_counts=clear（完了状態は維持）
```

### 4.4 counter_reset（カウンタリセット）

- `counter_reset` フィールドを持つ任務は、メインのリセットとは別にカウンタのみリセットされる
- 例: 演習系の quarterly/yearly 任務で `counter_reset: "daily"` → 毎日進捗リセットだが、任務自体の受注状態は四半期/年度単位

### 4.5 実行タイミング

- **母港帰還時**（`api_port/port` レスポンス処理内で `check_resets` を呼び出し）

## 5. 戦闘結果処理

### 5.1 出撃戦闘（on_battle_result）

```
入力:
  - map_area_str: "1-1", "7-2(2nd)" など
  - rank: "S", "A", "B", "C", "D", "E"
  - is_boss: bool
  - sunk_enemy_stypes: 撃沈した敵艦の stype 配列
  - active_quests: 受注中の任務ID集合
  - quest_defs: 任務定義リスト

処理:
  for each active quest:
    1. 演習任務 → スキップ
    2. パターンごとの処理:
```

| パターン | マッチ判定 | カウント方法 | 完了判定 |
|---------|----------|------------|---------|
| `sub_goals` | 各サブゴールが独立して area/boss/rank を判定 | `area_counts[sg.name]` をインクリメント | 全サブゴールが count 到達 |
| `area` | `does_battle_match` + 海域マッチ | `area_counts[area]` をインクリメント | 全海域が count_max 到達 |
| `counter` | `does_battle_match` で boss/rank/area を総合判定 | `count` をインクリメント。`enemy_type` 指定時は撃沈数をカウント | count >= count_max |

### 5.2 does_battle_match 判定

```
1. boss_only かつ ボス戦でない → false
2. rank が要求ランクより低い → false
3. area == "任意" → true
4. area == "演習" → false（演習専用パスで処理）
5. area を "/" で分割して area_matches でチェック
   - 完全一致 or ベース海域一致（ゲージサフィックス除去）
```

### 5.3 ランク値変換

```
S=5, A=4, B=3, C=2, D=1, E=0
```

### 5.4 敵艦タイプマッチ（enemy_type）

| enemy_type | マッチ stype |
|-----------|-------------|
| `carrier` | CVL(7), CV(11), CVB(18) |
| `transport` | AP(15) |
| `submarine` | SS(13), SSV(14) |

### 5.5 演習結果（on_exercise_result）

```
入力: rank, active_quests, quest_defs
処理:
  for each active quest where area == "演習":
    rank チェック → count++ → completed 判定
```

## 6. 手動更新（manual_update）

### 6.1 Tauri コマンド

```rust
#[tauri::command]
pub async fn update_quest_progress(
    quest_id: i32,       // API任務ID
    area: Option<String>, // 海域/サブゴール名（省略時は counter 更新）
    count: Option<i32>,   // 設定値（省略時はトグル動作）
) -> Result<bool, String>
```

### 6.2 動作

| 引数パターン | 動作 |
|-------------|------|
| `area=Some, count=Some` | 指定海域のカウントを設定値に変更 |
| `area=Some, count=None` | トグル: count_max<=1 なら 0↔1、それ以外はインクリメント（max超えたら0） |
| `area=None, count=Some` | カウンタ値を設定値に変更 |

### 6.3 イベント通知

更新後 `quest-progress-updated` イベントを emit → フロントエンドが自動リフレッシュ

## 7. フロントエンド表示（QuestProgressDisplay）

### 7.1 ファイル

- `src/components/homeport/QuestProgressDisplay.tsx`

### 7.2 Props

```typescript
{
  questId: string | null;                    // 任務文字列ID or APIプレフィックス付きID
  questById: Map<number, SortieQuestDef>;    // API番号→定義のルックアップ
  questProgress: Map<number, QuestProgressSummary>;  // API番号→進捗サマリ
  skipAreas?: Set<string>;                   // 推奨編成で表示済みの海域（重複回避）
}
```

### 7.3 表示パターン

| 任務タイプ | 表示内容 |
|-----------|---------|
| `sub_goals` 任務 | 各サブゴール行: 名前 + ドロップダウン(0〜count_max) + "/count_max" + 達成バッジ |
| `counter` 任務 | ドロップダウン(0〜count_max) + "/count_max" + 達成バッジ |
| `area` 任務（全海域カバー済み） | 表示なし（SortieQuestChecker 内でインライン表示） |

### 7.4 QuestProgressSummary（フロントエンド型）

```typescript
interface QuestProgressSummary {
  quest_id: number;
  quest_id_str: string;
  area_progress: { area: string; cleared: boolean; count: number; count_max: number }[];
  count: number;
  count_max: number;
  completed: boolean;
}
```

## 8. データフロー図

```
[API: battleresult/practice_battle]
    │
    ├─ on_battle_result() ─── area/counter/sub_goals パターン分岐
    │   └─ save_progress() → quest_progress.json
    │   └─ emit("quest-progress-updated")
    │
    └─ on_exercise_result() ─── counter パターン
        └─ save_progress() → quest_progress.json
        └─ emit("quest-progress-updated")

[API: api_port/port]
    └─ check_resets() ─── daily/weekly/monthly/quarterly/yearly 判定
        └─ save_progress()

[Frontend: QuestProgressDisplay / SortieQuestChecker]
    ├─ get_quest_progress コマンド → QuestProgressSummary[]
    ├─ update_quest_progress コマンド → 手動更新
    └─ listen("quest-progress-updated") → 自動リフレッシュ

[Google Drive Sync]
    ├─ notify_sync("quest_progress.json") → アップロード
    └─ drive-data-updated → load_progress() で再読み込み
```

## 9. マイグレーション

### 9.1 area_cleared → area_counts 移行

旧データ形式 `area_cleared: HashMap<String, bool>` から新形式 `area_counts: HashMap<String, i32>` への自動移行:

```
if area_counts is empty:
  for each area in quest.area.split('/'):
    area_counts[area] = if area_cleared[area] { min(1, count) } else { 0 }
```

### 9.2 sub_goals キー追加

既存エントリに新しいサブゴールキーが追加された場合、`ensure_entry` で自動追加:

```
for sg in quest.sub_goals:
  area_counts.entry(sg.name).or_insert(0)
```

### 9.3 count_max 同期

`ensure_entry` および `get_active_progress` で常に最新の定義値と同期:

```
entry.count_max = quest.count
```
