# Kantai (艦隊) Window + Screen-State Tracking + Debug Tab

## 概要
ゲームの艦隊タブ操作と連動して切り替わる **艦隊 window** を追加。
backend で **画面状態 (current_screen / current_fleet / quest filters)** を
クリック検知 + API 観測の両ソースから維持し、frontend の Debug タブで
リアルタイム可視化できるようにする。

## アーキテクチャ

```
[Game window]                   [AppState]                  [Other windows]
mouse_hook (Win)  ──click──┐
                           ├─→ current_screen ──emit──→ screen-changed → DebugTab
api/process_api ──URL──────┤    current_fleet  ──emit──→ fleet-view-changed → KantaiView / DebugTab
                           └─→ quest_period/category ──→ quest-filters-changed → DebugTab
```

## 主要 component

### Backend
- `kantai.rs` — 新規 window の show/hide/toggle commands
- `mouse_hook.rs::consume_clicks` — click → `ui_event::detect_event` → screen/fleet/quest の更新 + event emit
- `ui_event/mod.rs` — `Screen` enum + 画面別 detect 関数。座標は `docs/KNOWLEDGE/ui-regions.md` のキャリブレーション値
- `api/mod.rs::screen_from_api` — API URL → Screen マッピング(クリック検知の補完)
- `commands.rs` — `get_current_screen` / `get_current_fleet` / `get_quest_filters` invoke

### Frontend
- `tauri.conf.json` — `kantai` ラベルの window 追加(`visible:false`、600x900、リサイズ可)
- `App.tsx` — `getCurrentWindow().label === "kantai"` で `<KantaiView/>` を返す
- `components/kantai/KantaiView.tsx` — 第1〜第4 タブ + FleetPanel + 下部 UI ズーム slider
- `components/debug/DebugTab.tsx` — current_screen + 直近クリック(crop screenshot 付) + 直近 API
- `game_init.js` — control bar に ⚓艦隊 / 📊管理 ボタン追加

## State モデル

### `current_screen` (`Mutex<ui_event::Screen>`)
ゲーム画面の推定状態。初期値 `Unknown`。
更新源:
- click: `Navigate` / `SideMenuClick` イベントで `screen_from_event` → 新画面
- API: `screen_from_api` で URL → Screen

`api_start2/getData` (ゲーム再ロード)で必ず Unknown にリセット。

### `current_fleet` (`Mutex<Option<u32>>`)
艦隊持ち画面 (FleetComposition / Resupply / Remodel) における選択艦隊。
- 1〜4: 第N艦隊
- 5: 「他」(改装の追加タブ等)
- None: 艦隊持ち画面外

`FleetSelect{fleet}` クリック検知で更新、艦隊持ち画面以外への遷移で `None` に。

### `current_quest_period` / `current_quest_category` (`Mutex<Option<String>>`)
QuestList のサブ画面状態。
- period: 全 / 遂行中 / Daily / Weekly / Monthly / 単 / 他 / Others
- category: 出撃 / 演習 / 遠征 / 編成 / その他

クリック検知 (`QuestFilter` / `QuestCategoryFilter`) で更新。
QuestList 外への遷移で両方 `None` に。

## Event 仕様

| Event 名 | Payload | 発火タイミング |
|---------|---------|--------------|
| `screen-changed` | `string` (Screen variant名) | current_screen が変化したとき |
| `fleet-view-changed` | `number \| null` | current_fleet が変化、または艦隊持ち画面離脱で null |
| `quest-filters-changed` | `{period: string\|null, category: string\|null}` | filter 変化、または QuestList 離脱 |
| `click-event` | `{ts, x, y, screen, event}` | クリック毎(全件) |
| `click-screenshot` | `{ts, x, y, image: data:image/png;base64,…}` | クリック後の crop 完了時(同時1件のみ) |

## Kantai window UI

- 上部: 第1/第2/第3/第4 のタブ。クリックで切替、`localStorage.kc-kantai-fleet-id` 永続化。
  ゲームで艦隊タブクリック → `fleet-view-changed` 受信で自動切替(fleet 1〜4 のみ。5=「他」は無視)。
- 中央: 選択艦隊の `<FleetPanel/>` (homeport から再利用)
- 下部: UI サイズ slider (50%〜200%, 5%刻み, 初期 100%)。`localStorage.kc-kantai-ui-zoom` 永続化。

## Debug tab UI

- 上段カード: 「現在認識中の画面」(`Screen (日本語名) - 第N艦隊` 等のサブ画面合成表示)
- 左パネル: 直近 30 件のクリック (時刻 / 座標 / 画面 / 検知イベント / クリック地点 crop)
- 右パネル: 直近 30 件の API endpoint
- ⏸ 一時停止 / 🗑 クリア ボタン

クリック地点 crop は backend で `capture_and_crop` (Win + dev only) し base64 で event 送信。
hover で 200x200 拡大表示。

## 既知の制約 / 今後の課題

- `mouse_hook` は Win only。Mac は手動タブのみ
- 検知座標は `docs/KNOWLEDGE/ui-regions.md` の実測キャリブレーション。ゲーム UI が
  バナー差替え等で動くと再キャリブレーション要
- 検知できていない画面/領域は順次拡張
- 任務カテゴリの組み合わせ (period × category) は文字列ベース。enum 化検討余地

## 関連ドキュメント

- [window-role-swap.md](./window-role-swap.md) — game/management window 役割交換
- [action-analysis.md](./action-analysis.md) — action_log 解析手法
- [docs/KNOWLEDGE/ui-regions.md](../KNOWLEDGE/ui-regions.md) — 画面別座標マップ(実測キャリブ済)
