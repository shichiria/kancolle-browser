<!-- AUTO-GENERATED from source code -->

# 基本設計: データフロー

本ドキュメントは、KanColle Browser における API 傍受から UI 表示までの全体データフローを記述する。

---

## 目次

1. [全体アーキテクチャ概観](#1-全体アーキテクチャ概観)
2. [API 傍受方式](#2-api-傍受方式)
3. [process_api() のディスパッチフロー](#3-process_api-のディスパッチフロー)
4. [GameState の構造と書き込み](#4-gamestate-の構造と書き込み)
5. [フロントエンドへのイベント通知](#5-フロントエンドへのイベント通知)
6. [Tauri コマンド経由のデータ取得](#6-tauri-コマンド経由のデータ取得)
7. [ファイル I/O と永続化](#7-ファイルio-と永続化)
8. [Google Drive 同期](#8-google-drive-同期)

---

## 1. 全体アーキテクチャ概観

```
 +-----------+       HTTPS        +-------------------+       HTTP(S)       +------------------+
 | KanColle  | <================> | hudsucker Proxy    | <================> | WebView          |
 | Server    |   (MITM intercept) | (127.0.0.1:19080) |   (proxy_url)      | (game-content)   |
 +-----------+                    +-------------------+                    +------------------+
                                         |                                        |
                                    (1) Parse                              (8) Tauri event
                                    (2) process_api()                           listen()
                                         |                                        |
                                         v                                        v
                                  +-------------------+                    +------------------+
                                  | GameState          |  (7) invoke()     | React Frontend   |
                                  | Arc<RwLock<        | <===============  | (App.tsx)         |
                                  |   GameStateInner>> |  Tauri commands   |                  |
                                  +-------------------+                    +------------------+
                                         |
                                    (3) File I/O
                                    (4) Sync notify
                                         |
                                         v
                                  +-------------------+
                                  | Local Disk         |
                                  | sync/ + local/     |
                                  +-------------------+
                                         |
                                    (5) Google Drive
                                         Sync Engine
```

### データフローの3つの経路

| 経路 | 方向 | 説明 |
|------|------|------|
| Push型 | Backend -> Frontend | API 傍受時に `app.emit()` でイベント送信 |
| Pull型 | Frontend -> Backend | `invoke()` で Tauri コマンドを呼び出し |
| 永続化 | Backend -> Disk | ファイル I/O で JSON 保存、GDrive 同期 |

---

## 2. API 傍受方式

### 統一アーキテクチャ: プロキシ方式 (macOS / Windows 共通)

macOS / Windows 共に **同一のプロキシベース傍受方式** を採用している。
MEMORY.md に記載の「Windows WebView2 ネイティブ API」は過去の検討であり、
現在の実装では両プラットフォームとも `proxy_url()` 経由でプロキシを使用する。

```
  macOS: WKWebView + proxy_url(http://127.0.0.1:19080)
  Windows: WebView2 + proxy_url(http://127.0.0.1:19080)
         |
         v
  hudsucker MITM Proxy (src-tauri/src/proxy/mod.rs)
         |
         +-- should_intercept(): *.kancolle-server.com のみ MITM
         +-- handle_request(): /kcsapi/ リクエストの body を保存
         +-- handle_response(): レスポンスを解析 → process_api() 呼出
```

### プロキシ起動シーケンス

```
lib.rs::setup()
  |
  +-- proxy::start_proxy(app_handle, cache_dir)
  |     |
  |     +-- load_or_generate_ca()        ... CA証明書のロード/生成
  |     +-- TcpListener::bind(19080)     ... 固定ポート(fallback: OS割当)
  |     +-- Proxy::builder()             ... hudsucker プロキシ構築
  |     +-- emit("proxy-ready", port)    ... フロントエンドに通知
  |
  +-- app.manage(GameState::new(data_dir))
```

### レスポンス処理の詳細

```
KanColleHandler::handle_response()
  |
  +-- URI から /kcsapi/ を判定
  |     +-- 非API → maybe_cache_resource() でリソースキャッシュ
  |     +-- API → handle_api_response() へ
  |
  +-- handle_api_response()
        |
        +-- レスポンスボディ読み取り
        +-- gzip 圧縮の場合はデコンプレス
        +-- "svdata=" プレフィックス除去 → JSON 文字列取得
        +-- api::process_api() 呼出
        +-- app.emit("kancolle-api", ApiEvent) ... 生 API イベント
```

---

## 3. process_api() のディスパッチフロー

`src-tauri/src/api/mod.rs` の `process_api()` は、すべての API データ処理の
エントリーポイントである。**呼び出しスレッド**でパース、**非同期タスク**で
状態更新を行う2段階構成になっている。

### Phase 1: 同期パース (呼び出しスレッド)

```
process_api(app_handle, endpoint, json_str, request_body)
  |
  +-- endpoint でマッチ → ParsedApi enum にパース
  |
  |   /kcsapi/api_start2/getData        → ParsedApi::Start2
  |   /kcsapi/api_port/port             → ParsedApi::Port
  |   /kcsapi/api_get_member/slot_item  → ParsedApi::SlotItem
  |   /kcsapi/api_get_member/questlist  → ParsedApi::QuestList
  |   /kcsapi/api_req_hensei/change     → ParsedApi::HenseiChange
  |   /kcsapi/api_req_kousyou/remodel.. → ParsedApi::RemodelSlot
  |   /kcsapi/api_req_quest/start       → ParsedApi::QuestStart
  |   /kcsapi/api_req_quest/stop        → ParsedApi::QuestStop
  |   /kcsapi/api_req_quest/clearitem.. → ParsedApi::QuestClear
  |   /kcsapi/api_req_practice/battle.. → ParsedApi::ExerciseResult
  |   /kcsapi/api_req_ranking/...       → ParsedApi::Ranking
  |   api_req_map/* | api_req_sortie/*  → ParsedApi::Battle
  |   api_req_battle_midnight/*         → ParsedApi::Battle
  |   api_req_combined_battle/*         → ParsedApi::Battle
  |   その他                            → ParsedApi::Other
```

### Phase 2: 非同期状態更新 (tokio::spawn)

```
tauri::async_runtime::spawn(async move {
  |
  +-- Step 1: RwLock 短時間取得
  |     +-- allocate_raw_api_filename()  ... ファイル名とシーケンス番号のみ確保
  |
  +-- Step 2: ロック外で I/O
  |     +-- save_raw_api_to_disk()       ... 生 API ダンプをディスク書き込み
  |
  +-- Step 3: RwLock 再取得 → 状態更新
        |
        +-- ParsedApi に応じて各処理関数を呼出:
        |
        +-- Start2  → process_start2()     ... マスターデータ更新
        +-- Port    → process_port()       ... 艦船/艦隊/資源/入渠更新
        +-- Battle  → battle::process_battle()  ... 出撃/戦闘ログ
        +-- QuestList → process_questlist() ... 任務一覧更新
        +-- ...
})
```

### Battle API サブディスパッチ

`battle::process_battle()` はさらにエンドポイントで分岐する:

```
process_battle(state, endpoint, request_body, json, app)
  |
  +-- api_req_map/start      → on_sortie_start()    ... 出撃開始
  +-- api_req_map/next       → on_map_next()         ... 次セル移動
  |                            + 大破進撃警告
  |                            + 陣形ヒント表示
  |                            + ミニマップ更新
  +-- api_req_sortie/battle  → on_battle()            ... 昼戦
  |   (+ 各種連合艦隊戦闘)     + 陣形メモリ保存
  +-- api_req_battle_midnight → on_midnight_battle()  ... 夜戦
  +-- battleresult            → on_battle_result()    ... 戦闘結果
                                + HP 更新 → port-data 再emit
                                + 任務進捗更新
                                + 戦果(senka)記録
                                + EO ボーナス計算
```

---

## 4. GameState の構造と書き込み

### データ構造 (src-tauri/src/api/models.rs)

```
GameState {
  inner: Arc<RwLock<GameStateInner>>   ... スレッドセーフなラッパー
}

GameStateInner {
  +-- master: MasterData               ... api_start2 のマスターデータ (不変)
  |     +-- ships: HashMap<i32, MasterShipInfo>
  |     +-- stypes: HashMap<i32, String>
  |     +-- missions: HashMap<i32, MissionInfo>
  |     +-- slotitems: HashMap<i32, MasterSlotItemInfo>
  |     +-- equip_types: HashMap<i32, String>
  |
  +-- profile: UserProfile             ... プレイヤーの保有データ (毎port更新)
  |     +-- ships: HashMap<i32, ShipInfo>
  |     +-- slotitems: HashMap<i32, PlayerSlotItem>
  |     +-- fleets: Vec<Vec<i32>>       ... 艦隊編成 (艦船IDリスト)
  |     +-- combined_flag: i32
  |
  +-- sortie: SortieState              ... 出撃セッション (一時的)
  |     +-- battle_logger: BattleLogger
  |     +-- last_port_summary: Option<PortSummary>
  |
  +-- history: UserHistory             ... 蓄積データ (永続化対象)
  |     +-- active_quests: HashSet<i32>
  |     +-- active_quest_details: HashMap<i32, ActiveQuestDetail>
  |     +-- improved_equipment: HashSet<i32>
  |     +-- quest_progress: QuestProgressState
  |     +-- sortie_quest_defs: Vec<SortieQuestDef>
  |
  +-- senka: SenkaTracker              ... 戦果トラッカー
  +-- formation_memory: HashMap<String, i32>  ... 陣形記憶
  +-- sync_notifier: Option<mpsc::Sender<SyncCommand>>  ... GDrive同期
  +-- data_dir, improved_equipment_path, quest_progress_path, ...
}
```

### ロック戦略

```
process_api() 内の非同期タスク:

  Step 1: inner.write().await   ← 短時間 (ファイル名確保のみ、I/O なし)
          ↓ drop lock
  Step 2: save_raw_api_to_disk  ← ロック外で I/O 実行
          ↓
  Step 3: inner.write().await   ← 状態更新 + emit (I/O はしない)

  ※ ファイル I/O (battle_log 保存等) はロック内で行われるケースもある
     (BattleLogger::save_to_disk は同期 fs::write)
```

---

## 5. フロントエンドへのイベント通知

### イベント一覧

| イベント名 | ペイロード型 | 発火タイミング | 発火元 |
|-----------|-------------|--------------|--------|
| `proxy-ready` | `u16` (port) | プロキシ起動完了 | lib.rs setup |
| `port-data` | `PortSummary` | 母港画面表示 / 戦闘後HP更新 | process_port / battleresult |
| `fleet-updated` | `Vec<FleetSummary>` | 編成変更 | process_hensei_change |
| `master-data-loaded` | JSON | api_start2 処理完了 | process_start2 |
| `kancolle-api` | `ApiEvent` | 全 API レスポンス | proxy handle_api_response |
| `sortie-start` | JSON | 出撃開始 | battle::process_battle (map/start) |
| `sortie-update` | `SortieRecordSummary` | 戦闘進行中リアルタイム | battle::process_battle |
| `sortie-complete` | `SortieRecordSummary` | 出撃完了 (帰投) | process_port |
| `quest-list-updated` | `Vec<ActiveQuestDetail>` | 任務一覧変更 | process_questlist |
| `quest-started` | `i32` (quest_id) | 任務開始 | QuestStart |
| `quest-stopped` | `i32` (quest_id) | 任務中止/完了 | QuestStop / QuestClear |
| `quest-progress-updated` | `Vec<QuestProgressSummary>` | 任務進捗変更 | battleresult / exercise |
| `senka-updated` | `SenkaSummary` | 戦果変動 | port / battleresult / ranking |
| `drive-sync-status` | `SyncStatus` | 同期状態変化 | drive_sync::engine |
| `drive-data-updated` | `()` | リモートデータ反映完了 | drive_sync::engine |

### フロントエンドのリスナー (App.tsx)

```
useEffect(() => {
  listen("proxy-ready")      → setProxyPort + checkCa
  listen("port-data")        → setPortData + weaponIconSheet ロード
  listen("fleet-updated")    → setPortData (fleets のみ更新)
  listen("sortie-complete")  → setBattleLogs (upsert)
  listen("sortie-update")    → setBattleLogs (upsert)
  listen("quest-list-updated") → setActiveQuests + get_quest_progress
  listen("quest-progress-updated") → setQuestProgress
  listen("senka-updated")    → setSenkaData + checkpoint フラッシュ
  listen("drive-sync-status") → setDriveStatus
  listen("drive-data-updated") → 全データリロード
  listen("kancolle-api")     → setApiLog (デバッグ表示)
}, []);
```

### シーケンス図: 母港表示

```
  KanColle     hudsucker       process_api         GameState        Frontend
  Server       Proxy           (async task)        (RwLock)         (React)
    |              |                |                  |                |
    |-- api_port ->|                |                  |                |
    |              |-- parse ------>|                  |                |
    |              |                |-- write lock --->|                |
    |              |                |   update ships   |                |
    |              |                |   update fleets  |                |
    |              |                |   update senka   |                |
    |              |                |   build PortSummary               |
    |              |                |   cache summary  |                |
    |              |                |-- emit("port-data", summary) --->|
    |              |                |                  |   setPortData  |
    |              |-- emit("kancolle-api") ----------------------->|
    |              |                |                  |   setApiLog    |
```

### シーケンス図: 出撃〜戦闘〜帰投

```
  Server       Proxy       process_api      GameState       Frontend
    |              |             |               |               |
    |-- map/start->|             |               |               |
    |              |-- parse --->|               |               |
    |              |             |-- lock ------>|               |
    |              |             |  on_sortie_start              |
    |              |             |  save_to_disk |               |
    |              |             |-- emit("sortie-start") ----->|
    |              |             |-- emit("sortie-update") ---->|
    |              |             |               |               |
    |-- battle --->|             |               |               |
    |              |-- parse --->|               |               |
    |              |             |-- lock ------>|               |
    |              |             |  on_battle    |               |
    |              |             |  save formation_memory        |
    |              |             |               |               |
    |-- battleresult ->|        |               |               |
    |              |-- parse --->|               |               |
    |              |             |-- lock ------>|               |
    |              |             |  on_battle_result             |
    |              |             |  update ship HP               |
    |              |             |  quest progress               |
    |              |             |  senka tracking               |
    |              |             |-- emit("port-data") -------->|  (HP更新)
    |              |             |-- emit("sortie-update") ---->|
    |              |             |-- emit("senka-updated") ---->|
    |              |             |               |               |
    |-- api_port ->|             |               |               |
    |              |-- parse --->|               |               |
    |              |             |-- lock ------>|               |
    |              |             |  on_port (finalize sortie)    |
    |              |             |  save_to_disk (final)         |
    |              |             |-- emit("sortie-complete") -->|
    |              |             |-- emit("port-data") -------->|
```

---

## 6. Tauri コマンド経由のデータ取得

### コマンド一覧 (src-tauri/src/commands.rs)

フロントエンドから `invoke()` で呼び出される Pull 型のデータ取得。

| コマンド | 戻り値 | データソース |
|---------|--------|------------|
| `get_proxy_port` | `u16` | AppState.proxy_port |
| `get_expeditions` | `Vec<ExpeditionDef>` | 静的定義データ |
| `get_sortie_quests` | `Vec<SortieQuestDef>` | 静的定義データ |
| `get_map_recommendations` | `Vec<MapRecommendationDef>` | 静的定義データ |
| `get_active_quest_ids` | `Vec<ActiveQuestDetail>` | GameState.history.active_quest_details |
| `check_expedition_cmd` | `ExpeditionCheckResult` | GameState.profile (計算) |
| `check_sortie_quest_cmd` | `SortieQuestCheckResult` | GameState.profile (計算) |
| `check_map_recommendation_cmd` | `MapRecommendationCheckResult` | GameState.profile (計算) |
| `get_battle_logs` | `{ records, total }` | BattleLogger.completed / disk |
| `get_improvement_list` | `ImprovementListResponse` | GameState + 静的データ |
| `get_ship_list` | `ShipListResponse` | GameState.profile.ships + master |
| `get_equipment_list` | `EquipListResponse` | GameState.profile.slotitems + master |
| `get_quest_progress` | `Vec<QuestProgressSummary>` | GameState.history.quest_progress |
| `update_quest_progress` | `bool` | GameState 更新 + ファイル保存 |
| `get_cached_resource` | `String` (data URI / JSON) | local/cache/ ディスク |
| `get_map_sprite` | `String` (data URI) | local/cache/ + 画像切り出し |
| `get_drive_status` | `SyncStatus` | GameState.sync_notifier + manifest |

### コマンド呼出フロー

```
Frontend (React)                  Tauri IPC                  Backend (Rust)
     |                                |                           |
     |-- invoke("get_ship_list") ---->|                           |
     |                                |-- get_ship_list() ------->|
     |                                |   state.inner.read().await|
     |                                |   build ShipListResponse  |
     |                                |<-- Ok(response) ----------|
     |<-- Promise resolve ------------|                           |
     |   setShipList(data)            |                           |
```

---

## 7. ファイル I/O と永続化

### ディレクトリ構成

```
{app_local_data_dir}/
  +-- sync/                          ... GDrive 同期対象
  |     +-- battle_logs/             ... 出撃記録 (JSON per sortie)
  |     |     +-- 20260313_143025.json
  |     +-- raw_api/                 ... 生 API ダンプ (開発用)
  |     +-- quest_progress.json      ... 任務進捗
  |     +-- improved_equipment.json  ... 改修済み装備 ID セット
  |     +-- senka_log.json           ... 戦果ログ
  |     +-- formation_memory.json    ... 陣形記憶
  |
  +-- local/                         ... ローカル専用 (同期しない)
  |     +-- cache/                   ... プロキシ経由のリソースキャッシュ
  |     |     +-- kcs2/              ... ゲームアセット
  |     +-- game-webview/            ... WebView2 ユーザーデータ (Win)
  |     +-- game_muted               ... ミュート状態
  |     +-- formation_hint_enabled   ... 陣形ヒント有効フラグ
  |     +-- taiha_alert_enabled      ... 大破警告有効フラグ
  |     +-- minimap_enabled          ... ミニマップ有効フラグ
  |     +-- minimap_position.json    ... ミニマップ位置
  |     +-- minimap_size.json        ... ミニマップサイズ
  |     +-- cookies.json             ... DMM ログインクッキー
  |
  +-- sync_manifest.json             ... GDrive 同期メタデータ
```

### 各データの永続化タイミング

| データ | ファイルパス | 書き込みタイミング | 読み込みタイミング |
|-------|------------|-----------------|-----------------|
| 出撃記録 | `sync/battle_logs/{id}.json` | 出撃開始時 + 帰投時 | アプリ起動時 / 同期後 |
| 生 API | `sync/raw_api/{timestamp}_{seq}_{ep}.json` | 各 API レスポンス受信時 (有効時のみ) | - |
| 任務進捗 | `sync/quest_progress.json` | 戦闘結果時 / 手動更新時 | アプリ起動時 / 同期後 |
| 改修履歴 | `sync/improved_equipment.json` | 改修成功時 | アプリ起動時 / 同期後 |
| 戦果ログ | `sync/senka_log.json` | 経験値変動時 / EO クリア時 | アプリ起動時 |
| 陣形記憶 | `sync/formation_memory.json` | 戦闘開始時 (陣形確定) | アプリ起動時 / 同期後 |
| クッキー | `local/cookies.json` | 画面遷移時 / アプリ終了時 | ゲーム画面起動時 |

### 永続化シーケンス (改修成功の例)

```
process_api (remodel_slot, success=true)
  |
  +-- state.history.improved_equipment.insert(eq_id)
  +-- improvement::save_improved_history(path, &set)
  |     +-- serde_json::to_string(&ids)
  |     +-- fs::write(path, json)
  +-- notify_sync(&state, vec!["improved_equipment.json"])
        +-- sync_notifier.try_send(UploadChanged(...))
```

---

## 8. Google Drive 同期

### 同期アーキテクチャ

```
                          GameStateInner
                               |
                          sync_notifier
                          (mpsc::Sender)
                               |
                               v
  +------------------------------------------------------+
  | Sync Engine (tokio background task)                   |
  |                                                       |
  |  run_sync_loop():                                     |
  |    +-- UploadChanged(paths) → 差分アップロード        |
  |    +-- FullSync → 全ファイル双方向同期                |
  |    +-- 5分間隔ポーリング → リモート変更検出           |
  |    +-- Shutdown → タスク終了                          |
  |                                                       |
  |  変更検出後:                                          |
  |    +-- reload_game_state(app)                         |
  |    |     +-- quest_progress リロード                  |
  |    |     +-- improved_equipment リロード              |
  |    |     +-- battle_logger.reload_from_disk()          |
  |    +-- app.emit("drive-data-updated")                 |
  +------------------------------------------------------+
                               |
                               v
                        Google Drive API
                   "KanColle Browser Sync" フォルダ
```

### 同期対象ファイル (SYNC_TARGETS)

| パス | 種別 | 説明 |
|-----|------|------|
| `quest_progress.json` | ファイル | 任務進捗 |
| `improved_equipment.json` | ファイル | 改修履歴 |
| `battle_logs/` | ディレクトリ | 出撃記録群 |
| `raw_api/` | ディレクトリ | 生 API ダンプ |
| `senka_log.json` | ファイル | 戦果ログ |
| `formation_memory.json` | ファイル | 陣形記憶 |

### 同期後のデータ反映フロー

```
Sync Engine         GameState          Frontend
    |                   |                  |
    |-- download ------>|                  |
    |   (remote files)  |                  |
    |                   |                  |
    |-- reload_game_state()                |
    |   quest_progress  |                  |
    |   improved_equip  |                  |
    |   battle_logs     |                  |
    |                   |                  |
    |-- emit("drive-data-updated") ------->|
    |                   |                  |
    |                   |    invoke("get_quest_progress")
    |                   |<--------------------|
    |                   |-- response -------->|
    |                   |                  |
    |                   |    refreshBattleLogs()
    |                   |<--------------------|
    |                   |-- response -------->|
```

---

## 付録: 主要ソースファイルマップ

| ファイル | 役割 |
|---------|------|
| `src-tauri/src/lib.rs` | アプリ初期化、プロキシ起動、GameState 管理 |
| `src-tauri/src/proxy/mod.rs` | hudsucker MITM プロキシ、API 傍受 |
| `src-tauri/src/api/mod.rs` | process_api() メインディスパッチ |
| `src-tauri/src/api/battle.rs` | 戦闘 API サブディスパッチ |
| `src-tauri/src/api/models.rs` | GameState / マスターデータ / DTO 定義 |
| `src-tauri/src/api/fleet.rs` | 編成変更処理 |
| `src-tauri/src/api/ship.rs` | 艦船データ処理 |
| `src-tauri/src/battle_log/mod.rs` | BattleLogger、SortieRecord 定義 |
| `src-tauri/src/battle_log/storage.rs` | 出撃記録のディスク I/O |
| `src-tauri/src/battle_log/parser.rs` | 戦闘データパーサー |
| `src-tauri/src/commands.rs` | Tauri コマンド (フロントエンド API) |
| `src-tauri/src/improvement/mod.rs` | 改修データ・永続化 |
| `src-tauri/src/quest_progress/mod.rs` | 任務進捗追跡・永続化 |
| `src-tauri/src/senka.rs` | 戦果トラッカー |
| `src-tauri/src/drive_sync/` | Google Drive 同期エンジン |
| `src-tauri/src/game_window.rs` | ゲーム画面 WebView 管理 |
| `src/App.tsx` | フロントエンド: イベントリスナー + Tauri コマンド呼出 |
