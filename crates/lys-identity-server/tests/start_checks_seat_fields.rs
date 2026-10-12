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
