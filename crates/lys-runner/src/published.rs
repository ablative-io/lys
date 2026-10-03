//! The runner protocol as it is published: the runner's own section of the
//! service's API description, in the words of `OpenAPI` 3.1, so a tool that would
//! be a machine's runner is built from it and checked by the conformance
//! suite. It is written from the protocol's own types' names, and a test
//! holds each act and answer the types define to a place in it.

use serde_json::{Value, json};

use crate::dial::{
    DIAL_DOMAIN, EPOCH_HEADER, EPOCH_ROUTE, NONCE_HEADER, SIGNATURE_HEADER, STALE, TICKET_HEADER,
};
use crate::protocol::{PROTOCOL_VERSION, REQUEST_DOMAIN};

/// Every act the protocol defines, by its tag.
pub const ACTS: [&str; 14] = [
    "read_bytes",
    "input_bytes",
    "start",
    "input",
    "keys",
    "read",
    "wait",
    "resize",
    "end",
    "status",
    "operate",
    "outcome",
    "feed",
    "grant_channel",
];

/// Every answer the protocol defines, by its kind.
pub const ANSWERS: [&str; 11] = [
    "bytes",
    "feed",
    "grant_channel",
    "started",
    "delivered",
    "output",
    "matched",
    "ended",
    "status",
    "operation",
    "refused",
];

/// Every refusal a runner names for a request before it acts.
pub const REQUEST_REFUSALS: [&str; 5] = [
    "runner_request_unsigned",
    "runner_request_malformed",
    "runner_protocol_mismatch",
    "runner_request_replayed",
    "runner_request_misaddressed",
];

/// The runner's section.
pub fn section() -> Value {
    json!({
        "openapi": "3.1.0",
        "info": {
            "title": "lys runner protocol",
            "version": PROTOCOL_VERSION.to_string(),
            "description": "Any tool may be a machine's runner by speaking this protocol on a Unix socket. A connection carries the runner's greeting, one request line and one reply line of JSON. The caller gives up by closing the connection; nothing ends a request on a clock.",
        },
        "x-lys-runner": {
            "transport": "one JSON line each way on a Unix socket, mode 0600; never a network address",
            "greeting": {
                "members": {
                    "version": PROTOCOL_VERSION,
                    "runner": "the runner's own id, lowercase hex, kept for as long as its state lives",
                    "challenge": "32 random bytes in lowercase hex, made for this connection alone",
                },
                "said": "by the runner, first, on every connection",
            },
            "request": {
                "members": {
                    "version": PROTOCOL_VERSION,
                    "runner": "the greeting's runner",
                    "challenge": "the greeting's challenge",
                    "act": "the act's JSON, as a string: the bytes signed are the bytes sent",
                    "signature": "lowercase hex of the Ed25519 signature by the server's key",
                },
                "signed": format!("{REQUEST_DOMAIN}\\n<version>\\n<runner>\\n<challenge>\\n<act>"),
                "answered": "only on the connection whose greeting it names, by the runner it names: once",
            },
            "acts": {
                "read_bytes": {"session": "string", "cursor": "optional u64", "follow": "bool"},
                "input_bytes": {"session": "string", "data": "array of u8, exact bytes without newline"},
                "start": {"launch": {"session": "string", "program": "string", "arguments": ["string"], "directory": "string, empty for the runner's own", "environment": {"NAME": "value; a handle's id, never a credential"}, "columns": "u16", "rows": "u16", "rotation": "optional: accounts (handles), variable, limit {signal: exit_status, status} or {signal: words, words}, resume_arguments", "policy": "optional: {policy: {version, agent, rules}, digest: lowercase hex SHA-256 of \"lys-agent-policy/v1\\n\" then the policy's JSON}; recomputed and refused on a difference"}},
                "input": {"session": "string", "text": "string", "enter": "bool"},
                "keys": {"session": "string", "keys": ["enter", "tab", "escape", "backspace", "delete", "up", "down", "left", "right", "home", "end", "page_up", "page_down", "space", "ctrl_c", "ctrl_d", "ctrl_l", "ctrl_r", "ctrl_z"]},
                "read": {"session": "string", "cursor": "optional u64", "lines": "optional u32", "bytes": "optional u64", "follow": "bool: answer once output follows the cursor or the session ends"},
                "wait": {"session": "string", "cursor": "optional u64, the output's end when absent", "pattern": "string", "regex": "bool: a literal string unless asked"},
                "resize": {"session": "string", "columns": "u16", "rows": "u16"},
                "end": {"session": "string"},
                "status": {"session": "optional string"},
                "operate": {"operation": {"operation": "string: the server's stable id, never a connection's challenge", "session": "string", "request": "tagged by request: compact {text}, notice {text}, reminder {text} or stop; text is typed at the next turn boundary"}},
                "outcome": {"operation": "string"},
                "feed": {"cursor": "optional string: the last page's cursor", "follow": "bool: answer once an entry is committed after the cursor"},
                "grant_channel": {"description": "the connection becomes the grant channel: each question is written as one line, and its answer is read as one line"},
            },
            "act_tag": "act",
            "reply": {"version": PROTOCOL_VERSION, "answer": "tagged by kind"},
            "answers": ANSWERS,
            "bytes": {"output": {"session": "string", "from": "u64", "cursor": "u64", "oldest": "u64", "data": "array of u8, exact PTY bytes", "ended": "observed end or null"}},
            "request_refusals": REQUEST_REFUSALS,
            "act_refusals": ["session_unknown", "session_exists", "session_ended", "session_invalid", "cursor_expired", "cursor_ahead", "pattern_invalid", "size_invalid", "spawn_failed", "launch_without_directory", "trust_file_invalid", "trust_file_unreadable", "trust_file_unwritable", "write_failed", "resize_failed", "end_failed", "rotation_invalid", "runner_stopping", "caller_left", "operation_reused", "operation_unknown", "policy_invalid", "policy_rule_duplicate", "policy_target_ambiguous", "policy_target_uninspectable", "policy_digest_mismatch", "cursor_invalid", "grant_channel_unheld"],
            "judged_under": "a status's session names the policy its launch carried as policy {version, digest}; absent when none was carried",
            "ended": {"how": ["exited", "ended_by_runner_restart", "accounts_exhausted"], "at": "milliseconds since the Unix epoch", "status": "the exit status seen, or null: never invented", "signal": "string or null"},
            "dial": {
                "description": "A runner on another machine is reached through a bridge that dials the server; the server never dials it. The bridge carries its runner connection's greeting to next, and the request it is answered is signed over it. TLS for https://; cleartext http:// only to the machine's own loopback address.",
                "epoch": format!("GET {EPOCH_ROUTE}: made fresh each time the server starts"),
                "next": "POST /runner/dial/{machine}/next, the body the greeting",
                "reply": "POST /runner/dial/{machine}/replies/{ticket}",
                "headers": [EPOCH_HEADER, NONCE_HEADER, SIGNATURE_HEADER],
                "ticket_header": TICKET_HEADER,
                "signed": format!("{DIAL_DOMAIN}\\nPOST\\n<route>\\n<epoch>\\n<nonce>\\n<body>"),
                "stale": STALE,
            },
        },
    })
}
