//! An MCP server as a profile records it: an http or https address, or a
//! command with its arguments, directory and environment, and its channel
//! policy. A setting that is not a secret is a string, integer or boolean;
//! a secret is only ever the name of the secret whose handle the launch
//! sets. What would carry a credential in place of a handle is refused by
//! name, naming the server and where, never repeating the value.

use std::collections::BTreeMap;

use lys_home::harness::launch_fields::{Channel, Literal};
use reqwest::Url;
use serde::Deserialize;
use serde_json::Value;

use crate::error::ServerError;
use crate::provisioning_store::{McpCommand, McpServer, Setting};

/// The most servers a profile names.
const SERVERS_MAX: usize = 64;
/// The most arguments or settings a command takes.
const PARTS_MAX: usize = 128;

/// An MCP server as a profile is set with it.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct McpServerBody {
    name: String,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    command: Option<CommandBody>,
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    channel: Option<Channel>,
}

/// A command server as a profile is set with it; its environment is read
/// as JSON so a setting of another shape is refused by name.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
struct CommandBody {
    program: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    cwd: Option<String>,
    #[serde(default)]
    #[schema(value_type = Object)]
    env: BTreeMap<String, Value>,
}

/// Prefixes that begin a credential, never a setting.
const KEY_PREFIXES: [&str; 11] = [
    "sk-",
    "sk_live_",
    "rk_live_",
    "ghp_",
    "gho_",
    "github_pat_",
    "glpat-",
    "xoxb-",
    "xoxp-",
    "AKIA",
    "AIza",
];
/// Words that, in a variable or flag name, name a secret.
const SECRET_WORDS: [&str; 6] = ["TOKEN", "SECRET", "PASSWORD", "PASSWD", "API_KEY", "APIKEY"];

fn malformed(reason: String) -> ServerError {
    ServerError::RequestMalformed { reason }
}

fn inline(server: &str, member: String) -> ServerError {
    ServerError::McpCredentialInline {
        server: server.to_owned(),
        member,
    }
}

fn unrepresentable(server: &str, member: String, reason: &str) -> ServerError {
    ServerError::McpSettingUnrepresentable {
        server: server.to_owned(),
        member,
        reason: reason.to_owned(),
    }
}

/// Whether `name`, a variable or a flag, names a secret.
fn names_secret(name: &str) -> bool {
    let upper = name
        .trim_start_matches('-')
        .to_ascii_uppercase()
        .replace('-', "_");
    SECRET_WORDS.iter().any(|word| upper.contains(word)) || upper.ends_with("KEY")
}

/// Whether `text` has the shape of a credential: a bearer, a known key
/// prefix, or an address carrying a password.
fn credential_shaped(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed.to_ascii_lowercase().contains("bearer ")
        || KEY_PREFIXES
            .iter()
            .any(|prefix| trimmed.starts_with(prefix))
        || Url::parse(trimmed).is_ok_and(|url| url.password().is_some())
}

fn address(name: &str, given: &str) -> Result<String, ServerError> {
    let url = Url::parse(given.trim()).map_err(|error| {
        malformed(format!(
            "the URL of MCP server `{name}` does not read: {error}"
        ))
    })?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(malformed(format!(
            "the URL of MCP server `{name}` is not http or https"
        )));
    }
    if url.password().is_some() || !url.username().is_empty() {
        return Err(inline(name, "its url".to_owned()));
    }
    if let Some((key, _)) = url
        .query_pairs()
        .find(|(key, value)| names_secret(key) || credential_shaped(value))
    {
        return Err(inline(name, format!("its url's query `{key}`")));
    }
    Ok(url.to_string())
}

/// `given` exactly as the operator wrote it, refused when surrounding space
/// or a NUL would make the started program differ from the recorded one.
fn exact(name: &str, member: &str, given: &str) -> Result<(), ServerError> {
    if given.trim().is_empty() || given.trim() != given || given.contains('\0') {
        return Err(unrepresentable(
            name,
            member.to_owned(),
            "it is given exactly, not empty, without surrounding space or a NUL",
        ));
    }
    Ok(())
}

fn arguments(name: &str, given: &[String]) -> Result<Vec<String>, ServerError> {
    let mut after_secret_flag = false;
    for (at, arg) in given.iter().enumerate() {
        if arg.contains('\0') {
            return Err(unrepresentable(
                name,
                format!("argument {at}"),
                "an argument holds no NUL",
            ));
        }
        let (flag, value) = arg.split_once('=').unwrap_or((arg, ""));
        let secret_flag = flag.starts_with('-') && names_secret(flag);
        if after_secret_flag || credential_shaped(arg) || (secret_flag && !value.is_empty()) {
            return Err(inline(name, format!("argument {at}")));
        }
        after_secret_flag = secret_flag && value.is_empty();
    }
    Ok(given.to_vec())
}

fn setting(name: &str, variable: &str, given: Value) -> Result<Setting, ServerError> {
    let member = || format!("env `{variable}`");
    if let Value::Object(mut object) = given {
        return match (object.remove("handle"), object.is_empty()) {
            (Some(Value::String(secret)), true) if !secret.trim().is_empty() => {
                Ok(Setting::Handle {
                    handle: secret.trim().to_owned(),
                })
            }
            _ => Err(unrepresentable(
                name,
                member(),
                "an object setting is only {\"handle\": \"<secret>\"}",
            )),
        };
    }
    let literal = match given {
        Value::Bool(value) => Literal::Boolean(value),
        Value::String(value) if value.contains('\0') => {
            return Err(unrepresentable(name, member(), "text holds a NUL"));
        }
        Value::String(value) => Literal::Text(value),
        Value::Number(number) => Literal::Integer(number.as_i64().ok_or_else(|| {
            unrepresentable(
                name,
                member(),
                "a number is carried only as a whole integer",
            )
        })?),
        Value::Null | Value::Array(_) | Value::Object(_) => {
            return Err(unrepresentable(
                name,
                member(),
                "a setting is a string, an integer, a boolean or a handle",
            ));
        }
    };
    if names_secret(variable) || credential_shaped(&literal.text()) {
        return Err(inline(name, member()));
    }
    Ok(Setting::Literal(literal))
}

fn command(name: &str, given: CommandBody) -> Result<McpCommand, ServerError> {
    let program = given.program;
    exact(name, "its program", &program)?;
    if let Some(cwd) = &given.cwd {
        exact(name, "cwd", cwd)?;
    }
    if credential_shaped(&program) {
        return Err(inline(name, "its program".to_owned()));
    }
    if given.args.len() > PARTS_MAX || given.env.len() > PARTS_MAX {
        return Err(malformed(format!(
            "MCP server `{name}` has more than {PARTS_MAX} arguments or settings"
        )));
    }
    let mut env = BTreeMap::new();
    for (variable, value) in given.env {
        let valid = !variable.is_empty()
            && variable
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
            && !variable.starts_with(|c: char| c.is_ascii_digit());
        if !valid {
            return Err(unrepresentable(
                name,
                format!("env `{variable}`"),
                "a variable name is letters, digits and _ and does not begin with a digit",
            ));
        }
        env.insert(variable.clone(), setting(name, &variable, value)?);
    }
    Ok(McpCommand {
        program,
        args: arguments(name, &given.args)?,
        cwd: given.cwd,
        env,
    })
}

/// The servers `given` names, each checked, in order.
pub(crate) fn servers(given: Vec<McpServerBody>) -> Result<Vec<McpServer>, ServerError> {
    if given.len() > SERVERS_MAX {
        return Err(malformed(format!(
            "mcp_servers holds more than {SERVERS_MAX} servers"
        )));
    }
    let mut kept: Vec<McpServer> = Vec::new();
    for server in given {
        let name = server.name.trim().to_owned();
        if name.is_empty() {
            return Err(malformed("a name in mcp_servers is empty".to_owned()));
        }
        if kept.iter().any(|other| other.name == name) {
            return Err(malformed(format!("MCP server `{name}` is named twice")));
        }
        let (url, command) = match (server.url, server.command) {
            (Some(url), None) => (address(&name, &url)?, None),
            (None, Some(given)) => (String::new(), Some(command(&name, given)?)),
            _ => {
                return Err(malformed(format!(
                    "MCP server `{name}` names exactly one of url and command"
                )));
            }
        };
        kept.push(McpServer {
            name,
            url,
            command,
            channel: server.channel.unwrap_or_default(),
        });
    }
    Ok(kept)
}
