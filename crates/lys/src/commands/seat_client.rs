//! Seat and attach requests use the local install or a signed-in person pass.
//! Operator authority remains restricted to the installed numeric loopback server.
//! Personal authority is read from the client's owner-only file and bound to its server.

use std::fs;
use std::net::IpAddr;
use std::path::{Path, PathBuf};

use serde_json::Value;
use zeroize::Zeroizing;

use crate::commands::error::{CliError, CliResult};
use crate::identity::IdentityError;
use crate::identity::error::ErrorKind;
use crate::identity::install::layout::Layout;
use crate::identity::loopback_http::{Authority, Failure, HTTP_DEFAULT_PORT, Request, exchange};
use crate::identity::private_files;

/// The header an operator request carries, as the server reads it.
pub const OPERATOR_HEADER: &str = "lys-operator";

/// The configuration key naming the operator token's file.
const TOKEN_KEY: &str = "operator_token_file";

/// A refusal by `name`, in `words`.
pub fn refused(name: &str, words: impl Into<String>) -> CliError {
    CliError::Runner(lys_runner::RunnerError::refused(name, words))
}

/// A failure of the install's files, naming the file.
fn install_failure(kind: ErrorKind, path: &Path, detail: impl Into<String>) -> CliError {
    CliError::Identity(IdentityError::new(kind, "reach", "identity server", detail).at(path))
}

/// The installed identity server, reached over loopback as the operator.
pub struct Server {
    connection: Connection,
}

enum Connection {
    Local {
        authority: Authority,
        prefix: String,
        token: Option<Zeroizing<String>>,
    },
    Person {
        client: Box<lys_runner::dial::HttpClient>,
        pass: crate::commands::sign_in::Pass,
    },
}

#[path = "seat_client/personal.rs"]
mod personal;

impl Server {
    /// The installed server, or the one `base` names over it. The operator
    /// token is always the install's.
    ///
    /// # Errors
    ///
    /// `not_installed` when no install is found; the install's
    /// configuration refusals; `server_base_invalid`, `server_not_loopback`,
    /// `operator_token_absent` and `operator_token_invalid` by name.
    pub fn reach(base: Option<&str>) -> CliResult<Self> {
        Self::open(base, true)
    }

    /// The installed server, or the one `base` names over it, reached with
    /// no operator token: the install's token is not read, and each request
    /// carries only the credential its caller brings.
    ///
    /// # Errors
    ///
    /// `not_installed` when no install is found; the install's
    /// configuration refusals; `server_base_invalid` and
    /// `server_not_loopback` by name.
    pub fn reach_without_operator(base: Option<&str>) -> CliResult<Self> {
        Self::open(base, false)
    }

    /// The server `base` names or the install's own, with the install's
    /// operator token when `operator` asks for it.
    fn open(base: Option<&str>, operator: bool) -> CliResult<Self> {
        let layout = Layout::discover()?;
        let path = layout.service_config();
        if operator {
            let base = base.map(crate::commands::sign_in::base).transpose()?;
            let numeric_loopback = base.as_deref().is_none_or(|base| {
                named_base(base).is_ok_and(|(authority, _)| {
                    authority
                        .host
                        .parse::<IpAddr>()
                        .is_ok_and(|ip| ip.is_loopback())
                })
            });
            if !path.is_file() || !numeric_loopback {
                let pass = crate::commands::sign_in::load(base.as_deref())?;
                let client = lys_runner::dial::HttpClient::new(&pass.server)?;
                return Ok(Self {
                    connection: Connection::Person {
                        client: Box::new(client),
                        pass,
                    },
                });
            }
        }
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(install_failure(
                    ErrorKind::NotInstalled,
                    &path,
                    "no identity install was found here: run `lys identity install`, \
                     or set LYS_IDENTITY_HOME to the install's root",
                ));
            }
            Err(error) => {
                return Err(install_failure(
                    ErrorKind::ConfigUnreadable,
                    &path,
                    format!("cannot read the installed service configuration: {error}"),
                ));
            }
        };
        let config: Value = serde_json::from_slice(&bytes).map_err(|error| {
            install_failure(
                ErrorKind::ConfigInvalid,
                &path,
                format!("the installed service configuration is not JSON: {error}"),
            )
        })?;
        let (authority, prefix) = match base {
            Some(base) => named_base(base)?,
            None => installed_base(&config, &path)?,
        };
        loopback(&authority)?;
        let token = if operator {
            Some(token(&config, &path)?)
        } else {
            None
        };
        Ok(Self {
            connection: Connection::Local {
                authority,
                prefix,
                token,
            },
        })
    }

    /// `GET path` under the base, answering the body of a success.
    ///
    /// # Errors
    ///
    /// The server's refusal by its name and words, or a named failure to
    /// reach it or to read its answer.
    pub fn get(&self, path: &str) -> CliResult<Value> {
        self.send("GET", path, None, None)
    }

    /// `POST path` with `body` under the base, answering the body of a
    /// success. An uncertain outcome is never retried.
    ///
    /// # Errors
    ///
    /// The server's refusal by its name and words, or a named failure to
    /// reach it or to read its answer.
    pub fn post(&self, path: &str, body: &Value) -> CliResult<Value> {
        self.send("POST", path, Some(body), None)
    }

    /// `POST path` with `body` under the base, carrying the caller's own
    /// `credential` header beside it, answering the body of a success. The
    /// credential is never printed. An uncertain outcome is never retried.
    ///
    /// # Errors
    ///
    /// The server's refusal by its name and words, or a named failure to
    /// reach it or to read its answer.
    pub fn post_carrying(
        &self,
        path: &str,
        body: &Value,
        credential: (&str, &[u8]),
    ) -> CliResult<Value> {
        self.send("POST", path, Some(body), Some(credential))
    }

    fn send(
        &self,
        method: &str,
        path: &str,
        body: Option<&Value>,
        credential: Option<(&str, &[u8])>,
    ) -> CliResult<Value> {
        let Connection::Local {
            authority,
            prefix,
            token,
        } = &self.connection
        else {
            return self.person_send(method, path, body, credential);
        };
        let route = format!("{prefix}{path}");
        let bytes = match body {
            Some(body) => serde_json::to_vec(body).map_err(|source| CliError::JsonSerialize {
                what: "request body",
                source,
            })?,
            None => Vec::new(),
        };
        let mut headers: Vec<(&str, &[u8])> = Vec::with_capacity(3);
        if let Some(token) = token {
            headers.push((OPERATOR_HEADER, token.as_bytes()));
        }
        headers.extend(credential);
        headers.push(("Content-Type", b"application/json".as_slice()));
        let request = Request {
            method,
            path: &route,
            headers: &headers,
            body: &bytes,
        };
        let answer = exchange(authority, &request).map_err(|failure| match failure {
            Failure::Unreachable(detail) => refused(
                "identity_server_unreachable",
                format!("{authority} could not be reached: {detail}; nothing was sent"),
            ),
            Failure::Uncertain(detail) | Failure::Malformed(detail) => refused(
                "identity_server_outcome_uncertain",
                format!("{method} {route}: no whole answer ({detail}); no retry was made"),
            ),
        })?;
        let parsed = serde_json::from_slice::<Value>(&answer.body);
        if (200..300).contains(&answer.status) {
            return parsed.map_err(|error| {
                refused(
                    "identity_server_answer_unreadable",
                    format!(
                        "{method} {route}: HTTP {} with a body that is not JSON: {error}",
                        answer.status
                    ),
                )
            });
        }
        Err(match parsed {
            Ok(body) => server_refusal(method, &route, answer.status, &body),
            Err(error) => refused(
                "identity_server_refused",
                format!(
                    "{method} {route}: HTTP {} with a body that is not JSON: {error}",
                    answer.status
                ),
            ),
        })
    }

    fn person_send(
        &self,
        method: &str,
        path: &str,
        body: Option<&Value>,
        credential: Option<(&str, &[u8])>,
    ) -> CliResult<Value> {
        let Connection::Person { client, pass } = &self.connection else {
            return Err(refused(
                "person_pass_unavailable",
                "no personal connection is held",
            ));
        };
        if credential.is_some() {
            return Err(refused(
                "credential_conflict",
                "a personal pass cannot carry another credential",
            ));
        }
        let bearer = Zeroizing::new(format!("Bearer {}", pass.access_token));
        let bytes = body
            .map(serde_json::to_vec)
            .transpose()
            .map_err(|source| CliError::JsonSerialize {
                what: "request body",
                source,
            })?
            .unwrap_or_default();
        let response = client.send(
            method,
            path,
            &[
                ("authorization", &bearer),
                ("content-type", "application/json"),
            ],
            &bytes,
        )?;
        let answer: Value = serde_json::from_slice(&response.body).map_err(|error| {
            refused(
                "identity_server_answer_unreadable",
                format!("Lys did not answer in JSON: {error}"),
            )
        })?;
        if (200..300).contains(&response.status) {
            return Ok(answer);
        }
        let error = server_refusal(method, path, response.status, &answer);
        if response.status == 401 {
            let name = answer
                .get("refusal")
                .and_then(Value::as_str)
                .unwrap_or("identity_server_refused");
            return Err(refused(
                name,
                format!("{error}; run `lys sign-in --server {}`", pass.server),
            ));
        }
        Err(error)
    }
}

/// The server's refusal, by the name and words its body gives.
fn server_refusal(method: &str, route: &str, status: u16, body: &Value) -> CliError {
    let name = body.get("refusal").and_then(Value::as_str);
    let reason = body.get("reason").and_then(Value::as_str);
    match (name, reason) {
        (Some(name), Some(reason)) => {
            refused(name, format!("{reason} ({method} {route}, HTTP {status})"))
        }
        (Some(name), None) => refused(
            name,
            format!("the server gave no reason ({method} {route}, HTTP {status})"),
        ),
        (None, _) => refused(
            "identity_server_refused",
            format!("{method} {route}: HTTP {status} naming no refusal"),
        ),
    }
}

/// The installed service's own base: its `listen` address, under `/api`
/// when it serves a surface, as the service is configured to start.
fn installed_base(config: &Value, path: &Path) -> CliResult<(Authority, String)> {
    let listen = config
        .get("listen")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            install_failure(
                ErrorKind::ConfigInvalid,
                path,
                "the installed service has no listen address",
            )
        })?;
    let authority = Authority::parse(listen, None).map_err(|reason| {
        install_failure(
            ErrorKind::ConfigInvalid,
            path,
            format!("the listen address {listen} does not read: {reason}"),
        )
    })?;
    let prefix = match config.get("surface_dir") {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(_)) => "/api".to_owned(),
        Some(_) => {
            return Err(install_failure(
                ErrorKind::ConfigInvalid,
                path,
                "surface_dir is not a directory path",
            ));
        }
    };
    Ok((authority, prefix))
}

/// The base `--server` names: `http://host:port`, with any path under it.
fn named_base(base: &str) -> CliResult<(Authority, String)> {
    let rest = base.strip_prefix("http://").ok_or_else(|| {
        refused(
            "server_base_invalid",
            format!("--server {base}: expected http://<loopback address>:<port>[/path]"),
        )
    })?;
    let (authority, path) = rest.split_once('/').unwrap_or((rest, ""));
    let authority = Authority::parse(authority, Some(HTTP_DEFAULT_PORT))
        .map_err(|reason| refused("server_base_invalid", format!("--server {base}: {reason}")))?;
    let path = path.trim_end_matches('/');
    let prefix = if path.is_empty() {
        String::new()
    } else {
        format!("/{path}")
    };
    Ok((authority, prefix))
}

/// Refuses an address that is not a numeric loopback one: the operator
/// token never leaves this machine.
fn loopback(authority: &Authority) -> CliResult<()> {
    if authority
        .host
        .parse::<IpAddr>()
        .is_ok_and(|ip| ip.is_loopback())
    {
        return Ok(());
    }
    Err(refused(
        "server_not_loopback",
        format!(
            "{authority} is not a numeric loopback address; the operator token is sent \
             only to one"
        ),
    ))
}

/// The install's operator token, from the owner-only file its service
/// configuration names. A service install keeps none.
fn token(config: &Value, path: &Path) -> CliResult<Zeroizing<String>> {
    let file = match config.get(TOKEN_KEY) {
        Some(Value::String(file)) => PathBuf::from(file),
        None | Some(Value::Null) => {
            return Err(refused(
                "operator_token_absent",
                format!(
                    "the install configured at {} keeps no operator token (a service install \
                     keeps none), so this command cannot act as its operator",
                    path.display()
                ),
            ));
        }
        Some(_) => {
            return Err(install_failure(
                ErrorKind::ConfigInvalid,
                path,
                format!("{TOKEN_KEY} is not a file path"),
            ));
        }
    };
    let bytes = private_files::read(&file)?.ok_or_else(|| {
        refused(
            "operator_token_absent",
            format!("the operator token file {} does not exist", file.display()),
        )
    })?;
    let text = std::str::from_utf8(&bytes).map_err(|error| {
        refused(
            "operator_token_invalid",
            format!(
                "the operator token file {} is not text: {error}",
                file.display()
            ),
        )
    })?;
    let token = Zeroizing::new(text.trim().to_owned());
    if token.is_empty() || token.bytes().any(|byte| byte.is_ascii_control()) {
        return Err(refused(
            "operator_token_invalid",
            format!(
                "the operator token file {} holds no single-line token",
                file.display()
            ),
        ));
    }
    Ok(token)
}

/// A new operation id, `op-` and 32 hex digits.
///
/// # Errors
///
/// `operation_id_unavailable` when the secure random source fails.
pub fn operation() -> CliResult<String> {
    lys_identity::OperationId::generate()
        .map(|operation| operation.to_string())
        .map_err(|error| {
            refused(
                "operation_id_unavailable",
                format!("no operation id could be made: {error}"),
            )
        })
}

/// `value` as one path segment, refused by `name` otherwise: letters,
/// digits, `-`, `_` and `.`, never `.` or `..` alone.
///
/// # Errors
///
/// `name` when `value` is not one such segment.
pub fn segment<'a>(name: &str, what: &str, value: &'a str) -> CliResult<&'a str> {
    let plain = value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'));
    if plain && !value.is_empty() && value != "." && value != ".." {
        return Ok(value);
    }
    Err(refused(
        name,
        format!("{what} {value:?} is not one path segment of letters, digits, '-', '_' or '.'"),
    ))
}
