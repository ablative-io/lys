//! Regression fixtures preserve the members that prevented seat imports.

use super::{Harness, TestResult, Value, files, manifest, write};
use lys_identity_server::seat_import_plan::{Fragment, SeatFacts, assemble, confirmable};
use lys_identity_server::seat_import_profiles::Replaced;
use lys_identity_server::seat_import_sources::{claude_mcp, claude_settings, codex_profile};
use serde_json::json;

fn live_shape(seat: &str) -> TestResult {
    let cases: Vec<Value> = serde_json::from_str(include_str!("live_shapes.json"))?;
    let case = cases
        .iter()
        .find(|case| case["seat"] == seat)
        .ok_or("seat fixture missing")?;
    let root = tempfile::tempdir()?;
    let given = if case["harness"] == "claude" {
        write(root.path(), "settings.json", &case["settings"].to_string())?;
        write(root.path(), "mcp.json", &case["mcp"].to_string())?;
        write(root.path(), "system-prompt.md", "Seat instructions.\n")?;
        manifest(Harness::Claude, Some(root.path().to_path_buf()), None)
    } else {
        let profile = write(
            root.path(),
            "channels.toml",
            &toml::to_string(&case["profile"])?,
        )?;
        manifest(Harness::Codex, None, Some(profile))
    };
    let fragment = files(&given);
    assert!(
        fragment.refusals.is_empty(),
        "{seat}: {:?}",
        fragment.refusals
    );
    let [destination] = fragment.destinations.as_slice() else {
        return Err(format!("{seat}: expected one destination").into());
    };
    let settings = &destination.change["settings"];
    let servers = settings["mcp_servers"]
        .as_array()
        .ok_or("server list missing")?;
    let haematite = servers
        .iter()
        .find(|server| server["name"] == "haematite")
        .ok_or("key-file server was dropped")?;
    let original = if given.harness == Harness::Claude {
        &case["mcp"]["mcpServers"]["haematite"]["args"]
    } else {
        &case["profile"]["mcp_servers"]["haematite"]["args"]
    };
    assert_eq!(&haematite["command"]["args"], original);
    assert!(
        fragment
            .sources
            .iter()
            .all(|source| source.revision_kind == "sha256_bytes")
    );
    for refusal in case["reported_refusals"]
        .as_array()
        .ok_or("reported refusals missing")?
    {
        let member = refusal["member"].as_str().ok_or("member missing")?;
        if member.contains("haematite.args") {
            continue;
        }
        let excluded = fragment
            .excluded
            .iter()
            .find(|entry| entry.source_id == member)
            .ok_or_else(|| format!("{seat}: {member} was silently dropped"))?;
        assert!(!excluded.reason.is_empty());
        if member.contains(".channel") || member.contains("#env.") {
            assert!(excluded.reason.starts_with("no Lys owner"));
        } else if excluded.reason.starts_with("approvals_only") {
            // An entry holding only tool approvals starts nothing: it is excluded by its shape, and no server is made of it.
            let name = member.rsplit('.').next().ok_or("member has no name")?;
            assert!(servers.iter().all(|server| server["name"] != name));
        } else {
            assert!(excluded.reason.starts_with("harness_ui_state"));
        }
    }
    if given.harness == Harness::Codex && case["profile"].get("model").is_none() {
        assert_eq!(settings["model_access"], json!([]));
    }
    Ok(())
}

#[test]
fn seat_import_waffles_live_shape() -> TestResult {
    live_shape("waffles")
}
#[test]
fn seat_import_buckley_live_shape() -> TestResult {
    live_shape("buckley")
}
#[test]
fn seat_import_apollo_live_shape() -> TestResult {
    live_shape("apollo")
}
#[test]
fn seat_import_hermes_live_shape() -> TestResult {
    live_shape("hermes")
}
#[test]
fn seat_import_nimbus_live_shape() -> TestResult {
    live_shape("nimbus")
}
#[test]
fn seat_import_gypsy_live_shape() -> TestResult {
    live_shape("gypsy")
}

#[test]
fn seat_import_key_file_paths_keep_inline_credential_refusals() -> TestResult {
    for args in [
        json!(["--key-file", "/missing/private.key"]),
        json!(["--key-file=/missing/private.key"]),
    ] {
        let document = json!({"mcpServers":{"storage":{"command":"storage", "args":args}}});
        let sorted = claude_mcp("claude.mcp", &document);
        assert!(sorted.refusals.is_empty(), "{:?}", sorted.refusals);
        assert!(sorted.secrets.is_empty());
        assert_eq!(sorted.parts.mcp_servers.len(), 1);
        assert_eq!(
            serde_json::to_value(&sorted.parts.mcp_servers[0])?["command"]["args"],
            args
        );
    }
    for args in [
        json!(["--token", "fixture-credential"]),
        json!(["--token=fixture-credential"]),
        json!(["--key-file", "Bearer fixture-credential"]),
        json!(["--key-file=Bearer fixture-credential"]),
    ] {
        let document = json!({"mcpServers":{"storage":{"command":"storage", "args":args}}});
        let sorted = claude_mcp("claude.mcp", &document);
        assert!(
            sorted
                .refusals
                .iter()
                .any(|refusal| refusal.name == "import_credential_inline")
        );
        assert!(sorted.parts.mcp_servers.is_empty());
    }
    Ok(())
}

#[test]
fn seat_import_model_missing_differs_from_model_empty() {
    let sorted = codex_profile(
        "codex.profile",
        &json!({"mcp_servers":{"tools":{"command":"tools"}}}),
    );
    assert!(sorted.refusals.is_empty());
    assert!(sorted.parts.models.is_empty());
    let sorted = codex_profile("codex.profile", &json!({"model":""}));
    assert!(
        sorted
            .refusals
            .iter()
            .any(|refusal| refusal.member == "codex.profile#model")
    );
    assert!(sorted.parts.models.is_empty());
}

#[test]
fn seat_import_empty_sources_refuse_confirmation() {
    let facts = SeatFacts {
        seat: "seat".to_owned(),
        agent: "agent".to_owned(),
        responsible: Some("person".to_owned()),
    };
    let plan = assemble(
        facts,
        Harness::Claude,
        1,
        vec![Fragment::default()],
        &|_| Ok(0),
    );
    assert!(
        plan.refusals
            .iter()
            .any(|refusal| refusal.name == "import_source_missing")
    );
    assert_eq!(
        confirmable(&plan)
            .err()
            .map(|error| error.name())
            .as_deref(),
        Some("import_plan_refused")
    );
}

#[test]
fn seat_import_ui_and_environment_exclusions_keep_unknown_refusals() {
    let sorted = claude_settings(
        "claude.settings",
        &json!({
            "switchModelsOnFlag":false,"theme":"dark","voice":{"enabled":true},
            "env":{"CLAUDE_CODE_AUTOCOMPACT_PCT_OVERRIDE":"85"},"unknownSetting":true
        }),
        Replaced::default(),
    );
    assert_eq!(sorted.refusals.len(), 1);
    assert_eq!(sorted.refusals[0].member, "claude.settings#unknownSetting");
    assert_eq!(sorted.excluded.len(), 4);
}

#[test]
fn seat_import_excludes_a_tools_only_server_by_its_shape_whatever_its_name() {
    let sorted = codex_profile(
        "codex.profile",
        &json!({"mcp_servers":{
            "fixture_server":{"tools":{"look":{"approval_mode":"approve"}}},
            "other_fixture":{"tools":{"look":{"approval_mode":"approve"}},"colour":"blue"}
        }}),
    );
    assert!(sorted.excluded.iter().any(|(member, reason)| {
        member == "codex.profile#mcp_servers.fixture_server" && reason.starts_with("approvals_only")
    }));
    assert!(
        sorted
            .parts
            .mcp_servers
            .iter()
            .all(|server| server.name != "fixture_server")
    );
    assert!(sorted.refusals.iter().any(|refusal| {
        refusal.member == "codex.profile#mcp_servers.other_fixture"
            && refusal.name == "import_member_unsupported"
    }));
}
