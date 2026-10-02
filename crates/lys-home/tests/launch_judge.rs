#![cfg(test)]
//! The launch's settings file installs the runner's judge as the session's
//! `PreToolUse` hook when a judge is named, and is unchanged without one.

use std::error::Error;
use std::path::Path;

use serde_json::{Value, json};

use lys_home::HomeError;
use lys_home::harness::claude_code::launch_env::{Judge, env_settings, settings};
use lys_home::harness::claude_code::template::parse_template;

const TEMPLATE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/launch/template.json"
);

type Fallible = Result<(), Box<dyn Error>>;

#[test]
fn a_named_judge_is_the_hook_for_every_tool_beside_the_same_env() -> Fallible {
    let template = parse_template(&std::fs::read(TEMPLATE)?)?;
    let judge = Judge::new(
        Path::new("/opt/lys/bin/lys"),
        Path::new("/run/lys/runner.sock"),
    )?;
    let value: Value = serde_json::from_slice(&settings(&template, Some(&judge))?)?;
    let plain: Value = serde_json::from_slice(&env_settings(&template)?)?;
    assert_eq!(value["env"], plain["env"]);
    assert_eq!(value["permissions"], plain["permissions"]);
    assert_eq!(value["permissions"], json!({"defaultMode": "plan"}));
    assert_eq!(
        value["hooks"],
        json!({"PreToolUse": [{"matcher": "*", "hooks": [{
            "type": "command",
            "command": "'/opt/lys/bin/lys' runner judge --harness claude --socket '/run/lys/runner.sock'"
        }]}]})
    );
    assert_eq!(value.as_object().ok_or("not an object")?.len(), 3);
    Ok(())
}

#[test]
fn without_a_judge_the_settings_are_the_environment_file_byte_for_byte() -> Fallible {
    let template = parse_template(&std::fs::read(TEMPLATE)?)?;
    assert_eq!(settings(&template, None)?, env_settings(&template)?);
    Ok(())
}

#[test]
fn a_judge_path_that_is_relative_or_quoted_is_refused_by_its_flag() {
    for (program, socket, flag) in [
        ("lys", "/run/lys/runner.sock", "--judge-program"),
        ("/opt/lys/bin/lys", "runner.sock", "--judge-socket"),
        ("/opt/it's/lys", "/run/lys/runner.sock", "--judge-program"),
        (
            "/opt/lys/bin/lys",
            "/run/lys/run\nner.sock",
            "--judge-socket",
        ),
    ] {
        match Judge::new(Path::new(program), Path::new(socket)) {
            Err(HomeError::JudgePath { flag: named, .. }) => assert_eq!(named, flag),
            other => panic!("{program} and {socket} were not refused by path: {other:?}"),
        }
    }
}
