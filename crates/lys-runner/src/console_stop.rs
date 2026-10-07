//! A console stop has one wire shape and one signing domain. The service
//! verifies the exact body bytes sent; its operation is the replay guard,
//! and only the service supplies the recorded time.

use serde::{Deserialize, Serialize};

/// The console stop's route before the installed API prefix.
pub const PATH: &str = "/runtime/stop-everything/console";

/// The domain separating a console stop from every other signed request.
pub const DOMAIN: &str = "lys-identity/console-stop/v1";

/// The header carrying the signature of the exact request body.
pub const SIGNATURE_HEADER: &str = "x-lys-console-signature";

/// A stop claimed at the console, with no caller-provided clock or nonce.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = ConsoleStopBody)]
pub struct Request {
    /// The operation spent once by the cord store.
    pub operation: String,
    /// The console's claim of who asked, recorded without a directory lookup.
    pub by: String,
    /// Why everything is stopped.
    pub reason: String,
    /// Whether sessions are killed rather than hung up.
    pub kill: bool,
}

impl Request {
    /// Build the shared body without introducing a second wire shape.
    #[must_use]
    pub const fn new(operation: String, by: String, reason: String, kill: bool) -> Self {
        Self {
            operation,
            by,
            reason,
            kill,
        }
    }
}

/// The route as mounted with or without the screens' API prefix.
#[must_use]
pub fn path(under_api: bool) -> String {
    if under_api {
        format!("/api{PATH}")
    } else {
        PATH.to_owned()
    }
}

/// The domain, newline and exact sent body that both sides sign or verify.
#[must_use]
pub fn signed_bytes(body: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(DOMAIN.len() + 1 + body.len());
    bytes.extend_from_slice(DOMAIN.as_bytes());
    bytes.push(b'\n');
    bytes.extend_from_slice(body);
    bytes
}
