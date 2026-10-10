//! What a seat's declared resource files say, sorted member by member into
//! the provisioning profile HOME-037 keeps (AGENTS-003 R1, R3).
//!
//! Every member of a source lands in exactly one place: mapped into the
//! profile version's settings, replaced by Lys's own hooks or status line,
//! excluded with its reason, or refused by name. Nothing is dropped, nothing
//! is evaluated, and a member the profile cannot carry refuses the plan
//! rather than being approximated. Which document member goes where is
//! decided in `seat_import_sources`; this module holds the sorted result and
//! the profile version destination it becomes.
//!
//! A member under a secret-shaped value is never copied: it is refused
//! `import_credential_inline` by member name, its value is neither shown nor
//! hashed, and its pointer is kept so the source revision is taken over the
//! document with that value withheld.

use std::collections::{BTreeMap, BTreeSet};

use lys_home::harness::launch_fields::{Channel, DeclaredHarness, InstructionsMode, Literal};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use crate::launch_permissions::Permissions;
use crate::provisioning_store::{McpCommand, McpServer, Setting, Settings};
use crate::seat_import_plan::{DestinationEntry, Fragment, Harness, Refusal, record};

/// A declared source file is absent.
pub const SOURCE_MISSING: &str = "import_source_missing";
/// A declared source file exists and cannot be read.
pub const SOURCE_UNREADABLE: &str = "import_source_unreadable";
/// A declared source file is not the JSON, TOML or text its kind is.
pub const SOURCE_MALFORMED: &str = "import_source_malformed";
/// A source file is larger than its bound.
pub const BOUND_EXCEEDED: &str = "import_bound_exceeded";
/// A member the destination owner cannot carry.
pub const MEMBER_UNSUPPORTED: &str = "import_member_unsupported";
/// A member that could be read two ways.
pub const MEMBER_AMBIGUOUS: &str = "import_member_ambiguous";
/// A secret-shaped value written into a source.
pub const CREDENTIAL_INLINE: &str = "import_credential_inline";
/// The manifest names sources for both harnesses.
pub const SCOPE_AMBIGUOUS: &str = "import_scope_ambiguous";

/// What a secret-shaped value is replaced by before a revision is taken.
pub const WITHHELD: &str = "<credential withheld>";

pub(crate) const NO_MEMBER: &str =
    "the provisioning profile has no member that carries this setting";

/// A setting of the harness's own interface or preferences.
pub const UI_STATE: &str =
    "harness_ui_state: the harness's own interface or preference setting, not seat configuration";
/// A setting the machine owns rather than the seat.
pub const MACHINE_OWNED: &str = "machine_owned: the machine's own setting, never a profile member";
/// A setting whose role moves to Lys's own hooks, status line and delivery.
pub const REPLACED_BY_LYS: &str =
    "replaced_by_lys: the monitor's role moves to Lys's own hooks, status line and delivery";
/// A plugin or marketplace the seat enabled.
pub const PLUGIN_SET: &str = "no Lys owner: plugin/marketplace set";
/// A setting provisioning has no field for.
pub const NO_OWNER: &str = "no Lys owner: the provisioning settings have no field for this key";
/// A per-tool setting of an MCP server.
pub const NO_TOOL_LIST: &str =
    "no Lys owner: the profile's MCP server has no tool allow list or per-tool approval";

/// The environment names the machine owns.
const MACHINE_ENV: [&str; 3] = ["TERM", "COLORTERM", "CLAUDE_CODE_ENABLE_PROMPT_SUGGESTION"];

/// What the manifest says Lys's own hooks, status line and delivery replace
/// in a seat's environment: any variable holding the monitor's address, and
/// any variable named under one of the declared prefixes.
#[derive(Debug, Clone, Copy, Default)]
pub struct Replaced<'a> {
    /// The monitor address the manifest names.
    pub monitor: Option<&'a str>,
    /// The environment name prefixes the manifest declares replaced.
    pub prefixes: &'a [String],
}

impl Replaced<'_> {
    /// Whether the variable `name` holding `value` is replaced by Lys.
    fn covers(&self, name: &str, value: &Value) -> bool {
        let held = value.as_str();
        let prefixed = self.prefixes.iter().any(|prefix| name.starts_with(prefix));
        prefixed || self.monitor.is_some_and(|base| held == Some(base))
    }
}

/// The profile members one or more sources contributed, before they become
/// the profile version destination.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProfileParts {
    /// The models it may use.
    pub models: Vec<String>,
    /// Its instructions, appended to the harness's own prompt.
    pub instructions: Option<String>,
    /// The permissions its settings carried.
    pub permissions: Option<Permissions>,
    /// Its MCP servers.
    pub mcp_servers: Vec<McpServer>,
    /// The source entries that contributed.
    pub source_ids: Vec<String>,
}

impl ProfileParts {
    /// Whether no source contributed a member.
    pub fn is_empty(&self) -> bool {
        self.models.is_empty()
            && self.instructions.is_none()
            && self.permissions.is_none()
            && self.mcp_servers.is_empty()
    }

    /// Takes `other`'s members alongside these.
    pub fn absorb(&mut self, other: Self) {
        self.models.extend(other.models);
        if other.instructions.is_some() {
            self.instructions = other.instructions;
        }
        if other.permissions.is_some() {
            self.permissions = other.permissions;
        }
        self.mcp_servers.extend(other.mcp_servers);
        self.source_ids.extend(other.source_ids);
    }
}

/// One source sorted: what it maps, and every member it does not.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Sorted {
    /// The source entry's id, which prefixes every member name.
    pub source_id: String,
    /// What it maps into the profile.
    pub parts: ProfileParts,
    /// Each excluded member with its reason.
    pub excluded: Vec<(String, String)>,
    /// Each refused member.
    pub refusals: Vec<Refusal>,
    /// Each member replaced by Lys's own hooks or status line.
    pub replacements: Vec<String>,
    /// What must be true before the plan can be confirmed or started.
    pub prerequisites: Vec<String>,
    /// The JSON pointers of every secret-shaped value.
    pub secrets: BTreeSet<String>,
}

/// A refusal named `name` for `member`.
pub fn refusal(name: &str, member: &str, detail: &str) -> Refusal {
    Refusal {
        name: name.to_owned(),
        member: member.to_owned(),
        detail: detail.to_owned(),
    }
}

/// A fragment holding nothing.
pub fn empty_fragment() -> Fragment {
    Fragment::default()
}

/// The SHA-256 of `bytes`, as lowercase hex.
pub fn sha256_hex(bytes: &[u8]) -> String {
    crate::routes::hex(&Sha256::digest(bytes))
}

pub(crate) fn dotted(parent: &str, key: &str) -> String {
    if parent.is_empty() {
        key.to_owned()
    } else {
        format!("{parent}.{key}")
    }
}

pub(crate) fn child(pointer: &str, key: &str) -> String {
    format!("{pointer}/{}", key.replace('~', "~0").replace('/', "~1"))
}

pub(crate) fn strings(value: &Value) -> Option<Vec<String>> {
    value
        .as_array()?
        .iter()
        .map(|item| item.as_str().map(str::to_owned))
        .collect()
}

impl Sorted {
    pub(crate) fn new(source_id: &str) -> Self {
        Self {
            source_id: source_id.to_owned(),
            ..Self::default()
        }
    }

    pub(crate) fn member(&self, path: &str) -> String {
        if path.is_empty() {
            self.source_id.clone()
        } else {
            format!("{}#{path}", self.source_id)
        }
    }

    pub(crate) fn refuse(&mut self, name: &str, path: &str, detail: &str) {
        let member = self.member(path);
        self.refusals.push(refusal(name, &member, detail));
    }

    pub(crate) fn unsupported(&mut self, path: &str, detail: &str) {
        self.refuse(MEMBER_UNSUPPORTED, path, detail);
    }

    pub(crate) fn exclude(&mut self, path: &str, reason: &str) {
        let member = self.member(path);
        self.excluded.push((member, reason.to_owned()));
    }

    pub(crate) fn credential(&mut self, path: &str, pointer: String) {
        if self.secrets.insert(pointer) {
            self.refuse(
                CREDENTIAL_INLINE,
                path,
                "a secret-shaped value is written here; it is not copied, shown or hashed, and the member must name a broker handle instead",
            );
            self.exclude(
                path,
                "credential_inline: the value is withheld from the plan and the receipt",
            );
        }
    }

    pub(crate) fn holds_secret(&self, pointer: &str) -> bool {
        let below = format!("{pointer}/");
        self.secrets
            .iter()
            .any(|secret| secret == pointer || secret.starts_with(&below))
    }

    pub(crate) fn replace(&mut self, path: &str, pointer: &str, value: &Value, what: &str) {
        if self.holds_secret(pointer) {
            return;
        }
        let member = self.member(path);
        match serde_json::to_vec(value) {
            Ok(bytes) => {
                self.replacements.push(format!(
                    "{member}: replaced by Lys's own {what}; source definition sha256 {}",
                    sha256_hex(&bytes)
                ));
                self.excluded.push((member, REPLACED_BY_LYS.to_owned()));
            }
            Err(error) => self.unsupported(
                path,
                &format!("its definition could not be written as JSON: {error}"),
            ),
        }
    }

    pub(crate) fn model(&mut self, path: &str, value: &Value) {
        match value.as_str() {
            Some(model) if !model.is_empty() => self.parts.models.push(model.to_owned()),
            Some(_) | None => self.unsupported(path, "the model is not one nonempty text"),
        }
    }

    pub(crate) fn instructions(&mut self, path: &str, pointer: &str, value: &Value) {
        if self.holds_secret(pointer) {
            return;
        }
        match value.as_str() {
            Some(text) => self.parts.instructions = Some(text.to_owned()),
            None => self.unsupported(path, "the instructions are not text"),
        }
    }

    fn permission_list(&mut self, path: &str, value: &Value, list: &mut Vec<String>) {
        match strings(value) {
            Some(rules) => *list = rules,
            None => self.unsupported(path, "the rules are not a list of text"),
        }
    }

    pub(crate) fn claude_permissions(&mut self, value: &Value) {
        let Value::Object(members) = value else {
            self.unsupported("permissions", "the permissions are not an object");
            return;
        };
        let mut permissions = Permissions::default();
        for (key, item) in members {
            let path = dotted("permissions", key);
            match key.as_str() {
                "defaultMode" => match item.as_str() {
                    Some(mode) => permissions.default_mode = Some(mode.to_owned()),
                    None => self.unsupported(&path, "the permission mode is not text"),
                },
                "allow" => self.permission_list(&path, item, &mut permissions.allow),
                "deny" => self.permission_list(&path, item, &mut permissions.deny),
                "ask" => self.permission_list(&path, item, &mut permissions.ask),
                "additionalDirectories" => match strings(item) {
                    Some(dirs) => permissions.additional_directories = dirs,
                    None => self.unsupported(&path, "the directories are not a list of text"),
                },
                _ => self.unsupported(&path, NO_MEMBER),
            }
        }
        self.parts.permissions = Some(permissions);
    }

    pub(crate) fn environment(&mut self, value: &Value, replaced: Replaced<'_>) {
        let Value::Object(members) = value else {
            self.unsupported("env", "the environment is not an object");
            return;
        };
        for (name, item) in members {
            let path = dotted("env", name);
            let pointer = child("/env", name);
            if self.holds_secret(&pointer) {
                continue;
            }
            if MACHINE_ENV.contains(&name.as_str()) {
                self.exclude(&path, MACHINE_OWNED);
            } else if replaced.covers(name, item) {
                self.exclude(&path, REPLACED_BY_LYS);
            } else {
                self.unsupported(
                    &path,
                    "the profile carries no harness environment; only an MCP server's command carries one",
                );
            }
        }
    }

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
        let (mut program, mut url, mut cwd) = (None, None, None);
        let (mut args, mut env) = (Vec::new(), BTreeMap::new());
        for (key, item) in members {
            let at = dotted(&path, key);
            match (key.as_str(), harness) {
                ("command", _) => program = self.text(&at, item),
                ("url", _) => url = self.text(&at, item),
                ("cwd", _) => cwd = self.text(&at, item),
                ("args", _) => match strings(item) {
                    Some(given) => args = given,
                    None => self.unsupported(&at, "the arguments are not a list of text"),
                },
                ("env", _) => env = self.env_literals(&at, item).unwrap_or_default(),
                ("type", Harness::Claude) => {
                    if !matches!(item.as_str(), Some("stdio" | "http")) {
                        self.unsupported(&at, "only command (stdio) and HTTP servers are carried");
                    }
                }
                ("enabled", Harness::Codex) => match item.as_bool() {
                    Some(true) => {}
                    Some(false) => {
                        self.exclude(
                            &path,
                            "disabled_in_source: the server is disabled in the source profile and is not imported",
                        );
                        return;
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
                    Some("wake") => self.unsupported(
                        &at,
                        "the declared Codex launch contract refuses wake channels",
                    ),
                    Some(_) | None => self.unsupported(&at, "a channel is off or wake"),
                },
                _ => self.unsupported(
                    &at,
                    "the profile's MCP server has no member that carries this",
                ),
            }
        }
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
            (None, None) => {
                self.unsupported(&path, "the server names neither a command nor an address");
                return;
            }
        };
        self.push_server(name, String::new(), command);
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

    /// Every leaf under `value` at `path` excluded with `reason`, named by
    /// its path; no value is copied.
    pub(crate) fn exclude_leaves(&mut self, path: &str, value: &Value, reason: &str) {
        match value {
            Value::Object(members) if !members.is_empty() => {
                for (key, item) in members {
                    self.exclude_leaves(&dotted(path, key), item, reason);
                }
            }
            _ => self.exclude(path, reason),
        }
    }

    pub(crate) fn each_child(
        &mut self,
        path: &str,
        value: &Value,
        mut act: impl FnMut(&mut Self, &str, &str, &Value),
    ) {
        let Value::Object(members) = value else {
            self.unsupported(path, "the member is not an object");
            return;
        };
        let pointer = child("", path);
        for (key, item) in members {
            act(self, &dotted(path, key), &child(&pointer, key), item);
        }
    }
}

/// The profile version destination for `seat` from `parts`: one entry whose
/// change is `{settings}`, naming every source entry that contributed. Its
/// harness is declared as provisioning declares one, by the build `declared`
/// names: the build the seat's own profile version declares, read by the
/// caller. A build declared under another rendering contract than the
/// manifest's harness refuses the plan `import_member_unsupported`; with no
/// build declared the version declares none, and the plan says a person must
/// declare one. Plan assembly binds the seat's agent and the profile's
/// expected revision; nothing here reads the store.
pub fn map_profile(
    seat: &str,
    harness: Harness,
    declared: Option<&DeclaredHarness>,
    parts: ProfileParts,
) -> Fragment {
    let mut fragment = empty_fragment();
    if parts.is_empty() {
        return fragment;
    }
    let harness_name = harness.name();
    let contract = harness.rendering_contract();
    if let Some(build) = declared
        && build.description.rendering_contract != contract
    {
        fragment.refusals.push(refusal(
            MEMBER_UNSUPPORTED,
            "manifest.harness",
            &format!(
                "the manifest names a {harness_name} seat ({contract}), but the seat's profile version declares the build `{}` under {}",
                build.name,
                build.description.rendering_contract
            ),
        ));
        return fragment;
    }
    let settings = Settings {
        model_access: parts.models,
        tools: Vec::new(),
        skills: Vec::new(),
        mcp_servers: parts.mcp_servers,
        instructions: parts.instructions.unwrap_or_default(),
        instructions_mode: InstructionsMode::Append,
        note: format!("imported from seat {seat}'s {harness_name} resource files"),
        session: None,
        harness: declared.cloned(),
        skill_pins: Vec::new(),
        permissions: parts.permissions,
        runs_on: None,
        writable: None,
        working_folder: None,
    };
    let settings = match serde_json::to_value(&settings) {
        Ok(settings) => settings,
        Err(error) => {
            fragment.refusals.push(refusal(
                MEMBER_UNSUPPORTED,
                record::PROFILE,
                &format!("the imported settings could not be written as JSON: {error}"),
            ));
            return fragment;
        }
    };
    let mut change = Map::new();
    change.insert("settings".to_owned(), settings);
    fragment.destinations.push(DestinationEntry {
        record_kind: record::PROFILE.to_owned(),
        record_id: seat.to_owned(),
        expected_revision: None,
        change: Value::Object(change),
        source_entry_ids: parts.source_ids,
    });
    let prerequisite = match declared {
        Some(build) => format!(
            "{} {seat}: the imported version declares the {harness_name} build `{}` the seat's profile version declares and is unreviewed; a person answering for the agent reviews it before any start",
            record::PROFILE,
            build.name
        ),
        None => format!(
            "{} {seat}: the imported version names no declared {harness_name} harness build, because the seat's profile version declares none, and is unreviewed; a person answering for the agent declares the build and reviews it before any start",
            record::PROFILE
        ),
    };
    fragment.prerequisites.push(prerequisite);
    fragment
}
