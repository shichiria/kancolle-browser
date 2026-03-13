# 装備 (Equipment)

## スロット構造

### 艦スロット
- `api_slot`: 装備インスタンスIDの配列
  - `-1` = 空スロット
- `api_slot_ex`: 補強増設スロット
  - `-1` = 増設未開放
  - `0` = 増設開放済み・空
  - `>0` = 装備インスタンスID

### 走査ルール
装備チェック時は**必ず** `slot` と `slot_ex` の両方を走査:
```rust
for &slot_id in info.slot.iter().chain(std::iter::once(&info.slot_ex)) {
    if slot_id <= 0 { continue; }
    // process
}
```

## マスター装備タイプ配列

- `api_type[2]` = `item_type` (カテゴリ)
- `api_type[3]` = `icon_type` (アイコン)

### 主要なタイプ値
| item_type | 用途 |
|-----------|------|
| 30 | 輸送機材 (ドラム缶) |

| icon_type | 用途 |
|-----------|------|
| 14 | ダメコン (応急修理要員/女神) |

## 改修 (明石工廠)

### 利用可能条件
- **JST曜日** (日=0 ... 土=6) に依存
- 担当艦 = **第1艦隊、インデックス1** (0-based: `fleets[0][1]`)
- 曜日 AND 担当艦の両方が一致する必要あり

### コスト構造
- Phase 1 (★0→★5): 基本資材 + 消費装備
- Phase 2 (★6→★9): 通常より高コスト
- 転換コスト: 装備アップグレード用の別資材
- 消費装備数 = **全パスの最大値**

### データ処理
- `improved_equipment.json`: 改修済み装備IDの `HashSet<i32>`
- 静的改修データは `include_str!` で埋め込み — `OnceLock` でキャッシュ
- **BOM (`U+FEFF`)**: パース前にJSON先頭から除去が必要
