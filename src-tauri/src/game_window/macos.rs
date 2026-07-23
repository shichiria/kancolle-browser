use tauri::{AppHandle, Webview, WebviewBuilder, Window, Wry};
use url::Url;

pub(crate) const TITLEBAR_HEIGHT: f64 = 28.0;

const GAME_DATA_STORE_ID: [u8; 16] = [
    0x6b, 0x61, 0x6e, 0x63, 0x6f, 0x6c, 0x6c, 0x65, // "kancolle"
    0x2d, 0x62, 0x72, 0x6f, 0x77, 0x73, 0x65, 0x72, // "-browser"
];

pub(crate) async fn initialization_script(_app: &AppHandle, base_script: String) -> String {
    base_script
}

pub(crate) fn configure_webview(
    builder: WebviewBuilder<Wry>,
    _app: &AppHandle,
    proxy_port: u16,
) -> Result<WebviewBuilder<Wry>, String> {
    let proxy_url =
        Url::parse(&format!("http://127.0.0.1:{proxy_port}")).map_err(|error| error.to_string())?;
    Ok(builder
        .proxy_url(proxy_url)
        .data_store_identifier(GAME_DATA_STORE_ID))
}

pub(crate) async fn prepare_navigation(app: &AppHandle, webview: &Webview<Wry>) {
    crate::cookie::restore_cookies_native(app, webview).await;
}

pub(crate) fn install_diagnostics(_webview: &Webview<Wry>) -> Result<(), String> {
    Ok(())
}

pub(crate) fn install_input_tracking(
    _app: &AppHandle,
    _game_window: &Window<Wry>,
) -> Result<(), String> {
    Ok(())
}

pub(crate) fn set_muted(webview: &Webview<Wry>, muted: bool) -> Result<(), String> {
    use objc2::msg_send;
    use objc2::runtime::AnyObject;

    let muted_state: u64 = if muted { 1 } else { 0 }; // _WKMediaAudioMuted = 1 << 0
    webview
        .with_webview(move |webview| unsafe {
            let wk: *mut AnyObject = webview.inner().cast();
            let _: () = msg_send![wk, _setPageMuted: muted_state];
        })
        .map_err(|error| error.to_string())
}
