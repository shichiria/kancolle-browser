# 出撃札 (Sally Area)

イベント海域に出撃した艦に付与されるタグ。一度付くと、その札が許可された海域にしか
出撃できなくなる（艦の使い回し防止）。イベント終了時にクリアされる。

## API フィールド

`api_sally_area` — 各艦オブジェクトに含まれる整数。

| 値 | 意味 |
|----|------|
| 0 | 札なし（未出撃 / イベント期間外） |
| N (≥1) | 札N |

### 出現するエンドポイント

`api_port/port`, `api_get_member/ship3`, `api_get_member/ship_deck`,
`api_req_kaisou/slot_deprive`, `api_req_kousyou/getship`, `api_req_kaisou/powerup`
など、艦オブジェクトを返すもの全般。

### 重要: イベント期間外はフィールドごと存在しない

平時のレスポンスには `api_sally_area` キー自体が無い。
実測: `tests/fixtures/api_port_port.json` (619隻) は全艦でキー欠落。

→ Deserialize 側は必ず `#[serde(default)]` を付けて 0 にフォールバックすること
(`models/wire.rs` の `ApiShip`)。

### 実測データ (2026-07-15 `api_port/port`, 613隻)

| api_sally_area | 隻数 |
|---|---|
| 0 | 602 |
| 1 | 9 |
| 2 | 2 |

## 札の「名前」は API から取得できない

ゲーム内では札は色付きの名前ラベル（作戦名）で表示されるが、**その名称は API に
含まれない**（ゲーム側の UI 素材にのみ存在する）。他ツールはイベントごとに
ハードコードして対応している。

→ 本アプリでは推測せず **番号のまま `札N` として表示**し、番号ごとに固定色を割り当てる
(`ShipListTab.tsx` の `SALLY_COLORS`)。

## 海域側の受け入れ札: `api_mst_mapinfo.api_sally_flag`

`api_start2/getData` の `api_mst_mapinfo` 各エントリに 3 要素配列 `api_sally_flag` がある。

実測 (2026-07-15, 海域62):

| 海域 | api_sally_flag |
|---|---|
| 62-1 | `[1, 0, 1]` |
| 62-2 | `[1, 7, 1]` |
| 62-3 | `[1, 7, 1]` |
| 62-4 | `[1, 7, 1]` |

要素の正確な意味（どのビットがどの札に対応するか）は**未検証**。
ギミック解除で値が変わる可能性もあるため、この配列を使った出撃可否判定を実装する場合は
複数時点のログで裏取りしてから行うこと。現状の実装では未使用。

## 関連

- `src-tauri/src/api/models/wire.rs` — `ApiShip.api_sally_area`
- `src-tauri/src/api/models/mod.rs` — `ShipInfo.sally_area`
- `src-tauri/src/api/ship.rs` — `build_ship_info()`
- `src/components/ships/ShipListTab.tsx` — 札バッジ・札フィルタ
