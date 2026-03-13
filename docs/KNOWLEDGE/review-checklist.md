# ドメインバグレビュー観点

コードレビュー時にドメイン知識に基づいて確認すべき観点一覧。
各項目の詳細は対応するKNOWLEDGEファイルを参照。

## 1. インデックス & オフセット正確性 → battle.md

- [ ] HP配列インデックスが想定サイズに一致 (通常6, 連合12)
- [ ] `api_kouku_combined` が offset +6 でダメージ適用 (第2艦隊)
- [ ] `api_raigeki_combined` が offset +6 でダメージ適用 (第2艦隊)
- [ ] Hougeki `api_at_eflag` 方向: 1=敵→味方, 0=味方→敵
- [ ] `api_ship_ke` パディング (-1) がフィルタされている
- [ ] 連合艦隊HP分割: `main_fleet_count + i` で護衛インデックス
- [ ] 艦隊インデックスの重複なし (例: `fleet_idx == 1` 時の `vec![fleet_idx, 1]`)
- [ ] Raigeki `api_fdam`/`api_edam` が `hp.len()` に対して境界チェック

## 2. 戦闘フェーズ順序 → battle.md

- [ ] ダメージフェーズが正規順序で適用 (kouku → opening_atack → opening_taisen → hougeki1/2/3 → raigeki)
- [ ] 連合艦隊フェーズが含まれている (api_kouku_combined, api_raigeki_combined)
- [ ] 夜戦HPが夜戦レスポンスの `api_f_nowhps` から開始 (昼戦のものではない)
- [ ] フェーズの二重適用やスキップなし
- [ ] `sp_midnight` (夜戦開始) が新規PendingBattleを正しく生成

## 3. Null/欠損データ処理 → state-transitions.md

- [ ] 全戦闘フェーズがnull JSONをガード (`api_data.get(key)` + nullチェック)
- [ ] `Option` アンラップが `?` or `unwrap_or` を使用 (APIデータに素の `.unwrap()` なし)
- [ ] DTOのオプショナルフィールドに `#[serde(default)]`
- [ ] Float→Int: `.as_f64().unwrap_or(0.0) as i32` パターンの一貫使用

## 4. 連合艦隊ロジック → battle.md

- [ ] `is_combined` フラグが `state.profile.combined_flag > 0` で出撃時に設定
- [ ] 護衛艦隊は常にfleetインデックス1
- [ ] HP配列分割が境界超過しない (`hp_states.get()` を使用、直接index不可)
- [ ] 大破警告が第1・第2艦隊の両方をカバー
- [ ] 戦闘後HP更新が両艦隊をカバー
- [ ] キャッシュされた母港データサマリーが両艦隊を更新

## 5. 時刻境界ロジック → quest.md, senka.md

- [ ] JST 05:00境界: `now < today_5am` なら境界 = 昨日の05:00
- [ ] 四半期リセット: `q_month == 12 && m <= 3` → year - 1
- [ ] 年次リセット: 4月1日 (1月1日ではない)
- [ ] 週次リセット: 月曜 05:00 (日曜ではない)
- [ ] 戦果月境界: 月末日 22:00 JST
- [ ] EOボーナスカットオフタイミング正確
- [ ] ランキングチェックポイント: 03:00, 15:00 JST

## 6. 装備スロット処理 → equipment.md

- [ ] ダメコンチェックで `slot` と `slot_ex` の両方を走査
- [ ] ドラム缶カウントで `slot` と `slot_ex` の両方を走査
- [ ] 装備依存ロジック全般で `slot` と `slot_ex` の両方を走査
- [ ] `slot_ex` セマンティクス: -1 (未開放), 0 (空), >0 (装備あり)
- [ ] `api_type` 配列: index 2 = item_type, index 3 = icon_type

## 7. 状態遷移安全性 → state-transitions.md

- [ ] `on_port()` がアクティブ出撃を完了 (`end_time` 設定, `completed` へ遷移)
- [ ] `pending_battle` が `on_battle_result()` で `.take()` により消費
- [ ] `active_sortie` が戦闘処理前にNoneチェックされている
- [ ] 任務状態: API state 1=未受託, 2=進行中, 3=完了
- [ ] リソースライフサイクル: 新同期エンジン起動前に旧をシャットダウン
- [ ] `add_battle_exp` / `add_eo_bonus` で月境界チェック

## 8. フロントエンド状態一貫性

- [ ] イベント駆動更新が現在のフィルタ状態に一致 (日付範囲, ページネーション)
- [ ] 総数カウント (`battleLogsTotal`) が本当に新規レコードのみでインクリメント
- [ ] レンダー間で共有する可変フラグに `useRef` を使用 (クロージャ変数でなく)
- [ ] ステールクロージャ防止: useEffect内のイベントリスナーでrefsを更新
- [ ] `setBattleLogs` のupsertロジックが新規・既存の両方を正しく処理

## 9. 遠征条件ロジック → expedition.md

- [ ] 全条件がAND (すべて通過する必要あり)
- [ ] `ShipTypeCount` がstype所属でチェック (艦名ではない)
- [ ] CVE判定に `stype == 7` AND マスターID in `CVE_SHIP_IDS` の両方が必要
- [ ] ドラム缶 = `item_type == 30` (icon_typeやslotitem_idではない)
- [ ] `DrumShipCount` はドラム搭載艦数、`DrumTotal` はドラム総数
- [ ] ドラム検出で `slot` と `slot_ex` の両方を走査

## 10. 改修ロジック → equipment.md

- [ ] 曜日計算がJSTを使用 (ローカルタイムゾーンではない)
- [ ] 担当艦 = `fleets[0][1]` (第1艦隊、インデックス1、0-based)
- [ ] 消費装備数 = 全パスの最大値
- [ ] BOM (`U+FEFF`) が埋め込みJSONパース前に除去

## 11. 出撃任務マッチング → quest.md

- [ ] `ContainsShipName` が `starts_with` (前方一致) を使用、完全一致ではない
- [ ] `OrConditions` がいずれか一つの条件セットで満足
- [ ] 海域マッチング: 完全一致を優先、次にベース海域 (ゲージサフィックス除去)
- [ ] ランク比較が数値 (S=5 > A=4 > B=3 ...)

## 12. 戦闘結果詳細 → battle.md

- [ ] `api_mvp` が **1-based** (1 = 1番艦)
- [ ] ドロップ判定: `api_get_flag[1] == 1` (インデックス1、0ではない)
- [ ] 制空状態が `api_kouku.api_stage1.api_disp_seiku` から取得
- [ ] 陣形: `[自, 敵, 交戦形態]` — 連合は 11-14 範囲

## 13. 大破 & オーバーレイ → overlay.md

- [ ] 大破閾値: `hp / maxhp ≤ 0.25` AND `hp > 0`
- [ ] ダメコン = `icon_type == 14`、`slot` と `slot_ex` の両方チェック
- [ ] 陣形キーフォーマット: `"{area}-{no}-{cell}"` (3パーツ)
- [ ] オーバーレイ座標にズーム倍率 AND `scale_factor` の両方を適用

## 14. 戦果詳細 → senka.md

- [ ] 任務ボーナスが `api_type == 18` (clearitemget内) で識別
- [ ] 月末日 14:00 JST 以降の任務ボーナス → `quest_late`
- [ ] 月末日 22:00 JST 以降のEOボーナス → 翌月
- [ ] 確定戦果ベースが累積計算をオーバーライド

## 既知のバグパターン

過去に発生した典型的なバグ。レビュー時に同類のパターンがないか確認。

1. **連合艦隊フェーズ漏れ**: `api_kouku_combined`, `api_raigeki_combined` に offset +6 が必要
2. **Float→Int切り捨て**: ダメージ値は `f64`、`.as_f64()` してからキャスト
3. **slot_ex漏れ**: `slot` のみ走査して補強増設を見落とす
4. **四半期年境界**: `q_month == 12` かつ `m <= 3` で year - 1 が必要
5. **連合艦隊インデックス重複**: `fleet_idx == 1` 時の `vec![fleet_idx, 1]`
6. **HP配列境界**: 連合艦隊は12要素、通常は6
7. **ダメージクランプ**: HP計算後に `.max(0)` で負値を防止
8. **リソースライフサイクル**: 同期エンジン置換時にシャットダウン漏れ → タスクリーク
9. **日付フィルタ不一致**: イベント駆動更新がアクティブなフィルタ範囲外のレコードを挿入

## 重大度定義

- **CRITICAL**: ゲーム状態の誤り (HP計算、任務進捗、データ消失)
- **HIGH**: 特定条件下で誤った状態 (連合艦隊、夜戦開始、月境界)
- **MEDIUM**: 防御的コーディングの欠如 (nullガード、境界チェック、リソースリーク)
- **LOW**: コード動作とログ/ドキュメントの不整合
