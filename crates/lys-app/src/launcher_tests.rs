#![cfg(test)]

use lys_install::install::layout::Layout;

use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A second opening while a work runs is shown the first one's page and
/// claims nothing: one install, never two.
#[cfg(unix)]
#[test]
fn a_second_opening_is_shown_the_running_works_page() -> TestResult {
    let root = tempfile::tempdir()?;
    let layout = Layout::at(root.path().join("identity"));
    let Claim::Ours(mut first) = claim(&layout)? else {
        return Err("the first opening did not claim the install".into());
    };
    first.publish(4242)?;
    let mut theirs = 0;
    for _ in 0..2 {
        match claim(&layout)? {
            Claim::Theirs(port) => {
                assert_eq!(port, 4242);
                theirs += 1;
            }
            Claim::Ours(_) => return Err("a second work claimed the install".into()),
        }
    }
    assert_eq!(theirs, 2);
    drop(first);
    assert!(matches!(claim(&layout)?, Claim::Ours(_)));
    Ok(())
}

#[test]
fn the_port_is_the_works_first_line() {
    assert_eq!(read_port(&mut &b"51234\n"[..]), Some(51234));
    assert_eq!(read_port(&mut &b""[..]), None);
    assert_eq!(read_port(&mut &b"not a port\n"[..]), None);
}

#[test]
fn the_page_is_on_the_loopback_address() {
    assert_eq!(page_url(51234), "http://127.0.0.1:51234/install");
}

#[test]
fn the_log_is_in_the_installs_logs() {
    let layout = Layout::at("/root".into());
    assert_eq!(
        log_path(&layout),
        std::path::Path::new("/root/logs/lys-app.log")
    );
}
