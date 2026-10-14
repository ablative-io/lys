//! Credential values are withheld; key-file arguments carry paths.

use crate::seat_import_profiles::{Sorted, child, dotted};
use serde_json::Value;

const SECRET_WORDS: [&str; 8] = [
    "key",
    "token",
    "secret",
    "password",
    "passwd",
    "bearer",
    "authorization",
    "cookie",
];
/// Whether a variable, header, flag or query name names a secret.
#[must_use]
pub fn secret_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    SECRET_WORDS.iter().any(|word| lower.contains(word))
}

/// Whether `text` is shaped as a secret wherever it is written: a private
/// key, a bearer or basic credential, or an address carrying a user part or
/// a secret-named query value.
#[must_use]
pub fn secret_value(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("-----begin")
        || lower.starts_with("bearer ")
        || lower.starts_with("basic ")
        || address_carries_secret(text)
}

fn address_carries_secret(text: &str) -> bool {
    let Some((_, rest)) = text.split_once("://") else {
        return false;
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    if authority.contains('@') {
        return true;
    }
    let Some((_, query)) = rest.split_once('?') else {
        return false;
    };
    let query = query.split('#').next().unwrap_or_default();
    query.split('&').any(|pair| {
        pair.split_once('=')
            .is_some_and(|(name, value)| secret_name(name) && !value.is_empty())
    })
}

enum Flag {
    Plain,
    Inline,
    Next,
}

fn secret_flag(arg: &str) -> Flag {
    let body = arg.trim_start_matches('-');
    if body.len() == arg.len() {
        return Flag::Plain;
    }
    if body == "key-file" {
        return Flag::Plain;
    }
    if let Some(("key-file", value)) = body.split_once('=') {
        return if secret_value(value) {
            Flag::Inline
        } else {
            Flag::Plain
        };
    }
    match body.split_once('=') {
        Some((name, value)) if secret_name(name) && !value.is_empty() => Flag::Inline,
        None if secret_name(body) => Flag::Next,
        Some(_) | None => Flag::Plain,
    }
}

/// Every secret-shaped value under `value`, refused into `sorted`.
pub(super) fn scan(sorted: &mut Sorted, value: &Value, path: &str, pointer: &str, parent: &str) {
    match value {
        Value::String(text) if secret_value(text) => sorted.credential(path, pointer.to_owned()),
        Value::Array(items) => {
            let mut next_is_secret = false;
            for (index, item) in items.iter().enumerate() {
                let at = format!("{path}[{index}]");
                let below = format!("{pointer}/{index}");
                if parent == "args"
                    && let Value::String(arg) = item
                {
                    if std::mem::take(&mut next_is_secret) {
                        sorted.credential(&at, below);
                        continue;
                    }
                    match secret_flag(arg) {
                        Flag::Inline => {
                            sorted.credential(&at, below);
                            continue;
                        }
                        Flag::Next => next_is_secret = true,
                        Flag::Plain => {}
                    }
                }
                scan(sorted, item, &at, &below, "");
            }
        }
        Value::Object(members) => {
            for (key, item) in members {
                let at = dotted(path, key);
                let below = child(pointer, key);
                let named = matches!(parent, "env" | "headers" | "http_headers");
                if named && secret_name(key) && item.as_str().is_some_and(|text| !text.is_empty()) {
                    sorted.credential(&at, below);
                    continue;
                }
                scan(sorted, item, &at, &below, key);
            }
        }
        Value::String(_) | Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}
