# DMM page shim policy

KanColle Browserは `game-content` WebViewのDMMトップフレームへ、
`src-tauri/src/game_init.js` を初期化scriptとして適用する。このshimの用途は次に限定する。

- DMMのラッパーUIを隠し、`#game_frame` をゲームウィンドウへ固定配置する
- アプリ固有の操作バーをDMMトップフレームへ追加する
- クロスオリジンのKanColle iframe文書にはアクセスしない
- ゲーム操作の自動化、入力イベント生成、ゲーム状態の読取りを行わない

ゲーム幅・高さ・操作バー高さは `game_window.rs` の定数を唯一の情報源とし、
script生成時にプレースホルダーを展開する。DMM DOMのレイアウトスナップショットは
debug buildでのみ診断ログへ送信し、release buildでは無効化する。

DMM側の構造変更へ対応するときは、shimの責務を広げず、`#game_frame` の配置に必要な
最小限のDOM変更に留める。
