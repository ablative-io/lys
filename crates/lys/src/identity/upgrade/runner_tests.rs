#![cfg(test)]

//! Runner upgrades preserve sessions by refusing while any remain live.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_runner::protocol::{Greeting, reply_line, verify_request};
use lys_runner::{Act, Answer, Client, Launch};

use super::super::scratch::{A, B, Behaviour, Fixed, Recorder, Scratch, TestResult, build};
use super::super::{Parts, Ready, Unit, upgrade, version};
use crate::commands::error::CliError;
use crate::identity::install::layout::{BINARIES, Layout};
use crate::identity::install::{self, services};

struct Running {
    scratch: Scratch,
    key: Arc<Ed25519Identity>,
}
#[derive(serde::Deserialize)]
struct FixtureUnit {
    binary: String,
    args: Vec<String>,
    log: PathBuf,
    pid: PathBuf,
}

impl Running {
    fn new() -> TestResult<Self> {
        let scratch = Scratch::new()?;
        for directory in [
            scratch.layout.run_dir(),
            scratch.layout.logs_dir(),
            scratch.layout.data_dir(),
        ] {
            std::fs::set_permissions(directory, std::fs::Permissions::from_mode(0o700))?;
        }
        let key = Arc::new(install::service_key(&scratch.layout)?);
        runner_binary(&scratch.layout.bin_dir(), A, false)?;
        let running = Self { scratch, key };
        install::start_runner(
            &running.scratch.layout,
            &running.key,
            &running.scratch.layout.binary("lys"),
            &mut |_| {},
        )?;
        Ok(running)
    }

    fn client(&self) -> Client {
        Client::new(self.scratch.layout.runner_socket(), Arc::clone(&self.key))
    }

    fn pid(&self) -> PathBuf {
        self.scratch.layout.run_dir().join("runner.pid")
    }

    fn new_build(&self, refuses: bool) -> TestResult<PathBuf> {
        let new = self.scratch.work().join("b");
        build(&new, B, Behaviour::Serves)?;
        runner_binary(&new, B, refuses)?;
        Ok(new)
    }

    fn upgrade(&self, from: &Path, said: &mut Vec<String>) -> TestResult {
        let program = from.join("lys");
        let mut parts = Parts {
            runner: Some(&program),
            units: &self.scratch.units,
            engine: &mut Recorder::default(),
            render: &Fixed,
        };
        upgrade(&self.scratch.layout, from, None, &mut parts, &mut |line| {
            said.push(line.to_owned());
        })?;
        Ok(())
    }

    fn keys(&self) -> TestResult<Vec<Vec<u8>>> {
        Ok(vec![
            std::fs::read(self.scratch.layout.service_key())?,
            std::fs::read(self.scratch.layout.run_dir().join("runner-server.pub"))?,
        ])
    }

    fn unchanged(&self) -> TestResult<BTreeMap<PathBuf, Vec<u8>>> {
        let mut files = self.scratch.untouchable()?;
        files.extend(self.scratch.configuration()?);
        for path in [
            self.pid(),
            self.scratch.layout.build_record(),
            self.scratch.layout.service_key(),
            self.scratch.layout.run_dir().join("runner-server.pub"),
        ]
        .into_iter()
        .chain(self.scratch.units.iter().map(|unit| unit.pid.clone()))
        .chain(
            BINARIES
                .into_iter()
                .chain(["lys"])
                .map(|name| self.scratch.layout.binary(name)),
        ) {
            files.insert(path.clone(), std::fs::read(path)?);
        }
        Ok(files)
    }

    fn command(&self, from: &Path) -> TestResult<Output> {
        let fixture = self.scratch.work().join("command.json");
        let units: Vec<_> = self
            .scratch
            .units
            .iter()
            .map(|unit| {
                serde_json::json!({
                    "binary": unit.binary, "args": unit.args,
                    "log": unit.log, "pid": unit.pid,
                })
            })
            .collect();
        std::fs::write(&fixture, serde_json::to_vec(&units)?)?;
        Ok(Command::new(std::env::current_exe()?)
            .args([
                "--exact",
                "identity::upgrade::tests::runner::upgrade_command_child",
                "--nocapture",
            ])
            .env("LYS_UPGRADE_TEST_ROOT", &self.scratch.layout.root)
            .env("LYS_UPGRADE_TEST_FROM", from)
            .env("LYS_UPGRADE_TEST_UNITS", fixture)
            .output()?)
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        if let Err(error) = services::stop(&self.pid()) {
            eprintln!("runner cleanup failed: {error}");
        }
    }
}

fn runner_binary(dir: &Path, commit: &str, refuses: bool) -> TestResult {
    let program = std::env::current_exe()?
        .parent()
        .and_then(Path::parent)
        .ok_or("test executable lacks its build directory")?
        .join("lys");
    if !program.is_file() {
        return Err("the gate must build the lys binary before the runner tests".into());
    }
    let body = if refuses {
        "echo 'runner_already_running: replacement refused' >&2\nexit 3\n".to_owned()
    } else {
        let quoted = program.display().to_string().replace('\'', "'\\''");
        format!("exec '{quoted}' \"$@\"\n")
    };
    let file = dir.join("lys");
    std::fs::write(
        &file,
        format!(
            "#!/bin/sh\nif [ \"$1\" = --version ]; then echo 'lys 0.2.0 ({commit})'; exit 0; fi\n{body}"
        ),
    )?;
    std::fs::set_permissions(file, std::fs::Permissions::from_mode(0o755))?;
    Ok(())
}

#[test]
fn upgrade_places_the_new_lys_and_restarts_the_runner_on_it() -> TestResult {
    let running = Running::new()?;
    let new = running.new_build(false)?;
    let pid = std::fs::read(running.pid())?;
    let keys = running.keys()?;
    running.upgrade(&new, &mut Vec::new())?;
    assert_eq!(version(&running.scratch.layout.binary("lys"), "lys")?, B);
    assert_eq!(
        version(
            &running.scratch.layout.bin_previous_dir().join("lys"),
            "lys"
        )?,
        A
    );
    assert_ne!(std::fs::read(running.pid())?, pid);
    assert_eq!(running.keys()?, keys);
    assert!(matches!(
        running.client().ask(&Act::Status { session: None })?,
        Answer::Status { .. }
    ));
    let record: serde_json::Value =
        serde_json::from_slice(&std::fs::read(running.scratch.layout.build_record())?)?;
    assert_eq!(record["binaries"]["lys"], B);
    Ok(())
}

#[test]
fn runner_start_refusal_is_nonzero_named_and_never_reported_started() -> TestResult {
    let running = Running::new()?;
    let new = running.new_build(true)?;
    let keys = running.keys()?;
    let output = running.command(&new)?;
    let out = String::from_utf8(output.stdout)?;
    let err = String::from_utf8(output.stderr)?;
    assert_eq!(output.status.code(), Some(1), "{out}\n{err}");
    assert!(
        err.contains("upgrade_failed") && err.contains("runner_already_running"),
        "{err}"
    );
    assert!(!out.contains("runner started"), "{out}");
    assert_eq!(version(&running.scratch.layout.binary("lys"), "lys")?, A);
    assert_eq!(running.keys()?, keys);
    assert!(matches!(
        running.client().ask(&Act::Status { session: None })?,
        Answer::Status { .. }
    ));
    Ok(())
}

#[test]
fn an_upgrade_refuses_live_sessions_and_later_restarts_without_losing_keys() -> TestResult {
    let running = Running::new()?;
    let new = running.new_build(false)?;
    let keys = running.keys()?;
    for session in ["held-one", "held-two"] {
        let launch = Launch {
            session: session.into(),
            program: "/bin/cat".into(),
            arguments: vec![],
            directory: running.scratch.work().display().to_string(),
            environment: BTreeMap::new(),
            config: None,
            columns: 80,
            rows: 24,
            rotation: None,
            policy: None,
        };
        assert!(matches!(
            running.client().ask(&Act::Start {
                lys_mcp: None,
                launch: Box::new(launch)
            })?,
            Answer::Started { .. }
        ));
    }
    let before = running.unchanged()?;
    let pid = std::fs::read(running.pid())?;
    let mut said = Vec::new();
    let error = running
        .upgrade(&new, &mut said)
        .err()
        .ok_or("a live runner was upgraded")?;
    let named = error.to_string();
    assert!(named.contains("runner_sessions_live"), "{named}");
    for session in ["held-one", "held-two"] {
        assert!(named.contains(session), "{named}");
    }
    assert_eq!(running.unchanged()?, before);
    assert!(said.is_empty(), "{said:?}");
    let Answer::Status { status } = running.client().ask(&Act::Status { session: None })? else {
        return Err("the old runner stopped answering Status".into());
    };
    assert_eq!(status.sessions.len(), 2);
    assert!(
        status
            .sessions
            .iter()
            .all(|session| session.ended.is_none())
    );
    for session in ["held-one", "held-two"] {
        assert!(matches!(
            running.client().ask(&Act::End {
                session: session.into()
            })?,
            Answer::Ended { .. }
        ));
    }
    running.upgrade(&new, &mut Vec::new())?;
    assert_ne!(std::fs::read(running.pid())?, pid);
    assert_eq!(running.keys()?, keys);
    assert_eq!(version(&running.scratch.layout.binary("lys"), "lys")?, B);
    Ok(())
}

#[test]
fn unreadable_status_refuses_before_changing_any_file_or_process() -> TestResult {
    for answer in [None, Some("refused"), Some("malformed")] {
        let running = Running::new()?;
        let new = running.new_build(true)?;
        services::stop(&running.pid())?;
        let before = running.unchanged()?;
        let replying = answer
            .map(|answer| status_reply(&running, answer))
            .transpose()?;
        let mut said = Vec::new();
        let result = running.upgrade(&new, &mut said);
        if let Some(replying) = replying {
            replying
                .join()
                .map_err(|panic| format!("Status fixture panicked: {panic:?}"))??;
            std::fs::remove_file(running.scratch.layout.runner_socket())?;
        }
        let error = result
            .err()
            .ok_or("an unreadable Status was taken as empty")?;
        let named = error.to_string();
        assert!(named.contains("runner_status_unreadable"), "{named}");
        assert_eq!(running.unchanged()?, before);
        assert!(said.is_empty(), "{said:?}");
        assert!(!running.scratch.layout.bin_previous_dir().exists());
        assert!(!running.scratch.layout.upgrade_intent().exists());
    }
    Ok(())
}

fn status_reply(
    running: &Running,
    answer: &str,
) -> TestResult<std::thread::JoinHandle<std::io::Result<()>>> {
    let listener = UnixListener::bind(running.scratch.layout.runner_socket())?;
    let key = running.key.public_key_bytes();
    let reply = if answer == "refused" {
        reply_line(Answer::Refused {
            refusal: "runner_state_unavailable".into(),
            words: "Status refused".into(),
            oldest: None,
        })
    } else {
        "{not a Status}".into()
    };
    Ok(std::thread::spawn(move || {
        let (mut stream, _) = listener.accept()?;
        let greeting = Greeting::fresh("aa");
        writeln!(stream, "{}", greeting.line())?;
        let mut request = String::new();
        BufReader::new(stream.try_clone()?).read_line(&mut request)?;
        let act = verify_request(&request, &key, &greeting).map_err(std::io::Error::other)?;
        assert_eq!(act, Act::Status { session: None });
        writeln!(stream, "{reply}")?;
        Ok(())
    }))
}

#[test]
fn upgrade_command_child() -> TestResult {
    let Some(root) = std::env::var_os("LYS_UPGRADE_TEST_ROOT") else {
        return Ok(());
    };
    let from = PathBuf::from(std::env::var_os("LYS_UPGRADE_TEST_FROM").ok_or("missing build")?);
    let file = std::env::var_os("LYS_UPGRADE_TEST_UNITS").ok_or("missing units")?;
    let fixture: Vec<FixtureUnit> = serde_json::from_slice(&std::fs::read(file)?)?;
    let units = fixture
        .into_iter()
        .map(|unit| {
            let binary = BINARIES
                .into_iter()
                .find(|name| *name == unit.binary)
                .ok_or("unknown unit")?;
            Ok(Unit {
                binary,
                args: unit.args,
                log: unit.log,
                pid: unit.pid,
                ready: Ready {
                    says: "ready".into(),
                    answers: None,
                },
            })
        })
        .collect::<TestResult<Vec<_>>>()?;
    let program = from.join("lys");
    let mut parts = Parts {
        runner: Some(&program),
        units: &units,
        engine: &mut Recorder::default(),
        render: &Fixed,
    };
    let result = upgrade(
        &Layout::at(PathBuf::from(root)),
        &from,
        None,
        &mut parts,
        &mut |line| println!("{line}"),
    )
    .map_err(CliError::from);
    match result {
        Ok(_) => std::process::exit(0),
        Err(error) => {
            eprintln!("error: {error}");
            std::process::exit(1);
        }
    }
}

#[test]
fn recovery_after_lys_is_placed_restores_the_runner_starts_a_stopped_one_or_refuses_live_sessions()
-> TestResult {
    use super::super::intent::{Intent, Step};
    use super::super::{launch, swap};

    for state in ["empty", "live", "down"] {
        let running = Running::new()?;
        let new = running.new_build(false)?;
        let layout = &running.scratch.layout;
        let keys = running.keys()?;
        services::stop(&running.pid())?;
        swap::stop_all(&running.scratch.units, &mut |_| {})?;
        let mut intent = Intent {
            from: BINARIES
                .into_iter()
                .chain(["lys"])
                .map(|name| (name.into(), A.into()))
                .collect(),
            to: BINARIES
                .into_iter()
                .chain(["lys"])
                .map(|name| (name.into(), B.into()))
                .collect(),
            screens: false,
            screens_existed: true,
            files: Vec::new(),
            compose_changed: false,
            steps: vec![Step::Stopped],
        };
        intent.write(layout)?;
        swap::keep_binaries(layout, &mut intent)?;
        swap::place_binaries(
            layout,
            &new,
            &[BINARIES[0], BINARIES[1], "lys"],
            &mut intent,
        )?;
        for unit in &running.scratch.units {
            launch(layout, unit, false)?;
        }
        if state != "down" {
            install::start_runner(layout, &running.key, &layout.binary("lys"), &mut |_| {})?;
        }
        if state == "live" {
            let launch = Launch {
                session: "recovery-held".into(),
                program: "/bin/cat".into(),
                arguments: vec![],
                directory: running.scratch.work().display().to_string(),
                environment: BTreeMap::new(),
                config: None,
                columns: 80,
                rows: 24,
                rotation: None,
                policy: None,
            };
            assert!(matches!(
                running.client().ask(&Act::Start {
                    lys_mcp: None,
                    launch: Box::new(launch)
                })?,
                Answer::Started { .. }
            ));
        }
        let before = running.unchanged()?;
        let pid = std::fs::read(running.pid())?;
        let recorded = std::fs::read(layout.upgrade_intent())?;
        let result = swap::recover(
            layout,
            &running.scratch.units,
            &mut Recorder::default(),
            &mut |_| {},
        );
        if state == "live" {
            let error = result.err().ok_or("recovery acted on live sessions")?;
            let named = error.to_string();
            assert!(named.contains("runner_sessions_live"), "{named}");
            assert!(named.contains("recovery-held"), "{named}");
            assert_eq!(running.unchanged()?, before);
            assert_eq!(std::fs::read(layout.upgrade_intent())?, recorded);
        } else {
            result?;
            assert_eq!(version(&layout.binary("lys"), "lys")?, A);
            assert_ne!(
                std::fs::read(running.pid())?,
                pid,
                "recovery kept the replacement runner"
            );
            assert!(matches!(
                running.client().ask(&Act::Status { session: None })?,
                Answer::Status { .. }
            ));
            assert!(!layout.upgrade_intent().exists());
        }
        assert_eq!(running.keys()?, keys);
    }
    Ok(())
}
