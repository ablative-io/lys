//! The seat's declared resource files, read for a dry run (AGENTS-003 R1).
//!
//! Only the files the manifest declares are opened: for a Claude seat,
//! `settings.json`, `system-prompt.md` and `mcp.json` in the folder the
//! manifest names; for a Codex seat, the `<name>-channels.config.toml`
//! profile the manifest names. No folder is listed and nothing beside those
//! files is opened, so a credential file kept beside them is never read.
//! Each file must be a regular file (never a link) of at most 1,048,576
//! bytes.
//!
//! Every file becomes one source entry. A file that is missing, unreadable,
//! too large or not the JSON, TOML or text its kind is refuses by name and
//! keeps its entry, marked incomplete, and then no destination is offered
//! at all: a plan is never built from part of the declared files.
//!
//! A file's revision is the SHA-256 of its exact bytes. When the file holds
//! a secret-shaped value, that value is refused by member name and the
//! revision is taken instead over the parsed document with every such value
//! withheld, and is marked so; a prompt text holding one has no revision.
//! Before any member is sorted the whole document is scanned: an
//! environment or header value under a secret name, a secret flag's
//! argument, a bearer or private-key text, and an address with a user part
//! or a secret-named query are each refused `import_credential_inline`.

use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::{ErrorKind, Read};
use std::path::Path;

use lys_home::harness::launch_fields::DeclaredHarness;
use serde_json::Value;

use crate::launch_permissions::Permissions;
use crate::seat_import_plan::{Completeness, Excluded, Fragment, Harness, Manifest, SourceEntry};
use crate::seat_import_profiles::{
    BOUND_EXCEEDED, MEMBER_UNSUPPORTED, NO_MEMBER, NO_OWNER, PLUGIN_SET, ProfileParts, Replaced,
    SCOPE_AMBIGUOUS, SOURCE_MALFORMED, SOURCE_MISSING, SOURCE_UNREADABLE, Sorted, UI_STATE,
    WITHHELD, child, dotted, empty_fragment, map_profile, refusal, sha256_hex,
};

/// The largest declared resource file read, in bytes.
pub const RESOURCE_BOUND: u64 = 1 << 20;
const RESOURCE_BOUND_BYTES: usize = 1 << 20;

/// The Claude settings file, in the seat's resource folder.
pub const CLAUDE_SETTINGS: &str = "settings.json";
/// The Claude system prompt file, in the seat's resource folder.
pub const CLAUDE_PROMPT: &str = "system-prompt.md";
/// The Claude MCP configuration file, in the seat's resource folder.
pub const CLAUDE_MCP: &str = "mcp.json";

/// The revision kind of a file's exact bytes.
pub const REVISION_BYTES: &str = "sha256_bytes";
/// The revision kind of a document with its secret-shaped values withheld.
pub const REVISION_REDACTED: &str = "sha256_redacted_canonical";
/// The revision kind of a source that has no revision.
pub const REVISION_NONE: &str = "none";

const CODEX_MACHINE_OWNED: &str = "machine_owned: the approval policy and project trust are the machine's own Codex configuration, never a profile member (the Codex launch takes them from the machine)";
/// The Claude settings that are the harness's own interface or preferences.
const CLAUDE_UI: [&str; 7] = [
    "alwaysThinkingEnabled",
    "tui",
    "voiceEnabled",
    "agentPushNotifEnabled",
    "teammateMode",
    "autoCompactEnabled",
    "disableAgentView",
];
const SECRET_WORDS: [&str; 8] = [
    "key",
    "token",
    "secret",
    "password",
    "passwd",
    "bearer",
    "authorization",
    "cookie",
];
const SANDBOX_MODES: [&str; 3] = ["read-only", "workspace-write", "danger-full-access"];

/// The kinds of declared resource file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    ClaudeSettings,
    ClaudeMcp,
    ClaudePrompt,
    CodexProfile,
}

impl Kind {
    fn id(self) -> &'static str {
        match self {
            Self::ClaudeSettings => "claude.settings",
            Self::ClaudeMcp => "claude.mcp",
            Self::ClaudePrompt => "claude.system_prompt",
            Self::CodexProfile => "codex.profile",
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::ClaudeSettings => "claude_settings",
            Self::ClaudeMcp => "claude_mcp",
            Self::ClaudePrompt => "claude_system_prompt",
            Self::CodexProfile => "codex_profile",
        }
    }
}

/// Every declared resource file of `manifest`'s seat, read and sorted into
/// the provisioning profile destination, with every member it cannot map
/// named; the destination declares the build `declared` names, the one the
/// seat's own profile version declares. Reads nothing the manifest does not
/// declare and writes nothing.
pub fn read_files(manifest: &Manifest, declared: Option<&DeclaredHarness>) -> Fragment {
    let mut fragment = empty_fragment();
    if manifest
        .replaced_env_prefixes
        .iter()
        .any(|prefix| prefix.is_empty())
    {
        fragment.refusals.push(refusal(
            MEMBER_UNSUPPORTED,
            "manifest.replaced_env_prefixes",
            "an empty prefix would name every variable; name each replaced prefix whole",
        ));
        return fragment;
    }
    let files = match manifest.harness {
        Harness::Claude => {
            if manifest.codex_profile.is_some() {
                fragment.refusals.push(refusal(
                    SCOPE_AMBIGUOUS,
                    "manifest.codex_profile",
                    "a Claude seat's manifest names a Codex profile; name only the Claude resource folder",
                ));
            }
            let Some(folder) = &manifest.claude_folder else {
                fragment.refusals.push(refusal(
                    SOURCE_MISSING,
                    "manifest.claude_folder",
                    "a Claude seat's manifest names no resource folder",
                ));
                return fragment;
            };
            vec![
                (Kind::ClaudeSettings, folder.join(CLAUDE_SETTINGS)),
                (Kind::ClaudePrompt, folder.join(CLAUDE_PROMPT)),
                (Kind::ClaudeMcp, folder.join(CLAUDE_MCP)),
            ]
        }
        Harness::Codex => {
            if manifest.claude_folder.is_some() {
                fragment.refusals.push(refusal(
                    SCOPE_AMBIGUOUS,
                    "manifest.claude_folder",
                    "a Codex seat's manifest names a Claude resource folder; name only the Codex profile",
                ));
            }
            let Some(profile) = &manifest.codex_profile else {
                fragment.refusals.push(refusal(
                    SOURCE_MISSING,
                    "manifest.codex_profile",
                    "a Codex seat's manifest names no channels profile",
                ));
                return fragment;
            };
            vec![(Kind::CodexProfile, profile.clone())]
        }
    };
    let replaced = Replaced {
        monitor: manifest.monitor.as_ref().map(|source| source.base.as_str()),
        prefixes: &manifest.replaced_env_prefixes,
    };
    let mut parts = ProfileParts::default();
    let mut whole = true;
    for (kind, path) in files {
        match read_one(&manifest.seat, kind, &path, replaced, &mut fragment) {
            Some(sorted) => parts.absorb(sorted.parts),
            None => whole = false,
        }
    }
    if whole {
        fragment.merge(map_profile(
            &manifest.seat,
            manifest.harness,
            declared,
            parts,
        ));
    }
    fragment
}

/// One file read, sorted and recorded in `fragment`; none when it could not
/// be read whole.
fn read_one(
    seat: &str,
    kind: Kind,
    path: &Path,
    replaced: Replaced<'_>,
    fragment: &mut Fragment,
) -> Option<Sorted> {
    let locator = path.display().to_string();
    let entry = |revision_kind: &str, revision: String, completeness: Completeness| SourceEntry {
        id: kind.id().to_owned(),
        kind: kind.name().to_owned(),
        locator: locator.clone(),
        scope: format!("seat:{seat}"),
        revision_kind: revision_kind.to_owned(),
        source_revision: revision,
        completeness,
    };
    let read = read_bounded(path).and_then(|bytes| {
        let (sorted, document) = sort(kind, &bytes, replaced)?;
        Ok((bytes, sorted, document))
    });
    let (bytes, mut sorted, document) = match read {
        Ok(read) => read,
        Err((name, detail)) => {
            fragment.refusals.push(refusal(name, &locator, &detail));
            fragment.sources.push(entry(
                REVISION_NONE,
                String::new(),
                Completeness::Incomplete { reason: detail },
            ));
            return None;
        }
    };
    let (revision_kind, revision, completeness) =
        revision_of(&bytes, document.as_ref(), &sorted.secrets);
    fragment
        .sources
        .push(entry(revision_kind, revision.clone(), completeness));
    for (member, reason) in std::mem::take(&mut sorted.excluded) {
        fragment.excluded.push(Excluded {
            source_id: member,
            revision: revision.clone(),
            reason,
        });
    }
    fragment.refusals.append(&mut sorted.refusals);
    fragment.replacements.append(&mut sorted.replacements);
    fragment.prerequisites.append(&mut sorted.prerequisites);
    sorted.parts.source_ids.push(kind.id().to_owned());
    Some(sorted)
}

type Failure = (&'static str, String);

/// The bytes of the regular file at `path`, at most [`RESOURCE_BOUND`].
fn read_bounded(path: &Path) -> Result<Vec<u8>, Failure> {
    let shown = path.display();
    let unreadable = |error: std::io::Error| match error.kind() {
        ErrorKind::NotFound => (SOURCE_MISSING, format!("{shown} does not exist")),
        _ => (
            SOURCE_UNREADABLE,
            format!("{shown} cannot be read: {error}"),
        ),
    };
    let metadata = fs::symlink_metadata(path).map_err(unreadable)?;
    if metadata.file_type().is_symlink() {
        return Err((
            SOURCE_UNREADABLE,
            format!("{shown} is a symbolic link; the manifest names the file itself"),
        ));
    }
    if !metadata.is_file() {
        return Err((SOURCE_UNREADABLE, format!("{shown} is not a regular file")));
    }
    if metadata.len() > RESOURCE_BOUND {
        return Err((
            BOUND_EXCEEDED,
            format!("{shown} is larger than the {RESOURCE_BOUND}-byte resource bound"),
        ));
    }
    let mut bytes = Vec::new();
    File::open(path)
        .and_then(|file| file.take(RESOURCE_BOUND + 1).read_to_end(&mut bytes))
        .map_err(unreadable)?;
    if bytes.len() > RESOURCE_BOUND_BYTES {
        return Err((
            BOUND_EXCEEDED,
            format!("{shown} grew past the {RESOURCE_BOUND}-byte resource bound as it was read"),
        ));
    }
    Ok(bytes)
}

/// `bytes` parsed as `kind` is, and sorted, with the parsed document; none
/// for the prompt, which is text.
fn sort(
    kind: Kind,
    bytes: &[u8],
    replaced: Replaced<'_>,
) -> Result<(Sorted, Option<Value>), Failure> {
    let malformed = |what: &str, error: &dyn std::fmt::Display| {
        (
            SOURCE_MALFORMED,
            format!("{} is not {what}: {error}", kind.id()),
        )
    };
    match kind {
        Kind::ClaudeSettings | Kind::ClaudeMcp => {
            let document: Value =
                serde_json::from_slice(bytes).map_err(|error| malformed("JSON", &error))?;
            let sorted = if kind == Kind::ClaudeSettings {
                claude_settings(kind.id(), &document, replaced)
            } else {
                claude_mcp(kind.id(), &document)
            };
            Ok((sorted, Some(document)))
        }
        Kind::CodexProfile => {
            let text = std::str::from_utf8(bytes).map_err(|error| malformed("UTF-8", &error))?;
            let table: toml::Table =
                toml::from_str(text).map_err(|error| malformed("TOML", &error))?;
            let document =
                serde_json::to_value(&table).map_err(|error| malformed("a JSON tree", &error))?;
            Ok((codex_profile(kind.id(), &document), Some(document)))
        }
        Kind::ClaudePrompt => {
            let text = std::str::from_utf8(bytes).map_err(|error| malformed("UTF-8", &error))?;
            Ok((claude_prompt(kind.id(), text), None))
        }
    }
}

/// A source's revision kind, revision and completeness: the SHA-256 of its
/// exact bytes, or, when it holds a secret-shaped value, of its document
/// with every such value withheld. A text holding one has no revision.
fn revision_of(
    bytes: &[u8],
    document: Option<&Value>,
    secrets: &BTreeSet<String>,
) -> (&'static str, String, Completeness) {
    if secrets.is_empty() {
        return (REVISION_BYTES, sha256_hex(bytes), Completeness::Complete);
    }
    let Some(document) = document else {
        return no_revision("the text holds a secret-shaped value");
    };
    match withheld(document, secrets) {
        Ok(redacted) => (
            REVISION_REDACTED,
            sha256_hex(&redacted),
            Completeness::Complete,
        ),
        Err(reason) => no_revision(&reason),
    }
}

fn no_revision(reason: &str) -> (&'static str, String, Completeness) {
    (
        REVISION_NONE,
        String::new(),
        Completeness::Incomplete {
            reason: format!("no revision is taken: {reason}"),
        },
    )
}

/// The document's JSON with every secret-shaped value withheld.
fn withheld(value: &Value, secrets: &BTreeSet<String>) -> Result<Vec<u8>, String> {
    let mut copy = value.clone();
    for pointer in secrets {
        let Some(slot) = copy.pointer_mut(pointer) else {
            return Err(format!(
                "the withheld member {pointer} is not in the document"
            ));
        };
        *slot = Value::String(WITHHELD.to_owned());
    }
    serde_json::to_vec(&copy).map_err(|error| error.to_string())
}

/// Whether a variable, header, flag or query name names a secret.
pub fn secret_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    SECRET_WORDS.iter().any(|word| lower.contains(word))
}

/// Whether `text` is shaped as a secret wherever it is written: a private
/// key, a bearer or basic credential, or an address carrying a user part or
/// a secret-named query value.
pub fn secret_value(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("-----begin")
        || lower.starts_with("bearer ")
        || lower.starts_with("basic ")
        || address_carries_secret(text)
}

fn address_carries_secret(text: &str) -> bool {
    let Some((_, rest)) = text.split_once("://") else {
        return false;
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    if authority.contains('@') {
        return true;
    }
    let Some((_, query)) = rest.split_once('?') else {
        return false;
    };
    let query = query.split('#').next().unwrap_or_default();
    query.split('&').any(|pair| {
        pair.split_once('=')
            .is_some_and(|(name, value)| secret_name(name) && !value.is_empty())
    })
}

enum Flag {
    Plain,
    Inline,
    Next,
}

fn secret_flag(arg: &str) -> Flag {
    let body = arg.trim_start_matches('-');
    if body.len() == arg.len() {
        return Flag::Plain;
    }
    match body.split_once('=') {
        Some((name, value)) if secret_name(name) && !value.is_empty() => Flag::Inline,
        None if secret_name(body) => Flag::Next,
        Some(_) | None => Flag::Plain,
    }
}

/// Every secret-shaped value under `value`, refused into `sorted`.
fn scan(sorted: &mut Sorted, value: &Value, path: &str, pointer: &str, parent: &str) {
    match value {
        Value::String(text) if secret_value(text) => sorted.credential(path, pointer.to_owned()),
        Value::Array(items) => {
            let mut next_is_secret = false;
            for (index, item) in items.iter().enumerate() {
                let at = format!("{path}[{index}]");
                let below = format!("{pointer}/{index}");
                if parent == "args"
                    && let Value::String(arg) = item
                {
                    if std::mem::take(&mut next_is_secret) {
                        sorted.credential(&at, below);
                        continue;
                    }
                    match secret_flag(arg) {
                        Flag::Inline => {
                            sorted.credential(&at, below);
                            continue;
                        }
                        Flag::Next => next_is_secret = true,
                        Flag::Plain => {}
                    }
                }
                scan(sorted, item, &at, &below, "");
            }
        }
        Value::Object(members) => {
            for (key, item) in members {
                let at = dotted(path, key);
                let below = child(pointer, key);
                let named = matches!(parent, "env" | "headers" | "http_headers");
                if named && secret_name(key) && item.as_str().is_some_and(|text| !text.is_empty()) {
                    sorted.credential(&at, below);
                    continue;
                }
                scan(sorted, item, &at, &below, key);
            }
        }
        Value::String(_) | Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

/// The Claude `settings.json` document sorted; `replaced` names the
/// environment entries Lys replaces: the monitor address the manifest names
/// and the prefixes it declares.
pub fn claude_settings(source_id: &str, document: &Value, replaced: Replaced<'_>) -> Sorted {
    let mut sorted = Sorted::new(source_id);
    scan(&mut sorted, document, "", "", "");
    let Value::Object(members) = document else {
        sorted.unsupported("", "the settings are not a JSON object");
        return sorted;
    };
    for (key, item) in members {
        match key.as_str() {
            "model" => sorted.model(key, item),
            "permissions" => sorted.claude_permissions(item),
            "env" => sorted.environment(item, replaced),
            "statusLine" => sorted.replace(key, &child("", key), item, "status line"),
            "hooks" => sorted.each_child(key, item, |sorted, path, pointer, hook| {
                sorted.replace(path, pointer, hook, "hooks");
            }),
            "enabledPlugins" | "extraKnownMarketplaces" => {
                sorted.each_child(key, item, |sorted, path, _, _| {
                    sorted.exclude(path, PLUGIN_SET);
                });
            }
            "enableAllProjectMcpServers" => sorted.exclude(key, PLUGIN_SET),
            "modelSettings" => sorted.exclude_leaves(key, item, NO_OWNER),
            name if CLAUDE_UI.contains(&name) || name.starts_with("skip") => {
                sorted.exclude(key, UI_STATE);
            }
            _ => sorted.unsupported(key, NO_MEMBER),
        }
    }
    sorted
}

/// The Claude `mcp.json` document sorted.
pub fn claude_mcp(source_id: &str, document: &Value) -> Sorted {
    let mut sorted = Sorted::new(source_id);
    scan(&mut sorted, document, "", "", "");
    let Value::Object(members) = document else {
        sorted.unsupported("", "the MCP configuration is not a JSON object");
        return sorted;
    };
    for (key, item) in members {
        match key.as_str() {
            "mcpServers" => sorted.servers(Harness::Claude, key, item),
            _ => sorted.unsupported(key, NO_MEMBER),
        }
    }
    sorted.prerequisites.push(format!(
        "{}: the Claude resource files carry no channel policy, so every server is imported with its channel off; a person sets any wake channel on the profile version after import",
        sorted.member("mcpServers")
    ));
    sorted
}

/// The Claude `system-prompt.md` text sorted: the profile's instructions,
/// appended to the harness's own prompt as the file was.
pub fn claude_prompt(source_id: &str, text: &str) -> Sorted {
    let mut sorted = Sorted::new(source_id);
    if text.to_ascii_lowercase().contains("-----begin") {
        sorted.credential("", String::new());
        return sorted;
    }
    sorted.parts.instructions = Some(text.to_owned());
    sorted
}

/// A Codex `<name>-channels.config.toml` profile, read as JSON, sorted.
pub fn codex_profile(source_id: &str, document: &Value) -> Sorted {
    let mut sorted = Sorted::new(source_id);
    scan(&mut sorted, document, "", "", "");
    let Value::Object(members) = document else {
        sorted.unsupported("", "the profile is not a TOML table");
        return sorted;
    };
    for (key, item) in members {
        match key.as_str() {
            "model" => sorted.model(key, item),
            "developer_instructions" => sorted.instructions(key, &child("", key), item),
            "sandbox_mode" => match item.as_str() {
                Some(mode) if SANDBOX_MODES.contains(&mode) => {
                    sorted.parts.permissions = Some(Permissions {
                        default_mode: Some(mode.to_owned()),
                        ..Permissions::default()
                    });
                }
                Some(_) | None => {
                    sorted.unsupported(key, "the sandbox mode is not one the Codex launch takes")
                }
            },
            "approval_policy" => sorted.exclude(key, CODEX_MACHINE_OWNED),
            "model_reasoning_effort" | "service_tier" | "approvals_reviewer" => {
                sorted.exclude(key, NO_OWNER);
            }
            "projects" => sorted.each_child(key, item, |sorted, path, _, _| {
                sorted.exclude(path, CODEX_MACHINE_OWNED);
            }),
            "hooks" => sorted.each_child(key, item, |sorted, path, pointer, hook| {
                sorted.replace(path, pointer, hook, "hooks");
            }),
            "tui" => sorted.each_child(key, item, |sorted, path, _, _| {
                sorted.exclude(path, UI_STATE);
            }),
            "mcp_servers" => sorted.servers(Harness::Codex, key, item),
            _ => sorted.unsupported(key, NO_MEMBER),
        }
    }
    sorted
}
