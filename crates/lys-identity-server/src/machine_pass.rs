//! A machine asks Lys for a pass (ACCESS-005 R1), so it reaches a product
//! the way a person or an agent does, carrying its rights.
//!
//! - `POST /runner/dial/{machine}/pass` `{"app", "grant_binding"}`: the
//!   machine identity the computer `machine`'s join made asks for a pass to
//!   the app `app`, with its grant binding when `grant_binding` names the
//!   version served. The answer is `{"access_token", "token_type",
//!   "expires_in"}` and the binding beside it when one was asked for
//!   (`provider::machine_issue`).
//!
//! The request is signed as a dial is, over the dial domain, the method,
//! the route, this server's epoch, a nonce used once in it and the body
//! (`runner_dial::signed_by`), but by the key the computer's latest join
//! recorded, never by whatever key its runner record names now: a machine
//! is only ever the join's key (ACCESS-005, "SHALL NOT coerce an existing
//! runner record into a machine without the join's key"). A computer whose
//! joins made no machine, because it never joined or joined before machines
//! were identities, is refused `runner_dial_refused`, as is a request its
//! key did not sign, so a replaced machine's key no longer asks for
//! anything. A machine whose computer is retired is refused `HolderRetired`
//! by the issuer.
//!
//! The connection codes and the network are read and let go before the
//! provider reads the apps and the grants, which read them again under the
//! directory in the order `network_machines::joined_now` keeps.

use std::sync::Arc;

use axum::Json;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use lys_runner::dial::pass_route;
use serde::Deserialize;

use crate::error::ServerError;
use crate::error_machine::MachineError;
use crate::network_machines::{Joined, joined_now};
use crate::provider::{MachinePass, machine_pass};
use crate::routes::AppState;
use crate::runner_client::dial_key;
use crate::runner_dial::signed_by;

/// A machine's request for a pass.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct PassAsked {
    /// The app the pass is for: its audience, an app approved on the Apps
    /// screen.
    app: String,
    /// The grant binding version asked for beside the pass; absent, none.
    #[serde(default)]
    grant_binding: Option<String>,
}

/// The machine the latest join of the computer `computer` made, among
/// `machines`, in the order they joined; refused by name when no join of
/// it made one.
pub(crate) fn current<'a>(
    machines: &'a [Joined],
    computer: &str,
) -> Result<&'a Joined, ServerError> {
    machines
        .iter()
        .rev()
        .find(|machine| machine.machine == computer && !machine.replaced)
        .ok_or_else(|| {
            MachineError::dial_refused(format!(
                "computer `{computer}` has no machine identity: no join of it made one; give it a new connection code and join it again"
            ))
        })
}

/// A machine's pass, asked for with the key its join recorded.
pub(crate) async fn pass(
    State(state): State<Arc<AppState>>,
    Path(computer): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<MachinePass>, ServerError> {
    let machines = joined_now(&state)?;
    let machine = current(&machines, &computer)?;
    let key = dial_key(&machine.key).ok_or_else(|| {
        MachineError::dial_refused(format!(
            "the key computer `{computer}` joined with does not read"
        ))
    })?;
    signed_by(
        &state,
        &computer,
        &key,
        &headers,
        &pass_route(&computer),
        &body,
    )?;
    let asked: PassAsked =
        serde_json::from_slice(&body).map_err(|error| ServerError::RequestMalformed {
            reason: format!("the pass request does not read: {error}"),
        })?;
    Ok(Json(machine_pass(
        &state,
        machine.identity,
        &asked.app,
        asked.grant_binding.as_deref(),
    )?))
}

#[cfg(test)]
#[path = "machine_pass_tests.rs"]
mod tests;
