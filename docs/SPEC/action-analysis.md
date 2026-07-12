# アクション解析手法

action_logs (`actions_YYYYMMDD.jsonl`) と screenshots からゲーム画面とアクションを同定し、`docs/KNOWLEDGE/ui-regions.md` に反映するための手順。API を発行しない UI 内操作 (タブ切替、パネル展開、秘書艦タッチ等) も対象とする。

## 1. ログソース

### action_logs
- 場所: `%LOCALAPPDATA%\com.eo.kancolle-browser\local\action_logs\actions_YYYYMMDD.jsonl`
- 形式: JSONL (1 行 1 イベント)
- フィールド: `timestamp` / `category` / `action` / `detail`

### screenshots (dev build のみ)
- 場所: `%LOCALAPPDATA%\com.eo.kancolle-browser\local\action_logs\screenshots\`
- ファイル: `full_YYYYMMDD_HHMMSS_ms.png` / `crop_YYYYMMDD_HHMMSS_ms.png`
- レート制限: 最低 2 秒間隔、同時実行 1

### カテゴリ一覧

| category | 意味 | 例 |
|----------|------|----|
| `Click` | マウスクリック (ゲームキャンバス内) | `action=game_canvas`, `detail=x=.. y=.. event={..}` |
| `API` | `/kcsapi/` プロキシ通過 (生データ) | `action=/kcsapi/api_port/port` |
| `API_PARSED` | DTO パース成功 | `action=/kcsapi/api_port/port`, `detail=variant=..` |
| `Event` | 内部イベント発行 | `fleet-updated`, `port-data`, `sortie-complete` |
| `State` | ゲーム状態遷移 | `process_port`, `sortie_start` |
| `Command` | Tauri invoke | `open_game_window` |
| `Screen` | 画面遷移検知 | `Homeport -> FleetComposition` |
| `Fleet` | 艦隊タブ選択検知 | `fleet=2` |
| `Quest` | 任務フィルタ変更検知 | `period=..`, `category=..` |

### 画面追跡の現状 (2026-07 更新)

`current_screen` は**実装済み**: クリック検知 (`mouse_hook.rs` の `screen_from_event` — Navigate / SideMenuClick で更新) と API (`api/mod.rs` の `update_screen_from_api`) の両方から更新される。`detect_event` (`ui_event/`) は画面別ディスパッチで `FleetSelect` / `QuestFilter` / `QuestSelect` 等も返す。

なお「ヘッダーピクセル一致」による画面判定という当初想定の手法は未実装のまま (クリック/API ベースで代替)。API からの画面逆算は、クリック追跡が取れない場合の補助手段として引き続き有効。

## 2. 解析フロー

### Phase 1: セッション区切り

1. `category=Event action=proxy-ready` でセッション開始
2. `category=Command action=open_game_window` でゲームウィンドウ生成
3. ゲーム終了 (プロセス kill) まで 1 セッション
4. 1 日のログに複数セッションが含まれる場合あり

### Phase 2: 画面タイムライン構築

API → 画面のマッピング (主要なもの):

| API | 直後の画面 |
|-----|----------|
| `api_port/port` | 母港 |
| `api_get_member/deck` / `ship_deck` + `preset_deck` | 編成 (母港から遷移) |
| `api_get_member/ship3` | 編成後 (艦変更確定) |
| `api_req_hensei/change` | 編成内変更 |
| `api_req_hokyu/charge` | 補給実行 |
| `api_get_member/practice` | 演習相手一覧 |
| `api_req_member/get_practice_enemyinfo` | 演習敵詳細 |
| `api_get_member/mapinfo` | 出撃-海域選択 |
| `api_req_map/start` | 出撃開始 (陣形選択直前) |
| `api_req_sortie/battle` | 戦闘 (昼戦) |
| `api_req_battle_midnight/battle` | 夜戦 |
| `api_req_sortie/battleresult` | 戦果報告 |
| `api_req_map/next` | 次セル進撃/分岐 |
| `api_req_mission/start` | 遠征決定 |
| `api_req_mission/result` | 遠征結果 |
| `api_get_member/ndock` | 入渠-ドック選択 |
| `api_req_nyukyo/start` | 入渠開始 |
| `api_req_nyukyo/speedchange` | 入渠-高速修復 |
| `api_get_member/kdock` | 工廠-建造ドック |
| `api_req_kousyou/createship` | 建造開始 |
| `api_req_kousyou/createitem` | 開発実行 |
| `api_req_kousyou/destroyship` | 解体 |
| `api_req_kousyou/destroyitem2` | 装備廃棄 |
| `api_get_member/questlist` | 任務一覧 |
| `api_req_quest/start` | 任務受託 |
| `api_req_quest/clearitemget` | 任務完了報酬 |
| `api_req_kaisou/powerup` | 近代化改修 |
| `api_req_kaisou/remodeling` | 改造 |
| `api_req_kaisou/slotset` / `slot_exchange_index` | 装備変更 |
| `api_req_kaisou/slot_deprive` | 装備剥ぎ |
| `api_req_member/itemuse` | アイテム使用 |

**注意**: API を発行しない画面遷移は API のみでは検出不可。Click イベントの連続性 (座標・タイミング) で推定する。

### Phase 3: UnknownClick の意味推定

各 `UnknownClick {x, y}` について:

1. **直前の API から画面を推定** (Phase 2 のマップ)
2. **直後 (±1-3 秒) の API を確認**
   - API が発火 → その操作がその API を引き起こす (例: 編成画面で `ship_deck` → 艦隊タブ切替)
   - API 無し → UI 内遷移 (タブ切替、パネル展開、次へボタン等)
3. **座標を `ui-regions.md` の既知領域と突合**
   - 既知領域にマッチ → 既存マッピング妥当性検証
   - 既知領域外 → 新規領域候補
4. **連続クリックの形状を見る**
   - 同座標連打 → 読込待ちスキップ or 連打系ボタン (建造完了受取等)
   - 短時間の異なる座標 → ドラッグ or ダイアログ操作

### Phase 4: screenshots による検証

判別が難しいクリックは対応する `crop_YYYYMMDD_HHMMSS_ms.png` を目視確認する。ファイル名のタイムスタンプは Click イベントの timestamp と一致する (±0.1 秒)。

### Phase 5: ドキュメント反映

- **既知画面の新規領域**: `ui-regions.md` の該当セクションに追記
- **新規画面**: `ui-regions.md` に新セクション追加 + 画面判定のアンカー記載
- **API 無し UI アクション**: `ui-regions.md` に「UI のみ」ラベルで追記
- **画面遷移グラフ**: `state-transitions.md` の「想定APIコールシーケンス」に補完

## 3. 解析時の着目点

### よくあるパターン
- **秘書艦クリック** (母港): x≈550-900, y≈100-600 で API 無し、ボイス再生のみ
- **ポップアップ閉じ**: 遠征帰還通知の上にクリック → 通知消去
- **タブ連打**: 編成/遠征/任務画面のタブ、API なしで UI だけ切替
- **読込待ちスキップ**: 戦闘中/マップ進行中の画面全体クリック
- **ダブルクリック**: preset_deck 適用などのダブルアクション操作

### ノイズ除去
- クリック直後 50ms 以内の同座標クリックは連打扱い
- `y<0` / `x>1200` は画面外 (ウィンドウ境界のドラッグ等) — 記録されないはず
- マウス位置が動かずキーボード入力等が起きた場合は除外

## 4. 出力フォーマット

### ui-regions.md への追記形式

```
### {画面名}
{概要}
```
{座標範囲}: {説明}
```
→ イベント: `{event名}({パラメータ})` / (API: `{api_path}` or **UI のみ**)
```

### 新規画面追加時

1. 「画面判定」テーブルに行追加 (ヘッダーテキスト or 特徴)
2. 「確認済み画面一覧」のチェックボックスを [x] に
3. 座標マッピングは上記フォーマットで
4. `ui_event/mod.rs` の `Screen` enum に variant 追加 (実装は別タスク)

## 5. 実装反映 (将来)

解析で確定した領域は `ui_event/mod.rs` にも反映する:
- 新規 `Screen` variant 追加
- `detect_event` ディスパッチ追加
- `detect_{screen}` 関数を追加
- テスト (`ui_event/tests.rs`) に既知ケースを追加

画面追跡 (クリック + API ベース) は実装済みのため、`detect_{screen}` の拡張は追加した時点で実運用に乗る (2026-07 更新)。

## 関連ドキュメント

- [ui-regions.md](../KNOWLEDGE/ui-regions.md) — 画面別操作領域 (成果物)
- [state-transitions.md](../KNOWLEDGE/state-transitions.md) — 状態遷移シーケンス
- [api-intercept.md](./api-intercept.md) — API 傍受実装
