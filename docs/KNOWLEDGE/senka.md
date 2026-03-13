# 戦果 (Senka / Ranking Points)

## 基本計算式

```
senka = hq_exp_gain × 7.0 / 10000.0 + eo_bonus + quest_bonus
```

確定戦果がある場合:
```
senka = confirmed_base + delta_exp × 7/10000 + new_eo + new_quest
```

## 月境界

- ランキング月は**月末日 22:00 JST** で切り替わる
- 22:00以降: EOボーナスは翌月扱い
- 月末日 14:00以降: 任務ボーナスは `quest_late` としてタグ付け

## EOボーナス値

| マップ | ボーナス |
|--------|----------|
| 1-5, 1-6 | 75 |
| 2-5 | 100 |
| 3-5 | 150 |
| 4-5 | 180 |
| 5-5 | 200 |
| 6-5 | 250 |
| 7-5 | 170 |

## 戦闘経験値記録

- `add_battle_exp()`: 個別戦闘の司令部経験値をタイムスタンプ付きで記録
- `update_experience()` (母港時): 累計司令部経験値を追跡
- **トラップ**: `add_battle_exp` は月境界をチェックしない — 月末のエントリがリセットで失われる可能性

## ランキングデータ

### チェックポイント
- 03:00 JST と 15:00 JST
- ランキングページは前回チェックポイントまでのデータを表示

### 任務ボーナスアイテム
- `battleresult` の `clearitemget` 配列で `api_type: 18` で識別
- アイテムIDからボーナス値へのマッピング (例: 895→440, 896→50, 903→200, 904→80)

### ランキング復号
- ランキングAPIは暗号化された `api_rate` 値を返す
- 復号: `senka = floor(rate / key / user_key) - 91`
- `key` は13要素の `POSSIBLE_RANK` テーブルから `position % 13` で選択
- `user_key` はメンバーID除算チェックで決定

## ログエントリタイプ

`exp`, `eo`, `quest`, `quest_late`, `eo_cutoff`, `checkpoint`, `ranking_confirmed`
