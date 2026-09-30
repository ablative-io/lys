//! Installed command choices preserve native process inputs and kept profiles.

use std::error::Error;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

use lys_identity_server::harness_catalogue::{BuildSource, BuildView, Catalogue};
use lys_identity_server::launch_template::{Start, from_template, render};
use lys_identity_server::provisioning_store::{ProvisioningStore, Version};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

fn executable(directory: &Path, name: &str) -> TestResult {
    let path = directory.join(name);
    std::fs::write(
        &path,
        format!(
            "#!/bin/sh\nif test \"$1\" = --version; then printf '{name} fixture\\n'; else printf '%s\\n' \"$@\"; fi\n"
        ),
    )?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))?;
    Ok(())
}

fn builds(directory: &Path) -> Result<Vec<BuildView>, Box<dyn Error>> {
    let answer = Catalogue::embedded()?.installed_in(directory.as_os_str())?;
    let codex = answer
        .programs
        .into_iter()
        .find(|program| program.name == "Codex")
        .ok_or("Codex is absent")?;
    assert!(codex.not_found.is_none(), "{:?}", codex.not_found);
    Ok(codex.builds)
}

fn version(program: &str) -> Result<Version, Box<dyn Error>> {
    let catalogue: Value =
        serde_json::from_str(include_str!("../../../docs/harness/catalogue/codex.json"))?;
    Ok(serde_json::from_value(json!({
        "number": 1, "operation": "profile", "set_by": "operator", "set_at": 1,
        "settings": {
            "harness": {"name": "Codex", "program": program, "package": "fixture", "description": catalogue["description"]},
            "model_access": ["gpt-6.1-sol"], "tools": [], "skills": [], "mcp_servers": [],
            "permissions": {"default_mode": "workspace-write"},
            "instructions": "Carry the reviewed prompt.", "instructions_mode": "append", "note": "", "runs_on": "machine"
        }
    }))?)
}

#[test]
fn both_installed_codex_commands_are_choices_in_standard_order() -> TestResult {
    let directory = tempfile::tempdir()?;
    executable(directory.path(), "codex")?;
    executable(directory.path(), "cdx")?;
    let copies = builds(directory.path())?;
    assert_eq!(copies.len(), 2);
    for (copy, name) in copies.iter().zip(["codex", "cdx"]) {
        assert_eq!(copy.name, format!("Installed {name}"));
        assert_eq!(copy.source, BuildSource::Installed);
        assert_eq!(copy.package, format!("{name} fixture"));
        assert_eq!(
            Path::new(&copy.program),
            std::fs::canonicalize(directory.path().join(name))?
        );
    }
    Ok(())
}

#[test]
fn choosing_cdx_executes_its_path_with_the_codex_flags_unchanged() -> TestResult {
    let directory = tempfile::tempdir()?;
    executable(directory.path(), "codex")?;
    executable(directory.path(), "cdx")?;
    let copies = builds(directory.path())?;
    let copy = copies
        .iter()
        .find(|copy| copy.name == "Installed cdx")
        .ok_or("cdx is absent")?;
    let version = version(&copy.program)?;
    let rendered = render(
        &Start {
            agent: "agent",
            session: "session",
            machine: "machine",
            runtime: "runner",
            version: &version,
            skills: &[],
            policy: None,
        },
        &[],
    )?;
    let launch = from_template(&version, &rendered.template, &rendered.template_sha256)?;
    let expected = [
        "--model",
        "gpt-6.1-sol",
        "--ask-for-approval",
        "on-request",
        "-C",
        ".",
        "--sandbox",
        "workspace-write",
        "-c",
        "developer_instructions=\"Carry the reviewed prompt.\"",
    ];
    assert_eq!(launch.program, copy.program);
    assert_eq!(launch.arguments, expected);
    let output = Command::new(&launch.program)
        .args(&launch.arguments)
        .output()?;
    assert!(output.status.success(), "{}", output.status);
    assert_eq!(
        String::from_utf8(output.stdout)?,
        format!("{}\n", expected.join("\n"))
    );
    Ok(())
}

#[test]
fn an_absolute_fork_path_and_its_machine_survive_profile_reopen() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("profiles.json");
    let wanted = version("/opt/forks/another-codex")?;
    let mut store = ProvisioningStore::open(&path)?;
    store.set("agent", 0, wanted.clone())?;
    drop(store);
    let reopened = ProvisioningStore::open(&path)?;
    let kept = reopened.profile("agent").ok_or("profile is absent")?;
    assert_eq!(kept.versions, vec![wanted]);
    Ok(())
}

#[test]
fn a_declared_fork_is_offered_when_the_standard_command_is_missing() -> TestResult {
    let directory = tempfile::tempdir()?;
    executable(directory.path(), "another-codex")?;
    let mut source: Value =
        serde_json::from_str(include_str!("../../../docs/harness/catalogue/codex.json"))?;
    source["commands"] = json!(["codex", "another-codex"]);
    let text = serde_json::to_string(&source)?;
    let catalogue = Catalogue::read(&[("fixture.json", &text)])?;
    let answer = catalogue.installed_in(directory.path().as_os_str())?;
    let program = answer.programs.first().ok_or("program is absent")?;
    assert!(program.not_found.is_none());
    assert_eq!(program.builds.len(), 1);
    assert_eq!(program.builds[0].name, "Installed another-codex");
    Ok(())
}

#[test]
fn older_catalogue_sources_default_to_their_standard_command() -> TestResult {
    let directory = tempfile::tempdir()?;
    executable(directory.path(), "codex")?;
    executable(directory.path(), "cdx")?;
    let mut source: Value =
        serde_json::from_str(include_str!("../../../docs/harness/catalogue/codex.json"))?;
    source
        .as_object_mut()
        .ok_or("source is not an object")?
        .remove("commands");
    let text = serde_json::to_string(&source)?;
    let answer =
        Catalogue::read(&[("older.json", &text)])?.installed_in(directory.path().as_os_str())?;
    let program = answer.programs.first().ok_or("program is absent")?;
    assert_eq!(program.commands, ["codex"]);
    assert_eq!(program.builds.len(), 1);
    assert_eq!(program.builds[0].name, "Installed codex");
    Ok(())
}

#[test]
fn invalid_command_lists_name_the_catalogue_file() -> TestResult {
    let source: Value =
        serde_json::from_str(include_str!("../../../docs/harness/catalogue/codex.json"))?;
    for commands in [
        json!([]),
        json!(["cdx", "codex"]),
        json!(["codex", "codex"]),
        json!(["codex", "../cdx"]),
        json!(["codex", "cdx --flag"]),
    ] {
        let mut invalid = source.clone();
        invalid["commands"] = commands;
        let text = serde_json::to_string(&invalid)?;
        match Catalogue::read(&[("invalid.json", &text)]) {
            Err(lys_identity_server::error::ServerError::HarnessCatalogueUnreadable {
                file,
                reason,
            }) => {
                assert_eq!(file, "invalid.json");
                assert!(!reason.is_empty());
            }
            other => return Err(format!("invalid commands were not refused: {other:?}").into()),
        }
    }
    Ok(())
}
