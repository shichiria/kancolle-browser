<!-- AUTO-GENERATED from source code -->

# API傍受 詳細設計書

## 1. 概要

本アプリケーションは艦これブラウザゲームのAPI通信を傍受(インターセプト)し、ゲーム状態をリアルタイムに解析・表示する。API傍受はHTTPSプロキシを経由して行われ、macOS/Windows両プラットフォームで共通の仕組みを使用する。

## 2. アーキテクチャ

### 2.1 通信フロー

```
WebView (game-content)
  │
  │ proxy_url で接続
  ▼
hudsucker Proxy (127.0.0.1:19080)
  │
  ├─ /kcsapi/* → API傍受パイプライン
  │   ├─ handle_request():  リクエストボディ取得 (POST data)
  │   ├─ handle_response(): レスポンスボディ取得・デコード
  │   └─ api::process_api(): 状態更新 + イベント発行
  │
  ├─ /kcs2/*  → リソースキャッシュ (maybe_cache_resource)
  │
  └─ その他   → そのまま透過 (CONNECT tunnel)
```

### 2.2 プロキシ層 (`src-tauri/src/proxy/mod.rs`)

- **ライブラリ**: `hudsucker` (MITM対応HTTPSプロキシ)
- **ポート**: 固定 `19080`(使用中の場合はOS割り当て)
- **CA証明書**: `kancolle-browser/ca.{key,cert}.pem` を `dirs::data_local_dir()` 配下に永続化
- **MITM対象**: `*.kancolle-server.com` およびIPアドレス直接接続のみ。DMM/CDNはトンネル透過

#### KanColleHandler (HttpHandler実装)

| メソッド | 責務 |
|---|---|
| `should_intercept()` | CONNECT時にMITM対象かを判定。ゲームサーバードメインのみtrue |
| `handle_request()` | `/kcsapi/` パスの場合、リクエストボディを `RequestDataMap` に保存 |
| `handle_response()` | APIレスポンスのデコード(gzip展開、`svdata=` プレフィックス除去)後に `process_api()` を呼び出し |

#### リクエストデータの管理

```rust
type RequestDataMap = Arc<Mutex<HashMap<SocketAddr, (String, String)>>>;
```

HTTP/1.1の接続単位で `SocketAddr` をキーとし、`(URI, リクエストボディ)` を保持する。レスポンス処理時に `remove()` で取得・削除し、メモリリークを防止する。

#### レスポンスデコード処理

1. レスポンスボディのバイト列を取得
2. `Content-Encoding: gzip` の場合は `flate2` で展開
3. `svdata=` プレフィックスを除去してJSON文字列を取得
4. `api::process_api()` に `(endpoint, json_str, request_body)` を渡す
5. `kancolle-api` イベントとして生のAPIデータをフロントエンドにも発行

### 2.3 プラットフォーム別の接続方式

| プラットフォーム | 方式 | 詳細 |
|---|---|---|
| macOS | WKWebView + `proxy_url` | Tauri WebviewBuilder の `proxy_url()` で hudsucker プロキシを指定 |
| Windows | WebView2 + `proxy_url` | 同上。WebView2もTauriの `proxy_url()` を使用 |

両プラットフォームとも `game_window.rs` で `WebviewBuilder::proxy_url()` を設定し、同一の hudsucker プロキシに接続する。

## 3. ディスパッチテーブル (`process_api`)

`src-tauri/src/api/mod.rs` の `process_api()` 関数がAPIエンドポイントに応じてパースと振り分けを行う。

### 3.1 パース(同期フェーズ)

呼び出しスレッド上でJSONパースを行い、`ParsedApi` enumに変換する。大きなJSON文字列のクローンを避けるため、asyncタスク起動前にパースを完了させる。

### 3.2 APIパス → ParsedApi → ハンドラ 対応表

| APIエンドポイント | ParsedApi | ハンドラ関数 | ファイル |
|---|---|---|---|
| `/kcsapi/api_start2/getData` | `Start2` | `process_start2()` | `api/mod.rs` |
| `/kcsapi/api_port/port` | `Port` | `process_port()` | `api/mod.rs` |
| `/kcsapi/api_get_member/slot_item` | `SlotItem` | (インライン処理) | `api/mod.rs` |
| `/kcsapi/api_get_member/require_info` | `SlotItem` | (インライン処理) | `api/mod.rs` |
| `/kcsapi/api_get_member/questlist` | `QuestList` | `process_questlist()` | `api/mod.rs` |
| `/kcsapi/api_req_hensei/change` | `HenseiChange` | `fleet::process_hensei_change()` | `api/fleet.rs` |
| `/kcsapi/api_req_hensei/preset_select` | `HenseiPresetSelect` | `fleet::process_hensei_preset_select()` | `api/fleet.rs` |
| `/kcsapi/api_req_kousyou/remodel_slot` | `RemodelSlot` | (インライン処理) | `api/mod.rs` |
| `/kcsapi/api_req_quest/start` | `QuestStart` | (インライン処理) | `api/mod.rs` |
| `/kcsapi/api_req_quest/stop` | `QuestStop` | (インライン処理) | `api/mod.rs` |
| `/kcsapi/api_req_quest/clearitemget` | `QuestClear` | (インライン処理) | `api/mod.rs` |
| `/kcsapi/api_req_practice/battle_result` | `ExerciseResult` | `battle::process_exercise_result()` | `api/battle.rs` |
| `/kcsapi/api_get_member/ship3` | `Ship3` | `ship::process_ship3()` | `api/ship.rs` |
| `/kcsapi/api_req_kaisou/slot_deprive` | `SlotDeprive` | `ship::process_slot_deprive()` | `api/ship.rs` |
| `/kcsapi/api_req_ranking/mxltvkpyuklh` | `Ranking` | (インライン処理) | `api/mod.rs` |
| 戦闘系エンドポイント群 | `Battle` | `battle::process_battle()` | `api/battle.rs` |
| その他 | `Other` | (何もしない) | - |

### 3.3 戦闘系エンドポイント判定 (`is_battle_endpoint`)

以下のプレフィックスに一致するものが `Battle` として処理される:

| プレフィックス | 対象 |
|---|---|
| `/kcsapi/api_req_map/*` | 出撃開始・進撃 |
| `/kcsapi/api_req_sortie/*` | 通常艦隊戦闘 |
| `/kcsapi/api_req_battle_midnight/*` | 夜戦 |
| `/kcsapi/api_req_combined_battle/*` | 連合艦隊戦闘 |

#### 戦闘サブディスパッチ (`battle::process_battle`)

| エンドポイント | 処理カテゴリ |
|---|---|
| `api_req_map/start` | 出撃開始 |
| `api_req_map/next` | 進撃(次マス) |
| `api_req_sortie/battle` | 通常昼戦 |
| `api_req_sortie/airbattle` | 航空戦 |
| `api_req_sortie/ld_airbattle` | 長距離空襲 |
| `api_req_sortie/ld_shooting` | レーダー射撃 |
| `api_req_sortie/night_to_day` | 夜→昼戦 |
| `api_req_combined_battle/battle` | 連合艦隊昼戦(機動部隊) |
| `api_req_combined_battle/battle_water` | 連合艦隊昼戦(水上部隊) |
| `api_req_combined_battle/each_battle` | 連合艦隊各個昼戦 |
| `api_req_combined_battle/each_battle_water` | 連合艦隊各個昼戦(水上) |
| `api_req_combined_battle/ec_battle` | 敵連合艦隊戦 |
| `api_req_combined_battle/ld_airbattle` | 連合艦隊長距離空襲 |
| `api_req_combined_battle/ld_shooting` | 連合艦隊レーダー射撃 |
| `api_req_battle_midnight/battle` | 通常夜戦 |
| `api_req_battle_midnight/sp_midnight` | 開幕夜戦 |
| `api_req_combined_battle/midnight_battle` | 連合艦隊夜戦 |
| `api_req_combined_battle/sp_midnight` | 連合艦隊開幕夜戦 |
| `api_req_combined_battle/ec_midnight_battle` | 敵連合艦隊夜戦 |
| `api_req_combined_battle/ec_night_to_day` | 敵連合艦隊夜→昼戦 |
| `api_req_sortie/battleresult` | 戦闘結果(通常) |
| `api_req_combined_battle/battleresult` | 戦闘結果(連合) |

## 4. 非同期タスクと状態更新の順序保証

`process_api()` は以下の3ステップを **単一の async タスク** 内で実行し、状態更新の順序を保証する:

1. **raw API ファイル名の割り当て**: `GameStateInner` を短時間 write lock して、ファイル名とシーケンス番号を取得(I/O なし)
2. **raw API のディスク書き出し**: ロック外でファイル書き込み
3. **状態更新**: `GameStateInner` を write lock して `ParsedApi` に基づく状態更新を実行

```rust
tauri::async_runtime::spawn(async move {
    // Step 1: allocate filename (brief lock, no I/O)
    // Step 2: write raw API to disk (outside lock)
    // Step 3: re-acquire lock for state updates
});
```

## 5. 各ハンドラの処理内容

### 5.1 `process_start2` — マスタデータ読み込み

- **入力**: `ApiStart2` (レスポンスボディ)
- **書き込み先**: `state.master.*`

| フィールド | データソース | 内容 |
|---|---|---|
| `master.ships` | `api_mst_ship` | 艦船ID → 名前・艦種 |
| `master.stypes` | `api_mst_stype` | 艦種ID → 名前 |
| `master.missions` | `api_mst_mission` | 遠征ID → 名前・時間 |
| `master.slotitems` | `api_mst_slotitem` | 装備ID → 名前・種別・アイコン・ステータス |
| `master.equip_types` | `api_mst_slotitem_equiptype` | 装備種別ID → 名前 |

- **イベント**: `master-data-loaded` (各種マスタデータ件数)

### 5.2 `process_port` — 母港データ更新

- **入力**: `ApiPort` (レスポンスボディ)
- **書き込み先**: `state.profile.*`, `state.sortie.*`, `state.senka`

| 処理 | 詳細 |
|---|---|
| 出撃終了判定 | `battle_logger.is_in_sortie()` の場合 `on_port()` でログ確定 |
| 任務進捗リセット | `quest_progress::check_resets()` |
| 艦船情報更新 | `profile.ships` にプレイヤー艦船を再構築 |
| 艦隊編成更新 | `profile.fleets` に艦隊構成を反映 |
| 連合艦隊フラグ | `profile.combined_flag` を更新 |
| 艦隊サマリ構築 | 各艦のダメコン・特殊装備・先制対潜を判定しリッチなサマリを生成 |
| 入渠情報構築 | ドック状態に艦名を付与 |
| HQ経験値 | `senka.update_experience()` で戦果トラッカーを更新 |

- **イベント**: `port-data` (PortSummary), `sortie-complete` (出撃終了時), `senka-updated`

### 5.3 `SlotItem` 処理 — 装備一覧更新

- **入力**: `Vec<PlayerSlotItemApi>` (レスポンスボディ)
- **書き込み先**: `state.profile.slotitems`
- **ソース**: `api_get_member/slot_item` または `api_get_member/require_info` 内の `api_slot_item`
- `slotitems` を全クリアして再構築
- **イベント**: なし

### 5.4 `process_questlist` — 任務一覧更新

- **入力**: `ApiQuestListResponse` (レスポンスボディ)
- **書き込み先**: `state.history.active_quests`, `state.history.active_quest_details`

| api_state | 処理 |
|---|---|
| 2 (受託中) / 3 (達成) | `active_quests` に追加、`active_quest_details` にタイトル・カテゴリを保存 |
| 1 (未受託) | `active_quests` / `active_quest_details` から削除 |

- **イベント**: `quest-list-updated` (ActiveQuestDetail の配列)

### 5.5 `RemodelSlot` — 装備改修

- **入力**: リクエストボディ (`RemodelSlotReq`)、レスポンスボディ (`ApiRemodelSlotResponse`)
- **書き込み先**: `state.history.improved_equipment`
- 改修成功時 (`api_remodel_flag == 1`) のみ、マスタ装備IDを `improved_equipment` に追加
- ディスク永続化: `improvement::save_improved_history()`
- **同期通知**: `improved_equipment.json`
- **イベント**: なし

### 5.6 任務開始・中止・完了

#### QuestStart (`api_req_quest/start`)
- **書き込み先**: `state.history.active_quests` に追加
- **イベント**: `quest-started` (quest_id)

#### QuestStop (`api_req_quest/stop`)
- **書き込み先**: `active_quests`, `active_quest_details` から削除
- **イベント**: `quest-list-updated`, `quest-stopped` (quest_id)

#### QuestClear (`api_req_quest/clearitemget`)
- **書き込み先**: `active_quests`, `active_quest_details` から削除、戦果ボーナス加算
- 戦果ボーナス抽出: `api_bounus` 配列内の `api_type == 18` エントリを集計
- **イベント**: `quest-list-updated`, `quest-stopped`, `senka-updated` (ボーナスがある場合)

### 5.7 `battle::process_battle` — 戦闘処理

#### 出撃開始 (`api_req_map/start`)
- **入力**: レスポンスJSON + リクエストボディ
- **書き込み先**: `state.sortie.battle_logger`
- `on_sortie_start()` で出撃セッションを開始
- 陣形ヒント表示: `formation_memory` に記録があればオーバーレイに表示
- **イベント**: `sortie-start`, `sortie-update`

#### 進撃 (`api_req_map/next`)
- **入力**: `ApiMapNextResponse`
- **処理**:
  1. 大破進撃警告: 旗艦以外のHP<=25%の艦にダメコンがない場合、ゲームオーバーレイに警告表示
  2. `battle_logger.on_map_next()` でノード情報を記録
  3. 陣形ヒント表示
  4. 1-6ゴールノード(event_id=9)での戦果EO加算
- **イベント**: `sortie-update`, `senka-updated` (1-6 EO時)

#### 昼戦 (通常/連合)
- **入力**: `ApiBattleResponse`
- **処理**: `battle_logger.on_battle()` で戦闘データ記録
- 陣形を `formation_memory` に保存(`api_formation[0]`)
- ディスク永続化: `formation_memory.json`
- 陣形ヒントウィンドウを非表示
- **同期通知**: `formation_memory.json`

#### 夜戦 (通常/連合)
- **入力**: `ApiBattleResponse`
- **処理**: `battle_logger.on_midnight_battle()` で夜戦データ記録

#### 戦闘結果 (`battleresult`)
- **入力**: `ApiBattleResultResponse`
- **処理**:
  1. `battle_logger.on_battle_result()` でログ確定
  2. 戦闘後HPを `profile.ships` に反映
  3. キャッシュ済み `port_data` のHP更新 → `port-data` 再発行
  4. 任務進捗更新: `quest_progress::on_battle_result()`
  5. HQ経験値(`api_get_exp`)を戦果トラッカーに加算
  6. EO戦果(`api_get_exmap_rate`)を加算
- **イベント**: `sortie-update`, `port-data`, `quest-progress-updated`, `senka-updated`

### 5.8 `battle::process_exercise_result` — 演習結果

- **入力**: レスポンスJSON
- **処理**:
  1. HQ経験値を戦果トラッカーに加算
  2. 任務進捗更新: `quest_progress::on_exercise_result()`
- **イベント**: `senka-updated`, `quest-progress-updated`

### 5.9 `ship::process_ship3` — 装備変更後の艦船更新

- **入力**: レスポンスJSON (`api_ship_data`, `api_deck_data`)
- **書き込み先**: `state.profile.ships`, `state.profile.fleets`
- `api_ship_data` から該当艦の情報を再構築
- `api_deck_data` から艦隊編成を更新
- **イベント**: `fleet-updated` (FleetSummary の配列, `emit_fleet_update()` 経由)

### 5.10 `ship::process_slot_deprive` — 装備剥ぎ取り

- **入力**: レスポンスJSON (`api_ship_data.api_set_ship`, `api_ship_data.api_unset_ship`)
- **書き込み先**: `state.profile.ships`
- 装備を渡した側・受け取った側の2隻分を更新
- **イベント**: `fleet-updated` (`emit_fleet_update()` 経由)

### 5.11 `fleet::process_hensei_change` — 艦隊編成変更

- **入力**: リクエストボディ (`HenseiChangeReq`)
- **書き込み先**: `state.profile.fleets`

| api_ship_id | 処理 |
|---|---|
| `-2` | 旗艦以外を全解除 |
| `-1` | 指定位置の艦を解除 |
| `> 0` | 指定位置に配置(他艦隊にいれば入れ替え) |

- **イベント**: `fleet-updated` (`emit_fleet_update()` 経由)

### 5.12 `fleet::process_hensei_preset_select` — プリセット読み込み

- **入力**: `ApiHenseiPresetSelectResponse` (レスポンスボディ)
- **書き込み先**: `state.profile.fleets`
- プリセットの艦が他艦隊にいる場合は引き抜き
- **イベント**: `fleet-updated` (`emit_fleet_update()` 経由)

### 5.13 `Ranking` — ランキングデータ

- **入力**: 生のJSONテキスト
- **処理**: `senka::decrypt_ranking()` で復号。提督名を使って自分のランキング順位・戦果を確認
- **書き込み先**: `state.senka`
- **イベント**: `senka-updated` (自分のデータがあった場合)

## 6. DTO構造

### 6.1 レスポンスDTO (`api/dto/battle.rs`)

| 構造体 | 用途 | 主要フィールド |
|---|---|---|
| `ApiBattleResponse` | 昼戦/夜戦レスポンス | `api_formation`, `api_ship_ke`, `api_f_nowhps`, `api_e_nowhps`, `api_kouku`, `api_hougeki*`, `api_raigeki` |
| `ApiKouku` | 航空戦 | `api_stage1` (制空情報) |
| `ApiKoukuStage1` | 航空戦第1ステージ | `api_disp_seiku` (制空状態), `api_f_count`/`api_f_lostcount` |
| `ApiHougeki` | 砲撃戦 | `api_at_eflag` (攻撃側), `api_df_list` (防御側), `api_damage` |
| `ApiRaigeki` | 雷撃戦 | `api_fdam` (味方被ダメ), `api_edam` (敵被ダメ) |
| `ApiMapNextResponse` | 次マス情報 | `api_no`, `api_color_no`, `api_event_id` |
| `ApiBattleResultResponse` | 戦闘結果 | `api_win_rank`, `api_get_ship`, `api_mvp`, `api_get_base_exp` |
| `ApiGetShip` | ドロップ艦 | `api_ship_id`, `api_ship_name` |
| `ApiEnemyInfo` | 敵艦隊情報 | `api_deck_name` |
| `ApiQuestListResponse` | 任務一覧 | `api_list` (Value配列) |
| `ApiRemodelSlotResponse` | 装備改修結果 | `api_remodel_flag`, `api_after_slot` |
| `ApiAfterSlot` | 改修後装備 | `api_slotitem_id` |
| `ApiHenseiPresetSelectResponse` | プリセット読み込み | `api_fleet` (Value) |

### 6.2 リクエストDTO (`api/dto/request.rs`)

| 構造体 | 用途 | フィールド |
|---|---|---|
| `HenseiChangeReq` | 編成変更 | `api_id` (艦隊番号), `api_ship_idx` (位置), `api_ship_id` (艦船ID) |
| `RemodelSlotReq` | 装備改修 | `api_slot_id` (装備インスタンスID), `api_id` (マスタ装備ID) |
| `QuestReq` | 任務操作 | `api_quest_id` |

### 6.3 汎用レスポンスラッパー (`api/models.rs`)

```rust
struct ApiResponse<T> {
    api_result: i32,        // 1 = 成功
    api_result_msg: Option<String>,
    api_data: Option<T>,
}
```

全てのKanColle APIレスポンスはこのラッパーでデシリアライズされる。

## 7. GameState への書き込みパターン

### 7.1 状態構造

```
GameState (Arc<RwLock<GameStateInner>>)
├── master: MasterData           // マスタデータ (api_start2で設定、セッション中不変)
│   ├── ships:      HashMap<i32, MasterShipInfo>
│   ├── stypes:     HashMap<i32, String>
│   ├── missions:   HashMap<i32, MissionInfo>
│   ├── slotitems:  HashMap<i32, MasterSlotItemInfo>
│   └── equip_types: HashMap<i32, String>
├── profile: UserProfile         // プレイヤーデータ (母港帰還時に全更新)
│   ├── ships:      HashMap<i32, ShipInfo>
│   ├── slotitems:  HashMap<i32, PlayerSlotItem>
│   ├── fleets:     Vec<Vec<i32>>
│   └── combined_flag: i32
├── sortie: SortieState          // 出撃セッション
│   ├── battle_logger: BattleLogger
│   └── last_port_summary: Option<PortSummary>
├── history: UserHistory         // 蓄積データ
│   ├── active_quests:        HashSet<i32>
│   ├── active_quest_details: HashMap<i32, ActiveQuestDetail>
│   ├── improved_equipment:   HashSet<i32>
│   ├── quest_progress:       QuestProgressState
│   └── sortie_quest_defs:    Vec<SortieQuestDef>
├── senka: SenkaTracker          // 戦果トラッカー
├── formation_memory: HashMap<String, i32>  // 陣形記憶
└── sync_notifier: Option<mpsc::Sender>     // 同期通知チャネル
```

### 7.2 書き込みパターン一覧

| パターン | 説明 | 例 |
|---|---|---|
| 全クリア+再構築 | `clear()` してから全要素を `insert()` | `process_start2` (master各種), `process_port` (ships, fleets), `SlotItem` 処理 |
| 差分更新 | 特定エントリのみ `insert()` / `remove()` | `process_ship3`, `process_slot_deprive`, `process_hensei_change` |
| セット追加/削除 | `HashSet` への `insert()`/`remove()` | 任務開始/停止 (`active_quests`), 改修成功 (`improved_equipment`) |
| ロガー委譲 | `BattleLogger` のメソッドに処理を委譲 | `on_sortie_start`, `on_battle`, `on_midnight_battle`, `on_battle_result`, `on_port` |

### 7.3 ディスク永続化

| ファイル | タイミング | 関数 |
|---|---|---|
| `improved_equipment.json` | 改修成功時 | `improvement::save_improved_history()` |
| `quest_progress.json` | 戦闘結果・演習結果時 | `quest_progress::on_battle_result()` 等 |
| `formation_memory.json` | 戦闘開始時(陣形記録) | `models::save_formation_memory()` |
| `battle_logs/*.json` | 出撃完了時 | `BattleLogger::on_port()` |
| `raw_api/*.json` | 全API受信時 | `battle_log::save_raw_api_to_disk()` |
| `senka.json` | 戦果変動時 | `SenkaTracker` 内部 |

## 8. イベント発行パターン (`app.emit`)

### 8.1 イベント一覧

| イベント名 | ペイロード型 | 発行元 | 発行タイミング |
|---|---|---|---|
| `kancolle-api` | `ApiEvent` | `proxy/mod.rs` | 全API受信時(生データ) |
| `master-data-loaded` | `{ shipCount, stypeCount, ... }` | `process_start2` | マスタデータ読み込み完了 |
| `port-data` | `PortSummary` | `process_port`, 戦闘結果後 | 母港帰還、戦闘後HP更新 |
| `fleet-updated` | `Vec<FleetSummary>` | `emit_fleet_update()` | 編成変更、装備変更、プリセット読み込み |
| `sortie-start` | `{ in_sortie: true }` | `process_battle` (map/start) | 出撃開始 |
| `sortie-update` | `SortieRecordSummary` | `process_battle` | 出撃中の状態変化(進撃、戦闘結果) |
| `sortie-complete` | `SortieRecordSummary` | `process_port` | 母港帰還で出撃確定 |
| `quest-list-updated` | `Vec<&ActiveQuestDetail>` | `process_questlist`, 任務停止/完了 | 任務一覧変更 |
| `quest-started` | `i32` (quest_id) | `QuestStart` 処理 | 任務受諾 |
| `quest-stopped` | `i32` (quest_id) | `QuestStop`/`QuestClear` 処理 | 任務中止・完了 |
| `quest-progress-updated` | 進捗データ | 戦闘結果・演習結果 | 任務進捗変化 |
| `senka-updated` | `SenkaSummary` | 各種 | 戦果変動(HQ経験値、EO、任務、ランキング) |

### 8.2 オーバーレイ操作 (eval による直接注入)

| 操作 | メソッド | タイミング |
|---|---|---|
| 大破進撃警告 | `overlay.eval("window.showTaihaWarning(...)")` | `api_req_map/next` で大破艦検出時 |
| ミニマップ更新 | `overlay.eval("window.updateMinimap(...)")` | 出撃開始・進撃時 |
| ミニマップ非表示 | `overlay.eval("window.hideMinimap()")` | 母港帰還時 |

### 8.3 陣形ヒントウィンドウ (`formation.rs`)

| 操作 | 関数 | タイミング |
|---|---|---|
| 表示 | `show_formation_hint()` | 出撃開始・進撃時(過去の陣形記憶がある場合) |
| 非表示 | `hide_formation_hint()` | 戦闘開始時(昼戦) |

陣形ヒントは独立したウィンドウ (`formation-hint`) をゲームウィンドウ上の該当ボタン位置に重ねて表示する。ゲーム座標系(1200x720)からの座標変換はzoom率とDPIスケールを考慮して計算される。

## 9. 同期通知パターン

状態変更後にGDrive同期エンジンへの通知は `notify_sync()` ヘルパーを通じて行われる:

```rust
fn notify_sync(state: &GameStateInner, paths: Vec<&str>) {
    if let Some(tx) = &state.sync_notifier {
        tx.try_send(SyncCommand::UploadChanged(paths));
    }
}
```

`try_send()` を使用し、同期チャネルが満杯の場合は通知を破棄する(ブロッキング回避)。

## 10. 補助機能

### 10.1 先制対潜判定 (`ship::check_opening_asw`)

艦船の装備・ステータスから先制対潜攻撃の可否を判定する。以下の優先順位で評価:

1. 無条件対潜艦 (五十鈴改二、Fletcher級など)
2. 護衛空母 (対潜1以上の航空機があれば可)
3. 海防艦 (対潜60+ソナー or 対潜75+装備対潜4以上)
4. 軽空母 (対潜50+ソナー+対潜7航空機 / 対潜65+対潜7航空機 / 対潜100+ソナー+対潜1爆撃機)
5. 日向改二 (S-51J系1隻以上 or 回転翼2機以上)
6. 航空戦艦 (対潜100+ソナー+対潜装備)
7. 汎用艦 (DD/CL/CLT/CT/AO: 対潜100+ソナー)

### 10.2 司令部施設判定 (`ship::resolve_command_facility`)

| 装備ID | 名称 | 発動条件 |
|---|---|---|
| 107 | 艦隊司令部施設 | 連合艦隊の第1艦隊旗艦 |
| 413 | 精鋭水雷戦隊 司令部 | 非連合、旗艦CL/DD、随伴全員DD/CLT |
| 272 | 遊撃部隊 艦隊司令部 | 第3艦隊、7隻編成 |

### 10.3 戦果ボーナス抽出 (`extract_senka_from_clearitemget`)

任務完了レスポンスの `api_bounus` 配列から `api_type == 18`(ランキングポイント)のエントリを検出し、`senka_item_bonus()` でポイント値に変換して合計する。
