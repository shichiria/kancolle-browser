## Dev
- 起動: `npm run tauri dev`
- 1420ポート競合時: `taskkill`
- API開発: 保存済み全ログから仕様確認・実装

## Code
- コード検索前: 必ず `docs/CODEMAPS/` でモジュール構成等を把握すること

## Rules (禁止)
- 艦これの自動化 (BOT規約違反のため)
- 任務条件の独自推測。`Progress{SpecialBattle,Practice}.cs` を第一次ソースとし、未収録任務のみ wiki/任務名からの補完可 (出所コメント必須。詳細: docs/SPEC/sortie-quest.md)
- `--proxy-server` を自前指定しない `additional_browser_args` の使用 (wry は本引数指定時に proxy_url を無視する。使用時は必ずプロキシ引数を含めること。現行の使用例: game_window.rs)
