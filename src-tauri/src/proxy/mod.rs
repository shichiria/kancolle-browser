use flate2::read::GzDecoder;
use http_body_util::BodyExt;
use hudsucker::{
    certificate_authority::RcgenAuthority,
    hyper::{Request, Response},
    rcgen::{CertificateParams, KeyPair},
    rustls::crypto::aws_lc_rs,
    Body, HttpContext, HttpHandler, Proxy, RequestOrResponse,
};
use log::{error, info};
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};
use tokio::sync::oneshot;

use crate::api;

/// API intercept event payload sent to the frontend
#[derive(Clone, serde::Serialize)]
pub struct ApiEvent {
    pub endpoint: String,
    pub request_body: String,
    pub response_body: String,
}

/// Per-connection request data (URI and request body) keyed by client address.
/// Within a single HTTP/1.1 connection, requests are processed sequentially,
/// so using SocketAddr as the key safely isolates concurrent connections.
type RequestDataMap = Arc<Mutex<HashMap<SocketAddr, (String, String)>>>;

/// The proxy handler that intercepts KanColle API calls.
#[derive(Clone)]
struct KanColleHandler {
    app_handle: AppHandle,
    request_data: RequestDataMap,
    cache_dir: PathBuf,
}

impl HttpHandler for KanColleHandler {
    /// Only MITM (intercept) HTTPS connections to game servers.
    /// Game servers use domains like wXXy.kancolle-server.com (since Dec 2024).
    /// DMM login/CDN connections are tunneled through without decryption.
    async fn should_intercept(
        &mut self,
        _ctx: &HttpContext,
        req: &Request<Body>,
    ) -> bool {
        // CONNECT requests have URI in "host:port" format
        let host = req.uri().authority().map(|a| a.host().to_string())
            .or_else(|| {
                let s = req.uri().to_string();
                s.split(':').next().map(|h| h.to_string())
            })
            .unwrap_or_default();

        // MITM game servers (kancolle-server.com) and legacy IP addresses
        let intercept = host.ends_with(".kancolle-server.com")
            || host == "kancolle-server.com"
            || host.parse::<std::net::IpAddr>().is_ok();

        if intercept {
            info!("MITM intercept: {}", req.uri());
        }
        intercept
    }

    async fn handle_request(
        &mut self,
        ctx: &HttpContext,
        req: Request<Body>,
    ) -> RequestOrResponse {
        let uri = req.uri().to_string();

        // Capture request body for battle log (POST data contains api_deck_id, etc.)
        if req.uri().path().contains("/kcsapi/") {
            let (parts, body) = req.into_parts();
            let body_bytes = match body.collect().await {
                Ok(collected) => collected.to_bytes(),
                Err(_) => {
                    return RequestOrResponse::Request(Request::from_parts(parts, Body::empty()));
                }
            };
            let body_str = String::from_utf8_lossy(&body_bytes).to_string();
            self.request_data.lock().unwrap().insert(ctx.client_addr, (uri, body_str));

            let full_body = http_body_util::Full::new(body_bytes);
            RequestOrResponse::Request(Request::from_parts(parts, Body::from(full_body)))
        } else {
            self.request_data.lock().unwrap().insert(ctx.client_addr, (uri, String::new()));
            RequestOrResponse::Request(req)
        }
    }

    async fn handle_response(
        &mut self,
        ctx: &HttpContext,
        res: Response<Body>,
    ) -> Response<Body> {
        // Retrieve and remove per-connection request data to prevent memory leaks
        let (uri, req_body) = self.request_data.lock().unwrap()
            .remove(&ctx.client_addr)
            .unwrap_or_default();

        // Cache non-API resources (images, JSON, JS, CSS, etc.) for offline use
        if !uri.contains("/kcsapi/") {
            self.maybe_cache_resource(&uri, res).await
        } else {
            self.handle_api_response(uri, req_body, res).await
        }
    }

}

impl KanColleHandler {
    async fn handle_api_response(
        &self,
        uri: String,
        req_body: String,
        res: Response<Body>,
    ) -> Response<Body> {
        let endpoint = if let Some(pos) = uri.find("/kcsapi/") {
            uri[pos..].to_string()
        } else {
            return res;
        };

        // Strip query string if present
        let endpoint = if let Some(pos) = endpoint.find('?') {
            endpoint[..pos].to_string()
        } else {
            endpoint
        };

        info!("Intercepted KanColle API: {}", endpoint);

        // Read the response body
        let (parts, body) = res.into_parts();
        let body_bytes = match body.collect().await {
            Ok(collected) => collected.to_bytes(),
            Err(e) => {
                error!("Failed to read response body: {}", e);
                return Response::from_parts(parts, Body::empty());
            }
        };

        // Check for gzip Content-Encoding and decompress if needed
        let is_gzip = parts
            .headers
            .get("content-encoding")
            .and_then(|v| v.to_str().ok())
            .map(|v| v.contains("gzip"))
            .unwrap_or(false);

        let decoded_bytes = if is_gzip {
            match decompress_gzip(&body_bytes) {
                Ok(decompressed) => decompressed,
                Err(e) => {
                    error!("Failed to decompress gzip response for {}: {}", endpoint, e);
                    body_bytes.to_vec()
                }
            }
        } else {
            body_bytes.to_vec()
        };

        let body_str = String::from_utf8_lossy(&decoded_bytes).to_string();

        // KanColle API responses are prefixed with "svdata="
        let json_str = if body_str.starts_with("svdata=") {
            body_str[7..].to_string()
        } else {
            body_str.clone()
        };

        // Process the API data
        api::process_api(&self.app_handle, &endpoint, &json_str, &req_body);

        // Emit raw event to frontend
        let event = ApiEvent {
            endpoint: endpoint.clone(),
            request_body: req_body,
            response_body: json_str,
        };

        if let Err(e) = self.app_handle.emit("kancolle-api", &event) {
            error!("Failed to emit API event: {}", e);
        }

        // Reconstruct with Full<Bytes> body
        let full_body = http_body_util::Full::new(body_bytes);
        Response::from_parts(parts, Body::from(full_body))
    }

    /// Cache game resources (images, JSON, JS, CSS) from proxy responses.
    /// Only caches HTTP 200 responses. Files are written asynchronously
    /// to avoid blocking the response pipeline.
    async fn maybe_cache_resource(&self, uri: &str, res: Response<Body>) -> Response<Body> {
        // Only cache successful responses
        if res.status() != http::StatusCode::OK {
            return res;
        }

        // Extract the path from the URI, stripping query string
        let path = if let Some(pos) = uri.find('?') {
            &uri[..pos]
        } else {
            uri
        };

        // Only cache /kcs2/ resources (game assets)
        if !path.contains("/kcs2/") {
            return res;
        }

        // Extract relative path starting from kcs2/
        let rel_path = if let Some(pos) = path.find("/kcs2/") {
            &path[pos + 1..] // skip leading '/' to get "kcs2/..."
        } else {
            return res;
        };

        let cache_path = self.cache_dir.join(rel_path);

        // Skip if already cached (unless version changed)
        if cache_path.exists() {
            return res;
        }

        // Read the response body to cache it
        let (parts, body) = res.into_parts();
        let body_bytes = match body.collect().await {
            Ok(collected) => collected.to_bytes(),
            Err(e) => {
                error!("Failed to read resource body for caching: {}", e);
                return Response::from_parts(parts, Body::empty());
            }
        };

        // Decompress gzip/brotli if needed for storage
        let content_encoding = parts
            .headers
            .get("content-encoding")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_lowercase();

        let data_to_cache = if content_encoding.contains("gzip") {
            match decompress_gzip(&body_bytes) {
                Ok(decompressed) => decompressed,
                Err(e) => {
                    error!("Failed to decompress gzip resource {}: {}", rel_path, e);
                    body_bytes.to_vec()
                }
            }
        } else if content_encoding.contains("br") {
            match decompress_brotli(&body_bytes) {
                Ok(decompressed) => decompressed,
                Err(e) => {
                    error!("Failed to decompress brotli resource {}: {}", rel_path, e);
                    body_bytes.to_vec()
                }
            }
        } else {
            body_bytes.to_vec()
        };

        // Write to disk asynchronously (don't block response)
        let cache_path_owned = cache_path.clone();
        let rel_path_owned = rel_path.to_string();
        tokio::spawn(async move {
            if let Some(parent) = cache_path_owned.parent() {
                if let Err(e) = tokio::fs::create_dir_all(parent).await {
                    error!("Failed to create cache dir for {}: {}", rel_path_owned, e);
                    return;
                }
            }
            match tokio::fs::write(&cache_path_owned, &data_to_cache).await {
                Ok(_) => info!("Cached resource: {} ({} bytes)", rel_path_owned, data_to_cache.len()),
                Err(e) => error!("Failed to cache resource {}: {}", rel_path_owned, e),
            }
        });

        // Return original (possibly gzip-compressed) body to client
        let full_body = http_body_util::Full::new(body_bytes);
        Response::from_parts(parts, Body::from(full_body))
    }
}

// ---------------------------------------------------------------------------
// CA certificate persistence (PEM format)
// ---------------------------------------------------------------------------

/// Directory for storing CA certificate files
fn ca_data_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("kancolle-browser")
}

/// Get the path to the CA PEM certificate (for keychain installation)
pub fn ca_pem_path() -> PathBuf {
    ca_data_dir().join("ca.cert.pem")
}

/// Load existing CA from disk or generate a new one.
fn load_or_generate_ca() -> Result<RcgenAuthority, Box<dyn std::error::Error + Send + Sync>> {
    let dir = ca_data_dir();
    let key_pem_path = dir.join("ca.key.pem");
    let cert_pem_path = dir.join("ca.cert.pem");

    if key_pem_path.exists() && cert_pem_path.exists() {
        info!("Loading existing CA from {}", dir.display());
        let key_pem = fs::read_to_string(&key_pem_path)?;
        let cert_pem = fs::read_to_string(&cert_pem_path)?;

        let key_pair =
            KeyPair::from_pem(&key_pem).map_err(|e| format!("Failed to load CA key: {}", e))?;

        let issuer = hudsucker::rcgen::Issuer::from_ca_cert_pem(&cert_pem, key_pair)
            .map_err(|e| format!("Failed to create issuer from PEM: {}", e))?;

        let authority = RcgenAuthority::new(issuer, 1_000, aws_lc_rs::default_provider());
        return Ok(authority);
    }

    // Generate new CA
    info!("Generating new CA certificate in {}", dir.display());
    fs::create_dir_all(&dir)?;

    let key_pair = KeyPair::generate().map_err(|e| format!("KeyPair::generate: {}", e))?;

    let mut params = CertificateParams::default();
    params.is_ca = hudsucker::rcgen::IsCa::Ca(hudsucker::rcgen::BasicConstraints::Unconstrained);
    params
        .distinguished_name
        .push(hudsucker::rcgen::DnType::CommonName, "KanColle Browser CA");
    params.distinguished_name.push(
        hudsucker::rcgen::DnType::OrganizationName,
        "KanColle Browser",
    );

    let cert = params
        .self_signed(&key_pair)
        .map_err(|e| format!("self_signed: {}", e))?;

    // Save PEM files to disk
    fs::write(&key_pem_path, key_pair.serialize_pem())?;
    fs::write(&cert_pem_path, cert.pem())?;
    info!("CA certificate saved to {}", dir.display());

    // Build authority
    let issuer = hudsucker::rcgen::Issuer::from_ca_cert_der(cert.der(), key_pair)
        .map_err(|e| format!("Issuer::from_ca_cert_der: {}", e))?;

    let authority = RcgenAuthority::new(issuer, 1_000, aws_lc_rs::default_provider());
    Ok(authority)
}

// ---------------------------------------------------------------------------
// Proxy server
// ---------------------------------------------------------------------------

/// Start the proxy server and return the port it's listening on.
pub async fn start_proxy(
    app_handle: AppHandle,
    cache_dir: PathBuf,
) -> Result<u16, Box<dyn std::error::Error + Send + Sync>> {
    let ca = load_or_generate_ca()?;

    // Use a fixed port so WKWebView treats proxy as the same origin across restarts
    // (preserving cookies/sessions). Fall back to OS-assigned port if 19080 is in use.
    const PREFERRED_PORT: u16 = 19080;
    let listener = match tokio::net::TcpListener::bind(format!("127.0.0.1:{}", PREFERRED_PORT)).await {
        Ok(l) => l,
        Err(_) => tokio::net::TcpListener::bind("127.0.0.1:0").await?,
    };
    let actual_port = listener.local_addr()?.port();
    drop(listener);

    let bind_addr = SocketAddr::from(([127, 0, 0, 1], actual_port));

    let (port_tx, port_rx) = oneshot::channel();

    tokio::spawn(async move {
        let handler = KanColleHandler {
            app_handle,
            request_data: Arc::new(Mutex::new(HashMap::new())),
            cache_dir,
        };

        let proxy = Proxy::builder()
            .with_addr(bind_addr)
            .with_ca(ca)
            .with_rustls_connector(aws_lc_rs::default_provider())
            .with_http_handler(handler)
            .build()
            .expect("Failed to build proxy");

        let _ = port_tx.send(actual_port);

        info!("Proxy server starting on 127.0.0.1:{}", actual_port);
        if let Err(e) = proxy.start().await {
            error!("Proxy server error: {}", e);
        }
    });

    let port = port_rx.await?;
    Ok(port)
}

/// Decompress gzip-encoded bytes
fn decompress_gzip(data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    let mut decoder = GzDecoder::new(data);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;
    Ok(decompressed)
}

/// Decompress brotli-encoded bytes
fn decompress_brotli(data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    let mut decoder = brotli::Decompressor::new(data, 4096);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;
    Ok(decompressed)
}
