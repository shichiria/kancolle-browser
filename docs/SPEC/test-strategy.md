# テスト戦略仕様書

## 背景と目的

機能追加時に「1つ直せば1つバグが出る」手戻りが頻発していた。
原因: データ変換ロジックに対するリグレッションテストが不在。

**目的**: APIの生ログデータを入力として、データ変換・判定ロジックの自動反復テストを確立する。

## 現在の状態

- **53テスト全パス** (0.19秒)
- A群37エンドポイント + B群23エンドポイント = **60エンドポイント処理済み**
- DTO整理完了: `dto/battle.rs`(戦闘系), `dto/member.rs`(艦船/装備/任務系), `dto/ranking.rs`(ランキング)
- 全75エンドポイントのサンプルfixture取得済み（最新版、サニタイズ済み）

## テスト対象の原則

```
テスト可能              テスト対象外
─────────────          ──────────────
APIレスポンス解析        ゲーム自動操作(規約違反)
データ変換ロジック       Tauri IPC / WebView
状態更新ロジック         プロキシ通信
条件判定ロジック         Google Drive同期
計算・集計処理           UI描画(React Component)
ユーティリティ関数
```

## テスト全体像

```
┌─────────────────────────────────────────────────────────┐
│                    テスト階層                              │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  Layer 1: Deserialization (APIレスポンス→構造体)     ✅   │
│  ───────────────────────────────────────────────         │
│  53テスト実装済み (A群36 + B群14 + 横断3)               │
│                                                         │
│  Layer 2: Transformation (構造体→ゲーム状態)     未実装   │
│  ───────────────────────────────────────────────         │
│  入力: パース済み構造体 + マスターデータ                  │
│  検証: ShipInfo, FleetSummary, PortSummary等の生成       │
│                                                         │
│  Layer 3: Logic (判定・計算)                      未実装   │
│  ───────────────────────────────────────────────         │
│  入力: ゲーム状態                                        │
│  検証: 遠征条件判定、任務条件判定、戦果計算、             │
│        任務進捗リセット、ダメージ計算                     │
│                                                         │
│  Layer 4: Integration (API→状態更新→イベント)    未実装   │
│  ───────────────────────────────────────────────         │
│  入力: 生APIログ列(出撃1回分等)                         │
│  検証: 一連の状態遷移が正しいか                          │
│                                                         │
│  Layer 5: Frontend Utils (TS ユーティリティ)     未実装   │
│  ───────────────────────────────────────────────         │
│  入力: 各関数の引数                                      │
│  検証: フォーマット、色判定、predeck URL生成              │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

---

## APIエンドポイント完全リスト (75種)

生ログ8,164件(3月以降)から抽出。全エンドポイントの処理状況・テスト状況を明記。

### A. 処理済み＋テスト済み ✅

コード内で `process_api()` がハンドルし、デシリアライゼーションテストが存在するAPI。

#### 基本API (15個)

| # | エンドポイント | DTO | テスト | 処理内容 |
|---|--------------|-----|-------|---------|
| A1 | `api_start2/getData` | `ApiStart2` | a01 | マスターデータ |
| A2 | `api_port/port` | `ApiPort` | a02 | 母港 |
| A3 | `api_get_member/slot_item` | `Vec<PlayerSlotItemApi>` | a03 | 装備リスト |
| A4 | `api_get_member/require_info` | Value→`api_slot_item` | a04 | 装備リスト(抽出) |
| A5 | `api_get_member/questlist` | `ApiQuestListResponse` | a05 | 任務リスト |
| A6 | `api_get_member/ship3` | `ApiShip3Response` | a06 | 艦船更新 |
| A7 | `api_req_hensei/change` | request_body | a07 | 編成変更 |
| A8 | `api_req_hensei/preset_select` | `ApiHenseiPresetSelectResponse` | a08 | プリセット読込 |
| A9 | `api_req_kousyou/remodel_slot` | `ApiRemodelSlotResponse` | a09 | 装備改修 |
| A10 | `api_req_quest/start` | request_body | a10 | 任務受諾 |
| A11 | `api_req_quest/stop` | request_body | a10 | 任務放棄 |
| A12 | `api_req_quest/clearitemget` | Value→`api_bounus` | a12 | 任務完了 |
| A13 | `api_req_practice/battle_result` | `ApiExerciseResultResponse` | a13 | 演習結果 |
| A14 | `api_req_kaisou/slot_deprive` | `ApiSlotDepriveResponse` | a14 | 装備移動 |
| A15 | `api_req_ranking/mxltvkpyuklh` | `ApiRankingResponse` | a15 | ランキング暗号解読 |

#### 戦闘系 — 昼戦 (12個、DTO: `ApiBattleResponse`)

| # | エンドポイント | fixture有無 | テスト |
|---|--------------|-----------|-------|
| A16 | `api_req_sortie/battle` | ✅ | a16 |
| A17 | `api_req_sortie/airbattle` | ✅ | a17 |
| A18 | `api_req_sortie/ld_airbattle` | ✅ | a18 |
| A19-A27 | `api_req_sortie/ld_shooting`, `night_to_day`, `combined_battle/*` | (未取得) | — |

#### 戦闘系 — 夜戦 (6個、DTO: `ApiBattleResponse`)

| # | エンドポイント | fixture有無 | テスト |
|---|--------------|-----------|-------|
| A28 | `api_req_battle_midnight/battle` | ✅ | a28 |
| A29 | `api_req_battle_midnight/sp_midnight` | ✅ | a29 |
| A30-A33 | `api_req_combined_battle/*` | (未取得) | — |

#### 戦闘結果・マップ

| # | エンドポイント | DTO | テスト |
|---|--------------|-----|-------|
| A34 | `api_req_sortie/battleresult` | `ApiBattleResultResponse` | a34 |
| A35 | `api_req_combined_battle/battleresult` | (未取得) | — |
| A36 | `api_req_map/start` | Value (BattleLogger) | a36 |
| A37 | `api_req_map/next` | `ApiMapNextResponse` | a37 |

---

### B. 処理済み＋テスト済み ✅ (新規実装)

今回のTDDセッションで実装されたAPI。プロキシ傍受時に自動的に状態更新される。

#### 艦船/装備 状態更新

| # | エンドポイント | DTO / ParsedApi | テスト | 処理内容 |
|---|--------------|----------------|-------|---------|
| B1 | `api_req_hokyu/charge` | `ApiChargeResponse` | b01 | 補給→燃料/弾薬更新+fleet-updated |
| B2 | `api_get_member/ship_deck` | `ApiShip3Response`(再利用) | b02 | 艦船+艦隊更新(ship3と同構造) |
| B3 | `api_req_kaisou/powerup` | `ApiPowerupResponse` | b03 | 近代化改修→艦船+艦隊更新 |
| B4 | `api_req_kaisou/slot_exchange_index` | `ApiSlotExchangeResponse` | b04 | 装備入替→艦船更新 |
| B5 | `api_req_kousyou/getship` | `ApiGetShipResponse` | b05 | 建造完了→新艦+装備追加 |

#### 削除操作

| # | エンドポイント | ParsedApi | テスト | 処理内容 |
|---|--------------|----------|-------|---------|
| B6 | `api_req_kousyou/destroyitem2` | `DestroyItem2{item_ids}` | b06 | 装備廃棄→slotitems除去 |
| B7 | `api_req_kousyou/destroyship` | `DestroyShip{ship_id}` | b07 | 解体→ships/fleets除去 |
| B8 | `api_req_kousyou/createitem` | `ApiCreateItemResponse` | b08 | 開発→装備追加 |

#### リソース/情報更新

| # | エンドポイント | DTO | テスト | 処理内容 |
|---|--------------|-----|-------|---------|
| B9 | `api_get_member/material` | `Vec<Material>` | b09 | 資材8種更新→port-data再emit |
| B10 | `api_get_member/ndock` | `Vec<RepairDock>` | b10 | 入渠ドック更新→port-data再emit |
| B11 | `api_get_member/deck` | `Vec<Fleet>` | b11 | 艦隊更新→fleet-updated |
| B12 | `api_req_mission/result` | `ApiMissionResultResponse` | b12 | 遠征結果(ログ) |

#### 演習戦闘

| # | エンドポイント | DTO | テスト | 処理内容 |
|---|--------------|-----|-------|---------|
| B13a | `api_req_practice/battle` | `ApiBattleResponse` | b13 | Battle variantに統合 |
| B13b | `api_req_practice/midnight_battle` | `ApiBattleResponse` | b13 | Battle variantに統合 |

#### ログのみ (状態は後続APIで更新)

| # | エンドポイント | ParsedApi | テスト | 備考 |
|---|--------------|----------|-------|------|
| B14a | `api_req_kaisou/slotset` | LogOnly | b14 | ship3で更新 |
| B14b | `api_req_kaisou/slotset_ex` | LogOnly | b14 | ship3で更新 |
| B14c | `api_req_kaisou/unsetslot_all` | LogOnly | b14 | ship3で更新 |
| B14d | `api_req_kaisou/preset_slot_select` | LogOnly | b14 | ship3で更新 |
| B14e | `api_req_kaisou/remodeling` | LogOnly | b14 | portで更新 |
| B14f | `api_req_kousyou/createship` | LogOnly | b14 | getshipで更新 |
| B14g | `api_req_kousyou/createship_speedchange` | LogOnly | b14 | getshipで更新 |
| B14h | `api_req_mission/start` | LogOnly | b14 | portで更新 |
| B14i | `api_get_member/mapinfo` | LogOnly | b14 | 情報のみ |

---

### C. 未処理 — 状態影響なし/軽微 ℹ️

ゲーム状態に直接影響しない参照系API。サンプル取得済み、テスト優先度低。

| # | エンドポイント | 件数 | fixture | 備考 |
|---|--------------|------|---------|------|
| C1 | `api_get_member/preset_deck` | 295 | ✅ | プリセット一覧 |
| C2 | `api_req_kaisou/can_preset_slot_select` | 264 | ✅ | 装備プリセット可否 |
| C3 | `api_get_member/chart_additional_info` | 250 | ✅ | 戦績画面 |
| C4 | `api_get_member/useitem` | 184 | ✅ | 消費アイテム |
| C5 | `api_start2/get_option_setting` | 161 | ✅ | ゲーム設定 |
| C6-C29 | その他21エンドポイント | — | ✅ | 全fixture取得済み |

---

## 実装済みテスト一覧 (53テスト)

### Suite 1: APIデシリアライゼーション (実装済み ✅)

```
cargo test --lib api::tests     # 53テスト, 0.19秒
```

#### A群テスト (39テスト)

| モジュール | テスト数 | 検証内容 |
|-----------|---------|---------|
| a01_start2 | 1 | マスターデータ(艦船1674+, 装備721+, 艦種22+, 任務63+) |
| a02_port | 2 | 母港パース + get_material ヘルパー |
| a03_slot_item | 1 | 装備インスタンスリスト(3000+件) |
| a04_require_info | 1 | require_info→slot_item抽出 |
| a05_questlist | 1 | 任務リスト構造(api_no/state/title/category) |
| a06_ship3 | 1 | 艦船+艦隊データ(型付きDTO) |
| a07_hensei_change | 1 | 編成変更request_body(fleet_id/ship_idx/ship_id) |
| a08_preset_select | 2 | プリセット艦隊(api_id/api_ship直接アクセス) |
| a09_remodel_slot | 2 | 装備改修(レスポンス+リクエスト) |
| a10_quest_start_stop | 2 | 任務受諾/放棄(quest_id) |
| a12_quest_clearitemget | 2 | 任務完了+戦果ボーナス抽出(extract_senka_from_clearitemget) |
| a13_practice_result | 1 | 演習結果(型付きDTO: api_win_rank/api_get_exp) |
| a14_slot_deprive | 1 | 装備移動(型付きDTO: api_set_ship/api_unset_ship) |
| a15_ranking | 1 | ランキング(型付きDTO: 暗号化フィールド) |
| a16_sortie_battle | 2 | 昼戦(陣形/HP/砲撃構造) |
| a17_airbattle | 1 | 航空戦(api_kouku/stage1) |
| a18_ld_airbattle | 1 | 長距離空襲 |
| a28_midnight_battle | 1 | 夜戦(api_hougeki) |
| a29_sp_midnight | 1 | 開幕夜戦 |
| a34_battleresult | 3 | 戦闘結果(ランク/MVP/ドロップ/敵情報) |
| a36_map_start | 2 | 出撃開始(リクエスト+レスポンス) |
| a37_map_next | 1 | マス進行(api_no/event_id/color_no) |
| cross_cutting | 5 | 全戦闘DTO互換性, 全fixture api_result, 演習, エッジケース |

#### B群テスト (14テスト)

| モジュール | テスト数 | 検証内容 |
|-----------|---------|---------|
| b01_charge | 3 | 補給DTO(艦船/資材/全艦バリデーション) |
| b02_ship_deck | 1 | ship_deck→ApiShip3Response互換 |
| b03_powerup | 1 | 近代化改修(api_ship/powerup_flag) |
| b04_slot_exchange | 1 | 装備入替(api_ship_data) |
| b05_getship | 1 | 建造完了(api_ship/api_ship_id) |
| b06_destroyitem2 | 1 | 装備廃棄request(api_slotitem_ids) |
| b07_destroyship | 1 | 解体request(api_ship_id) |
| b08_createitem | 1 | 開発(api_create_flag) |
| b09_material | 1 | 資材8種(api_id 1-8) |
| b10_ndock | 1 | 入渠ドック4基(api_id 1-4) |
| b11_deck | 1 | 艦隊データ(api_id/api_ship) |
| b12_mission_result | 1 | 遠征結果(api_clear_result) |
| b13_practice_battles | 2 | 演習昼戦/夜戦のApiBattleResponse互換性 |
| b14_log_only | 1 | ログのみ9エンドポイントのapi_result検証 |

---

## DTO整理 (実施済み ✅)

### ParsedApi enum 構成

```rust
enum ParsedApi {
    // A群: 基本API (型付きDTO)
    Start2(ApiStart2), Port(ApiPort), SlotItem(Vec<PlayerSlotItemApi>),
    QuestList(ApiQuestListResponse), ExerciseResult(ApiExerciseResultResponse),
    HenseiChange{..}, HenseiPresetSelect(ApiHenseiPresetSelectResponse),
    RemodelSlot{..}, QuestStart{..}, QuestStop{..}, QuestClear{..},
    Ship3(ApiShip3Response), SlotDeprive(ApiSlotDepriveResponse),
    Ranking(ApiRankingResponse),

    // A群: 戦闘API (Value — endpoint別3種DTOに分岐)
    Battle(Value),

    // B群: 状態更新API (型付きDTO)
    Charge(ApiChargeResponse),
    Powerup(ApiPowerupResponse), SlotExchange(ApiSlotExchangeResponse),
    GetShip(ApiGetShipResponse),
    DestroyItem2{item_ids}, DestroyShip{ship_id},
    CreateItem(ApiCreateItemResponse),
    MemberMaterial(Vec<Material>), MemberNDock(Vec<RepairDock>),
    MemberDeck(Vec<Fleet>), MissionResult(ApiMissionResultResponse),

    // B群: ログのみ
    LogOnly,
    Other,
}
```

### DTOファイル配置

```
src-tauri/src/api/dto/
├── mod.rs          # モジュールエクスポート
├── battle.rs       # 戦闘系DTO (ApiBattleResponse, ApiBattleResultResponse, ApiMapNextResponse)
├── member.rs       # 艦船/装備/任務系DTO (Ship3, SlotDeprive, Charge, Powerup, etc.)
├── ranking.rs      # ランキングDTO (ApiRankingResponse, ApiRankingEntry)
└── request.rs      # リクエストDTO (HenseiChangeReq, RemodelSlotReq, QuestReq)
```

---

## 未実装テストスイート (今後のTDDで実装)

### Suite 2: データ変換

| # | テスト名 | 検証ポイント |
|---|---------|-------------|
| 2.1 | build_ship_info | 各ステータス値(火力/雷装/対空/装甲/対潜/回避/索敵/運) |
| 2.2 | extract_stat_value | 配列[0]が装備込み値 |
| 2.3 | extract_slot_ids | -1=空スロット保持 |
| 2.4 | collect_ship_marks | ダメコン/対潜/大発等フラグ |
| 2.5-2.8 | PortSummary/FleetSummary/資材/ドック構築 | 名前解決, enrich |

### Suite 3: 戦闘解析 (battle_log/parser.rs)

| # | テスト名 | 検証ポイント |
|---|---------|-------------|
| 3.1-3.6 | HP計算/航空戦/陣形/敵艦隊 | ダメージ全フェーズ合算 |
| 3.7-3.11 | MVP/ドロップ/勝利判定/大破 | 戦闘結果の正確な導出 |
| 3.12-3.15 | 砲撃/雷撃/支援/基地航空 | 個別攻撃フェーズ |

### Suite 4: 出撃記録 (battle_log/mod.rs)

Integration fixture: `sequences/sortie_multi_battle/` (4戦闘の出撃シーケンス16ファイル)

### Suite 5: 任務進捗 (quest_progress/mod.rs)

| # | テスト名 | 検証ポイント |
|---|---------|-------------|
| 5.1-5.5 | リセット判定 | デイリー/ウィークリー/マンスリー/クォータリー/年跨ぎ |
| 5.6-5.10 | 進捗管理 | インクリメント/上限/複数エリア/保存読込 |

### Suite 6-10: 判定ロジック・フロントエンド

遠征条件(18種), 出撃任務(10種), 戦果計算, 編成変更, FE Utils

---

## テストフィクスチャ管理

### ディレクトリ構造

```
src-tauri/tests/fixtures/
├── samples/                     # 全75エンドポイントの最新生ログ(サニタイズ済)
│   └── (75 JSON files)
└── sequences/                   # 統合テスト用シーケンス
    └── sortie_multi_battle/     # 4戦闘(昼×3+昼夜×1)の出撃 (16 files)
```

### サニタイズ済み項目

- `api_token`, `api_verno` → 削除
- `api_nickname` → "TestAdmiral"
- `api_member_id` → 0
- `api_comment`, `api_comment_id`, `api_nickname_id` → "0"

### fixture更新方法

1. アプリで `raw_api` を有効化してプレイ
2. `sync/raw_api/` から最新ログを取得（3月以前のログは削除済み）
3. サニタイズスクリプトで機密情報除去
4. `tests/fixtures/samples/` に配置

---

## テスト実行

```bash
# 全テスト (53テスト, ~0.2秒)
cargo test --lib api::tests

# 特定グループのみ
cargo test --lib api::tests::a01    # マスターデータ
cargo test --lib api::tests::b01    # 補給
cargo test --lib api::tests::cross  # 横断テスト

# 将来: フロントエンドテスト
npx vitest run src/utils/
```

---

## 開発ワークフロー

今後の機能追加・バグ修正は **/tdd** でテストファーストで実施:

```
1. /tdd で新機能のテストを先に書く (RED)
2. 実装して通す (GREEN)
3. リファクタ (REFACTOR)
4. 既存53テストも全て通ることを確認 → リグレッションなし
```

## テストデータ構築パターン

### Private フィールドを持つ構造体

`Material` 等の `#[serde(flatten)]` で private `_extra` フィールドを持つ構造体は、テストモジュール外からリテラル構築できない。

```rust
// NG: private フィールドがあるため外部モジュールからは構築不可
let m = models::Material { api_id: 1, api_value: 100, _extra: ... };

// OK: serde_json::from_value で構築
let materials: Vec<models::Material> = serde_json::from_value(serde_json::json!([
    {"api_id": 1, "api_value": 100},
    {"api_id": 2, "api_value": 200}
])).unwrap();
```

ヘルパー関数 (`get_material`) は `api/mod.rs` に配置し、アプリロジックとテストの両方からアクセス可能にする。

### Rustの型推論補助

複雑なイテレータチェーンやチャネル操作では、コンパイラの局所型推論が限界に達する場合がある:

```rust
// HashMap::iter() のクロージャに型注釈
.iter().map(|(k, v): (&i32, &String)| { ... })

// チャネル送信の型注釈
let _: Result<(), _> = tx.send(value);
```

## 成功基準

- **カバレッジ**: データ変換・判定ロジックの80%以上 (現在Layer 1完了)
- **リグレッション検出**: 既知バグケースをテストに追加し再発防止
- **実行速度**: 全テスト30秒以内 (現在0.2秒)
- **メンテナンス性**: 新API対応時にfixtureを追加するだけでテスト拡張可能
