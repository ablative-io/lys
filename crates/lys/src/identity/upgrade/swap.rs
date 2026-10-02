//! The swap of one build for another, each step recorded in the upgrade's
//! intent record as it completes, and its reverse.
//!
//! Forward: the service and then the broker are stopped; `bin/` is moved to
//! `bin.previous/` and the new binaries placed; the configuration and
//! compose files are copied to `config.previous/` and the new ones placed;
//! the screens are moved to `surface.previous/` and the new ones placed;
//! the compose services are brought to a changed definition; the broker and
//! then the service are started and waited on for ready. Back: whatever the
//! record and the disk say was replaced is put back, and the previous build
//! is started and waited on for ready.
//!
//! Invariants: every binary and file is placed by a copy to a temporary
//! name, read back and its SHA-256 compared with its source's, and only then
//! renamed over the old one; a copy that differs is removed and refused by
//! name, and nothing old is removed. `data/` and every credential are never
//! written. Putting back may run again after it was cut short: each of its
//! steps finds what it already did and does not undo it.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;

use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use super::super::error::{ErrorKind, IdentityError, IdentityResult};
use super::super::install::layout::Layout;
use super::super::install::surface;
use super::super::private_files;
use super::intent::{Intent, Kept, Step};
use super::render::RenderedFile;
use super::runner::Restart;
use super::{Engine, Unit, adopt, data_kept, launch, record_build};

fn io(action: &'static str, path: &Path, error: &std::io::Error) -> IdentityError {
    IdentityError::new(
        ErrorKind::PrivateFileIo,
        action,
        "upgrade",
        error.to_string(),
    )
    .at(path)
}

/// The SHA-256 of the file at `path`, lowercase hex.
fn digest_of(path: &Path) -> IdentityResult<String> {
    let bytes = Zeroizing::new(std::fs::read(path).map_err(|error| io("read back", path, &error))?);
    Ok(format!("{:x}", Sha256::digest(bytes.as_slice())))
}

/// Reads the copy at `placing` back and refuses it, removing it, unless its
/// SHA-256 is `expected`.
fn check_copy(placing: &Path, expected: &str, what: &str) -> IdentityResult<()> {
    let placed = digest_of(placing)?;
    if placed == expected {
        return Ok(());
    }
    let removed = match std::fs::remove_file(placing) {
        Ok(()) => String::new(),
        Err(error) => format!("; the copy could not be removed: {error}"),
    };
    Err(IdentityError::new(
        ErrorKind::ReadBackMismatch,
        "place",
        what,
        format!(
            "the copy's SHA-256 {placed} is not its source's {expected}; nothing old was replaced{removed}"
        ),
    )
    .at(placing))
}

fn remove_dir(path: &Path) -> IdentityResult<()> {
    if path.exists() {
        std::fs::remove_dir_all(path).map_err(|error| io("remove", path, &error))?;
    }
    Ok(())
}

fn rename(from: &Path, to: &Path, action: &'static str) -> IdentityResult<()> {
    std::fs::rename(from, to).map_err(|error| io(action, from, &error))
}

/// Copies `source` into `dir` as `name`: to a temporary name, read back and
/// compared with the source, then renamed, so `dir/name` is never a
/// half-written or differing binary.
pub fn place_binary(source: &Path, dir: &Path, name: &str) -> IdentityResult<()> {
    place_binary_with(source, dir, name, &mut |from, to| {
        std::fs::copy(from, to).map(drop)
    })
}

/// [`place_binary`], copying with `copy`.
pub fn place_binary_with(
    source: &Path,
    dir: &Path,
    name: &str,
    copy: &mut dyn FnMut(&Path, &Path) -> std::io::Result<()>,
) -> IdentityResult<()> {
    let target = dir.join(name);
    let placing = dir.join(format!(".{name}.placing"));
    let expected = digest_of(source)?;
    copy(source, &placing).map_err(|error| io("place binary", &placing, &error))?;
    check_copy(&placing, &expected, name)?;
    rename(&placing, &target, "place binary")
}

#[cfg(unix)]
fn create(path: &Path, private: bool) -> std::io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    if private {
        options.mode(private_files::FILE_MODE);
    }
    options.open(path)
}

#[cfg(not(unix))]
fn create(path: &Path, private: bool) -> std::io::Result<File> {
    if private {
        return Err(std::io::Error::other(
            "restricted file modes need a Unix host",
        ));
    }
    OpenOptions::new().write(true).create_new(true).open(path)
}

/// Writes `bytes` to `target` (owner-only when `private`): to a temporary
/// name, synced, read back and compared, then renamed over `target`.
pub fn place_file(target: &Path, bytes: &[u8], private: bool) -> IdentityResult<()> {
    let name = target
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let placing = target.with_file_name(format!(".{name}.placing"));
    match std::fs::remove_file(&placing) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(io("remove stale copy", &placing, &error)),
    }
    let mut file = create(&placing, private).map_err(|error| io("place", &placing, &error))?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|error| io("place", &placing, &error))?;
    drop(file);
    check_copy(&placing, &format!("{:x}", Sha256::digest(bytes)), &name)?;
    rename(&placing, target, "place")?;
    if let Some(parent) = target.parent() {
        File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| io("sync directory", parent, &error))?;
    }
    Ok(())
}

/// Stops the service and then the broker.
pub fn stop_all(units: &[Unit], say: &mut dyn FnMut(&str)) -> IdentityResult<()> {
    for unit in units.iter().rev() {
        adopt::stop(unit, say)?;
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

/// Moves `bin/` to `bin.previous/`, replacing an older one.
pub fn keep_binaries(layout: &Layout, intent: &mut Intent) -> IdentityResult<()> {
    remove_dir(&layout.bin_previous_dir())?;
    rename(
        &layout.bin_dir(),
        &layout.bin_previous_dir(),
        "keep previous binaries",
    )?;
    intent.done(layout, Step::BinariesKept)
}

/// Makes `bin/` when it is missing. Binaries hold no secret, so a `bin/`
/// others may read is used as it is, never refused as a private file.
pub fn ensure_bin(layout: &Layout) -> IdentityResult<()> {
    let bin = layout.bin_dir();
    std::fs::create_dir_all(&bin).map_err(|error| io("make bin/", &bin, &error))
}

/// Places each of `names` from `from` into a fresh `bin/`.
pub fn place_binaries(
    layout: &Layout,
    from: &Path,
    names: &[&str],
    intent: &mut Intent,
) -> IdentityResult<()> {
    ensure_bin(layout)?;
    for name in names {
        place_binary(&from.join(name), &layout.bin_dir(), name)?;
    }
    intent.done(layout, Step::BinariesPlaced)
}

fn keep_configuration(layout: &Layout, intent: &mut Intent) -> IdentityResult<()> {
    let kept = layout.config_previous_dir();
    remove_dir(&kept)?;
    private_files::ensure_dir(&kept)?;
    for file in intent.files.iter().filter(|file| file.existed) {
        let bytes = Zeroizing::new(
            std::fs::read(&file.target)
                .map_err(|error| io("keep configuration", &file.target, &error))?,
        );
        place_file(&kept.join(&file.name), &bytes, true)?;
    }
    intent.done(layout, Step::ConfigurationKept)
}

fn place_configuration(
    layout: &Layout,
    files: &[RenderedFile],
    intent: &mut Intent,
) -> IdentityResult<()> {
    for file in files {
        place_file(&file.target, &file.bytes, file.private)?;
    }
    intent.done(layout, Step::ConfigurationPlaced)
}

fn restore_configuration(layout: &Layout, files: &[Kept]) -> IdentityResult<()> {
    let kept = layout.config_previous_dir();
    for file in files {
        if file.existed {
            let copy = kept.join(&file.name);
            let bytes = Zeroizing::new(
                std::fs::read(&copy).map_err(|error| io("restore configuration", &copy, &error))?,
            );
            place_file(&file.target, &bytes, file.private)?;
            continue;
        }
        match std::fs::remove_file(&file.target) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(io("restore configuration", &file.target, &error)),
        }
    }
    Ok(())
}

fn place_screens(layout: &Layout, package: &Path, intent: &mut Intent) -> IdentityResult<()> {
    remove_dir(&layout.surface_previous_dir())?;
    if layout.surface_dir().exists() {
        rename(
            &layout.surface_dir(),
            &layout.surface_previous_dir(),
            "keep previous screens",
        )?;
    }
    intent.done(layout, Step::ScreensKept)?;
    surface::place(package, &layout.surface_dir())?;
    intent.done(layout, Step::ScreensPlaced)
}

/// What an upgrade places.
#[derive(Debug)]
pub struct Plan<'a> {
    /// The folder holding the new binaries.
    pub from: &'a Path,
    /// The binaries, in start order.
    pub names: &'a [&'static str],
    /// The screens package, when one is placed.
    pub package: Option<&'a Path>,
    /// The configuration and compose files rendered for the new build.
    pub files: &'a [RenderedFile],
}

fn swap_in(
    layout: &Layout,
    plan: &Plan<'_>,
    intent: &mut Intent,
    units: &[Unit],
    engine: &mut dyn Engine,
    say: &mut dyn FnMut(&str),
) -> IdentityResult<()> {
    stop_all(units, say)?;
    intent.done(layout, Step::Stopped)?;
    data_kept::keep(layout, intent)?;
    keep_binaries(layout, intent)?;
    place_binaries(layout, plan.from, plan.names, intent)?;
    keep_configuration(layout, intent)?;
    place_configuration(layout, plan.files, intent)?;
    if let Some(package) = plan.package {
        place_screens(layout, package, intent)?;
    }
    if intent.compose_changed {
        engine.apply(layout, say)?;
        intent.done(layout, Step::ComposeApplied)?;
    }
    Ok(())
}

/// Swaps the new build in and starts it, recording each step in `intent`.
/// The failure is the words that name what broke.
pub fn forward(
    layout: &Layout,
    plan: &Plan<'_>,
    intent: &mut Intent,
    units: &[Unit],
    engine: &mut dyn Engine,
    mut restart: Option<&mut Restart>,
    say: &mut dyn FnMut(&str),
) -> Result<(), String> {
    if let Some(restart) = restart.as_deref_mut() {
        restart.stop(say).map_err(|error| error.to_string())?;
    }
    swap_in(layout, plan, intent, units, engine, say).map_err(|error| error.to_string())?;
    start_all(layout, units, say)?;
    if let Some(restart) = restart {
        restart
            .start(layout, say, "started")
            .map_err(|error| error.to_string())?;
    }
    intent
        .done(layout, Step::Started)
        .map_err(|error| error.to_string())
}

/// Puts back what `intent` and the disk say was replaced, and starts the
/// previous build and waits for it ready.
pub fn back(
    layout: &Layout,
    intent: &Intent,
    units: &[Unit],
    engine: &mut dyn Engine,
    mut restart: Option<&mut Restart>,
    say: &mut dyn FnMut(&str),
) -> IdentityResult<()> {
    if let Some(restart) = restart.as_deref_mut() {
        restart.stop(say)?;
    }
    stop_all(units, say)?;
    let (bin, kept_bin) = (layout.bin_dir(), layout.bin_previous_dir());
    if kept_bin.exists() && (intent.has(Step::BinariesKept) || !bin.exists()) {
        remove_dir(&bin)?;
        rename(&kept_bin, &bin, "restore previous binaries")?;
        say("the previous binaries are back in bin/");
    }
    data_kept::restore(layout, intent, say)?;
    if intent.has(Step::ConfigurationKept) {
        restore_configuration(layout, &intent.files)?;
        say("the previous configuration and compose files are back");
    }
    let (screens, kept_screens) = (layout.surface_dir(), layout.surface_previous_dir());
    if intent.screens && (intent.has(Step::ScreensKept) || !screens.exists()) {
        if intent.screens_existed && kept_screens.exists() {
            remove_dir(&screens)?;
            rename(&kept_screens, &screens, "restore previous screens")?;
            say("the previous screens are back");
        } else if !intent.screens_existed {
            remove_dir(&screens)?;
        }
    }
    if intent.compose_changed && intent.has(Step::ConfigurationPlaced) {
        engine.apply(layout, say)?;
    }
    start_all(layout, units, say).map_err(|failure| {
        IdentityError::new(ErrorKind::UpgradeFailed, "put back", "install", failure)
    })?;
    if let Some(restart) = restart {
        restart.start(layout, say, "restored and ready")?;
    }
    Ok(())
}

/// Ends an upgrade that stopped part-way, before anything else is done: one
/// whose new build had started is finished, any other is put back. Each
/// says which, and the build then running is recorded.
pub fn recover(
    layout: &Layout,
    units: &[Unit],
    engine: &mut dyn Engine,
    say: &mut dyn FnMut(&str),
) -> IdentityResult<()> {
    let Some(intent) = Intent::read(layout)? else {
        return Ok(());
    };
    let mut restart = if intent.from.contains_key("lys") || intent.to.contains_key("lys") {
        Some(Restart::prepare(layout)?)
    } else {
        None
    };
    recover_intent(layout, &intent, units, engine, restart.as_mut(), say)
}

/// Recovers with the runner already checked before any upgrade mutation.
pub fn recover_with_runner(
    layout: &Layout,
    units: &[Unit],
    engine: &mut dyn Engine,
    restart: &mut Restart,
    say: &mut dyn FnMut(&str),
) -> IdentityResult<()> {
    let Some(intent) = Intent::read(layout)? else {
        return Ok(());
    };
    recover_intent(layout, &intent, units, engine, Some(restart), say)
}

fn recover_intent(
    layout: &Layout,
    intent: &Intent,
    units: &[Unit],
    engine: &mut dyn Engine,
    mut restart: Option<&mut Restart>,
    say: &mut dyn FnMut(&str),
) -> IdentityResult<()> {
    let mut names: Vec<&str> = units.iter().map(|unit| unit.binary).collect();
    if let Some(restart) = restart.as_deref_mut() {
        names.push("lys");
        if !intent.from.contains_key("lys") && !intent.to.contains_key("lys") {
            adopt::installed(layout, std::slice::from_ref(&restart.unit), say)?;
            let kept = layout.bin_previous_dir();
            if !intent.has(Step::Started) && kept.is_dir() && !kept.join("lys").is_file() {
                place_binary(&layout.binary("lys"), &kept, "lys")?;
            }
        }
    }
    if intent.has(Step::Started) {
        say(&format!(
            "an unfinished upgrade {} had started its new build: finishing it",
            intent.describe()
        ));
        for unit in units {
            launch(layout, unit, false)?;
        }
        if let Some(restart) = restart {
            restart.stop(say)?;
            restart.start(layout, say, "recovered and ready")?;
        }
    } else {
        say(&format!(
            "an unfinished upgrade {} stopped part-way: putting the previous build back",
            intent.describe()
        ));
        back(layout, intent, units, engine, restart, say)?;
    }
    record_build(layout, &names, say)?;
    Intent::clear(layout)
}
