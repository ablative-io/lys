#![cfg(test)]
use crate::launch_template::Start;
use crate::provisioning_store::Version;
use serde_json::json;

#[test]
fn claude_and_codex_refuse_before_rendering_when_the_server_model_proxy_is_unset()
-> Result<(), Box<dyn std::error::Error>> {
    for contract in ["claude-code/template-v1", "codex/template-v1"] {
        let mut harness = crate::harness_description::declared();
        harness["description"]["rendering_contract"] = json!(contract);
        let version: Version = serde_json::from_value(json!({
            "number": 1, "operation": "profile", "settings": {
                "model_access": ["model"], "tools": [], "skills": [], "mcp_servers": [],
                "permissions": {"default_mode": "plan"}, "instructions": "", "note": "",
                "harness": harness
            }, "set_by": "person", "set_at": 1,
            "reviewed": {"operation": "review", "by": "person", "at": 1}
        }))?;
        let start = Start {
            agent: "agent",
            session: "session",
            machine: "machine",
            runtime: "sh",
            version: &version,
            skills: &[],
            policy: None,
            model_proxy: None,
        };
        let error = crate::start_checks::fields(&start, &[])
            .err()
            .ok_or("an unproxied model launch was rendered")?;
        assert_eq!(error.name(), "model_proxy_unset");
        assert!(error.to_string().contains("model proxy"));
    }
    Ok(())
}
