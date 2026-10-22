#![cfg(test)]

//! DIRECTORY-115: client releases are placed inside the upgrade's
//! transaction and removed, by name, when it is refused; a release that
//! existed before the attempt is untouched; a finished upgrade serves what
//! it placed.

use std::path::{Path, PathBuf};

use super::super::scratch::{
    A, B, Behaviour, Fixed, Recorder, Scratch, TestResult, build, ready_lines,
};
use super::super::{BuildRecord, ClientRender, Parts, record_build, upgrade};
use crate::identity::error::IdentityResult;
use crate::identity::install::layout::{
    BINARIES, ClientSet, Layout, PlacedClients, client_releases, prepare_clients,
};

/// A build that fails after the clients are placed.
const C: &str = "cccccccccccccccccccccccccccccccccccccccc";
const PLATFORM: &str = "aarch64-apple-darwin";

/// Client inputs for `commit`, one platform, under the scratch's work folder.
fn inputs(scratch: &Scratch, commit: &str) -> TestResult<ClientSet> {
    let folder = scratch.work().join("clients").join(commit);
    let platform = folder.join(PLATFORM);
    std::fs::create_dir_all(&platform)?;
    for name in ["lys", "lys-containment-entry"] {
        std::fs::write(platform.join(name), format!("{name}/{commit}"))?;
    }
    Ok(prepare_clients(Some(&folder), &folder, commit)?)
}

/// Places `set` as a finished attempt leaves it, saying nothing.
fn placed(set: &ClientSet, layout: &Layout) -> TestResult {
    set.place(layout, &mut PlacedClients::default(), &mut |_| {})?;
    Ok(())
}

/// A's clients served: placed, named by `clients.json` and recorded. The
/// manifest's bytes, to compare after a refused attempt.
fn serving_a(scratch: &Scratch) -> TestResult<Vec<u8>> {
    let old = inputs(scratch, A)?;
    placed(&old, &scratch.layout)?;
    let manifest = old.manifest(&scratch.layout)?;
    super::super::swap::place_file(&manifest.target, &manifest.bytes, false)?;
    record_build(&scratch.layout, &BINARIES, &mut |_| {})?;
    Ok(manifest.bytes.to_vec())
}

fn release_dir(layout: &Layout, commit: &str) -> PathBuf {
    layout.root.join("clients").join(PLATFORM).join(commit)
}

fn manifest_bytes(layout: &Layout) -> TestResult<Vec<u8>> {
    Ok(std::fs::read(layout.root.join("clients.json"))?)
}

/// An upgrade to the build in `from` placing `clients` inside its transaction.
fn attempt(
    scratch: &Scratch,
    from: &Path,
    clients: &ClientSet,
    said: &mut Vec<String>,
) -> IdentityResult<BuildRecord> {
    let render = ClientRender {
        base: &Fixed,
        clients,
    };
    let mut parts = Parts {
        runner: None,
        units: &scratch.units,
        engine: &mut Recorder::default(),
        render: &render,
        clients: Some(clients),
    };
    upgrade(&scratch.layout, from, None, &mut parts, &mut |line| {
        said.push(line.to_owned());
    })
}

#[test]
fn clients_rollback_removes_the_release_a_refused_upgrade_placed() -> TestResult {
    let scratch = Scratch::new()?;
    let before = serving_a(&scratch)?;
    let failing = scratch.work().join("c");
    build(&failing, C, Behaviour::ExitsEarly)?;
    let next = inputs(&scratch, C)?;
    let mut said = Vec::new();
    let refused = attempt(&scratch, &failing, &next, &mut said)
        .err()
        .ok_or("a service that exited was taken as ready")?;
    assert!(
        refused
            .to_string()
            .contains("the previous build is back and running"),
        "{refused}"
    );
    let dir = release_dir(&scratch.layout, C);
    assert!(!dir.exists(), "{said:?}");
    assert!(
        said.contains(&format!("client release removed: {}", dir.display())),
        "{said:?}"
    );
    assert_eq!(manifest_bytes(&scratch.layout)?, before);
    assert_eq!(
        std::fs::read_to_string(release_dir(&scratch.layout, A).join("lys"))?,
        format!("lys/{A}"),
        "the served release is untouched"
    );
    assert_eq!(scratch.running()?, ready_lines(A));
    assert!(!scratch.layout.upgrade_intent().exists());
    Ok(())
}

#[test]
fn clients_rollback_keeps_a_release_directory_that_existed_before_the_attempt() -> TestResult {
    let scratch = Scratch::new()?;
    let before = serving_a(&scratch)?;
    let failing = scratch.work().join("c");
    build(&failing, C, Behaviour::ExitsEarly)?;
    let next = inputs(&scratch, C)?;
    placed(&next, &scratch.layout)?;
    let dir = release_dir(&scratch.layout, C);
    let mut said = Vec::new();
    assert!(attempt(&scratch, &failing, &next, &mut said).is_err());
    assert_eq!(
        std::fs::read_to_string(dir.join("lys"))?,
        format!("lys/{C}"),
        "{said:?}"
    );
    assert!(
        !said
            .iter()
            .any(|line| line.starts_with("client release removed")),
        "{said:?}"
    );
    assert_eq!(manifest_bytes(&scratch.layout)?, before);
    assert_eq!(scratch.running()?, ready_lines(A));
    Ok(())
}

#[test]
fn clients_rollback_refuses_a_changed_served_release_before_stopping_anything() -> TestResult {
    let scratch = Scratch::new()?;
    serving_a(&scratch)?;
    // C is served with other bytes: clients.json names it for the platform.
    let served = inputs(&scratch, C)?;
    placed(&served, &scratch.layout)?;
    let manifest = served.manifest(&scratch.layout)?;
    super::super::swap::place_file(&manifest.target, &manifest.bytes, false)?;
    let folder = scratch.work().join("clients").join(C).join(PLATFORM);
    std::fs::write(folder.join("lys"), "rebuilt at the same commit")?;
    let rebuilt = prepare_clients(Some(folder.parent().ok_or("no folder")?), &folder, C)?;
    let new = scratch.work().join("b");
    build(&new, B, Behaviour::Serves)?;
    let mut said = Vec::new();
    let refused = attempt(&scratch, &new, &rebuilt, &mut said)
        .err()
        .ok_or("a served release was replaced")?;
    assert!(
        refused.to_string().contains("client_release_changed"),
        "{refused}"
    );
    assert_eq!(
        std::fs::read_to_string(release_dir(&scratch.layout, C).join("lys"))?,
        format!("lys/{C}")
    );
    assert_eq!(scratch.running()?, ready_lines(A), "{said:?}");
    assert!(!scratch.layout.bin_previous_dir().exists());
    assert!(!scratch.layout.upgrade_intent().exists());
    Ok(())
}

#[test]
fn clients_rollback_leaves_a_finished_upgrade_serving_what_it_placed() -> TestResult {
    let scratch = Scratch::new()?;
    serving_a(&scratch)?;
    let new = scratch.work().join("b");
    build(&new, B, Behaviour::Serves)?;
    let next = inputs(&scratch, B)?;
    let mut said = Vec::new();
    let record = attempt(&scratch, &new, &next, &mut said)?;
    assert_eq!(
        std::fs::read_to_string(release_dir(&scratch.layout, B).join("lys"))?,
        format!("lys/{B}")
    );
    assert_eq!(client_releases(&scratch.layout)?, next.releases);
    assert_eq!(record.clients, next.releases);
    assert!(
        !said
            .iter()
            .any(|line| line.starts_with("client release removed")),
        "{said:?}"
    );
    assert_eq!(scratch.running()?, ready_lines(B));
    Ok(())
}
