#![cfg(test)]
//! Root bindings admit generated skills without admitting unrelated paths.

use std::collections::BTreeMap;
use std::error::Error;

use super::{Config, File, checked, prepare};
use crate::Launch;
use crate::protocol::hex;
use crate::tracking::Harness;
use sha2::{Digest, Sha256};

type TestResult = Result<(), Box<dyn Error>>;

fn launch() -> Launch {
    Launch {
        session: "launch-config-admission".to_owned(),
        program: "/bin/cat".to_owned(),
        arguments: vec![String::new()],
        directory: String::new(),
        environment: BTreeMap::new(),
        columns: 80,
        rows: 24,
        rotation: None,
        policy: None,
        config: Some(Config {
            requires_controls: false,
            files: vec![File {
                path: "settings.json".to_owned(),
                text: "exact settings".to_owned(),
                sha256: hex(&Sha256::digest(b"exact settings")),
            }],
            argument_files: BTreeMap::from([(0, String::new())]),
            environment_paths: BTreeMap::from([(
                "CONFIG_FILE".to_owned(),
                "settings.json".to_owned(),
            )]),
            working_directory: true,
            harness: None,
        }),
    }
}

#[test]
fn launch_config_empty_argument_path_binds_to_generated_root() -> TestResult {
    let state = tempfile::tempdir()?;
    let mut launch = launch();
    prepare(state.path(), &mut launch)?;
    let root = state.path().join("sessions/launch-config-admission/config");
    assert_eq!(launch.arguments.len(), 1);
    assert_eq!(std::path::Path::new(&launch.arguments[0]), root);
    assert_eq!(launch.directory, root.to_str().ok_or("root is not text")?);
    assert_eq!(
        launch.environment.get("CONFIG_FILE"),
        Some(
            &root
                .join("settings.json")
                .to_str()
                .ok_or("file is not text")?
                .to_owned()
        )
    );
    assert_eq!(
        std::fs::read(root.join("settings.json"))?,
        b"exact settings"
    );
    Ok(())
}

#[test]
fn launch_config_foreign_argument_paths_and_out_of_range_root_are_refused() -> TestResult {
    for path in ["foreign.json", "../outside", "/outside", "."] {
        let state = tempfile::tempdir()?;
        let mut launch = launch();
        launch
            .config
            .as_mut()
            .ok_or("config missing")?
            .argument_files = BTreeMap::from([(0, path.to_owned())]);
        let error = prepare(state.path(), &mut launch).expect_err("foreign path admitted");
        assert_eq!(error.name(), "launch_config_refused");
        assert!(!state.path().join("sessions").exists());
    }
    let launch = launch();
    let config = launch.config.as_ref().ok_or("config missing")?;
    assert_eq!(
        checked(config, 0)
            .expect_err("out of range root admitted")
            .name(),
        "launch_config_refused"
    );
    Ok(())
}

#[test]
fn launch_config_non_claude_settings_keep_non_json_fixture_bytes() -> TestResult {
    for harness in [None, Some(Harness::Codex)] {
        let state = tempfile::tempdir()?;
        let mut launch = launch();
        let config = launch.config.as_mut().ok_or("config missing")?;
        config.harness = harness;
        config.argument_files.clear();
        prepare(state.path(), &mut launch)?;
        assert_eq!(
            std::fs::read(
                state
                    .path()
                    .join("sessions/launch-config-admission/config/settings.json")
            )?,
            b"exact settings"
        );
    }
    Ok(())
}

#[test]
fn launch_config_claude_settings_still_require_json_before_publication() -> TestResult {
    let state = tempfile::tempdir()?;
    let mut launch = launch();
    let config = launch.config.as_mut().ok_or("config missing")?;
    config.harness = Some(Harness::ClaudeCode);
    config.argument_files.clear();
    let error = prepare(state.path(), &mut launch).expect_err("invalid Claude settings admitted");
    assert_eq!(error.name(), "launch_config_refused");
    assert!(error.to_string().contains("settings.json is not JSON"));
    assert!(!state.path().join("sessions").exists());
    Ok(())
}
