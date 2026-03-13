<!-- AUTO-GENERATED from source code -->

# 装備改修（Improvement）詳細設計

## 概要

明石の工廠で実施可能な装備改修を一覧表示する機能。曜日・二番艦によって利用可能な改修が変化するゲーム仕様に対応し、本日改修可能な装備を視覚的に区別して表示する。

---

## 1. データソース: equipment_upgrades.json

### ファイル位置

`src-tauri/data/equipment_upgrades.json`（コンパイル時に `include_str!` でバイナリに埋め込み）

### JSON 構造

```jsonc
[
  {
    "eq_id": 1,                    // 装備マスターID
    "improvement": [               // 改修パス（複数の改修経路がありうる）
      {
        "helpers": [               // 二番艦（担当艦）と対応曜日
          {
            "ship_ids": [1, 2, 254, 255, 434, 435],  // 艦船マスターID
            "days": [0, 1, 2, 3, 4, 5, 6]            // 0=日〜6=土
          }
        ],
        "convert": {               // 更新先（任意）
          "id_after": 293,
          "lvl_after": 0
        },
        "costs": {                 // 改修コスト
          "fuel": 10,
          "ammo": 20,
          "steel": 40,
          "baux": 0,
          "p1": {                  // Phase 1: ★0→★5
            "devmats": 2,
            "devmats_sli": 2,      // 確実化時の開発資材
            "screws": 1,
            "screws_sli": 2,       // 確実化時の改修資材
            "equips": [            // 消費装備
              { "id": 1, "eq_count": 1 }
            ],
            "consumable": []
          },
          "p2": {                  // Phase 2: ★6→★9
            "devmats": 2,
            "devmats_sli": 3,
            "screws": 1,
            "screws_sli": 2,
            "equips": [
              { "id": 1, "eq_count": 2 }
            ],
            "consumable": []
          },
          "conv": {                // 更新時コスト
            "devmats": 2,
            "devmats_sli": 4,
            "screws": 2,
            "screws_sli": 6,
            "equips": [
              { "id": 28, "eq_count": 1 }
            ],
            "consumable": []
          }
        }
      }
    ],
    "convert_to": [                // 更新先装備一覧（参考用）
      { "id_after": 293, "lvl_after": 0 }
    ],
    "upgrade_for": [293, 382, 393, 394]  // この装備が消費素材となる改修先
  }
]
```

### 静的ロード

```rust
static UPGRADE_DATA: OnceLock<Vec<EquipmentUpgradeEntry>> = OnceLock::new();

fn get_upgrade_data() -> &'static [EquipmentUpgradeEntry] {
    UPGRADE_DATA.get_or_init(|| {
        let json_str = include_str!("../../data/equipment_upgrades.json");
        let json_str = json_str.strip_prefix('\u{feff}').unwrap_or(json_str);
        serde_json::from_str(json_str).expect("...")
    })
}
```

- `OnceLock` により初回アクセス時に1回だけパースし、以降は `&'static` 参照を返す
- BOM (`U+FEFF`) を除去してからパース

---

## 2. バックエンド: improvement/mod.rs

### Rust データ構造（内部）

```rust
struct EquipmentUpgradeEntry {
    eq_id: i32,
    improvement: Vec<ImprovementPath>,
    convert_to: serde_json::Value,   // 未使用（将来用）
    upgrade_for: serde_json::Value,  // 未使用（将来用）
}

struct ImprovementPath {
    helpers: Vec<ImprovementHelper>,
    convert: serde_json::Value,
    costs: Option<ImprovementCosts>,
}

struct ImprovementHelper {
    ship_ids: Vec<i32>,   // 二番艦の艦船マスターID
    days: Vec<i32>,       // 0=日〜6=土
}

struct ImprovementCosts {
    p1: Option<CostPhase>,   // ★0-5
    p2: Option<CostPhase>,   // ★6-9
    conv: Option<CostPhase>, // 更新
}

struct CostPhase {
    equips: Vec<CostEquip>,
}

struct CostEquip {
    id: i32,        // 消費装備ID
    eq_count: i32,  // 消費数
}
```

### レスポンス型（フロントエンドへ送出）

```rust
pub struct ImprovementListResponse {
    pub items: Vec<ImprovementItem>,
    pub day_of_week: i32,          // 現在のJST曜日 (0=日〜6=土)
    pub secretary_ship: String,    // 第1艦隊2番艦の名前
}

pub struct ImprovementItem {
    pub eq_id: i32,
    pub name: String,
    pub eq_type: i32,
    pub type_name: String,         // 装備種名（「小口径主砲」等）
    pub sort_value: i32,           // 装備種に応じたステータス値
    pub available_today: bool,     // 本日改修可能か
    pub today_helpers: Vec<String>,// 本日の担当艦名リスト
    pub matches_secretary: bool,   // 現在の二番艦が担当艦か
    pub previously_improved: bool, // 過去に改修実績があるか
    pub consumed_equips: Vec<ConsumedEquipInfo>,
}

pub struct ConsumedEquipInfo {
    pub eq_id: i32,
    pub name: String,
    pub counts: [i32; 3],  // [p1(★0-5), p2(★6-9), conv(更新)]
    pub owned: i32,        // ロックされていない所持数
}
```

### build_improvement_list() のロジック

```
1. get_upgrade_data() で静的データ取得
2. jst_day_of_week() で現在のJST曜日を計算
3. 第1艦隊の2番艦（index=1）のmaster_idと名前を取得
   - 艦これの仕様: 明石が旗艦、2番艦が改修の担当艦
4. 各装備エントリに対して:
   a. マスターデータから装備名・種別を取得
   b. get_type_name() で装備種名を日本語に変換（全46種類対応）
   c. get_primary_stat() で装備種に応じたソート用ステータスを取得
      - 砲: 火力, 魚雷: 雷装, 戦闘機: 対空, 偵察機: 索敵, etc.
   d. 各改修パスの helpers をチェック:
      - days に今日が含まれるか → available_today
      - ship_ids に現在の二番艦が含まれるか → matches_secretary
      - 該当する艦名を today_helpers に集約
   e. improved_equipment 履歴に含まれるか → previously_improved
   f. costs から消費装備を集約（全パスの最大値を採用）
      - ロックされていない同装備の所持数を計算
5. レスポンスを返却
```

### 曜日計算

```rust
fn jst_day_of_week() -> i32 {
    let jst = FixedOffset::east_opt(9 * 3600).unwrap();
    let now_jst = Utc::now().with_timezone(&jst);
    now_jst.weekday().num_days_from_sunday() as i32  // 0=日〜6=土
}
```

### 装備種名マッピング

`get_type_name()` は装備の `item_type` を受け取り、日本語名を返す。全46種別に対応:

| item_type | 名前 | item_type | 名前 |
|-----------|------|-----------|------|
| 1 | 小口径主砲 | 25 | オートジャイロ |
| 2 | 中口径主砲 | 26 | 対潜哨戒機 |
| 3 | 大口径主砲 | 29, 42 | 探照灯 |
| 4 | 副砲 | 32 | 潜水艦魚雷 |
| 5 | 魚雷 | 33 | 照明弾 |
| 6 | 艦上戦闘機 | 34 | 司令部施設 |
| 7 | 艦上爆撃機 | 36 | 高射装置 |
| 8 | 艦上攻撃機 | 37 | 対地装備 |
| 9 | 艦上偵察機 | 38 | 大口径主砲II |
| 10 | 水上偵察機 | 39 | 水上艦要員 |
| 11 | 水上爆撃機 | 40 | 大型ソナー |
| 12 | 小型電探 | 41 | 大型飛行艇 |
| 13 | 大型電探 | 45 | 水上戦闘機 |
| 14 | ソナー | 46 | 特型内火艇 |
| 15 | 爆雷 | 47 | 陸上攻撃機 |
| 16, 27, 28 | 追加装甲 | 48 | 局地戦闘機 |
| 17 | 機関部強化 | 49 | 陸上偵察機 |
| 18 | 対空強化弾 | 51 | 潜水艦装備 |
| 19 | 対艦強化弾 | 93 | 大型電探II |
| 21 | 対空機銃 | 94 | 艦上偵察機II |
| 22 | 特殊潜航艇 | 95 | 副砲II |
| 24 | 上陸用舟艇 | その他 | その他 |

---

## 3. 改修履歴の永続化

### ファイル

`sync/improved_equipment.json` — 改修実績のある装備ID配列

```json
[1, 2, 15, 46, 58]
```

### 永続化関数

```rust
pub fn load_improved_history(path: &Path) -> HashSet<i32>
pub fn save_improved_history(path: &Path, history: &HashSet<i32>)
```

- `HashSet<i32>` として保持
- Google Drive 同期対象（SYNC_TARGETS に `improved_equipment.json` として登録）

---

## 4. Tauri コマンド

```rust
#[tauri::command]
pub(crate) async fn get_improvement_list(
    state: tauri::State<'_, GameState>,
) -> Result<ImprovementListResponse, String> {
    let inner = state.inner.read().await;
    Ok(improvement::build_improvement_list(&inner))
}
```

- `lib.rs` の `invoke_handler` に `commands::get_improvement_list` として登録

---

## 5. フロントエンド: ImprovementTab.tsx

### ファイル構成

| ファイル | 役割 |
|----------|------|
| `src/components/improvement/ImprovementTab.tsx` | メインコンポーネント |
| `src/components/improvement/ImprovementTab.css` | スタイル |
| `src/components/improvement/index.ts` | re-export |
| `src/types/improvement.ts` | TypeScript 型定義 |

### TypeScript 型定義

```typescript
interface ConsumedEquipInfo {
  eq_id: number;
  name: string;
  counts: [number, number, number]; // [p1(★0-5), p2(★6-9), conv(更新)]
  owned: number;
}

interface ImprovementItem {
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

interface ImprovementListResponse {
  items: ImprovementItem[];
  day_of_week: number;
  secretary_ship: string;
}
```

### Props

```typescript
{ portDataVersion: number }
```

- `portDataVersion` が変化するたびに `get_improvement_list` を再呼び出し

### State

| state | 型 | 初期値 | 説明 |
|-------|----|----|------|
| `data` | `ImprovementListResponse \| null` | `null` | バックエンドから取得した全データ |
| `typeFilters` | `Set<number>` | localStorage から復元 | 装備種フィルタ |

### UI 構成

```
┌─ improvement-header ─────────────────────────┐
│ [曜日表示]  [2番艦名]             [件数表示] │
├─ improvement-filters ────────────────────────┤
│ [種別ボタン] [種別ボタン] ... [全表示ボタン] │
├─ improvement-list ───────────────────────────┤
│ [装備名] [種別] [消費装備×数(所持数)] [★] [担当艦] │
│ [装備名] [種別] [消費装備×数(所持数)]      [担当艦] │
│ ...                                           │
└──────────────────────────────────────────────┘
```

### ソート順序

`displayItems` は以下の優先順でソート:

1. 本日改修可能（`available_today`）を上位に
2. 改修実績あり（`previously_improved`）を上位に
3. ソート値（`sort_value` = 装備種ごとの主要ステータス）降順
4. 名前の辞書順

### 装備種フィルタ

- 装備種ごとのトグルボタンを表示
- フィルタ状態は `localStorage` に `IMPROVEMENT_TYPE_FILTERS` キーで永続化
- フィルタ未選択時は全種類を表示

### CSS クラスによる状態表示

| クラス | 条件 | 視覚効果 |
|--------|------|----------|
| `imp-available` | 本日改修可能 | 通常色 |
| `imp-unavailable` | 本日改修不可 | 薄暗い表示 (opacity: 0.6) |
| `imp-match` | 現在の二番艦が担当艦 | シアン系背景ハイライト |
| `imp-history` | 改修実績あり | 金色の ★ マーク |
| `imp-consumed-zero` | 消費装備の所持数 = 0 | 赤色表示 |
| `imp-owned-zero` | 所持数 = 0 | 赤色表示 |

### 消費装備の表示形式

ツールチップ:
```
装備名
★0-5: ×1  ★6-9: ×2  更新: ×1
所持(ロックなし): 3
```

行内表示:
```
装備名 ×1/2/1 (3)
```

---

## 6. データフロー

```
equipment_upgrades.json (コンパイル時埋込)
         ↓
    OnceLock で初回パース
         ↓
build_improvement_list() ← GameStateInner (マスターデータ + プロフィール + 履歴)
         ↓
ImprovementListResponse (Serialize)
         ↓ Tauri IPC
ImprovementTab.tsx → 表示
         ↑
  portDataVersion 変更で再取得
```

---

## 7. 関連ファイル一覧

| ファイル | 役割 |
|----------|------|
| `src-tauri/src/improvement/mod.rs` | 改修リスト構築ロジック |
| `src-tauri/data/equipment_upgrades.json` | 改修データ（静的埋込） |
| `src-tauri/src/commands.rs` | Tauri コマンド定義 |
| `src/components/improvement/ImprovementTab.tsx` | フロントエンド |
| `src/components/improvement/ImprovementTab.css` | スタイル |
| `src/types/improvement.ts` | TypeScript 型定義 |
