use clap::Parser;

use crate::cli::{Cli, Command};

type Result = std::result::Result<(), Box<dyn std::error::Error>>;

#[test]
fn claude_keeps_wrapper_selection_separate_from_harness_arguments() -> Result {
    let cli = Cli::try_parse_from([
        "lys",
        "claude",
        "--agent",
        "my agent",
        "--",
        "--model",
        "sonnet",
        "two words",
    ])?;
    let Command::Claude(args) = cli.command else {
        return Err("not Claude".into());
    };
    assert_eq!(args.options.agent.as_deref(), Some("my agent"));
    assert_eq!(args.arguments, ["--model", "sonnet", "two words"]);
    Ok(())
}

#[test]
fn codex_forwards_flags_and_space_containing_words_unchanged() -> Result {
    let cli = Cli::try_parse_from([
        "lys",
        "codex",
        "--server",
        "https://issuer.example/api",
        "--",
        "exec",
        "two words",
        "--json",
    ])?;
    let Command::Codex(args) = cli.command else {
        return Err("not Codex".into());
    };
    assert_eq!(
        args.options.server.as_deref(),
        Some("https://issuer.example/api")
    );
    assert_eq!(args.arguments, ["exec", "two words", "--json"]);
    assert!(!cli.json, "harness --json must not become Lys JSON mode");
    Ok(())
}

#[test]
fn run_requires_a_command_and_keeps_its_argument_boundaries() -> Result {
    assert!(Cli::try_parse_from(["lys", "run"]).is_err());
    let cli = Cli::try_parse_from([
        "lys",
        "run",
        "--agent",
        "fork",
        "/opt/fork",
        "--",
        "--flag",
        "two words",
    ])?;
    let Command::Run(args) = cli.command else {
        return Err("not run".into());
    };
    assert_eq!(args.command, "/opt/fork");
    assert_eq!(args.options.agent.as_deref(), Some("fork"));
    assert_eq!(args.arguments, ["--flag", "two words"]);
    Ok(())
}

#[test]
fn agents_requires_a_harness_for_an_explicit_default() -> Result {
    assert!(Cli::try_parse_from(["lys", "agents", "--default", "mine"]).is_err());
    let cli = Cli::try_parse_from(["lys", "agents", "--harness", "claude", "--default", "mine"])?;
    let Command::Agents(args) = cli.command else {
        return Err("not agents".into());
    };
    assert_eq!(args.harness.as_deref(), Some("claude"));
    assert_eq!(args.default.as_deref(), Some("mine"));
    Ok(())
}

#[test]
fn every_personal_verb_parses_without_an_invented_server() -> Result {
    for command in ["claude", "codex", "agents"] {
        let cli = Cli::try_parse_from(["lys", command])?;
        let server = match cli.command {
            Command::Claude(args) | Command::Codex(args) => args.options.server,
            Command::Agents(args) => args.server,
            _ => return Err("wrong verb".into()),
        };
        assert_eq!(server, None);
    }
    Ok(())
}
