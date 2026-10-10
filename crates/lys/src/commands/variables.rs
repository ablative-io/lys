//! `lys variables`: a Lys-started run reads and patches its own agent's and
//! its own session's variables (AGENTS-001 R2), as the run itself, by the
//! run pass Lys rendered into its seat's own configuration (DIRECTORY-077
//! R2). Nothing is signed here and no key is held: the pass and the Lys
//! address are read from where the launch put them, the `lys` entry of the
//! launch's `mcp.json` for a Claude Code seat, or `LYS_AGENT_PASS` and
//! `LYS_SEAT` for a Codex seat, and sent to the numeric loopback address
//! they name. The pass is never printed.
//!
//! Every act and every refusal is the server's; this file asks, and prints
//! the answer as human lines or one JSON object.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};
use zeroize::Zeroizing;

use crate::cli::{VariablesArgs, VariablesCommand};
use crate::commands::error::{CliError, CliResult};
use crate::identity::loopback_http::{Authority, Failure, Request, exchange};

/// The header a run's request carries, as the server reads it.
pub const PASS_HEADER: &str = "lys-agent-pass";
/// The header naming the seat that signed the pass, as the server reads it.
pub const SEAT_HEADER: &str = "lys-seat";
/// The Codex launch's pass variable.
pub const PASS_VARIABLE: &str = "LYS_AGENT_PASS";
/// The Codex launch's seat variable.
pub const SEAT_VARIABLE: &str = "LYS_SEAT";
/// The Lys address variable a Codex launch carries.
pub const URL_VARIABLE: &str = "LYS_MCP_URL";
/// The file a Claude Code launch carries the `lys` entry in.
pub const MCP_CONFIG: &str = "mcp.json";

/// A refusal by `name`, in `words`.
fn refused(name: &str, words: impl Into<String>) -> CliError {
    CliError::Runner(lys_runner::RunnerError::refused(name, words))
}

/// Where a run reaches Lys as itself: the address and the two headers.
struct Run {
    authority: Authority,
    prefix: String,
    pass: Zeroizing<String>,
    seat: Option<String>,
}

impl Run {
    /// The run's address and pass, from the environment or the launch's
    /// `mcp.json` under `folder`.
    fn discover(folder: &Path) -> CliResult<Self> {
        if let (Ok(pass), Ok(url)) = (std::env::var(PASS_VARIABLE), std::env::var(URL_VARIABLE)) {
            let seat = std::env::var(SEAT_VARIABLE).ok();
            return Self::at(&url, pass, seat);
        }
        let path = folder.join(MCP_CONFIG);
        let text = fs::read_to_string(&path).map_err(|error| {
            refused(
                "run_pass_absent",
                format!(
                    "no run pass: {PASS_VARIABLE} and {URL_VARIABLE} are not set and {} cannot be read ({error}); \
                     `lys variables` runs inside a seat Lys started",
                    path.display()
                ),
            )
        })?;
        let root: Value = serde_json::from_str(&text).map_err(|error| {
            refused(
                "run_pass_absent",
                format!("{} is not JSON: {error}", path.display()),
            )
        })?;
        let entry = root.pointer("/mcpServers/lys").ok_or_else(|| {
            refused(
                "run_pass_absent",
                format!("{} carries no lys entry under mcpServers", path.display()),
            )
        })?;
        let url = entry
            .get("url")
            .and_then(Value::as_str)
            .ok_or_else(|| refused("run_pass_absent", "the lys entry names no url"))?;
        let headers = entry.get("headers").and_then(Value::as_object);
        let pass = headers
            .and_then(|headers| headers.get(PASS_HEADER))
            .and_then(Value::as_str)
            .ok_or_else(|| refused("run_pass_absent", "the lys entry carries no run pass"))?;
        let seat = headers
            .and_then(|headers| headers.get(SEAT_HEADER))
            .and_then(Value::as_str)
            .map(str::to_owned);
        Self::at(url, pass.to_owned(), seat)
    }

    /// The run at the Lys MCP address `url`, whose API is beside it.
    fn at(url: &str, pass: String, seat: Option<String>) -> CliResult<Self> {
        let authority = Authority::from_http_url(url)
            .map_err(|detail| refused("server_base_invalid", format!("{url}: {detail}")))?;
        let path = url
            .splitn(4, '/')
            .nth(3)
            .map_or(String::new(), |rest| format!("/{rest}"));
        let prefix = path
            .strip_suffix("/mcp")
            .map_or(path.clone(), str::to_owned);
        Ok(Self {
            authority,
            prefix,
            pass: Zeroizing::new(pass),
            seat,
        })
    }

    fn send(&self, method: &str, path: &str, body: Option<&Value>) -> CliResult<Value> {
        let route = format!("{}{path}", self.prefix);
        let bytes = match body {
            Some(body) => serde_json::to_vec(body).map_err(|source| CliError::JsonSerialize {
                what: "request body",
                source,
            })?,
            None => Vec::new(),
        };
        let mut headers: Vec<(&str, &[u8])> = vec![
            (PASS_HEADER, self.pass.as_bytes()),
            ("Content-Type", b"application/json".as_slice()),
        ];
        if let Some(seat) = &self.seat {
            headers.push((SEAT_HEADER, seat.as_bytes()));
        }
        let request = Request {
            method,
            path: &route,
            headers: &headers,
            body: &bytes,
        };
        let answer = exchange(&self.authority, &request).map_err(|failure| match failure {
            Failure::Unreachable(detail) => refused(
                "identity_server_unreachable",
                format!(
                    "{} could not be reached: {detail}; nothing was sent",
                    self.authority
                ),
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
        let (name, reason) = parsed.ok().map_or_else(
            || {
                (
                    "identity_server_refused".to_owned(),
                    "a body that is not JSON".to_owned(),
                )
            },
            |body| {
                (
                    body.get("refusal")
                        .and_then(Value::as_str)
                        .unwrap_or("identity_server_refused")
                        .to_owned(),
                    body.get("reason")
                        .and_then(Value::as_str)
                        .unwrap_or("the server gave no reason")
                        .to_owned(),
                )
            },
        );
        Err(refused(
            &name,
            format!("{reason} ({method} {route}, HTTP {})", answer.status),
        ))
    }
}

/// The route of `scope`: the run's own agent map, or its own session's.
fn route(session: bool) -> &'static str {
    if session {
        "/me/session/variables"
    } else {
        "/me/variables"
    }
}

/// One `key=value` as given: a JSON value when it parses as one, else the
/// text; `key=` alone removes the key.
fn parsed(given: &str) -> CliResult<(String, Value)> {
    let Some((key, value)) = given.split_once('=') else {
        return Err(refused(
            "variables_malformed",
            format!("`{given}` is not key=value"),
        ));
    };
    let value = if value.is_empty() {
        Value::Null
    } else {
        serde_json::from_str(value).unwrap_or_else(|_text| Value::String(value.to_owned()))
    };
    Ok((key.to_owned(), value))
}

/// Runs `lys variables`.
///
/// # Errors
///
/// The server's refusal by its name and words, or a named failure to find
/// the run's pass or reach the server.
pub fn run(args: VariablesArgs, json: bool) -> CliResult<()> {
    let folder = args.folder.clone().map_or_else(
        || {
            std::env::current_dir().map_err(|source| CliError::Io {
                context: "reading the working folder".to_owned(),
                source,
            })
        },
        Ok::<PathBuf, CliError>,
    )?;
    let run = Run::discover(&folder)?;
    let answer = match args.command {
        VariablesCommand::Get { session } => run.send("GET", route(session), None)?,
        VariablesCommand::Set {
            session,
            revision,
            expires_at,
            values,
        } => {
            let mut map: BTreeMap<String, Value> = BTreeMap::new();
            for given in &values {
                let (key, value) = parsed(given)?;
                map.insert(key, value);
            }
            let mut body = Map::new();
            body.insert("revision".to_owned(), json!(revision));
            body.insert(
                "values".to_owned(),
                Value::Object(map.into_iter().collect()),
            );
            if let Some(expires_at) = expires_at {
                body.insert("expires_at".to_owned(), json!(expires_at));
            }
            run.send("POST", route(session), Some(&Value::Object(body)))?
        }
    };
    if json {
        println!("{answer}");
    } else {
        lines(&answer);
    }
    Ok(())
}

/// The read, as lines: the scope and revision, then each variable.
fn lines(answer: &Value) {
    let scope = answer.get("scope").map_or_else(
        || "?".to_owned(),
        |scope| {
            format!(
                "{} {}",
                scope.get("kind").and_then(Value::as_str).unwrap_or("?"),
                scope.get("id").and_then(Value::as_str).unwrap_or("?")
            )
        },
    );
    let revision = answer.get("revision").and_then(Value::as_u64).unwrap_or(0);
    println!("{scope} at revision {revision}");
    if let Some(values) = answer.get("values").and_then(Value::as_object) {
        for (name, held) in values {
            let value = held.get("value").map_or(Value::Null, Clone::clone);
            let author = held.get("author").and_then(Value::as_str).unwrap_or("?");
            let revision = held.get("revision").and_then(Value::as_u64).unwrap_or(0);
            let expiry = held
                .get("expires_at")
                .and_then(Value::as_u64)
                .map_or(String::new(), |at| format!(", expires at {at}"));
            println!("{name} = {value} (revision {revision}, by {author}{expiry})");
        }
    }
    if let Some(expired) = answer.get("expired").and_then(Value::as_array)
        && !expired.is_empty()
    {
        let names: Vec<&str> = expired.iter().filter_map(Value::as_str).collect();
        println!("expired: {}", names.join(", "));
    }
}
