//! `lys identity upgrade`: a running install moved to a newer build, with
//! the previous build kept to return to.
//!
//! The install runs the secrets broker and the directory service from its
//! own `bin/`. An upgrade reads every binary's `--version`, the installed
//! and the new, before it stops anything, and refuses by name when the root
//! holds no install, when the folder lacks a binary, or when a version
//! cannot be read. It then stops the service and the broker, each waited on
//! through its exit lock ([`super::install::exit_wait`]); moves `bin/` to
//! `bin.previous/`, replacing an older one; places each new binary by a
//! copy to a temporary name and a rename; places a screens package through
//! the install's own verify-and-place path, keeping the previous screens in
//! `surface.previous/`; and starts the broker and the service, each waited
//! on for ready on its readiness event.
//!
//! `--back` ([`back`]) returns to the build kept in `bin.previous/` by the
//! same stop, swap, start and ready path: it reads the version of every
//! binary in `bin/` and in `bin.previous/`, then exchanges `bin/` with
//! `bin.previous/` and the screens with `surface.previous/` when that
//! exists, so the build it leaves becomes the one kept and `--back` again
//! returns to it.
//!
//! Each build states, in its `--version` detail, the data formats it knows:
//! lines `data format N adds KIND, KIND` naming the log event kinds each
//! format added; a build stating none predates the numbering and reads as
//! format [`BASELINE_FORMAT`]. `install/build.json` records the newest
//! format the data may hold: the highest any build recorded there states.
//!
//! Invariants: nothing is stopped until every input has been read and
//! checked, and `--back` is refused `back_would_not_read`, naming the first
//! event kind the kept build does not know, when the data's format is newer
//! than the kept build's: data is never rolled back. Neither runs over an
//! unfinished upgrade's intent record (`install/upgrade.json`), and
//! `--back` never runs over a half-made exchange; each is refused by name
//! before anything stops. When a start or a readiness
//! fails, what was started is stopped, the build it began from (and its
//! screens) is put back, started and waited on for ready, and the command
//! fails naming the binary and its log. An upgrade writes only `bin/`,
//! `bin.previous/`, the screens, the logs, the process files and
//! `install/build.json`: never `data/`, a credential, `deployment.toml`,
//! `identity.json` or the compose services.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde::Serialize;
use sha2::{Digest, Sha256};

use super::cli::Source;
use super::error::{ErrorKind, IdentityError, IdentityResult};
use super::install::layout::{BINARIES, BROKER_PORT, Layout, SERVICE_PORT};
use super::install::{services, surface};
use super::private_files;
use crate::commands::output::Emitter;

mod back;

pub use back::back;

/// What the operator chose.
#[derive(Debug)]
pub struct Options {
    /// The folder holding the newly built binaries, or back to the kept build.
    pub from: Source,
    /// A compiled screens package to verify and place.
    pub surface: Option<PathBuf>,
    /// The data root; the platform's application data path when absent.
    pub root: Option<PathBuf>,
}

/// How a started process is known to be ready: its log gains `says` after
/// it starts, and then, when `answers` names a loopback port and path, that
/// port answers the path. Both are checked once on each change to the log.
#[derive(Debug, Clone)]
pub struct Ready {
    /// The text the process writes once it is listening.
    pub says: String,
    /// The loopback port and path that answer once it is ready.
    pub answers: Option<(u16, &'static str)>,
}

/// One process the install runs from its `bin/`.
#[derive(Debug, Clone)]
pub struct Unit {
    /// The binary's name in `bin/`.
    pub binary: &'static str,
    /// Its arguments.
    pub args: Vec<String>,
    /// Where its output is appended.
    pub log: PathBuf,
    /// Where its pid is kept, its exit lock beside it.
    pub pid: PathBuf,
    /// How it is known to be ready.
    pub ready: Ready,
}

/// The broker and the service as the install runs them, in start order.
pub fn units(layout: &Layout) -> Vec<Unit> {
    let broker_args = [
        "serve",
        "--root",
        &layout.broker_root().display().to_string(),
        "--keys",
        &layout.broker_keys().display().to_string(),
        "--listen",
        &format!("127.0.0.1:{BROKER_PORT}"),
        "--directory-config",
        &layout.service_config().display().to_string(),
    ]
    .map(str::to_string);
    vec![
        Unit {
            binary: BINARIES[0],
            args: broker_args.to_vec(),
            log: layout.logs_dir().join("secrets.log"),
            pid: layout.run_dir().join("secrets.pid"),
            ready: Ready {
                says: "listening on".to_string(),
                answers: Some((BROKER_PORT, "/")),
            },
        },
        Unit {
            binary: BINARIES[1],
            args: vec![layout.service_config().display().to_string()],
            log: layout.logs_dir().join("identity.log"),
            pid: layout.run_dir().join("identity.pid"),
            ready: Ready {
                says: "listening on".to_string(),
                answers: Some((SERVICE_PORT, "/api/authority")),
            },
        },
    ]
}

fn refuse(
    kind: ErrorKind,
    action: &'static str,
    resource: &str,
    detail: impl Into<String>,
) -> IdentityError {
    IdentityError::new(kind, action, resource, detail)
}

fn io(action: &'static str, path: &Path, error: &std::io::Error) -> IdentityError {
    refuse(ErrorKind::PrivateFileIo, action, "binaries", error.to_string()).at(path)
}

/// Whether what `log` holds from byte `from` on contains `line`.
fn log_says(log: &Path, from: u64, line: &str) -> bool {
    std::fs::read(log).is_ok_and(|bytes| {
        let start = usize::try_from(from).unwrap_or(usize::MAX).min(bytes.len());
        String::from_utf8_lossy(&bytes[start..]).contains(line)
    })
}

/// Starts `unit` from `bin/` and waits for it ready. A process already
/// running is left alone unless `replace` asks for it to be started afresh,
/// and its whole log is read for its word. `true` when it was started.
pub fn launch(layout: &Layout, unit: &Unit, replace: bool) -> IdentityResult<bool> {
    let before = std::fs::metadata(&unit.log).map_or(0, |meta| meta.len());
    let program = layout.binary(unit.binary);
    let started = services::start_detached(&program, &unit.args, &unit.log, &unit.pid, replace)?;
    let from = if started { before } else { 0 };
    let ready = &unit.ready;
    services::wait_until(unit.binary, &unit.log, &unit.pid, &mut || {
        log_says(&unit.log, from, &ready.says)
            && ready
                .answers
                .is_none_or(|(port, path)| services::answers(port, path))
    })?;
    Ok(started)
}

/// Copies `source` into `dir` as `name`: to a temporary name first, then
/// renamed, so `dir/name` is never a half-written binary.
pub fn place_binary(source: &Path, dir: &Path, name: &str) -> IdentityResult<()> {
    let target = dir.join(name);
    let placing = dir.join(format!(".{name}.placing"));
    std::fs::copy(source, &placing).map_err(|error| io("place binary", &placing, &error))?;
    std::fs::rename(&placing, &target).map_err(|error| io("place binary", &target, &error))
}

/// The format a build that states no data format is read as: the first
/// numbered, which every build from before the numbering writes.
pub const BASELINE_FORMAT: u32 = 1;

/// Each data format a build states, with the log event kinds it added.
pub type Formats = BTreeMap<u32, Vec<String>>;

/// The newest format in `formats`, or [`BASELINE_FORMAT`] when it is empty.
pub fn newest(formats: &Formats) -> u32 {
    match formats.last_key_value() {
        Some((format, _)) => *format,
        None => BASELINE_FORMAT,
    }
}

/// Runs `program --version` and answers what its first line,
/// `name VERSION (COMMIT)`, holds in its parentheses (a commit, perhaps
/// with `; dirty`, or the words of a build with no commit), and the data
/// formats its `data format N adds KIND, KIND` lines state.
pub fn stated(program: &Path, name: &str) -> IdentityResult<(String, Formats)> {
    let unreadable = |detail: String| {
        refuse(ErrorKind::VersionUnreadable, "read version", name, detail).at(program)
    };
    let output = Command::new(program)
        .arg("--version")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|error| unreadable(format!("--version could not run: {error}")))?;
    if !output.status.success() {
        return Err(unreadable(format!("--version exited {}", output.status)));
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let first = text.lines().next().unwrap_or_default().trim();
    let commit = first
        .strip_prefix(&format!("{name} "))
        .and_then(|rest| rest.split_once(" ("))
        .and_then(|(_, inside)| inside.strip_suffix(')'))
        .filter(|commit| !commit.is_empty())
        .map(str::to_string)
        .ok_or_else(|| unreadable(format!("`{first}` is not `{name} VERSION (COMMIT)`")))?;
    let mut formats = Formats::new();
    for line in text.lines().skip(1) {
        let Some(declared) = line.trim().strip_prefix("data format ") else {
            continue;
        };
        let (number, kinds) = declared.split_once(" adds ").unwrap_or((declared, ""));
        let parsed = number.trim().parse::<u32>();
        let number = parsed.map_err(|error| {
            unreadable(format!("`data format {declared}` names no format: {error}"))
        })?;
        let known = formats.entry(number).or_default();
        for kind in kinds.split(',').map(str::trim) {
            if !kind.is_empty() {
                known.push(kind.to_string());
            }
        }
    }
    Ok((commit, formats))
}

/// The commit `program --version` names, as [`stated`] reads it.
pub fn version(program: &Path, name: &str) -> IdentityResult<String> {
    stated(program, name).map(|(commit, _)| commit)
}

/// The build an install is running, as `install/build.json` records it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BuildRecord {
    /// Each binary in `bin/` by name, with the commit its `--version` names.
    pub binaries: BTreeMap<String, String>,
    /// The placed screens, when there are any.
    pub surface: Option<SurfaceBuild>,
    /// The newest data format the data here may hold: the highest any
    /// build recorded here states.
    pub data_format: u32,
}

/// The placed screens package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SurfaceBuild {
    /// The commit the manifest names.
    pub commit: String,
    /// The SHA-256 of the placed manifest, lowercase hex.
    pub manifest_sha256: String,
}

/// Reads what `bin/` and the placed screens are, writes it to
/// `install/build.json` and says it.
pub fn record_build(
    layout: &Layout,
    names: &[&str],
    say: &mut dyn FnMut(&str),
) -> IdentityResult<BuildRecord> {
    let mut binaries = BTreeMap::new();
    let mut data_format = recorded_format(layout)?;
    for name in names {
        let (commit, formats) = stated(&layout.binary(name), name)?;
        say(&format!("build {name} {commit}"));
        binaries.insert((*name).to_string(), commit);
        data_format = data_format.max(newest(&formats));
    }
    say(&format!("build data format {data_format}"));
    let manifest_path = layout.surface_dir().join(surface::MANIFEST);
    let surface = match std::fs::read(&manifest_path) {
        Ok(bytes) => {
            let manifest: surface::Manifest = serde_json::from_slice(&bytes).map_err(|error| {
                refuse(
                    ErrorKind::ConfigInvalid,
                    "read screens",
                    "surface",
                    error.to_string(),
                )
                .at(&manifest_path)
            })?;
            let digest = format!("{:x}", Sha256::digest(&bytes));
            say(&format!(
                "build screens {} manifest sha256 {digest}",
                manifest.commit
            ));
            Some(SurfaceBuild {
                commit: manifest.commit,
                manifest_sha256: digest,
            })
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(io("read screens", &manifest_path, &error)),
    };
    let record = BuildRecord {
        binaries,
        surface,
        data_format,
    };
    let text = serde_json::to_vec_pretty(&record).map_err(|error| {
        refuse(
            ErrorKind::RenderFailed,
            "render",
            "build.json",
            error.to_string(),
        )
    })?;
    private_files::ensure_dir(&layout.install_dir())?;
    let path = layout.build_record();
    std::fs::write(&path, text).map_err(|error| io("write build record", &path, &error))?;
    Ok(record)
}

/// The data format `install/build.json` records: none (0) when there is no
/// record or the record predates formats, refused by name when unreadable.
fn recorded_format(layout: &Layout) -> IdentityResult<u32> {
    let path = layout.build_record();
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(io("read build record", &path, &error)),
    };
    let invalid = |detail: String| {
        refuse(ErrorKind::ConfigInvalid, "read build", "build.json", detail).at(&path)
    };
    let parsed = serde_json::from_slice::<serde_json::Value>(&bytes);
    let record = parsed.map_err(|error| invalid(error.to_string()))?;
    let Some(format) = record.get("data_format") else {
        return Ok(0);
    };
    match format.as_u64().map(u32::try_from) {
        Some(Ok(number)) => Ok(number),
        _ => Err(invalid(format!("data_format {format} is not a format number"))),
    }
}

/// Refuses by name unless `layout` holds an install with its binaries.
fn require_install(layout: &Layout, names: &[&str]) -> IdentityResult<()> {
    let mut needed = vec![layout.deployment_config(), layout.service_config()];
    needed.extend(names.iter().map(|name| layout.binary(name)));
    match needed.into_iter().find(|path| !path.is_file()) {
        None => Ok(()),
        Some(missing) => Err(refuse(
            ErrorKind::NotInstalled,
            "find install",
            "install",
            "no install here to upgrade; run `lys identity install` from the new build first",
        )
        .at(&missing)),
    }
}

/// What has been changed, so a failure puts back exactly that.
#[derive(Default)]
struct Moved {
    bin: bool,
    surface: bool,
}

fn remove_dir(path: &Path) -> IdentityResult<()> {
    if path.exists() {
        std::fs::remove_dir_all(path).map_err(|error| io("remove", path, &error))?;
    }
    Ok(())
}

fn swap_in(
    layout: &Layout,
    from: &Path,
    names: &[&str],
    package: Option<&Path>,
    moved: &mut Moved,
) -> IdentityResult<()> {
    remove_dir(&layout.bin_previous_dir())?;
    std::fs::rename(layout.bin_dir(), layout.bin_previous_dir())
        .map_err(|error| io("keep previous binaries", &layout.bin_dir(), &error))?;
    moved.bin = true;
    private_files::ensure_dir(&layout.bin_dir())?;
    for name in names {
        place_binary(&from.join(name), &layout.bin_dir(), name)?;
    }
    if let Some(package) = package {
        remove_dir(&layout.surface_previous_dir())?;
        if layout.surface_dir().exists() {
            std::fs::rename(layout.surface_dir(), layout.surface_previous_dir())
                .map_err(|error| io("keep previous screens", &layout.surface_dir(), &error))?;
        }
        moved.surface = true;
        surface::place(package, &layout.surface_dir())?;
    }
    Ok(())
}

/// Puts back what `moved` names.
fn put_back(layout: &Layout, moved: &Moved) -> IdentityResult<()> {
    if moved.bin {
        remove_dir(&layout.bin_dir())?;
        std::fs::rename(layout.bin_previous_dir(), layout.bin_dir())
            .map_err(|error| io("restore previous binaries", &layout.bin_dir(), &error))?;
    }
    if moved.surface {
        remove_dir(&layout.surface_dir())?;
        if layout.surface_previous_dir().exists() {
            std::fs::rename(layout.surface_previous_dir(), layout.surface_dir())
                .map_err(|error| io("restore previous screens", &layout.surface_dir(), &error))?;
        }
    }
    Ok(())
}

fn stop_all(units: &[Unit], say: &mut dyn FnMut(&str)) -> IdentityResult<()> {
    for unit in units.iter().rev() {
        if services::stop(&unit.pid)? {
            say(&format!("{} stopped", unit.binary));
        }
    }
    Ok(())
}

fn start_all(layout: &Layout, units: &[Unit], say: &mut dyn FnMut(&str)) -> Result<(), String> {
    for unit in units {
        launch(layout, unit, true).map_err(|error| {
            format!(
                "{} did not start ready: {error}; its output is in {}",
                unit.binary,
                unit.log.display()
            )
        })?;
        say(&format!("{} started and ready", unit.binary));
    }
    Ok(())
}

/// Upgrades the install under `layout` to the binaries in `from` (and the
/// screens in `package`), running `units` in their order, and returns the
/// build now running. `say` hears every line the upgrade prints.
pub fn upgrade(
    layout: &Layout,
    from: &Path,
    package: Option<&Path>,
    units: &[Unit],
    say: &mut dyn FnMut(&str),
) -> IdentityResult<BuildRecord> {
    let names: Vec<&str> = units.iter().map(|unit| unit.binary).collect();
    back::refuse_unfinished(layout)?;
    require_install(layout, &names)?;
    for name in &names {
        let new = from.join(name);
        if !new.is_file() {
            return Err(refuse(
                ErrorKind::BinaryMissing,
                "find new binary",
                name,
                format!("{} holds no {name}", from.display()),
            )
            .at(&new));
        }
        let installed = version(&layout.binary(name), name)?;
        let incoming = version(&new, name)?;
        say(&format!("{name}: installed {installed}, new {incoming}"));
    }
    if let Some(package) = package {
        let (manifest, _) = surface::verify(package)?;
        say(&format!("screens: new {}", manifest.commit));
    }
    let mut swap = |moved: &mut Moved| swap_in(layout, from, &names, package, moved);
    let undo = |moved: &Moved| put_back(layout, moved);
    cycle(layout, units, say, &mut swap, &undo, "the previous build")
}

/// Stops `units`, runs `swap`, starts them ready and records the build; on
/// a failure stops what started, runs `undo` over what `swap` moved, starts
/// `began` (the build it began from) ready and fails naming what broke.
fn cycle(
    layout: &Layout,
    units: &[Unit],
    say: &mut dyn FnMut(&str),
    swap: &mut dyn FnMut(&mut Moved) -> IdentityResult<()>,
    undo: &dyn Fn(&Moved) -> IdentityResult<()>,
    began: &str,
) -> IdentityResult<BuildRecord> {
    let names: Vec<&str> = units.iter().map(|unit| unit.binary).collect();
    let mut moved = Moved::default();
    let outcome = stop_all(units, say)
        .and_then(|()| swap(&mut moved))
        .map_err(|error| error.to_string())
        .and_then(|()| start_all(layout, units, say));
    let Err(failure) = outcome else {
        return record_build(layout, &names, say);
    };
    say(&format!("upgrade failed: {failure}"));
    say(&format!("putting {began} back"));
    let restored = stop_all(units, say)
        .and_then(|()| undo(&moved))
        .map_err(|error| error.to_string())
        .and_then(|()| start_all(layout, units, say));
    if let Err(again) = restored {
        return Err(refuse(
            ErrorKind::UpgradeFailed,
            "upgrade",
            "install",
            format!("{failure}; {began} did not come back either: {again}"),
        ));
    }
    record_build(layout, &names, say)?;
    Err(refuse(
        ErrorKind::UpgradeFailed,
        "upgrade",
        "install",
        format!("{failure}; {began} is back and running"),
    ))
}

/// Runs `lys identity upgrade`.
pub fn run(options: &Options, json: bool) -> IdentityResult<()> {
    let layout = match &options.root {
        Some(root) => Layout::at(root.clone()),
        None => Layout::discover()?,
    };
    let mut emitter = Emitter::new(json);
    emitter.field("root", "root", layout.root.display().to_string());
    let mut say = |line: &str| {
        if !json {
            println!("{line}");
        }
    };
    let units = units(&layout);
    let record = match (&options.from.folder, options.from.back) {
        (None, true) if options.surface.is_none() => back(&layout, &units, &mut say)?,
        (Some(from), false) => {
            upgrade(&layout, from, options.surface.as_deref(), &units, &mut say)?
        }
        _ => {
            return Err(refuse(
                ErrorKind::ConfigInvalid,
                "choose build",
                "upgrade",
                "name exactly one of --from DIR or --back, and --surface only with --from",
            ));
        }
    };
    let value = serde_json::to_value(&record).map_err(|error| {
        refuse(
            ErrorKind::RenderFailed,
            "render",
            "build.json",
            error.to_string(),
        )
    })?;
    if json {
        emitter.field("build", "build", value);
    }
    emitter.finish();
    Ok(())
}

#[cfg(test)]
#[path = "upgrade_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "upgrade_back_tests.rs"]
mod back_tests;
