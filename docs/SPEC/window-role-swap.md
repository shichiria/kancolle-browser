# Window Role Swap (Game → Main)

## 概要
ゲームwindowをメインwindow化し、現メインの管理UI(React SPA)はゲーム上部コントロールバーのボタンから hide/show 切替する。

## 背景
- 現状: 起動時に管理UI(1400x900)が表示 → ユーザーが「Open Game」押下でゲームwindowを生成
- 変更後: 起動時に直接ゲームwindowを表示 → 管理UIはコントロールバーボタンで開閉

## 設計判断

| 項目 | 決定 |
|------|------|
| window生成戦略 | 全window事前生成 + hide/show切替(状態保持) |
| CA未インストール時 | 起動時ポップアップ→install。Cancelでアプリ終了 |
| 管理window開閉 | hide/show |
| ゲームwindow閉じた時 | アプリ全体終了 |

## アーキテクチャ

### 起動シーケンス
```
tauri::run()
  ↓
setup hook
  ├─ proxy 起動 (async spawn)
  ├─ management window 生成 (hidden, label="management")
  └─ proxy-ready 待ち
  ↓
proxy ready (event)
  ├─ CA インストール確認
  │    ├─ 済 → 続行
  │    └─ 未 → ask dialog
  │            ├─ Install → install_ca_cert → 続行
  │            └─ Cancel  → app.exit(0)
  └─ game window 生成 + show
       └─ 子window(formation-hint/battle-info/expedition-notify)も生成 (hidden)
  ↓
ユーザー操作
  ├─ control bar「📊 管理」 → toggle_management_window
  ├─ game window 閉じる    → save cookies → app.exit(0)
  └─ management window 閉じる → prevent + hide()
```

### 主要変更点

#### 1. `tauri.conf.json`
```json
"windows": [{
  "label": "management",
  "title": "KanColle Browser",
  "width": 1400, "height": 900,
  "minWidth": 800, "minHeight": 600,
  "visible": false
}]
```

#### 2. 新規 Tauri commands (`game_window.rs` または `management.rs` 新設)
- `toggle_management_window(app)` — 状態見て show/hide + focus
- `show_management_window(app)` / `hide_management_window(app)` — 個別操作

#### 3. 起動時 CA確認 (`lib.rs` setup or run handler)
- `tauri-plugin-dialog` 追加 (Cargo.toml + plugin初期化)
- proxy ready 後に `ca::is_ca_installed()` 確認
- 未済 → `ask("CA証明書が未インストールです。インストールしますか?", ...)` 
  - Yes → `ca::install_ca_cert()`
  - No → `app.exit(0)`

#### 4. game window 自動open (lib.rs setup)
- 現 `open_game_window` をsetup hookから呼ぶ
- ただし CA確認後に呼ぶよう順序制御

#### 5. game window CloseRequested ハンドラ (lib.rs run handler)
- 現状: WindowEvent::CloseRequestedハンドラなし(Tauriデフォルトで閉じる)
- 変更: `game` ラベルのCloseRequested で
  - mouse_hook::uninstall()
  - cookies保存(現app exit時の処理を移植)
  - `app.exit(0)`

#### 6. management window CloseRequested ハンドラ
- イベントintercept → `api.prevent_close()` → `win.hide()`

#### 7. control bar (`game_init.js`) ボタン追加
- 既存ボタン群の末尾(spacer前)に「📊」追加
- click → `__TAURI_INTERNALS__.invoke('toggle_management_window')`

#### 8. React側整理 (`App.tsx`, `HomeportTab.tsx`)
- 削除: 「Open Game」「Close Game」ボタン (App.tsx:340-346)
- 削除: 「Install CA Cert」ボタン (起動時ダイアログに移行, App.tsx:330-338)
- `gameOpen` state → 常時 `true` 想定に整理 or 削除
- HomeportTab の `gameOpen` 分岐を見直し

## フェーズ分割

| # | 内容 | 主要ファイル | 確認方法 |
|---|------|----------|----------|
| 0 | `tauri-plugin-dialog` 追加 | Cargo.toml, lib.rs | `cargo check` |
| 1 | management window を hidden起動に | tauri.conf.json, lib.rs | 起動して画面真っ黒(window非表示)になる |
| 2 | `toggle_management_window` コマンド + control barボタン | game_window.rs (or new), game_init.js, lib.rs | 既存「Open Game」で起動後、control barボタンで開閉確認 |
| 3 | management CloseRequested → hide | lib.rs run handler | ×ボタンでアプリ終了せず非表示になる |
| 4 | 起動時 proxy-ready 後にgame 自動open | lib.rs setup | 起動 → 自動でゲーム画面表示 |
| 5 | 起動時 CA未確認ダイアログ | lib.rs, Cargo.toml | CA削除した状態で起動 → ダイアログ表示 |
| 6 | game CloseRequested → app.exit | lib.rs run handler | game ×ボタンでアプリ全体終了 |
| 7 | React UI整理 | App.tsx, HomeportTab | 「Open Game」等が消えていることを目視確認 |
| 8 | hide/show統一の再点検 | overlay.rs | 既存子window の挙動劣化なし |

## リスク

| リスク | 対策 |
|--------|------|
| Win + WebView2 + proxy のloop問題 (CAUTION.md記載) | CA未済時は絶対にgame window を生成しない (Phase 5でガード) |
| dialogがmain threadブロック | `tauri-plugin-dialog` の async API を `async_runtime::spawn` 内で呼ぶ |
| game window 作成中に管理を開かれた競合 | setupフェーズ完了フラグ(AtomicBool)で排他 |
| management hide中もReact state保持されるか | Tauriのwebviewはhidden中もJSランタイム維持(検証必要) |
| 既存「Open Game」ボタンが消えた状態でCA未済になった場合 | Phase 5以降 起動時必ず確認するため到達不能 |

## 削除されるコード(参考)

- `App.tsx:340-346` Open/Close Game ボタン
- `App.tsx:330-338` CA install ボタン
- `App.tsx:29` `gameOpen` state(残す可能性も)
- `App.tsx:305-322` openGame/closeGame関数

## オープン課題

- management window のタイトルバー「最小化」: hide()と同等扱いでよいか
- ゲーム読込中の「読み込み画面」表示有無 (今回スコープ外、必要なら別SPEC)
- macOSでのCmd+Q挙動 (game window フォーカス時にどう振る舞うか)

## 既知の副作用 (本変更で発生、追加対応要検討)

### `reset_browser_data` が事実上利用不能になる
- **Windows**: 元実装は「gameを閉じてから実行」を要求。Game close = アプリ終了になったため、UIから到達不能。
- **macOS**: clear後に `win.close()` を呼ぶ → CloseRequested → app.exit。データクリアの後始末が完了せずアプリが終了する可能性。
- 対処案: `AppState` に `intentional_close: AtomicBool` を追加し、reset_browser_data で true 設定 → CloseRequested で true なら exit せず close のみ。あるいは reset 後に自動 reopen させる。

### `close_game_window` コマンドが dead code 化
- React 側「Close Game」ボタン削除に伴い、JS から呼ばれなくなった。
- invoke_handler 登録は残存。呼ばれても hide でなく実際に close するので、呼んだ側で app.exit が発火する。
- 対処案: 削除 or `hide_game_window` に rename して将来用途に残す。
