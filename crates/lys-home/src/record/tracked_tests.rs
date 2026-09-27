//! Gates on the tracked set: exactly the session file, its index and head,
//! each block and each template, in byte order; never the lock file, a
//! temporary, a file at the home root, a file under a directory that does
//! not name it, or a name that is not a lowercase hash.

use std::error::Error;
use std::path::Path;

use crate::record::Home;
use crate::record::blocks::Hash;
use crate::record::tracked::tracked_set;

const SESSION: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/launch/session.jsonl"
);

type Gate = Result<(), Box<dyn Error>>;

/// A home holding session `fixture` with its index, head and lock, one
/// block, one template and three stray files; returns the five paths the
/// tracked set must be.
fn fixture(root: &Path) -> Result<(Home, Vec<String>), Box<dyn Error>> {
    let home = Home::open(root)?;
    std::fs::copy(SESSION, home.session_path("fixture")?)?;
    drop(home.open_session("fixture")?);
    let block = home.blocks()?.put(b"fixture block")?.hash;
    let template = home.templates().put(b"fixture template")?.hash;
    std::fs::write(root.join("blocks").join(".incoming.1.00.tmp"), b"tmp")?;
    std::fs::write(root.join("sessions").join("fixture.head.tmp"), b"tmp")?;
    std::fs::write(root.join("notes.txt"), b"notes")?;
    for present in ["fixture.jsonl", "fixture.index.jsonl", "fixture.head", "fixture..lock"] {
        assert!(root.join("sessions").join(present).is_file(), "{present}");
    }
    let b = block.as_str();
    let t = template.as_str();
    let expected = vec![
        format!("blocks/{}/{b}", &b[..2]),
        "sessions/fixture.head".to_owned(),
        "sessions/fixture.index.jsonl".to_owned(),
        "sessions/fixture.jsonl".to_owned(),
        format!("templates/{}/{t}", &t[..2]),
    ];
    Ok((home, expected))
}

#[test]
fn the_tracked_set_is_exactly_the_five_files_in_byte_order() -> Gate {
    let dir = tempfile::tempdir()?;
    let (home, expected) = fixture(dir.path())?;
    let tracked = tracked_set(&home)?;
    assert_eq!(tracked.len(), 5);
    assert_eq!(tracked, expected);
    Ok(())
}

#[test]
fn a_hash_under_the_wrong_directory_or_in_uppercase_is_not_tracked() -> Gate {
    let dir = tempfile::tempdir()?;
    let (home, expected) = fixture(dir.path())?;
    let blocks = dir.path().join("blocks");
    std::fs::create_dir_all(blocks.join("ab"))?;
    std::fs::write(blocks.join("ab").join(format!("cd{}", "0".repeat(62))), b"x")?;
    // Another block's hash, so a filesystem that folds case cannot land the
    // uppercase name on the stored block's own file.
    let other = Hash::of(b"another fixture block");
    let shard = &other.as_str()[..2];
    std::fs::create_dir_all(blocks.join(shard))?;
    std::fs::write(blocks.join(shard).join(other.as_str().to_uppercase()), b"x")?;
    assert!(blocks.join(shard).join(other.as_str().to_uppercase()).is_file());
    let tracked = tracked_set(&home)?;
    assert_eq!(tracked.len(), 5);
    assert_eq!(tracked, expected);
    Ok(())
}
