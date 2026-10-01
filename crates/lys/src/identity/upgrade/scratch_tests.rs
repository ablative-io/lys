#![cfg(test)]

//! A scratch install for the upgrade tests: build A installed and running as
//! two stub binaries, with its configuration and compose files, credentials
//! and data. The stubs answer `--version` as a Lys binary does, write their
//! configuration and `NAME COMMIT ready` to their log once started, and then
//! block on a pipe nobody writes until they are stopped. Each build renders
//! its own files: build B adds a configuration key and a compose
//! environment value that A does not have.

use std::collections::BTreeMap;
use std::error::Error;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use super::render::{Render, RenderedFile};
use super::{BuildRecord, Engine, Parts, Ready, Unit, adopt, launch, record_build, upgrade};
use crate::identity::error::IdentityResult;
use crate::identity::install::layout::{BINARIES, Layout};
use crate::identity::install::services;

/// A test's result.
pub type TestResult<T = ()> = Result<T, Box<dyn Error>>;

fn create_pipe(path: &Path) -> TestResult {
    let mode = nix::sys::stat::Mode::from_bits_truncate(0o666);
    nix::unistd::mkfifo(path, mode)
        .map_err(|error| format!("upgrade_fixture_pipe_create_failed: {error}").into())
}

/// Build A's commit.
pub const A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
/// Build B's commit.
pub const B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

/// The configuration key only build B renders.
pub const NEW_KEY: &str = "\"new_key\": \"b-only\"";
/// The compose environment value only build B renders.
pub const NEW_VALUE: &str = "NEW_VALUE=b";

/// How a stub behaves.
#[derive(Clone, Copy)]
pub enum Behaviour {
    /// Says ready and keeps running.
    Serves,
    /// Exits before it is ready.
    ExitsEarly,
}

/// Writes the stub `name` answering `--version` with `commit` into `dir`.
pub fn stub(dir: &Path, name: &str, commit: &str, behaviour: Behaviour) -> TestResult {
    std::fs::create_dir_all(dir)?;
    let body = match behaviour {
        Behaviour::Serves => format!(
            "if [ -n \"$2\" ]; then echo \"configured: $(cat \"$2\")\"; fi\n\
             echo \"{name} {commit} ready\"\nexec cat \"$1\"\n"
        ),
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

/// A build: the broker serving, the service as `service` says.
pub fn build(dir: &Path, commit: &str, service: Behaviour) -> TestResult {
    stub(dir, BINARIES[0], commit, Behaviour::Serves)?;
    stub(dir, BINARIES[1], commit, service)
}

/// A screens package from `commit`.
pub fn package(dir: &Path, commit: &str) -> TestResult {
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

/// The scratch install's state directory, where credentials live.
pub fn state(layout: &Layout) -> PathBuf {
    layout.root.join("state")
}

fn file(
    name: &'static str,
    target: PathBuf,
    text: &str,
    private: bool,
    compose: bool,
) -> RenderedFile {
    RenderedFile {
        name,
        target,
        bytes: Zeroizing::new(text.as_bytes().to_vec()),
        private,
        compose,
    }
}

/// The files build `commit` renders.
pub fn files_for(layout: &Layout, commit: &str, screens: bool) -> Vec<RenderedFile> {
    let (key, value) = if commit == B {
        (format!(", {NEW_KEY}"), format!("{NEW_VALUE}\n"))
    } else {
        (String::new(), String::new())
    };
    let service = format!("{{\"built_for\": \"{commit}\", \"screens\": {screens}{key}}}\n");
    vec![
        file(
            "compose.yaml",
            layout.deploy_dir().join("compose.yaml"),
            "services: {}\n",
            false,
            true,
        ),
        file(
            "compose.env",
            state(layout).join("compose.env"),
            &format!("PLAIN=1\n{value}"),
            true,
            true,
        ),
        file(
            "identity.json",
            layout.service_config(),
            &service,
            true,
            false,
        ),
    ]
}

/// Each build's own templates, chosen by the service's commit.
pub struct Fixed;

impl Render for Fixed {
    fn render(
        &self,
        layout: &Layout,
        build: &BTreeMap<String, String>,
        screens: bool,
    ) -> IdentityResult<Vec<RenderedFile>> {
        let commit = build.get(BINARIES[1]).map_or("", String::as_str);
        Ok(files_for(layout, commit, screens))
    }
}

/// An engine that records the compose environment it was applied with.
#[derive(Default)]
pub struct Recorder {
    /// The compose environment at each application, in order.
    pub applied: Vec<String>,
}

impl Engine for Recorder {
    fn apply(&mut self, layout: &Layout, say: &mut dyn FnMut(&str)) -> IdentityResult<()> {
        let environment = std::fs::read_to_string(state(layout).join("compose.env"));
        self.applied.push(environment.unwrap_or_default());
        say("compose applied");
        Ok(())
    }
}

/// A scratch install and the units that run it.
pub struct Scratch {
    /// Holds the root and the work folder.
    pub dir: tempfile::TempDir,
    /// The install's root.
    pub layout: Layout,
    /// The broker and the service.
    pub units: Vec<Unit>,
}

impl Scratch {
    /// The root laid out with A's files, credentials and data, and the
    /// units, with nothing in `bin/` and nothing running.
    pub fn laid_out() -> TestResult<Self> {
        let dir = tempfile::tempdir()?;
        let layout = Layout::at(dir.path().join("root"));
        let work = dir.path().join("work");
        for made in [
            layout.logs_dir(),
            layout.run_dir(),
            layout.deploy_dir(),
            layout.data_dir().join("directory"),
            state(&layout),
            work.clone(),
        ] {
            std::fs::create_dir_all(made)?;
        }
        std::fs::write(layout.deployment_config(), "[deployment]\nadmin = 1\n")?;
        std::fs::write(state(&layout).join("rauthy-admin-password"), "test-only-1")?;
        std::fs::write(state(&layout).join("spicedb-preshared-key"), "test-only-2")?;
        std::fs::write(layout.data_dir().join("directory").join("log"), [7_u8; 64])?;
        for rendered in files_for(&layout, A, true) {
            std::fs::write(&rendered.target, rendered.bytes.as_slice())?;
        }
        let hold = work.join("hold");
        let made = create_pipe(&hold);
        assert!(made.is_ok(), "mkfifo failed: {made:?}");
        made?;
        let units = BINARIES
            .into_iter()
            .zip(["secrets", "identity"])
            .map(|(binary, file)| {
                let mut args = vec![hold.display().to_string()];
                if file == "identity" {
                    args.push(layout.service_config().display().to_string());
                }
                Unit {
                    binary,
                    args,
                    log: layout.logs_dir().join(format!("{file}.log")),
                    pid: layout.run_dir().join(format!("{file}.pid")),
                    ready: Ready {
                        says: "ready".to_string(),
                        answers: None,
                    },
                }
            })
            .collect();
        Ok(Self { dir, layout, units })
    }

    /// Build A installed as this card installs it: in `bin/`, started with
    /// exit locks and its build recorded.
    pub fn new() -> TestResult<Self> {
        let scratch = Self::laid_out()?;
        build(&scratch.layout.bin_dir(), A, Behaviour::Serves)?;
        package(&scratch.layout.surface_dir(), A)?;
        for unit in &scratch.units {
            assert!(launch(&scratch.layout, unit, false)?, "A did not start");
        }
        record_build(&scratch.layout, &BINARIES, &mut |_| {})?;
        Ok(scratch)
    }

    /// Where the new builds are made, outside the root.
    pub fn work(&self) -> PathBuf {
        self.dir.path().join("work")
    }

    /// Every file under `data/`, every credential and `deployment.toml`.
    pub fn untouchable(&self) -> TestResult<BTreeMap<PathBuf, Vec<u8>>> {
        let mut files = BTreeMap::new();
        let mut pending = vec![self.layout.data_dir()];
        while let Some(dir) = pending.pop() {
            for entry in std::fs::read_dir(dir)? {
                let path = entry?.path();
                if path.is_dir() {
                    pending.push(path);
                } else {
                    files.insert(path.clone(), std::fs::read(&path)?);
                }
            }
        }
        for name in ["rauthy-admin-password", "spicedb-preshared-key"] {
            let path = state(&self.layout).join(name);
            files.insert(path.clone(), std::fs::read(&path)?);
        }
        let path = self.layout.deployment_config();
        files.insert(path.clone(), std::fs::read(&path)?);
        Ok(files)
    }

    /// The configuration and compose files, by path.
    pub fn configuration(&self) -> TestResult<BTreeMap<PathBuf, Vec<u8>>> {
        let mut files = BTreeMap::new();
        for rendered in files_for(&self.layout, A, true) {
            let bytes = std::fs::read(&rendered.target)?;
            files.insert(rendered.target, bytes);
        }
        Ok(files)
    }

    /// Each unit's last `NAME COMMIT ready` line, each unit running.
    pub fn running(&self) -> TestResult<Vec<String>> {
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

    /// Upgrades to the build in `from`, rendering with [`Fixed`] and
    /// applying compose through `engine`; `said` gathers every line.
    pub fn upgrade(
        &self,
        from: &Path,
        package: Option<&Path>,
        engine: &mut Recorder,
        said: &mut Vec<String>,
    ) -> IdentityResult<BuildRecord> {
        let mut parts = Parts {
            runner: None,
            units: &self.units,
            engine,
            render: &Fixed,
        };
        upgrade(&self.layout, from, package, &mut parts, &mut |line| {
            said.push(line.to_string());
        })
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        for unit in &self.units {
            if let Err(error) = adopt::stop(unit, &mut |_| {}) {
                eprintln!("stopping {} after the test: {error}", unit.binary);
            }
        }
    }
}

/// The ready line of every binary of build `commit`.
pub fn ready_lines(commit: &str) -> Vec<String> {
    BINARIES
        .iter()
        .map(|name| format!("{name} {commit} ready"))
        .collect()
}

#[test]
fn upgrade_fixture_pipe_needs_no_command_on_path() -> TestResult {
    let output = Command::new(std::env::current_exe()?)
        .args([
            "--exact",
            "identity::upgrade::scratch::upgrade_fixture_pipe_child",
            "--nocapture",
        ])
        .env("PATH", "")
        .env("LYS_UPGRADE_PIPE_CHILD", "1")
        .output()?;
    assert!(
        output.status.success(),
        "pipe fixture child failed: {} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("upgrade_fixture_pipe_proved"));
    Ok(())
}

#[test]
fn upgrade_fixture_pipe_child() -> TestResult {
    use std::io::{Read, Write};
    use std::os::fd::AsRawFd;
    use std::os::unix::fs::{FileTypeExt, OpenOptionsExt};

    use nix::fcntl::{FcntlArg, OFlag, fcntl};

    if std::env::var_os("LYS_UPGRADE_PIPE_CHILD").is_none() {
        return Ok(());
    }
    let scratch = Scratch::laid_out()?;
    let path = scratch.work().join("hold");
    assert!(std::fs::metadata(&path)?.file_type().is_fifo());
    let mut pipe = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(OFlag::O_NONBLOCK.bits())
        .open(&path)?;
    let mut writer = std::fs::OpenOptions::new().write(true).open(path)?;
    let mut byte = [0];
    let empty = pipe
        .read(&mut byte)
        .err()
        .ok_or("the empty FIFO did not wait for a writer")?;
    assert_eq!(empty.kind(), std::io::ErrorKind::WouldBlock);
    fcntl(pipe.as_raw_fd(), FcntlArg::F_SETFL(OFlag::empty()))?;
    let reader = std::thread::spawn(move || {
        pipe.read_exact(&mut byte)?;
        Ok::<_, std::io::Error>(byte)
    });
    let sent = writer.write_all(b"x");
    drop(writer);
    let read = reader.join();
    sent?;
    let received = read.map_err(|payload| {
        let detail = if let Some(message) = payload.downcast_ref::<&str>() {
            (*message).to_owned()
        } else if let Some(message) = payload.downcast_ref::<String>() {
            message.clone()
        } else {
            format!(
                "non-string panic payload ({:?})",
                payload.as_ref().type_id()
            )
        };
        format!("upgrade_fixture_pipe_reader_panicked: {detail}")
    })??;
    assert_eq!(received, *b"x");
    println!("upgrade_fixture_pipe_proved");
    Ok(())
}

#[test]
fn upgrade_fixture_pipe_refuses_to_replace_an_existing_file() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("hold");
    std::fs::write(&path, b"retained")?;
    let error = create_pipe(&path)
        .err()
        .ok_or("pipe creation replaced an existing file")?;
    assert!(
        error
            .to_string()
            .contains("upgrade_fixture_pipe_create_failed")
    );
    assert_eq!(std::fs::read(path)?, b"retained");
    Ok(())
}
