#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Gates on the template store: named by hash, written once, never rewritten,
//! and absent until the first template is stored.

use crate::record::Home;
use crate::record::blocks::Hash;

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/launch/template.json"
);

#[test]
fn a_template_is_kept_once_under_its_hash_and_a_second_put_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let home = Home::open(dir.path().join("home")).unwrap();
    let bytes = std::fs::read(FIXTURE).unwrap();
    let store = home.templates();
    assert!(!store.root().exists(), "nothing until the first put");
    let first = store.put(&bytes).unwrap();
    assert!(first.new);
    assert_eq!(first.hash, Hash::of(&bytes));
    let path = home
        .root()
        .join("templates")
        .join(&first.hash.as_str()[..2])
        .join(first.hash.as_str());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    assert_eq!(store.path_of(&first.hash), path);
    let mtime = std::fs::metadata(&path).unwrap().modified().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(20));
    let second = store.put(&bytes).unwrap();
    assert!(!second.new);
    assert_eq!(second.hash, first.hash);
    assert_eq!(std::fs::metadata(&path).unwrap().modified().unwrap(), mtime);
    assert_eq!(
        std::fs::read_dir(path.parent().unwrap()).unwrap().count(),
        1
    );
    assert!(store.contains(&first.hash));
    assert_eq!(store.get(&first.hash).unwrap(), bytes);
    let missing = Hash::of(b"never stored");
    let err = store.get(&missing).unwrap_err().to_string();
    assert!(err.contains(missing.as_str()), "{err}");
    dir.close().unwrap();
}

#[test]
fn opening_a_home_creates_only_sessions_and_blocks() {
    let dir = tempfile::tempdir().unwrap();
    let home = Home::open(dir.path().join("home")).unwrap();
    let mut names: Vec<String> = std::fs::read_dir(home.root())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(names, ["blocks", "sessions"]);
    assert!(!home.templates().contains(&Hash::of(b"x")));
    assert_eq!(names.len(), 2);
    dir.close().unwrap();
}
