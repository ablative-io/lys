//! `lys identity upgrade --back`: the install returned to the build kept
//! beside it, through the same stop, swap, start and ready path as an
//! upgrade (the parent module's `cycle`).
//!
//! Invariants: nothing is stopped until the version of every binary in
//! `bin/` and in `bin.previous/` has been read and the data's format checked
//! against the kept build's. With no kept build it is refused
//! `nothing_to_return_to`; when the data may hold a format newer than the
//! kept build reads it is refused `back_would_not_read`, naming the first
//! event kind the kept build does not know, because data is never rolled
//! back. The swap is an exchange, so the build it leaves becomes the one
//! kept and `--back` again returns to it. An exchange stopped part-way, or
//! an unfinished upgrade's intent record, is refused by name and never
//! removed or guessed past.

use std::path::{Path, PathBuf};

use super::{BuildRecord, Formats, Moved, Unit, cycle, io, newest, recorded_format};
use super::{refuse, require_install, stated};
use crate::identity::error::{ErrorKind, IdentityResult};
use crate::identity::install::layout::Layout;

/// The intent record an unfinished upgrade leaves in `install/`.
const INTENT: &str = "upgrade.json";

/// Refuses by name when an upgrade left its intent record unfinished.
pub(super) fn refuse_unfinished(layout: &Layout) -> IdentityResult<()> {
    let intent = layout.install_dir().join(INTENT);
    if !intent.exists() {
        return Ok(());
    }
    Err(refuse(
        ErrorKind::UpgradeUnfinished,
        "read intent",
        "upgrade",
        "an earlier upgrade did not finish; nothing is stopped until it is finished or put back",
    )
    .at(&intent))
}

/// The name beside `a` an exchange moves it through, refused by name when
/// an exchange that stopped part-way left it: it is never removed.
fn aside(a: &Path) -> IdentityResult<PathBuf> {
    let aside = a.with_extension("exchanging");
    if !aside.exists() {
        return Ok(aside);
    }
    Err(refuse(
        ErrorKind::UpgradeUnfinished,
        "exchange",
        "install",
        "an earlier exchange stopped part-way here; nothing is moved until it is put back",
    )
    .at(&aside))
}

/// Exchanges what `a` and `b` hold by three renames through the name
/// beside `a`; either may be absent.
fn exchange(a: &Path, b: &Path) -> IdentityResult<()> {
    let aside = aside(a)?;
    for (from, to) in [(a, aside.as_path()), (b, a), (aside.as_path(), b)] {
        if from.exists() {
            std::fs::rename(from, to).map_err(|error| io("exchange", from, &error))?;
        }
    }
    Ok(())
}

/// Refuses `back_would_not_read` when the data may hold a format newer than
/// the `kept` build's, naming the first event kind of `running` it lacks.
fn check_formats(layout: &Layout, running: &Formats, kept: &Formats) -> IdentityResult<String> {
    let data = recorded_format(layout)?.max(newest(running));
    let known = newest(kept);
    if data <= known {
        return Ok(format!("data format {data}, kept build reads {known}"));
    }
    let first = running
        .range(known.saturating_add(1)..)
        .flat_map(|(_, kinds)| kinds)
        .next();
    let unknown = match first {
        Some(kind) => format!("`{kind}`"),
        None => format!("of data format {data}"),
    };
    Err(refuse(
        ErrorKind::BackWouldNotRead,
        "compare data formats",
        "data",
        format!(
            "the kept build reads data format {known} and the data may hold format {data}: \
             it does not know the event kind {unknown}; data is never rolled back"
        ),
    )
    .at(&layout.data_dir()))
}

/// Returns the install under `layout` to the build kept in `bin.previous/`
/// (and the screens in `surface.previous/` when there are any), running
/// `units` in their order, and returns the build now running. The build it
/// leaves becomes the one kept.
pub fn back(
    layout: &Layout,
    units: &[Unit],
    say: &mut dyn FnMut(&str),
) -> IdentityResult<BuildRecord> {
    let names: Vec<&str> = units.iter().map(|unit| unit.binary).collect();
    refuse_unfinished(layout)?;
    require_install(layout, &names)?;
    let (bin, kept_bin) = (layout.bin_dir(), layout.bin_previous_dir());
    if !kept_bin.is_dir() {
        return Err(refuse(
            ErrorKind::NothingToReturnTo,
            "find kept build",
            "install",
            "no earlier build is kept here; only an upgrade keeps one",
        )
        .at(&kept_bin));
    }
    let (mut running, mut kept) = (Formats::new(), Formats::new());
    for name in &names {
        let (now, now_formats) = stated(&layout.binary(name), name)?;
        let (then, then_formats) = stated(&kept_bin.join(name), name)?;
        say(&format!("{name}: running {now}, kept {then}"));
        for (into, from) in [(&mut running, now_formats), (&mut kept, then_formats)] {
            for (format, kinds) in from {
                into.entry(format).or_default().extend(kinds);
            }
        }
    }
    say(&check_formats(layout, &running, &kept)?);
    let (screens, kept_screens) = (layout.surface_dir(), layout.surface_previous_dir());
    // A half-made exchange is refused here, before anything stops.
    aside(&bin)?;
    aside(&screens)?;
    let mut swap = |moved: &mut Moved| -> IdentityResult<()> {
        exchange(&bin, &kept_bin)?;
        moved.bin = true;
        if kept_screens.exists() {
            exchange(&screens, &kept_screens)?;
            moved.surface = true;
        }
        Ok(())
    };
    let undo = |moved: &Moved| -> IdentityResult<()> {
        if moved.bin {
            exchange(&bin, &kept_bin)?;
        }
        if moved.surface {
            exchange(&screens, &kept_screens)?;
        }
        Ok(())
    };
    cycle(layout, units, say, &mut swap, &undo, "the build it began from")
}
