---
description: セッションログから失敗を抽出し failures.jsonl に記録
---
# /failure - セッション失敗記録

LOG/ と GEMINI/ のセッションログ、および現在の会話から失敗・バグ・設計ミスを抽出し、`failures.jsonl` に追記する。

## Process

1. ソースの走査:
   - 現在の会話を走査する
   - `$ARGUMENTS` が指定された場合はその内容のみを記録
   - `$ARGUMENTS` に `--logs` が含まれる場合は `LOG/*.md` と `GEMINI/*.md` も走査（backup/ 内は除外）

2. 以下を探す:
   - バグの発見と修正
   - 設計上の欠陥
   - ビルドエラー（単純な typo 除く）
   - WebView2/Tauri固有の問題（デッドロック、プロキシ等）
   - LLMの指示無視・誤出力
   - データ/設定の不整合
   - ドメイン知識の誤解（艦これAPI仕様の誤認等）

3. 各失敗について JSON line を作成:
```json
{"timestamp":"ISO","category":"bug|design_flaw|build_error|webview_issue|prompt_ignored|config_error|data_issue|domain_misunderstanding","severity":"critical|high|medium|low","summary":"100文字以内","root_cause":"原因","fix":"修正内容","files":["関連ファイル"],"lesson":"一行のルール"}
```

4. `failures.jsonl`（プロジェクトルート）に追記する。なければ新規作成。**絶対に上書きしない。**

5. 報告: 「N件の失敗を記録しました」と一覧を表示。

## カテゴリ説明

| category | 内容 |
|----------|------|
| `bug` | コードのバグ |
| `design_flaw` | アーキテクチャ・設計の問題 |
| `build_error` | コンパイル/ビルド失敗（非trivial） |
| `webview_issue` | WebView2/WKWebView/Tauri固有の問題 |
| `prompt_ignored` | LLMが指示を無視した |
| `config_error` | 設定ミス |
| `data_issue` | データ不整合 |
| `domain_misunderstanding` | 艦これ仕様の誤解 |

## Rules
- typo や単純構文エラーは除外
- `lesson` が最重要フィールド — レビューエージェントの指示に使われる
- `failures.jsonl` は追記のみ。絶対に既存行を変更しない
- CAUTION.md に既に記載の既知問題も、lesson が異なれば記録してよい
