#![cfg(test)]

//! The intent record, and an upgrade stopped part-way: the next run names
//! it and finishes it or puts the previous build back, so the install runs
//! one whole build.

use std::collections::BTreeMap;
use std::io::Write;

use super::super::scratch::{
    A, B, Behaviour, Recorder, Scratch, TestResult, build, files_for, ready_lines,
};
use super::super::{swap, version};
use super::{Intent, Kept, Step};
use crate::identity::error::ErrorKind;
use crate::identity::install::layout::BINARIES;

/// The intent an upgrade of `scratch` from A to B writes before its first
/// stop.
fn intent_to_b(scratch: &Scratch) -> Intent {
    let build = |commit: &str| -> BTreeMap<String, String> {
        BINARIES
            .iter()
            .map(|name| ((*name).to_string(), commit.to_string()))
            .collect()
    };
    Intent {
        from: build(A),
        to: build(B),
        screens: false,
        screens_existed: true,
        files: files_for(&scratch.layout, B, true)
            .iter()
            .map(|file| Kept {
                name: file.name.to_string(),
                target: file.target.clone(),
                existed: file.target.is_file(),
                private: file.private,
            })
            .collect(),
        compose_changed: true,
        steps: Vec::new(),
    }
}

#[test]
fn the_record_holds_each_step_as_it_completes() -> TestResult {
    let scratch = Scratch::laid_out()?;
    let layout = &scratch.layout;
    assert_eq!(Intent::read(layout)?, None);
    let mut intent = intent_to_b(&scratch);
    intent.write(layout)?;
    intent.done(layout, Step::Stopped)?;
    intent.done(layout, Step::BinariesKept)?;
    let read = Intent::read(layout)?.ok_or("no record was written")?;
    assert_eq!(read.steps, [Step::Stopped, Step::BinariesKept]);
    assert!(read.has(Step::BinariesKept) && !read.has(Step::BinariesPlaced));
    assert_eq!(read.describe(), format!("from {A} to {B}"));
    let text = std::fs::read_to_string(layout.upgrade_intent())?;
    assert!(text.contains("\"binaries_kept\""), "{text}");
    Intent::clear(layout)?;
    assert_eq!(Intent::read(layout)?, None);
    Ok(())
}

/// Starts an upgrade of `scratch` to B and cuts it off after `bin/` was
/// moved aside and before the last binary was copied, as a kill would.
fn killed_after_the_first_copy(scratch: &Scratch) -> TestResult {
    let layout = &scratch.layout;
    let new = scratch.work().join("b");
    build(&new, B, Behaviour::Serves)?;
    let mut intent = intent_to_b(scratch);
    intent.write(layout)?;
    swap::stop_all(&scratch.units, &mut |_| {})?;
    intent.done(layout, Step::Stopped)?;
    swap::keep_binaries(layout, &mut intent)?;
    std::fs::create_dir_all(layout.bin_dir())?;
    swap::place_binary(&new.join(BINARIES[0]), &layout.bin_dir(), BINARIES[0])?;
    assert!(
        !layout.binary(BINARIES[1]).exists(),
        "the last copy was made"
    );
    Ok(())
}

#[test]
fn the_next_upgrade_puts_back_an_upgrade_killed_part_way_then_runs() -> TestResult {
    let scratch = Scratch::new()?;
    killed_after_the_first_copy(&scratch)?;
    let mut said = Vec::new();
    let mut engine = Recorder::default();
    scratch.upgrade(&scratch.work().join("b"), None, &mut engine, &mut said)?;
    let named = said
        .iter()
        .position(|line| {
            line == &format!(
                "an unfinished upgrade from {A} to {B} stopped part-way: putting the previous build back"
            )
        })
        .ok_or_else(|| format!("the unfinished upgrade was not named: {said:?}"))?;
    let back = said
        .iter()
        .position(|line| line == "the previous binaries are back in bin/")
        .ok_or("the previous binaries were not put back")?;
    let upgraded = said
        .iter()
        .position(|line| line == &format!("{}: installed {A}, new {B}", BINARIES[1]))
        .ok_or("the new upgrade did not run")?;
    assert!(named < back && back < upgraded, "{said:?}");
    assert_eq!(scratch.running()?, ready_lines(B));
    for name in BINARIES {
        assert_eq!(version(&scratch.layout.binary(name), name)?, B);
        let kept = scratch.layout.bin_previous_dir().join(name);
        assert_eq!(version(&kept, name)?, A, "the kept build is whole");
    }
    assert!(!scratch.layout.upgrade_intent().exists());
    Ok(())
}

#[test]
fn recovery_alone_leaves_the_previous_build_whole_and_running() -> TestResult {
    let scratch = Scratch::new()?;
    killed_after_the_first_copy(&scratch)?;
    let mut said = Vec::new();
    let mut engine = Recorder::default();
    swap::recover(&scratch.layout, &scratch.units, &mut engine, &mut |line| {
        said.push(line.to_string());
    })?;
    assert!(
        said.iter().any(|line| line.contains("stopped part-way")),
        "{said:?}"
    );
    assert_eq!(scratch.running()?, ready_lines(A));
    for name in BINARIES {
        assert_eq!(version(&scratch.layout.binary(name), name)?, A);
    }
    assert!(!scratch.layout.upgrade_intent().exists());
    assert!(
        engine.applied.is_empty(),
        "no compose file was placed, so none is applied"
    );
    Ok(())
}

#[test]
fn an_upgrade_killed_after_its_new_build_started_is_finished() -> TestResult {
    let scratch = Scratch::new()?;
    let new = scratch.work().join("b");
    build(&new, B, Behaviour::Serves)?;
    scratch.upgrade(&new, None, &mut Recorder::default(), &mut Vec::new())?;
    let mut intent = intent_to_b(&scratch);
    intent.steps = vec![Step::Stopped, Step::BinariesKept, Step::Started];
    intent.write(&scratch.layout)?;
    std::fs::remove_file(scratch.layout.build_record())?;
    let mut said = Vec::new();
    swap::recover(
        &scratch.layout,
        &scratch.units,
        &mut Recorder::default(),
        &mut |line| said.push(line.to_string()),
    )?;
    let finishing =
        format!("an unfinished upgrade from {A} to {B} had started its new build: finishing it");
    assert!(said.contains(&finishing), "{said:?}");
    assert_eq!(scratch.running()?, ready_lines(B));
    let written: serde_json::Value =
        serde_json::from_slice(&std::fs::read(scratch.layout.build_record())?)?;
    assert_eq!(written["binaries"]["lys-identity-server"], B);
    assert!(!scratch.layout.upgrade_intent().exists());
    Ok(())
}

#[test]
fn a_copy_that_differs_from_its_source_is_refused_and_nothing_old_goes() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (old, new) = (dir.path().join("old"), dir.path().join("new"));
    build(&old, A, Behaviour::Serves)?;
    build(&new, B, Behaviour::Serves)?;
    let before = std::fs::read(old.join(BINARIES[1]))?;
    let mut copies = 0;
    let refused = swap::place_binary_with(
        &new.join(BINARIES[1]),
        &old,
        BINARIES[1],
        &mut |from, to| {
            copies += 1;
            std::fs::copy(from, to)?;
            std::fs::OpenOptions::new()
                .append(true)
                .open(to)?
                .write_all(b"# corrupted in the copy\n")
        },
    )
    .err()
    .ok_or("a corrupted copy was placed")?;
    assert_eq!(copies, 1);
    assert_eq!(refused.kind(), ErrorKind::ReadBackMismatch);
    assert!(
        refused.to_string().contains("nothing old was replaced"),
        "{refused}"
    );
    assert_eq!(
        std::fs::read(old.join(BINARIES[1]))?,
        before,
        "the old binary stays"
    );
    assert!(
        !old.join(format!(".{}.placing", BINARIES[1])).exists(),
        "the differing copy was removed"
    );
    swap::place_binary(&new.join(BINARIES[1]), &old, BINARIES[1])?;
    assert_eq!(version(&old.join(BINARIES[1]), BINARIES[1])?, B);
    Ok(())
}

#[test]
fn a_placed_file_is_read_back_before_it_replaces_the_old() -> TestResult {
    let dir = tempfile::tempdir()?;
    let target = dir.path().join("compose.env");
    std::fs::write(&target, "OLD=1\n")?;
    swap::place_file(&target, b"NEW=1\n", true)?;
    assert_eq!(std::fs::read_to_string(&target)?, "NEW=1\n");
    let mode = std::os::unix::fs::PermissionsExt::mode(&std::fs::metadata(&target)?.permissions());
    assert_eq!(mode & 0o777, 0o600, "a private file is owner-only");
    assert!(!dir.path().join(".compose.env.placing").exists());
    Ok(())
}
