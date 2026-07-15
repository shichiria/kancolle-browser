<!-- AUTO-GENERATED from source code -->

# 基地航空隊追跡 詳細設計書

## 1. 概要

基地航空隊 (LBAS) の編成・行動・補給状態を API 傍受で追跡し、kantai ウィンドウの 🛩 タブ (AirBaseTab) に表示する。出撃中は基地航空隊攻撃の結果 (制空状態・機数損失) を波ごとに記録し、損失を各中隊へ比例配分して現在機数を推定する。

- バックエンド: `src-tauri/src/api/air_corps.rs`
- データ構造: `src-tauri/src/api/models/air_base.rs` (`AirBase` / `AirBasePlane` / `AirBaseAttackWave` / `AirBaseDistance`)
- フロントエンド: `src/components/kantai/AirBaseTab.tsx` (KantaiView の 🛩 陣形タブ)

## 2. データ構造 (`api/models/air_base.rs`)

| 構造体 | 役割 |
|--------|------|
| `AirBase` | 1基地 (area_id + rid)。名前・行動 (待機/出撃/防空/退避/休息)・戦闘行動半径・中隊リスト・直近攻撃波 |
| `AirBasePlane` | 1中隊。slotid・機体ID・機数 (count/max_count)・疲労 (cond)・state (0=未配備/1=配備/2=配置転換中) |
| `AirBaseAttackWave` | 出撃1波の記録 (base_id, wave_index, 制空状態, stage1機数) |
| `AirBaseDistance` | 戦闘行動半径 (base + bonus) |

GameStateInner には `air_bases` として保持される。

## 3. 対応 API (`api/mod.rs` → `air_corps.rs`)

| endpoint | 処理 |
|----------|------|
| `api_get_member/base_air_corps` | `parse_air_bases` — 全基地の状態を再構築 |
| `api_req_air_corps/set_plane` | `apply_set_plane` — 中隊の配備/入替 (api_data の plane_info を反映) |
| `api_req_air_corps/set_action` | `apply_set_action` — 行動指定 (待機/出撃/防空/退避/休息) |
| `api_req_air_corps/supply` | `apply_supply` — 補給 (機数・cond 回復を反映) |
| `api_req_air_corps/change_name` | `apply_change_name` — 基地名変更 |
| `api_req_air_corps/change_deployment_base` | `apply_change_deployment` — 配置転換 |
| `api_port/airCorpsCondRecoveryWithTimer` | 疲労タイマー回復の反映 |

いずれも処理後に `air-base-updated` イベントを emit する (`emit_air_base_update`)。

## 4. 戦闘連携 (`api/battle.rs`)

- **出撃開始** (`api_req_map/start`): `clear_recent_attacks` — 🛩 タブの攻撃波表示を当該出撃分のみにするためクリア
- **戦闘中** (昼戦系 API に `api_air_base_attack` がある場合): `apply_battle_attack` — 波ごとに `AirBaseAttackWave` を記録し、stage1 の損失機数を **出撃中の各中隊へ機数比例で配分** (`distribute_losses` — floor + 剰余を小数部の大きい順に配分) して現在機数を減算

## 5. イベント / コマンド

| 種別 | 名前 | 内容 |
|------|------|------|
| イベント | `air-base-updated` | `Vec<AirBase>` (全基地スナップショット) |
| コマンド | `get_air_bases` | 現在の `Vec<AirBase>` を返す (AirBaseTab 初期表示用) |

## 6. フロントエンド (AirBaseTab)

- `get_air_bases` で初期取得、`air-base-updated` を購読して更新
- 基地ごとに: 行動状態・戦闘行動半径・各中隊の機体名/機数/疲労・直近攻撃波の制空結果を表示
- 型定義: `src/types/airbase.ts` (`AirBase` / `AirBasePlane` / `AirBaseAttackWave` / `AirBaseDistance`)

## 7. 制約・注意

- 損失配分は比例推定であり、実ゲーム内部の個別中隊損失とは一致しない場合がある (次回 `base_air_corps` 取得で真値に補正される)
- 防空 (基地空襲) の損失反映は `api_air_base_attack` 経由の出撃波のみ対象
