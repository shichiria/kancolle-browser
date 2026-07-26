use chrono::Local;
use log::{error, info};
use std::fs;
use std::path::PathBuf;

/// Write a raw API dump outside of the GameState lock.
pub(crate) fn save_to_disk(
    directory: &PathBuf,
    filename: &str,
    endpoint: &str,
    request_body: &str,
    response_body: &str,
) -> bool {
    if let Err(error) = fs::create_dir_all(directory) {
        error!("Failed to create raw API dir: {error}");
        return false;
    }

    let path = directory.join(filename);
    let redacted_request = redact_request_body(request_body);
    let dump = serde_json::json!({
        "endpoint": endpoint,
        "timestamp": Local::now().to_rfc3339(),
        "request_body_length": request_body.len(),
        "request_body": redacted_request,
        "response_body_length": response_body.len(),
        // Preserve the exact decoded API JSON text as well as a parsed view.
        // The raw text is what lets future parser fixes reproduce edge cases.
        "response_body_raw": response_body,
        "response_body": serde_json::from_str::<serde_json::Value>(response_body)
            .unwrap_or_else(|_| serde_json::Value::String(response_body.to_string())),
    });

    match serde_json::to_string_pretty(&dump) {
        Ok(json) => match crate::log_io::write_file(&path, json.as_bytes()) {
            Ok(()) => {
                info!("Raw API saved: {filename}");
                true
            }
            Err(error) => {
                error!("Failed to write raw API dump {filename}: {error}");
                false
            }
        },
        Err(error) => {
            error!("Failed to serialize raw API dump: {error}");
            false
        }
    }
}

fn redact_request_body(request_body: &str) -> String {
    match serde_urlencoded::from_str::<Vec<(String, String)>>(request_body) {
        Ok(mut fields) => {
            for (key, value) in &mut fields {
                if crate::sensitive::is_sensitive_key(key) {
                    *value = "<redacted>".to_string();
                }
            }
            serde_urlencoded::to_string(fields)
                .unwrap_or_else(|_| crate::diagnostics::redact_sensitive(request_body))
        }
        Err(_) => crate::diagnostics::redact_sensitive(request_body),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn request_redaction_keeps_non_secret_parameters() {
        let body = "api_token=super-secret&rpctoken=rpc-secret&st=session-secret&api_verno=1&api_deck_id=2";
        let safe = redact_request_body(body);
        let fields: HashMap<String, String> = serde_urlencoded::from_str(&safe).unwrap();

        assert_eq!(
            fields.get("api_token").map(String::as_str),
            Some("<redacted>")
        );
        assert_eq!(fields.get("api_verno").map(String::as_str), Some("1"));
        assert_eq!(fields.get("api_deck_id").map(String::as_str), Some("2"));
        assert!(!safe.contains("super-secret"));
        assert!(!safe.contains("rpc-secret"));
        assert!(!safe.contains("session-secret"));
    }

    #[test]
    fn raw_dump_keeps_exact_response_text() {
        let marker = format!(
            "kancolle-browser-raw-test-{}-{}",
            std::process::id(),
            Local::now().timestamp_nanos_opt().unwrap_or_default()
        );
        let dir = std::env::temp_dir().join(marker);
        let response = r#"{"api_result":1,"api_data":{"value":1.0}}"#;

        assert!(save_to_disk(
            &dir,
            "sample.json",
            "/kcsapi/api_test",
            "api_verno=1",
            response
        ));

        let saved: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(dir.join("sample.json")).unwrap()).unwrap();
        assert_eq!(saved["response_body_raw"], response);
        assert_eq!(saved["response_body"]["api_data"]["value"], 1.0);

        let _ = fs::remove_dir_all(dir);
    }
}
