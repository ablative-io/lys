#![cfg(test)]
//! The runner checks renderer fixtures through its existing home dependency.

use std::collections::BTreeMap;
use std::error::Error;

use lys_home::harness::launch_fields::{InstructionsMode, ProfileLaunchFields};
use lys_home::harness::rendering;
use lys_home::harness::rendering_launch;
use lys_home::harness::skills::{SkillFile, check};
use lys_home::record::blocks::Hash;
use serde_json::{Value, json};

use crate::launch_config::{Config, checked};
use crate::tracking::Harness;

#[test]
fn launch_config_rendered_claude_and_codex_skill_roots_are_admitted() -> Result<(), Box<dyn Error>>
{
    let skill = SkillFile {
        name: "fixture-skill".to_owned(),
        text: "# Skill\n".to_owned(),
        sha256: Hash::of(b"# Skill\n").as_str().to_owned(),
    };
    for codex in [false, true] {
        let catalogue: Value = serde_json::from_str(if codex {
            include_str!("../../../../docs/harness/catalogue/codex.json")
        } else {
            include_str!("../../../../docs/harness/catalogue/claude-code.json")
        })?;
        let mut value = json!({
            "identity":{"agent":"agent-fixture","session":"session-fixture","machine":"machine-fixture","version":1},
            "harness":{"name":"fixture","description":catalogue["description"],"program":"/opt/fixture/harness","package":"fixture"},
            "instructions":"Reviewed instructions.\n",
            "models":["fixture-model"],"mcp_servers":[],
            "skills":[check(&skill)?],
            "resume":"019c6e27-e55b-73d1-87d8-4e01f1f75043",
            "context_window":200_000,
            "config_dir":"/opt/fixture/seat-config",
            "settings":{"theme":"seat","nested":{"words":["keep","verbatim"]}},
            "model_proxy":"http://127.0.0.1:9000/anthropic",
            "run":"0123456789abcdef0123456789abcdef"
        });
        if !codex {
            value["plugins"] = json!(["fixture@market"]);
            value["hooks"] = json!({"PreToolUse":[{"matcher":"Read","command":"seat-before","timeout":9}],"Stop":[{"command":"seat-stop"}]});
            value["status_line"] = json!("test -f \"$LYS_RUNNER_SOCKET\" && cat");
        }
        let fields: ProfileLaunchFields = serde_json::from_value(value)?;
        let rendered = rendering::render(
            &fields,
            std::slice::from_ref(&skill),
            &json!({"allow":[],"deny":[],"ask":[],"hard_rules":[],"additional_directories":[],"default_mode":if codex {"read-only"} else {"plan"}}),
            &[],
        )?;
        let launch = rendering_launch::render(
            if codex {
                "codex/template-v1"
            } else {
                "claude-code/template-v1"
            },
            &fields.harness.program,
            &rendered.text,
            InstructionsMode::Append,
        )?;
        if codex {
            assert_eq!(
                launch.arguments.join("\n") + "\n",
                include_str!("../fixtures/codex-argv-full.txt")
            );
        } else {
            assert_eq!(
                launch
                    .files
                    .iter()
                    .find(|file| file.path == "settings.json")
                    .ok_or("settings missing")?
                    .text,
                include_str!("../fixtures/claude-settings-full.json")
            );
        }
        assert!(launch.argument_files.iter().any(|(index, path)| {
            path.is_empty() && launch.arguments[*index - 1] == "--add-dir"
        }));
        let config = Config {
            requires_controls: false,
            files: launch.files,
            argument_files: launch.argument_files,
            environment_paths: launch.environment_paths,
            working_directory: false,
            harness: Some(if codex {
                Harness::Codex
            } else {
                Harness::ClaudeCode
            }),
        };
        checked(&config, launch.arguments.len())?;
        let mut foreign = config.clone();
        foreign.argument_files = BTreeMap::from([(0, "foreign.json".to_owned())]);
        assert_eq!(
            checked(&foreign, launch.arguments.len())
                .expect_err("foreign file admitted")
                .name(),
            "launch_config_refused"
        );
    }
    Ok(())
}
