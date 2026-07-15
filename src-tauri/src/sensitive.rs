//! Shared sensitive-field registry used by every backend log sink.
//!
//! The JSON file is also imported by the frontend diagnostics bridge, keeping
//! the Rust and TypeScript redaction policies sourced from the same list.

use std::sync::LazyLock;

static SENSITIVE_KEYS: LazyLock<Vec<String>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../../src/sensitive-keys.json"))
        .expect("sensitive-keys.json must contain a string array")
});

pub fn keys() -> &'static [String] {
    &SENSITIVE_KEYS
}

pub fn is_sensitive_key(key: &str) -> bool {
    SENSITIVE_KEYS
        .iter()
        .any(|sensitive| key.eq_ignore_ascii_case(sensitive))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_registry_contains_game_tokens() {
        assert!(is_sensitive_key("api_token"));
        assert!(is_sensitive_key("rpctoken"));
        assert!(is_sensitive_key("st"));
    }
}
