use log::{info, warn};
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use yup_oauth2::authenticator::Authenticator;
use yup_oauth2::authenticator_delegate::InstalledFlowDelegate;

/// Type alias for the authenticator we use.
type Connector = hyper_rustls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>;
pub type DriveAuthenticator = Authenticator<Connector>;

/// Custom delegate that opens the browser automatically for OAuth consent.
struct BrowserFlowDelegate;

impl InstalledFlowDelegate for BrowserFlowDelegate {
    fn present_user_url<'a>(
        &'a self,
        url: &'a str,
        _need_code: bool,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(async move {
            info!("Opening browser for OAuth: {}", url);
            if let Err(e) = open::that(url) {
                warn!("Failed to open browser: {}", e);
            }
            Ok(String::new())
        })
    }
}

/// Google Drive API scope — only access files created by this app.
const DRIVE_SCOPE: &str = "https://www.googleapis.com/auth/drive.file";

/// OAuth 2.0 client credentials (Desktop app — not confidential per Google docs).
const GOOGLE_CLIENT_ID: &str = "1018502336976-phtmhta3sc679d185tgkin6o31s4eafu.apps.googleusercontent.com";
const GOOGLE_CLIENT_SECRET: &str = "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";

/// Returns the embedded client credentials, or None if not yet configured.
pub fn client_credentials() -> Option<(&'static str, &'static str)> {
    if GOOGLE_CLIENT_ID.is_empty() || GOOGLE_CLIENT_SECRET.is_empty() {
        None
    } else {
        Some((GOOGLE_CLIENT_ID, GOOGLE_CLIENT_SECRET))
    }
}

fn build_secret(client_id: &str, client_secret: &str) -> yup_oauth2::ApplicationSecret {
    yup_oauth2::ApplicationSecret {
        client_id: client_id.to_string(),
        client_secret: client_secret.to_string(),
        auth_uri: "https://accounts.google.com/o/oauth2/auth".to_string(),
        token_uri: "https://oauth2.googleapis.com/token".to_string(),
        redirect_uris: vec!["http://localhost".to_string()],
        ..Default::default()
    }
}

/// Build an OAuth2 authenticator using InstalledFlow (opens browser for consent).
/// Token is persisted to `data_dir/google_drive_token.json`.
pub async fn authenticate(
    client_id: &str,
    client_secret: &str,
    data_dir: &Path,
) -> Result<DriveAuthenticator, String> {
    let secret = build_secret(client_id, client_secret);
    let token_path = data_dir.join("google_drive_token.json");

    let auth = yup_oauth2::InstalledFlowAuthenticator::builder(
        secret,
        yup_oauth2::InstalledFlowReturnMethod::HTTPRedirect,
    )
    .persist_tokens_to_disk(&token_path)
    .flow_delegate(Box::new(BrowserFlowDelegate))
    .build()
    .await
    .map_err(|e| format!("Failed to build authenticator: {}", e))?;

    // Force a token request to trigger the browser flow if needed
    let _token = auth
        .token(&[DRIVE_SCOPE])
        .await
        .map_err(|e| format!("Failed to get token: {}", e))?;

    info!("Google Drive authentication successful");
    Ok(auth)
}

/// Check if we have a valid cached token (no browser interaction).
pub async fn try_restore_auth(
    client_id: &str,
    client_secret: &str,
    data_dir: &Path,
) -> Option<DriveAuthenticator> {
    let token_path = data_dir.join("google_drive_token.json");
    if !token_path.exists() {
        return None;
    }

    let secret = build_secret(client_id, client_secret);

    let auth = yup_oauth2::InstalledFlowAuthenticator::builder(
        secret,
        yup_oauth2::InstalledFlowReturnMethod::HTTPRedirect,
    )
    .persist_tokens_to_disk(&token_path)
    .build()
    .await
    .ok()?;

    match auth.token(&[DRIVE_SCOPE]).await {
        Ok(_) => {
            info!("Restored Google Drive auth from cached token");
            Some(auth)
        }
        Err(e) => {
            warn!("Failed to restore auth: {}", e);
            None
        }
    }
}

/// Delete the cached token to log out.
pub fn logout(data_dir: &Path) {
    let token_path = data_dir.join("google_drive_token.json");
    if token_path.exists() {
        let _ = std::fs::remove_file(&token_path);
        info!("Removed Google Drive token");
    }
}

/// Check if a token file exists.
pub fn has_token(data_dir: &Path) -> bool {
    data_dir.join("google_drive_token.json").exists()
}
