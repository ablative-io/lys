#![cfg(test)]

//! An upgrade on a scratch root: build A installed and running as two stub
//! binaries, upgraded to build B. The stubs answer `--version` as a Lys
//! binary does, say `ready` in their log once started and then block on a
//! pipe nobody writes, until they are stopped.

use std::collections::BTreeMap;
use std::error::Error;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};

use super::{Ready, Unit, launch, upgrade, version};
use crate::error::ErrorKind;
use crate::install::layout::{BINARIES, Layout};
use crate::install::services;

type TestResult = Result<(), Box<dyn Error>>;

const A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

/// How a stub behaves once started.
#[derive(Clone, Copy)]
enum Behaviour {
    /// Says ready and keeps running.
    Serves,
    /// Exits before it is ready.
    ExitsEarly,
}

/// Writes the stub `name` answering `--version` with `commit` into `dir`.
fn stub(dir: &Path, name: &str, commit: &str, behaviour: Behaviour) -> TestResult {
    std::fs::create_dir_all(dir)?;
    let body = match behaviour {
        Behaviour::Serves => format!("echo \"{name} {commit} ready\"\nexec cat \"$1\"\n"),
        Behaviour::ExitsEarly => format!("echo \"{name} {commit} failing\"\nexit 3\n"),
    };
    let script = format!(
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo \"{name} 0.2.0 ({commit})\"; exit 0; fi\n{body}"
    );
    let path = dir.join(name);
    std::fs::write(&path, script)?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))?;
    Ok(())
}

/// A build: every binary the install runs, each a stub of `commit`.
fn build(dir: &Path, commit: &str, service: Behaviour) -> TestResult {
    stub(dir, BINARIES[0], commit, Behaviour::Serves)?;
    stub(dir, BINARIES[1], commit, service)
}

/// A screens package from `commit`.
fn package(dir: &Path, commit: &str) -> TestResult {
    std::fs::create_dir_all(dir)?;
    let index = format!("<html>{commit}</html>");
    std::fs::write(dir.join("index.html"), &index)?;
    let manifest = serde_json::json!({
        "format": "lys-identity-surface/v1",
        "commit": commit,
        "files": [{
            "path": "index.html",
            "bytes": index.len(),
            "sha256": format!("{:x}", Sha256::digest(index.as_bytes())),
        }],
    });
    std::fs::write(dir.join("surface-manifest.json"), manifest.to_string())?;
    Ok(())
}

/// A scratch install running build A, and the units that run it.
struct Scratch {
    dir: tempfile::TempDir,
    layout: Layout,
    units: Vec<Unit>,
}

impl Scratch {
    fn new() -> Result<Self, Box<dyn Error>> {
        let dir = tempfile::tempdir()?;
        let layout = Layout::at(dir.path().join("root"));
        let work = dir.path().join("work");
        for made in [
            layout.logs_dir(),
            layout.run_dir(),
            layout.data_dir().join("directory"),
            work.clone(),
        ] {
            std::fs::create_dir_all(made)?;
        }
        std::fs::write(layout.deployment_config(), "[deployment]\nadmin = 1\n")?;
        std::fs::write(layout.service_config(), "{\"listen\": \"127.0.0.1:0\"}\n")?;
        std::fs::write(layout.data_dir().join("directory").join("log"), [7_u8; 64])?;
        let hold = work.join("hold");
        let made = Command::new("mkfifo").arg(&hold).status()?;
        assert!(made.success(), "mkfifo failed");
        let units = BINARIES
            .into_iter()
            .zip(["secrets", "identity"])
            .map(|(binary, file)| Unit {
                binary,
                args: vec![hold.display().to_string()],
                log: layout.logs_dir().join(format!("{file}.log")),
                pid: layout.run_dir().join(format!("{file}.pid")),
                ready: Ready {
                    says: "ready".to_string(),
                    answers: None,
                },
            })
            .collect();
        let scratch = Self { dir, layout, units };
        build(&scratch.layout.bin_dir(), A, Behaviour::Serves)?;
        package(&scratch.layout.surface_dir(), A)?;
        for unit in &scratch.units {
            assert!(launch(&scratch.layout, unit, false)?, "A did not start");
        }
        Ok(scratch)
    }

    /// Where the new builds are made, outside the root.
    fn work(&self) -> PathBuf {
        self.dir.path().join("work")
    }

    /// Every file under `data/`, `identity.json` and `deployment.toml`.
    fn untouchable(&self) -> Result<BTreeMap<PathBuf, Vec<u8>>, Box<dyn Error>> {
        let mut files = BTreeMap::new();
        let mut pending = vec![self.layout.data_dir()];
        while let Some(dir) = pending.pop() {
            for entry in std::fs::read_dir(dir)? {
                let path = entry?.path();
                if path.is_dir() {
                    pending.push(path);
                } else {
                    let bytes = std::fs::read(&path)?;
                    files.insert(path, bytes);
                }
            }
        }
        for path in [
            self.layout.service_config(),
            self.layout.deployment_config(),
        ] {
            let bytes = std::fs::read(&path)?;
            files.insert(path, bytes);
        }
        Ok(files)
    }

    /// Each unit's commit as its running process last said it.
    fn running(&self) -> Result<Vec<String>, Box<dyn Error>> {
        let mut said = Vec::new();
        for unit in &self.units {
            assert!(services::alive(&unit.pid), "{} is not running", unit.binary);
            let log = std::fs::read_to_string(&unit.log)?;
            let last = log
                .lines()
                .rev()
                .find(|line| line.ends_with(" ready"))
                .ok_or("no ready line")?;
            said.push(last.to_string());
        }
        Ok(said)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        for unit in &self.units {
            if let Err(error) = services::stop(&unit.pid) {
                eprintln!("stopping {} after the test: {error}", unit.binary);
            }
        }
    }
}

fn ready_lines(commit: &str) -> Vec<String> {
    BINARIES
        .iter()
        .map(|name| format!("{name} {commit} ready"))
        .collect()
}

#[test]
fn an_upgrade_runs_the_new_build_and_keeps_the_previous() -> TestResult {
    let scratch = Scratch::new()?;
    assert_eq!(scratch.running()?, ready_lines(A));
    let new = scratch.work().join("b");
    build(&new, B, Behaviour::Serves)?;
    let screens = scratch.work().join("screens-b");
    package(&screens, B)?;
    let mut said = Vec::new();
    let record = upgrade(
        &scratch.layout,
        &new,
        Some(&screens),
        &scratch.units,
        &mut |line| said.push(line.to_string()),
    )?;
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
    let refused = upgrade(
        &scratch.layout,
        &new,
        Some(&screens),
        &scratch.units,
        &mut |line| said.push(line.to_string()),
    )
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
    Ok(())
}

#[test]
fn neither_upgrade_touches_data_or_configuration() -> TestResult {
    let scratch = Scratch::new()?;
    let before = scratch.untouchable()?;
    assert_eq!(before.len(), 3, "the data file and both configurations");
    let good = scratch.work().join("b");
    build(&good, B, Behaviour::Serves)?;
    upgrade(&scratch.layout, &good, None, &scratch.units, &mut |_| {})?;
    assert_eq!(scratch.untouchable()?, before, "after the upgrade that ran");
    let bad = scratch.work().join("c");
    build(&bad, A, Behaviour::ExitsEarly)?;
    let refused = upgrade(&scratch.layout, &bad, None, &scratch.units, &mut |_| {});
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
    let nowhere = Layout::at(scratch.work().join("nowhere"));
    let refused = upgrade(&nowhere, &empty, None, &scratch.units, &mut |_| {});
    refusals.push(refused.err().map(|error| error.kind()));
    let refused = upgrade(&scratch.layout, &empty, None, &scratch.units, &mut |_| {});
    refusals.push(refused.err().map(|error| error.kind()));
    let mute = scratch.work().join("mute");
    build(&mute, B, Behaviour::Serves)?;
    std::fs::write(mute.join(BINARIES[1]), "#!/bin/sh\nexit 0\n")?;
    let refused = upgrade(&scratch.layout, &mute, None, &scratch.units, &mut |_| {});
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
