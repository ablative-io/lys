//! Gates on the remote rule: every URL and scp-style remote refused as
//! `remote_not_local` with the stage 3 words, and a path taken as an
//! absolute path, a relative one against the base it is given.

use std::error::Error;
use std::path::{Path, PathBuf};

use crate::home_move::remote::take_remote;

const STAGE_THREE: &str =
    "shipping off this machine waits for stage 3's encryption from the secrets step";

#[test]
fn every_remote_off_this_machine_is_refused_by_name() {
    let refused = [
        "ssh://host.example/r.git",
        "host.example:r.git",
        "git@github.com:o/r.git",
        "https://github.com/o/r.git",
        "git://host.example/r.git",
        "file:///tmp/r.git",
    ];
    let mut count = 0;
    for remote in refused {
        let taken = take_remote(remote, Path::new("/tmp/work"));
        let text = taken.as_ref().map_or_else(ToString::to_string, |path| {
            format!("taken as {}", path.display())
        });
        assert!(taken.is_err(), "{text}");
        assert!(text.starts_with("remote_not_local"), "{text}");
        assert!(text.contains(STAGE_THREE), "{text}");
        assert!(text.contains(remote), "{text}");
        count += 1;
    }
    assert_eq!(count, 6);
}

#[test]
fn a_path_is_taken_as_an_absolute_path_on_this_machine() -> Result<(), Box<dyn Error>> {
    let base = Path::new("/tmp/work");
    assert_eq!(
        take_remote("/tmp/r.git", base)?,
        PathBuf::from("/tmp/r.git")
    );
    assert_eq!(
        take_remote("r.git", base)?,
        PathBuf::from("/tmp/work/r.git")
    );
    assert_eq!(take_remote("./a:b", base)?, PathBuf::from("/tmp/work/a:b"));
    Ok(())
}
