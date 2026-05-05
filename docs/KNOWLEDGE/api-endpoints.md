# API エンドポイント

## レスポンス共通構造

```json
{ "api_result": 1, "api_result_msg": "成功", "api_data": {...} }
```
- `api_data` は `Option<T>` — 一部APIでは `null` が返る

## マスター & プレイヤーデータ

| エンドポイント | 概要 | レスポンス |
|---------------|------|-----------|
| `api_start2/getData` | マスターデータ (セッション1回) | 艦船・装備・任務等の全マスター |
| `api_port/port` | 母港 (状態同期ポイント) | 艦隊・資材・任務等の全プレイヤーデータ |
| `api_get_member/require_info` | ログイン直後の装備一覧 | `api_slot_item[]` |
| `api_get_member/slot_item` | 装備画面の装備一覧 | `Vec<PlayerSlotItemApi>` |
| `api_get_member/ship3` | 装備変更後の艦更新 | 艦データ + 艦隊構成 |
| `api_get_member/ship_deck` | 装備変更後の艦+艦隊更新 | `api_ship_data[]` + `api_deck_data[]` |
| `api_get_member/questlist` | 任務一覧 | `ApiQuestListResponse` |
| `api_get_member/deck` | 艦隊一覧 | Fleet配列 (全置換) |
| `api_get_member/ndock` | 入渠ドック | RepairDock配列 (全置換) |
| `api_get_member/mapinfo` | 海域情報 | `api_map_info[]` + `api_air_base[]` |

### api_get_member/material (資材)

`{api_id, api_value}` の配列 (8要素):

| api_id | 資材 |
|--------|------|
| 1 | 燃料 |
| 2 | 弾薬 |
| 3 | 鋼材 |
| 4 | ボーキサイト |
| 5 | 高速建造材 |
| 6 | 高速修復材 (バケツ) |
| 7 | 開発資材 |
| 8 | 改修資材 (ネジ) |

## 艦隊編成

| エンドポイント | 概要 | api_data |
|---------------|------|----------|
| `api_req_hensei/change` | 編成変更 | request_bodyから解析 |
| `api_req_hensei/preset_select` | プリセット読込 | 艦隊データ |

## 装備操作

| エンドポイント | 概要 | api_data |
|---------------|------|----------|
| `api_req_kaisou/slotset` | 装備装着 | null (ship3で更新) |
| `api_req_kaisou/unsetslot_all` | 全装備解除 | null (ship3で更新) |
| `api_req_kaisou/preset_slot_select` | 装備プリセット読込 | null (ship3で更新) |
| `api_req_kaisou/slotset_ex` | 補強増設装備 | null (ship3で更新) |
| `api_req_kaisou/slot_deprive` | 装備剥ぎ | 更新データ |
| `api_req_kaisou/slot_exchange_index` | 装備入替 | `api_ship_data` (更新済み艦) |
| `api_req_kaisou/powerup` | 近代化改修 | `api_ship` + `api_deck[]` |

## 建造・廃棄

| エンドポイント | 概要 | api_data | 備考 |
|---------------|------|----------|------|
| `api_req_kousyou/createship` | 建造開始 | null | |
| `api_req_kousyou/createship_speedchange` | 高速建造 | null | |
| `api_req_kousyou/getship` | 建造完了 | `api_ship` + `api_slotitem[]` + `api_kdock[]` | 艦と初期装備をマージ |
| `api_req_kousyou/remodel_slot` | 装備改修 | 改修結果 | |
| `api_req_kousyou/remodeling` | 改装 (改造) | null (ship3で更新) | |
| `api_req_kousyou/createitem` | 装備開発 | `api_create_flag`, `api_get_items[]`, `api_material[8]` | 資材配列は**8要素** |
| `api_req_kousyou/destroyitem2` | 装備廃棄 | `api_get_material[4]` | request_body: `api_slotitem_ids` (カンマ区切り) |
| `api_req_kousyou/destroyship` | 艦解体 | `api_material[4]` + `api_unset_list` | **全艦隊から該当艦IDを除去必須** |

### destroyship の注意点
- request_body の `api_ship_id` で解体対象を特定
- **全艦隊 (`state.profile.fleets`)** から該当ship_idを除去すること
- 艦隊配列に残った無効ship_idはクラッシュ・UI不具合の原因

## 補給

### api_req_hokyu/charge (補給)

レスポンス:
- `api_ship[]`: 補給された各艦のデータ
  - `api_id`: 艦ID
  - `api_fuel`: 燃料
  - `api_bull`: 弾薬
  - `api_onslot[]`: 各スロットの搭載機数
- `api_material[4]`: 位置配列 `[燃料, 弾薬, 鋼材, ボーキ]`
- `api_use_bou`: 使用バケツ数

**注意**: `api_material` は**位置配列** (api_portのMaterialオブジェクト配列とは異なる形式)

## 遠征

| エンドポイント | 概要 | api_data |
|---------------|------|----------|
| `api_req_mission/start` | 遠征開始 | `api_complatetime` (完了タイムスタンプ) |
| `api_req_mission/result` | 遠征帰還 | `api_get_material[4]`, `api_clear_result`, `api_get_exp` 等 |

- `api_clear_result > 0` で遠征成功

## メンバー設定・アイテム

| エンドポイント | 概要 | api_data |
|---------------|------|----------|
| `api_req_member/itemuse` | アイテム使用 (間宮/伊良湖等) | 対象艦の更新データ |
| `api_req_member/set_oss_condition` | OSS (オンスクリーン設定) 更新 | null |
| `api_req_member/get_practice_enemyinfo` | 演習相手の艦隊詳細 | 敵艦隊データ |
| `api_get_member/practice` | 演習相手一覧 | 演習5件 |
| `api_get_member/useitem` | 所持アイテム一覧 | `api_useitem[]` |

### set_oss_condition

- request_body: `api_language_type=<N>` + `api_oss_items[0..7]=<0|1>` の 8 フラグ
- レスポンス: 確認のみ (api_data なし)
- 画面: 編成画面の上部 y<150 付近にトグル UI (要 UI 特定)

## 任務

| エンドポイント | 概要 | api_data |
|---------------|------|----------|
| `api_req_quest/start` | 任務受託 | request_bodyから解析 |
| `api_req_quest/stop` | 任務放棄 | request_bodyから解析 |
| `api_req_quest/clearitemget` | 任務完了 | `api_bounus` (報酬) |

## 戦闘

### 昼戦 (api_req_sortie)
| エンドポイント | 概要 |
|---------------|------|
| `battle` | 通常昼戦 |
| `airbattle` | 航空戦 |
| `ld_airbattle` | 長距離航空戦 |
| `ld_shooting` | レーダー射撃 |
| `night_to_day` | 夜→昼戦 |

### 連合艦隊昼戦 (api_req_combined_battle)
| エンドポイント | 概要 |
|---------------|------|
| `battle` | 機動部隊昼戦 |
| `battle_water` | 水上部隊昼戦 |
| `each_battle` | 各艦隊昼戦 |
| `each_battle_water` | 各艦隊水上昼戦 |
| `ec_battle` | 敵連合昼戦 |
| `ld_airbattle` | 連合長距離航空 |
| `ld_shooting` | 連合レーダー射撃 |

### 夜戦 (api_req_battle_midnight)
| エンドポイント | 概要 |
|---------------|------|
| `battle` | 通常夜戦 (昼戦の続き) |
| `sp_midnight` | 夜戦開始 (昼戦なし) |

### 連合艦隊夜戦 (api_req_combined_battle)
| エンドポイント | 概要 |
|---------------|------|
| `midnight_battle` | 連合夜戦 |
| `sp_midnight` | 連合夜戦開始 |
| `ec_midnight_battle` | 敵連合夜戦 |
| `ec_night_to_day` | 敵連合夜→昼 |

### 戦闘結果 & ナビゲーション
| エンドポイント | 概要 |
|---------------|------|
| `api_req_sortie/battleresult` | 通常戦闘結果 |
| `api_req_combined_battle/battleresult` | 連合戦闘結果 |
| `api_req_map/start` | 出撃開始 |
| `api_req_map/next` | 次ノード進撃 |

### 演習
| エンドポイント | 概要 | 備考 |
|---------------|------|------|
| `api_req_practice/battle` | 演習昼戦 | ApiBattleResponse互換 |
| `api_req_practice/midnight_battle` | 演習夜戦 | ApiBattleResponse互換 |
| `api_req_practice/battle_result` | 演習結果 | 出撃追跡不要 |

## その他

| エンドポイント | 概要 |
|---------------|------|
| `api_req_ranking/mxltvkpyuklh` | ランキング (raw JSON文字列) |

## 資材配列フォーマットの違い

| API | 形式 | 要素数 |
|-----|------|--------|
| `api_port/port` の material | `{api_id, api_value}` オブジェクト配列 | 8 |
| `api_get_member/material` | `{api_id, api_value}` オブジェクト配列 | 8 |
| `api_req_hokyu/charge` の api_material | 位置配列 `[燃,弾,鋼,ボ]` | 4 |
| `destroyitem2` の api_get_material | 位置配列 | 4 |
| `destroyship` の api_material | 位置配列 | 4 |
| `createitem` の api_material | 位置配列 | **8** |
