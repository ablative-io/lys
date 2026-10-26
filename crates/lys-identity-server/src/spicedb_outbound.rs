//! The server owns `SpiceDB` wait records, keepalive and caller cancellation.
//! The shared HTTP transport only lends each checked-out socket to this policy.

use std::net::TcpStream;

use lys_identity::grants::{GrantError, Model};

use crate::spicedb::{SpiceDb, SpiceDbEngine, SpiceDbSettings};
use crate::spicedb_http::{Answer, Client};
use crate::waits::Call;

pub(crate) fn engine(settings: SpiceDbSettings) -> SpiceDbEngine {
    SpiceDbEngine::using(settings, post_json)
}

/// Reach the engine `settings` names under the server's socket policy and
/// write the schema `model` gives. Every binary that talks to `SpiceDB`
/// opens it here, so none waits on a dead peer without keepalive.
///
/// # Errors
/// Refuses credential loading, schema construction, or the engine's
/// connection and schema write.
pub fn open(settings: &SpiceDbSettings, model: &Model) -> Result<SpiceDb, GrantError> {
    SpiceDb::open_connected(&engine(settings.clone()).connection()?, model)
}

/// Post through the server's wait registry and SDK-023 socket policy.
///
/// # Errors
/// Names registry, keepalive, transport or framing failures, or the administrator
/// whose cancellation closed this call. No request body or credential is recorded.
pub fn post_json(
    client: &Client,
    authority: &str,
    path: &str,
    bearer: &str,
    body: &str,
) -> Result<Answer, String> {
    let call =
        Call::begin("SpiceDB", authority, "POST", path).map_err(|error| error.to_string())?;
    client
        .post_json(authority, path, bearer, body, |stream| {
            let leave = call.closer();
            if leave.left() {
                return Err("the caller closed the SpiceDB call".to_owned());
            }
            crate::outbound::keepalive(stream)
                .map_err(|error| format!("{authority} keepalive could not be set: {error}"))?;
            hold(&call, stream)?;
            Ok(move || {
                // Disarm before the transport can make this socket available to
                // another call. A cancel's close must never reach its next owner.
                leave.done().map_err(|error| error.to_string())?;
                if leave.left() {
                    Err("the caller closed the SpiceDB call".to_owned())
                } else {
                    Ok(())
                }
            })
        })
        .map_err(|error| call.reason().unwrap_or(error))
}

fn hold(call: &Call, stream: &TcpStream) -> Result<(), String> {
    let close = stream.try_clone().map_err(|error| error.to_string())?;
    call.closer()
        .hold(Box::new(move || {
            match close.shutdown(std::net::Shutdown::Both) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotConnected => {
                    tracing::debug!("spicedb_already_closed: {error}");
                }
                Err(error) => tracing::error!("spicedb_close_failed: {error}"),
            }
        }))
        .map_err(|error| error.to_string())
}
