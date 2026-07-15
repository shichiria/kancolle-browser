use log::info;
use tauri::{AppHandle, Manager, Webview, WebviewBuilder, Window, Wry};

pub(crate) const TITLEBAR_HEIGHT: f64 = 0.0;

pub(crate) async fn initialization_script(app: &AppHandle, base_script: String) -> String {
    let restore_script = crate::cookie::build_cookie_restore_script(app).await;
    format!("{base_script}\n{restore_script}")
}

pub(crate) fn configure_webview(
    builder: WebviewBuilder<Wry>,
    app: &AppHandle,
    proxy_port: u16,
) -> Result<WebviewBuilder<Wry>, String> {
    let data_dir = app
        .path()
        .app_local_data_dir()
        .map(|dir| dir.join("local").join("game-webview"))
        .map_err(|error| error.to_string())?;
    let browser_args = format!(
        "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection \
         --proxy-server=http://127.0.0.1:{proxy_port} \
         --proxy-bypass-list=*.dmm.com;*.dmm-corp.com;*.dmm.co.jp;*.dmmgames.com"
    );

    Ok(builder
        .data_directory(data_dir)
        .additional_browser_args(&browser_args))
}

pub(crate) async fn prepare_navigation(_app: &AppHandle, _webview: &Webview<Wry>) {}

pub(crate) fn install_input_tracking(
    app: &AppHandle,
    game_window: &Window<Wry>,
) -> Result<(), String> {
    let hwnd = game_window.hwnd().map_err(|error| error.to_string())?;
    let data_dir = app
        .path()
        .app_local_data_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."));

    match crate::mouse_hook::install(hwnd.0 as isize) {
        Ok(receiver) => {
            info!("Mouse hook installed for game window");
            let click_app = app.clone();
            tauri::async_runtime::spawn(async move {
                crate::mouse_hook::consume_clicks(receiver, click_app, data_dir).await;
            });
        }
        Err(error) => log::warn!("Failed to install mouse hook: {error}"),
    }
    Ok(())
}

pub(crate) fn set_muted(webview: &Webview<Wry>, muted: bool) -> Result<(), String> {
    use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2_8;
    use windows_core::Interface;

    webview
        .with_webview(move |webview| unsafe {
            let controller = webview.controller();
            if let Ok(core) = controller.CoreWebView2() {
                if let Ok(core8) = core.cast::<ICoreWebView2_8>() {
                    let _ = core8.SetIsMuted(muted);
                }
            }
        })
        .map_err(|error| error.to_string())
}
