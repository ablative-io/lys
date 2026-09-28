//! The install's operator token: a secret in the install's state folder,
//! readable only by the account that owns the install, which already holds
//! every credential the administrator's power rests on. A request carrying
//! it in the `lys-operator` header acts as the administrator, and every
//! record it writes names the operator token as how it was authenticated,
//! so the log never says a sign-in took place when none did.

use std::path::Path;

use axum::http::HeaderMap;
use lys_identity::{Actor, AuthMethod, Provenance};
use zeroize::Zeroizing;

use crate::config::Config;
use crate::error::ServerError;
use crate::routes::AppState;
use crate::session::now;

/// The header an operator request carries.
pub const HEADER: &str = "lys-operator";

/// The fewest characters an operator token holds.
const TOKEN_MIN: usize = 32;

/// The token the configuration names, read once at start; none when it
/// names no file.
pub fn token(config: &Config) -> Result<Option<Zeroizing<String>>, ServerError> {
    config.operator_token_file.as_deref().map(read).transpose()
}

fn read(path: &Path) -> Result<Zeroizing<String>, ServerError> {
    let text = Zeroizing::new(std::fs::read_to_string(path).map_err(|error| {
        ServerError::ConfigInvalid {
            reason: format!(
                "the operator token {} cannot be read: {error}",
                path.display()
            ),
        }
    })?);
    let token = Zeroizing::new(text.trim().to_string());
    if token.len() < TOKEN_MIN {
        return Err(ServerError::ConfigInvalid {
            reason: format!(
                "the operator token {} is shorter than {TOKEN_MIN} characters",
                path.display()
            ),
        });
    }
    Ok(token)
}

/// The administrator, for a request carrying the operator token; none for
/// a request that carries no `lys-operator` header. A header that does not
/// match, or a service with no token or no administrator yet, is refused.
pub fn actor(state: &AppState, headers: &HeaderMap) -> Result<Option<Actor>, ServerError> {
    let Some(value) = headers.get(HEADER) else {
        return Ok(None);
    };
    let offered = value
        .to_str()
        .map_err(|_unread| ServerError::OperatorRefused {
            reason: "the header is not text",
        })?;
    let Some(token) = state.operator_token.as_ref() else {
        return Err(ServerError::OperatorRefused {
            reason: "this service holds no operator token",
        });
    };
    if !same(offered.trim(), token) {
        return Err(ServerError::OperatorRefused {
            reason: "the token does not match",
        });
    }
    let login = state
        .admission
        .administrator_login()
        .ok_or(ServerError::OperatorRefused {
            reason: "there is no administrator yet",
        })?;
    Ok(Some(Actor::new(
        login,
        Provenance::new(AuthMethod::Operator, now()),
    )))
}

/// Whether `a` and `b` are the same text, compared in time that does not
/// depend on where they first differ.
fn same(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.bytes()
            .zip(b.bytes())
            .fold(0u8, |differ, (x, y)| differ | (x ^ y))
            == 0
}
