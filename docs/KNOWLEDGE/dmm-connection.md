# DMM 接続 (WebView2)

## 症状

WebView2 で `https://play.games.dmm.com/game/kancolle` にアクセスすると、ログイン成功 → ゲームページ到達 → 数秒以内に `accounts.dmm.com/service/login/password` に強制リダイレクト、を無限ループする。Edge ブラウザ直接アクセスでは動作する。

## 原因

DMM が **`navigator.userAgentData.brands`** で WebView2 を識別し、ブラウザ以外と判定された場合は認証Cookieが揃っていてもログインページへリダイレクトする。User-Agent ヘッダ単独の偽装では Sec-CH-UA / userAgentData は変わらないため不十分。

## 対処

`src/game_init.js` の冒頭で `Object.defineProperty(navigator, 'userAgentData', ...)` で Edge ブラウザの brands を返すよう偽装する。あわせて `WebviewBuilder::user_agent(...)` で UA ヘッダも Edge に合わせる。

Edge のバージョンは固定しない。Windows では `tauri::webview_version()` からインストール済み WebView2 の完全バージョンを取得し、UA ヘッダ、`brands`、`uaFullVersion`、`fullVersionList` を同じ値から生成する。固定値は WebView2 の自動更新後に実ランタイムとの不一致を起こし、同じログインループを再発させる。

`WebviewBuilder::user_agent(...)` と JavaScript の `navigator.userAgentData` 偽装だけでは、HTTP の User-Agent Client Hints (`Sec-CH-UA` など) に WebView2 のブランドが残る。DMMへの初回遷移前に CDP の `Emulation.setUserAgentOverride` を `userAgentMetadata` 付きで適用し、HTTPヘッダとJavaScriptから見える値を同じEdge識別情報へ揃える。

## 関連: Proxy 設定

WebView2 + hudsucker proxy の組み合わせで `play.games.dmm.com:443` への CONNECT トンネル中継が `os error 10054 (WSAECONNRESET)` で切られる事象がある（DMM 側の挙動変更が原因と思われ、本家 hudsucker 0.24 / ideamans-hudsucker 0.25 + http2 のいずれでも再現する）。

回避策として WebView2 の **`--proxy-bypass-list`** で DMM ドメインを bypass し、kancolle-server.com のみ proxy 経由で MITM することで API 取得は維持できる。

### `additional_browser_args` の罠

wry の実装上、`additional_browser_args` を渡すと `proxy_url` の設定が**完全に無視される**（`webview2/mod.rs` の `unwrap_or_else` で proxy が組み込まれるのは args 未指定時のみ）。`additional_browser_args` を使う場合は `--proxy-server=...` も自分で含める必要がある。

```rust
let browser_args = format!(
    "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection \
     --proxy-server=http://127.0.0.1:{} \
     --proxy-bypass-list=*.dmm.com;*.dmm-corp.com;*.dmm.co.jp;*.dmmgames.com",
    proxy_port
);
```

macOS (WKWebView) は `proxy_url` のままで問題ないため、`additional_browser_args` は Windows のみで使用する。

## 関連コード

- `src-tauri/src/game_init.js` — userAgentData 偽装
- `src-tauri/src/game_window/` — user_agent / OS別proxy・browser args設定
- `src-tauri/src/proxy/mod.rs` — `should_intercept` で kancolle-server.com のみ MITM
