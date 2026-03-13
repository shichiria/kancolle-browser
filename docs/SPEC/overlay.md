<!-- AUTO-GENERATED from source code -->

# オーバーレイ設計書

## 概要

ゲームウィンドウ上に重ねて表示する情報表示レイヤー。マルチWebViewアーキテクチャにより、ゲーム操作を妨げずに情報を提示する。

---

## オーバーレイ機能一覧

| 機能 | ウィンドウ | WebView | HTML | トリガー |
|------|-----------|---------|------|----------|
| ミニマップ | `game` (子WebView) | `game-overlay` | `overlay.html` | 出撃中 (`api_req_map/start`, `/next`) |
| 陣形ヒント | `formation-hint` (別ウィンドウ) | `formation-hint-content` | `formation-hint.html` | セル到達時に過去の陣形を記憶から表示 |
| 大破警告 | `game` (子WebView) | `game-overlay` | `overlay.html` | `/api_req_map/next` で大破進撃検知 |
| 遠征通知 | `expedition-notify` (別ウィンドウ) | `expedition-notify-content` | `expedition-notify.html` | 帰還1分前 (フロントエンドTimer) |

### ウィンドウ構成

```
game (メインウィンドウ)
  +-- game-content (WebView: ゲーム本体, proxy経由)
  +-- game-overlay (WebView: 透明オーバーレイ, overlay.html)
        ミニマップ / 大破警告を表示
        通常時は 1x1px に縮小してクリック透過

formation-hint (別ウィンドウ, always-on-top)
  +-- formation-hint-content (WebView: formation-hint.html)
        装飾なし, 透明, マウスイベント無視 (set_ignore_cursor_events)
        シアン色の枠線でゲーム内ボタン位置をハイライト

expedition-notify (別ウィンドウ, always-on-top)
  +-- expedition-notify-content (WebView: expedition-notify.html)
        装飾なし, 透明, マウスイベント無視
        ゲームウィンドウ右上に追従
```

---

## overlay.rs の実装詳細

**ファイル**: `src-tauri/src/overlay.rs`

### AppState フィールド (lib.rs で定義)

| フィールド | 型 | デフォルト | 永続化先 |
|------------|-----|-----------|----------|
| `formation_hint_enabled` | `AtomicBool` | `true` | `local/formation_hint_enabled` |
| `taiha_alert_enabled` | `AtomicBool` | `true` | `local/taiha_alert_enabled` |
| `minimap_enabled` | `AtomicBool` | `true` | `local/minimap_enabled` |
| `expedition_notify_visible` | `AtomicBool` | `false` | - (実行時のみ) |
| `formation_hint_rect` | `Mutex<FormationHintRect>` | default | - (実行時のみ) |
| `minimap_position` | `Mutex<Option<(f64, f64)>>` | `None` | `local/minimap_position.json` |
| `minimap_size` | `Mutex<(f64, f64)>` | `(310, 210)` | `local/minimap_size.json` |
| `game_zoom` | `Mutex<f64>` | `1.0` | - |

### Tauri コマンド

| コマンド | 引数 | 戻り値 | 説明 |
|----------|------|--------|------|
| `set_formation_hint_enabled` | `enabled: bool` | `Result<(), String>` | 陣形ヒントの有効/無効切替。無効時は即座に非表示化 |
| `get_formation_hint_enabled` | - | `bool` | 現在の陣形ヒント有効状態を取得 |
| `set_taiha_alert_enabled` | `enabled: bool` | `Result<(), String>` | 大破警告の有効/無効切替 |
| `get_taiha_alert_enabled` | - | `bool` | 現在の大破警告有効状態を取得 |
| `set_overlay_visible` | `visible: bool` | `Result<(), String>` | オーバーレイ表示/非表示。非表示時は1x1pxに縮小 |
| `dismiss_overlay` | - | `Result<(), String>` | 大破警告を閉じる。ミニマップ有効なら復帰、なければ非表示 |
| `toggle_minimap` | - | `Result<bool, String>` | ミニマップのON/OFF切替。有効化時に出撃中なら即座にデータ送信 |
| `get_minimap_enabled` | - | `bool` | 現在のミニマップ有効状態を取得 |
| `move_minimap` | `dx: f64, dy: f64` | `Result<(), String>` | ミニマップをドラッグ移動。位置をディスクに永続化 |
| `resize_minimap` | `w: f64` | `Result<(), String>` | ミニマップのリサイズ。アスペクト比5:3で高さ自動計算 |
| `show_expedition_notification` | `notifications: Vec<ExpeditionNotifyItem>` | `Result<(), String>` | 遠征通知ウィンドウを表示 |
| `hide_expedition_notification` | - | `Result<(), String>` | 遠征通知ウィンドウを非表示 |

### ミニマップ定数

```rust
const MINIMAP_DEFAULT_W: f64 = 310.0;
const MINIMAP_DEFAULT_H: f64 = 210.0;
const MINIMAP_MIN_W: f64 = 200.0;
const MINIMAP_MAX_W: f64 = 600.0;
const MINIMAP_MARGIN: f64 = 6.0;
const MINIMAP_ASPECT: f64 = 0.68;  // h/w 比率
```

### show_minimap_overlay ロジック

1. `game-overlay` WebView と `game` ウィンドウを取得
2. ゲームウィンドウの物理サイズから論理サイズを計算 (`scale_factor` 適用)
3. 保存済み位置があれば使用、なければ右下デフォルト位置を使用
4. コントロールバー高さ (`CONTROL_BAR_HEIGHT * zoom`) を考慮してY座標下限を設定
5. オーバーレイの位置とサイズを設定

### ウィンドウ追従ロジック

`game_window.rs` の `on_window_event` で以下のイベントをハンドル:

- **Resized**: `game-content` WebView リサイズ + 陣形ヒント再配置 + ミニマップ再配置 + 遠征通知再配置
- **Moved**: 陣形ヒント再配置 + 遠征通知再配置

```
reposition_formation_hint(app)  // FormationHintRect の dx/dy を使って画面座標を再計算
reposition_expedition_notification(app)  // ゲームウィンドウ右上に追従
show_minimap_overlay(app)  // ミニマップ有効時のみ
```

---

## api/formation.rs のロジック

**ファイル**: `src-tauri/src/api/formation.rs`

### 陣形名マッピング

```rust
fn formation_name(id: i32) -> &'static str
```

| ID | 名前 | 種別 |
|----|------|------|
| 1 | 単縦陣 | 通常 |
| 2 | 複縦陣 | 通常 |
| 3 | 輪形陣 | 通常 |
| 4 | 梯形陣 | 通常 |
| 5 | 単横陣 | 通常 |
| 6 | 警戒陣 | 通常 |
| 11 | 第一警戒航行序列(対潜警戒) | 連合 |
| 12 | 第二警戒航行序列(前方警戒) | 連合 |
| 13 | 第三警戒航行序列(輪形陣) | 連合 |
| 14 | 第四警戒航行序列(戦闘隊形) | 連合 |

### 陣形ボタン座標 (1200x720 キャンバス座標)

```rust
fn get_formation_button_rect(formation: i32, _ship_count: usize)
    -> Option<(f64, f64, f64, f64)>  // (x, y, w, h)
```

ボタンサイズ: 154x48 px (sally_jin アトラスのラベルスプライト)

**通常陣形 (3列 x 2行):**

| 陣形 | 中心 (cx, cy) | グリッド位置 |
|------|--------------|-------------|
| 単縦陣 | (663, 278) | 1列1行 |
| 複縦陣 | (858, 278) | 2列1行 |
| 輪形陣 | (1056, 278) | 3列1行 |
| 梯形陣 | (766, 517) | 1列2行 |
| 単横陣 | (960, 517) | 2列2行 |
| 警戒陣 | (1048, 517) | 3列2行 |

**連合艦隊陣形 (2列 x 2行):**

| 陣形 | 中心 (cx, cy) |
|------|--------------|
| 第一警戒 | (743, 263) |
| 第二警戒 | (993, 263) |
| 第三警戒 | (743, 468) |
| 第四警戒 | (993, 468) |

### show_formation_hint ロジック

1. `formation_hint_enabled` が `false` なら即リターン
2. ゲームウィンドウの `inner_position` とスケールファクターを取得
3. ボタン座標に `zoom * scale` を乗じて物理ピクセルオフセットを計算
4. コントロールバー高さ (28px) をY座標に加算
5. macOS では追加の座標オフセットを適用 (`dx += 6*scale, dy += 30*scale`)
6. `FormationHintRect` にオフセットを保存 (ウィンドウ移動時の再配置用)
7. `formation-hint` ウィンドウのサイズ・位置を設定して表示

### 陣形記憶の仕組み (battle.rs)

- **記録**: 戦闘開始API (`/api_req_sortie/battle` 等) で自軍陣形を `formation_memory` に保存
  - キー: `"{map_area}-{map_no}-{cell_no}"` (例: `"1-1-3"`)
  - 値: 陣形ID
  - 永続化: `formation_memory.json` にJSON保存 + GDrive同期
- **表示**: セル到達API (`/api_req_map/start`, `/next`) で記憶済み陣形をルックアップ
  - 大破警告表示中はスキップ
- **非表示**: 戦闘開始時に `hide_formation_hint` を呼び出し

---

## api/minimap.rs のロジック

**ファイル**: `src-tauri/src/api/minimap.rs`

### update_minimap_overlay

出撃中のノード情報が更新されるたびに呼び出される。

1. `minimap_enabled` を確認、無効なら即リターン
2. `send_minimap_data` を呼び出し

### send_minimap_data

1. `game-overlay` WebView を取得
2. `SortieRecord` から各ノードの情報を JSON 化:
   - `cell_no`: セル番号
   - `event_kind`: イベント種別 (戦闘/資源/ボス等)
   - `event_id`: イベントID
   - `has_battle`: 戦闘有無
3. `map_display` (例: `"1-1"`) をシリアライズ
4. `show_minimap_overlay` でオーバーレイサイズ・位置を設定
5. `window.updateMinimap(mapDisplay, nodes)` を eval で呼び出し

### overlay.html 側のミニマップ描画

1. マップデータ読み込み (`_info.json` + `_image.json`) をTauriコマンド経由で取得
2. スプライト画像 (背景、セルマーカー、ルート接続線) を `get_map_sprite` で取得
3. SVG でノード円+ラベルを描画
   - 色: `CELL_COLORS` (戦闘=赤、ボス=濃赤、資源=黄、等)
   - 現在ノード: 黄色パルスアニメーション
   - 未訪問ルート: `grayscale(100%) opacity(40%)`
4. 下部に現在ノードラベルを表示

### ドラッグ・リサイズ

- **ドラッグ**: タイトルバー (`minimap-drag`) の `mousedown` → `mousemove` で `move_minimap` コマンドを呼出
- **リサイズ**: 右下ハンドル (`minimap-resize-handle`) → `resize_minimap` コマンドを呼出

---

## 大破警告の実装 (battle.rs + overlay.html)

### トリガー条件

`/api_req_map/next` (次のセルへ進撃) を傍受した時:

1. `taiha_alert_enabled` を確認
2. 出撃中の艦隊の各艦のHPを確認
3. `hp / maxhp <= 0.25` かつ `hp > 0` (大破状態) の艦を検出
4. ダメコン (応急修理女神/要員, `icon_type == 14`) を装備していれば除外
5. 大破艦が1隻以上いればオーバーレイを全画面展開して `showTaihaWarning(ships)` を eval

### overlay.html 側の表示

- 半透明黒背景 + 赤フラッシュアニメーション (0.5秒 x 3回)
- ダイアログ: 警告アイコン + 「大破進撃警告」タイトル + 大破艦名リスト + 確認ボタン
- 確認ボタン押下で `dismiss_overlay` コマンドを呼出 → ミニマップ復帰 or 非表示

---

## 遠征通知の実装

### トリガー (フロントエンド: App.tsx)

- 1秒タイマーで `portData.fleets` の遠征帰還時刻をチェック
- 帰還まで60秒以内の艦隊を検出
- `show_expedition_notification` コマンドを呼出

### overlay.rs 側

- `expedition-notify` ウィンドウをゲームウィンドウ右上に配置
- 高さは通知件数に応じて動的計算: `BASE_H (28px) + items * ITEM_H (18px)`
- 幅: 250px 固定
- ウィンドウ位置: `game` ウィンドウの `inner_position + inner_size` から逆算

### expedition-notify.html 側

- `showNotifications(items)` で DOM を動的生成
- フェードインアニメーション (右からスライド, 0.3秒)
- 各遠征を `[艦隊番号] 遠征名` 形式で表示

---

## ゲームウィンドウとの連携 (game_window.rs)

**ファイル**: `src-tauri/src/game_window.rs`

### ウィンドウ構成定数

```rust
const GAME_WIDTH: f64 = 1200.0;   // KanColle ネイティブ解像度
const GAME_HEIGHT: f64 = 720.0;
const CONTROL_BAR_HEIGHT: f64 = 28.0;  // 注入されたコントロールバー
const MACOS_TITLEBAR_HEIGHT: f64 = 28.0;  // macOS のみ (Windows: 0.0)
```

### open_game_window の処理フロー

1. 既存ウィンドウがあればフォーカス
2. プロキシポートを `AppState` から取得
3. `game` ウィンドウを作成 (最小サイズ: 50% スケール)
4. `game-content` WebView を追加 (プロキシ経由, about:blank → DMM ナビゲーション)
5. **`game-overlay` WebView を追加** (透明、1x1px 非表示)
6. **`formation-hint` ウィンドウ+WebView を作成** (装飾なし、透明、always-on-top、カーソル無視)
7. **`expedition-notify` ウィンドウ+WebView を作成** (同上)
8. `on_window_event` でリサイズ/移動イベントをフック

### close_game_window の処理フロー

1. `formation-hint` ウィンドウを閉じる
2. `expedition-notify` ウィンドウを閉じる
3. Cookie 保存
4. `game` ウィンドウを閉じる

### set_game_zoom のオーバーレイ連動

- ズーム変更時に `game-content` WebView のzoomを設定
- ウィンドウサイズを `GAME_WIDTH * zoom x (GAME_HEIGHT * zoom + CONTROL_BAR + TITLEBAR)` に変更
- **オーバーレイは意図的にリサイズしない** (1x1px のまま。リサイズすると入力をブロックする)
- ミニマップ有効時は `show_minimap_overlay` で再配置
