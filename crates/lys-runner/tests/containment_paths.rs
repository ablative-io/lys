#![cfg(test)]
//! Real directory objects detect replacement and refuse symlinks at every depth.

use std::error::Error;
use std::os::unix::fs::symlink;

use lys_runner::containment_paths::Directory;

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn a_held_directory_rechecks_its_identity() -> TestResult {
    let temp = tempfile::tempdir()?;
    let path = temp.path().canonicalize()?;
    let first = Directory::open(&path)?;
    let second = Directory::open(&path)?;
    assert_eq!(first.identity(), second.identity());
    first.verify()?;
    Ok(())
}

#[test]
fn replacing_a_root_does_not_replace_its_held_authority() -> TestResult {
    let temp = tempfile::tempdir()?;
    let parent = temp.path().canonicalize()?;
    let path = parent.join("work");
    std::fs::create_dir(&path)?;
    let held = Directory::open(&path)?;
    std::fs::rename(&path, parent.join("original"))?;
    std::fs::create_dir(&path)?;
    assert_ne!(held.identity(), Directory::open(&path)?.identity());
    let refusal = held.verify().err().ok_or("replacement accepted")?;
    assert!(refusal.to_string().contains("identity changed"));
    assert!(refusal.to_string().contains(&path.display().to_string()));
    Ok(())
}

#[test]
fn links_at_the_root_and_in_ancestors_are_refused() -> TestResult {
    let temp = tempfile::tempdir()?;
    let parent = temp.path().canonicalize()?;
    std::fs::create_dir_all(parent.join("real/child"))?;
    symlink(parent.join("real"), parent.join("link"))?;
    assert!(Directory::open(&parent.join("link")).is_err());
    assert!(Directory::open(&parent.join("link/child")).is_err());
    Ok(())
}

#[test]
fn shared_textual_prefix_is_not_shared_directory_identity() -> TestResult {
    let temp = tempfile::tempdir()?;
    let parent = temp.path().canonicalize()?;
    std::fs::create_dir(parent.join("work"))?;
    std::fs::create_dir(parent.join("work-other"))?;
    assert_ne!(
        Directory::open(&parent.join("work"))?.identity(),
        Directory::open(&parent.join("work-other"))?.identity()
    );
    Ok(())
}

#[test]
fn ordinary_files_cannot_be_writable_directory_roots() -> TestResult {
    let temp = tempfile::tempdir()?;
    let file = temp.path().canonicalize()?.join("file");
    std::fs::write(&file, "owned fixture")?;
    assert!(Directory::open(&file).is_err());
    Ok(())
}
