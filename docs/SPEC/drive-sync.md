<!-- AUTO-GENERATED from source code -->

# Google Drive 同期（Drive Sync）詳細設計

## 概要

Google Drive を介した複数端末間のデータ同期機能。OAuth 2.0 による認証、tokio バックグラウンドタスクによる非同期同期エンジン、タイムスタンプベースの競合解決を実装。

---

## 1. アーキテクチャ概要

```
┌─────────────────────────────────────────────────────────┐
│ Frontend (SettingsTab)                                   │
│   drive_login → drive_logout → drive_force_sync          │
│   ← drive-sync-status (event)                            │
│   ← drive-data-updated (event)                           │
└─────────┬───────────────────────────────────────────────┘
          │ Tauri IPC
┌─────────▼───────────────────────────────────────────────┐
│ Commands (commands.rs)                                   │
│   drive_login / drive_logout / get_drive_status /        │
│   drive_force_sync                                       │
└─────────┬───────────────────────────────────────────────┘
          │ mpsc::Sender<SyncCommand>
┌─────────▼───────────────────────────────────────────────┐
│ Sync Engine (engine.rs) — tokio::spawn バックグラウンド  │
│   run_sync_loop:                                         │
│     - 初期 full_sync                                     │
│     - コマンド受信 (UploadChanged / FullSync / Shutdown) │
│     - 5分間隔ポーリング                                   │
└─────────┬───────────────────────────────────────────────┘
          │ Google Drive API
┌─────────▼───────────────────────────────────────────────┐
│ Files (files.rs) — DriveHub ラッパー                     │
│   ensure_sync_folder / ensure_subfolder /                │
│   upload_file / download_file / list_files / delete_file │
└─────────────────────────────────────────────────────────┘
```

---

## 2. OAuth2 認証: auth.rs

### 認証方式

- **InstalledFlowAuthenticator** (yup_oauth2) を使用
- ブラウザを自動で開いてユーザー同意を取得（`open::that(url)`）
- HTTPリダイレクト (`http://localhost`) でトークンを受け取り

### スコープ

```
https://www.googleapis.com/auth/drive.file
```

- `drive.file` スコープ: アプリが作成したファイルのみアクセス可能（全ドライブアクセスではない）

### クレデンシャル

```rust
const GOOGLE_CLIENT_ID: &str = "1018502336976-...";
const GOOGLE_CLIENT_SECRET: &str = "GOCSPX-...";
```

- Desktop app 向けの OAuth クレデンシャル（Google のドキュメントで非機密扱い）
- `client_credentials()` で取得。空の場合は `None` を返し同期不可

### トークン永続化

- ファイル: `{data_dir}/google_drive_token.json`
- `yup_oauth2::InstalledFlowAuthenticator` の `persist_tokens_to_disk()` で自動管理
- トークンのリフレッシュも自動

### 公開API

```rust
/// ブラウザを開いてOAuth認証を実施（初回 or トークン期限切れ時）
pub async fn authenticate(
    client_id: &str,
    client_secret: &str,
    data_dir: &Path,
) -> Result<DriveAuthenticator, String>

/// キャッシュ済みトークンからの復元（起動時の自動復元用、ブラウザ不要）
pub async fn try_restore_auth(
    client_id: &str,
    client_secret: &str,
    data_dir: &Path,
) -> Option<DriveAuthenticator>

/// ログアウト（トークンファイル削除）
pub fn logout(data_dir: &Path)

/// トークンファイルの存在チェック
pub fn has_token(data_dir: &Path) -> bool
```

### 起動時の自動復元（lib.rs）

```rust
tauri::async_runtime::spawn(async move {
    if let Some((client_id, client_secret)) = drive_sync::auth::client_credentials() {
        if let Some(auth) = drive_sync::auth::try_restore_auth(...).await {
            let sync_tx = drive_sync::engine::start_sync_engine(...).await;
            inner.sync_notifier = Some(sync_tx);
        }
    }
});
```

- アプリ起動時に非同期タスクとして実行
- キャッシュトークンがあれば自動的に同期エンジンを開始

---

## 3. 同期エンジン: engine.rs

### チャネル構成

```rust
pub enum SyncCommand {
    UploadChanged(Vec<String>),  // 指定ファイルをアップロード
    FullSync,                     // 完全同期（ダウンロード + アップロード）
    Shutdown,                     // エンジン停止
}
```

- `mpsc::channel::<SyncCommand>(64)` — バッファサイズ64
- `Sender` は `GameStateInner.sync_notifier` に格納

### start_sync_engine()

```rust
pub async fn start_sync_engine(
    app: AppHandle,
    data_dir: PathBuf,
    auth: DriveAuthenticator,
) -> mpsc::Sender<SyncCommand>
```

1. mpsc チャネルを作成
2. `files::build_hub(auth)` で DriveHub を構築
3. `tokio::spawn` でバックグラウンドタスクを起動
4. `Sender` を返却

### run_sync_loop() メインループ

```rust
loop {
    tokio::select! {
        cmd = rx.recv() => {
            // コマンド処理
            UploadChanged(paths) → 各ファイルを個別アップロード
            FullSync → full_sync() 実行
            Shutdown | None → break
        }
        _ = interval.tick() => {
            // 5分間隔ポーリング → full_sync()
        }
    }
}
```

### ポーリング間隔

```rust
const POLL_INTERVAL: Duration = Duration::from_secs(300);  // 5分
```

### ステータス通知

```rust
fn emit_status(app, syncing, last_sync, error) {
    app.emit("drive-sync-status", SyncStatus { ... });
}
```

```rust
pub struct SyncStatus {
    pub authenticated: bool,
    pub email: Option<String>,
    pub syncing: bool,
    pub last_sync: Option<String>,
    pub error: Option<String>,
}
```

---

## 4. 同期ターゲット: mod.rs

### SyncTarget 定義

```rust
pub struct SyncTarget {
    pub relative: &'static str,  // sync/ 内の相対パス
    pub is_dir: bool,            // ディレクトリか単一ファイルか
}
```

### 同期対象一覧

```rust
pub const SYNC_TARGETS: &[SyncTarget] = &[
    SyncTarget { relative: "quest_progress.json",       is_dir: false },
    SyncTarget { relative: "improved_equipment.json",    is_dir: false },
    SyncTarget { relative: "battle_logs",                is_dir: true  },
    SyncTarget { relative: "raw_api",                    is_dir: true  },
    SyncTarget { relative: "senka_log.json",             is_dir: false },
    SyncTarget { relative: "formation_memory.json",      is_dir: false },
];
```

---

## 5. SyncManifest（同期メタデータ）

### 構造

```rust
pub struct SyncManifest {
    pub files: HashMap<String, SyncFileEntry>,  // 相対パス → ファイルメタデータ
    pub drive_folder_id: Option<String>,        // ルートフォルダID
    pub subfolder_ids: HashMap<String, String>, // "battle_logs" → Drive folder ID
    pub last_full_sync: Option<DateTime<Utc>>,  // 最後の完全同期時刻
}

pub struct SyncFileEntry {
    pub drive_file_id: String,              // Drive ファイルID
    pub remote_modified: DateTime<Utc>,     // Drive 上の更新時刻
    pub local_modified: DateTime<Utc>,      // ローカルの更新時刻（前回同期時）
    pub content_hash: String,               // MD5ハッシュ
}
```

### 永続化

- ファイル: `{data_dir}/sync_manifest.json`
- `load_manifest()` / `save_manifest()` で読み書き

---

## 6. Google Drive フォルダ構造

```
Google Drive/
└── KanColle Browser Sync/        ← ルートフォルダ (drive_folder_id)
    ├── quest_progress.json
    ├── improved_equipment.json
    ├── senka_log.json
    ├── formation_memory.json
    ├── battle_logs/              ← サブフォルダ (subfolder_ids["battle_logs"])
    │   ├── 2026-03-01.json
    │   └── ...
    └── raw_api/                  ← サブフォルダ (subfolder_ids["raw_api"])
        ├── api_port_port_1234567890.json
        └── ...
```

---

## 7. ファイル操作: files.rs

### Hub 型

```rust
type Connector = hyper_rustls::HttpsConnector<...>;
pub type Hub = DriveHub<Connector>;

pub fn build_hub(auth: DriveAuthenticator) -> Hub {
    // hyper_util + hyper_rustls でHTTPSクライアント構築
    DriveHub::new(client, auth)
}
```

### フォルダ操作

```rust
/// ルートフォルダ "KanColle Browser Sync" を検索 or 作成
pub async fn ensure_sync_folder(hub: &Hub) -> Result<String, String>

/// サブフォルダを検索 or 作成
pub async fn ensure_subfolder(hub: &Hub, parent_id: &str, name: &str) -> Result<String, String>
```

### ファイル操作

```rust
/// ファイルをアップロード（新規作成 or 既存更新）
pub async fn upload_file(
    hub: &Hub,
    parent_id: &str,
    file_name: &str,
    local_path: &Path,
    existing_file_id: Option<&str>,  // Some → update, None → create
) -> Result<(String, DateTime<Utc>), String>

/// ファイルをダウンロード（alt=media でレスポンスボディから取得）
pub async fn download_file(
    hub: &Hub,
    file_id: &str,
    local_path: &Path,
) -> Result<(), String>

/// フォルダ内のファイル一覧を取得（ページネーション対応、1000件/ページ）
pub async fn list_files(
    hub: &Hub,
    folder_id: &str,
) -> Result<Vec<RemoteFile>, String>

/// ファイル削除
pub async fn delete_file(hub: &Hub, file_id: &str) -> Result<(), String>
```

### RemoteFile

```rust
pub struct RemoteFile {
    pub id: String,
    pub name: String,
    pub modified_time: DateTime<Utc>,
    pub md5: Option<String>,
}
```

---

## 8. 同期アルゴリズム

### full_sync() の処理フロー

```
1. 各 SyncTarget を順番に処理
2. ファイルターゲット → sync_single_file()
   ディレクトリターゲット → sync_directory()
3. ダウンロードがあった場合:
   a. reload_game_state() でメモリ上のデータをリロード
   b. "drive-data-updated" イベントを発火
```

### sync_single_file() の競合解決

```
            ┌─────────────────┬───────────────────┐
            │  リモートあり    │  リモートなし      │
┌───────────┼─────────────────┼───────────────────┤
│ ローカル  │ 変更比較         │ アップロード       │
│ あり      │  → 競合解決      │                   │
├───────────┼─────────────────┼───────────────────┤
│ ローカル  │ ダウンロード     │ 何もしない         │
│ なし      │                 │                   │
└───────────┴─────────────────┴───────────────────┘
```

### 変更検出

| チェック | 方法 |
|----------|------|
| ローカル変更 | MD5ハッシュがマニフェストと異なるか |
| リモート変更 | Drive の modifiedTime がマニフェストより新しいか |

### 競合解決（タイムスタンプ方式）

```
if ローカル変更 && リモート変更:
    if ローカルタイムスタンプ > リモートタイムスタンプ:
        → ローカル優先（アップロード）
    else:
        → リモート優先（ダウンロード）
elif ローカル変更:
    → アップロード
elif リモート変更:
    → ダウンロード
else:
    → 何もしない
```

### sync_directory() の追加処理

- ローカルにあってリモートにないファイル → アップロード
- リモートにあってローカルにないファイル → ダウンロード
- 両方にあるファイル → sync_single_file と同じロジック

---

## 9. reload_game_state()

同期でダウンロードされたデータをメモリ上のゲーム状態に反映する関数。

```rust
async fn reload_game_state(app: &AppHandle) {
    let game_state = app.state::<GameState>();
    let mut state = game_state.inner.write().await;

    // 1. 任務進捗をリロード
    state.history.quest_progress = load_progress(&qp_path);

    // 2. 改修装備履歴をリロード
    state.history.improved_equipment = load_improved_history(&ie_path);

    // 3. 戦闘ログをディスクからリロード
    state.sortie.battle_logger.reload_from_disk();
}
```

### リロードタイミング

- `full_sync()` 内で各 `SyncTarget` の処理後、ダウンロードがあった場合に即座に実行
- ターゲットごとに逐次実行することで、遅いターゲット（raw_api 等）を待たずにUIが更新される

### イベント発火

```rust
if changed {
    reload_game_state(app).await;
    let _ = app.emit("drive-data-updated", ());
}
```

- `drive-data-updated` イベントでフロントエンドに更新を通知

---

## 10. notify_sync()（即時アップロード通知）

ゲーム内のデータ変更時に同期エンジンへアップロードを依頼する関数。

```rust
fn notify_sync(state: &GameStateInner, paths: Vec<&str>) {
    if let Some(tx) = &state.sync_notifier {
        let _ = tx.try_send(SyncCommand::UploadChanged(
            paths.into_iter().map(|s| s.to_string()).collect(),
        ));
    }
}
```

### 呼び出し箇所

| 変更イベント | 通知されるファイル |
|-------------|------------------|
| 任務進捗更新 | `quest_progress.json` |
| 改修実施 | `improved_equipment.json` |
| 戦果変更 | `senka_log.json` |
| 戦闘ログ追加 | `battle_logs/YYYY-MM-DD.json` |
| 編成記憶更新 | `formation_memory.json` |

---

## 11. Tauri コマンド

### drive_login

```rust
pub(crate) async fn drive_login(app: AppHandle, state: State<'_, GameState>) -> Result<(), String>
```

1. `client_credentials()` でOAuth情報を取得
2. `authenticate()` でブラウザ認証フロー実行
3. `start_sync_engine()` でバックグラウンドタスク開始
4. `sync_notifier` を GameState に格納

### drive_logout

```rust
pub(crate) async fn drive_logout(state: State<'_, GameState>) -> Result<(), String>
```

1. `SyncCommand::Shutdown` を送信してエンジン停止
2. `auth::logout()` でトークンファイル削除

### get_drive_status

```rust
pub(crate) async fn get_drive_status(state: State<'_, GameState>) -> Result<SyncStatus, String>
```

- `sync_notifier` の有無で `authenticated` を判定
- マニフェストから `last_full_sync` を取得

### drive_force_sync

```rust
pub(crate) async fn drive_force_sync(state: State<'_, GameState>) -> Result<(), String>
```

- `SyncCommand::FullSync` をチャネルに送信

---

## 12. フロントエンド: SettingsTab

### ファイル構成

| ファイル | 役割 |
|----------|------|
| `src/components/settings/SettingsTab.tsx` | 設定画面全体 |
| `src/components/settings/SettingsTab.css` | スタイル |
| `src/types/common.ts` | DriveStatus 型定義 |

### DriveStatus 型

```typescript
interface DriveStatus {
  authenticated: boolean;
  email?: string;
  syncing: boolean;
  last_sync?: string;
  error?: string;
}
```

### Props

```typescript
interface SettingsTabProps {
  driveStatus: DriveStatus;
  driveLoading: boolean;
  onDriveStatusChange: (status: DriveStatus) => void;
  onDriveLoadingChange: (v: boolean) => void;
  // ... 他の設定用props
}
```

### UI 状態遷移

```
[未認証]
  ├─ 説明テキスト「Google Driveと同期して、複数端末間でデータを共有できます。」
  ├─ エラー表示（あれば）
  └─ [Googleでログイン] ボタン
       ↓ クリック
  invoke("drive_login") → ブラウザ認証フロー
       ↓ 成功
[認証済み]
  ├─ メールアドレス or 「認証済み」
  ├─ ステータス: 「同期中」| 「エラー: ...」| 「変更待機中」
  ├─ [手動同期] ボタン → invoke("drive_force_sync")
  ├─ [ログアウト] ボタン → invoke("drive_logout")
  └─ 最終同期時刻
```

### イベント監視（親コンポーネント）

```typescript
listen<DriveStatus>("drive-sync-status", (event) => {
    setDriveStatus(event.payload);
});

listen("drive-data-updated", () => {
    // ポートデータ等を再取得
});
```

### CSS クラス

| クラス | 用途 |
|--------|------|
| `.drive-sync-content` | 同期セクションのコンテナ |
| `.drive-sync-btn` | ログイン/手動同期ボタン |
| `.drive-sync-btn-sm` | 小型ボタン（手動同期/ログアウト） |
| `.drive-sync-email` | メールアドレス表示（シアン色） |
| `.drive-sync-status-value.syncing` | 同期中の状態表示（シアン色） |
| `.drive-sync-status-value.error` | エラー状態表示（赤色） |
| `.drive-sync-error` | エラーメッセージボックス |

---

## 13. セキュリティ考慮

- OAuth 2.0 の `drive.file` スコープにより、アプリが作成したファイルのみアクセス可能
- トークンは `google_drive_token.json` にローカル保存（yup_oauth2 の標準機能）
- ログアウト時にトークンファイルを物理削除
- Client Secret はデスクトップアプリでは非機密（Google のガイドラインに準拠）

---

## 14. データフロー全体図

```
[ゲームプレイ]
     ↓ API傍受
[データ変更] (任務/改修/戦果/戦闘ログ)
     ↓ ファイル保存
[sync/ ディレクトリ]
     ↓ notify_sync()
[SyncCommand::UploadChanged] → mpsc チャネル
     ↓
[Sync Engine (tokio task)]
     ↓ upload_file()
[Google Drive] ← "KanColle Browser Sync" フォルダ
     ↓ 5分ポーリング or FullSync コマンド
[Sync Engine]
     ↓ download_file() (リモートが新しい場合)
[sync/ ディレクトリ]
     ↓ reload_game_state()
[GameStateInner] (メモリ上のデータ更新)
     ↓ emit("drive-data-updated")
[Frontend] (UI再描画)
```

---

## 15. 関連ファイル一覧

| ファイル | 役割 |
|----------|------|
| `src-tauri/src/drive_sync/mod.rs` | SyncTarget, SyncManifest, SyncCommand, SyncStatus 定義 |
| `src-tauri/src/drive_sync/auth.rs` | OAuth2 認証フロー |
| `src-tauri/src/drive_sync/engine.rs` | 同期エンジン（バックグラウンドタスク） |
| `src-tauri/src/drive_sync/files.rs` | Google Drive API ラッパー |
| `src-tauri/src/commands.rs` | Tauri コマンド（drive_login 等） |
| `src-tauri/src/lib.rs` | 起動時の自動復元処理 |
| `src-tauri/src/api/mod.rs` | notify_sync() 関数 |
| `src/components/settings/SettingsTab.tsx` | 設定UI |
| `src/components/settings/SettingsTab.css` | スタイル |
| `src/types/common.ts` | DriveStatus 型定義 |
