# 遠征 (Expedition)

## 条件タイプ

すべての条件はAND論理（全条件を満たす必要あり）。

- `ShipTypeCount { stype: [i32], count: i32 }` — stype所属で艦数カウント
- `DrumShipCount { count: i32 }` — ドラム缶を**1つ以上**搭載する艦の数
- `DrumTotal { count: i32 }` — 艦隊全体のドラム缶総数
- `FlagshipType { stype: [i32] }` — 旗艦が指定艦種のいずれか
- `FlagshipLevel { level: i32 }` — 旗艦レベル要件
- `TotalLevel { level: i32 }` — 全艦レベル合計
- `TotalFirepower/TotalAntiAir/TotalASW/TotalLOS { value: i32 }` — ステータス合計要件

## CVE (護衛空母) 特殊ケース

CVEは独立したstypeではない。判定には**両方**必要:
1. `stype == 7` (CVL)
2. マスターIDが `CVE_SHIP_IDS` ハードコードセットに含まれる (鳳翔改二/改二戦, 龍鳳改二, 大鷹型 等)

## ドラム缶識別

- 装備 `item_type == 30` (輸送機材カテゴリ)
- `slot` と `slot_ex` の両方を走査する必要あり

## 大成功条件

- `Regular` — 全艦キラキラ (cond ≥ 50)
- `Drum` — ≥4隻キラキラ OR 全艦キラキラ
- `Level` — ≥4隻キラキラ OR 全艦キラキラ

## API

- `api_mission`: `[type, mission_id, return_time, ?]` — `type==0` は遠征なし
- `return_time` はミリ秒エポック
