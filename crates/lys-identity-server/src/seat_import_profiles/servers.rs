//! MCP transport members are classified without reading credential files.

use super::{MEMBER_AMBIGUOUS, NO_TOOL_LIST, Sorted, child, dotted, strings};
use crate::provisioning_store::{McpCommand, McpServer, Setting};
use crate::seat_import_plan::Harness;
use lys_home::harness::launch_fields::{Channel, Literal};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

#[derive(Default)]
struct ServerParts {
    program: Option<String>,
    url: Option<String>,
    cwd: Option<String>,
    args: Vec<String>,
    env: BTreeMap<String, Setting>,
}

impl Sorted {
    fn env_literals(&mut self, path: &str, value: &Value) -> Option<BTreeMap<String, Setting>> {
        let Value::Object(members) = value else {
            self.unsupported(path, "the environment is not an object");
            return None;
        };
        let mut env = BTreeMap::new();
        for (name, item) in members {
            let literal = match item {
                Value::String(text) => Literal::Text(text.clone()),
                Value::Bool(flag) => Literal::Boolean(*flag),
                Value::Number(number) => {
                    let Some(whole) = number.as_i64() else {
                        self.unsupported(&dotted(path, name), "only whole numbers are carried");
                        continue;
                    };
                    Literal::Integer(whole)
                }
                Value::Null | Value::Array(_) | Value::Object(_) => {
                    self.unsupported(
                        &dotted(path, name),
                        "only text, numbers and booleans are carried",
                    );
                    continue;
                }
            };
            env.insert(name.clone(), Setting::Literal(literal));
        }
        Some(env)
    }

    fn server(&mut self, harness: Harness, root: &str, name: &str, value: &Value) {
        let path = dotted(root, name);
        if self.holds_secret(&child(&child("", root), name)) {
            self.exclude(
                &path,
                "credential_inline: this server carries a secret-shaped value and is not mapped until that value names a broker handle",
            );
            return;
        }
        let Value::Object(members) = value else {
            self.unsupported(&path, "a server is not an object");
            return;
        };
        let Some(ServerParts {
            program,
            url,
            cwd,
            args,
            env,
        }) = self.server_members(harness, &path, members)
        else {
            return;
        };
        let command = match (program, url) {
            (Some(program), None) => Some(McpCommand {
                program,
                args,
                cwd,
                env,
            }),
            (None, Some(address)) => {
                if !args.is_empty() || !env.is_empty() || cwd.is_some() {
                    self.refuse(
                        MEMBER_AMBIGUOUS,
                        &path,
                        "an address server also names a command's arguments, environment or folder",
                    );
                    return;
                }
                self.push_server(name, address, None);
                return;
            }
            (Some(_), Some(_)) => {
                self.refuse(
                    MEMBER_AMBIGUOUS,
                    &path,
                    "the server names both a command and an address",
                );
                return;
            }
            // Tool approvals alone start nothing: the entry is left over from a server the profile no longer starts,
            // known by its shape and never by its name.
            (None, None) if !members.is_empty() && members.keys().all(|key| key == "tools") => {
                self.exclude(
                    &path,
                    "approvals_only: this entry holds tool approvals for a server it does not start; the approvals are not carried",
                );
                return;
            }
            (None, None) => {
                self.unsupported(&path, "the server names neither a command nor an address");
                return;
            }
        };
        self.push_server(name, String::new(), command);
    }

    fn server_members(
        &mut self,
        harness: Harness,
        path: &str,
        members: &Map<String, Value>,
    ) -> Option<ServerParts> {
        let mut parts = ServerParts::default();
        for (key, item) in members {
            let at = dotted(path, key);
            match (key.as_str(), harness) {
                ("command", _) => parts.program = self.text(&at, item),
                ("url", _) => parts.url = self.text(&at, item),
                ("cwd", _) => parts.cwd = self.text(&at, item),
                ("args", _) => match strings(item) {
                    Some(given) => parts.args = given,
                    None => self.unsupported(&at, "the arguments are not a list of text"),
                },
                ("env", _) => {
                    let given = self.env_literals(&at, item)?;
                    parts.env = given;
                },
                ("type", Harness::Claude) => {
                    if !matches!(item.as_str(), Some("stdio" | "http")) {
                        self.unsupported(&at, "only command (stdio) and HTTP servers are carried");
                    }
                }
                ("enabled", Harness::Codex) => match item.as_bool() {
                    Some(true) => {}
                    Some(false) => {
                        self.exclude(
                            path,
                            "disabled_in_source: the server is disabled in the source profile and is not imported",
                        );
                        return None;
                    }
                    None => self.unsupported(&at, "enabled is not a boolean"),
                },
                ("tools", Harness::Codex) => match item {
                    Value::Object(tools) => {
                        for tool in tools.keys() {
                            self.exclude(&dotted(&at, tool), NO_TOOL_LIST);
                        }
                    }
                    _ => self.unsupported(&at, "the tools are not a table"),
                },
                ("channel", Harness::Codex) => match item.as_str() {
                    Some("off") => {}
                    Some("wake") => self.exclude(
                        &at,
                        "no Lys owner: the declared Codex launch contract cannot carry this wake policy; the server is imported with channel off",
                    ),
                    Some(_) | None => self.unsupported(&at, "a channel is off or wake"),
                },
                _ => self.unsupported(
                    &at,
                    "the profile's MCP server has no member that carries this",
                ),
            }
        }
        Some(parts)
    }

    fn push_server(&mut self, name: &str, url: String, command: Option<McpCommand>) {
        self.parts.mcp_servers.push(McpServer {
            name: name.to_owned(),
            url,
            command,
            channel: Channel::Off,
        });
    }

    fn text(&mut self, path: &str, value: &Value) -> Option<String> {
        let text = value.as_str().map(str::to_owned);
        if text.is_none() {
            self.unsupported(path, "the value is not text");
        }
        text
    }

    pub(crate) fn servers(&mut self, harness: Harness, root: &str, value: &Value) {
        match value {
            Value::Object(servers) => {
                for (name, server) in servers {
                    self.server(harness, root, name, server);
                }
            }
            _ => self.unsupported(root, "the servers are not an object"),
        }
    }
}
