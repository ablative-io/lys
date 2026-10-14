use serde_json::json;

use super::{arguments, ready, started, valid_command};
use crate::commands::launch_catalog::Agent;

type Result = std::result::Result<(), Box<dyn std::error::Error>>;

fn agent() -> Agent {
    Agent {
        id: "agent-a".to_owned(),
        name: "Mine".to_owned(),
        state: "active".to_owned(),
        harness: Some("claude".to_owned()),
        program: Some("claude".to_owned()),
        profile_version: Some(1),
        reviewed: true,
        controlled: false,
        runs_on: Some("machine-a".to_owned()),
        writable: None,
        working_folder: Some("/work".to_owned()),
        sessions: Vec::new(),
        seats: Vec::new(),
        unknown: Vec::new(),
    }
}

fn reply() -> serde_json::Value {
    let mut reply = json!({"agent":"agent-a","machine":"machine-a","session":"op-start","provisioning_version":1,
        "runner":{"session":"op-start","state":"running"}});
    template(
        &mut reply,
        json!({"harness":"claude-code","slots":{"env":{"ANTHROPIC_BASE_URL":"https://proxy.example/run","LYS_RUN":"0123456789abcdef0123456789abcdef"}}}),
    );
    reply
}

fn template(reply: &mut serde_json::Value, native: serde_json::Value) {
    let text = native.to_string();
    reply["harness"] = native["harness"].clone();
    reply["template_sha256"] = json!(lys_home::record::blocks::Hash::of(text.as_bytes()).as_str());
    reply["template"] = json!(text);
}

#[test]
fn harness_arguments_refuse_before_start_until_the_server_has_a_wire_field() -> Result {
    arguments(&[])?;
    let error = arguments(&["--model".to_owned(), "two words".to_owned()])
        .err()
        .ok_or("arguments discarded")?
        .to_string();
    assert!(error.contains("launch_arguments_unavailable"));
    assert!(error.contains("no session was started"));
    Ok(())
}

#[test]
fn reviewed_terminal_profile_on_this_machine_is_ready() -> Result {
    ready(&agent())?;
    Ok(())
}

#[test]
fn launch_preserves_controls_and_uses_the_managed_seats_starting_folder() -> Result {
    let mut agent = agent();
    agent.controlled = true;
    agent.working_folder = None;
    ready(&agent)?;
    assert!(agent.controlled);
    Ok(())
}

#[test]
fn inactive_unreviewed_and_missing_folder_terminal_profiles_refuse() -> Result {
    for (reason, change) in [
        ("AgentNotActive", 0),
        ("ProfileNotReviewed", 1),
        ("WorkingFolderUnnamed", 2),
    ] {
        let mut agent = agent();
        match change {
            0 => agent.state = "suspended".to_owned(),
            1 => agent.reviewed = false,
            _ => agent.working_folder = None,
        }
        let error = ready(&agent).err().ok_or("ineligible profile accepted")?;
        assert!(error.to_string().contains(reason), "{error}");
    }
    Ok(())
}

#[test]
fn start_acknowledgement_requires_the_requested_agent_machine_session_and_profile() -> Result {
    started(reply(), &agent(), "machine-a", "op-start")?;
    for (field, value) in [
        ("agent", json!("another")),
        ("machine", json!("another")),
        ("session", json!("another")),
        ("provisioning_version", json!(2)),
    ] {
        let mut reply = reply();
        reply[field] = value;
        assert!(
            started(reply, &agent(), "machine-a", "op-start").is_err(),
            "{field}"
        );
    }
    let mut reply = reply();
    reply["runner"]["session"] = json!("another");
    assert!(started(reply, &agent(), "machine-a", "op-start").is_err());
    Ok(())
}

#[test]
fn launch_refuses_a_missing_proxy_instead_of_reporting_direct_provider_use_as_success() -> Result {
    let mut reply = reply();
    template(
        &mut reply,
        json!({"harness":"claude-code","slots":{"env":{"LYS_RUN":"0123456789abcdef0123456789abcdef"}}}),
    );
    let error = started(reply, &agent(), "machine-a", "op-start")
        .err()
        .ok_or("missing proxy accepted")?;
    assert!(error.to_string().contains("model_proxy_missing"));
    Ok(())
}

#[test]
fn codex_native_template_preserves_the_servers_proxy_base() -> Result {
    let mut reply = reply();
    template(
        &mut reply,
        json!({"harness":"codex","fields":{"model_proxy":"https://proxy.example/run","run":"0123456789abcdef0123456789abcdef"}}),
    );
    let mut agent = agent();
    agent.harness = Some("codex".to_owned());
    started(reply, &agent, "machine-a", "op-start")?;
    Ok(())
}

#[test]
fn an_explicit_machine_does_not_treat_the_reviewed_default_as_exclusive_permission() -> Result {
    let mut agent = agent();
    agent.runs_on = Some("another-machine".to_owned());
    ready(&agent)?;
    Ok(())
}

#[test]
fn template_hash_harness_and_per_run_identity_are_required() -> Result {
    for (field, value) in [
        ("template_sha256", json!("wrong")),
        ("harness", json!("codex")),
    ] {
        let mut reply = reply();
        reply[field] = value;
        assert!(started(reply, &agent(), "machine-a", "op-start").is_err());
    }
    let mut reply = reply();
    template(
        &mut reply,
        json!({"harness":"claude-code","slots":{"env":{"ANTHROPIC_BASE_URL":"https://proxy.example/run"}}}),
    );
    let error = started(reply, &agent(), "machine-a", "op-start")
        .err()
        .ok_or("run identity absent but accepted")?;
    assert!(error.to_string().contains("launch_run_missing"));
    Ok(())
}

#[test]
fn an_unused_other_harness_proxy_field_cannot_replace_the_native_proxy() -> Result {
    let mut reply = reply();
    template(
        &mut reply,
        json!({"harness":"claude-code","slots":{"env":{"LYS_RUN":"0123456789abcdef0123456789abcdef"}},"fields":{"model_proxy":"https://unused.example"}}),
    );
    let error = started(reply, &agent(), "machine-a", "op-start")
        .err()
        .ok_or("decoy proxy accepted")?;
    assert!(error.to_string().contains("model_proxy_missing"));
    Ok(())
}

#[test]
fn malformed_start_diagnostics_never_echo_response_credentials() -> Result {
    let error = started(
        json!({"agent":"private-response"}),
        &agent(),
        "machine-a",
        "op-start",
    )
    .err()
    .ok_or("malformed start accepted")?
    .to_string();
    assert!(error.contains("launch_answer_unreadable"));
    assert!(!error.contains("private-response"));
    Ok(())
}

#[test]
fn invalid_command_boundaries_refuse_without_echoing_control_bytes_or_payloads() -> Result {
    for command in ["", "../program", "private\x1b[2J", "claude; other-command"] {
        let error = valid_command(command)
            .err()
            .ok_or("invalid command accepted")?
            .to_string();
        assert!(error.contains("launch_command_invalid"));
        assert!(!error.contains("private"));
        assert!(!error.contains('\x1b'));
    }
    valid_command("claude")?;
    valid_command("/opt/fork with spaces")?;
    Ok(())
}
