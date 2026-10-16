#![cfg(test)]
use crate::launch_template::HandleName;
use crate::model_account_start::{routed, wire};
use crate::provisioning_store::Version;
use serde_json::json;
use std::collections::BTreeMap;
use std::error::Error;

#[test]
fn seat_program_accounts_share_native_provider_and_selector_mapping() -> Result<(), Box<dyn Error>>
{
    for (name, contract, provider, variable) in [
        (
            "openai-responses",
            "codex/template-v1",
            "https://api.openai.com",
            "CODEX_API_KEY",
        ),
        (
            "anthropic-messages",
            "claude-code/template-v1",
            "https://api.anthropic.com",
            "ANTHROPIC_AUTH_TOKEN",
        ),
    ] {
        let description = json!({"rendering_contract":"seat-program/v1",
            "models":{"minimum":0,"maximum":0,"further_encoding":{"kind":"array"}},
            "permissions":{"modes":[],"rule_forms":[]},
            "mcp":{"transports":[],"working_directory":false,"handle_variables":false,"channel_policies":[]},
            "seat_program":{"executable":"/opt/seat/host","executable_sha256":"a".repeat(64),
                "arguments":["{seat_file}"],"seat_file":"/opt/seat/seat.json",
                "seat_file_sha256":"b".repeat(64),"adapter":"acp/1","model_wire":name}});
        let mut profile: Version = serde_json::from_value(
            json!({"number":1,"operation":"fixture","set_by":"fixture","set_at":1,
            "settings":{"model_access":[],"tools":[],"skills":[],"mcp_servers":[],"instructions":"","note":"",
            "harness":{"name":"seat","program":"/opt/seat/host","package":"seat 1","description":description},
            "session":{"stall_after":300,"accounts":{"accounts":["first"],"variable":"LYS_HANDLE_ACCOUNT","limit":{"signal":"plan_window"}}}}}),
        )?;
        let rotation = profile
            .settings
            .session
            .as_ref()
            .and_then(|s| s.accounts.as_ref())
            .ok_or("rotation absent")?;
        assert_eq!(
            routed(
                &profile,
                rotation,
                &[HandleName {
                    id: "first".into(),
                    secret: "login".into(),
                    env: String::new()
                }],
                &json!({"secrets":[{"name":"login","class":"credential","header":"authorization","upstream":provider}]})
            )?,
            provider
        );
        let mut environment = BTreeMap::new();
        wire(&profile)?.bind(&mut environment, "first");
        assert_eq!(environment["LYS_ACCOUNT_VARIABLE"], variable);
        assert_eq!(environment[variable], "lys-account.first");
        if name == "anthropic-messages" {
            assert_eq!(environment["CLAUDE_CODE_OAUTH_TOKEN"], "");
            assert_eq!(environment["ANTHROPIC_API_KEY"], "");
        }
        profile
            .settings
            .harness
            .as_mut()
            .ok_or("harness absent")?
            .description
            .rendering_contract = contract.into();
        let mut native = BTreeMap::new();
        wire(&profile)?.bind(&mut native, "first");
        assert_eq!(environment, native);
    }
    Ok(())
}
