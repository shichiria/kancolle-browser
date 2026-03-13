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
