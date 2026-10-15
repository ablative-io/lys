#![cfg(test)]
//! Runner launches retain the reviewed seat fields and native integration.
use crate as lys_home;
use std::error::Error;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::process::{Command, Stdio};

use lys_home::harness::launch_fields::{InstructionsMode, ProfileLaunchFields};
use lys_home::harness::rendering::{self, SecretBinding};
use lys_home::harness::rendering_launch::{self, Launch};
use lys_home::harness::skills::{SkillFile, check};
use lys_home::record::blocks::Hash;
use serde_json::{Value, json};

type ResultTest = Result<(), Box<dyn Error>>;
type RenderingCase = (&'static str, fn() -> ResultTest);
const RESUME: &str = "019c6e27-e55b-73d1-87d8-4e01f1f75043";
const HOOK: &str = "\"$LYS_PROGRAM\" runner hook --socket \"$LYS_RUNNER_SOCKET\"";

fn launch(codex: bool, full: bool) -> Result<Launch, Box<dyn Error>> {
    launch_inputs(codex, full, None)
}

fn launch_inputs(
    codex: bool,
    full: bool,
    runner: Option<(&str, &str)>,
) -> Result<Launch, Box<dyn Error>> {
    let catalogue: Value = serde_json::from_str(if codex {
        include_str!("../../../../docs/harness/catalogue/codex.json")
    } else {
        include_str!("../../../../docs/harness/catalogue/claude-code.json")
    })?;
    let mut value = json!({
        "identity":{"agent":"agent-fixture","session":"session-fixture","machine":"machine-fixture","version":1},
        "harness":{"name":"fixture","description":catalogue["description"],"program":"/opt/fixture/harness","package":"fixture"},
        "instructions":"", "models":["fixture-model"],"mcp_servers":[],"skills":[]
    });
    let mut skills = Vec::new();
    if full {
        value["resume"] = json!(RESUME);
        value["context_window"] = json!(200_000);
        value["config_dir"] = json!("/opt/fixture/seat-config");
        value["settings"] = json!({"theme":"seat","nested":{"words":["keep","verbatim"]}});
        value["model_proxy"] = json!("http://127.0.0.1:9000/anthropic");
        value["run"] = json!("0123456789abcdef0123456789abcdef");
        value["instructions"] = json!("Reviewed instructions.\n");
        let skill = SkillFile {
            name: "fixture-skill".to_owned(),
            text: "# Skill\n".to_owned(),
            sha256: Hash::of(b"# Skill\n").as_str().to_owned(),
        };
        value["skills"] = json!([check(&skill)?]);
        skills.push(skill);
        if !codex {
            value["plugins"] = json!(["fixture@market"]);
            value["hooks"] = json!({"PreToolUse":[{"matcher":"Read","command":"seat-before","timeout":9}],"Stop":[{"command":"seat-stop"}]});
            value["status_line"] = json!("test -f \"$LYS_RUNNER_SOCKET\" && cat");
        }
    }
    let fields: ProfileLaunchFields = serde_json::from_value(value)?;
    let rendered = rendering::render(
        &fields,
        &skills,
        &json!({"allow":[],"deny":[],"ask":[],"hard_rules":[],"additional_directories":[],"default_mode":if codex {"read-only"} else {"plan"}}),
        &[] as &[SecretBinding],
    )?;
    let text = if let Some((program, socket)) = runner {
        let mut template: Value = serde_json::from_str(&rendered.text)?;
        let environment = if codex {
            &mut template["environment"]
        } else {
            &mut template["slots"]["env"]
        };
        environment["LYS_PROGRAM"] = json!(program);
        environment["LYS_RUNNER_SOCKET"] = json!(socket);
        serde_json::to_string(&template)?
    } else {
        rendered.text
    };
    Ok(rendering_launch::render(
        if codex {
            "codex/template-v1"
        } else {
            "claude-code/template-v1"
        },
        &fields.harness.program,
        &text,
        InstructionsMode::Append,
    )?)
}

fn settings(launch: &Launch) -> Result<Value, Box<dyn Error>> {
    Ok(serde_json::from_str(
        &launch
            .files
            .iter()
            .find(|file| file.path == "settings.json")
            .ok_or("settings missing")?
            .text,
    )?)
}

#[test]
fn claude_runner_hooks_and_judge_are_installed() -> ResultTest {
    let value = settings(&launch(false, false)?)?;
    for event in [
        "SessionStart",
        "UserPromptSubmit",
        "Stop",
        "SessionEnd",
        "PreCompact",
        "PostCompact",
        "PostToolUse",
        "PostToolUseFailure",
    ] {
        assert_eq!(
            value["hooks"][event][0]["hooks"][0],
            json!({"type":"command","command":HOOK}),
            "{event}"
        );
        if [
            "SessionStart",
            "SessionEnd",
            "PreCompact",
            "PostCompact",
            "PostToolUse",
            "PostToolUseFailure",
        ]
        .contains(&event)
        {
            assert_eq!(value["hooks"][event][0]["matcher"], "*");
        } else {
            assert!(value["hooks"][event][0].get("matcher").is_none());
        }
    }
    assert_eq!(
        value["hooks"]["PreToolUse"][0]["hooks"][0]["command"],
        "\"$LYS_PROGRAM\" runner judge --harness claude --socket \"$LYS_RUNNER_SOCKET\""
    );
    Ok(())
}

#[test]
fn claude_seat_hooks_and_settings_follow_lys() -> ResultTest {
    let value = settings(&launch(false, true)?)?;
    assert_eq!(
        value["hooks"]["Stop"][1]["hooks"][0]["command"],
        "seat-stop"
    );
    assert_eq!(
        value["hooks"]["PreToolUse"][1],
        json!({"matcher":"Read","hooks":[{"type":"command","command":"seat-before","timeout":9}]})
    );
    assert_eq!(value["theme"], "seat");
    assert_eq!(value["nested"], json!({"words":["keep","verbatim"]}));
    assert_eq!(value["enabledPlugins"], json!({"fixture@market":true}));
    Ok(())
}

#[test]
fn forced_claude_settings_replace_seat_values() -> ResultTest {
    use lys_home::harness::claude_code::launch_env::env_settings;
    use lys_home::harness::claude_code::template::parse_template;
    let mut template = parse_template(include_bytes!("../fixtures/launch/template.json"))?;
    let plain: Value = serde_json::from_slice(&env_settings(&template)?)?;
    template.seat.settings = Some(serde_json::from_value(json!({
        "env":{"FOREIGN":"seat"}, "statusLine":{"command":"foreign"},
        "permissions":{"defaultMode":"bypassPermissions"}, "hooks":{"Stop":[]},
        "enabledPlugins":{"foreign@market":true}, "theme":"kept"
    }))?);
    let merged: Value = serde_json::from_slice(&env_settings(&template)?)?;
    for key in [
        "env",
        "statusLine",
        "permissions",
        "hooks",
        "enabledPlugins",
    ] {
        assert_eq!(merged.get(key), plain.get(key), "{key}");
    }
    assert_eq!(merged["theme"], "kept");
    Ok(())
}

#[test]
fn both_harnesses_render_config_resume_and_skill_directory() -> ResultTest {
    for codex in [false, true] {
        let full = launch(codex, true)?;
        let variable = if codex {
            "CODEX_HOME"
        } else {
            "CLAUDE_CONFIG_DIR"
        };
        assert_eq!(
            full.environment.get(variable).map(String::as_str),
            Some("/opt/fixture/seat-config")
        );
        assert!(
            full.arguments
                .windows(2)
                .any(|pair| pair == [if codex { "resume" } else { "--resume" }, RESUME])
        );
        assert!(
            full.arguments
                .windows(2)
                .any(|pair| pair == ["--add-dir", "/opt/fixture/seat-config"])
        );
        assert!(
            full.argument_files.iter().any(|(index, path)| {
                path.is_empty() && full.arguments[*index - 1] == "--add-dir"
            }),
            "kept skills need the generated config root"
        );
        assert!(!launch(codex, false)?.environment.contains_key(variable));
    }
    Ok(())
}

#[test]
fn codex_notify_and_proxy_are_rendered() -> ResultTest {
    let launch = launch(true, true)?;
    assert!(launch.arguments.iter().any(|arg|arg=="notify=[\"$LYS_PROGRAM\", \"runner\", \"notify\", \"--socket\", \"$LYS_RUNNER_SOCKET\"]"));
    assert!(
        launch
            .arguments
            .iter()
            .any(|arg| arg == "model_provider=\"lys\"")
    );
    assert!(launch.arguments.iter().any(|arg| arg == "theme=\"seat\""));
    Ok(())
}

#[test]
fn named_runner_paths_are_literal_notify_and_judge_inputs() -> ResultTest {
    let runner = Some(("/opt/fixture/lys", "/run/fixture/runner.sock"));
    let codex = launch_inputs(true, false, runner)?;
    assert!(codex.arguments.iter().any(|arg| arg == "notify=[\"/opt/fixture/lys\", \"runner\", \"notify\", \"--socket\", \"/run/fixture/runner.sock\"]"));
    let claude = settings(&launch_inputs(false, false, runner)?)?;
    assert_eq!(
        claude["hooks"]["PreToolUse"][0]["hooks"][0]["command"],
        "'/opt/fixture/lys' runner judge --harness claude --socket '/run/fixture/runner.sock'"
    );
    Ok(())
}

#[test]
fn seat_status_runs_after_lys_with_the_same_input() -> ResultTest {
    let launch = launch(false, true)?;
    let mut value = settings(&launch)?;
    let command = value["statusLine"]["command"]
        .take()
        .as_str()
        .ok_or("command missing")?
        .to_owned();
    let dir = tempfile::tempdir()?;
    let program = dir.path().join("lys");
    std::fs::write(
        &program,
        "#!/bin/sh\ncat >/dev/null\n: > \"$4\"\nprintf lys\n",
    )?;
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700))?;
    let mut child = Command::new("sh")
        .args(["-c", &command])
        .env("LYS_PROGRAM", program)
        .env("LYS_RUNNER_SOCKET", dir.path().join("lys-ran"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .take()
        .ok_or("stdin missing")?
        .write_all(b"{\"words\":\"input\"}")?;
    let output = child.wait_with_output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"lys | {\"words\":\"input\"}\n");
    Ok(())
}

fn compare(actual: &str, expected: &str) {
    if actual.as_bytes() != expected.as_bytes() {
        let offset = expected
            .as_bytes()
            .iter()
            .zip(actual.as_bytes())
            .position(|(left, right)| left != right)
            .unwrap_or(expected.len().min(actual.len()));
        eprintln!(
            "--- fixture\n+++ rendered\n@@ byte {offset} @@\n- {:?}\n+ {:?}",
            expected.as_bytes().get(offset),
            actual.as_bytes().get(offset)
        );
        let left: Vec<_> = expected.lines().collect();
        let right: Vec<_> = actual.lines().collect();
        for i in 0..left.len().max(right.len()) {
            if left.get(i) != right.get(i) {
                eprintln!(
                    "line {}\n- {}\n+ {}",
                    i + 1,
                    left.get(i).copied().unwrap_or("<absent>"),
                    right.get(i).copied().unwrap_or("<absent>")
                );
            }
        }
    }
    assert!(
        actual.as_bytes() == expected.as_bytes(),
        "rendered bytes differ from fixture"
    );
}

#[test]
fn claude_full_settings_match_fixture_bytes() -> ResultTest {
    let launch = launch(false, true)?;
    compare(
        &launch
            .files
            .iter()
            .find(|file| file.path == "settings.json")
            .ok_or("settings missing")?
            .text,
        include_str!("../fixtures/claude-settings-full.json"),
    );
    Ok(())
}
#[test]
fn claude_minimal_settings_match_fixture_bytes() -> ResultTest {
    let launch = launch(false, false)?;
    compare(
        &launch
            .files
            .iter()
            .find(|file| file.path == "settings.json")
            .ok_or("settings missing")?
            .text,
        include_str!("../fixtures/claude-settings-minimal.json"),
    );
    Ok(())
}
#[test]
fn codex_full_arguments_match_fixture_bytes() -> ResultTest {
    compare(
        &(launch(true, true)?.arguments.join("\n") + "\n"),
        include_str!("../fixtures/codex-argv-full.txt"),
    );
    Ok(())
}
#[test]
fn codex_minimal_arguments_match_fixture_bytes() -> ResultTest {
    compare(
        &(launch(true, false)?.arguments.join("\n") + "\n"),
        include_str!("../fixtures/codex-argv-minimal.txt"),
    );
    Ok(())
}

#[test]
fn aa_all_runner_requirements_are_exercised() {
    let cases: [RenderingCase; 11] = [
        (
            "claude_runner_hooks_and_judge_are_installed",
            claude_runner_hooks_and_judge_are_installed,
        ),
        (
            "claude_seat_hooks_and_settings_follow_lys",
            claude_seat_hooks_and_settings_follow_lys,
        ),
        (
            "forced_claude_settings_replace_seat_values",
            forced_claude_settings_replace_seat_values,
        ),
        (
            "both_harnesses_render_config_resume_and_skill_directory",
            both_harnesses_render_config_resume_and_skill_directory,
        ),
        (
            "codex_notify_and_proxy_are_rendered",
            codex_notify_and_proxy_are_rendered,
        ),
        (
            "named_runner_paths_are_literal_notify_and_judge_inputs",
            named_runner_paths_are_literal_notify_and_judge_inputs,
        ),
        (
            "seat_status_runs_after_lys_with_the_same_input",
            seat_status_runs_after_lys_with_the_same_input,
        ),
        (
            "claude_full_settings_match_fixture_bytes",
            claude_full_settings_match_fixture_bytes,
        ),
        (
            "claude_minimal_settings_match_fixture_bytes",
            claude_minimal_settings_match_fixture_bytes,
        ),
        (
            "codex_full_arguments_match_fixture_bytes",
            codex_full_arguments_match_fixture_bytes,
        ),
        (
            "codex_minimal_arguments_match_fixture_bytes",
            codex_minimal_arguments_match_fixture_bytes,
        ),
    ];
    let mut failures = Vec::new();
    for (name, case) in cases {
        match std::panic::catch_unwind(case) {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                eprintln!("{name}: {error}");
                failures.push(name);
            }
            Err(_) => {
                eprintln!("{name}: assertion failed");
                failures.push(name);
            }
        }
    }
    assert!(
        failures.is_empty(),
        "runner rendering failures: {failures:?}"
    );
}
