---
description: docs/KNOWLEDGE/ のドメイン知識に基づくコードレビュー
---
# Domain Bug Review

`docs/KNOWLEDGE/` のドメイン知識を使ってコードのロジックバグを検出する。

## 手順

1. `docs/KNOWLEDGE/README.md` を読み、全KNOWLEDGEファイルを把握する
2. レビュー対象に関連するKNOWLEDGEファイルを読み込む
3. `docs/KNOWLEDGE/review-checklist.md` の観点に沿ってコードをレビューする
4. 発見事項を報告する

## スコープ

`$ARGUMENTS` が指定された場合、そのファイル/ディレクトリをレビューする。
未指定の場合は `docs/KNOWLEDGE/README.md` の内容に関連するソースコード全体を対象とする。

## 出力フォーマット

```
# Domain Bug Review Report

Scope: [レビュー対象]
Knowledge: [参照したKNOWLEDGEファイル]

## Summary
| Severity | Count |
|----------|-------|
| CRITICAL | N     |
| HIGH     | N     |
| MEDIUM   | N     |
| LOW      | N     |

## Findings

### [SEVERITY] Title
- **File**: path:line
- **Issue**: 何が問題か
- **Expected**: KNOWLEDGEの仕様が求める動作 (参照ファイル・セクション)
- **Fix**: 修正案
```
