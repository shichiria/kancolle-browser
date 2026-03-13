# kancolle-browser SPEC Index

## 基本設計

| ドキュメント | 概要 |
|-------------|------|
| [architecture.md](./architecture.md) | システム構成・技術スタック・モジュール構成・通信フロー・デプロイ構成 |
| [data-flow.md](./data-flow.md) | API傍受→パース→GameState→UI表示の全体データフロー |
| [platform.md](./platform.md) | macOS/Windows プラットフォーム差異・CA証明書・既知の制約 |

## 詳細設計

| ドキュメント | 概要 |
|-------------|------|
| [api-intercept.md](./api-intercept.md) | API傍受・process_api()ディスパッチ・DTO・イベント発行 |
| [battle-log.md](./battle-log.md) | BattleLogger状態管理・戦闘データ解析・ファイルI/O |
| [expedition.md](./expedition.md) | 遠征定義・大成功判定・帰還通知 |
| [quest-progress.md](./quest-progress.md) | 任務進捗追跡・リセットロジック・永続化 |
| [sortie-quest.md](./sortie-quest.md) | 出撃任務条件チェッカー・海域ルート推奨 |
| [improvement.md](./improvement.md) | 装備改修データ・曜日別表示・改修履歴 |
| [senka.md](./senka.md) | 戦果計算・ランキング復号・チェックポイント |
| [drive-sync.md](./drive-sync.md) | Google Drive同期・OAuth2認証・競合解決 |
| [overlay.md](./overlay.md) | オーバーレイ（ミニマップ・陣形・大破警告・遠征通知） |
| [frontend.md](./frontend.md) | Reactコンポーネント階層・状態管理・型定義・CSS設計 |

