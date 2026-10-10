#![cfg(test)]
//! AGENTS-003 R1: a seat's declared resource files are read and every
//! member is mapped into the provisioning profile destination or named, as
//! excluded, replaced or refused. Only the declared files are read; a
//! credential file beside them is never opened; a secret-shaped value is
//! refused by member name and never appears in the plan. Every fixture is
//! fabricated in a temporary folder and every value in it is fake.

use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

#[path = "support/harness_description.rs"]
mod harness_description;

use lys_home::harness::launch_fields::DeclaredHarness;
use lys_identity_server::seat_import_plan::{
    Completeness, Fragment, Harness, Manifest, MonitorSource,
};
use lys_identity_server::seat_import_sources::read_files;
use serde_json::Value;
use sha2::{Digest, Sha256};

type TestResult = Result<(), Box<dyn Error>>;

/// The fabricated secret every credential fixture holds; it must never
/// appear in any answer.
const CANARY: &str = "fixture-canary-7f3b9e";

const UNSUPPORTED: &str = "import_member_unsupported";

/// The fabricated monitor address the manifest names.
const MONITOR: &str = "http://127.0.0.1:9/";

/// A settings file shaped as a real seat's: every key a live seat carries,
/// with fake values.
const SETTINGS: &str = r#"{
  "permissions": {
    "defaultMode": "bypassPermissions",
    "allow": ["Read"],
    "additionalDirectories": ["/tmp/fixture-dir"]
  },
  "model": "fixture-model-1",
  "modelSettings": { "fixture-model-1": { "effortLevel": "high" } },
  "alwaysThinkingEnabled": true,
  "env": {
    "TERM": "xterm-fixture",
    "COLORTERM": "truecolor",
    "CLAUDE_CODE_ENABLE_PROMPT_SUGGESTION": "false",
    "MONITOR_URL": "http://127.0.0.1:9/",
    "MONITOR_GATE_STATE_DIR": "/tmp/fixture-gate"
  },
  "statusLine": { "type": "command", "command": "fixture-status-line", "padding": 0 },
  "hooks": { "Stop": [{ "hooks": [{ "type": "command", "command": "fixture-hook" }] }] },
  "enabledPlugins": { "fixture-plugin@fixture-market": true },
  "extraKnownMarketplaces": { "fixture-market": { "source": { "source": "directory" } } },
  "enableAllProjectMcpServers": true,
  "disableAgentView": true,
  "tui": "fullscreen",
  "skipDangerousModePermissionPrompt": true,
  "skipWorkflowUsageWarning": true,
  "autoCompactEnabled": true,
  "teammateMode": "in-process",
  "agentPushNotifEnabled": true,
  "voiceEnabled": true
}"#;

const MCP: &str = r#"{
  "mcpServers": {
    "tools": {
      "command": "/bin/fixture-tools",
      "args": ["--port", "1"],
      "env": { "FIXTURE_NAME": "fixture" }
    },
    "web": { "type": "http", "url": "http://127.0.0.1:8/mcp" }
  }
}"#;

const PROMPT: &str = "You are a fixture seat.\n";

fn write(dir: &Path, name: &str, text: &str) -> Result<PathBuf, Box<dyn Error>> {
    let path = dir.join(name);
    fs::write(&path, text)?;
    Ok(path)
}

fn chmod(path: &Path, mode: u32) -> TestResult {
    fs::set_permissions(path, fs::Permissions::from_mode(mode))?;
    Ok(())
}

fn manifest(harness: Harness, claude: Option<PathBuf>, codex: Option<PathBuf>) -> Manifest {
    Manifest {
        seat: "fixture-seat".to_owned(),
        harness,
        codex_profile: codex,
        claude_folder: claude,
        seat_identity_file: None,
        monitor: Some(MonitorSource {
            base: MONITOR.to_owned(),
            secret_env: None,
            secret_header: None,
            agent: "fixture-agent".to_owned(),
            session: None,
        }),
        replaced_env_prefixes: vec!["MONITOR_".to_owned()],
        secret_handles: Vec::new(),
        recipient_map: BTreeMap::new(),
    }
}

/// The declared files read for a seat whose profile version declares no
/// build.
fn files(given: &Manifest) -> Fragment {
    read_files(given, None)
}

/// A Claude resource folder holding the three declared files and, beside
/// them, a credential file at mode 000 holding the canary.
fn claude_folder(root: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let folder = root.join("seats").join("fixture");
    fs::create_dir_all(&folder)?;
    write(&folder, "settings.json", SETTINGS)?;
    write(&folder, "mcp.json", MCP)?;
    write(&folder, "system-prompt.md", PROMPT)?;
    let credential = write(&folder, "env", &format!("FIXTURE_TOKEN={CANARY}\n"))?;
    chmod(&credential, 0o000)?;
    Ok(folder)
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Whether `member` is excluded with a reason starting `reason`.
fn excluded(fragment: &Fragment, member: &str, reason: &str) -> bool {
    fragment
        .excluded
        .iter()
        .any(|one| one.source_id == member && one.reason.starts_with(reason))
}

fn refused(fragment: &Fragment, name: &str, member: &str) -> bool {
    fragment
        .refusals
        .iter()
        .any(|one| one.name == name && one.member == member)
}

#[test]
fn seat_import_sources_maps_declared_files() -> TestResult {
    let root = tempfile::tempdir()?;
    let folder = claude_folder(root.path())?;
    let fragment = files(&manifest(Harness::Claude, Some(folder.clone()), None));

    // A real seat's settings refuse nothing: every member the profile cannot
    // carry is a named exclusion the person sees; every other member is
    // asserted in the destination's settings below.
    assert!(fragment.refusals.is_empty(), "{:?}", fragment.refusals);
    for (member, reason) in [
        ("modelSettings.fixture-model-1.effortLevel", "no Lys owner"),
        ("alwaysThinkingEnabled", "harness_ui_state"),
        ("env.TERM", "machine_owned"),
        ("env.COLORTERM", "machine_owned"),
        ("env.CLAUDE_CODE_ENABLE_PROMPT_SUGGESTION", "machine_owned"),
        ("env.MONITOR_URL", "replaced_by_lys"),
        ("env.MONITOR_GATE_STATE_DIR", "replaced_by_lys"),
        ("statusLine", "replaced_by_lys"),
        ("hooks.Stop", "replaced_by_lys"),
        (
            "enabledPlugins.fixture-plugin@fixture-market",
            "no Lys owner: plugin",
        ),
        (
            "extraKnownMarketplaces.fixture-market",
            "no Lys owner: plugin",
        ),
        ("enableAllProjectMcpServers", "no Lys owner: plugin"),
        ("disableAgentView", "harness_ui_state"),
        ("tui", "harness_ui_state"),
        ("skipDangerousModePermissionPrompt", "harness_ui_state"),
        ("skipWorkflowUsageWarning", "harness_ui_state"),
        ("autoCompactEnabled", "harness_ui_state"),
        ("teammateMode", "harness_ui_state"),
        ("agentPushNotifEnabled", "harness_ui_state"),
        ("voiceEnabled", "harness_ui_state"),
    ] {
        let member = format!("claude.settings#{member}");
        assert!(
            excluded(&fragment, &member, reason),
            "{member} is not {reason}"
        );
    }

    assert_eq!(fragment.sources.len(), 3);
    for (id, file) in [
        ("claude.settings", "settings.json"),
        ("claude.mcp", "mcp.json"),
        ("claude.system_prompt", "system-prompt.md"),
    ] {
        let source = fragment
            .sources
            .iter()
            .find(|source| source.id == id)
            .ok_or(format!("no source entry {id}"))?;
        let path = folder.join(file);
        assert_eq!(source.locator, path.display().to_string());
        assert_eq!(source.revision_kind, "sha256_bytes");
        assert_eq!(source.source_revision, sha256(&fs::read(&path)?));
        assert_eq!(source.completeness, Completeness::Complete);
    }

    let [destination] = fragment.destinations.as_slice() else {
        return Err(format!("one destination, found {:?}", fragment.destinations).into());
    };
    assert_eq!(destination.record_kind, "profile_version");
    let change = destination
        .change
        .as_object()
        .ok_or("the change is an object")?;
    assert_eq!(change.keys().collect::<Vec<_>>(), ["settings"]);
    assert_eq!(destination.record_id, "fixture-seat");
    let settings = &destination.change["settings"];
    assert_eq!(
        settings["model_access"],
        serde_json::json!(["fixture-model-1"])
    );
    assert_eq!(settings["instructions"], PROMPT);
    assert_eq!(settings["permissions"]["default_mode"], "bypassPermissions");
    assert_eq!(
        settings["permissions"]["allow"],
        serde_json::json!(["Read"])
    );
    let directories = serde_json::json!(["/tmp/fixture-dir"]);
    assert_eq!(
        settings["permissions"]["additional_directories"],
        directories
    );
    let servers = settings["mcp_servers"]
        .as_array()
        .ok_or("mcp_servers is a list")?;
    let names: Vec<&str> = servers
        .iter()
        .filter_map(|one| one["name"].as_str())
        .collect();
    assert_eq!(names, ["tools", "web"]);
    assert_eq!(servers[0]["command"]["program"], "/bin/fixture-tools");
    assert_eq!(
        servers[0]["command"]["args"],
        serde_json::json!(["--port", "1"])
    );
    assert_eq!(servers[0]["command"]["env"]["FIXTURE_NAME"], "fixture");
    assert_eq!(servers[1]["url"], "http://127.0.0.1:8/mcp");
    for id in ["claude.settings", "claude.mcp", "claude.system_prompt"] {
        assert!(destination.source_entry_ids.iter().any(|one| one == id));
    }

    let status = "claude.settings#statusLine: replaced by Lys's own status line";
    assert!(
        fragment
            .replacements
            .iter()
            .any(|one| one.starts_with(status))
    );
    assert_eq!(settings.get("harness"), None, "no build is declared");
    Ok(())
}

#[test]
fn seat_import_sources_declares_the_seats_build_for_its_harness() -> TestResult {
    let root = tempfile::tempdir()?;
    let folder = claude_folder(root.path())?;
    let declared: DeclaredHarness = serde_json::from_value(harness_description::declared())?;
    let given = manifest(Harness::Claude, Some(folder), None);
    let fragment = read_files(&given, Some(&declared));
    assert!(fragment.refusals.is_empty(), "{:?}", fragment.refusals);
    let [destination] = fragment.destinations.as_slice() else {
        return Err(format!("one destination, found {:?}", fragment.destinations).into());
    };
    let harness = &destination.change["settings"]["harness"];
    assert_eq!(*harness, serde_json::to_value(&declared)?);

    // A build declared under another harness's contract is refused, never
    // carried into the version.
    let text = "model = \"fixture-model-1\"\n";
    let profile = write(root.path(), "fixture-channels.config.toml", text)?;
    let given = manifest(Harness::Codex, None, Some(profile));
    let fragment = read_files(&given, Some(&declared));
    assert!(
        refused(&fragment, UNSUPPORTED, "manifest.harness"),
        "{:?}",
        fragment.refusals
    );
    assert!(fragment.destinations.is_empty());
    Ok(())
}

#[test]
fn seat_import_sources_refuses_an_unknown_member() -> TestResult {
    let root = tempfile::tempdir()?;
    let folder = claude_folder(root.path())?;
    let settings = r#"{ "model": "m", "fixtureNeverSeen": 1, "env": { "FIXTURE": "1" } }"#;
    write(&folder, "settings.json", settings)?;
    let fragment = files(&manifest(Harness::Claude, Some(folder), None));
    for member in [
        "claude.settings#fixtureNeverSeen",
        "claude.settings#env.FIXTURE",
    ] {
        assert!(refused(&fragment, UNSUPPORTED, member), "{member}");
    }
    Ok(())
}

#[test]
fn seat_import_sources_never_reads_the_credential_file() -> TestResult {
    let root = tempfile::tempdir()?;
    let folder = claude_folder(root.path())?;
    let fragment = files(&manifest(Harness::Claude, Some(folder.clone()), None));
    let answer = serde_json::to_string(&fragment)?;
    assert!(!answer.contains(CANARY));
    assert!(!answer.contains(&folder.join("env").display().to_string()));
    assert!(
        fragment
            .refusals
            .iter()
            .all(|one| one.name != "import_source_unreadable"),
        "{:?}",
        fragment.refusals
    );
    Ok(())
}

#[test]
fn seat_import_keys_refuses_inline() -> TestResult {
    let root = tempfile::tempdir()?;
    let folder = root.path().join("seat");
    fs::create_dir_all(&folder)?;
    let settings = write(
        &folder,
        "settings.json",
        &format!(r#"{{ "model": "fixture-model-1", "env": {{ "FIXTURE_SECRET": "{CANARY}" }} }}"#),
    )?;
    write(&folder, "system-prompt.md", PROMPT)?;
    write(
        &folder,
        "mcp.json",
        &format!(
            r#"{{ "mcpServers": {{
                "leaky": {{ "command": "/bin/fixture", "env": {{ "FIXTURE_API_TOKEN": "{CANARY}" }} }},
                "flagged": {{ "command": "/bin/fixture", "args": ["--token", "{CANARY}"] }},
                "addressed": {{ "type": "http", "url": "https://fixture:{CANARY}@127.0.0.1/mcp" }},
                "queried": {{ "type": "http", "url": "https://127.0.0.1/mcp?access_token={CANARY}" }},
                "clean": {{ "command": "/bin/fixture-clean" }}
            }} }}"#
        ),
    )?;
    let fragment = files(&manifest(Harness::Claude, Some(folder), None));

    for member in [
        "claude.settings#env.FIXTURE_SECRET",
        "claude.mcp#mcpServers.leaky.env.FIXTURE_API_TOKEN",
        "claude.mcp#mcpServers.flagged.args[1]",
        "claude.mcp#mcpServers.addressed.url",
        "claude.mcp#mcpServers.queried.url",
    ] {
        assert!(
            refused(&fragment, "import_credential_inline", member),
            "{member}"
        );
        assert!(
            fragment.excluded.iter().any(|one| one.source_id == member),
            "{member}"
        );
    }
    let answer = serde_json::to_string(&fragment)?;
    assert!(!answer.contains(CANARY));

    let source = fragment
        .sources
        .iter()
        .find(|source| source.id == "claude.settings")
        .ok_or("no settings source")?;
    assert_eq!(source.revision_kind, "sha256_redacted_canonical");
    assert_ne!(source.source_revision, sha256(&fs::read(&settings)?));

    let servers: Vec<&str> = fragment
        .destinations
        .iter()
        .flat_map(|one| {
            one.change["settings"]["mcp_servers"]
                .as_array()
                .into_iter()
                .flatten()
        })
        .filter_map(|one| one["name"].as_str())
        .collect();
    assert_eq!(servers, ["clean"]);
    Ok(())
}

#[test]
fn seat_import_sources_distinguishes_roots() -> TestResult {
    let root = tempfile::tempdir()?;
    let first = root.path().join("one").join("seats").join("fixture");
    let second = root.path().join("two").join("seats").join("fixture");
    for (folder, model) in [(&first, "model-one"), (&second, "model-two")] {
        fs::create_dir_all(folder)?;
        let settings = format!(r#"{{ "model": "{model}" }}"#);
        write(folder, "settings.json", &settings)?;
        write(folder, "mcp.json", r#"{ "mcpServers": {} }"#)?;
        write(folder, "system-prompt.md", PROMPT)?;
    }
    let fragment = files(&manifest(Harness::Claude, Some(second.clone()), None));
    let shown = second.display().to_string();
    assert!(
        fragment
            .sources
            .iter()
            .all(|one| one.locator.starts_with(&shown))
    );
    let [destination] = fragment.destinations.as_slice() else {
        return Err("one destination".into());
    };
    assert_eq!(
        destination.change["settings"]["model_access"],
        serde_json::json!(["model-two"])
    );
    Ok(())
}

#[test]
fn seat_import_sources_names_each_read_failure() -> TestResult {
    let root = tempfile::tempdir()?;

    let missing = root.path().join("missing");
    fs::create_dir_all(&missing)?;
    write(&missing, "settings.json", SETTINGS)?;
    write(&missing, "system-prompt.md", PROMPT)?;
    let fragment = files(&manifest(Harness::Claude, Some(missing.clone()), None));
    let locator = missing.join("mcp.json").display().to_string();
    assert!(refused(&fragment, "import_source_missing", &locator));
    assert!(fragment.destinations.is_empty());
    let source = fragment
        .sources
        .iter()
        .find(|one| one.id == "claude.mcp")
        .ok_or("the missing file keeps its entry")?;
    assert!(matches!(
        source.completeness,
        Completeness::Incomplete { .. }
    ));

    let malformed = root.path().join("malformed");
    fs::create_dir_all(&malformed)?;
    write(&malformed, "settings.json", "{ not json")?;
    write(&malformed, "mcp.json", MCP)?;
    write(&malformed, "system-prompt.md", PROMPT)?;
    let fragment = files(&manifest(Harness::Claude, Some(malformed.clone()), None));
    let locator = malformed.join("settings.json").display().to_string();
    assert!(refused(&fragment, "import_source_malformed", &locator));
    assert!(fragment.destinations.is_empty());

    let linked = root.path().join("linked");
    fs::create_dir_all(&linked)?;
    let elsewhere = write(root.path(), "elsewhere.json", SETTINGS)?;
    std::os::unix::fs::symlink(&elsewhere, linked.join("settings.json"))?;
    write(&linked, "mcp.json", MCP)?;
    write(&linked, "system-prompt.md", PROMPT)?;
    let fragment = files(&manifest(Harness::Claude, Some(linked.clone()), None));
    let locator = linked.join("settings.json").display().to_string();
    assert!(refused(&fragment, "import_source_unreadable", &locator));
    assert!(fragment.destinations.is_empty());

    let large = root.path().join("large");
    fs::create_dir_all(&large)?;
    let padding = "x".repeat(1 << 20);
    let settings = format!(r#"{{ "model": "{padding}" }}"#);
    write(&large, "settings.json", &settings)?;
    write(&large, "mcp.json", MCP)?;
    write(&large, "system-prompt.md", PROMPT)?;
    let fragment = files(&manifest(Harness::Claude, Some(large.clone()), None));
    let locator = large.join("settings.json").display().to_string();
    assert!(refused(&fragment, "import_bound_exceeded", &locator));
    assert!(fragment.destinations.is_empty());
    Ok(())
}

#[test]
fn seat_import_sources_refuses_a_manifest_naming_both_harnesses() -> TestResult {
    let root = tempfile::tempdir()?;
    let folder = claude_folder(root.path())?;
    let profile = root.path().join("fixture-channels.config.toml");
    fs::write(&profile, "model = \"m\"\n")?;
    let fragment = files(&manifest(Harness::Claude, Some(folder), Some(profile)));
    let member = "manifest.codex_profile";
    assert!(refused(&fragment, "import_scope_ambiguous", member));
    assert!(fragment.sources.iter().all(|one| one.id != "codex.profile"));
    Ok(())
}

/// A channels profile shaped as a real seat's, with fake values.
const CODEX: &str = r#"
model = "fixture-codex-model"
model_reasoning_effort = "high"
approval_policy = "never"
sandbox_mode = "workspace-write"
approvals_reviewer = "user"
developer_instructions = "Fixture instructions."
service_tier = "fast"

[projects."/tmp/fixture-project"]
trust_level = "trusted"

[hooks.state."fixture-plugin:hooks/hooks.json:stop:0:0"]
trusted_hash = "fixture-hash"

[mcp_servers.local]
command = "/bin/fixture-local"
args = ["serve"]
enabled = true
channel = "off"

[mcp_servers.local.env]
FIXTURE_LEVEL = 3

[mcp_servers.local.tools.read]
approval_mode = "approve"

[mcp_servers.remote]
url = "http://127.0.0.1:7/mcp"
enabled = true

[mcp_servers.dormant]
command = "/bin/fixture-dormant"
enabled = false

[tui.model_availability_nux]
"fixture-model" = 1
"#;

#[test]
fn seat_import_sources_preserves_channels() -> TestResult {
    let root = tempfile::tempdir()?;
    let profile = write(root.path(), "fixture-channels.config.toml", CODEX)?;
    let fragment = files(&manifest(Harness::Codex, None, Some(profile.clone())));

    // A real seat's profile refuses nothing: every member the profile cannot
    // carry is a named exclusion; model, sandbox_mode, developer_instructions
    // and the local and remote servers are asserted in the settings below.
    assert!(fragment.refusals.is_empty(), "{:?}", fragment.refusals);
    for (member, reason) in [
        ("model_reasoning_effort", "no Lys owner"),
        ("service_tier", "no Lys owner"),
        ("approvals_reviewer", "no Lys owner"),
        ("approval_policy", "machine_owned"),
        ("projects./tmp/fixture-project", "machine_owned"),
        ("hooks.state", "replaced_by_lys"),
        ("mcp_servers.local.tools.read", "no Lys owner"),
        ("mcp_servers.dormant", "disabled_in_source"),
        ("tui.model_availability_nux", "harness_ui_state"),
    ] {
        let member = format!("codex.profile#{member}");
        assert!(
            excluded(&fragment, &member, reason),
            "{member} is not {reason}"
        );
    }

    let [source] = fragment.sources.as_slice() else {
        return Err("one source entry".into());
    };
    assert_eq!(source.kind, "codex_profile");
    assert_eq!(source.source_revision, sha256(&fs::read(&profile)?));

    let [destination] = fragment.destinations.as_slice() else {
        return Err("one destination".into());
    };
    let settings = &destination.change["settings"];
    assert_eq!(
        settings["model_access"],
        serde_json::json!(["fixture-codex-model"])
    );
    assert_eq!(settings["instructions"], "Fixture instructions.");
    assert_eq!(settings["permissions"]["default_mode"], "workspace-write");
    let servers = settings["mcp_servers"]
        .as_array()
        .ok_or("mcp_servers is a list")?;
    let local = servers
        .iter()
        .find(|one| one["name"] == "local")
        .ok_or("local is mapped")?;
    assert_eq!(local["command"]["env"]["FIXTURE_LEVEL"], 3);
    assert!(local.get("channel").is_none_or(|channel| channel == "off"));
    assert!(servers.iter().all(|one| one["name"] != "dormant"));
    assert!(
        servers
            .iter()
            .any(|one| one["url"] == "http://127.0.0.1:7/mcp")
    );
    Ok(())
}

#[test]
fn seat_import_sources_refuses_a_wake_channel_it_cannot_keep() -> TestResult {
    let root = tempfile::tempdir()?;
    let text = "[mcp_servers.waker]\ncommand = \"/bin/fixture-wake\"\nchannel = \"wake\"\n";
    let profile = write(root.path(), "fixture-channels.config.toml", text)?;
    let fragment = files(&manifest(Harness::Codex, None, Some(profile)));
    let member = "codex.profile#mcp_servers.waker.channel";
    assert!(refused(&fragment, UNSUPPORTED, member));
    Ok(())
}

#[test]
fn seat_import_sources_reads_nothing_it_was_not_given() -> TestResult {
    let mut given = manifest(Harness::Codex, None, None);
    given.monitor = None;
    let fragment = files(&given);
    let member = "manifest.codex_profile";
    assert!(refused(&fragment, "import_source_missing", member));
    assert!(fragment.sources.is_empty());
    assert!(fragment.destinations.is_empty());
    let value: Value = serde_json::to_value(&fragment)?;
    assert!(value["references"].as_array().is_some_and(Vec::is_empty));
    Ok(())
}
