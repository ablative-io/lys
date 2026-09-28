#![cfg(test)]

//! `lys identity upgrade --back` on a scratch root: build A installed and
//! running as two stub binaries, upgraded to build B, then returned to A
//! and forward to B again. The stubs answer `--version` as a Lys binary
//! does, with any data format lines they are given, say `ready` in their
//! log once started and then block on a pipe nobody writes, until they are
//! stopped.

use std::collections::BTreeMap;
use std::error::Error;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use clap::Parser;
use sha2::{Digest, Sha256};

use super::{BASELINE_FORMAT, Ready, Unit, back, launch, newest, stated, upgrade, version};
use crate::identity::cli::IdentityCommand;
use crate::identity::error::{ErrorKind, IdentityError};
use crate::identity::install::layout::{BINARIES, Layout};
use crate::identity::install::services;

type TestResult = Result<(), Box<dyn Error>>;

const A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

/// The event kind build B's data format adds in these tests.
const NEW_KIND: &str = "team.renamed";

/// How a stub behaves once started.
#[derive(Clone, Copy)]
enum Behaviour {
    /// Says ready and keeps running.
    Serves,
    /// Exits before it is ready.
    ExitsEarly,
}

/// Writes the stub `name` into `dir`, answering `--version` with `commit`
/// and then each of `formats` as a line of its detail.
fn stub(
    dir: &Path,
    name: &str,
    commit: &str,
    behaviour: Behaviour,
    formats: &[&str],
) -> TestResult {
    std::fs::create_dir_all(dir)?;
    let body = match behaviour {
        Behaviour::Serves => format!("echo \"{name} {commit} ready\"\nexec cat \"$1\"\n"),
        Behaviour::ExitsEarly => format!("echo \"{name} {commit} failing\"\nexit 3\n"),
    };
    let mut detail = String::new();
    for line in formats {
        detail.push_str("echo \"");
        detail.push_str(line);
        detail.push_str("\"; ");
    }
    let script = format!(
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo \"{name} 0.2.0 ({commit})\"; {detail}exit 0; fi\n{body}"
    );
    let path = dir.join(name);
    std::fs::write(&path, script)?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))?;
    Ok(())
}

/// A build: every binary the install runs, each a stub of `commit`; the
/// service states `formats`.
fn build(dir: &Path, commit: &str, service: Behaviour, formats: &[&str]) -> TestResult {
    stub(dir, BINARIES[0], commit, Behaviour::Serves, &[])?;
    stub(dir, BINARIES[1], commit, service, formats)
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
        build(&scratch.layout.bin_dir(), A, Behaviour::Serves, &[])?;
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

    /// Upgrades the scratch install to build B (and B's screens), whose
    /// service states `formats`.
    fn upgrade_to_b(&self, formats: &[&str]) -> TestResult {
        let new = self.work().join("b");
        build(&new, B, Behaviour::Serves, formats)?;
        let screens = self.work().join("screens-b");
        package(&screens, B)?;
        let screens = Some(screens.as_path());
        upgrade(&self.layout, &new, screens, &self.units, &mut |_| {})?;
        assert_eq!(self.running()?, ready_lines(B), "the upgrade to B ran");
        Ok(())
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
        for path in [self.layout.service_config(), self.layout.deployment_config()] {
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

    /// Each running process's pid file, which a stop and a start change.
    fn pids(&self) -> Result<Vec<String>, Box<dyn Error>> {
        let mut pids = Vec::new();
        for unit in &self.units {
            pids.push(std::fs::read_to_string(&unit.pid)?);
        }
        Ok(pids)
    }

    /// The commit each binary in `dir` answers with.
    fn commits(dir: &Path) -> Result<Vec<String>, Box<dyn Error>> {
        let mut commits = Vec::new();
        for name in BINARIES {
            commits.push(version(&dir.join(name), name)?);
        }
        Ok(commits)
    }

    /// What the placed screens (or the kept ones) say they are.
    fn screens(dir: &Path) -> Result<String, Box<dyn Error>> {
        Ok(std::fs::read_to_string(dir.join("index.html"))?)
    }

    /// What `install/build.json` records.
    fn record(&self) -> Result<serde_json::Value, Box<dyn Error>> {
        let bytes = std::fs::read(self.layout.build_record())?;
        Ok(serde_json::from_slice(&bytes)?)
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

fn both(commit: &str) -> Vec<String> {
    vec![commit.to_string(); BINARIES.len()]
}

#[test]
fn back_runs_the_kept_build_and_back_again_runs_the_newer() -> TestResult {
    let scratch = Scratch::new()?;
    let before = scratch.untouchable()?;
    assert_eq!(before.len(), 3, "the data file and both configurations");
    scratch.upgrade_to_b(&[])?;
    let bin = scratch.layout.bin_dir();
    let kept = scratch.layout.bin_previous_dir();
    let screens = scratch.layout.surface_dir();
    let kept_screens = scratch.layout.surface_previous_dir();
    let mut said = Vec::new();
    let mut hear = |line: &str| said.push(line.to_string());
    let record = back(&scratch.layout, &scratch.units, &mut hear)?;
    assert_eq!(scratch.running()?, ready_lines(A), "back runs A");
    assert_eq!(Scratch::commits(&bin)?, both(A));
    assert_eq!(Scratch::commits(&kept)?, both(B), "the build left is the one kept");
    assert_eq!(Scratch::screens(&screens)?, format!("<html>{A}</html>"));
    assert_eq!(Scratch::screens(&kept_screens)?, format!("<html>{B}</html>"));
    for name in BINARIES {
        assert!(said.contains(&format!("{name}: running {B}, kept {A}")), "{said:?}");
        assert_eq!(record.binaries.get(name).map(String::as_str), Some(A));
    }
    assert_eq!(scratch.record()?["binaries"]["lys-identity-server"], A);
    assert_eq!(scratch.record()?["surface"]["commit"], A);
    back(&scratch.layout, &scratch.units, &mut |_| {})?;
    assert_eq!(scratch.running()?, ready_lines(B), "back again runs B");
    assert_eq!(Scratch::commits(&bin)?, both(B));
    assert_eq!(Scratch::commits(&kept)?, both(A));
    assert_eq!(Scratch::screens(&screens)?, format!("<html>{B}</html>"));
    assert_eq!(scratch.record()?["binaries"]["lys-identity-server"], B);
    assert_eq!(scratch.untouchable()?, before, "data and configuration untouched");
    Ok(())
}

#[test]
fn with_nothing_kept_back_is_refused_and_stops_nothing() -> TestResult {
    let scratch = Scratch::new()?;
    let pids = scratch.pids()?;
    let refused = back(&scratch.layout, &scratch.units, &mut |_| {})
        .err()
        .ok_or("back ran with no kept build")?;
    assert_eq!(refused.kind(), ErrorKind::NothingToReturnTo);
    assert!(refused.to_string().starts_with("nothing_to_return_to: "), "{refused}");
    assert_eq!(scratch.pids()?, pids, "nothing was stopped or started");
    assert_eq!(scratch.running()?, ready_lines(A));
    assert!(!scratch.layout.bin_previous_dir().exists());
    Ok(())
}

#[test]
fn a_start_failure_during_back_leaves_the_build_it_began_from_running() -> TestResult {
    let scratch = Scratch::new()?;
    scratch.upgrade_to_b(&[])?;
    let kept = scratch.layout.bin_previous_dir();
    stub(&kept, BINARIES[1], A, Behaviour::ExitsEarly, &[])?;
    let mut said = Vec::new();
    let mut hear = |line: &str| said.push(line.to_string());
    let refused = back(&scratch.layout, &scratch.units, &mut hear)
        .err()
        .ok_or("a kept service that exited was taken as ready")?;
    assert_eq!(refused.kind(), ErrorKind::UpgradeFailed);
    let message = refused.to_string();
    assert!(message.contains("lys-identity-server did not start ready"), "{message}");
    assert!(message.contains(&scratch.units[1].log.display().to_string()), "{message}");
    assert!(message.contains("the build it began from is back and running"), "{message}");
    assert!(said.contains(&"putting the build it began from back".to_string()), "{said:?}");
    assert_eq!(scratch.running()?, ready_lines(B), "B runs again");
    let service_log = std::fs::read_to_string(&scratch.units[1].log)?;
    assert!(service_log.contains(&format!("lys-identity-server {A} failing")));
    assert_eq!(Scratch::commits(&scratch.layout.bin_dir())?, both(B));
    assert_eq!(Scratch::commits(&kept)?, both(A), "A is still the one kept");
    let screens = scratch.layout.surface_dir();
    assert_eq!(Scratch::screens(&screens)?, format!("<html>{B}</html>"));
    assert_eq!(scratch.record()?["binaries"]["lys-identity-server"], B);
    Ok(())
}

#[test]
fn back_is_refused_when_the_kept_build_would_not_read_the_data() -> TestResult {
    let scratch = Scratch::new()?;
    let format_two = format!("data format 2 adds {NEW_KIND}");
    scratch.upgrade_to_b(&["data format 1", &format_two])?;
    assert_eq!(scratch.record()?["data_format"], 2);
    let pids = scratch.pids()?;
    let refused = back(&scratch.layout, &scratch.units, &mut |_| {})
        .err()
        .ok_or("back ran to a build that does not know the data")?;
    assert_eq!(refused.kind(), ErrorKind::BackWouldNotRead);
    let message = refused.to_string();
    assert!(message.starts_with("back_would_not_read: "), "{message}");
    assert!(message.contains(&format!("`{NEW_KIND}`")), "{message}");
    assert_eq!(scratch.pids()?, pids, "nothing was stopped or started");
    assert_eq!(scratch.running()?, ready_lines(B));
    assert_eq!(Scratch::commits(&scratch.layout.bin_dir())?, both(B));
    // The same kept commit, now knowing format 2: the refusal was the
    // format's alone, and back runs.
    let kept = scratch.layout.bin_previous_dir();
    stub(&kept, BINARIES[1], A, Behaviour::Serves, &[&format_two])?;
    back(&scratch.layout, &scratch.units, &mut |_| {})?;
    assert_eq!(scratch.running()?, ready_lines(A));
    assert_eq!(scratch.record()?["data_format"], 2, "data is never rolled back");
    Ok(())
}

#[test]
fn the_data_format_outlives_the_build_that_wrote_it() -> TestResult {
    let scratch = Scratch::new()?;
    let format_two = format!("data format 2 adds {NEW_KIND}");
    scratch.upgrade_to_b(&[&format_two])?;
    // Build C states no format; running it cannot make format 2 data unread.
    let c = scratch.work().join("c");
    build(&c, A, Behaviour::Serves, &[])?;
    upgrade(&scratch.layout, &c, None, &scratch.units, &mut |_| {})?;
    assert_eq!(scratch.record()?["data_format"], 2);
    let kept = scratch.layout.bin_previous_dir();
    stub(&kept, BINARIES[1], B, Behaviour::Serves, &[])?;
    let refused = back(&scratch.layout, &scratch.units, &mut |_| {});
    let refused = refused.err().ok_or("back ignored the recorded format")?;
    assert_eq!(refused.kind(), ErrorKind::BackWouldNotRead);
    assert!(refused.to_string().contains("of data format 2"), "{refused}");
    Ok(())
}

#[test]
fn an_unfinished_upgrade_or_exchange_is_refused_before_anything_stops() -> TestResult {
    let scratch = Scratch::new()?;
    scratch.upgrade_to_b(&[])?;
    let pids = scratch.pids()?;
    let intent = scratch.layout.install_dir().join("upgrade.json");
    std::fs::write(&intent, "{}")?;
    let new = scratch.work().join("b");
    let mut refusals = vec![
        back(&scratch.layout, &scratch.units, &mut |_| {}).err(),
        upgrade(&scratch.layout, &new, None, &scratch.units, &mut |_| {}).err(),
    ];
    std::fs::remove_file(&intent)?;
    let aside = scratch.layout.root.join("bin.exchanging");
    std::fs::create_dir_all(&aside)?;
    refusals.push(back(&scratch.layout, &scratch.units, &mut |_| {}).err());
    let kinds: Vec<_> = refusals
        .iter()
        .map(|refused| refused.as_ref().map(IdentityError::kind))
        .collect();
    assert_eq!(kinds, [Some(ErrorKind::UpgradeUnfinished); 3]);
    assert!(aside.is_dir(), "a half-made exchange is never removed");
    assert_eq!(scratch.pids()?, pids, "nothing was stopped or started");
    assert_eq!(scratch.running()?, ready_lines(B));
    Ok(())
}

#[test]
fn a_version_detail_states_its_data_formats() -> TestResult {
    let dir = tempfile::tempdir()?;
    let name = BINARIES[1];
    stub(dir.path(), name, A, Behaviour::Serves, &[])?;
    let (commit, formats) = stated(&dir.path().join(name), name)?;
    assert_eq!(commit, A);
    assert!(formats.is_empty());
    assert_eq!(newest(&formats), BASELINE_FORMAT);
    let three = "data format 3 adds grant.moved, team.renamed";
    stub(dir.path(), name, A, Behaviour::Serves, &["data format 1", three])?;
    let (_, formats) = stated(&dir.path().join(name), name)?;
    assert_eq!(newest(&formats), 3);
    assert_eq!(formats.get(&1), Some(&Vec::new()));
    let added = ["grant.moved".to_string(), "team.renamed".to_string()];
    assert_eq!(formats.get(&3), Some(&added.to_vec()));
    stub(dir.path(), name, A, Behaviour::Serves, &["data format three"])?;
    let refused = stated(&dir.path().join(name), name);
    assert!(refused.is_err_and(|error| error.kind() == ErrorKind::VersionUnreadable));
    Ok(())
}

/// `lys identity` as the CLI parses it.
#[derive(Debug, Parser)]
struct Identity {
    #[command(subcommand)]
    command: IdentityCommand,
}

#[test]
fn upgrade_takes_exactly_one_of_from_and_back() {
    let parse = |args: &[&str]| {
        let words = ["identity", "upgrade"].iter().chain(args);
        Identity::try_parse_from(words).map(|cli| cli.command)
    };
    let Ok(IdentityCommand::Upgrade { from, surface, .. }) = parse(&["--back"]) else {
        panic!("--back did not parse as an upgrade");
    };
    assert!(from.back && from.folder.is_none() && surface.is_none());
    let Ok(IdentityCommand::Upgrade { from, .. }) = parse(&["--from", "dir"]) else {
        panic!("--from did not parse as an upgrade");
    };
    assert!(!from.back && from.folder.is_some());
    let refused = [
        parse(&[]),
        parse(&["--back", "--from", "dir"]),
        parse(&["--back", "--surface", "screens"]),
    ];
    assert_eq!(refused.iter().filter(|parsed| parsed.is_err()).count(), refused.len());
}

#[test]
fn the_new_refusals_have_their_names() {
    let named = [
        (ErrorKind::NothingToReturnTo, "nothing_to_return_to"),
        (ErrorKind::BackWouldNotRead, "back_would_not_read"),
        (ErrorKind::UpgradeUnfinished, "upgrade_unfinished"),
    ];
    for (kind, name) in named {
        assert_eq!(kind.name(), name);
    }
}
