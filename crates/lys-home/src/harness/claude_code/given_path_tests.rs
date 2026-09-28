#![cfg(test)]
//! Gates on the record's canonicalisation rule (HOME-010 R1): a symlinked
//! directory, a `..` and a trailing slash each resolve; a symlinked or
//! dangling document is named at its link position; an absent path, parent
//! or component is kept as given and marked unresolved; a relative path is
//! left alone; and any other failure is refused by operation and path. Every
//! directory and link is made under a temporary directory by the test.

use std::path::{Path, PathBuf};

use crate::error::HomeError;
use crate::harness::claude_code::given_path::{
    CANONICALISING, GivenPath, canonical_dir, canonical_document,
};

type Outcome = Result<(), Box<dyn std::error::Error>>;

fn put(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, bytes)
}

fn text(path: &Path) -> Result<&str, Box<dyn std::error::Error>> {
    Ok(path.to_str().ok_or("a UTF-8 path")?)
}

#[cfg(unix)]
#[test]
fn the_ten_shapes_canonicalise_by_the_record_s_rule() -> Outcome {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let dir = tempfile::tempdir()?;
    let b = dir.path().canonicalize()?;
    let real = b.join("real");
    put(&real.join("w").join("CLAUDE.md"), b"chain\n")?;
    std::fs::create_dir_all(real.join("c"))?;
    symlink(&real, b.join("link"))?;
    let mut cases = 0;

    let through_link = canonical_document(&b.join("link").join("w").join("CLAUDE.md"))?;
    assert_eq!(
        through_link,
        GivenPath::Canonical(real.join("w").join("CLAUDE.md"))
    );
    cases += 1;

    let dotted = canonical_document(&real.join("w").join("..").join("w").join("CLAUDE.md"))?;
    assert_eq!(
        dotted,
        GivenPath::Canonical(real.join("w").join("CLAUDE.md"))
    );
    cases += 1;

    let slashed = canonical_dir(Path::new(&format!("{}/", text(&real.join("c"))?)))?;
    assert_eq!(slashed, GivenPath::Canonical(real.join("c")));
    assert_eq!(
        serde_json::to_string(slashed.path())?,
        serde_json::to_string(text(&real.join("c"))?)?
    );
    cases += 1;

    let gone = format!("{}/", text(&b.join("gone").join("c"))?);
    let unresolved = canonical_dir(Path::new(&gone))?;
    assert_eq!(unresolved, GivenPath::Unresolved(PathBuf::from(&gone)));
    let bytes = unresolved.path().as_os_str().as_encoded_bytes();
    assert_eq!(bytes, gone.as_bytes());
    cases += 1;

    let through_file = real.join("w").join("CLAUDE.md").join("x");
    assert_eq!(
        canonical_document(&through_file)?,
        GivenPath::Unresolved(through_file)
    );
    cases += 1;

    let absent = real.join("w").join("absent.md");
    assert_eq!(canonical_document(&absent)?, GivenPath::Unresolved(absent));
    cases += 1;

    let dangling = real.join("w").join("dangling.md");
    symlink(real.join("t").join("none.md"), &dangling)?;
    assert_eq!(
        canonical_document(&dangling)?,
        GivenPath::Canonical(dangling)
    );
    cases += 1;

    let relative = canonical_document(Path::new("instructions.md"))?;
    assert_eq!(
        relative,
        GivenPath::Relative(PathBuf::from("instructions.md"))
    );
    cases += 1;

    let locked = b.join("locked");
    let behind = locked.join("w").join("CLAUDE.md");
    put(&behind, b"locked\n")?;
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000))?;
    let refused = canonical_document(&behind);
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755))?;
    let Err(error) = refused else {
        return Err("a path behind a directory that cannot be searched is refused".into());
    };
    let shown = error.to_string();
    let HomeError::Io { context, path, .. } = &error else {
        return Err(format!("an I/O refusal, not {shown}").into());
    };
    assert_eq!(*context, CANONICALISING);
    assert_eq!(*path, behind);
    assert!(shown.contains("canonicalising a given path"), "{shown}");
    assert!(shown.contains(text(&behind)?), "{shown}");
    cases += 1;

    let claude_md = real.join("w").join("CLAUDE.md");
    std::fs::remove_file(&claude_md)?;
    put(&real.join("t").join("target.md"), b"target\n")?;
    symlink(real.join("t").join("target.md"), &claude_md)?;
    assert_eq!(
        canonical_document(&claude_md)?,
        GivenPath::Canonical(claude_md)
    );
    cases += 1;

    assert_eq!(cases, 10);
    Ok(())
}
