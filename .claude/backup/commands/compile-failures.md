---
description: failures.jsonl からレビューエージェント用指示書を生成
---
# /compile-failures - 失敗 → レビュー指示書生成

`failures.jsonl` を読み込み、パターンを抽出し、レビューエージェント向けの構造化指示書を生成する。

## Process

1. プロジェクトルートの `failures.jsonl` を読む。なければその旨を伝えて終了。

2. カテゴリ別にグルーピングし、`lesson` フィールドを抽出する。

3. 重複排除: 同じ意味の lesson はマージする。

4. 以下の構造で指示書を生成:

```markdown
# レビュー指示書 (failures.jsonl から自動生成)
Generated: {date} | 分析した失敗: {count}件

## 絶対ルール (severity=critical/high)
- {lesson}
  - 根拠: {summary} ({timestamp})

## 注意事項 (severity=medium/low)
- {lesson}

## カテゴリ別パターン
### {category} ({count}件)
- {lesson 1}
- {lesson 2}

## このプロジェクト固有の注意点
### WebView2 / Tauri
- {webview_issue の lesson をここに集約}

### 艦これドメイン
- {domain_misunderstanding の lesson をここに集約}
```

5. `reviewer_instructions.md`（プロジェクトルート）に書き出す。毎回上書き。

6. 報告と提案:
   - 「reviewer_instructions.md を生成しました」
   - 「domain-bug-review コマンドや code-reviewer エージェントのプロンプトに組み込むことを推奨します」
   - 「または、レビュー前に `Read ./reviewer_instructions.md` を実行させてください」

## Rules
- `failures.jsonl` は読み取り専用（絶対に変更しない）
- `reviewer_instructions.md` は毎回再生成
- 指示は具体的・実行可能に保つ
- エージェントプロンプトの自動変更はしない — ユーザーが判断する
