# 設計クリーンアップ計画 (refactor-design-cleanup)

作成: 2026-07-15 / ベースコミット: `7cb0965` (feat: 診断ログ基盤 + 退避艦の大破警告抑止)
監査エビデンス: `.reports/design-audit-2026-07-15.md` (Rust/フロントエンド/横断の3系統監査)

## 1. 目的と背景

リポジトリ全体 (Rust ~13.6kL / React ~6.5kL) の設計監査で確認された負債を、**挙動を変えずに**解消する。
機械的健全性は高い (cargo test 141 green / vitest 45 green / clippy・tsc 警告0) ため、テストを安全網として段階的に構造を改善する。

確認された負債 (詳細は監査レポート参照):

| 分類 | 内容 |
|------|------|
| 構造 | `process_api` 983行のgod関数、App.tsx godコンポーネント (全windowで全副作用実行)、800L超4ファイル |
| 重複 | ウィンドウトグル3種 / Checker 3種 / redact 3実装 (キー不一致) / 設定永続化5連 / type filter 3種 / 条件行markup 4コピー |
| 暗黙契約 | invoke文字列36種 vs Rustコマンド67 (型・レジストリなし)、イベント名~20種が裸文字列散在 |
| プロセス | CIテストゲートなし、lint設定皆無、コンポーネントテスト基盤なし |
| ドキュメント | 診断ログ・退避艦追跡がSPEC/CODEMAPS未反映、stale記載数件 |

### ユーザー決定事項 (2026-07-15)
1. WIP (診断ログ+退避艦追跡) は現状のままコミット済み (`7cb0965`)。非整合の修正は本計画の項目とする
2. game_init.js のDMMページレイアウト操作・広告除去は**容認** (「ゲーム画面に操作していなければOK」)。境界の明文化と座標ハードコード解消は実施
3. スコープ: フルスコープ (構造+プロセス基盤+セキュリティ+CSS+ドキュメント)

## 2. 実行プロトコル (実行AIは必読)

- **挙動不変が絶対原則**。バグ修正・機能変更・「ついでの改善」は禁止。例外は Phase 1 のWIP非整合修正のみ
- **1項目 = 1コミット = 単独revert可能**。コミット形式: `refactor: W3-1 ウィンドウトグル統合` のように項目IDを含める (Phase 1/2 は `fix:`/`ci:`/`docs:`/`test:` を適宜)
- 各項目の完了時に code-reviewer エージェントを実行し、CRITICAL/HIGH は修正してからコミット (プロジェクト規約)
- **検証 (毎項目必須)**:
  ```
  npm test                                        # vitest + cargo check + cargo test
  npx tsc --noEmit                                # TS型チェック
  cargo clippy --lib --manifest-path src-tauri/Cargo.toml -- -D warnings
  ```
  UI影響項目 (📺印) は手動smoke: `taskkill //IM kancolle-browser.exe //F` と `taskkill //IM cargo.exe //F` で全プロセス終了 → `npm run tauri dev` をバックグラウンド起動 → ユーザーに確認を依頼 (3window表示 / ゲーム起動 / オーバーレイ)
- **テスト修正の制約**: モジュール移動に伴う `use` パス変更のみ可。**アサーション・期待値の変更は禁止** (必要になったら挙動が変わったシグナル → 中断)
- **迷ったら停止**: 設計判断が必要になった場合、実施せず SESSION.md に疑問を記録して停止。30分詰まったら `git checkout .` で破棄し SESSION.md に記録して次項目へ
- **行番号は `7cb0965` 時点の目安**。必ず関数名・シグネチャで対象を特定すること
- 各項目開始前: `git status` が clean であること (`.claude/scheduled_tasks.lock` は無視してよい)

### 実行順序と依存

```
Phase 1 (整合) → Phase 2 (安全網) → Phase 3 (Rust) ┬→ Phase 5 (CSS) → Phase 6 (深部) → Phase 7 (仕上げ)
                                     Phase 4 (FE) ──┘
依存edge:
  W3-4 (events.rs+EVENTS) → W4-2 (useTauriEvent)
  W2-3 (コンポーネントテスト基盤) → W4-4, W4-5
  W3-6 → W3-7 (同一ファイル api/mod.rs、順序厳守)
  W4-1, W4-2 → W4-3 → W4-4 → W4-5 (順序厳守)
Phase 3 と Phase 4 は依存edge以外は並行可能。Phase内の他項目は原則独立。
```

---

## 3. Phase 1: WIP残課題の解消 + ドキュメント整合

### W1-1 `get_action_log` のrelease整合
- **目的**: action_log は release でも常時ONになったが、ビューアコマンドが `#[cfg(debug_assertions)]` のままで release では空を返す非整合を解消
- **対象**: `src-tauri/src/commands.rs` の `get_action_log` (~:980-995)
- **手順**: `#[cfg(debug_assertions)]` ゲートと release 用スタブ (空Vec返し) を除去し、常にリングバッファを返す実装に一本化
- **受入基準**: release ビルド想定のコードパスでも `action_log::recent()` 相当が返る。debug/release で同一実装
- **検証**: 標準3点 + DebugTab でアクションが表示されること (📺)

### W1-2 redactキーリストの一元化
- **目的**: 秘匿キーリストが3箇所 (diagnostics.rs / battle_log redact_request_body / diagnostics.ts) に独立実装され、battle_log 側に `rpctoken`・`st` が欠落
- **対象**: `src-tauri/src/diagnostics.rs` (:179-238 redact_sensitive)、`src-tauri/src/battle_log/mod.rs` (redact_request_body、SECRET_KEYS ~:388-413)、`src/diagnostics.ts` (:14-19)
- **手順**:
  1. `diagnostics.rs` に `pub const SECRET_KEYS: &[&str]` を定義 (現9キー: api_token, authorization, password, client_secret, access_token, refresh_token, cookie, rpctoken, st)
  2. `battle_log::redact_request_body` のローカルリストを削除し `diagnostics::SECRET_KEYS` を参照 (rpctoken/st が自動補完される)
  3. Rust側にテスト追加: SECRET_KEYS 全キーが redact_request_body で実際にマスクされること
  4. `diagnostics.ts` のregexキーと SECRET_KEYS の一致を確認し、両ファイルに相互参照コメントを付ける
- **受入基準**: Rust内のキーリスト定義が1箇所。`rpctoken=xxx`/`st=xxx` を含むリクエストボディが raw_api 保存時にマスクされるテストがpass
- **検証**: 標準3点

### W1-3 diagnostics.ts のIPCバッチング

**実施済み (2026-07-15)**: 100ms/64件バッチ、エラー即時送信、`beforeunload` best-effort flushを実装。
- **目的**: console呼び出し毎に個別 `invoke("log_frontend_event")` が飛ぶ構造を、バッファリングでIPC増幅を抑止
- **対象**: `src/diagnostics.ts`、必要なら `src-tauri/src/commands.rs` の `log_frontend_event` / `src-tauri/src/diagnostics.rs` の `frontend_event`
- **手順**:
  1. diagnostics.ts にバッファ (string[]) + 100ms debounce flush + 上限 (例: 64行 or 64KB で即flush) を実装
  2. `log_frontend_event` を複数行受け付け可能に (`lines: Vec<String>` 版コマンドを追加するか、既存を配列対応に変更。**フロントとRustを同一コミットで整合させる**)
  3. `beforeunload` で同期的に可能な範囲で flush (取りこぼしは許容、コメントで明記)
- **受入基準**: 連続 console.log 10回で invoke が1-2回に集約される (手動確認)。エラー経路 (window.error) は即時 flush
- **検証**: 標準3点 + DevTools console からの出力が `local/logs/session_*.log` に届くこと (📺)

### W1-4 ドキュメント整合一式
- **目的**: WIP 2機能のSPEC反映と stale 記載の解消。コード変更なし
- **対象と内容**:
  1. `docs/SPEC/diagnostics.md` **新規**: 4系統ログ (session/action/raw_api/click-screenshot) の責務・保存先・保持ポリシー・redact仕様・log_frontend_event 契約。README「診断ログ」節と整合させる
  2. `docs/SPEC/battle-log.md`: 退避艦追跡を追記 (escape_ship_ids の解決、pending → goback_port 確定フロー、大破警告抑止との関係、連合艦隊の6スロット規則)
  3. `docs/SPEC/SPEC.md`: 索引に diagnostics.md を追加
  4. `docs/CODEMAPS/backend.md`: モジュール数を22に修正、diagnostics 追加、「action_log (dev only)」→常時ONに修正
  5. `docs/CODEMAPS/architecture.md`: diagnostics.rs/.ts をモジュールマップに追加、行数更新
  6. `docs/CODEMAPS/dependencies.md`: `ideamans-hudsucker 0.25` (fork) に修正、vitest / tauri-plugin-dialog / windows-sys を追記
  7. `docs/SPEC/test-strategy.md`: テスト数 135→141 に更新
  8. `CAUTION.md`: game_init.js ポリシーを明文化 — 「DMMラッパーページ (トップフレーム) のレイアウト操作・広告非表示・レイアウト診断はOK / ゲームiframe内は見た目CSSのみ / ゲームstateの読取・関数呼出・自動化は禁止 / ゲームデータ捕捉は必ずproxy経由」。CA証明書パスが identifier ではなく `kancolle-browser` リテラルである点 (`proxy/mod.rs` ca_data_dir ~:298-307) を注記し、**パス変更は既存ユーザーの証明書再インストールを招くため禁止**と明記
- **受入基準**: 上記8点が反映され、SPEC.md 索引から全ドキュメントが辿れる
- **検証**: ドキュメントのみ (テスト不要)。コミットは `docs:`

---

## 4. Phase 2: プロセス基盤 — 安全網

### W2-1 CIワークフロー追加
- **目的**: テスト・型・lint を PR/push でゲートする (現状 release.yml のみでテストは一度もCIで走らない)
- **対象**: `.github/workflows/ci.yml` **新規**、`rustfmt.toml`・`clippy.toml` (デフォルト設定で新規、変更検知の基準固定用)
- **手順**: windows-latest 1本で: checkout → Node 22 + Rust stable セットアップ → `npm ci` → `npx tsc --noEmit` → `npx vitest run` → `cargo test --manifest-path src-tauri/Cargo.toml` → `cargo clippy --manifest-path src-tauri/Cargo.toml --lib -- -D warnings`。Rust キャッシュ (Swatinem/rust-cache) 推奨
- **受入基準**: push で CI が走り green。所要 <15分
- **検証**: ブランチに push して Actions 確認 (コミットは develop 直で可、Actions の結果をユーザーに報告)
- **リスク**: windows runner のビルド時間。キャッシュで緩和

### W2-2 ESLint導入
- **目的**: lint 基盤ゼロの解消。特に react-hooks/exhaustive-deps で hook 依存の安全網を得る
- **対象**: `eslint.config.js` **新規** (flat config)、`package.json` (devDeps: eslint, typescript-eslint, eslint-plugin-react-hooks / scripts: `"lint": "eslint src"`)、W2-1 の ci.yml に組込み
- **手順**: recommended + react-hooks 構成で導入 → `npm run lint` の既存違反を解消 (機械的修正のみ。`no-console` は diagnostics がconsoleを捕捉・転送する設計のため **warn 留め**とし、設定にコメントで理由を記す)
- **受入基準**: `npm run lint` エラー0。CI に lint ステップ追加済み
- **検証**: 標準3点 + lint
- **リスク**: exhaustive-deps が既存の意図的な依存省略 (doCheckRef パターン等) を警告する場合、**ロジックを変えず** ref パターンを維持し、個別行の disable コメント+理由で対応

### W2-3 コンポーネントテスト基盤
- **目的**: React コンポーネント/フックをテスト可能にする (Phase 4 の App.tsx 解体の安全網)
- **対象**: `package.json` (devDeps: @testing-library/react, @testing-library/jest-dom, jsdom)、`vite.config.ts` or `vitest.config.ts` (environment: jsdom, setupFiles)、`src/test/setup.ts` **新規**
- **手順**: vitest の jsdom 環境設定 → `@tauri-apps/api` の invoke/listen モックヘルパーを `src/test/tauri-mock.ts` に用意 → 代表テスト2-3本 (例: `HpBar` の色分岐、`ClearButton` の確認フロー) で配線を確認
- **受入基準**: `npx vitest run` で既存45 + 新規テストが green。tauri API がテストで安全にモックできる
- **検証**: 標準3点

---

## 5. Phase 3: Rust構造分割 (機械的・テストが守る)

### W3-1 ウィンドウトグル統合
- **目的**: `kantai.rs`(52L) と `management.rs`(52L) は LABEL 以外バイト一致、`quests/mod.rs`(31L) は第3の方言 (async / get_webview_window / is_visible().unwrap_or / action_log なし)。3実装を1つに
- **対象**: `src-tauri/src/{kantai.rs, management.rs, quests/mod.rs}` → `src-tauri/src/window_toggle.rs` **新規**、`src-tauri/src/lib.rs` (:382-421 close-intercept 3連、mod宣言、generate_handler)
- **手順**:
  1. `window_toggle.rs` に汎用 `fn show(app,label)/hide(app,label)/toggle(app,label)` を実装 (実装は management.rs 版をベースに統一。quests も同挙動にし、action_log::record も3窓共通で記録)
  2. 既存9コマンド (show/hide/toggle × management/kantai/quests) は window_toggle.rs 内の薄いラッパとして残す (**コマンド名変更禁止** — フロント互換)
  3. 旧3ファイル削除、lib.rs の mod / generate_handler を更新
  4. lib.rs の close-intercept 3連を `["management","kantai","quests"]` ループに置換
- **受入基準**: 3ファイル→1ファイル。9コマンド名は不変。quests 窓もトグルが action_log に記録される
- **検証**: 標準3点 + 手動smoke: コントロールバー 📊⚓📜 で3窓の開閉 (📺)
- **リスク**: quests のみ `get_webview_window` だった差異 — 統一実装が3窓で動くことをsmoke必須

### W3-2 設定永続化ヘルパー
- **目的**: 「AtomicBool + `local/<name>` に "0"/"1"」ブロックが5箇所 (overlay.rs :14-24/:47-53/:70-79/:196-198、game_window.rs :416-421)、復元が lib.rs :306-374 に7連。共通化
- **対象**: `src-tauri/src/settings.rs` **新規**、`overlay.rs`、`game_window.rs`、`lib.rs`
- **手順**:
  1. `settings.rs` に `persist_flag(app, name, value: bool)` / `restore_flag(local_dir, name, default: bool) -> bool` と、JSON版 `persist_json<T: Serialize>` / `restore_json<T: DeserializeOwned>` を実装
  2. 5箇所の書込ブロックと lib.rs の復元ブロックを置換。**ファイル名・フォーマット ("0"/"1") は不変** (既存ユーザーの設定を壊さない)
- **受入基準**: フラグ永続化ロジックの重複0。既存の設定ファイルがそのまま読める
- **検証**: 標準3点 + 手動smoke: ミュート/ミニマップ等をトグル→再起動→状態維持 (📺)

### W3-3 paths.rs 新設
- **目的**: `data_dir.join("sync"/"local"/...)` の手組みが74箇所/15ファイル (drive_sync/engine 19、migration 15、lib 11、overlay 6…) に散在
- **対象**: `src-tauri/src/paths.rs` **新規**、利用側15ファイル
- **手順**:
  1. `paths.rs` に `sync_dir(data_dir)`, `local_dir(data_dir)`, `logs_dir`, `action_logs_dir`, `cache_dir`, `battle_logs_dir`, `raw_api_dir`, 個別ファイル (`quest_progress_path` 等) を関数として定義
  2. 74箇所を段階置換 (ファイル単位でコンパイル確認しながら)
  3. **CA証明書パス (`proxy/mod.rs` ca_data_dir) は現状維持** — paths.rs に `ca_dir()` として移すのは可だが、返すパス自体 (`kancolle-browser` リテラル) は変更禁止 (W1-4 の CAUTION 注記参照)
- **受入基準**: `grep -rn '\.join("sync")\|\.join("local")' src-tauri/src` が paths.rs 内のみ。生成されるパスは全て従前と同一
- **検証**: 標準3点 + 起動して各ログ/キャッシュが従来の場所に書かれること (📺)

### W3-4 イベント名レジストリ
- **目的**: ~20イベント名が Rust ~40 emit箇所 / TS 19 listen箇所で裸文字列。3種 (screen-changed / fleet-view-changed / quest-filters-changed) は2箇所からemitされ、リネームがコンパイラの守りなしに4+ファイルに波及する
- **対象**: `src-tauri/src/events.rs` **新規**、emit側 (api/mod.rs, mouse_hook.rs, overlay.rs, drive_sync/engine.rs, proxy/mod.rs, lib.rs 等)、`src/constants.ts` (EVENTS 追加)、listen側4ファイル (App.tsx, DebugTab.tsx, KantaiView.tsx, SortieQuestChecker.tsx)
- **手順**:
  1. `events.rs` に `pub const PORT_DATA: &str = "port-data";` 形式で全イベント定数を定義 (イベント名の値は不変)
  2. Rust側 emit を定数参照に置換
  3. `src/constants.ts` に `export const EVENTS = { PORT_DATA: "port-data", ... } as const;` を追加し、listen側を置換
  4. **dead emit 削除**: `master-data-loaded` (api/mod.rs ~:1398) はフロントにリスナーが存在しないため emit ごと削除
- **受入基準**: 裸のイベント名文字列リテラルが Rust では events.rs、TS では constants.ts のみ。値の変更なし (git diff で文字列値が変わっていないこと)
- **検証**: 標準3点 + 手動smoke: 母港データ更新・Debugタブのクリック/画面イベント表示 (📺)

### W3-5 commands.rs 分割
- **目的**: 1039L の god module をドメイン別に分割
- **対象**: `src-tauri/src/commands.rs` → `src-tauri/src/commands/{mod,drive,cache,lists,quest,checkers,debug}.rs`
- **手順**:
  1. `commands/` ディレクトリ化し、mod.rs で `pub use` 再エクスポート (**lib.rs の generate_handler は無変更**)
  2. 移動: drive.rs ← drive_login/logout/get_drive_status/drive_force_sync (~:900-977) / cache.rs ← reset_browser_data, get_cached_resource, clear_resource_cache, clear_browser_cache, get_map_sprite (~:369-804) / lists.rs ← get_ship_list, get_equipment_list, get_improvement_list (~:188-316) / quest.rs ← get/update/clear_quest_progress (~:835-894), get_active_quest_ids / checkers.rs ← check_expedition_cmd, check_sortie_quest_cmd, check_map_recommendation_cmd (~:44-186) / debug.rs ← log_frontend_event, get_action_log, get_current_screen, get_current_fleet, get_quest_filters / 残り (get_proxy_port, raw_api系, cookie系, expedition/sortie_quest取得系) は mod.rs か適切なファイルへ
  3. checkers.rs の3コマンドが持つ艦隊index検証+艦データ構築の~40行重複を `fn build_fleet_check_data(inner, fleet_index)` に抽出
- **受入基準**: 各ファイル <400L。コマンド名・シグネチャ不変。generate_handler 無変更。重複40行×3 → ヘルパー1つ
- **検証**: 標準3点 + 手動smoke: 各タブのデータ表示・チェッカー実行 (📺)

### W3-6 api/mod.rs 分割① (screen / port / quest)
- **目的**: 1648L の解体第一段。自己完結した関心事を先に切り出す
- **対象**: `src-tauri/src/api/mod.rs` → `api/screen.rs`, `api/port.rs`, `api/quest.rs` **新規**
- **手順**:
  1. `api/screen.rs` ← `screen_from_api` / `screen_has_fleet_tabs` / `update_screen_from_api` (~:148-282)。mouse_hook からの参照パスを更新
  2. `api/port.rs` ← `process_port` (~:1410-1594) + `process_start2` (~:1316-1407) + `get_material` (~:44-50、テストから参照されるため pub(crate) 維持)
  3. `api/quest.rs` ← `process_questlist` (~:1599-1648) + `extract_senka_from_clearitemget` (~:1272-1313) + QuestStart/Stop/Clear のハンドラ処理 (~:981-1032 のmatch腕から呼ぶ関数として抽出)
  4. mod.rs に `pub(crate) mod screen; mod port; mod quest;` を追加、ディスパッチャの呼び出し先を更新
- **受入基準**: api/mod.rs が ~700L 以下に減少。api::tests の 141 テストが `use` パス修正のみで green
- **検証**: 標準3点

### W3-7 api/mod.rs 分割② (parse.rs + 汎用パーサ)
- **目的**: 残る「エンドポイント→ParsedApi」540行match (~25回の同一スケルトン) を集約し、mod.rs を <300L のディスパッチャに
- **対象**: `src-tauri/src/api/mod.rs` → `api/parse.rs` **新規**
- **手順**:
  1. `api/parse.rs` へ移動: `ParsedApi` enum (~:55-138、`pub(crate)` 化) + パースmatch全体 (~:296-835) を `pub(crate) fn parse(endpoint, request_body, json_str) -> ParsedApi` として抽出
  2. 汎用ヘルパー `fn parse_api<T: DeserializeOwned>(json_str: &str, ctor: impl FnOnce(T) -> ParsedApi) -> ParsedApi` を定義し、同一スケルトン~25腕を置換 (エラーログの文言・levelは既存踏襲)
  3. `variant_name!` マクロ (~:878-898) を `impl ParsedApi { fn variant_name(&self) -> &'static str }` メソッドに変換 (マクロ削除)
  4. mod.rs は「parse呼び出し → 非同期タスク → ハンドラmatch (サブモジュール委譲)」のみに
- **受入基準**: api/mod.rs <300L。parse.rs <600L。全テスト green (アサーション変更なし)
- **検証**: 標準3点 + 手動smoke: ゲームプレイでAPI処理 (母港/編成/任務) が正常 (📺)
- **リスク**: enum可視性変更に伴う参照調整。`#[allow(clippy::large_enum_variant)]` は enum とともに移動

### W3-8 AppState サブ構造体化
- **目的**: lib.rs :45-73 の18フィールド平置き (Mutex/AtomicBool混在) を、GameStateInner と同様のグループ構造に
- **対象**: `src-tauri/src/lib.rs`、AppState 利用側 (overlay.rs, mouse_hook.rs, commands/, game_window.rs, api/)
- **手順**: `OverlayPrefs` (formation_hint/taiha_alert/minimap/battle_info の enabled 群 + expedition_notify_visible)、`MinimapState` (position/size)、`ScreenTracking` (current_screen/current_fleet)、`QuestFilters` (period/category) にグループ化。`proxy_port`/`game_muted`/`game_zoom`/`formation_hint_rect` は直下維持で可。フィールドアクセスを一括置換
- **受入基準**: AppState 直下フィールド ≤8。同期プリミティブの種類は変えない (AtomicBool は AtomicBool のまま)
- **検証**: 標準3点 + 手動smoke: オーバーレイ設定トグル (📺)

### W3-9 game_window.rs 分割 + 座標定数の一元化 📺

**実施済み (2026-07-15)**: `game_window/{mod,platform,windows,macos}.rs` に分割し、`open_game_window` を20行の共通フローへ縮小。
- **目的**: `open_game_window` ~300行 (:25-328) の解体と、game_init.js に二重管理されている座標 (top:28px / 1200 / 720) のRust定数一元化
- **対象**: `src-tauri/src/game_window.rs` → `game_window/{mod.rs, windows.rs, macos.rs}`、`src-tauri/src/game_init.js`
- **手順**:
  1. オーバーレイ子window builder 3個 (formation-hint ~:158-182 / battle-info ~:186-220 / expedition-notify ~:226-252) を `fn build_overlay_windows(app)` 等に抽出
  2. プラットフォーム分岐 (macOS: proxy_url+data_store_identifier / Win: data_directory+additional_browser_args ~:109-136、Winマウスフック ~:304-324、mute実装 ~:408-458) を `game_window/windows.rs` / `game_window/macos.rs` へ
  3. game_init.js の `28px`/`1200`/`720` ハードコードを `__KC_CONTROL_BAR__`/`__KC_GAME_W__`/`__KC_GAME_H__` プレースホルダにし、Rust側で `include_str!` 後に `GAME_WIDTH`/`GAME_HEIGHT`/`CONTROL_BAR_HEIGHT` (:12-15) から `.replace()` 注入
  4. **注意**: CAUTION.md の規約 — `additional_browser_args` 使用時は必ず `--proxy-server` を自前指定 (既存実装を変えない)
- **受入基準**: open_game_window <100L。game_init.js 内に数値座標のハードコードなし。mac/win 分岐がインラインに残らない
- **検証**: 標準3点 + 手動smoke必須: ゲーム起動・ズーム・ミュート・オーバーレイ位置 (📺)
- **リスク**: macOS側はWinマシンでコンパイル確認不可 → `cargo check --target aarch64-apple-darwin` は不要、cfg分岐の機械移動に留め、マクロ/属性を変更しない

### W3-10 lock().unwrap() ポリシー統一 (任意・後回し可)
- **目的**: `std::sync::Mutex` の `.lock().unwrap()` 43箇所は、panic時のロック毒化で連鎖crashする
- **対象**: mouse_hook.rs (10)、overlay.rs (10)、commands/ (5)、api/mod.rs (4)、proxy/mod.rs (3)、lib.rs (3) 他
- **手順**: `fn lock_or_recover<T>(m: &Mutex<T>) -> MutexGuard<T>` (poison時は `into_inner` で回復しwarnログ) を settings.rs か新規 sync_util.rs に定義し、UI状態系の43箇所を置換。プロキシの `request_data` も含む
- **受入基準**: `.lock().unwrap()` が0 (テスト除く)。panic連鎖の可能性が解消
- **検証**: 標準3点

---

## 6. Phase 4: フロントエンド構造

### W4-1 src/api.ts 型付きinvokeレイヤー
- **目的**: 生 `invoke("...")` 36種/38箇所/14ファイルを型付き関数に集約 (Rust側67コマンドとの契約を1ファイルに可視化)
- **対象**: `src/api.ts` **新規**、invoke利用14ファイル、`src/components/common/ClearButton.tsx`
- **手順**:
  1. `api.ts` に全コマンドのラッパーを定義: `export function getQuestProgress(): Promise<QuestProgressSummary[]> { return invoke("get_quest_progress"); }` 形式。引数は camelCase (Tauri の暗黙変換) を維持し、Rust側 snake_case との対応をファイル冒頭コメントで明記
  2. 全コンポーネントの `invoke(` 直呼びを api.ts 経由に置換
  3. ClearButton はコマンド文字列 prop (`command: string`) を廃止し `onClear: () => Promise<string>` callback に変更。SettingsTab の8個所は `() => api.clearBattleLogs()` 形式で渡す (参照検索可能に)
- **受入基準**: `grep -rn "invoke(" src --include="*.tsx"` が 0 (api.ts と diagnostics.ts のみ許可)。tsc green
- **検証**: 標準3点 + 手動smoke: 設定タブの各クリアボタン (📺)

### W4-2 useTauriEvent hook
- **目的**: `listen → unlisten.then(f=>f())` 定型の21箇所 (App 11 / DebugTab 6 / KantaiView 2 / SortieQuestChecker 2) を hook 化
- **対象**: `src/hooks/useTauriEvent.ts` **新規**、上記4ファイル
- **手順**: `function useTauriEvent<T>(event: string, handler: (payload: T) => void)` を実装 (handler は ref 経由で最新を呼ぶ — 依存配列問題を回避)。EVENTS 定数 (W3-4) を併用して全 listen を置換
- **受入基準**: 生 `listen(` 呼びがコンポーネントから消える。リスナー数・イベント名は不変
- **検証**: 標準3点 + W2-3 のモックで hook のマウント/アンマウトテスト1本

### W4-3 データhooks抽出
- **目的**: App.tsx (440L、hook 33個) からデータ管理を分離。get_quest_progress→Map 構築の3重複 (:207/:234/:262) を解消
- **対象**: `src/hooks/{useNow.ts, usePortData.ts, useBattleLogs.ts, useQuestProgress.ts}` **新規**、`src/App.tsx`
- **手順**:
  1. `useNow(intervalMs=1000)`: 1Hz tick を隔離
  2. `usePortData()`: portData/portDataVersion/weaponIconSheet/senka + 関連リスナー (port-data, fleet-updated, senka-updated)
  3. `useBattleLogs()`: battleLogs/total/期間フィルタ + refreshBattleLogs + sortie-complete/sortie-update リスナー (:120-194)
  4. `useQuestProgress()`: questProgress Map + quest-list-updated/quest-progress-updated リスナー + Map構築の一元化
  5. App.tsx は hooks を呼んで配るだけに (この時点では props 配布構造は維持)
- **受入基準**: App.tsx <250L。同一イベントの listen 数が増えていない (usePortData を複数窓で使う設計は W4-4 で)
- **検証**: 標準3点 + hooks のユニットテスト (モックイベント発火→state反映) 各1本

### W4-4 per-window roots (App.tsx 解体) 📺
- **目的**: quests/kantai 窓でも management 用の全副作用 (11リスナー+1Hz+battle log取得+drive status) が走る構造を解消
- **対象**: `src/main.tsx`、`src/App.tsx` → `src/windows/{ManagementApp.tsx, KantaiApp.tsx, QuestsApp.tsx}` **新規**
- **手順**:
  1. VIEW_MODE 判定 (App.tsx :30-39) を main.tsx に移し、label で root コンポーネントを出し分け
  2. `QuestsApp`: useQuestProgress + sortieQuests/activeQuests 取得のみ mount → `<QuestTab>` (現App.tsx :298-305 の3props)
  3. `KantaiApp`: usePortData + useNow + 遠征/任務チェック関連 + air-base/fleet-view リスナー → `<KantaiView>` (現 :308-321 の9props)
  4. `ManagementApp`: 全hooks + タブUI (現 :324以降)
  5. `import "./diagnostics"` は main.tsx 先頭を維持 (順序重要)。App.tsx は削除
- **受入基準**: 各窓が自分の必要リスナーのみ登録 (DebugTab か diagnostics ログで確認可)。3窓の表示内容・挙動は従前と同一
- **検証**: 標準3点 + コンポーネントテスト (各Appがcrashなくmountし主要子が出る) + 手動smoke必須: 3窓すべての表示・更新 (📺)
- **リスク**: 本計画で最も回帰リスクが高い項目。W4-3 完了と W2-3 基盤を前提に着手。1コミットでrevert可能に

### W4-5 再レンダ抑制 (memo + Context)
- **目的**: 1Hz tick で全ツリー再レンダされる構造 (memo/Context 利用0) の解消。HomeportTab 13props / FleetPanel 10props のドリル解消
- **対象**: `src/contexts/PortDataContext.tsx` **新規**、FleetPanel、HomeportTab、KantaiView
- **手順**: PortDataProvider (portData/portDataVersion/weaponIconSheet) を ManagementApp/KantaiApp に配置 → 中間コンポーネントの転送専用 props を Context 消費に置換 → `React.memo(FleetPanel)` + now は useNow を末端 (残り時間表示コンポーネント) で直接呼ぶ形に変更
- **受入基準**: HomeportTab の props ≤6。1Hz tick での再レンダが残り時間表示系に限定される (React DevTools Profiler で確認、結果をSESSION.mdに記録)
- **検証**: 標準3点 + 手動smoke: 母港/艦隊窓の表示・遠征タイマー進行 (📺)

### W4-6 Checker 3兄弟の統合
- **目的**: Expedition/MapRecommendation/SortieQuest の3チェッカーが複製する「localStorage選択 + doCheck + doCheckRef + auto-check effect」と、条件行 5-span markup の4コピーを統合
- **対象**: `src/hooks/useChecker.ts` **新規**、`src/components/common/ConditionList.tsx` **新規**、3チェッカー (ExpeditionChecker.tsx, MapRecommendationChecker.tsx, SortieQuestChecker.tsx :265-272/:317-324)
- **手順**:
  1. `useChecker<TResult>({ storageKey, check: () => Promise<TResult>, autoCheckDeps })` を実装 (checking/result state + doCheckRef パターン内蔵)
  2. `<ConditionList conditions={ConditionResult[]}>` に 5-span 行レンダを共通化 (className は既存 `.exp-cond` 系を維持 — CSS変更なし)
  3. 3チェッカーを置換。SortieQuestChecker は quest選択・進捗更新など固有部分を残す
- **受入基準**: doCheck/auto-check 定型の重複0。条件行 markup 定義が1箇所。表示は従前と同一
- **検証**: 標準3点 + コンポーネントテスト (ConditionList の ok/ng 表示) + 手動smoke: 3チェッカー動作 (📺)

### W4-7 リスト系・小物の重複解消
- **目的**: type filter ロジック3重複 (ShipListTab :25-34,:48-61 / EquipListTab :23-32,:34-47 / ImprovementTab :22-31,:55-71)、questId→apiNo 解決の逐語重複 (SortieQuestChecker :178-188 / QuestProgressDisplay :17-27)、日付ロジックの再実装 (App.tsx :58-66 / BattleTab :57 — format.ts に既存)
- **対象**: `src/hooks/useTypeFilter.ts` **新規**、`src/utils/quest.ts` **新規** (resolveApiNo)、utils/format.ts 利用側
- **手順**: useTypeFilter(storageKey) 抽出→3タブ適用 / resolveApiNo(questById, questId) 抽出→2箇所適用 / 日付インライン実装を toDateStr/daysInMonth 呼び出しに置換
- **受入基準**: 3重複・2重複・再実装が各1実装に。フィルタのlocalStorage互換維持 (キー不変)
- **検証**: 標準3点 + 手動smoke: 3リストタブのフィルタ動作 (📺)

### W4-8 定数・型整理
- **目的**: STORAGE_KEYS バイパス3件と型の重複/緩さの解消
- **対象**: `src/constants.ts`、`src/components/kantai/KantaiView.tsx` (:15-16)、`src/components/quests/QuestTab.tsx` (:85,:94)、`src/types/quest.ts` (:53)、`src/types/port.ts` (:34-35)、`src/types/battle.ts` (:49-56)
- **手順**:
  1. STORAGE_KEYS に `KANTAI_FLEET_ID: "kc-kantai-fleet-id"`, `KANTAI_UI_ZOOM: "kc-kantai-ui-zoom"`, `PINNED_QUESTS: "pinned_quests"` を追加し参照置換 (**キー値は不変** — 既存ユーザーのlocalStorage維持。UIズーム2系統はキー統合しない)
  2. `AreaProgress` を named interface 化し quest.ts と SortieQuestChecker のインライン重複宣言を置換
  3. `FleetData.mission?: unknown[]` / `ship_ids?` と BattleNode の legacy フィールド群: 実データ (battle_logs 保存形式) と突合し、**読み込み互換に必要なら型コメントで理由明記、不要なら削除**。判断がつかない場合は削除せず SESSION.md に記録
- **受入基準**: localStorage キー直書き0。同一shape の重複型宣言0
- **検証**: 標準3点 + 手動smoke: 艦隊窓のズーム/選択・任務ピンの永続化 (📺)

---

## 7. Phase 5: CSS設計

### W5-1 デザイントークン導入
- **目的**: CSS変数0、ダークテーマ7色が165回/15ファイルに直書き (全hexは361箇所)、color.ts にも重複
- **対象**: `src/styles/tokens.css` **新規** (main.tsx で import)、全16 CSSファイル、`src/utils/color.ts`
- **手順**:
  1. `:root { --bg-0:#1a1a2e; --bg-1:#16213e; --bg-2:#0f3460; --accent:#e94560; --text:#e0e0e0; --ok:#4caf50; --info:#4fc3f7; ... }` を定義 (実際の値は現CSSから正確に収集)
  2. パレット7色+状態色 (#f44336/#ffeb3b 等) の直書きを `var(--...)` に機械置換
  3. color.ts の TS側色定数にトークン名の対応コメントを付与 (TSは実行時にhexが必要なため値は残す)
- **受入基準**: パレット色のhex直書きが tokens.css 以外で0。見た目の変化なし (置換前後スクリーンショット比較)
- **検証**: 標準3点 + 手動smoke: 全タブ+3窓の見た目確認 (📺)

### W5-2 共有クラス整理
- **目的**: ファイル跨ぎで使われるクラス (.no-data は App.css 定義で9コンポーネント使用 / .exp-cond, .checking は ExpeditionChecker.css 定義で3チェッカー使用) の暗黙結合を解消。BattleDetailView.css 471L の分割
- **対象**: `src/styles/shared.css` **新規**、App.css、ExpeditionChecker.css、BattleDetailView.css
- **手順**: 共有クラスを shared.css へ移動 (セレクタ・宣言は不変) + 利用元コンポーネント一覧をコメント記載 / BattleDetailView.css をコンポーネント対応 (BattleNodeDetail/MapRouteView) で分割
- **受入基準**: 共有クラスの定義場所が shared.css に集約。見た目の変化なし
- **検証**: 標準3点 + 手動smoke (📺)

---

## 8. Phase 6: セキュリティ・深部

### W6-1 OAuth secret の option_env! 化
- **目的**: `drive_sync/auth.rs:35-36` に Google OAuth client secret がリテラルで残存 (リポジトリ唯一のハードコード秘密)
- **対象**: `src-tauri/src/drive_sync/auth.rs`、`docs/SPEC/drive-sync.md`
- **手順**: `const GOOGLE_CLIENT_SECRET: &str = match option_env!("KC_GOOGLE_CLIENT_SECRET") { Some(v) => v, None => "<現行値>" };` 形式でビルド時上書き可能に (デフォルトは現行値でビルド互換維持)。drive-sync.md にローテーション手順 (GCP Console での再発行 → env 設定 → リビルド) と「Googleはデスクトップアプリの client secret を機密と扱わない」公式見解を記載
- **受入基準**: env でビルド時に差し替え可能。既定ビルドの挙動不変
- **検証**: 標準3点 + Drive同期ログイン動作 (📺)

### W6-2 game-controls capability 最小化 📺
- **目的**: `capabilities/game-controls.json` が game-content/game-overlay (DMM/ゲームを表示するwebview) に `core:default` を付与しており、ゲームページから任意の登録コマンドを invoke できる。実使用コマンドのみに縮小
- **対象**: `src-tauri/capabilities/game-controls.json`
- **手順**:
  1. game_init.js と overlay 系JSが実際に invoke するコマンドを列挙 (`grep -n "invoke(" src-tauri/src/game_init.js src-tauri/src/overlay.rs` 等で確認: set_game_zoom / toggle_game_mute / get_game_mute / show_*_window系 / log_frontend_event / dismiss_overlay 等)
  2. `core:default` を外し、`core:event:default` + 列挙コマンドの permission (`"identifier": "core:invoke"` 相当の個別 allow) に置換 (Tauri v2 の command scope 記法は公式docsで確認)
  3. **手動smoke必須**: コントロールバー全ボタン (ズーム/ミュート/📊⚓📜/オーバーレイ操作)、diagnostics のフロントログ到達
- **受入基準**: game系webviewから実使用外のコマンドが invoke 不能。全コントロールバー機能が動作
- **検証**: 標準3点 + 手動smoke必須 (📺)。動かない場合は revert して SESSION.md へ

### W6-3 air_corps の typed DTO 化
- **目的**: 基地航空隊サブシステム全体が生 `serde_json::Value` 掘り45箇所で、DTO層 (dto/member.rs 等) と不整合
- **対象**: `src-tauri/src/api/dto/air_corps.rs` **新規**、`src-tauri/src/api/air_corps.rs`、`api/parse.rs` (ParsedApi の air_corps 系 variant を型付きに)、`api/tests.rs`
- **手順**: 既存 fixture (`src-tauri/tests/fixtures/samples/` の air_corps 系) から `#[serde(flatten)] extra` 付きDTOを定義 (CAUTION.md 規約) → parse.rs の variant を Value → DTO に変更 → air_corps.rs のハンドラを DTO ベースに書換え → デシリアライゼーションテストを追加
- **受入基準**: air_corps.rs の `["api_..."]` インデックスが0。基地航空隊の追跡挙動不変 (既存テスト+新テスト green)
- **検証**: 標準3点 + 手動smoke: 基地航空隊タブ表示 (📺)

### W6-4 dto/battle.rs の未使用 struct 検証
- **目的**: `dto/battle.rs` に構造体丸ごと `#[allow(dead_code)]` が4件 (:5,:34,:44,:52) あり、battle.rs は生Value解析 (8箇所) のため未使用の疑い
- **対象**: `src-tauri/src/api/dto/battle.rs`
- **手順**: 各structの参照を検索 → テストのみが使う場合はテスト資産として維持するか判断 (デシリアライズ検証として価値があれば `#[cfg(test)]` 相当に注記) → 完全未使用なら削除。battle.rs 本体の生Value解析をDTO化するのは**本項目のスコープ外** (将来候補としてSESSION.mdに記録)
- **受入基準**: dead_code allow の理由が「schema記録」「テスト用」など明確化されるか、削除される
- **検証**: 標準3点

---

## 9. Phase 7: 仕上げ

### W7-1 ドキュメント最終整合
- **対象**: `docs/CODEMAPS/*` 全5ファイル (分割後のモジュール構成・行数で再生成)、`docs/SPEC/architecture.md`/`frontend.md`/`api-intercept.md` (新モジュール構成反映)、`SESSION.md` (完了項目の削除)、メモリ更新
- **受入基準**: CODEMAPS のモジュールマップが実ファイル構成と一致。完了メトリクス (下記) の達成値を計測して記録
- **検証**: ドキュメントのみ

---

## 10. 完了メトリクス

| メトリクス | 現状 (7cb0965) | 目標 |
|-----------|---------------|------|
| 非テストRustファイル 800L超 | 4 (api/mod 1648, commands 1039, senka 929, quest_progress 842) | 0 ※senka/quest_progress は分割対象外のため例外可、ただし api/mod と commands は必須 |
| api/mod.rs | 1648L | <300L |
| 生 invoke 文字列 (TSX内) | 38箇所 | 0 (api.ts 経由) |
| イベント名リテラル | Rust ~40 / TS 19箇所 | events.rs / constants.ts のみ |
| パレット色 hex直書き | 165箇所 | 0 (tokens.css のみ) |
| redact キーリスト実装 | 3 (不一致) | Rust 1 + TS 1 (テストで同期) |
| `.lock().unwrap()` | 43 | 0 (W3-10実施時) |
| CI テストゲート | なし | push/PR で tsc+vitest+cargo test+clippy |
| cargo test / vitest | 141 / 45 | 維持または増加、全green |
| real_session_* テスト (ui_event) | green | 無傷で green |

※ senka/mod.rs (929L)・quest_progress/mod.rs (842L) の分割は本計画に含めない (単一ドメインで凝集しており、機械分割の利益が薄い)。800L超過は許容し、次回機能追加時に検討。

## 11. Out of scope

- 機能追加・挙動変更全般 (診断ログ/退避艦追跡の機能拡張を含む)
- SESSION.md 記載の実機確認タスク (戦果動作確認 / Mac自動ログイン / 陣形ヒントzoomずれ / 任務237 OR判定)
- api/tests.rs (1764L) の分割 (テストは行数規約の対象外とする)
- Windows fleet panel 表示バグ (既知バグ、別トラック)
- ts-rs / specta による Rust→TS 型自動生成 (将来候補。W4-1 の api.ts が土台になる)
- CSP null の見直し (ゲーム互換のため現状維持、SPEC/architecture.md §10 記載済み)
- senka / quest_progress / battle_log/parser の内部分割 (凝集済み単一ドメイン)

## 12. 追補 (2026-07-15 実行後の再監査フォローアップ)

Phase 1/3/6 相当の実行 (`2640ab8`〜`e981c18`) 後の再監査で確定した追加項目。
実行済み範囲: api分割 (mod 99L + parse/dispatch/port/quest/screen)、window_toggle/settings/paths/events、AppStateサブ構造体化、game_window platform分割、air_corps DTO化、OAuth option_env!化、sensitive-keys.json一元化、log_io.rs (バッファ書込+250ms周期flush+保持共通化)。品質ゲート4点 (cargo test 147 / vitest 45 / tsc / clippy -D warnings) green。

### W1-5 flush_all() によるクラッシュ時のaction_log取りこぼし解消 (推奨・小)

**実施済み (2026-07-15)**: 全sinkを `log_io::flush_all()` に集約し、周期/panic/shutdown経路を統一。API/API_PARSED action logもdebug限定化。

- **目的**: panicフックと shutdown() が `log::logger().flush()` のみで `action_log::flush()` を呼ばず、クラッシュ直前の行動ログ (最大64KB/250ms分) が失われる
- **対象**: `src-tauri/src/log_io.rs` (周期スレッド ~:86-97)、`src-tauri/src/diagnostics.rs` (panicフック ~:107、shutdown ~:162)
- **手順**: `log_io::flush_all()` を新設 (`log::logger().flush()` + `action_log::flush()` を集約、将来のsinkもここへ) → 周期スレッド/panicフック/shutdown の3呼び出し元を flush_all() に統一
- **受入基準**: panic後にセッションログと `actions_YYYYMMDD.jsonl` の双方へ直前行が残る (テスト: バッファ未満の行 → flush_all() → ファイル内容確認)。flush対象の列挙が1箇所
- **検証**: 標準3点

### W3-11 api/models.rs (728L) のカテゴリ分割 (任意)

**実施済み (2026-07-15)**: `models/{mod,wire,summary,air_base}.rs` へ分割し、全型を再export。formation memory I/Oは `formation.rs` へ移動。

- **目的**: ドメイン状態 / 入力DTO (Deserialize) / 出力View (Serialize) の3責務が1ファイルに同居
- **対象**: `src-tauri/src/api/models.rs` → `api/models/{mod,wire,summary,air_base}.rs`
- **手順** (第1段・低リスク): mod.rs = 状態コア (GameStateInner等 ~250L) + 全型の `pub use` 再公開 / wire.rs = 入力Deserialize (ApiResponse, start2系, port系 ~200L) / summary.rs = 出力Serialize (PortSummary, ShipListItem等 ~230L) / air_base.rs = AirBase系4型 (~75L)。`load/save_formation_memory` は api/formation.rs へ。**re-export により外部参照の変更ゼロ**
- **受入基準**: `models::<型>` の外部参照がコンパイル無変更で通る。`_extra` 付き入力DTOと Serialize view型がファイル単位で分離
- **検証**: 標準3点
- 第2段 (wire.rs → api/dto/ への移送統一) は将来任意

### 見送り (再監査で妥当性確認済み)
- ログ3系統 (diagnostics/action_log/raw_api) のフォーマット層マージ — 用途が異なるため log_io 基盤共有まででよい
- raw_api の per-file 方式変更 — 耐クラッシュ性良好・replayツール前提のため現行維持 (保持上限は log_io で導入済み)
