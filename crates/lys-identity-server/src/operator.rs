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
pub fn token(
    config: &Config,
    say: &dyn Fn(&str),
) -> Result<Option<Zeroizing<String>>, ServerError> {
    config
        .operator_token_file
        .as_deref()
        .map(|path| read(path, say))
        .transpose()
}

fn read(path: &Path, say: &dyn Fn(&str)) -> Result<Zeroizing<String>, ServerError> {
    read_protected(path, |file| protect(file, path, say))
}

fn read_protected(
    path: &Path,
    protect_file: impl FnOnce(&std::fs::File) -> std::io::Result<()>,
) -> Result<Zeroizing<String>, ServerError> {
    use std::io::Read;
    let invalid = |error: std::io::Error| ServerError::ConfigInvalid {
        reason: format!(
            "the operator token {} cannot be protected and read: {error}",
            path.display()
        ),
    };
    let mut file = std::fs::File::open(path).map_err(invalid)?;
    protect_file(&file).map_err(invalid)?;
    let mut text = Zeroizing::new(String::new());
    file.read_to_string(&mut text).map_err(invalid)?;
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

#[cfg(unix)]
fn protect(file: &std::fs::File, path: &Path, say: &dyn Fn(&str)) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(std::io::Error::other("not a regular file"));
    }
    if metadata.permissions().mode() & 0o7777 & !0o600 != 0 {
        file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
        if file.metadata()?.permissions().mode() & 0o7777 != 0o600 {
            return Err(std::io::Error::other("permissions did not become 0600"));
        }
        say(&format!(
            "operator token {} permissions tightened to 0600",
            path.display()
        ));
    }
    Ok(())
}

#[cfg(not(unix))]
fn protect(_file: &std::fs::File, _path: &Path, _say: &dyn Fn(&str)) -> std::io::Result<()> {
    Err(std::io::Error::other(
        "operator token permissions require Unix",
    ))
}

/// Validate a presented operator credential before another authentication path
/// can accept the request. Personal endpoints require a real person's session.
/// During an upgrade, operator writes cannot precede the rollback boundary.
pub async fn guard(
    axum::extract::State(state): axum::extract::State<std::sync::Arc<AppState>>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, ServerError> {
    if actor(&state, request.headers())?.is_some() {
        let path = request.uri().path();
        let path = path.strip_prefix("/api").unwrap_or(path);
        if path == "/me" || path.starts_with("/me/") {
            return Err(ServerError::OperatorRefused {
                reason: "the operator has no personal account",
            });
        }
    }
    Ok(next.run(request).await)
}

/// Whether the managed install is still in its reversible upgrade window.
/// Every writer of a form an older build cannot read must defer on `true`
/// and refuse on an I/O error. An unmanaged service has no intent path.
pub fn upgrade_pending(state: &AppState) -> std::io::Result<bool> {
    state
        .operator_upgrade_file
        .as_deref()
        .map_or(Ok(false), Path::try_exists)
}

/// The administrator, for a request carrying the operator token; none for
/// a request that carries no `lys-operator` header. A header that does not
/// match, or a service with no token or no administrator yet, is refused.
pub fn actor(state: &AppState, headers: &HeaderMap) -> Result<Option<Actor>, ServerError> {
    let Some(value) = headers.get(HEADER) else {
        return Ok(None);
    };
    if headers.get_all(HEADER).iter().count() != 1 {
        return Err(ServerError::OperatorRefused {
            reason: "the operator header must occur once",
        });
    }
    match upgrade_pending(state) {
        Ok(false) => {}
        Ok(true) => {
            return Err(ServerError::OperatorRefused {
                reason: "the upgrade is still reversible",
            });
        }
        Err(_) => {
            return Err(ServerError::OperatorRefused {
                reason: "the upgrade state cannot be read",
            });
        }
    }
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
        .administrator_login()?
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

#[cfg(test)]
#[path = "operator_tests.rs"]
mod tests;
