//! Read-only WebView2 diagnostics for the DMM container and all child frames.
//!
//! This uses the browser's DevTools Protocol event stream. It does not evaluate
//! JavaScript or modify the cross-origin KanColle/OpenSocial gadget.

use std::cell::RefCell;

use serde_json::{json, Value};
use tauri::{Webview, Wry};
use webview2_com::Microsoft::Web::WebView2::Win32::{
    ICoreWebView2, ICoreWebView2DevToolsProtocolEventReceiver,
};
use webview2_com::{
    take_pwstr, CallDevToolsProtocolMethodCompletedHandler,
    DevToolsProtocolEventReceivedEventHandler,
};
use windows_core::{HSTRING, PWSTR};

const MAX_EVENT_BYTES: usize = 32 * 1024;
const CDP_EVENTS: &[&str] = &[
    "Runtime.exceptionThrown",
    "Runtime.consoleAPICalled",
    "Runtime.executionContextCreated",
    "Log.entryAdded",
    "Network.requestWillBeSent",
    "Network.requestWillBeSentExtraInfo",
    "Network.responseReceived",
    "Network.loadingFailed",
    "Page.frameNavigated",
    "Page.javascriptDialogOpening",
    "Page.javascriptDialogClosed",
];
const CDP_ENABLE_METHODS: &[&str] = &[
    "Runtime.enable",
    "Log.enable",
    "Network.enable",
    "Page.enable",
];

thread_local! {
    // WebView2 event receivers are apartment-bound. Retain them on the UI thread
    // for the lifetime of the game webview so their subscriptions stay active.
    static EVENT_RECEIVERS: RefCell<Vec<ICoreWebView2DevToolsProtocolEventReceiver>> =
        const { RefCell::new(Vec::new()) };
}

pub(super) fn install(webview: &Webview<Wry>) -> Result<(), String> {
    webview
        .with_webview(|webview| unsafe {
            let core = match webview.controller().CoreWebView2() {
                Ok(core) => core,
                Err(error) => {
                    log::error!(target: "webview_cdp", "Cannot access CoreWebView2: {error}");
                    return;
                }
            };

            if let Err(error) = install_for_core(&core) {
                log::error!(target: "webview_cdp", "Failed to install WebView2 diagnostics: {error}");
            }
        })
        .map_err(|error| error.to_string())
}

unsafe fn install_for_core(core: &ICoreWebView2) -> Result<(), String> {
    for &event_name in CDP_EVENTS {
        subscribe(core, event_name)?;
    }
    for &method in CDP_ENABLE_METHODS {
        enable_domain(core, method)?;
    }
    log::info!(
        target: "webview_cdp",
        "WebView2 diagnostics active: {}",
        CDP_EVENTS.join(", ")
    );
    Ok(())
}

unsafe fn subscribe(core: &ICoreWebView2, event_name: &'static str) -> Result<(), String> {
    let receiver = core
        .GetDevToolsProtocolEventReceiver(&HSTRING::from(event_name))
        .map_err(|error| format!("{event_name} receiver: {error}"))?;
    let handler = DevToolsProtocolEventReceivedEventHandler::create(Box::new(move |_, args| {
        let Some(args) = args else {
            return Ok(());
        };
        let mut payload = PWSTR::null();
        unsafe { args.ParameterObjectAsJson(&mut payload)? };
        let payload = take_pwstr(payload);
        if let Some(summary) = summarize_event(event_name, &payload) {
            log::info!(target: "webview_cdp", "[{event_name}] {summary}");
        }
        Ok(())
    }));
    let mut token = 0;
    receiver
        .add_DevToolsProtocolEventReceived(&handler, &mut token)
        .map_err(|error| format!("{event_name} subscription: {error}"))?;
    EVENT_RECEIVERS.with(|receivers| receivers.borrow_mut().push(receiver));
    Ok(())
}

unsafe fn enable_domain(core: &ICoreWebView2, method: &'static str) -> Result<(), String> {
    let handler =
        CallDevToolsProtocolMethodCompletedHandler::create(Box::new(move |result, response| {
            if let Err(error) = result {
                log::error!(target: "webview_cdp", "{method} failed: {error}");
            } else if response.contains("\"error\"") {
                log::warn!(target: "webview_cdp", "{method} response: {response}");
            }
            Ok(())
        }));
    core.CallDevToolsProtocolMethod(&HSTRING::from(method), &HSTRING::from("{}"), &handler)
        .map_err(|error| format!("{method}: {error}"))
}

fn summarize_event(event_name: &str, payload: &str) -> Option<String> {
    let value: Value = match serde_json::from_str(payload) {
        Ok(value) => value,
        Err(error) => {
            return Some(limit_event(format!(
                "{{\"parseError\":{:?},\"payload\":{:?}}}",
                error.to_string(),
                payload
            )))
        }
    };

    let summary = match event_name {
        "Network.requestWillBeSent" => summarize_request(&value)?,
        "Network.requestWillBeSentExtraInfo" => summarize_request_extra_info(&value)?,
        "Network.responseReceived" => summarize_response(&value)?,
        "Network.loadingFailed" => summarize_loading_failed(&value)?,
        "Runtime.consoleAPICalled" => summarize_console(&value),
        "Runtime.executionContextCreated" => json!({
            "context": select_fields(
                &value["context"],
                &["id", "uniqueId", "origin", "name", "auxData"],
            )
        }),
        "Runtime.exceptionThrown" => json!({
            "timestamp": value["timestamp"],
            "exceptionDetails": value["exceptionDetails"],
        }),
        "Log.entryAdded" => summarize_log_entry(&value)?,
        "Page.frameNavigated" => json!({
            "type": value["type"],
            "frame": select_fields(
                &value["frame"],
                &["id", "parentId", "loaderId", "name", "url", "unreachableUrl", "securityOrigin"],
            )
        }),
        "Page.javascriptDialogOpening" => select_fields(
            &value,
            &[
                "url",
                "frameId",
                "message",
                "type",
                "hasBrowserHandler",
                "defaultPrompt",
            ],
        ),
        "Page.javascriptDialogClosed" => select_fields(&value, &["frameId", "result", "userInput"]),
        _ => value,
    };

    serde_json::to_string(&summary).ok().map(limit_event)
}

fn summarize_request_extra_info(value: &Value) -> Option<Value> {
    const AUTH_COOKIE_NAMES: &[&str] = &[
        "althash",
        "login_secure_id",
        "login_session_id",
        "secid",
        "INT_SESID",
        "INT_SESID_SECURE",
    ];
    let cookies = value["associatedCookies"]
        .as_array()?
        .iter()
        .filter(|entry| {
            entry["cookie"]["name"]
                .as_str()
                .is_some_and(|name| AUTH_COOKIE_NAMES.contains(&name))
        })
        .map(|entry| {
            json!({
                "cookie": select_fields(
                    &entry["cookie"],
                    &["name", "domain", "path", "secure", "httpOnly", "sameSite", "expires"],
                ),
                "blockedReasons": entry["blockedReasons"],
                "exemptionReason": entry["exemptionReason"],
            })
        })
        .collect::<Vec<_>>();
    if cookies.is_empty() {
        return None;
    }
    Some(json!({
        "requestId": value["requestId"],
        "associatedAuthCookies": cookies,
        "browserHeaders": select_browser_headers(&value["headers"]),
    }))
}

fn summarize_request(value: &Value) -> Option<Value> {
    let resource_type = value["type"].as_str().unwrap_or_default();
    if !is_diagnostic_resource(resource_type) {
        return None;
    }
    Some(json!({
        "requestId": value["requestId"],
        "loaderId": value["loaderId"],
        "frameId": value["frameId"],
        "documentURL": value["documentURL"],
        "type": value["type"],
        "url": value["request"]["url"],
        "method": value["request"]["method"],
        "hasPostData": value["request"]["hasPostData"],
        "browserHeaders": select_browser_headers(&value["request"]["headers"]),
        "initiator": select_fields(
            &value["initiator"],
            &["type", "url", "lineNumber", "columnNumber", "stack"],
        ),
    }))
}

fn select_browser_headers(headers: &Value) -> Value {
    let mut selected = serde_json::Map::new();
    let Some(headers) = headers.as_object() else {
        return Value::Object(selected);
    };
    for (name, value) in headers {
        if matches!(
            name.to_ascii_lowercase().as_str(),
            "user-agent"
                | "sec-ch-ua"
                | "sec-ch-ua-full-version-list"
                | "sec-ch-ua-platform"
                | "sec-ch-ua-mobile"
        ) {
            selected.insert(name.to_ascii_lowercase(), value.clone());
        }
    }
    Value::Object(selected)
}

fn summarize_log_entry(value: &Value) -> Option<Value> {
    let text = value["entry"]["text"].as_str().unwrap_or_default();
    if text.starts_with("Tracking Prevention blocked access to storage") {
        return None;
    }
    Some(json!({
        "entry": select_fields(
            &value["entry"],
            &["source", "level", "text", "timestamp", "url", "lineNumber", "stackTrace"],
        )
    }))
}

fn summarize_response(value: &Value) -> Option<Value> {
    let resource_type = value["type"].as_str().unwrap_or_default();
    let status = value["response"]["status"].as_f64().unwrap_or_default();
    if !is_diagnostic_resource(resource_type) && status < 400.0 {
        return None;
    }
    Some(json!({
        "requestId": value["requestId"],
        "loaderId": value["loaderId"],
        "frameId": value["frameId"],
        "type": value["type"],
        "response": select_fields(
            &value["response"],
            &["url", "status", "statusText", "mimeType", "protocol", "fromDiskCache", "fromServiceWorker"],
        ),
    }))
}

fn summarize_loading_failed(value: &Value) -> Option<Value> {
    let resource_type = value["type"].as_str().unwrap_or_default();
    let canceled = value["canceled"].as_bool().unwrap_or(false);
    if !is_diagnostic_resource(resource_type) && canceled {
        return None;
    }
    Some(select_fields(
        value,
        &[
            "requestId",
            "loaderId",
            "type",
            "errorText",
            "canceled",
            "blockedReason",
            "corsErrorStatus",
        ],
    ))
}

fn summarize_console(value: &Value) -> Value {
    let args = value["args"]
        .as_array()
        .map(|args| {
            args.iter()
                .map(|arg| {
                    select_fields(
                        arg,
                        &[
                            "type",
                            "subtype",
                            "className",
                            "value",
                            "unserializableValue",
                            "description",
                        ],
                    )
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    json!({
        "type": value["type"],
        "args": args,
        "executionContextId": value["executionContextId"],
        "timestamp": value["timestamp"],
        "stackTrace": value["stackTrace"],
        "context": value["context"],
    })
}

fn select_fields(value: &Value, fields: &[&str]) -> Value {
    let mut selected = serde_json::Map::new();
    for &field in fields {
        if let Some(field_value) = value.get(field) {
            selected.insert(field.to_string(), field_value.clone());
        }
    }
    Value::Object(selected)
}

fn is_diagnostic_resource(resource_type: &str) -> bool {
    matches!(
        resource_type,
        "Document" | "XHR" | "Fetch" | "EventSource" | "WebSocket" | "Other"
    )
}

fn limit_event(mut event: String) -> String {
    if event.len() <= MAX_EVENT_BYTES {
        return event;
    }
    let mut boundary = MAX_EVENT_BYTES;
    while !event.is_char_boundary(boundary) {
        boundary -= 1;
    }
    event.truncate(boundary);
    event.push_str("<truncated>");
    event
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_summary_excludes_headers_cookies_and_post_body() {
        let payload = json!({
            "requestId": "1",
            "frameId": "gadget",
            "type": "XHR",
            "request": {
                "url": "https://example.test/pay",
                "method": "POST",
                "hasPostData": true,
                "postData": "api_token=secret",
                "headers": {
                    "Cookie": "session=secret",
                    "User-Agent": "Edge test",
                    "sec-ch-ua": "Microsoft Edge"
                }
            },
            "initiator": { "type": "script", "url": "shop.js", "lineNumber": 10 }
        })
        .to_string();

        let summary = summarize_event("Network.requestWillBeSent", &payload).unwrap();
        let redacted = crate::diagnostics::redact_sensitive(&summary);
        assert!(redacted.contains("example.test/pay"));
        assert!(redacted.contains("\"method\":\"POST\""));
        assert!(redacted.contains("shop.js"));
        assert!(!redacted.contains("api_token"));
        assert!(!redacted.contains("Cookie"));
        assert!(!redacted.contains("session=secret"));
        assert!(redacted.contains("Edge test"));
        assert!(redacted.contains("Microsoft Edge"));
    }

    #[test]
    fn routine_image_requests_are_not_logged() {
        let payload = json!({
            "requestId": "2",
            "type": "Image",
            "request": { "url": "https://example.test/image.png", "method": "GET" }
        })
        .to_string();
        assert!(summarize_event("Network.requestWillBeSent", &payload).is_none());
    }

    #[test]
    fn request_extra_info_logs_auth_cookie_metadata_without_values() {
        let payload = json!({
            "requestId": "auth-request",
            "associatedCookies": [
                {
                    "cookie": {
                        "name": "login_session_id",
                        "value": "secret-session-value",
                        "domain": ".dmm.com",
                        "path": "/",
                        "secure": true,
                        "httpOnly": false,
                        "sameSite": "None"
                    },
                    "blockedReasons": []
                },
                {
                    "cookie": {
                        "name": "tracking",
                        "value": "secret-tracking-value",
                        "domain": ".dmm.com"
                    },
                    "blockedReasons": []
                }
            ],
            "headers": { "Cookie": "login_session_id=secret-session-value" }
        })
        .to_string();

        let summary = summarize_event("Network.requestWillBeSentExtraInfo", &payload).unwrap();
        assert!(summary.contains("login_session_id"));
        assert!(!summary.contains("secret-session-value"));
        assert!(!summary.contains("tracking"));
        assert!(!summary.contains("\"Cookie\":"));
        assert!(!summary.contains("login_session_id="));
    }

    #[test]
    fn exception_summary_keeps_source_and_stack() {
        let payload = json!({
            "timestamp": 1,
            "exceptionDetails": {
                "text": "Uncaught TypeError",
                "url": "https://example.test/shop.js",
                "stackTrace": { "callFrames": [{ "functionName": "buy", "lineNumber": 42 }] }
            }
        })
        .to_string();
        let summary = summarize_event("Runtime.exceptionThrown", &payload).unwrap();
        assert!(summary.contains("Uncaught TypeError"));
        assert!(summary.contains("shop.js"));
        assert!(summary.contains("buy"));
    }

    #[test]
    fn repetitive_tracking_prevention_messages_are_filtered() {
        let payload = json!({
            "entry": {
                "source": "other",
                "level": "warning",
                "text": "Tracking Prevention blocked access to storage for https://tracker.test"
            }
        })
        .to_string();
        assert!(summarize_event("Log.entryAdded", &payload).is_none());
    }
}
