//! Catalogue reads count retained work and executable cache entries.

use std::error::Error;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use serde_json::{Value, json};

use super::{BuildSource, Catalogue, profile_builds};
use crate::provisioning_store::ProvisioningStore;

type TestResult = Result<(), Box<dyn Error>>;

fn estate(path: &Path, count: u32) -> TestResult {
    let source: Value =
        serde_json::from_str(include_str!("../../../docs/harness/catalogue/codex.json"))?;
    let versions: Vec<Value> = (1..=count).map(|number| json!({
        "number": number, "operation": format!("profile-{number}"), "set_by": "operator", "set_at": 1,
        "reviewed": if number > count / 2 { json!(null) } else { json!({"operation": format!("review-{number}"), "by": "reviewer", "at": 2}) },
        "settings": {
            "model_access": ["model"], "tools": [], "skills": [], "mcp_servers": [], "instructions": "", "note": "",
            "harness": {"name": "Reviewed build", "program": "/opt/codex", "package": if number > count / 2 { "unreviewed" } else { "build" }, "description": source["description"]}
        }
    })).collect();
    std::fs::write(
        path,
        serde_json::to_vec(&json!({"profiles": [{"agent": "agent", "versions": versions}]}))?,
    )?;
    Ok(())
}

#[test]
fn catalogue_get_does_not_visit_retained_profile_history() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("profiles.json");
    estate(&path, 24)?;
    let store = ProvisioningStore::open(&path)?;
    let catalogue = Catalogue::embedded()?;
    let mut programs = catalogue.programs.clone();
    let mut visits = 0;
    profile_builds(&mut programs, &store, || visits += 1);
    let codex = programs
        .iter()
        .find(|program| program.name == "Codex")
        .ok_or("Codex is absent")?;
    assert_eq!(codex.builds.len(), 1);
    assert_eq!(codex.builds[0].source, BuildSource::Profile);
    assert_eq!(codex.builds[0].program, "/opt/codex");
    assert_eq!(visits, 0, "a catalogue GET revisited retained versions");
    Ok(())
}

#[test]
fn repeated_catalogue_get_work_is_independent_of_history_length() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("profiles.json");
    for count in [8, 128] {
        estate(&path, count)?;
        let store = ProvisioningStore::open(&path)?;
        let catalogue = Catalogue::embedded()?;
        let mut visits = 0;
        for pass in 0..2 {
            let mut programs = catalogue.programs.clone();
            profile_builds(&mut programs, &store, || visits += 1);
            assert_eq!(
                programs
                    .iter()
                    .map(|program| program.builds.len())
                    .sum::<usize>(),
                1
            );
            assert_eq!(visits, 0, "{count} versions, read {pass}");
        }
    }
    Ok(())
}

#[test]
fn only_current_executable_metadata_is_retained_per_command() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("codex");
    let catalogue = Catalogue::embedded()?;
    for version in ["first", "second-longer"] {
        std::fs::write(&path, format!("#!/bin/sh\nprintf '{version}\\n'\n"))?;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700))?;
        let answer = catalogue.installed_in(directory.path().as_os_str())?;
        let codex = answer
            .programs
            .iter()
            .find(|program| program.name == "Codex")
            .ok_or("Codex is absent")?;
        assert_eq!(codex.builds.len(), 1);
        assert_eq!(codex.builds[0].package, version);
    }
    assert_eq!(
        catalogue
            .installed
            .lock()
            .map_err(|error| error.to_string())?
            .len(),
        1
    );
    Ok(())
}
