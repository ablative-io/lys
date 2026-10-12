//! The exact reviewed launch config chooses its signed transport request.

use lys_runner::Act;

pub(super) fn select_transport(act: Act) -> Act {
    let Act::Start {
        launch,
        lys_mcp,
        proxy,
    } = act
    else {
        return act;
    };
    let requires_controls = launch
        .config
        .as_ref()
        .is_some_and(|config| config.requires_controls);
    if !requires_controls {
        return Act::Start {
            launch,
            lys_mcp,
            proxy,
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

pub(super) fn select_resume(act: Act, conversation: Option<&str>) -> Result<Act, crate::error::ServerError> {
    let mut act = select_transport(act);
    if let Some(conversation) = conversation {
        let Act::StartManaged { managed, .. } = &mut act else {
            return Err(lys_runner::RunnerError::refused("control_binding_invalid", "a resumed conversation requires a managed harness").into());
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
            let act = Act::Start { launch:Box::new(launch), lys_mcp:None, proxy:None };
            for invalid in ["", "../thread", "not-a-uuid"] {
                assert!(super::select_resume(act.clone(), Some(invalid)).is_err());
            }
            let Act::StartManaged { managed, .. } = super::select_resume(act, Some(conversation))? else {
                return Err("resume did not select managed transport".into());
            };
            assert_eq!(managed.conversation, conversation);
            assert!(managed.resume);
        }
        Ok(())
    }

}
