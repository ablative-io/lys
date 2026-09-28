#![cfg(test)]
//! DIRECTORY-029 R12, CONFORMANCE 5.1: no start path spawns a process. Every
//! start file this brief names in the library and the route is read and
//! holds no process-spawning API; and the scanner is shown to fire on a line
//! that does. The route's own test gives a start whose executable would
//! leave a marker if it ran, and finds none. The command line's start file,
//! crates/lys/src/identity/start.rs, is not in the tree: its subcommand needs
//! a dispatch arm in crates/lys/src/main.rs, which is outside this brief's
//! wall, so that part of R12 stops there and is named in its dev record.

use std::error::Error;
use std::fs;
use std::path::Path;

/// Every start file the brief names, relative to the repository root.
const START_FILES: [&str; 17] = [
    "crates/lys-identity/src/start/mod.rs",
    "crates/lys-identity/src/start/request.rs",
    "crates/lys-identity/src/start/error.rs",
    "crates/lys-identity/src/start/authority.rs",
    "crates/lys-identity/src/start/checks.rs",
    "crates/lys-identity/src/start/active.rs",
    "crates/lys-identity/src/start/profile_review.rs",
    "crates/lys-identity/src/start/machine_role.rs",
    "crates/lys-identity/src/start/credentials.rs",
    "crates/lys-identity/src/start/egress.rs",
    "crates/lys-identity/src/start/profile_command.rs",
    "crates/lys-identity/src/start/launch_record.rs",
    "crates/lys-identity/src/start/state.rs",
    "crates/lys-identity/src/start/withdrawal.rs",
    "crates/lys-identity/src/start/give.rs",
    "crates/lys-identity/src/start/command.rs",
    "crates/lys-identity-server/src/start.rs",
];

/// What spawns, or asks for, a process.
const SPAWNING: [&str; 5] = [
    "std::process",
    "tokio::process",
    "Command::new",
    "fork(",
    "exec(",
];

/// How many lines of `text` name a process-spawning API, as `rg -n` counts them.
fn spawning(text: &str) -> usize {
    text.lines()
        .filter(|line| SPAWNING.iter().any(|needle| line.contains(needle)))
        .count()
}

#[test]
fn no_start_file_names_a_process_spawning_api() -> Result<(), Box<dyn Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut read = 0;
    for file in START_FILES {
        let text = fs::read_to_string(root.join(file))
            .map_err(|error| format!("{file} could not be read: {error}"))?;
        assert_eq!(spawning(&text), 0, "{file} names a process-spawning API");
        read += 1;
    }
    assert_eq!(read, START_FILES.len());
    Ok(())
}

#[test]
fn every_library_start_file_is_listed() -> Result<(), Box<dyn Error>> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/start");
    let mut found = 0;
    for entry in fs::read_dir(dir)? {
        let name = entry?.file_name().to_string_lossy().into_owned();
        let listed = format!("crates/lys-identity/src/start/{name}");
        assert!(START_FILES.contains(&listed.as_str()), "{name} is not listed");
        found += 1;
    }
    assert_eq!(found, 16);
    Ok(())
}

#[test]
fn the_scan_fires_on_a_spawning_line() {
    assert_eq!(spawning("let c = std::process::Command::new(\"x\");"), 1);
    assert_eq!(spawning("let c = 1;\nfork();\nlet d = 2;"), 1);
}
