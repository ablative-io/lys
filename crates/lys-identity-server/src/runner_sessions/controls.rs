//! The exact reviewed launch config chooses its signed transport request.

use super::{AppState, Arc, Launch, ServerError, Value};
use lys_runner::Act;

/// Start `launch` for `agent` on `machine`'s runner and keep it running;
/// refuse when the machine names no runner. A start sent again finds the
/// session the runner already holds.
pub async fn run_on_runner(
    state: &Arc<AppState>,
    (agent, machine, caller): (&str, &str, &str),
    launch: Launch,
) -> Result<Value, ServerError> {
    super::run_resuming(state, (agent, machine, caller), launch, None).await
}

pub(super) fn select_transport(act: Act) -> Act {
    let Act::Start {
        launch,
        lys_mcp,
        proxy,
        ..
    } = act
    else {
        return act;
    };
    let requires_controls = launch
        .config
        .as_ref()
        .is_some_and(|config| config.requires_controls);
    if !requires_controls {
        let tracking = launch
            .config
            .as_ref()
            .and_then(|config| config.harness)
            .map(|harness| {
                let variable = match harness {
                    lys_runner::tracking::Harness::ClaudeCode => "CLAUDE_CONFIG_DIR",
                    lys_runner::tracking::Harness::Codex => "CODEX_HOME",
                };
                lys_runner::protocol::TrackingSpec {
                    harness,
                    config_home: launch
                        .environment
                        .get(variable)
                        .map(std::path::PathBuf::from),
                }
            });
        return Act::Start {
            launch,
            lys_mcp,
            proxy,
            tracking,
        };
    }
    let transport = match launch.config.as_ref().and_then(|config| config.harness) {
        Some(lys_runner::tracking::Harness::ClaudeCode) => {
            lys_runner::harness_control::Transport::Claude
        }
        Some(lys_runner::tracking::Harness::Codex) => lys_runner::harness_control::Transport::Codex,
        None => lys_runner::harness_control::Transport::Pty,
    };
    let conversation = if transport == lys_runner::harness_control::Transport::Claude {
        lys_runner::harness_control::Pending::new(
            launch.session.clone(),
            lys_runner::harness_control::Kind::Human,
            String::new(),
        )
        .uuid
    } else {
        String::new()
    };
    Act::StartManaged {
        managed: Box::new(lys_runner::harness_control::ManagedLaunch {
            launch: *launch,
            transport,
            conversation,
            requires_controls,
            resume: false,
            owner: None,
        }),
        lys_mcp,
        proxy,
    }
}

pub(super) fn select_resume(
    act: Act,
    conversation: Option<&str>,
) -> Result<Act, crate::error::ServerError> {
    let mut act = select_transport(act);
    if let Some(conversation) = conversation {
        let Act::StartManaged { managed, .. } = &mut act else {
            return Err(lys_runner::RunnerError::refused(
                "control_binding_invalid",
                "a resumed conversation requires a managed harness",
            )
            .into());
        };
        managed.transport.validate_resume(conversation)?;
        conversation.clone_into(&mut managed.conversation);
        managed.resume = true;
    }
    Ok(act)
}

#[cfg(test)]
mod tests {
    use super::select_transport;
    use lys_runner::{Act, Launch};
    use serde_json::json;

    #[test]
    fn controls_pty_tracking_uses_the_rendered_harness_and_config_home()
    -> Result<(), Box<dyn std::error::Error>> {
        for (harness, environment, expected) in [
            (
                json!("codex"),
                json!({"CODEX_HOME":"/codex-home", "CLAUDE_CONFIG_DIR":"/other"}),
                json!({"harness":"codex", "config_home":"/codex-home"}),
            ),
            (
                json!("claude_code"),
                json!({}),
                json!({"harness":"claude_code", "config_home":null}),
            ),
            (
                serde_json::Value::Null,
                json!({"CODEX_HOME":"/other"}),
                serde_json::Value::Null,
            ),
        ] {
            let act: Act = serde_json::from_value(json!({
                "act":"start", "launch":{
                    "session":"selected", "program":"/harness", "arguments":[], "directory":"/",
                    "environment":environment, "columns":80, "rows":24,
                    "config":{"files":[], "argument_files":{}, "environment_paths":{},
                        "working_directory":false, "harness":harness, "requires_controls":false}
                }
            }))?;
            let selected = serde_json::to_value(select_transport(act))?;
            assert_eq!(selected["tracking"], expected, "{selected}");
        }
        Ok(())
    }

    #[test]
    fn only_the_exact_reviewed_config_selects_the_managed_transport()
    -> Result<(), Box<dyn std::error::Error>> {
        for required in [false, true] {
            let launch: Launch = serde_json::from_value(json!({
                "session":"selected","program":"/harness","arguments":[],"directory":"/",
                "environment":{},"columns":80,"rows":24,"rotation":null,"policy":null,
                "config":{"files":[],"argument_files":{},"environment_paths":{},"working_directory":false,
                    "harness":"claude_code","requires_controls":required}
            }))?;
            let selected = select_transport(Act::Start {
                tracking: None,
                launch: Box::new(launch),
                lys_mcp: None,
                proxy: None,
            });
            assert_eq!(matches!(selected, Act::StartManaged { .. }), required);
            if let Act::StartManaged { managed, .. } = selected {
                assert!(managed.requires_controls);
            }
        }
        Ok(())
    }
    #[test]
    fn an_existing_harness_conversation_is_forwarded_without_creating_another()
    -> Result<(), Box<dyn std::error::Error>> {
        let conversation = "019c6e27-e55b-73d1-87d8-4e01f1f75043";
        for harness in ["claude_code", "codex"] {
            let launch: Launch = serde_json::from_value(json!({
                "session":"selected","program":"/harness","arguments":[],"directory":"/",
                "environment":{},"columns":80,"rows":24,"rotation":null,"policy":null,
                "config":{"files":[],"argument_files":{},"environment_paths":{},"working_directory":false,
                    "harness":harness,"requires_controls":true}
            }))?;
            let act = Act::Start {
                tracking: None,
                launch: Box::new(launch),
                lys_mcp: None,
                proxy: None,
            };
            for invalid in ["", "../thread", "not-a-uuid"] {
                assert!(super::select_resume(act.clone(), Some(invalid)).is_err());
            }
            let Act::StartManaged { managed, .. } = super::select_resume(act, Some(conversation))?
            else {
                return Err("resume did not select managed transport".into());
            };
            assert_eq!(managed.conversation, conversation);
            assert!(managed.resume);
        }
        Ok(())
    }
}
