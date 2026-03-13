<!-- AUTO-GENERATED from source code -->

# 戦果（Senka）詳細設計

## 概要

月間のランキング戦果をリアルタイムに追跡する機能。経験値ベースの推計戦果、EO（Extra Operation）ボーナス、任務ボーナスを積算し、ランキングページでの実測値で補正する二段階計算方式を採用。

---

## 1. 戦果計算の基本原理

### 計算式

```
戦果 = 月間経験値獲得量 × 7 / 10000 + EOボーナス + 任務ボーナス
```

### 確認済み戦果ベース（confirmed_senka）

ランキングページから実際の戦果を復号した場合、以降は差分ベースで計算:

```
戦果 = 確認済み戦果 + (カットオフ以降の経験値 × 7 / 10000) + (カットオフ以降のEO) + (カットオフ以降の任務)
```

---

## 2. バックエンド: senka/mod.rs

### 主要データ構造

#### SenkaData（永続化データ）

```rust
pub struct SenkaData {
    pub month: String,                    // "2026-03" 形式
    pub month_start_exp: Option<i64>,     // 月初のHQ経験値
    pub last_exp: Option<i64>,            // 最後に記録したHQ経験値
    pub eo_bonus: i64,                    // 月間EOボーナス累計
    pub quest_bonus: i64,                 // 月間任務ボーナス累計
    pub last_checkpoint: Option<String>,  // 最後のチェックポイント通過時刻 (ISO8601)
    pub confirmed_senka: Option<i64>,     // ランキングページで確認した戦果
    pub confirmed_cutoff: Option<String>, // 確認済み戦果のデータ反映カットオフ (ISO8601)
    pub entries: Vec<SenkaLogEntry>,      // 戦果イベントログ
}
```

#### SenkaLogEntry（ログエントリ）

```rust
pub struct SenkaLogEntry {
    pub timestamp: String,       // ISO8601
    pub entry_type: String,      // "exp" | "eo" | "quest" | "checkpoint" | "ranking_confirmed" | "eo_cutoff" | "quest_late"
    pub exp_gain: Option<i64>,   // 経験値型の場合
    pub bonus: Option<i64>,      // ボーナス型の場合
    pub detail: Option<String>,  // 説明テキスト
}
```

#### SenkaTracker（ランタイムトラッカー）

```rust
pub struct SenkaTracker {
    pub data: SenkaData,
    path: PathBuf,   // sync/senka_log.json
}
```

#### SenkaSummary（フロントエンド送出用）

```rust
pub struct SenkaSummary {
    pub total: f64,              // 合計戦果
    pub exp_senka: f64,          // 経験値由来の戦果
    pub eo_bonus: i64,           // EOボーナス（差分 or 累計）
    pub quest_bonus: i64,        // 任務ボーナス（差分 or 累計）
    pub monthly_exp_gain: i64,   // 経験値獲得量（差分 or 累計）
    pub tracking_active: bool,   // トラッキング中か
    pub next_checkpoint: String, // 次のランキング更新時刻 (ISO8601)
    pub checkpoint_crossed: bool,// チェックポイント通過直後か
    pub eo_cutoff_active: bool,  // EO月末カットオフ中か
    pub quest_cutoff_active: bool, // 任務月末カットオフ中か
    pub confirmed_senka: Option<i64>,    // 確認済み戦果
    pub confirmed_cutoff: Option<String>,// カットオフ時刻
    pub is_confirmed_base: bool, // 確認済みベースで計算中か
}
```

### ログエントリの種別

| entry_type | 意味 | 記録元 |
|------------|------|--------|
| `exp` | 戦闘での提督経験値獲得 | `add_battle_exp()` |
| `eo` | EOクリアボーナス | `add_eo_bonus()` |
| `quest` | 任務達成ボーナス | `add_quest_bonus()` |
| `quest_late` | 月末14:00以降の任務ボーナス（翌月扱い） | `add_quest_bonus()` |
| `eo_cutoff` | 月末22:00以降のEO（戦果無効） | `add_eo_bonus()` |
| `checkpoint` | ランキング更新ポイント通過 | `check_checkpoint()` |
| `ranking_confirmed` | ランキングページからの実測確認 | `confirm_ranking()` |

---

## 3. ランキング暗号化の復号

### 概要

艦これのランキングAPIは暗号化されたフィールド名と値を返す。復号には `user_key` の特定が必要。

### 暗号化フィールドマッピング

| 暗号化名 | 実際の意味 |
|----------|-----------|
| `api_mxltvkpyuklh` | 順位 (position) |
| `api_wuhnhojjxmke` | 暗号化された戦果 (rate) |
| `api_mtjmdcwtvhdr` | 提督名 |
| `api_itslcqtmrxtf` | 暗号化された甲章数 |
| `api_itbrdpdbkynm` | コメント |

### 復号キーテーブル

```rust
const POSSIBLE_RANK: [i64; 13] = [
    8931, 1201, 1156, 5061, 4569, 4732, 3779, 4568, 5695, 4619, 4912, 5669, 6586,
];
```

キー選択: `key = POSSIBLE_RANK[position % 13]`

### user_key 特定アルゴリズム

```
Phase 1: user_key 候補の絞り込み
  1. 最初のエントリで user_key 候補 10〜99 を全探索
     → rate / key / user_key - 91 が非負の整数になるか判定
  2. 以降のエントリで候補を絞り込み (retain)

Phase 2: 全エントリの復号
  - senka = floor(rate / key / user_key) - 91
  - medal_count = (medal_enc / (key + 1853)) - 157
  - 自分の提督名と一致するエントリの senka を own_senka として返却
```

### 復号結果

```rust
pub struct RankingEntry {
    pub position: i32,
    pub admiral_name: String,
    pub senka: i64,
    pub medal_count: i32,
    pub comment: String,
}
```

---

## 4. 時間ベースのルール

### ランキング月の判定

```rust
fn current_ranking_month(now) -> String {
    // 月末最終日の22:00 JST以降 → 翌月扱い
    if 最終日 && 時 >= 22 { 翌月 } else { 当月 }
}
```

### チェックポイント（ランキング更新タイミング）

- 毎日 **03:00 JST** と **15:00 JST** にランキングが更新される
- `check_checkpoint()` で通過を検出し、フロントエンドに通知

### ランキングデータのカットオフ

ランキング更新はリアルタイムではなく、一定時刻までのデータを反映:

| ランキング更新 | データ反映カットオフ |
|---------------|-------------------|
| 03:00 | 当日 02:00 まで |
| 15:00 | 当日 14:00 まで |

```rust
fn ranking_data_cutoff(now) -> DateTime {
    if hour >= 15 { 当日14:00 }
    else if hour >= 3 { 当日02:00 }
    else { 前日14:00 }
}
```

### 月末カットオフ

| カットオフ | 時刻 | 影響 |
|-----------|------|------|
| EO | 月末最終日 22:00 JST | 以降のEOクリアは戦果に加算されない |
| 任務 | 月末最終日 14:00 JST | 以降の任務達成は「翌月扱い」として表示 |

---

## 5. EOボーナスマップ

```rust
pub fn eo_bonus_for_map(area: i32, map: i32) -> i64 {
    (1, 5) => 75,   // 1-5
    (1, 6) => 75,   // 1-6
    (2, 5) => 100,  // 2-5
    (3, 5) => 150,  // 3-5
    (4, 5) => 180,  // 4-5
    (5, 5) => 200,  // 5-5
    (6, 5) => 250,  // 6-5
    (7, 5) => 170,  // 7-5
    _ => 0,
}
```

---

## 6. 任務戦果ボーナスアイテム

```rust
pub fn senka_item_bonus(api_id: i64) -> i64 {
    895 => 440,  896 => 50,   897 => 11,   898 => 800,
    900 => 200,  901 => 350,  902 => 180,  903 => 300,
    904 => 165,  905 => 175,  907 => 210,  908 => 215,
    909 => 330,  910 => 400,  911 => 250,  912 => 315,
    913 => 340,  914 => 160,
}
```

- `clearitemget` API の `api_bounus` 配列から `api_type: 18` のアイテムを検出
- `api_id` に対応するボーナス値 x `api_count` で合計を計算

---

## 7. API 連携ポイント

### 経験値の記録（3箇所）

| API | 呼び出し | 説明 |
|-----|---------|------|
| `api_port/port` | `update_experience(hq_exp)` | 母港帰投時のHQ経験値で月間差分を更新 |
| `api_req_sortie/battleresult` | `add_battle_exp(api_get_exp)` | 出撃戦闘結果の個別経験値を記録 |
| `api_req_practice/battle_result` | `add_battle_exp(api_get_exp, "演習")` | 演習結果の経験値を記録 |

### EOボーナスの記録（2箇所）

| API | 条件 | 説明 |
|-----|------|------|
| `api_req_sortie/battleresult` | `api_get_exmap_rate > 0` | EO海域クリア時のランキングポイント |
| `api_req_map/next` | 1-6 ゴール到達 | 1-6はbattleresult無しでクリアするため特別処理 |

### 任務ボーナスの記録

| API | 処理 |
|-----|------|
| `api_req_quest/clearitemget` | `api_bounus` から `api_type: 18` のアイテムを抽出し `senka_item_bonus()` で変換 |

### ランキング復号

| API | 処理 |
|-----|------|
| `api_req_ranking/mxltvkpyuklh` | `decrypt_ranking()` で復号し、自分のエントリがあれば `confirm_ranking()` |

---

## 8. データの永続化

### ファイル

`sync/senka_log.json` — SenkaData の JSON

### 保存タイミング

以下の操作ごとに即座にファイルへ書き出し:

- `update_experience()` — 経験値変化またはチェックポイント通過時
- `add_battle_exp()` — 戦闘経験値記録時
- `add_eo_bonus()` — EOクリア時
- `add_quest_bonus()` — 任務達成時
- `confirm_ranking()` — ランキング確認時
- `reset_month()` — 月替わり時

### 月替わり処理

```rust
fn reset_month(&mut self, new_month: &str, current_exp: i64) {
    self.data = SenkaData {
        month: new_month,
        month_start_exp: Some(current_exp),
        last_exp: Some(current_exp),
        // 他は全てリセット
    };
}
```

### Google Drive 同期

- `SYNC_TARGETS` に `senka_log.json` として登録
- 各記録操作後に `notify_sync()` で同期エンジンに通知

---

## 9. フロントエンド表示

### イベントリスナー

```typescript
listen<SenkaSummary>("senka-updated", (event) => { ... })
```

### TypeScript 型定義

```typescript
interface SenkaSummary {
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

### 表示パターン（HomeportTab）

#### 確認済み戦果がある場合 (`is_confirmed_base = true`)

```
戦果: 1234.5 (1200+34.5+EO75+任50)
```

- ツールチップに詳細表示:
  - 確認済み: 1200 (14:00まで反映)
  - 追加経験値: +34.5 (exp +49286)
  - 追加EO: +75
  - 追加任務: +50

#### 未確認の場合 (`is_confirmed_base = false`)

```
戦果: ランキング画面で確認してください
```

#### チェックポイント通過通知

```
ランキング更新を通過しました - ランキング画面で戦果を再確認してください
```

---

## 10. データフロー

```
API傍受 (api_port, battleresult, clearitemget, ranking)
         ↓
  SenkaTracker.add_*() / update_experience() / confirm_ranking()
         ↓
  SenkaData 更新 → senka_log.json 保存
         ↓
  SenkaSummary 計算 → "senka-updated" イベント発火
         ↓ Tauri Event
  HomeportTab.tsx → 母港画面に戦果表示
         ↓
  notify_sync() → Google Drive 同期エンジンに通知
```

---

## 11. 関連ファイル一覧

| ファイル | 役割 |
|----------|------|
| `src-tauri/src/senka/mod.rs` | 戦果計算・復号・トラッキングのコアロジック |
| `src-tauri/src/api/mod.rs` | API傍受からの戦果記録呼び出し |
| `src-tauri/src/api/battle.rs` | 戦闘結果からの経験値・EOボーナス記録 |
| `src-tauri/src/api/models.rs` | GameStateInner に SenkaTracker を保持 |
| `src/components/homeport/HomeportTab.tsx` | 戦果表示UI |
| `src/types/senka.ts` | TypeScript 型定義 |
