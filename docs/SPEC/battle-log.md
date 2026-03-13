<!-- AUTO-GENERATED from source code -->

# 戦闘ログシステム 詳細設計書

## 概要

戦闘ログシステムは、艦これの出撃（ソーティ）における戦闘データをAPI傍受によりリアルタイムに記録・保存・表示する機能である。バックエンド（Rust/Tauri）でAPIレスポンスを解析し、フロントエンド（React/TypeScript）で一覧表示・詳細表示・マップルート表示を行う。

---

## 1. 状態管理（BattleLogger）

### 1.1 BattleLogger構造体

```rust
pub struct BattleLogger {
    active_sortie: Option<SortieRecord>,     // 進行中の出撃（なければNone）
    pending_battle: Option<PendingBattle>,    // 戦闘API受信中の中間データ
    completed: Vec<SortieRecord>,             // 完了済み出撃（メモリ内、新しい順、最大200件）
    save_dir: Option<PathBuf>,                // 永続化ディレクトリ（sync/battle_logs/）
    raw_dir: Option<PathBuf>,                 // 生APIダンプディレクトリ（sync/raw_api/）
    raw_enabled: bool,                        // 生API保存の有効/無効（開発用、デフォルトOFF）
    raw_seq: u32,                             // 生APIダンプの連番カウンタ
}
```

- `GameState.sortie.battle_logger` として `GameStateInner` に保持される
- 初期化: `BattleLogger::new(sync_dir.join("battle_logs"), sync_dir.join("raw_api"))`
- 初期化時にディスクから既存レコードを読み込み（最大200件、新しい順）
- 中断されたレコード（`end_time == None`）は自動修復（`fix_interrupted_records`）

### 1.2 状態遷移図

```
                      api_req_map/start
                           |
                           v
    +--------------------------------------------------+
    |            IDLE（出撃していない）                  |
    |         active_sortie = None                      |
    |         pending_battle = None                     |
    +--------------------------------------------------+
                           |
             on_sortie_start() を呼び出し
             SortieRecord を生成、最初のノード追加
             ディスクに即時保存（クラッシュリカバリ）
                           |
                           v
    +--------------------------------------------------+
    |          IN_SORTIE（出撃中・非戦闘）              |
    |         active_sortie = Some(...)                 |
    |         pending_battle = None                     |
    +--------------------------------------------------+
           |                    |              ^
           |                    |              |
  api_req_map/next        api_req_sortie/     |
  on_map_next()           battle 等           |
  新ノード追加             on_battle()         |
           |                    |              |
           v                    v              |
    +------+-----+  +------------------------+|
    |  同じ状態に |  |  BATTLE_PENDING        ||
    |  戻る      |  |  pending_battle =      ||
    |            |  |    Some(PendingBattle)  ||
    +------+-----+  +------------------------+|
           |              |           |        |
           |   api_req_battle_        |        |
           |   midnight/battle        |        |
           |   on_midnight_battle()   |        |
           |   HP更新、夜戦フラグ     |        |
           |              |           |        |
           |              v           |        |
           |   +---------------------+|        |
           |   | MIDNIGHT_PENDING    ||        |
           |   | had_night_battle=   ||        |
           |   |   true             ||        |
           |   +---------------------+|        |
           |              |           |        |
           |              v           v        |
           |   +--------------------------+    |
           |   | api_req_sortie/          |    |
           |   |   battleresult           |    |
           |   | on_battle_result()       |    |
           |   | BattleDetail を組立て    |    |
           |   | 最終ノードに付与         |    |
           |   | ディスクに部分保存       |----+
           |   +--------------------------+
           |
           v
    +--------------------------------------------------+
    |            api_port/port                          |
    |            on_port()                              |
    |            end_time 設定、completed に追加         |
    |            ディスクに最終保存                      |
    |            active_sortie = None に戻る            |
    +--------------------------------------------------+
                           |
                           v
                    IDLE に遷移
```

### 1.3 イベント発火タイミング

| タイミング | Tauriイベント | ペイロード |
|---|---|---|
| 出撃開始 | `sortie-start` | `{ in_sortie: true }` |
| 出撃開始（詳細） | `sortie-update` | `SortieRecordSummary` |
| マップ移動 | `sortie-update` | `SortieRecordSummary` |
| 戦闘結果確定 | `sortie-update` | `SortieRecordSummary` |
| 帰投 | `sortie-complete` | `SortieRecordSummary` |
| 戦闘結果確定後 | `port-data` | 更新済みHP付きの艦隊サマリー |

---

## 2. メイン処理フロー（battle_log/mod.rs）

### 2.1 on_sortie_start

**トリガー:** `api_req_map/start`

1. リクエストボディから `api_maparea_id`, `api_mapinfo_no`, `api_deck_id` を解析
2. 指定艦隊の艦娘情報をスナップショット（名前、レベル、艦種、装備のマスターID・改修・熟練度）
3. レスポンスから最初のマス番号（`api_no`）、イベント種別（`api_color_no`）、イベントID（`api_event_id`）を取得
4. マルチゲージマップの場合、`api_eventmap.api_gauge_num` を記録
5. `SortieRecord` を生成し、最初の `BattleNode` を追加
6. ディスクに即時保存（クラッシュリカバリ用）
7. `active_sortie` にセット

### 2.2 on_map_next

**トリガー:** `api_req_map/next`

1. DTOからマス番号・イベント種別・イベントIDを取得
2. `active_sortie.nodes` に新しい `BattleNode` を追加
3. 大破進撃チェック（battle.rs側で実施）

### 2.3 on_battle / on_midnight_battle / on_battle_result

後述の「parser.rs」セクションで詳述。

### 2.4 on_port

**トリガー:** `api_port/port`

1. `active_sortie` を `take()` で取り出し
2. `end_time` に現在時刻を設定
3. `pending_battle` をクリア
4. ディスクに最終保存
5. `completed` の先頭に挿入（最大200件でtruncate）
6. GDrive同期通知（`battle_logs/{id}.json`）

---

## 3. 戦闘データ解析（battle_log/parser.rs）

### 3.1 昼戦処理 `on_battle`

**トリガー:** 以下のAPIエンドポイント群
- `api_req_sortie/battle`, `airbattle`, `ld_airbattle`, `ld_shooting`, `night_to_day`
- `api_req_combined_battle/battle`, `battle_water`, `each_battle`, `each_battle_water`, `ec_battle`, `ld_airbattle`, `ld_shooting`

**処理内容:**

1. `PendingBattle` を新規作成
2. **陣形解析:** `api_formation` → `[自軍陣形, 敵軍陣形, 交戦形態]`
3. **HP初期値記録:**
   - `api_f_nowhps` + `api_f_maxhps` → 味方HP
   - `api_e_nowhps` + `api_e_maxhps` → 敵HP
4. **敵艦情報:** `api_ship_ke` (艦船ID), `api_ship_lv` (レベル), `api_eSlot` (装備)
5. **航空戦解析:** `extract_air_battle()` で `api_kouku.api_stage1` から制空状態・機数を抽出
6. **夜戦フラグ:** `api_midnight_flag`
7. **味方HP計算:** `calculate_hp_after_from_start()` で各フェーズのダメージを順次適用
8. **敵HP計算:** `calculate_enemy_hp_after()` で同様に計算
9. 生JSON保存（`api_data` 部分）

### 3.2 夜戦処理 `on_midnight_battle`

**トリガー:**
- `api_req_battle_midnight/battle`, `sp_midnight`
- `api_req_combined_battle/midnight_battle`, `sp_midnight`, `ec_midnight_battle`, `ec_night_to_day`

**処理内容:**

1. `pending_battle` が存在しない場合（`sp_midnight` 等、夜戦開始の場合）は新規作成し、陣形・HP・敵艦情報を初期化
2. `had_night_battle = true` を設定
3. 夜戦開始時HPから `api_hougeki` のダメージを適用して味方・敵HPを更新
4. 生夜戦JSONを保存

### 3.3 戦闘結果処理 `on_battle_result`

**トリガー:** `api_req_sortie/battleresult`, `api_req_combined_battle/battleresult`

**処理内容:**

1. `pending_battle` を `take()` で取得
2. 基本情報: 勝敗ランク (`api_win_rank`), 敵艦隊名 (`api_enemy_info.api_deck_name`)
3. MVP艦インデックス (`api_mvp`, 1-based)、基本経験値 (`api_get_base_exp`)
4. 艦別獲得経験値 (`api_get_ship_exp`)
5. ドロップ判定: `api_get_flag[1] == 1` の場合、`api_get_ship` から艦名・IDを取得
6. 敵艦リスト構築: `pending.enemy_ship_ids` とマスターデータから名前を解決。レベル・装備配列はオフセット調整あり（`api_ship_ke` の `-1` パディングに対応）
7. HP状態構築: `pending` の before/after/max から `HpState` ベクタを構築
8. 生JSON結合: 昼戦JSONと夜戦JSONがある場合 `{ "day": ..., "night": ... }` に統合
9. `BattleDetail` を組み立てて `active_sortie` の最終ノードに付与
10. ディスクに部分保存（クラッシュリカバリ）

### 3.4 ダメージ計算フロー

昼戦のHPは以下の順序で各フェーズのダメージを適用する：

```
開始HP (api_f_nowhps / api_e_nowhps)
  │
  ├─ 1. api_kouku        (航空戦: api_stage3.api_fdam / api_edam)
  ├─ 2. api_opening_atack (開幕雷撃: api_fdam / api_edam)
  ├─ 3. api_opening_taisen(先制対潜: api_at_eflag + api_df_list + api_damage)
  ├─ 4. api_hougeki1      (砲撃戦1: 同上)
  ├─ 5. api_hougeki2      (砲撃戦2: 同上)
  ├─ 6. api_hougeki3      (砲撃戦3: 同上)
  └─ 7. api_raigeki       (雷撃戦: api_fdam / api_edam)
                           │
                           v
                     昼戦後HP
                           │
                     夜戦 api_hougeki (砲撃)
                           │
                           v
                     最終HP (0以下は0にクランプ)
```

### 3.5 砲撃ダメージ解析詳細

**新形式 (api_at_eflag あり):**
- `api_at_eflag[i] == 1` → 敵が攻撃 → ターゲットは味方艦（0-based）
- `api_at_eflag[i] == 0` → 味方が攻撃 → ターゲットは敵艦（0-based）
- `api_df_list[i]` = ターゲットインデックスの配列（連撃等で複数）
- `api_damage[i]` = ダメージ値の配列（f64→i32変換）

**旧形式 (api_at_eflag なし):**
- ターゲットインデックス 1-6 = 味方艦, 7-12 = 敵艦 (1-based)

### 3.6 航空戦解析

`api_kouku` から以下を抽出：

| フィールド | 値 | 説明 |
|---|---|---|
| `api_stage1.api_disp_seiku` | 0-4 | 制空状態 |
| `api_stage1.api_f_count` / `api_f_lostcount` | int | 味方機数 [出撃, 喪失] |
| `api_stage1.api_e_count` / `api_e_lostcount` | int | 敵機数 [出撃, 喪失] |

制空状態コード:
- 0: 航空劣勢
- 1: 航空優勢
- 2: 制空権確保
- 3: 航空均衡
- 4: 制空権喪失

---

## 4. ファイルI/O（battle_log/storage.rs）

### 4.1 ディレクトリ構造

```
{AppData}/com.eo.kancolle-browser/
└── sync/
    ├── battle_logs/              ← 出撃記録の永続化ディレクトリ
    │   ├── 20260313_143025.json  ← 個別出撃レコード（タイムスタンプID）
    │   ├── 20260313_151200.json
    │   └── ...
    └── raw_api/                  ← 生APIダンプ（開発用、デフォルト無効）
        ├── 20260313_143025_000_api_req_map_start.json
        ├── 20260313_143025_001_api_req_sortie_battle.json
        └── ...
```

### 4.2 ファイル命名規則

- **出撃レコード:** `{YYYYMMDD_HHMMSS}.json` （出撃開始時刻）
- **生APIダンプ:** `{YYYYMMDD_HHMMSS}_{3桁連番}_{エンドポイント名}.json`
  - エンドポイント名は `/kcsapi/` プレフィックスを除去し `/` を `_` に変換

### 4.3 保存形式（SortieRecord JSON）

```json
{
  "id": "20260313_143025",
  "fleet_id": 1,
  "map_area": 2,
  "map_no": 3,
  "map_display": "2-3",
  "ships": [
    {
      "name": "長門改二",
      "ship_id": 541,
      "lv": 99,
      "stype": 8,
      "slots": [
        { "id": 7, "rf": 6 },
        { "id": 7, "rf": 4 },
        { "id": 15 },
        { "id": 36, "mas": 7 }
      ],
      "slot_ex": { "id": 43, "rf": 2 }
    }
  ],
  "nodes": [
    {
      "cell_no": 1,
      "event_kind": 0,
      "event_id": 0,
      "battle": null
    },
    {
      "cell_no": 3,
      "event_kind": 4,
      "event_id": 4,
      "battle": {
        "rank": "S",
        "enemy_name": "敵前衛艦隊",
        "enemy_ships": [
          { "ship_id": 1501, "level": 1, "name": "軽巡ホ級", "slots": [501, 502] }
        ],
        "formation": [1, 1, 1],
        "air_battle": {
          "air_superiority": 1,
          "friendly_plane_count": [20, 3],
          "enemy_plane_count": [10, 8]
        },
        "friendly_hp": [
          { "before": 77, "after": 72, "max": 77 }
        ],
        "enemy_hp": [
          { "before": 30, "after": 0, "max": 30 }
        ],
        "drop_ship": "睦月",
        "drop_ship_id": 1,
        "mvp": 1,
        "base_exp": 120,
        "ship_exp": [360, 120, 120, 120, 120, 120],
        "night_battle": false
      }
    }
  ],
  "start_time": "2026-03-13T14:30:25+09:00",
  "end_time": "2026-03-13T14:35:10+09:00",
  "is_combined": false,
  "gauge_num": null
}
```

### 4.4 保存タイミング

| タイミング | 保存内容 | 目的 |
|---|---|---|
| 出撃開始 | 初期 SortieRecord（nodes=[最初のマス]） | クラッシュリカバリ |
| 戦闘結果確定 | 部分 SortieRecord（BattleDetail 付きノード追加済み） | クラッシュリカバリ |
| 帰投（on_port） | 完成 SortieRecord（end_time 設定済み） | 最終保存 |

### 4.5 読み込み

**起動時 (`load_from_disk`):**
1. `save_dir` 内の全 `.json` ファイルを走査
2. `SortieRecord` にデシリアライズ
3. レガシーフォーマットのマイグレーション（`BattleNode.migrate_legacy()`）
4. `start_time` 降順でソート
5. 最大200件に制限

**日付範囲検索 (`get_records_by_date_range`):**
1. ファイル名の先頭8文字（`YYYYMMDD`）で日付フィルタ
2. ファイル名降順ソート後にJSONを読み込み
3. レガシーマイグレーション適用

**同期後リロード (`reload_from_disk`):**
- GDrive同期で新ファイルがダウンロードされた後に呼び出し
- `load_from_disk` + `fix_interrupted_records`

### 4.6 中断レコード修復 (`fix_interrupted_records`)

- `end_time == None` かつ `active_sortie` でないレコードを検出
- `end_time = start_time` を設定して再保存
- アプリクラッシュ等で帰投前に終了した場合の救済

### 4.7 GDrive同期

- `battle_logs/` ディレクトリは `SyncTarget` として登録
- 帰投時に `notify_sync` で同期エンジンに通知
- 同期後に `reload_from_disk` で他端末のレコードも読み込み

---

## 5. データ構造

### 5.1 バックエンド（Rust）

```rust
/// 出撃レコード全体
struct SortieRecord {
    id: String,                          // タイムスタンプID "YYYYMMDD_HHMMSS"
    fleet_id: i32,                       // 艦隊番号 (1-based)
    map_area: i32,                       // 海域番号 (例: 2)
    map_no: i32,                         // マップ番号 (例: 3)
    map_display: String,                 // 表示文字列 (例: "2-3")
    ships: Vec<SortieShip>,              // 出撃時の艦隊スナップショット
    nodes: Vec<BattleNode>,              // 通過マスの一覧
    start_time: DateTime<Local>,         // 出撃開始時刻
    end_time: Option<DateTime<Local>>,   // 帰投時刻（進行中はNone）
    is_combined: bool,                   // 連合艦隊フラグ
    gauge_num: Option<i32>,              // マルチゲージ番号
}

/// 出撃時の艦娘スナップショット
struct SortieShip {
    name: String,                        // 艦名
    ship_id: i32,                        // マスター艦船ID
    lv: i32,                             // レベル
    stype: i32,                          // 艦種ID
    slots: Vec<SlotItemSnapshot>,        // 通常装備スロット
    slot_ex: Option<SlotItemSnapshot>,   // 補強増設スロット
}

/// 装備スナップショット
struct SlotItemSnapshot {
    id: i32,                             // マスター装備ID
    rf: i32,                             // 改修値 (0-10, ★)
    mas: Option<i32>,                    // 熟練度 (0-7, >>)
}

/// 1マスの情報
struct BattleNode {
    cell_no: i32,                        // マス番号
    event_kind: i32,                     // イベント種別 (api_color_no)
    event_id: i32,                       // イベントID (5=ボス)
    battle: Option<BattleDetail>,        // 戦闘詳細（戦闘がない場合はNone）
}

/// 戦闘詳細データ
struct BattleDetail {
    rank: String,                        // 勝敗ランク (S/A/B/C/D/E)
    enemy_name: String,                  // 敵艦隊名
    enemy_ships: Vec<EnemyShip>,         // 敵艦情報
    formation: [i32; 3],                 // [自軍陣形, 敵軍陣形, 交戦形態]
    air_battle: Option<AirBattleResult>, // 航空戦結果
    friendly_hp: Vec<HpState>,           // 味方HP状態
    enemy_hp: Vec<HpState>,              // 敵HP状態
    drop_ship: Option<String>,           // ドロップ艦名
    drop_ship_id: Option<i32>,           // ドロップ艦マスターID
    mvp: Option<i32>,                    // MVP艦インデックス (1-based)
    base_exp: Option<i32>,               // 基本取得経験値
    ship_exp: Vec<i32>,                  // 艦別取得経験値
    night_battle: bool,                  // 夜戦発生/可能フラグ
    raw_battle: Option<Value>,           // 生戦闘API JSON（保存時のみ）
    raw_result: Option<Value>,           // 生戦闘結果API JSON（保存時のみ）
}

/// HP状態
struct HpState {
    before: i32,                         // 戦闘前HP
    after: i32,                          // 戦闘後HP
    max: i32,                            // 最大HP
}

/// 敵艦情報
struct EnemyShip {
    ship_id: i32,                        // マスター艦船ID
    level: i32,                          // レベル
    name: Option<String>,                // 艦名（マスターデータから解決）
    slots: Vec<i32>,                     // 装備マスターID
}

/// 航空戦結果
struct AirBattleResult {
    air_superiority: Option<i32>,        // 制空状態 (0-4)
    friendly_plane_count: Option<[i32; 2]>,  // [出撃機数, 喪失機数]
    enemy_plane_count: Option<[i32; 2]>,     // [出撃機数, 喪失機数]
}

/// 戦闘API蓄積中の中間データ（バックエンド内部のみ）
struct PendingBattle {
    friendly_hp_before: Vec<(i32, i32)>, // (現在HP, 最大HP)
    friendly_hp_after: Vec<(i32, i32)>,
    enemy_hp_before: Vec<(i32, i32)>,
    enemy_hp_after: Vec<(i32, i32)>,
    formation: [i32; 3],
    enemy_ship_ids: Vec<i32>,
    enemy_ship_levels: Vec<i32>,
    enemy_ship_slots: Vec<Vec<i32>>,
    air_battle: Option<AirBattleResult>,
    midnight_flag: bool,
    had_night_battle: bool,
    raw_battle_json: Option<Value>,
    raw_midnight_json: Option<Value>,
}
```

### 5.2 フロントエンド（TypeScript）

フロントエンドの型定義は `src/types/battle.ts` に集約されている。バックエンドの `SortieRecordSummary` がフロントエンドの `SortieRecord` に対応する。

主な差異:
- `start_time` / `end_time` は `DateTime<Local>` → `string`（`%Y-%m-%d %H:%M:%S` フォーマット）
- `map_area`, `map_no`, `is_combined`, `gauge_num` はサマリーに含まれない
- `raw_battle`, `raw_result` はフロントエンドに送信されない

### 5.3 レガシーフォーマットのマイグレーション

旧バージョンでは `BattleNode` に戦闘データが直接フラットに格納されていた。`migrate_legacy()` メソッドが旧フィールドを検出し、`BattleDetail` に変換する。旧フィールドは `skip_serializing` のためJSON出力には含まれない。

---

## 6. API処理ルーティング（api/battle.rs）

### 6.1 エンドポイント判定

`is_battle_endpoint()` で以下のプレフィックスにマッチするAPIを戦闘系として識別:

- `/kcsapi/api_req_map/`
- `/kcsapi/api_req_sortie/`
- `/kcsapi/api_req_battle_midnight/`
- `/kcsapi/api_req_combined_battle/`

### 6.2 エンドポイント分類

| カテゴリ | エンドポイント | 処理メソッド |
|---|---|---|
| 出撃開始 | `api_req_map/start` | `on_sortie_start` |
| マップ移動 | `api_req_map/next` | `on_map_next` |
| 昼戦 | `api_req_sortie/battle` | `on_battle` |
| 航空戦 | `api_req_sortie/airbattle` | `on_battle` |
| 長距離航空戦 | `api_req_sortie/ld_airbattle` | `on_battle` |
| 長距離射撃 | `api_req_sortie/ld_shooting` | `on_battle` |
| 夜→昼戦 | `api_req_sortie/night_to_day` | `on_battle` |
| 通常夜戦 | `api_req_battle_midnight/battle` | `on_midnight_battle` |
| 開幕夜戦 | `api_req_battle_midnight/sp_midnight` | `on_midnight_battle` |
| 連合昼戦 | `api_req_combined_battle/*` (6種) | `on_battle` |
| 連合夜戦 | `api_req_combined_battle/*_midnight*` (4種) | `on_midnight_battle` |
| 戦闘結果 | `api_req_sortie/battleresult` | `on_battle_result` |
| 連合結果 | `api_req_combined_battle/battleresult` | `on_battle_result` |

### 6.3 戦闘結果後の副作用

`on_battle_result` 呼び出し後に `api/battle.rs` で以下を実行:

1. **艦娘HP更新:** 戦闘後HPを `profile.ships` に反映し、`port-data` イベントを再送信
2. **任務進捗更新:** マップ/ランク/ボス判定から出撃任務の進捗を更新
3. **撃沈敵艦種判定:** 敵HP≦0の艦の艦種を抽出し、撃沈系任務に反映
4. **戦果記録:** `api_get_exp`（HQ経験値）と `api_get_exmap_rate`（EO戦果ボーナス）を戦果トラッカーに記録
5. **陣形記憶:** 選択陣形を `{area}-{no}-{cell}` キーで保存し、次回同マスでヒント表示

### 6.4 大破進撃警告（api_req_map/next）

マップ移動時に以下を判定:
- 出撃艦隊の各艦のHP/最大HP比が25%以下（大破）
- ダメコン装備の有無（`icon_type == 14`）
- ダメコンなしの大破艦がいる場合、ゲームオーバーレイに警告を表示

---

## 7. フロントエンド表示

### 7.1 コンポーネント構成

```
BattleTab                              ← 戦闘ログ一覧タブ
├── DateRangePicker                    ← 日付範囲選択
├── BattleDetailView                   ← 出撃詳細表示（選択時）
│   ├── MapRouteView                   ← マップルート可視化（SVG+スプライト）
│   └── BattleNodeDetail[]             ← 各戦闘マスの詳細
│       └── BattleHpBar                ← HP増減バー（共通コンポーネント）
```

### 7.2 BattleTab（一覧画面）

- **フィルタバー:** 日付範囲ピッカー + プリセット（今日/今月/全て）+ マップフィルタ（プルダウン）
- **レコード一覧:** マップ名、艦隊番号、艦名一覧、各ノードの勝敗ランクバッジ（色付き）、ドロップ艦、出撃日時
- レコードクリックで `BattleDetailView` に遷移
- `onRefresh` で再取得、`onDateChange` で日付範囲変更
- `totalRecords` でバックエンド総件数を表示

### 7.3 BattleDetailView（詳細画面）

- **ヘッダ:** 戻るボタン、マップ名、艦隊番号、出撃日時、制空計算ボタン（外部サイト連携）
- **出撃艦隊:** 艦名 + レベル一覧
- **マップ+戦闘スプリットビュー:** 上部にマップ、下部に戦闘ノード一覧。境界線はマウスドラッグでリサイズ可能（比率 15%-80%）
- マップ上のセルクリックで対応する戦闘ノードにスクロール＋ハイライト（1.5秒）

### 7.4 MapRouteView（マップルート表示）

- ゲーム内マップアセットを `get_cached_resource` / `get_map_sprite` コマンドで取得
- **描画レイヤー (z-index順):**
  1. 背景画像（terrain, `bg[0]`）
  2. セルマーカー（`bg[1]`）
  3. ルートスプライト（`route_N`、通過済みはカラー、未通過はグレースケール40%透明）
  4. SVGオーバーレイ: 接続線（破線）、出撃マーカー、通過セル（色分け円 + ラベル）
- セルの色はイベント種別に応じて `CELL_COLORS` で決定
- ボスマスは半径18px、通常マスは14px
- マップデータ未キャッシュ時はフォールバックメッセージ表示

### 7.5 BattleNodeDetail（戦闘マス詳細）

- **ヘッダ行:** マスラベル（英字またはマス番号）、イベント種別、勝敗ランク（色付き）、敵艦隊名、MVP艦名、獲得経験値、夜戦バッジ、ドロップ艦
- **陣形行:** 自軍陣形 vs 敵軍陣形 | 交戦形態
- **航空戦行:** 制空状態（色付き）、味方機残/出撃数、敵機残/出撃数
- **HP表示:** 味方艦隊と敵艦隊を左右に並べて表示。各艦のHP増減バーで戦闘前→戦闘後の変化を可視化

### 7.6 定数定義（constants.ts）

| 定数 | 用途 |
|---|---|
| `FORMATION_NAMES` | 陣形ID → 日本語名 (1:単縦陣 〜 14:第四警戒航行序列) |
| `ENGAGEMENT_NAMES` | 交戦形態ID → 日本語名 (1:同航戦 〜 4:T字不利) |
| `RANK_COLORS` | ランク文字 → 色 (S:金, A:赤, B:橙, C/D/E:灰系) |
| `EVENT_LABELS` | `api_color_no` → イベント種別名 (0:始点 〜 10:泊地) |
| `EVENT_ID_LABELS` | `api_event_id` → イベント名 (6:航路選択) |
| `AIR_SUPERIORITY_LABELS` | 制空状態ID → テキスト+色 |

---

## 8. Tauriコマンド

| コマンド | 引数 | 戻り値 | 説明 |
|---|---|---|---|
| `get_battle_logs` | `limit?`, `offset?`, `date_from?`, `date_to?` | `{ records, total }` | 戦闘ログ取得。日付指定時はディスク全検索 |
| `clear_battle_logs` | なし | `()` | メモリ+ディスクの全レコード削除 |
| `set_raw_api_enabled` | `enabled: bool` | `()` | 生APIダンプの有効/無効切替 |
| `clear_raw_api` | なし | `()` | 生APIダンプの全削除 |

---

## 9. 関連ファイル一覧

### バックエンド
- `src-tauri/src/battle_log/mod.rs` — データ構造定義、BattleLogger本体、状態管理
- `src-tauri/src/battle_log/parser.rs` — 戦闘API解析、ダメージ計算
- `src-tauri/src/battle_log/storage.rs` — ファイル永続化、読み込み、マイグレーション
- `src-tauri/src/api/battle.rs` — APIルーティング、副作用処理
- `src-tauri/src/api/dto/battle.rs` — API DTOの型定義
- `src-tauri/src/commands.rs` — Tauriコマンド定義

### フロントエンド
- `src/components/battle/BattleTab.tsx` — 戦闘ログ一覧画面
- `src/components/battle/BattleDetailView.tsx` — 出撃詳細・スプリットビュー
- `src/components/battle/MapRouteView.tsx` — マップルート可視化
- `src/components/battle/BattleNodeDetail.tsx` — 戦闘マス詳細表示
- `src/components/battle/constants.ts` — 表示用定数
- `src/types/battle.ts` — TypeScript型定義
