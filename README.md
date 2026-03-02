# KanColle Browser(仮)

艦隊これくしょん（艦これ）用のクロスプラットフォーム専用ブラウザです。リアルタイムの艦隊管理、戦闘分析、遠征管理などの機能を備えています。

[Tauri v2](https://tauri.app/) + React + TypeScript で構築。

> **Note:** HTTPS通信の傍受によるAPI取得のため、初回起動時にCA証明書のインストールが必要です。macOSではキーチェーンへの登録確認、WindowsではUACダイアログが表示されます。


## 機能

- **艦隊ダッシュボード** — 全4艦隊のリアルタイム表示（HP・燃料・弾薬・士気）
- **遠征チェッカー** — 成功/大成功の判定とカウントダウンタイマー
- **出撃任務チェッカー** — 314以上の任務に対応した編成条件の自動判定
- **海域ルート推奨編成** — 通常海域（1-1〜7-5）のルート制御用推奨編成チェッカー
- **戦闘ログ** — HP/ダメージ・陣形・制空値の記録とマップルート可視化
- **装備改修トラッカー** — 曜日別の改修可否カレンダーと改修履歴
- **艦娘・装備一覧** — フィルタ付きの艦娘リストと装備リスト
- **Google Drive 同期** — 任務進捗・改修履歴・戦闘ログのクラウドバックアップ（任意）
- **クロスプラットフォーム** — macOS（WKWebView + プロキシ）/ Windows（WebView2 + ネイティブAPI傍受）

## 基本方針（特徴）
- デフォルトで使いやすくする
- UIは凝らない
- 普段の艦これを便利にする

## インストール方法
[https://github.com/shichiria/kancolle-browser/releases](https://github.com/shichiria/kancolle-browser/releases)に遷移する。

- **Mac**: `KanColle.Browser_[バージョン]_universal.dmg`
- **Windows**: `KanColle.Browser_[バージョン]_x64_en-US.msi`

上記ファイルをダウンロードしてインストールしてください。macOS版は下記の「macOS でのインストール」も参照してください。

## macOS でのインストール

本アプリはコード署名されていないため、ダウンロード後にそのままダブルクリックすると「開発元を確認できないため開けません」と表示されます。

以下の手順で開いてください:

1. アプリインストール後にダブルクリック
2. KanColle Browserは開いていません・・・のダイアログを出し、完了を押す
3. 設定からプライバシーとセキュリティを開く
4. 一番下にスクロールして、お使いのMacを保護するために・・・でこのまま開くを押す
5. さらにダイアログが出てくるのでこのまま開くを押す

この操作は初回のみ必要です。2回目以降は通常通りダブルクリックで起動できます。

## 注意点
まだベータ版なのでデータ構造が変わって前のバージョンのログが読み込めなくなる可能性があります。

開発後にまだイベントがないため、イベント海域では使用しないでください。
連合艦隊など動作がおかしくなる可能性があります。


## 証明書のアンインストール方法

特に残しておいても実害はないので気になる方だけアンインストールしてください。

### Windows
  1. Win + R → certmgr.msc を開く
  2. 信頼されたルート証明機関 → 証明書 を展開
  3. 「KanColle Browser CA」を探して右クリック → 削除

### macOS
  1. Keychain Access.app を開く
  2. 証明書 を選択
  3. 「KanColle Browser CA」を探して右クリック → 削除


## 必要環境

- [Rust](https://www.rust-lang.org/tools/install) 1.70+
- [Node.js](https://nodejs.org/) 18+
- プラットフォーム固有:
  - **macOS**: Xcode Command Line Tools
  - **Windows**: [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)（C++ ワークロード）、WebView2 ランタイム

## ビルド・起動

```bash
# フロントエンドの依存関係をインストール
npm install

# 開発モードで起動
npm run tauri dev

# プロダクションビルド
npm run tauri build
```

## アーキテクチャ

```
src/                    # React フロントエンド
  App.tsx               # メインUI（母港・戦闘・改修・艦娘・装備・設定タブ）
  App.css               # スタイル

src-tauri/src/          # Rust バックエンド
  lib.rs                # Tauri コマンド定義
  api/                  # ゲーム状態管理・APIパース
  proxy/                # API傍受用MITMプロキシ（macOS）
  battle_log/           # 戦闘データの記録・保存
  quest_progress/       # 任務進捗の追跡
  sortie_quest/         # 出撃・演習任務の条件チェッカー
  expedition/           # 遠征条件の判定
  improvement/          # 装備改修の追跡
  drive_sync/           # Google Drive 同期

src-tauri/data/         # 静的ゲームデータ
  expeditions.json      # 遠征定義（58以上）
  sortie_quests.json    # 任務条件（314以上）
  equipment_upgrades.json  # 装備改修カレンダー（366以上）
  map_recommendations.json # 通常海域の推奨編成
  edges.json            # マップ辺トポロジ
```

### API 傍受方式

- **macOS**: [hudsucker](https://github.com/omjadas/hudsucker) によるMITMプロキシ（ポート19080、CA証明書を自動インストール）
- **Windows**: WebView2 のネイティブAPI傍受（プロキシ不要）


## 免責事項

本ツールはゲームデータの閲覧・分析のみを行います。ゲーム操作の自動化は**一切行いません**。

「艦隊これくしょん」は株式会社KADOKAWAおよび合同会社DMM.comの登録商標です。本プロジェクトはいずれの企業とも提携・推奨の関係にありません。

## ライセンス

[MIT](LICENSE)
