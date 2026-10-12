//! Source boundaries and Codex member classification.

use super::{
    Completeness, Harness, MCP, PROMPT, SETTINGS, TestResult, Value, claude_folder, excluded,
    files, fs, manifest, refused, sha256, write,
};

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
fn seat_import_sources_excludes_a_wake_channel_it_cannot_keep() -> TestResult {
    let root = tempfile::tempdir()?;
    let text = "[mcp_servers.waker]\ncommand = \"/bin/fixture-wake\"\nchannel = \"wake\"\n";
    let profile = write(root.path(), "fixture-channels.config.toml", text)?;
    let fragment = files(&manifest(Harness::Codex, None, Some(profile)));
    let member = "codex.profile#mcp_servers.waker.channel";
    assert!(fragment.refusals.is_empty(), "{:?}", fragment.refusals);
    assert!(excluded(&fragment, member, "no Lys owner"));
    let [destination] = fragment.destinations.as_slice() else {
        return Err("one destination".into());
    };
    let server = &destination.change["settings"]["mcp_servers"][0];
    assert_eq!(server["name"], "waker");
    assert_eq!(server["command"]["program"], "/bin/fixture-wake");
    assert!(server.get("channel").is_none_or(|channel| channel == "off"));
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
