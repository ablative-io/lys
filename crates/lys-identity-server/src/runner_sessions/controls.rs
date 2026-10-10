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
            owner: None,
        }),
        lys_mcp,
        proxy,
    }
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
}
