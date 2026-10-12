//! The profile's own context and Codex proxy survive the start checks.

use std::error::Error;

use lys_identity_server::launch_template::Start;
use lys_identity_server::provisioning_store::Version;
use lys_identity_server::start_checks::fields;
use serde_json::{Value, json};

#[test]
fn start_checks_codex_keeps_the_proxy_and_mints_a_run() -> Result<(), Box<dyn Error>> {
    let catalogue: Value =
        serde_json::from_str(include_str!("../../../docs/harness/catalogue/codex.json"))?;
    let version: Version = serde_json::from_value(json!({"number":1, "operation":"fixture",
        "set_by":"fixture", "set_at":1, "settings": {
            "model_access":["model-a"], "tools":[], "skills":[], "mcp_servers":[],
            "instructions":"", "note":"", "harness": {"name":"fixture",
                "program":"/opt/fixture/codex", "package":"fixture",
                "description":catalogue["description"]}}}))?;
    let view = fields(
        &Start {
            agent: "agent",
            session: "session",
            machine: "machine",
            runtime: "/opt/fixture/lys",
            version: &version,
            skills: &[],
            policy: None,
            model_proxy: Some("http://127.0.0.1:18484/anthropic"),
        },
        &[],
    )?;
    assert_eq!(
        view.model_proxy.as_deref(),
        Some("http://127.0.0.1:18484/anthropic")
    );
    assert!(
        view.run
            .as_deref()
            .is_some_and(lys_home::proxy::usage::is_run_key)
    );
    Ok(())
}

#[test]
fn start_checks_carries_every_reviewed_seat_field() -> Result<(), Box<dyn Error>> {
    let catalogue: Value = serde_json::from_str(include_str!(
        "../../../docs/harness/catalogue/claude-code.json"
    ))?;
    let seat = json!({"hooks":{"Stop":[{"command":"seat-stop"}]},
        "status_line":"seat-status", "settings":{"theme":"dark"},
        "resume":"019c6e27-e55b-73d1-87d8-4e01f1f75043",
        "context_window":200_000, "config_dir":"/tmp/chosen-config"});
    let version: Version = serde_json::from_value(json!({"number":1, "operation":"fixture",
    "set_by":"fixture", "set_at":1, "settings":{
        "model_access":["model-a"], "tools":[], "skills":[], "mcp_servers":[],
        "instructions":"", "note":"", "hooks":seat["hooks"],
        "status_line":seat["status_line"], "settings":seat["settings"],
        "session":{"resume":seat["resume"], "context_window":seat["context_window"], "config_dir":seat["config_dir"]},
        "harness":{"name":"fixture", "program":"/opt/fixture/claude", "package":"fixture", "description":catalogue["description"]}
    }}))?;
    let view = fields(
        &Start {
            agent: "agent",
            session: "session",
            machine: "machine",
            runtime: "/opt/fixture/lys",
            version: &version,
            skills: &[],
            policy: None,
            model_proxy: None,
        },
        &[],
    )?;
    assert_eq!(serde_json::to_value(view.seat)?, seat);
    Ok(())
}

#[test]
fn start_checks_carries_profile_level_seat_fields_and_refuses_conflicting_session_fields()
-> Result<(), Box<dyn Error>> {
    let catalogue: Value = serde_json::from_str(include_str!(
        "../../../docs/harness/catalogue/claude-code.json"
    ))?;
    let seat = json!({"hooks":{"Stop":[{"command":"seat-stop"}]},
        "status_line":"seat-status", "settings":{"theme":"dark"},
        "resume":"019c6e27-e55b-73d1-87d8-4e01f1f75043",
        "context_window":200_000, "config_dir":"/tmp/chosen-config"});
    let mut settings = seat.as_object().ok_or("seat is not an object")?.clone();
    settings.extend(json!({"model_access":["model-a"], "tools":[], "skills":[], "mcp_servers":[],
        "instructions":"", "note":"", "harness":{"name":"fixture",
        "program":"/opt/fixture/claude", "package":"fixture", "description":catalogue["description"]}})
        .as_object().ok_or("settings are not an object")?.clone());
    let mut version: Version = serde_json::from_value(json!({"number":1, "operation":"fixture",
        "set_by":"fixture", "set_at":1, "settings":settings}))?;
    for conflicting in [false, true] {
        if conflicting {
            version.settings.session =
                Some(serde_json::from_value(json!({"context_window":100_000}))?);
        }
        let answer = fields(
            &Start {
                agent: "agent",
                session: "session",
                machine: "machine",
                runtime: "/opt/fixture/lys",
                version: &version,
                skills: &[],
                policy: None,
                model_proxy: None,
            },
            &[],
        );
        if conflicting {
            let error = answer.expect_err("conflicting windows accepted");
            assert!(
                error.to_string().contains("context_window")
                    && error.to_string().contains("conflict"),
                "{error}"
            );
        } else {
            assert_eq!(serde_json::to_value(answer?.seat)?, seat);
        }
    }
    Ok(())
}
