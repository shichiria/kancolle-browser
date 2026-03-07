#!/bin/bash
INPUT=$(cat)
FILE_PATH=$(python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('tool_input',{}).get('file_path',''))" <<< "$INPUT")

# ゲート対象の拡張子（プロジェクトに合わせて変更）
case "$FILE_PATH" in
  *.rs|*.ts|*.tsx|*.js|*.jsx|*.css)
    ;;
  *)
    exit 0
    ;;
esac

PROJECT_DIR="${CLAUDE_PROJECT_DIR:-$(python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('cwd',''))" <<< "$INPUT")}"
MARKER="$PROJECT_DIR/.claude/gemini_approved"

if [ ! -f "$MARKER" ]; then
  cat >&2 <<'MSG'
[Gemini Gate] ソースコード編集がブロックされました。

手順:
1. Geminiに方針レビューを依頼（コード変更は絶対にさせない）:
   PROMPT="... DO NOT modify any code. Only review the approach and provide feedback. ..."
   LOGFILE="GEMINI/$(date +%Y%m%d%H%M%S).md"
   mkdir -p GEMINI
   { printf '## Prompt\n\n%s\n\n## Response\n\n' "$PROMPT"; gemini --yolo -p "$PROMPT"; } | tee "$LOGFILE"
2. レビュー結果を精査し必要な部分のみ取り込む
3. touch .claude/gemini_approved でゲート解除
4. 実装完了後 rm .claude/gemini_approved
MSG
  exit 2
fi

exit 0
