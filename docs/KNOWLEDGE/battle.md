# 戦闘 (Battle)

## HP配列

### 通常戦闘 (6隻)
- `api_f_nowhps` / `api_f_maxhps`: 6要素、インデックス 0-5
- `api_e_nowhps` / `api_e_maxhps`: 6要素、インデックス 0-5
- **パディングなし** (0-based)

### 連合艦隊戦闘 (12隻)
- `api_f_nowhps` / `api_f_maxhps`: **12要素**
  - 0-5 = 第1艦隊 (main fleet)
  - 6-11 = 第2艦隊 (escort fleet)
- `api_e_nowhps` / `api_e_maxhps`: 連合敵で最大12要素

### 敵艦配列
- `api_ship_ke`: 敵マスターID、先頭に `-1` パディングあり
- `api_ship_lv`: 敵レベル、同じパディング構造
- オフセット計算: `levels.len() - ids.len()`

## 戦闘フェーズ順序

ダメージは以下の厳密な順序で適用:

| 順序 | Key | 形式 | 対象 | 備考 |
|------|-----|------|------|------|
| 1 | `api_kouku` | kouku | 第1艦隊 (0-5) | api_stage3.api_fdam |
| 2 | `api_kouku_combined` | kouku | **第2艦隊 (offset +6)** | 連合艦隊のみ |
| 3 | `api_opening_atack` | raigeki | api_fdam indexed | 開幕雷撃 |
| 4 | `api_opening_taisen` | hougeki | api_at_eflag filtered | 先制対潜 |
| 5 | `api_hougeki1` | hougeki | api_at_eflag filtered | 砲撃戦1 |
| 6 | `api_hougeki2` | hougeki | api_at_eflag filtered | 砲撃戦2 |
| 7 | `api_hougeki3` | hougeki | api_at_eflag filtered | 砲撃戦3 |
| 8 | `api_raigeki` | raigeki | api_fdam indexed | 閉幕雷撃 |
| 9 | `api_raigeki_combined` | raigeki | **第2艦隊 (offset +6)** | 連合艦隊のみ |

### 夜戦 (別APIコール)
- 単一フェーズ: `api_hougeki`
- HPは夜戦レスポンスの `api_f_nowhps` (昼戦後HP) から開始
- `sp_midnight` (夜戦開始): `api_f_nowhps` は戦闘前HP

## ダメージ形式

### Hougeki (砲撃)
- `api_at_eflag`: 1 = 敵→味方, 0 = 味方→敵
- `api_df_list`: 二重配列のターゲットインデックス (0-based)
- `api_damage`: 二重配列、**float** 値 (i32に切り捨て)
- フォールバック (旧形式、eflagなし): 1-6 = 味方, 7-12 = 敵 (1-based)

### Raigeki (雷撃)
- `api_fdam`: 味方へのダメージ、位置0-basedインデックス、**float**
- `api_edam`: 敵へのダメージ、位置0-basedインデックス、**float**
- 連合艦隊: `api_raigeki_combined` は第2艦隊に offset +6

### Kouku (航空戦)
- `api_stage3.api_fdam` / `api_stage3.api_edam`: **float** 値
- `api_kouku_combined.api_stage3.api_fdam`: インデックス0-5が**第2艦隊 (hp[6..12])** に対応

## 連合艦隊

- `api_combined_flag` (母港データ): 0=なし, 1=機動部隊, 2=水上部隊, 3=輸送護衛
- 連合艦隊は**常に** fleet 0 (main) + fleet 1 (escort)
- HP分割: `hp_states[main_fleet_count + i]` = 護衛艦i番目

### 連合艦隊バトルエンドポイント
昼: `battle`, `battle_water`, `each_battle`, `each_battle_water`, `ec_battle`, `ld_airbattle`, `ld_shooting`
夜: `midnight_battle`, `sp_midnight`, `ec_midnight_battle`, `ec_night_to_day`

## 戦闘結果

### 陣形
- `api_formation`: 3要素配列 `[自陣形, 敵陣形, 交戦形態]`
- 通常: 1=単縦, 2=複縦, 3=輪形, 4=梯形, 5=単横, 6=警戒
- 連合: 11=第一警戒(対潜), 12=第二警戒(前方), 13=第三警戒(輪形), 14=第四警戒(戦闘)

### 制空状態
- `api_kouku.api_stage1.api_disp_seiku`: 0=劣勢, 1=優勢, 2=制空権確保, 3=均衡, 4=喪失

### MVP
- `api_mvp`: **1-based** (1 = 艦隊の1番艦、0ではない)

### ドロップ
- `api_get_flag[1] == 1` でドロップ判定 (インデックス1、0ではない)
- ドロップ艦情報: `api_get_ship.api_ship_id`, `api_get_ship.api_ship_name`

### ランク値
- S=5, A=4, B=3, C=2, D=1, E=0 (ランク比較用)
