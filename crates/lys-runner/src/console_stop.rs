//! The console stop request shared by the command and the service.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// The console stop route without the screens' API prefix.
pub const ROUTE: &str = "/runtime/stop-everything/console";
/// The domain separating a console stop from every other signed request.
pub const DOMAIN: &str = "lys-identity/console-stop/v1";
/// The header carrying the hexadecimal service-key signature.
pub const SIGNATURE: &str = "x-lys-console-signature";

/// A console's claim and instruction, with replay judged by its operation.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = ConsoleStopBody)]
pub struct Body {
    /// The operation spent by the cord store.
    pub operation: String,
    /// The person's claim at this computer's console.
    pub by: String,
    /// Why everything must stop.
    pub reason: String,
    /// Whether to kill processes immediately.
    pub kill: bool,
}

/// The route under the installed service's mount.
#[must_use]
pub const fn route(surface: bool) -> &'static str {
    if surface {
        "/api/runtime/stop-everything/console"
    } else {
        ROUTE
    }
}

/// The signature binds the domain to the body exactly as transmitted.
#[must_use]
pub fn signed_bytes(body: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(DOMAIN.len() + 1 + body.len());
    bytes.extend_from_slice(DOMAIN.as_bytes());
    bytes.push(b'\n');
    bytes.extend_from_slice(body);
    bytes
}
