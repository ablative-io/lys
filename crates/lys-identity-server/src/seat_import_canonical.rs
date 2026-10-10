//! The canonical JSON text a seat import plan is hashed over (AGENTS-003
//! R4): every object's members in byte order, so the same plan always has
//! the same revision wherever it is assembled.

use std::collections::BTreeMap;

use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::routes::hex;

/// `value` as JSON with every object's members in byte order.
pub fn canonical(value: &Value, out: &mut String) {
    match value {
        Value::Object(members) => {
            let sorted: BTreeMap<&String, &Value> = members.iter().collect();
            out.push('{');
            for (index, (key, member)) in sorted.into_iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                canonical(&Value::String(key.clone()), out);
                out.push(':');
                canonical(member, out);
            }
            out.push('}');
        }
        Value::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                canonical(item, out);
            }
            out.push(']');
        }
        other => out.push_str(&other.to_string()),
    }
}

pub(crate) fn sha256(text: &str) -> String {
    hex(&Sha256::digest(text.as_bytes()))
}
