#![cfg(test)]

//! An upgrade on a scratch root: build A installed and running as two stub
//! binaries, upgraded to build B (see the scratch fixture). The first two
//! tests are the round's proof (R4): each holds every check its rows name,
//! runs with no container runtime, no listener on the live port and no live
//! state directory, and removes its temporary install when it ends.

use super::scratch::{
    A, B, Behaviour, Fixed, NEW_KEY, NEW_VALUE, Recorder, Scratch, TestResult, build, package,
    ready_lines, recorded_as, state, stub,
};
use super::{Parts, upgrade, version};
use crate::identity::error::ErrorKind;
use crate::identity::install::layout::{BINARIES, Layout};

#[test]
fn an_upgrade_runs_the_new_build_and_keeps_the_previous() -> TestResult {
    let scratch = Scratch::new()?;
    assert_eq!(scratch.running()?, ready_lines(A));
    assert_eq!(scratch.recorded()?, recorded_as(A));
    let a_files = scratch.configuration()?;
    assert_eq!(
        a_files.len(),
        3,
        "compose.yaml, compose.env and identity.json"
    );
    let new = scratch.work().join("b");
    build(&new, B, Behaviour::Serves)?;
    let screens = scratch.work().join("screens-b");
    package(&screens, B)?;
    let mut said = Vec::new();
    let mut engine = Recorder::default();
    let record = scratch.upgrade(&new, Some(&screens), &mut engine, &mut said)?;
    assert_eq!(scratch.running()?, ready_lines(B));

    assert_eq!(
        scratch.recorded()?,
        recorded_as(B),
        "the recorded build names B for every binary"
    );
    assert_eq!(record.binaries, recorded_as(B));

    for name in BINARIES {
        assert_eq!(
            version(&scratch.layout.binary(name), name)?,
            B,
            "{name} --version names B"
        );
        let kept = scratch.layout.bin_previous_dir().join(name);
        assert_eq!(version(&kept, name)?, A);
        assert!(
            said.contains(&format!("{name}: installed {A}, new {B}")),
            "{said:?}"
        );
    }

    let service_log = std::fs::read_to_string(&scratch.units[1].log)?;
    let started_with = service_log
        .lines()
        .rev()
        .find_map(|line| line.strip_prefix("configured: "))
        .filter(|configured| configured.contains(B))
        .ok_or("B's service never said the configuration it started with")?;
    assert!(
        started_with.contains(NEW_KEY),
        "the service started with B's new key: {started_with}"
    );

    let environment = std::fs::read_to_string(state(&scratch.layout).join("compose.env"))?;
    assert!(environment.contains(NEW_VALUE), "{environment}");
    assert_eq!(engine.applied.len(), 1, "compose applied once, with B's");
    assert!(engine.applied[0].contains(NEW_VALUE));

    assert_eq!(
        scratch.kept_configuration()?,
        a_files,
        "config.previous/ holds each of A's files, byte for byte"
    );

    assert_eq!(
        record.surface.map(|screens| screens.commit).as_deref(),
        Some(B)
    );
    let previous = scratch.layout.surface_previous_dir().join("index.html");
    assert_eq!(
        std::fs::read_to_string(previous)?,
        format!("<html>{A}</html>")
    );
    assert!(
        !scratch.layout.upgrade_intent().exists(),
        "a finished upgrade leaves no intent record"
    );
    let temporary = scratch.dir.path().to_path_buf();
    drop(scratch);
    assert!(!temporary.exists(), "the scratch install is removed");
    Ok(())
}

#[test]
fn a_service_that_exits_before_ready_puts_the_previous_build_back() -> TestResult {
    let scratch = Scratch::new()?;
    let a_files = scratch.configuration()?;
    assert_eq!(
        a_files.len(),
        3,
        "compose.yaml, compose.env and identity.json"
    );
    let new = scratch.work().join("b");
    build(&new, B, Behaviour::ExitsEarly)?;
    let screens = scratch.work().join("screens-b");
    package(&screens, B)?;
    let mut said = Vec::new();
    let mut engine = Recorder::default();
    let refused = scratch
        .upgrade(&new, Some(&screens), &mut engine, &mut said)
        .err()
        .ok_or("a service that exited was taken as ready")?;
    assert_eq!(refused.kind(), ErrorKind::UpgradeFailed);
    let message = refused.to_string();
    assert!(
        message.contains("lys-identity-server did not start ready"),
        "{message}"
    );
    assert!(
        message.contains(&scratch.units[1].log.display().to_string()),
        "{message}"
    );
    assert!(
        message.contains("the previous build is back and running"),
        "{message}"
    );
    let service_log = std::fs::read_to_string(&scratch.units[1].log)?;
    assert!(
        service_log.contains(&format!("lys-identity-server {B} failing")),
        "B was started and failed"
    );
    assert_eq!(
        engine.applied.len(),
        2,
        "B's compose applied, then A's again"
    );

    assert_eq!(scratch.running()?, ready_lines(A), "A is running again");
    for name in BINARIES {
        assert_eq!(version(&scratch.layout.binary(name), name)?, A);
    }

    assert_eq!(
        scratch.recorded()?,
        recorded_as(A),
        "the recorded build names A for every binary"
    );

    assert_eq!(
        scratch.configuration()?,
        a_files,
        "A's configuration and compose files are back, byte for byte"
    );

    let index = scratch.layout.surface_dir().join("index.html");
    assert_eq!(std::fs::read_to_string(index)?, format!("<html>{A}</html>"));
    assert!(!scratch.layout.upgrade_intent().exists());
    let temporary = scratch.dir.path().to_path_buf();
    drop(scratch);
    assert!(!temporary.exists(), "the scratch install is removed");
    Ok(())
}

#[test]
fn neither_upgrade_touches_data_or_a_credential() -> TestResult {
    let scratch = Scratch::new()?;
    let before = scratch.untouchable()?;
    assert_eq!(
        before.len(),
        4,
        "the data file, both credentials and deployment.toml"
    );
    let good = scratch.work().join("b");
    build(&good, B, Behaviour::Serves)?;
    let mut engine = Recorder::default();
    scratch.upgrade(&good, None, &mut engine, &mut Vec::new())?;
    assert_eq!(scratch.untouchable()?, before, "after the upgrade that ran");
    let bad = scratch.work().join("c");
    build(&bad, A, Behaviour::ExitsEarly)?;
    let refused = scratch.upgrade(&bad, None, &mut engine, &mut Vec::new());
    assert!(refused.is_err_and(|error| error.kind() == ErrorKind::UpgradeFailed));
    assert_eq!(scratch.untouchable()?, before, "after the upgrade put back");
    assert_eq!(scratch.running()?, ready_lines(B));
    Ok(())
}

#[test]
fn refusals_are_named_and_stop_nothing() -> TestResult {
    let scratch = Scratch::new()?;
    let mut refusals = Vec::new();
    let empty = scratch.work().join("empty");
    std::fs::create_dir_all(&empty)?;
    let mut engine = Recorder::default();
    let nowhere = Layout::at(scratch.work().join("nowhere"));
    let mut parts = Parts {
        units: &scratch.units,
        engine: &mut engine,
        render: &Fixed,
    };
    let refused = upgrade(&nowhere, &empty, None, &mut parts, &mut |_| {});
    refusals.push(refused.err().map(|error| error.kind()));
    let refused = scratch.upgrade(&empty, None, &mut engine, &mut Vec::new());
    refusals.push(refused.err().map(|error| error.kind()));
    let mute = scratch.work().join("mute");
    build(&mute, B, Behaviour::Serves)?;
    std::fs::write(mute.join(BINARIES[1]), "#!/bin/sh\nexit 0\n")?;
    let refused = scratch.upgrade(&mute, None, &mut engine, &mut Vec::new());
    refusals.push(refused.err().map(|error| error.kind()));
    assert_eq!(
        refusals,
        [
            Some(ErrorKind::NotInstalled),
            Some(ErrorKind::BinaryMissing),
            Some(ErrorKind::VersionUnreadable),
        ]
    );
    assert_eq!(scratch.running()?, ready_lines(A));
    assert!(!scratch.layout.bin_previous_dir().exists());
    assert!(!scratch.layout.upgrade_intent().exists());
    assert!(engine.applied.is_empty(), "nothing was applied");
    Ok(())
}

#[test]
fn a_version_line_gives_its_commit_and_dirty_state() -> TestResult {
    let dir = tempfile::tempdir()?;
    stub(
        dir.path(),
        "lys-secrets",
        &format!("{A}; dirty"),
        Behaviour::Serves,
    )?;
    let read = version(&dir.path().join("lys-secrets"), "lys-secrets")?;
    assert_eq!(read, format!("{A}; dirty"));
    let misnamed = version(&dir.path().join("lys-secrets"), "lys-identity-server");
    assert!(misnamed.is_err_and(|error| error.kind() == ErrorKind::VersionUnreadable));
    Ok(())
}
