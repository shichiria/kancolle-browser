# Diagnostics / action / raw API logging

## Responsibilities and storage

| Stream | Location under app local data | Default | Retention |
|---|---|---|---|
| Session diagnostics | `local/logs/session_<timestamp>_<pid>.log` | ON | 90 days, max 200 files |
| Action log | `local/action_logs/actions_YYYYMMDD.jsonl` | ON | 90 days, max 200 files |
| Raw API dump | `sync/raw_api/*.json` | OFF | 90 days, max 5000 files (cleaned at startup) |
| Click screenshots | `local/action_logs/screenshots/*.png` | Debug UI capture only | Managed separately |

Session diagnostics contains Rust logs, frontend console output, unhandled frontend errors,
and panic information. Action logs contain compact structured actions and share the session ID.
Raw API files are opt-in because they contain private gameplay data and are much larger.

## I/O policy

`log_io.rs` owns the shared buffered writer and file-retention utility. Session and action
streams are flushed every 250 ms or at 64 KiB; error records, panic handling, and application
shutdown force an immediate flush. Raw API dumps remain one JSON file per intercepted API but
use the shared file utility and are written outside `GameState` locks.

Frontend console records are buffered for 100 ms (maximum 64 entries) and sent through
`log_frontend_events`. Error and unhandled-rejection paths flush the current batch immediately.
The legacy single-record `log_frontend_event` command remains for the DMM page shim.

## Redaction

The canonical sensitive-key list is `src/sensitive-keys.json`. Rust loads it through
`sensitive.rs`; frontend diagnostics imports the same JSON. Both session text and raw API
request bodies redact matching credential values before persistence.
