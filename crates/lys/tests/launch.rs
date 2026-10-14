//! Every personal launch verb refuses absent authority before touching a harness.

use std::process::Command;

type Result = std::result::Result<(), Box<dyn std::error::Error>>;

#[test]
fn claude_codex_run_and_agents_name_an_absent_recorded_server_without_operator_fallback() -> Result
{
    let root = tempfile::tempdir()?;
    std::fs::write(
        root.path().join("identity.json"),
        "not a usable operator install",
    )?;
    for arguments in [
        vec!["claude"],
        vec!["codex"],
        vec!["run", "claude"],
        vec!["agents"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_lys"))
            .env("LYS_IDENTITY_HOME", root.path())
            .args(arguments)
            .output()?;
        assert_eq!(output.status.code(), Some(1));
        let error = String::from_utf8(output.stderr)?;
        assert!(error.contains("server_address_missing"), "{error}");
        assert!(
            error.contains("lys sign-in --server <server-address>"),
            "{error}"
        );
        assert!(!error.contains("identity install"), "{error}");
        assert!(output.stdout.is_empty());
    }
    Ok(())
}

#[test]
fn each_personal_verb_prints_the_exact_supplied_sign_in_command_in_json_failure() -> Result {
    let root = tempfile::tempdir()?;
    for arguments in [
        vec!["claude"],
        vec!["codex"],
        vec!["run", "claude"],
        vec!["agents"],
    ] {
        let mut full = vec![
            "--json",
            arguments[0],
            "--server",
            "https://issuer.example/api",
        ];
        full.extend(arguments.iter().skip(1).copied());
        let output = Command::new(env!("CARGO_BIN_EXE_lys"))
            .env("LYS_IDENTITY_HOME", root.path())
            .args(full)
            .output()?;
        assert_eq!(output.status.code(), Some(1));
        let answer: serde_json::Value = serde_json::from_slice(&output.stdout)?;
        assert_eq!(answer["ok"], false);
        let error = answer["error"]
            .as_str()
            .ok_or("JSON failure has no reason")?;
        assert!(error.contains("person_pass_missing"), "{error}");
        assert!(
            error.contains("lys sign-in --server https://issuer.example/api"),
            "{error}"
        );
    }
    Ok(())
}

#[test]
fn personal_verbs_are_explained_by_their_own_help() -> Result {
    for command in ["claude", "codex", "run", "agents"] {
        let output = Command::new(env!("CARGO_BIN_EXE_lys"))
            .args([command, "--help"])
            .output()?;
        assert_eq!(output.status.code(), Some(0));
        let help = String::from_utf8(output.stdout)?;
        assert!(help.contains("--server"));
        assert!(help.contains(if command == "agents" {
            "--default"
        } else {
            "--agent"
        }));
    }
    Ok(())
}
