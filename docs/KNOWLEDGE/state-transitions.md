# 状態遷移 & APIシーケンス

## APIレスポンスエンベロープ

全レスポンス共通:
```json
{ "api_result": 1, "api_result_msg": "成功", "api_data": {...} }
```
- `api_data` は常に `Option<T>` — 省略されるレスポンスあり

## 想定APIコールシーケンス

1. `api_start2/getData` — マスターデータ (セッション1回)
2. `api_port/port` — 母港画面 (出撃完了、任務リセットチェック)
3. 出撃: `api_req_map/start` → 戦闘 → `api_req_map/next` (ループ) → `api_port/port`
4. 戦闘: 昼戦API → (任意の夜戦) → `battleresult`

## 装備データの初期化順序

装備リスト (`player_slotitems`) は2箇所で取得される:
1. **`api_get_member/require_info`** — ログイン直後、`api_port/port` **より前**に呼ばれる
2. **`api_get_member/slot_item`** — 装備画面で呼ばれる

`require_info` を処理しないと、最初の `port` 時点で `player_slotitems` が空のまま。
ダメコン判定・OASW判定等の装備依存ロジックが初回ログイン時に動かない原因になる。

## 装備変更の検出

- `api_get_member/ship3` — 装備変更後にサーバーが送信。艦データ + 艦隊構成を一括更新
- `api_req_kaisou/slot_deprive` (装備剥ぎ) にも対応が必要
- `ship3` 受信時はマスター装備データ参照でアイコン・特殊装備を再計算

## マップノード分類

| api_event_id | 意味 |
|--------------|------|
| 1 | 何もなし (ダド) |
| 2 | 資源獲得 |
| 3 | 渦潮 |
| 4 | 通常戦闘 |
| 5 | ボス戦闘 |
| 6 | 気のせい / 能動分岐 |
| 7 | 航空偵察 / 索敵分岐 |
| 8 | 護衛成功 / 報酬 |
| 9 | 揚陸地点 |
| 10 | 泊地修理 |

### api_color_no (event_kind)
ノードの色。4=赤(戦闘), 5=赤(ボス), 7=航空, 8=空襲。

### 非戦闘ノード判定
`!hasBattle && (event_kind === 4 || event_kind === 5 || event_id === 6)` が基本ルール。
ただし以下の補完が必要:
- `event_id === 7` (航空偵察) — 戦闘なしなら非戦闘扱い
- `event_id === 1` (ダド) — 一部マップで使用
- `event_kind === 7, 8` (航空/空襲) — 戦闘パケット欠損時の考慮

### セルラベル
- `api_no` (cell_no) は**エッジID** (ノードIDではない)
- 複数エッジが同じノードに到達可能 (例: 1-3のエッジ5と11が共にEノード)
- A-Z ラベルはコミュニティ管理 (KC3Kai edges.json)、ゲームAPI内には存在しない

## マルチゲージマップ

### ゲージ構成
| 海域 | ゲージ数 | ボスセル |
|------|---------|---------|
| 7-2 | 2 | Gauge 1: Cell 7, Gauge 2: Cell 15 |
| 7-5 | 3 | Gauge 1: Cell 9, Gauge 2: Cell 14, Gauge 3: Cell 19 |

### マップID計算
`map_id = area × 10 + map_no` (例: 7-2 → 72)

### ゲージ情報の取得元
- **通常海域**: `api_get_member/mapinfo` の `api_map_info[]` から取得
  - `api_id`: map_id
  - `api_gauge_num`: ゲージ番号 (>0 でマルチゲージ)
- **イベント海域**: `api_req_map/start` の `api_eventmap.api_gauge_num` から取得
- **注意**: `api_eventmap` はイベント海域**のみ**。通常海域では `null`

### ボスセル推定
- `api_req_map/start` の `api_bosscell_no` でアクティブなボスセルを特定
- ボスセル番号からゲージ番号を推定可能

### 任務とゲージ
- 任務海域キー: `{area}-{no}` (基本) / `{area}-{no}({gauge})` (ゲージ指定)
- 例: `7-2(2nd)` は第2ゲージのみカウント、`7-2` はゲージ不問

## 不変条件

- `on_port()` はアクティブ出撃を完了させる
- `pending_battle` は `on_battle_result()` で `.take()` により消費
- `active_sortie` が `Some` でないと戦闘処理は進行しない
- 任務状態: API state 1=未受託, 2=進行中, 3=完了

## クラッシュリカバリ

- `fix_interrupted_records()` (起動時): `end_time=None` のレコードに `end_time` を設定
- ファイルの最終更新時刻を `end_time` のフォールバックとして使用

## リソースライフサイクル

- 同期エンジン置換時は古いエンジンをシャットダウンしてからnewを起動
- シャットダウン漏れはtokioタスクリーク
