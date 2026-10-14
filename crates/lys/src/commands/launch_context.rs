//! Launches require personal sign-in even on a host with an operator install.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::commands::error::CliResult;
use crate::commands::seat_client::{Server, refused};
use crate::commands::sign_in::{base, load_at, pass_file};
use crate::identity::install::layout::Layout;

/// Nonsecret context and the personal connection used by every launch request.
pub struct Context {
    /// The connection carrying the person's pass, never operator authority.
    pub server: Arc<Server>,
    /// The exact API base recorded at sign-in.
    pub base: String,
    /// The signed-in person.
    pub person: String,
    /// The local state root.
    pub root: PathBuf,
}

/// Read sign-in state without selecting any service-install defaults.
///
/// # Errors
/// Names missing server addresses, sign-in and expired or unsafe passes.
pub fn read(named: Option<&str>) -> CliResult<Context> {
    read_at(&Layout::discover()?.root, named)
}

/// Read a named state root; exposed for fixtures that do not change the environment.
///
/// # Errors
/// Names every absent or invalid sign-in value before reaching a server.
pub fn read_at(root: &Path, named: Option<&str>) -> CliResult<Context> {
    let named = named.map(base).transpose()?;
    let action = named.as_ref().map_or_else(
        || "the server address is missing; run `lys sign-in --server <server-address>`".to_owned(),
        |server| format!("run `lys sign-in --server {server}`"),
    );
    if !pass_file(root).try_exists().map_err(|error| {
        refused(
            "person_pass_unreadable",
            format!("cannot locate sign-in state: {error}; {action}"),
        )
    })? {
        return Err(refused(
            if named.is_none() {
                "server_address_missing"
            } else {
                "person_pass_missing"
            },
            format!("this terminal is not signed in; {action}"),
        ));
    }
    let pass = load_at(root, None, false)
        .map_err(|error| refused("person_sign_in_unavailable", format!("{error}; {action}")))?;
    base(&pass.server)?;
    pass.person
        .parse::<lys_identity::PersonId>()
        .map_err(|error| {
            refused(
                "person_pass_unreadable",
                format!("the sign-in does not identify a person: {error}; {action}"),
            )
        })?;
    let action = format!("run `lys sign-in --server {}`", pass.server);
    if let Some(named) = &named
        && *named != pass.server
    {
        return Err(refused(
            "person_pass_wrong_server",
            format!("the sign-in belongs to another server; run `lys sign-in --server {named}`"),
        ));
    }
    let now = u64::try_from(chrono::Utc::now().timestamp())
        .map_err(|error| refused("sign_in_clock_unavailable", error.to_string()))?;
    if pass.expires_at <= now {
        return Err(refused(
            "person_pass_expired",
            format!("the person's pass has expired; {action}"),
        ));
    }
    Ok(Context {
        base: pass.server.clone(),
        person: pass.person.clone(),
        root: root.to_owned(),
        server: Arc::new(Server::personal(pass)?),
    })
}

/// The surface on the same recorded server; no unrelated host is substituted.
#[must_use]
pub fn surface(base: &str, fragment: &str) -> String {
    let origin = base.strip_suffix("/api").unwrap_or(base);
    format!("{origin}/#/{fragment}")
}

#[cfg(test)]
#[path = "launch_context_tests.rs"]
mod tests;
