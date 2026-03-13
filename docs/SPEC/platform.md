<!-- AUTO-GENERATED from source code -->

# プラットフォーム基本設計

KanColle Browser における macOS / Windows のプラットフォーム差異を整理する。

---

## 1. アーキテクチャ概要

### macOS

```
┌───────────────┐     ┌──────────────────────────┐     ┌─────────────────┐
│ WKWebView     │     │ hudsucker MITM Proxy      │     │ KanColle Server │
│ (game-content)│────▶│ 127.0.0.1:19080           │────▶│ HTTPS (443)     │
│ proxy_url()   │     │ CA証明書で TLS 終端        │     │                 │
└───────────────┘     └──────────────────────────┘     └─────────────────┘
```

- WebView エンジン: **WKWebView** (macOS 14+ 必須)
- API 傍受方式: **hudsucker MITM プロキシ** (ポート 19080 固定)
- Tauri feature: `macos-proxy`, `macos-private-api`

### Windows

```
┌───────────────┐     ┌──────────────────────────┐     ┌─────────────────┐
│ WebView2      │     │ hudsucker MITM Proxy      │     │ KanColle Server │
│ (game-content)│────▶│ 127.0.0.1:19080           │────▶│ HTTPS (443)     │
│ proxy_url()   │     │ CA証明書で TLS 終端        │     │                 │
└───────────────┘     └──────────────────────────┘     └─────────────────┘
```

- WebView エンジン: **WebView2** (Chromium ベース)
- API 傍受方式: **hudsucker MITM プロキシ** (macOS と同一)
- ネイティブ API: `webview2-com 0.38`, `windows-core 0.61`

> 両プラットフォームとも hudsucker プロキシ経由で API を傍受する共通設計。差異は WebView エンジン固有の API やデータ永続化方式にある。

---

## 2. プラットフォーム差異比較表

| 項目 | macOS | Windows |
|------|-------|---------|
| WebView エンジン | WKWebView | WebView2 (Chromium) |
| プロキシ | hudsucker (ポート 19080) | hudsucker (ポート 19080) |
| CA 証明書確認 | `security find-certificate -c` | `certutil -verifystore Root` |
| CA 証明書インストール | `security import` + `add-trusted-cert` (2段階) | `certutil -addstore Root` (PowerShell UAC 昇格) |
| CA インストール認証 | macOS パスワードダイアログ | UAC 昇格ダイアログ |
| データ永続化 | `data_store_identifier` (WKWebsiteDataStore) | `data_directory` (ファイルベース) |
| ミュート API | `objc2` で `_setPageMuted:` (Private API) | `webview2-com` で `ICoreWebView2_8::SetIsMuted` |
| ブラウザデータリセット | `clear_all_browsing_data()` + ファイル削除 | ゲーム画面を閉じてからファイル削除 |
| タイトルバー高さ | 28px (inner_size に含まれる) | 0px (inner_size に含まれない) |
| ブラウザキャッシュ場所 | `~/Library/Caches/<app>/WebKit/` | `local/game-webview/EBWebView/Default/Cache` 等 |
| 陣形ヒント座標補正 | `dx += 6*scale`, `dy += 30*scale` | 補正なし |
| DOM 初期化 | 即時注入可能 | `MutationObserver` で DOM 出現を待機 |
| platform 固有依存 | `objc2 0.6`, `objc2-foundation 0.3` | `webview2-com 0.38`, `windows-core 0.61` |

---

## 3. CA 証明書管理 (`ca.rs`)

### 3.1 証明書の生成と永続化

両プラットフォーム共通。起動時に `load_or_generate_ca()` が呼ばれ、PEM ファイルが存在すればロード、なければ新規生成する。

- 保存先: `{local_data_dir}/kancolle-browser/`
  - `ca.key.pem` — 秘密鍵
  - `ca.cert.pem` — 証明書 (CN: "KanColle Browser CA")

### 3.2 証明書の確認 (`is_ca_installed`)

| プラットフォーム | コマンド |
|----------------|---------|
| macOS | `security find-certificate -c "KanColle Browser CA"` |
| Windows | `certutil -verifystore Root "KanColle Browser CA"` |

### 3.3 証明書のインストール (`install_ca_cert`)

#### macOS (2段階)

1. **Keychain にインポート**: `security import <pem> -k <login.keychain-db> -t cert`
2. **SSL 信頼設定**: `security add-trusted-cert -d -r trustRoot -k <login.keychain-db> <pem>`
   - macOS のパスワードダイアログが表示される (ユーザー操作必須)

#### Windows

1. **PowerShell UAC 昇格**: `Start-Process -FilePath certutil.exe -ArgumentList '-addstore','Root','<pem>' -Verb RunAs -Wait`
   - UAC ダイアログが表示される (ユーザー操作必須)
   - `CREATE_NO_WINDOW` フラグでコンソールウィンドウを非表示

---

## 4. ゲームウィンドウ管理 (`game_window.rs`)

### 4.1 ウィンドウサイズ計算

```
ウィンドウ高さ = GAME_HEIGHT (720) + CONTROL_BAR_HEIGHT (28) + MACOS_TITLEBAR_HEIGHT
```

| 定数 | macOS | Windows |
|------|-------|---------|
| `MACOS_TITLEBAR_HEIGHT` | 28.0 | 0.0 |

macOS では `tao/tauri` が `inner_size` にタイトルバーを含むため (tauri-apps/tauri#6333)、28px の補正が必要。

### 4.2 データ永続化方式

#### macOS: `data_store_identifier`

```rust
const GAME_DATA_STORE_ID: [u8; 16] = [...]; // "kancolle-browser" のバイト列
game_wv_builder = game_wv_builder.data_store_identifier(GAME_DATA_STORE_ID);
```

- `WKWebsiteDataStore` の固定 UUID で Cookie/セッション/キャッシュをネイティブ永続化
- macOS 14+ が必須

#### Windows: `data_directory`

```rust
let data_dir = app.path().app_local_data_dir()?.join("local").join("game-webview");
game_wv_builder = game_wv_builder.data_directory(data_dir);
```

- ファイルベースの WebView2 プロファイル (`EBWebView/` ディレクトリ)

### 4.3 ミュート制御 (`toggle_game_mute`)

#### macOS

```rust
use objc2::msg_send;
let _: () = msg_send![wk, _setPageMuted: muted_state]; // Private API
```

- `_WKMediaAudioMuted = 1 << 0` で WKWebView 全体をミュート
- `macOSPrivateApi: true` が `tauri.conf.json` で必要

#### Windows

```rust
use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2_8;
core8.SetIsMuted(muted);
```

- `ICoreWebView2_8` インターフェースの公式 API

---

## 5. Cookie 管理 (`cookie.rs`)

### 共通方式

両プラットフォームとも同一のフロー:

1. **保存**: `save_game_cookies` — `cookies_for_url()` で DMM 関連ドメインの Cookie を取得し、JSON ファイルに永続化
2. **復元**: `build_cookie_restore_script` — 保存した Cookie を `document.cookie` で注入する JavaScript を生成
3. **注入タイミング**: `about:blank` → `initialization_script` で注入 → 500ms 待機 → DMM へ遷移

### プラットフォーム固有の注意点

| 問題 | 説明 |
|------|------|
| WebView2 SameSite 制約 | ネイティブ `set_cookie` API は `SameSite=None` のドット付きドメイン Cookie を拒否する。`document.cookie` 直接注入で回避 |
| macOS ポート固定 | WKWebView はポート変更でオリジン不一致と判定し Cookie が消失するため、19080 に固定 |
| アプリ終了時保存 | `RunEvent::ExitRequested` ハンドラで Cookie を同期保存 (async 不可のためブロッキング) |

---

## 6. オーバーレイ実装 (`overlay.rs`)

### 共通設計

マルチ WebView 構成:

| WebView | 用途 | 配置 |
|---------|------|------|
| `game-content` | ゲーム本体 | 下層 |
| `game-overlay` | ミニマップ/大破警告 | 上層 (透明) |
| `formation-hint-content` | 陣形ヒント | 別ウィンドウ (click-through) |
| `expedition-notify-content` | 遠征通知 | 別ウィンドウ (click-through) |

### プラットフォーム固有の差異

#### 陣形ヒント座標補正 (`api/formation.rs`)

```rust
#[cfg(target_os = "macos")]
{
    dx += (6.0 * scale) as i32;
    dy += (30.0 * scale) as i32;
}
```

macOS ではウィンドウ装飾と座標系の違いにより、ヒントウィンドウの位置を補正する必要がある。Windows では補正不要。

#### 通知ウィンドウ位置

`MACOS_TITLEBAR_HEIGHT` がオフセット計算に含まれるため、macOS ではコントロールバーの Y 座標が 28px 下がる:

```rust
let top_offset = MACOS_TITLEBAR_HEIGHT + CONTROL_BAR_HEIGHT + margin;
```

---

## 7. ブラウザデータリセット (`commands.rs`)

### macOS

1. ゲームウィンドウが開いていれば `clear_all_browsing_data()` API を使用
2. ゲームウィンドウを閉じる
3. ファイルシステムから削除:
   - `~/Library/Caches/<app>/` (HTTP キャッシュ)
   - `~/Library/WebKit/<app>/` (WKWebsiteDataStore)
   - `~/Library/HTTPStorages/<app>/` (Cookie/HTTP ストレージ)
4. 保存済み Cookie ファイル削除

対象アプリ名: `kancolle-browser`, `com.eo.kancolle-browser`

### Windows

1. **前提条件**: ゲーム画面が閉じていること (EBWebView ディレクトリがロックされるため)
2. `local/game-webview/EBWebView/` ディレクトリを削除
3. 保存済み Cookie ファイル削除

### ブラウザキャッシュのみ削除 (`clear_browser_cache`)

| macOS | Windows |
|-------|---------|
| `~/Library/Caches/<app>/WebKit/` | `EBWebView/Default/Cache`, `Code Cache`, `GPUCache`, `ShaderCache` 等 8 ディレクトリ |

---

## 8. Cargo.toml ターゲット依存

### 共通依存

```toml
tauri = { features = ["macos-proxy", "macos-private-api", "unstable"] }
hudsucker = { features = ["rcgen-ca", "rustls-client"] }
```

### macOS 固有

```toml
[target.'cfg(target_os = "macos")'.dependencies]
objc2 = "0.6"
objc2-foundation = { version = "0.3", features = ["NSValue"] }
```

- WKWebView のミュート制御 (`_setPageMuted:`)

### Windows 固有

```toml
[target.'cfg(target_os = "windows")'.dependencies]
webview2-com = "0.38"
windows-core = "0.61"
```

- WebView2 のミュート制御 (`ICoreWebView2_8::SetIsMuted`)
- `webview2-com 0.38` は PascalCase API、out-ptr 文字列方式

---

## 9. Tauri 設定 (`tauri.conf.json`)

```json
{
  "app": {
    "macOSPrivateApi": true,
    "security": { "csp": null }
  }
}
```

- `macOSPrivateApi: true` — WKWebView の `_setPageMuted:` 等の Private API 使用に必須
- `csp: null` — Content Security Policy 無効化 (ゲーム iframe の制約回避)

---

## 10. 初期化スクリプト (`game_init.js`)

### 共通処理

- CSS 注入: スクロールバー非表示、DMM UI 隠蔽、ゲームフレーム固定
- コントロールバー挿入: ズーム選択、ミュートボタン、陣形/大破/MAP 切替

### WebView2 互換対策

```javascript
// MutationObserver で DOM (head/documentElement) 出現を待機
function injectStyle() {
    var target = document.head || document.documentElement;
    if (!target) return false;
    // ...
}
if (!injectStyle()) {
    var obs = new MutationObserver(function(mutations, observer) {
        if (injectStyle()) observer.disconnect();
    });
    obs.observe(document, { childList: true, subtree: true });
}
```

WebView2 では `initialization_script` 実行時に DOM が未構築の場合があるため、`MutationObserver` でフォールバックする。WKWebView では通常この問題は発生しない。

---

## 11. 既知の制約・注意点

### WebView2 デッドロック (Windows)

> **CAUTION.md より**: ウィンドウ生成は必ず `async fn` で行うこと。メインスレッドブロック等によるデッドロック/白画面が発生する。

`with_webview()` を同期コマンド内で呼ぶとデッドロックする。setup ハンドラや async コンテキストから使用すること。

### `additional_browser_args` 使用禁止

`additional_browser_args` を使用するとプロキシ設定が上書きされ、API 傍受が無効化される。スクロールバー消去等は CSS で対応すること。

### macOS ポート固定の必要性

WKWebView はプロキシポート変更でオリジン不一致と判定し、Cookie やセッションが揮発する。19080 ポートへの固定が必須 (使用中の場合は OS 割当にフォールバック)。

### macOS タイトルバー高さ

`tao/tauri` が `inner_size` にタイトルバーを含む macOS 固有の挙動 (tauri-apps/tauri#6333) により、28px の補正定数が全座標計算に影響する。

### Cookie の SameSite 制約

WebView2 のネイティブ Cookie API は `SameSite=None` のドット付きドメイン Cookie を拒否する。`document.cookie` による JavaScript 直接注入で回避する方式を採用。

### Windows ブラウザデータ削除の制約

WebView2 の `EBWebView/` ディレクトリはプロセス起動中はロックされる。リセット操作はゲーム画面を閉じた状態でのみ実行可能。

---

## 12. `cfg(target_os)` 使用箇所一覧

| ファイル | 用途 |
|---------|------|
| `ca.rs` | CA 証明書の確認・インストール (security / certutil) |
| `game_window.rs` | タイトルバー高さ定数、データ永続化方式、ミュート API |
| `commands.rs` | ブラウザデータリセット、ブラウザキャッシュ削除 |
| `api/formation.rs` | 陣形ヒント座標の macOS 補正 |
| `Cargo.toml` | objc2 (macOS) / webview2-com (Windows) 依存 |
