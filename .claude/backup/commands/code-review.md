---
description: code-reviewerエージェントを呼び出して統合レビュー実行
---
# /code-review

code-reviewer エージェントを起動し、未コミットの変更に対して統合レビューを実行する。

## 実行

`$ARGUMENTS` が指定された場合はそのファイル/ディレクトリを対象として code-reviewer エージェントに渡す。
未指定の場合は `git diff` の全変更が対象。

**Agent tool** で `subagent_type: "code-reviewer"` を使って起動すること。

プロンプト例:
```
以下の変更をレビューしてください:
対象: {$ARGUMENTS または git diff の全変更}

手順:
1. reviewer_instructions.md を読む
2. docs/KNOWLEDGE/README.md → 関連KNOWLEDGEを読む
3. 変更ファイルと周辺コードを読む
4. 3軸（失敗パターン・ドメイン知識・セキュリティ/品質）でレビュー
5. レポートを出力
```
