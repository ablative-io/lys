#![cfg(test)]

//! An upgrade on a scratch root: build A installed and running as two stub
//! binaries, upgraded to build B (see the scratch fixture).

use super::scratch::{
    A, B, Behaviour, Fixed, Recorder, Scratch, TestResult, build, package, ready_lines, stub,
};
use super::{Parts, upgrade, version};
use crate::identity::error::ErrorKind;
use crate::identity::install::layout::{BINARIES, Layout};

#[test]
fn an_upgrade_runs_the_new_build_and_keeps_the_previous() -> TestResult {
    let scratch = Scratch::new()?;
    assert_eq!(scratch.running()?, ready_lines(A));
    let new = scratch.work().join("b");
    build(&new, B, Behaviour::Serves)?;
    let screens = scratch.work().join("screens-b");
    package(&screens, B)?;
    let mut said = Vec::new();
    let record = scratch.upgrade(&new, Some(&screens), &mut Recorder::default(), &mut said)?;
    assert_eq!(scratch.running()?, ready_lines(B));
    for name in BINARIES {
        assert_eq!(version(&scratch.layout.binary(name), name)?, B);
        let kept = scratch.layout.bin_previous_dir().join(name);
        assert_eq!(version(&kept, name)?, A);
        assert!(
            said.contains(&format!("{name}: installed {A}, new {B}")),
            "{said:?}"
        );
        assert_eq!(record.binaries.get(name).map(String::as_str), Some(B));
    }
    assert_eq!(
        record.surface.map(|screens| screens.commit).as_deref(),
        Some(B)
    );
    let previous = scratch.layout.surface_previous_dir().join("index.html");
    assert_eq!(
        std::fs::read_to_string(previous)?,
        format!("<html>{A}</html>")
    );
    let written: serde_json::Value =
        serde_json::from_slice(&std::fs::read(scratch.layout.build_record())?)?;
    assert_eq!(written["binaries"]["lys-identity-server"], B);
    assert_eq!(written["surface"]["commit"], B);
    assert!(
        !scratch.layout.upgrade_intent().exists(),
        "a finished upgrade leaves no intent record"
    );
    Ok(())
}

#[test]
fn a_service_that_exits_before_ready_puts_the_previous_build_back() -> TestResult {
    let scratch = Scratch::new()?;
    let new = scratch.work().join("b");
    build(&new, B, Behaviour::ExitsEarly)?;
    let screens = scratch.work().join("screens-b");
    package(&screens, B)?;
    let mut said = Vec::new();
    let refused = scratch
        .upgrade(&new, Some(&screens), &mut Recorder::default(), &mut said)
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
    assert_eq!(scratch.running()?, ready_lines(A));
    let service_log = std::fs::read_to_string(&scratch.units[1].log)?;
    assert!(service_log.contains(&format!("lys-identity-server {B} failing")));
    for name in BINARIES {
        assert_eq!(version(&scratch.layout.binary(name), name)?, A);
    }
    let index = scratch.layout.surface_dir().join("index.html");
    assert_eq!(std::fs::read_to_string(index)?, format!("<html>{A}</html>"));
    let written: serde_json::Value =
        serde_json::from_slice(&std::fs::read(scratch.layout.build_record())?)?;
    assert_eq!(written["binaries"]["lys-identity-server"], A);
    assert!(!scratch.layout.upgrade_intent().exists());
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
        runner: None,
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

#[test]
fn old_install_gains_runner_and_later_upgrades_keep_sessions_and_keys() -> TestResult {
    use crate::identity::install::services;
    use lys_core::Ed25519Identity;
    use lys_runner::{Act, Answer, Client};
    use std::os::unix::fs::PermissionsExt;
    use std::sync::Arc;

    struct RunnerCleanup(std::path::PathBuf);
    impl Drop for RunnerCleanup {
        fn drop(&mut self) {
            if let Err(error) = services::stop(&self.0) {
                eprintln!("runner cleanup {} failed: {error}", self.0.display());
            }
        }
    }
    let scratch = Scratch::new()?;
    for directory in [
        scratch.layout.run_dir(),
        scratch.layout.logs_dir(),
        scratch.layout.data_dir(),
    ] {
        std::fs::set_permissions(directory, std::fs::Permissions::from_mode(0o700))?;
    }
    let pid = scratch.layout.run_dir().join("runner.pid");
    let cleanup = RunnerCleanup(pid.clone());
    assert!(!scratch.layout.runner_socket().exists());
    assert!(!pid.exists());
    let program = std::env::current_exe()?
        .parent()
        .and_then(std::path::Path::parent)
        .ok_or("test executable lacks target/debug parent")?
        .join("lys");
    assert!(
        program.is_file(),
        "build the lys binary before this real-runner test"
    );
    let new = scratch.work().join("b");
    build(&new, B, Behaviour::Serves)?;
    let key = Arc::new(crate::identity::install::service_key(&scratch.layout)?);
    let key_bytes = std::fs::read(scratch.layout.service_key())?;
    let public_file = scratch.layout.run_dir().join("runner-server.pub");
    let mut engine = Recorder::default();
    let mut parts = Parts {
        runner: Some(&program),
        units: &scratch.units,
        engine: &mut engine,
        render: &Fixed,
    };
    upgrade(&scratch.layout, &new, None, &mut parts, &mut |_| {})?;
    let client = Client::new(scratch.layout.runner_socket(), Arc::clone(&key));
    assert!(matches!(
        client.ask(&Act::Status { session: None })?,
        Answer::Status { .. }
    ));
    let first_pid = std::fs::read_to_string(&pid)?;
    let public = std::fs::read(&public_file)?;
    assert_eq!(
        public,
        lys_runner::protocol::hex(&key.public_key_bytes()).as_bytes()
    );
    let launch = lys_runner::Launch {
        session: "upgrade-keeps-session".into(),
        program: "/bin/cat".into(),
        arguments: vec![],
        directory: scratch.work().display().to_string(),
        environment: std::collections::BTreeMap::default(),
        columns: 80,
        rows: 24,
        rotation: None,
        policy: None,
    };
    client.ask(&Act::Start { launch })?;
    upgrade(&scratch.layout, &new, None, &mut parts, &mut |_| {})?;
    assert_eq!(std::fs::read_to_string(&pid)?, first_pid);
    let Answer::Status { status } = client.ask(&Act::Status { session: None })? else {
        return Err("no Status after second upgrade".into());
    };
    assert!(
        status
            .sessions
            .iter()
            .any(|s| s.session == "upgrade-keeps-session" && s.ended.is_none())
    );
    assert_eq!(std::fs::read(scratch.layout.service_key())?, key_bytes);
    assert_eq!(std::fs::read(&public_file)?, public);
    assert_eq!(
        Ed25519Identity::load(&scratch.layout.service_key())?.public_key_bytes(),
        key.public_key_bytes()
    );
    services::stop(&pid)?;
    upgrade(&scratch.layout, &new, None, &mut parts, &mut |_| {})?;
    assert!(matches!(
        client.ask(&Act::Status { session: None })?,
        Answer::Status { .. }
    ));
    assert_ne!(std::fs::read_to_string(&pid)?, first_pid);
    assert_eq!(std::fs::read(scratch.layout.service_key())?, key_bytes);
    services::stop(&pid)?;
    drop(cleanup);
    Ok(())
}
