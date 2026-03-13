# 任務 (Quest)

## リセットタイミング

すべてのリセットはJST (UTC+9) 基準。

| タイプ | 境界 | 備考 |
|--------|------|------|
| daily | 毎日 05:00 JST | |
| weekly | 月曜 05:00 JST | |
| monthly | 月初1日 05:00 JST | |
| quarterly | 3/6/9/12月 1日 05:00 JST | 年境界: 1-3月のDec四半期は前年 |
| yearly | 4月1日 05:00 JST | 会計年度 |
| once/limited | リセットなし | |

### 境界計算
`now < today_5am` の場合、境界は**昨日の05:00**（今日ではない）。

### 四半期の年トラップ
`q_month = 12` かつ現在月が 1, 2, **3** (3月1日05:00前) の場合、年を-1する。
条件: `q_month == 12 && m <= 3`

### カウンタリセット vs フルリセット
- フルリセット (`reset`): カウント、海域データ、完了状態をクリア
- カウンタリセット (`counter_reset`): 進捗カウンタのみクリア、完了状態は保持

## 任務進捗パターン

- `sub_goals` — 複数の独立条件、各自の海域/ランク/ボス/カウント
- `area` — 海域別追跡 (海域名は `/` で分割)
- `counter` — 単純インクリメント ("任意" or "演習" 任務)

## 出撃任務マッチング

### 艦名マッチング
- `ContainsShipName` は **`starts_with`** (前方一致、完全一致ではない)
- 例: "那智" は "那智改二", "那智改二丙" にマッチ
- 改装形態を横断してマッチング

### 条件タイプ
- `ShipTypeCount { stype: [i32], count: i32 }` — 指定艦種N隻
- `MaxShipTypeCount { stype: [i32], max: i32 }` — 指定艦種が最大N隻
- `OnlyShipTypes { stype: [i32] }` — 全艦が指定艦種のみ
- `ContainsShipName { names: [String] }` — 前方一致の艦名を含む
- `OrConditions` — 複数の代替条件セット (いずれか一つで満足)
- `no_conditions: true` — 条件チェックをバイパス

### 海域文字列フォーマット
- `"{map_area}-{map_no}"` 例: "1-1", "7-2"
- 複数ゲージ: `"{map_area}-{map_no}({gauge})"` 例: "7-2(2nd)"
- マッチング: 完全一致を優先、次にベース海域 (ゲージサフィックスを除去)

## 敵艦種マッチング

- `carrier` → stype 7 (CVL), 11 (CV), 18 (CVB)
- `transport` → stype 15 (AP)
- `submarine` → stype 13 (SS), 14 (SSV)
