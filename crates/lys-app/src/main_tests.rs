#![cfg(test)]

use std::path::PathBuf;

use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// The arguments `words` name.
fn args(words: &[&str]) -> Result<Args, Refusal> {
    parse(words.iter().map(|word| (*word).to_string()))
}

#[test]
fn each_mode_is_read_from_its_argument() -> TestResult {
    assert_eq!(args(&[])?.mode, Mode::Open);
    assert_eq!(args(&["--serve"])?.mode, Mode::Serve);
    assert_eq!(args(&["--at-login"])?.mode, Mode::AtLogin);
    assert_eq!(args(&["--uninstall"])?.mode, Mode::Uninstall(false));
    assert_eq!(
        args(&["--uninstall", "--remove-data"])?.mode,
        Mode::Uninstall(true)
    );
    assert_eq!(args(&["--version"])?.mode, Mode::Version);
    let rooted = args(&["--serve", "--root", "/data/lys"])?;
    assert_eq!(rooted.root, Some(PathBuf::from("/data/lys")));
    Ok(())
}

#[test]
fn the_systems_process_serial_number_is_passed_over() -> TestResult {
    assert_eq!(args(&["-psn_0_12345"])?.mode, Mode::Open);
    Ok(())
}

#[test]
fn anything_else_is_refused_by_name() {
    let refused = [
        vec!["--shell"],
        vec!["--remove-data"],
        vec!["--serve", "--remove-data"],
        vec!["--root"],
    ];
    let mut named = 0;
    for words in refused {
        let refusal = args(&words).err();
        assert_eq!(
            refusal.map(|refusal| refusal.name),
            Some("argument_unknown"),
            "{words:?}"
        );
        named += 1;
    }
    assert_eq!(named, 4);
}

#[test]
fn the_version_names_the_crate_and_the_build() {
    assert!(VERSION.starts_with(env!("CARGO_PKG_VERSION")));
    assert!(VERSION.contains(env!("LYS_BUILD")));
}
