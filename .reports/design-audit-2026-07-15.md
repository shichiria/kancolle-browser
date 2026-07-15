# 設計監査レポート (2026-07-15)

対象コミット: `7cb0965` (WIP含む作業ツリーを監査後にコミットしたもの)。
3系統の監査エージェント (Rustバックエンド / フロントエンド / 横断) の所見を収録。
本レポートは `docs/SPEC/refactor-design-cleanup.md` のエビデンス。行番号は監査時点の目安。

---

# Part 1: Rust バックエンド監査

対象: `src-tauri/`(非テスト33ファイル・約13,600行)。重要度: HIGH / MED / LOW。

## 環境メモ (生成物の場所)
- identifier: `com.eo.kancolle-browser` (`tauri.conf.json:5`)、Windows ベース: `%LOCALAPPDATA%\com.eo.kancolle-browser\`
- `local\logs\session_<日時>_<pid>.log` (diagnostics、90日/最大200) / `local\action_logs\actions_YYYYMMDD.jsonl` (常時ON化)
- `local\cache\` / `local\game-webview\EBWebView\` / `local\{game_muted,formation_hint_enabled,taiha_alert_enabled,minimap_enabled,battle_info_enabled}` ("0"/"1") / `local\minimap_{position,size}.json`
- `sync\raw_api\` (既定ON化) / `sync\battle_logs\` / `sync\improved_equipment.json` ほか / `google_drive_token.json` (ベース直下)
- ⚠ CA証明書のみ別フォルダ: `%LOCALAPPDATA%\kancolle-browser\ca.cert.pem` (identifierではなくリテラル。`proxy/mod.rs:298-307`)

## Findings

### 1. `process_api` が983行の神関数 — HIGH
`api/mod.rs:286-1269`。「エンドポイント→`ParsedApi` の540行パースmatch(296-835)」+「spawn非同期タスク内の366行ハンドラmatch(901-1267)」。`ParsedApi` enum ~40バリアント(55-138)。新規エンドポイントのたびに同関数2箇所+enum+`variant_name!`マクロ(885-898)を変更。肥大関数は他に `process_port`(1410-1594)、`process_start2`(1316-1407)。

### 2. 800行ハード上限の超過 — HIGH
違反(非テスト): `api/mod.rs` 1648、`commands.rs` 1039、`senka/mod.rs` 929、`quest_progress/mod.rs` 842。テスト `api/tests.rs` 1764。約15ファイルが400-800帯。

### 3. `game_init.js` が DMMページをDOM改変+DOM情報送信 — HIGH (→ユーザー容認済み 2026-07-15)
`game_window.rs:9` `include_str!`、`game_window.rs:92` で `game-content` webview に `initialization_script` 注入 (DMMオリジントップフレームで実行)。WIPで受動的CSS隠しから拡大:
- `reportLayout(stage)`: DMMのDOM (全iframe、`[id*=game]`、body children、location、viewport) を `invoke('log_frontend_event')` で送信 (DOMContentLoaded/+3s/+10s)
- `isolateGameFrame()`: `#game_frame` 祖先の兄弟を `display:none !important`、親の position/transform/z-index/overflow 書換、iframe固定 (fixed, 1200×720, z-index:10000)。MutationObserver + 2s×15 再適用
ゲームiframe内 (クロスオリジン) へは非侵入。JS内に `top:28px`/`1200px`/`720px` をハードコードし Rust `GAME_WIDTH/GAME_HEIGHT/CONTROL_BAR_HEIGHT` (`game_window.rs:12-15`) と二重化。
**判定 (横断監査)**: 「ゲームJS環境注入禁止」ルールの字義には準拠 (`if(isTop)` ガードでDMMラッパーのみ、サブフレームはCSSのみ、データ捕捉は100% proxy)。ユーザー決定: 容認+CAUTION.md明文化+座標一元化 (W1-4/W3-9)。

### 4. Google OAuth クライアントシークレットのハードコード — HIGH (セキュリティ)
`drive_sync/auth.rs:35-36`: `GOOGLE_CLIENT_SECRET = "GOCSPX-***"` (本レポートではマスク) がコンパイル埋め込み定数。リポジトリ全体で唯一のハードコード秘密。Googleはデスクトップapp secretを「機密でない」とするがgit履歴に残り、ローテーションに再コンパイル要。Tauri capabilities (default.json) は狭く良好。→ W6-1

### 5. 秘密情報レダクションが3重実装・キー不一致 — MED
`diagnostics.rs:179-238` (rpctoken/st含む) / `battle_log/mod.rs redact_request_body` (**rpctoken/st欠落**) / `src/diagnostics.ts:14-19`。生APIダンプがセッションログなら消すトークンを残しうる。→ W1-2

### 6. ウィンドウトグルモジュール3本 — MED (重複)
`kantai.rs`(52L)/`management.rs`(52L) は LABEL 以外バイト一致。`quests/mod.rs`(31L) は別方言 (async・get_webview_window・is_visible().unwrap_or・action_log無し)。close-intercept も `lib.rs:382-421` で3重。→ W3-1

### 7. 場当たり的な設定永続化 — MED
「AtomicBool + `local/<name>` に "0"/"1"」反復: `overlay.rs:14-24/47-53/70-79/196-198`、`game_window.rs:416-421`。復元は `lib.rs:306-374`。フラットフラグ / local JSON / sync JSON の3規約混在。→ W3-2

### 8. `AppState` が雑多構造体 — MED
`lib.rs:45-73`: 18フィールド (Mutex<u16>・AtomicBool×5・Mutex<Option>・Mutex<f64>・Mutex<(f64,f64)>・Mutex<Screen> 等) グルーピング無し。`GameStateInner` (`api/models.rs:161-190`: master/profile/sortie/history/senka) と対照的。→ W3-8

### 9. 型付きDTOと生`Value`の不整合 — MED
`["api_…"]`/`.get("api_…")` 出現: `air_corps.rs` **45**、`battle_log/parser.rs` 26、`api/mod.rs` 14、`battle.rs` 8、`battle_info.rs` 4 (計146、テスト37含む)。`ParsedApi` の air-corps系/`MapInfoData`/`BaseAirCorps` は生Value。パースmatchに同一骨格~25回。→ W3-7 (汎用parse_api) / W6-3 (air_corps DTO)

### 10. raw_api 既定ON化と `get_action_log` のrelease不整合 — MED
WIPで `raw_enabled` false→true、action_log 常時ON化 (7→90日、毎行flush)。しかし `commands.rs:980-995 get_action_log` は `#[cfg(debug_assertions)]` のままで release は空Vec → 書くのに閲覧不可の不整合。→ W1-1

### 11. 常時ONロギング3系統の責務重複 — MED
diagnostics.rs (セッションtext) / action_log.rs (JSONL) / raw_api ダンプ。全て毎write flush、高頻度API時にI/O増幅。`session_id` は共有済み。→ W1-3 (フロントIPC) + 将来の共有writer検討

### 12. イベント名が散在文字列・dead emit — MED/LOW
~20種を~40箇所で裸文字列emit。`screen-changed`/`fleet-view-changed`/`quest-filters-changed` は `api/mod.rs:254/262/275` と `mouse_hook.rs:309/317/331/402` の2系統からemit。`master-data-loaded` (emit `api/mod.rs:1398`) はフロントlisten無し = dead。→ W3-4

### 13. プラットフォーム `#[cfg]` インライン混在 — MED
密度: game_window 9、mouse_hook 7、ca 4、commands 3。最悪 `open_game_window`(25-328, ~300行)。`toggle_game_mute`(408-458) は objc2 / webview2_com をインライン。→ W3-9

### 14. `.lock().unwrap()` 43箇所 — LOW/MED
mouse_hook 10・overlay 10・commands 5・api/mod 4・proxy 3・lib 3 等。ロック保持中panicで毒化→連鎖crash。非テスト unwrap/expect 計~81 (低リスク: 静的URL parse、include_str! JSON expect)。`proxy/mod.rs:401 expect` はspawn内。→ W3-10

### 15. コマンドエラーが `Result<T,String>` — LOW
一貫しているが型なし。日本語文言と `e.to_string()` 混在。将来の thiserror 化候補 (本計画スコープ外)。

### 16. dead/スキーマ保持コード — LOW
`#[allow(dead_code)]` 20件 (多くはDTOフィールドの schema completeness 注記で妥当)。`dto/battle.rs` の構造体丸ごと4件 (5,34,44,52) は battle.rs が生Value解析のため未使用疑い。→ W6-4。TODO/FIXME/HACK は皆無 (良好)。

## Good patterns (維持・拡張)
- `GameStateInner` のサブ構造体分割 / 順序保証の2段API処理 (parse→spawn、ロック外I/O、`api/mod.rs:838-869`)
- 型DTO層 (`api/dto/*`) / プロキシのコネクション別隔離 (`proxy/mod.rs:28-31,86,102-108`) / MITM対象の限定
- ApiEvent の生ペイロード除去 (WIP、セキュリティ改善) / get_cached_resource のパストラバーサル防御 (`commands.rs:501-509`)
- diagnostics の panic hook・早期バッファ・レダクション・保持上限 / tests.rs 1764行 / 狭い capabilities

## Metrics
- **>400行**: api/tests.rs 1764(test)・api/mod.rs 1648・commands.rs 1039・senka 929・quest_progress 842・battle_log/mod 758・api/battle 752・api/models 728・sortie_quest 724・air_corps 648・battle_log/parser 634・ui_event/mod 627・mouse_hook 585・api/ship 533・drive_sync/engine 525・expedition 509・lib 502・game_window 464・overlay 441・proxy 429
- **>100行関数**: process_api ~983・open_game_window ~300・run setup closure ~215・process_port ~185・get_map_sprite ~127・reset_browser_data ~113
- **emit イベント名 (~20)**: proxy-ready, kancolle-api, port-data, master-data-loaded(dead), sortie-update, sortie-complete, senka-updated, fleet-updated, quest-list-updated, quest-started, quest-stopped, quest-progress-updated, quest-filters-changed(2箇所), screen-changed(2箇所), fleet-view-changed(2箇所), air-base-updated, drive-sync-status, drive-data-updated, click-event, click-screenshot

---

# Part 2: フロントエンド監査 (要点)

前提: any/@ts-ignore/React.FC ゼロ、リスナーcleanup完備、immutability遵守 — 型衛生は良好。問題は構造。

1. **App.tsx godコンポーネント — HIGH**: 20 useState + 6 useRef + 7 useEffect (hook 33)。VIEW_MODE分岐がrender時 (:298/:308/:324) のため quests/kantai 窓でも 1Hz interval (:86-89)、11リスナー (:149-296)、battle log取得、drive status 等が全て実行される。QuestTab が必要とするのは3propsのみ
2. **型付きAPIレイヤー不在 — HIGH**: src/api.ts/hooks/store/context 皆無。生 invoke 36種/38箇所/14ファイル vs Rust 67コマンド。ClearButton はコマンド名を data prop で受け (SettingsTab :187-223 の8個)、参照検索不可視。get_quest_progress 呼び出しが App.tsx :207/:234/:262 で3重複
3. **Context/memo ゼロ — MED/HIGH**: 1Hz tick が全ツリー再レンダ。now が4層ドリル (App→HomeportTab→FleetPanel→ExpeditionChecker)。HomeportTab 13props / FleetPanel 10props
4. **型ドリフト — MED**: データ型snake_case vs invoke引数camelCase (Tauri暗黙変換依存)。FleetData.mission?: unknown[]、BattleNode legacyフィールド、AreaProgress が quest.ts:53 と SortieQuestChecker.tsx:192 で二重インライン宣言
5. **複雑度ホットスポット — MED**: SortieQuestChecker.tsx 366L (fetch+変換+render+進捗更新)。MapRouteView.tsx の110行fetch effect (:12-126)。※CODEMAPS の「SettingsTab 463L」はフォルダ合計 (tsx 229+css 233) で単一ファイル問題なし
6. **localStorage — LOW/MED**: STORAGE_KEYS は概ね機能。バイパス3件: KantaiView.tsx:15-16 (kc-kantai-fleet-id / kc-kantai-ui-zoom)、QuestTab.tsx:85 (pinned_quests)。UIズーム2系統 (ui-zoom default135 / kc-kantai-ui-zoom default100) 併存
7. **CSS — MED**: CSS変数0。パレット7色が165回/15ファイル (全hex 361箇所)、color.ts にも重複。クラスのファイル跨ぎ結合: .exp-cond (定義ExpeditionChecker.css、使用SortieQuestChecker:266,318/MapRecommendationChecker:102)、.no-data (定義App.css:206、9コンポーネント使用)、.checking
8. **重複 — MED**: Checker3種が「localStorage選択+doCheck+doCheckRef+auto-check」複製、条件行5-span markup 4コピー (ExpeditionChecker:98-105 / MapRecommendationChecker:101-108 / SortieQuestChecker:265-272,317-324)。questId→apiNo が SortieQuestChecker:178-188 / QuestProgressDisplay:17-27 で逐語重複。type filter が Ship/Equip/Improvement 3タブで3重複。日付ロジックが App.tsx:58-66 / BattleTab.tsx:57 で format.ts (toDateStr/daysInMonth) を使わず再実装
9. **diagnostics.ts (WIP) — MED**: 設計は概ね健全 (originals保持・循環安全・無限ループなし)。懸念: redactキー2重 (→W1-2)、console毎個別IPC (→W1-3)。DebugTab/action_log とは補完関係
10. **テスト — MED**: vitest ^4.1.0、utils 2ファイル45テストのみ。@testing-library/jsdom なし → コンポーネント/フックはテスト不能。ESLint 皆無

イベントリスナー実態: 19種、App 11 / DebugTab 6 / KantaiView 2 / SortieQuestChecker 2。kancolle-api は App+DebugTab で二重バッファ。
FleetPanel の HomeportTab/KantaiView 共用は健全 (同一props、fleetIndex===0 の分岐のみ)。QuestTab と SortieQuestChecker は役割が異なり重複ではない。

良パターン: feature folder + barrel index / STORAGE_KEYS / utils分離 / listener cleanup完備 / doCheckRef stale-closure対策 / common/ primitives / immutability。

---

# Part 3: 横断監査 (要点)

## WIP実態 (→ 7cb0965 でコミット済み)
- **診断ログ基盤** (~13ファイル): diagnostics.rs 296L + diagnostics.ts 84L + action_log 常時ON化 + ApiEvent 縮小 + raw_api 既定ON + README「診断ログ」節。契約は端から端まで閉じ、テスト付き (+6)、警告0
- **退避艦追跡** (battle.rs +164L): escape_ship_ids() — battleresult の api_escape/api_tow_idx → pending → goback_port 確定で escaped_ship_ids。目的: 退避済み艦の大破警告抑止 (battle.rs:171-175)。連合艦隊6スロット規則対応。テスト3件

## ビルド/テスト健全性
cargo test --lib **141 passed / 0.21s** ・ clippy --lib **0 warnings** ・ tsc **0 errors** ・ vitest 45。
**CI は release.yml のみ (v*タグ)** — テスト/lint が CI で一度も走らない。lint設定皆無 (eslint/prettier/rustfmt.toml/clippy.toml なし)。

## ドキュメントドリフト
- diagnostics/退避追跡が docs/SPEC・docs/CODEMAPS に皆無 (README のみ)
- backend.md:4 「20 modules」→ 実22 / backend.md:153 「action_log dev only」→ stale
- dependencies.md (2026-03-22): hudsucker 0.24 → 実 **ideamans-hudsucker 0.25 (fork)**、vitest/tauri-plugin-dialog/windows-sys 欠落
- test-strategy.md:12 「Rust 135」→ 141。行数記載・SPEC.md 索引はおおむね正確

## セキュリティ/設定
- tauri.conf.json:44 `csp: null` (ゲーム互換のため意図的・SPEC記載あり、現状維持)
- **game-controls.json が game-content/game-overlay に core:default 付与** — DMM/ゲーム表示webviewから登録コマンドを invoke 可能 (game_init.js が実際に使用: zoom/mute/log_frontend_event)。fs/shell系は非公開。→ W6-2 で最小化
- game_init.js: 「ゲームJS環境注入禁止」の**字義に準拠** — `if(isTop)` ガードで全ての挙動的JSはDMMラッパーのみ、ゲームiframe内はCSSのみ (CAUTION.md:31 容認済)、データ捕捉100% proxy。ユーザー容認済み (2026-07-15)
- log_frontend_event は16KB上限+redact 済み (diagnostics.rs:19)

## パス管理
`data_dir.join(...)` **74箇所/15ファイル** (engine 19・migration 15・lib 11・overlay 6・commands 5・models 5…)。"sync"/"local" リテラル手組み、centralモジュール無し。→ W3-3
