<!-- Generated: 2026-03-13 | Files scanned: 3 | Token estimate: ~500 -->
# Dependencies

## Rust (Cargo.toml)
### Core
- tauri 2 (tray-icon, macos-proxy, macos-private-api, unstable)
- tauri-plugin-opener 2
- tokio (full) — async runtime
- serde/serde_json — serialization

### Networking
- hudsucker 0.24 (rcgen-ca, rustls-client) — HTTP proxy
- http 1, hyper 1, http-body-util — HTTP primitives
- rustls 0.23 — TLS
- url 2 — URL parsing

### Google Drive
- google-drive3 7.0 — Drive API
- yup-oauth2 12 — OAuth2
- hyper-util, hyper-rustls — HTTP client for Drive

### Utilities
- chrono (serde) — date/time
- flate2 — gzip decompression
- brotli 8 — brotli decompression
- base64 0.22, md-5 0.10, image 0.25 (png)
- dirs 6, open 5, mime, serde_urlencoded
- log, env_logger — logging

### Platform-specific
- macOS: objc2, objc2-foundation (NSValue)
- Windows: webview2-com 0.38, windows-core 0.61

## Frontend (package.json)
### Runtime
- react 19.1, react-dom 19.1
- @tauri-apps/api 2, @tauri-apps/plugin-opener 2

### Dev
- typescript 5.8, vite 7, @vitejs/plugin-react 4
- @types/react 19.1, @types/react-dom 19.1
- @tauri-apps/cli 2

## External Services
- Google Drive API (sync)
- KanColle game servers (via proxy)
