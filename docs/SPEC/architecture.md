<!-- AUTO-GENERATED from source code -->
# 基本設計書 (Architecture)

## 1. システム概要

KanColle Browser は、ブラウザゲーム「艦隊これくしょん」専用のクロスプラットフォームデスクトップクライアントである。
ゲームの HTTP API をリアルタイムで傍受・解析し、艦隊編成、出撃ログ、任務進捗、戦果ランキングなどの情報を
統合的に表示するツールを提供する。

```
┌─────────────────────────────────────────────────────────┐
│                  KanColle Browser v0.3.0                │
│                                                         │
│  ┌──────────────┐   IPC (invoke/emit)   ┌────────────┐ │
│  │  Rust Backend │◄────────────────────►│  React UI  │ │
│  │  (Tauri v2)   │                      │ (TypeScript)│ │
│  └──────┬───────┘                       └────────────┘ │
│         │                                               │
│  ┌──────▼───────┐                                       │
│  │ HTTP Proxy   │ MITM (hudsucker)                      │
│  │ (localhost)  │◄──── WebView (WKWebView/WebView2)     │
│  └──────┬───────┘                                       │
│         │                                               │
│         ▼                                               │
│  KanColle Game Server (*.kancolle-server.com)           │
└─────────────────────────────────────────────────────────┘
```

## 2. 技術スタック

### 2.1 コアフレームワーク

| 技術              | バージョン | 用途                           |
|-------------------|-----------|-------------------------------|
| Tauri             | 2.x      | デスクトップアプリフレームワーク    |
| Rust              | 2021 ed. | バックエンド (stable toolchain) |
| React             | 19.1.x   | フロントエンド UI               |
| TypeScript        | 5.8.x    | フロントエンド型安全             |
| Vite              | 7.0.x    | ビルドツール / 開発サーバー       |

### 2.2 Rust 依存クレート

| クレート           | バージョン | 用途                                    |
|-------------------|-----------|----------------------------------------|
| tauri             | 2        | アプリケーションコア (tray-icon, macos-proxy, macos-private-api) |
| hudsucker         | 0.24     | MITM HTTP/HTTPS プロキシ (rcgen-ca, rustls-client)             |
| tokio             | 1        | 非同期ランタイム (full features)                                |
| serde / serde_json| 1        | JSON シリアライズ/デシリアライズ                                 |
| google-drive3     | 7.0      | Google Drive API v3 クライアント                                |
| yup-oauth2        | 12       | OAuth 2.0 認証フロー                                           |
| hyper / hyper-util| 1 / 0.1  | HTTP クライアント/サーバー                                      |
| hyper-rustls      | 0.27     | HTTPS コネクタ (native-tokio)                                  |
| rustls            | 0.23     | TLS 実装                                                       |
| chrono            | 0.4      | 日時処理 (serde 対応)                                          |
| flate2            | 1        | gzip 圧縮/解凍                                                 |
| brotli            | 8        | Brotli 圧縮解凍                                                |
| image             | 0.25     | 画像処理 (PNG)                                                 |
| base64            | 0.22     | Base64 エンコード/デコード                                      |
| md-5              | 0.10     | MD5 ハッシュ (同期チェック用)                                    |
| dirs              | 6        | OS標準ディレクトリパス取得                                       |
| url               | 2        | URL パース                                                     |
| open              | 5        | システムブラウザ起動 (OAuth用)                                   |
| serde_urlencoded  | 0.7.1    | URL-encoded フォームデータパース                                 |

### 2.3 プラットフォーム固有依存

| クレート           | プラットフォーム | バージョン | 用途                        |
|-------------------|----------------|-----------|----------------------------|
| objc2             | macOS          | 0.6       | Objective-C ランタイムブリッジ |
| objc2-foundation  | macOS          | 0.3       | Foundation フレームワーク     |
| webview2-com      | Windows        | 0.38      | WebView2 COM API バインディング |
| windows-core      | Windows        | 0.61      | Windows API コアバインディング  |

### 2.4 フロントエンド依存

| パッケージ             | バージョン | 用途                  |
|-----------------------|-----------|----------------------|
| react                 | ^19.1.0  | UIライブラリ           |
| react-dom             | ^19.1.0  | DOM レンダリング       |
| @tauri-apps/api       | ^2       | Tauri IPC クライアント |
| @tauri-apps/plugin-opener | ^2   | 外部URL/ファイル操作   |
| @vitejs/plugin-react  | ^4.6.0   | Vite React プラグイン  |
| @tauri-apps/cli       | ^2       | Tauri CLI ツール       |


## 3. モジュール構成

### 3.1 Rust バックエンド (src-tauri/src/)

```
src-tauri/src/
│
├── main.rs                   エントリポイント (kancolle_browser_lib::run() 呼出)
├── lib.rs                    Tauri アプリ初期化、状態管理、プロキシ起動、GDrive自動復元
│
├── api/                      ──── API 傍受・解析エンジン ────
│   ├── mod.rs                process_api(): メインAPIディスパッチャ
│   ├── models.rs             GameState, MasterData, UserProfile, SortieState, UserHistory
│   ├── battle.rs             戦闘API (出撃/夜戦/結果) ハンドラ
│   ├── ship.rs               艦船データ更新 (装備変更, slot_deprive)
│   ├── fleet.rs              編成変更 (hensei/change, preset_select)
│   ├── formation.rs          陣形ヒントオーバーレイ制御
│   ├── minimap.rs            ミニマップデータ送信
│   ├── dto/                  データ転送オブジェクト
│   │   ├── battle.rs         戦闘/任務/改修レスポンス構造体
│   │   └── request.rs        編成/改修/任務リクエスト構造体
│   └── tests.rs              APIハンドラテスト
│
├── proxy/
│   └── mod.rs                HTTPSプロキシサーバー (hudsucker ベース)
│                             MITM傍受、リソースキャッシュ、CA証明書管理
│
├── commands.rs               Tauri コマンドハンドラ (フロントエンドAPI)
├── game_window.rs            ゲームウィンドウ管理 (Multi-WebView, ズーム, ミュート)
├── overlay.rs                オーバーレイUI (ミニマップ, 陣形ヒント, 大破警告, 遠征通知)
├── cookie.rs                 Cookie 永続化 (DMM ログイン維持)
├── ca.rs                     CA証明書インストール (macOS Keychain / Windows certutil)
├── migration.rs              データディレクトリマイグレーション (flat → sync/local)
│
├── battle_log/               ──── 出撃ログ記録 ────
│   ├── mod.rs                BattleLogger: 出撃追跡、結果処理
│   ├── parser.rs             戦闘データ解析 (ダメージ, 陣形, ドロップ)
│   └── storage.rs            ログファイル I/O
│
├── expedition/
│   └── mod.rs                遠征定義 & 大成功判定チェッカー
│
├── sortie_quest/
│   └── mod.rs                出撃任務定義、マップ推奨ルート
│
├── quest_progress/
│   └── mod.rs                任務進捗追跡、リセットロジック
│
├── senka/
│   └── mod.rs                戦果 (ランキングポイント) 計算・追跡
│
├── improvement/
│   └── mod.rs                装備改修リスト管理
│
└── drive_sync/               ──── Google Drive 同期 ────
    ├── mod.rs                SyncManifest, SyncTarget 定義
    ├── auth.rs               OAuth 2.0 認証フロー (InstalledFlow)
    ├── engine.rs             同期エンジン (tokio task + mpsc チャネル)
    └── files.rs              GDrive ファイル操作 (アップロード/ダウンロード)
```

### 3.2 フロントエンド (src/)

```
src/
├── main.tsx                  React エントリポイント
├── App.tsx                   ルートコンポーネント (タブ制御, イベントリスナー)
├── App.css                   ルートレイアウトスタイル
├── constants.ts              localStorage キー定数
│
├── types/                    ──── 型定義 (10ファイル) ────
│   ├── index.ts              エクスポート集約
│   ├── port.ts               PortData, FleetData 等
│   ├── battle.ts             SortieRecord, BattleNode 等
│   ├── quest.ts              SortieQuestDef, QuestProgressSummary 等
│   ├── expedition.ts         ExpeditionDef, ExpeditionCheckResult 等
│   ├── improvement.ts        ImprovementEntry 等
│   ├── ship.ts               ShipListItem 等
│   ├── equipment.ts          EquipListItem 等
│   ├── senka.ts              SenkaSummary 等
│   └── common.ts             TabId, DriveStatus 等
│
├── utils/                    ──── ユーティリティ (4ファイル) ────
│   ├── index.ts              エクスポート集約
│   ├── format.ts             時間/数値フォーマット
│   ├── color.ts              HP/コンディションカラー
│   └── map.ts                マップアセット処理
│
└── components/               ──── 機能別コンポーネント ────
    ├── common/               共通UI: HpBar, BattleHpBar, ClearButton,
    │                         DateRangePicker, ListTable
    ├── homeport/             母港タブ: FleetPanel, ExpeditionChecker,
    │                         SortieQuestChecker, MapRecommendationChecker,
    │                         QuestProgressDisplay
    ├── battle/               戦闘ログタブ: BattleTab, BattleDetailView,
    │                         BattleNodeDetail, MapRouteView
    ├── ships/                艦船一覧タブ: ShipListTab
    ├── equips/               装備一覧タブ: EquipListTab
    ├── improvement/          改修タブ: ImprovementTab
    └── settings/             設定タブ: SettingsTab (GDrive同期, キャッシュ管理)
```


## 4. 通信フロー

### 4.1 API 傍受フロー (メインデータパイプライン)

```
┌──────────────┐     HTTPS      ┌───────────────────┐     HTTPS     ┌──────────────┐
│  DMM Game    │ ◄─────────────►│  MITM Proxy       │ ◄────────────►│  KanColle    │
│  (WebView)   │  via proxy     │  (hudsucker)      │  forwarded   │  Game Server │
│              │  127.0.0.1:    │  127.0.0.1:19080  │              │  *.kancolle-  │
│              │  {proxy_port}  │  (macOS default)  │              │  server.com  │
└──────────────┘                └────────┬──────────┘              └──────────────┘
                                         │
                              ┌──────────▼──────────┐
                              │  should_intercept() │
                              │  *.kancolle-server   │
                              │  .com のみ MITM     │
                              │  DMM/CDN は素通し   │
                              └──────────┬──────────┘
                                         │
                    ┌────────────────────┼────────────────────┐
                    │ /kcsapi/*          │                    │ /kcs2/*
                    ▼                    │                    ▼
          ┌─────────────────┐            │         ┌──────────────────┐
          │ handle_request()│            │         │ maybe_cache_     │
          │ リクエストボディ  │            │         │ resource()       │
          │ キャプチャ       │            │         │ ローカルキャッシュ  │
          └────────┬────────┘            │         │ に保存            │
                   │                     │         └──────────────────┘
                   ▼                     │
          ┌─────────────────┐            │
          │handle_response()│            │
          │ gzip/brotli解凍  │            │
          │ "svdata=" 除去   │            │
          └────────┬────────┘            │
                   │                     │
                   ▼                     │
          ┌─────────────────┐            │
          │ process_api()   │            │
          │ エンドポイント別   │            │
          │ ディスパッチ     │            │
          └────────┬────────┘            │
                   │                     │
                   ▼                     │
          ┌─────────────────┐            │
          │  GameState      │            │
          │  (Arc<RwLock>)  │            │
          │  状態更新        │            │
          └────────┬────────┘            │
                   │ emit()              │
                   ▼                     │
          ┌─────────────────┐            │
          │ React Frontend  │◄───────────┘
          │ listen() で受信  │  kancolle-api イベント (生データ)
          │ 状態反映→再描画   │  port-data, senka-updated 等 (加工済み)
          └─────────────────┘
```

### 4.2 フロントエンド→バックエンド通信 (Tauri IPC)

```
React Component                    Rust Command Handler
─────────────────                  ─────────────────────
invoke("open_game_window")    ───► game_window::open_game_window()
invoke("get_expeditions")     ───► commands::get_expeditions()
invoke("check_expedition_cmd")───► commands::check_expedition_cmd()
invoke("get_battle_logs")     ───► commands::get_battle_logs()
invoke("get_ship_list")       ───► commands::get_ship_list()
invoke("drive_login")         ───► commands::drive_login()
invoke("drive_force_sync")    ───► commands::drive_force_sync()
invoke("set_game_zoom")       ───► game_window::set_game_zoom()
invoke("toggle_game_mute")    ───► game_window::toggle_game_mute()
invoke("set_overlay_visible") ───► overlay::set_overlay_visible()
...
```

### 4.3 バックエンド→フロントエンド通知 (Tauri Events)

```
Rust (emit)                        React (listen)
───────────                        ──────────────
"proxy-ready"                 ───► プロキシポート番号取得
"port-data"                   ───► 母港データ全体更新 (艦隊/資源/入渠)
"kancolle-api"                ───► 生APIデータ (デバッグログ用)
"master-data-loaded"          ───► マスターデータ読込完了通知
"quest-list-updated"          ───► 受注任務一覧更新
"quest-started" / "stopped"   ───► 個別任務の開始/停止
"senka-updated"               ───► 戦果データ更新
"sortie-complete"             ───► 出撃完了 (戦闘ログ追加)
"battle-state"                ───► 戦闘中の状態更新
"drive-status-updated"        ───► GDrive 同期ステータス変更
"drive-data-updated"          ───► GDrive 同期完了 (データリロード)
```


## 5. 状態管理

### 5.1 Rust 側 (GameState)

```
AppState (Tauri Managed State)
├── proxy_port: Mutex<u16>
├── game_muted: AtomicBool
├── formation_hint_enabled: AtomicBool
├── taiha_alert_enabled: AtomicBool
├── minimap_enabled: AtomicBool
├── expedition_notify_visible: AtomicBool
├── formation_hint_rect: Mutex<FormationHintRect>
├── game_zoom: Mutex<f64>
├── minimap_position: Mutex<Option<(f64, f64)>>
└── minimap_size: Mutex<(f64, f64)>

GameState (Tauri Managed State)
└── inner: Arc<RwLock<GameStateInner>>
    ├── master: MasterData                    ← api_start2 マスターデータ (不変)
    │   ├── ships:      HashMap<i32, MasterShipInfo>
    │   ├── stypes:     HashMap<i32, String>
    │   ├── missions:   HashMap<i32, MissionInfo>
    │   ├── slotitems:  HashMap<i32, MasterSlotItemInfo>
    │   └── equip_types:HashMap<i32, String>
    │
    ├── profile: UserProfile                  ← api_port で更新
    │   ├── ships:      HashMap<i32, ShipInfo>
    │   ├── slotitems:  HashMap<i32, PlayerSlotItem>
    │   ├── fleets:     Vec<Vec<i32>>
    │   └── combined_flag: i32
    │
    ├── sortie: SortieState                   ← 出撃中の状態
    │   ├── battle_logger: BattleLogger
    │   └── last_port_summary: Option<PortSummary>
    │
    ├── history: UserHistory                  ← 蓄積データ
    │   ├── active_quests: HashSet<i32>
    │   ├── active_quest_details: HashMap<i32, ActiveQuestDetail>
    │   ├── sortie_quest_defs: Vec<SortieQuestDef>
    │   ├── improved_equipment: HashSet<i32>
    │   └── quest_progress: QuestProgressState
    │
    ├── senka: SenkaTracker                   ← 戦果追跡
    ├── formation_memory: HashMap<String, i32>← 陣形記憶
    ├── sync_notifier: Option<mpsc::Sender>   ← GDrive 同期チャネル
    └── data_dir / *_path                     ← ファイルパス
```

### 5.2 フロントエンド側 (React State)

App.tsx がルートコンポーネントとして全状態を管理し、各タブコンポーネントに props で配布する。

```
App.tsx (useState)
├── proxyPort          ← proxy-ready イベント
├── portData           ← port-data イベント (母港全データ)
├── senkaData          ← senka-updated イベント
├── activeQuests       ← quest-list-updated イベント
├── questProgress      ← port-data 受信時に invoke で取得
├── battleLogs         ← invoke("get_battle_logs") で取得
├── apiLog             ← kancolle-api イベント (デバッグ用)
├── gameOpen           ← ウィンドウ開閉状態
├── caInstalled        ← CA証明書インストール状態
├── driveStatus        ← drive-status-updated イベント
├── activeTab          ← 現在のタブ選択
├── uiZoom             ← UI ズーム倍率 (localStorage 永続化)
└── weaponIconSheet    ← 装備アイコンスプライトシート
```


## 6. データ永続化

### 6.1 ディレクトリ構造

アプリデータは OS 標準のローカルデータディレクトリに保存される。
- macOS: `~/Library/Application Support/com.eo.kancolle-browser/`
- Windows: `%LOCALAPPDATA%/com.eo.kancolle-browser/`

```
{app_local_data_dir}/
├── sync/                          ──── GDrive 同期対象 ────
│   ├── quest_progress.json        任務進捗データ
│   ├── improved_equipment.json    改修済み装備ID一覧
│   ├── senka_log.json             戦果ログ
│   ├── formation_memory.json      陣形記憶
│   ├── battle_logs/               出撃記録 (JSON/戦闘ごと)
│   │   └── {id}.json
│   └── raw_api/                   生APIダンプ (デバッグ用)
│       └── {seq}_{endpoint}.json
│
├── local/                         ──── ローカル専用 ────
│   ├── dmm_cookies.json           DMM ログインCookie
│   ├── game_muted                 ミュート状態 ("0"/"1")
│   ├── formation_hint_enabled     陣形ヒント ON/OFF
│   ├── taiha_alert_enabled        大破警告 ON/OFF
│   ├── minimap_enabled            ミニマップ ON/OFF
│   ├── minimap_position.json      ミニマップ位置 [x, y]
│   ├── minimap_size.json          ミニマップサイズ [w, h]
│   ├── cache/                     ゲームリソースキャッシュ
│   │   └── kcs2/...               画像/JSON/JS/CSS
│   └── game-webview/              WebView2 プロファイル (Windows)
│
├── google_drive_token.json        OAuth2 トークン
├── sync_manifest.json             GDrive 同期メタデータ
│
└── {CA cert dir}/                 CA 証明書 (別パス: data_local_dir)
    ├── ca.cert.pem
    └── ca.key.pem
```

### 6.2 Google Drive 同期

```
┌─────────────┐     mpsc channel     ┌──────────────────┐     HTTPS      ┌──────────┐
│ GameState   │ ── SyncCommand ─────►│ Sync Engine      │ ◄────────────►│ Google   │
│ (API処理後) │    UploadChanged     │ (tokio task)     │  Drive API v3 │ Drive    │
│             │                      │                  │               │          │
│             │◄─ drive-data-updated─│ FullSync 完了時   │               │          │
│             │   (emit → React)     │ reload_game_state│               │          │
└─────────────┘                      └──────────────────┘               └──────────┘

同期対象 (SYNC_TARGETS):
  - quest_progress.json      (ファイル)
  - improved_equipment.json  (ファイル)
  - senka_log.json           (ファイル)
  - formation_memory.json    (ファイル)
  - battle_logs/             (ディレクトリ)
  - raw_api/                 (ディレクトリ)

同期方式:
  - UploadChanged: API処理後にファイル変更があれば即座にアップロード
  - FullSync: 5分間隔ポーリング + 手動トリガー (download + upload)
  - 競合解決: MD5ハッシュ + 更新日時ベースの last-write-wins
```


## 7. プラットフォーム別アーキテクチャ

### 7.1 macOS

```
┌───────────────────────────────────────────────┐
│  Tauri App                                    │
│  ┌─────────────────────────────┐              │
│  │  WKWebView (game-content)   │              │
│  │  proxy_url → 127.0.0.1:19080│              │
│  │  data_store_identifier      │              │
│  │  (WKWebsiteDataStore)       │              │
│  └──────────────┬──────────────┘              │
│                 │                              │
│  ┌──────────────▼──────────────┐              │
│  │  MITM Proxy (hudsucker)     │              │
│  │  port 19080 (固定優先)       │              │
│  │  CA: Keychain 登録           │              │
│  └─────────────────────────────┘              │
│                                               │
│  Tauri features: macos-proxy, macos-private-api│
│  objc2: WKWebView ミュート制御                  │
│  (_setPageMuted via Objective-C runtime)       │
└───────────────────────────────────────────────┘
```

- **プロキシ設定**: Tauri の `macos-proxy` feature でネイティブにプロキシ URL を WebView に設定
- **Cookie永続化**: `data_store_identifier` による WKWebsiteDataStore (macOS >= 14)
- **CA証明書**: `security import` + `security add-trusted-cert` でログインキーチェーンに登録
- **ミュート**: `objc2` 経由で `_setPageMuted:` Objective-C メッセージ送信

### 7.2 Windows

```
┌───────────────────────────────────────────────┐
│  Tauri App                                    │
│  ┌─────────────────────────────┐              │
│  │  WebView2 (game-content)    │              │
│  │  proxy_url → 127.0.0.1:    │              │
│  │  {dynamic_port}             │              │
│  │  data_directory             │              │
│  │  (file-based profile)       │              │
│  └──────────────┬──────────────┘              │
│                 │                              │
│  ┌──────────────▼──────────────┐              │
│  │  MITM Proxy (hudsucker)     │              │
│  │  port 19080 or fallback     │              │
│  │  CA: certutil -addstore Root│              │
│  └─────────────────────────────┘              │
│                                               │
│  webview2-com: ICoreWebView2_8 ミュート制御     │
│  (SetIsMuted via COM interface)               │
│  windows-core: COM バインディング               │
└───────────────────────────────────────────────┘
```

- **プロキシ設定**: Tauri の `proxy_url` でWebView2にプロキシ設定
- **Cookie永続化**: `data_directory` によるファイルベースの WebView2 プロファイル + Cookie JS復元
- **CA証明書**: PowerShell `Start-Process -Verb RunAs` で UAC 昇格後 `certutil -addstore Root` 実行
- **ミュート**: `webview2-com` 経由で `ICoreWebView2_8::SetIsMuted` COM API 呼出

> **注意**: Windows WebView2 + proxy 構成で wry 0.54.2 にデッドロック問題あり。
> `with_webview()` を同期コマンド内で使用するとデッドロックする。setup/async で使用すること。


## 8. ウィンドウ構成

```
┌─────────────────────────────┐
│  Main Window ("main")       │  1400x900 (min: 800x600)
│  React SPA (管理UI)         │
│  ├ 母港タブ                  │
│  ├ 戦闘ログタブ              │
│  ├ 艦船一覧タブ              │
│  ├ 装備一覧タブ              │
│  ├ 改修タブ                  │
│  └ 設定タブ                  │
└─────────────────────────────┘

┌─────────────────────────────┐
│  Game Window ("game")       │  1200x748 (1200x720 + 28px control bar)
│  Multi-WebView 構成:        │
│  ├ game-content             │  ゲーム本体 (DMM → 艦これ)
│  └ game-overlay             │  透過オーバーレイ (ミニマップ, 大破警告)
└─────────────────────────────┘

┌──────────────────┐  ┌──────────────────┐
│ Formation Hint   │  │ Expedition Notify │
│ ("formation-     │  │ ("expedition-     │
│  hint")          │  │  notify")         │
│ 200x170          │  │ 250x100           │
│ 透過, always-on- │  │ 透過, always-on-  │
│ top, click-      │  │ top, click-       │
│ through          │  │ through           │
└──────────────────┘  └──────────────────┘
```


## 9. ビルド・リリース構成

### 9.1 開発

```bash
npm run tauri dev
# → Vite dev server (localhost:1420) + Rust バックエンドのホットリロード
```

### 9.2 ビルド

```bash
npm run tauri build
# → tsc && vite build (フロントエンド) → cargo build (Rust) → バンドル
```

### 9.3 CI/CD (GitHub Actions)

```
リリースワークフロー (.github/workflows/release.yml):
  トリガー: v* タグ push
  マトリクス:
    ├── macOS: universal-apple-darwin (aarch64 + x86_64)
    └── Windows: x86_64-pc-windows-msvc
  ステップ:
    1. Node.js 22 セットアップ
    2. Rust stable toolchain セットアップ
    3. npm install
    4. tauri-action v0 → ビルド + GitHub Release (Draft)
  バージョニング: tagName: v__VERSION__ (Cargo.toml のバージョンを使用)
```

### 9.4 アプリ識別子

| 項目       | 値                           |
|-----------|------------------------------|
| identifier| com.eo.kancolle-browser      |
| version   | 0.3.0 (Cargo.toml, tauri.conf.json, package.json で統一) |
| license   | MIT                          |


## 10. セキュリティ考慮

- **MITM 対象の限定**: `should_intercept()` で `*.kancolle-server.com` と IP アドレスのみを MITM 対象とし、DMM ログイン/CDN 通信は素通しさせる
- **CA 証明書**: ユーザー操作による明示的なインストール (macOS: パスワードダイアログ, Windows: UAC 昇格)
- **CSP**: `null` 設定 (ゲーム互換性のため無効化)
- **OAuth トークン**: ローカルファイルに永続化、アプリスコープ `drive.file` (自アプリ作成ファイルのみ)
- **Cookie**: 終了時に自動保存、起動時に JS 経由で復元
