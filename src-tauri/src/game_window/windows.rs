use log::info;
use std::path::Path;
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
    // `*.dmmapis.com` includes DMM's payment gateway (`gw.dmmapis.com`). Keep it off
    // the interception proxy, consistent with the other DMM-owned hosts; only
    // kancolle-server.com needs MITM for KanColle API capture.
    let browser_args = format!(
        "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection \
         --proxy-server=http://127.0.0.1:{proxy_port} \
         --proxy-bypass-list=*.dmm.com;*.dmm-corp.com;*.dmm.co.jp;*.dmmgames.com;*.dmmapis.com"
    );

    Ok(builder
        .data_directory(data_dir)
        .additional_browser_args(&browser_args))
}

pub(crate) async fn prepare_navigation(_app: &AppHandle, _webview: &Webview<Wry>) {}

pub(crate) fn install_diagnostics(webview: &Webview<Wry>) -> Result<(), String> {
    crate::game_window::webview_diagnostics::install(webview)
}

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

pub(crate) fn save_screenshot(window: &Window<Wry>, path: &Path) -> Result<(), String> {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::Graphics::Gdi::{
        ClientToScreen, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC,
        GetDIBits, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
    };
    use windows_sys::Win32::Storage::Xps::PrintWindow;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetClientRect, GetWindowRect, PW_RENDERFULLCONTENT,
    };

    let hwnd = window.hwnd().map_err(|error| error.to_string())?.0;
    unsafe {
        let mut window_rect: windows_sys::Win32::Foundation::RECT = std::mem::zeroed();
        let mut client_rect: windows_sys::Win32::Foundation::RECT = std::mem::zeroed();
        if GetWindowRect(hwnd, &mut window_rect) == 0 {
            return Err("GetWindowRect failed".to_string());
        }
        if GetClientRect(hwnd, &mut client_rect) == 0 {
            return Err("GetClientRect failed".to_string());
        }
        let outer_width = window_rect.right - window_rect.left;
        let outer_height = window_rect.bottom - window_rect.top;
        let client_width = client_rect.right - client_rect.left;
        let client_height = client_rect.bottom - client_rect.top;
        if outer_width <= 0 || outer_height <= 0 || client_width <= 0 || client_height <= 0 {
            return Err(format!(
                "Invalid game window size: outer={outer_width}x{outer_height}, client={client_width}x{client_height}"
            ));
        }
        let mut client_origin = POINT { x: 0, y: 0 };
        if ClientToScreen(hwnd, &mut client_origin) == 0 {
            return Err("ClientToScreen failed".to_string());
        }

        let window_dc = GetDC(hwnd);
        if window_dc.is_null() {
            return Err("GetDC failed".to_string());
        }
        let memory_dc = CreateCompatibleDC(window_dc);
        if memory_dc.is_null() {
            ReleaseDC(hwnd, window_dc);
            return Err("CreateCompatibleDC failed".to_string());
        }
        let bitmap = CreateCompatibleBitmap(window_dc, outer_width, outer_height);
        if bitmap.is_null() {
            DeleteDC(memory_dc);
            ReleaseDC(hwnd, window_dc);
            return Err("CreateCompatibleBitmap failed".to_string());
        }
        let previous_bitmap = SelectObject(memory_dc, bitmap);

        if PrintWindow(hwnd, memory_dc, PW_RENDERFULLCONTENT) == 0 {
            SelectObject(memory_dc, previous_bitmap);
            DeleteObject(bitmap);
            DeleteDC(memory_dc);
            ReleaseDC(hwnd, window_dc);
            return Err("PrintWindow failed".to_string());
        }

        let mut bitmap_info: BITMAPINFO = std::mem::zeroed();
        bitmap_info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bitmap_info.bmiHeader.biWidth = outer_width;
        bitmap_info.bmiHeader.biHeight = -outer_height;
        bitmap_info.bmiHeader.biPlanes = 1;
        bitmap_info.bmiHeader.biBitCount = 32;
        bitmap_info.bmiHeader.biCompression = BI_RGB;
        let mut pixels = vec![0_u8; (outer_width * outer_height) as usize * 4];
        let copied = GetDIBits(
            memory_dc,
            bitmap,
            0,
            outer_height as u32,
            pixels.as_mut_ptr().cast(),
            &mut bitmap_info,
            DIB_RGB_COLORS,
        );

        SelectObject(memory_dc, previous_bitmap);
        DeleteObject(bitmap);
        DeleteDC(memory_dc);
        ReleaseDC(hwnd, window_dc);

        if copied == 0 {
            return Err("GetDIBits failed".to_string());
        }
        for pixel in pixels.chunks_exact_mut(4) {
            pixel.swap(0, 2);
        }
        let image = image::RgbaImage::from_raw(outer_width as u32, outer_height as u32, pixels)
            .ok_or_else(|| "Failed to create screenshot image".to_string())?;
        let client_left = (client_origin.x - window_rect.left).max(0) as u32;
        let client_top = (client_origin.y - window_rect.top).max(0) as u32;
        let client_width = client_width as u32;
        let client_height = client_height as u32;
        let expected_game_height = (client_width as f64 * crate::game_window::GAME_HEIGHT
            / crate::game_window::GAME_WIDTH)
            .round() as u32;
        let game_height = expected_game_height.min(client_height);
        let game_top = client_top + client_height - game_height;
        let game = image::DynamicImage::ImageRgba8(image).crop_imm(
            client_left,
            game_top,
            client_width,
            game_height,
        );
        game.save(path)
            .map_err(|error| format!("PNGの保存に失敗しました: {error}"))
    }
}
