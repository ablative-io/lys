#![cfg(test)]
//! The setup code's handover: the browser gets it in the address fragment,
//! and without a browser it goes to an owner-only file named in one line.

use std::cell::RefCell;
use std::error::Error;
use std::os::unix::fs::PermissionsExt;

use super::{Purpose, address_with, digest, hand_over, pending_text};
use crate::commands::output::Emitter;
use crate::identity::install::layout::Layout;

type TestResult = Result<(), Box<dyn Error>>;

const CODE: &str = "Zq81mTn4Rw0pLk7Hs2Vb6Xc3Yd9Fg5Aj";

#[test]
fn the_code_rides_only_in_the_fragment_of_the_address() {
    let address = address_with(CODE, &super::super::ports::Ports::default().setup_url());
    assert_eq!(address, format!("http://localhost:8490/setup#code={CODE}"));
    let (before, after) = address.split_once('#').unwrap_or_default();
    assert!(!before.contains(CODE), "the code never reaches a server");
    assert_eq!(after, format!("code={CODE}"));
}

#[test]
fn a_browser_that_takes_the_address_leaves_no_file() -> TestResult {
    let root = tempfile::TempDir::new()?;
    let layout = Layout::at(root.path().to_path_buf());
    let handed = RefCell::new(Vec::new());
    let open = |address: &str| {
        handed.borrow_mut().push(address.to_owned());
        true
    };
    hand_over(&layout, CODE, &open, &mut Emitter::new(true))?;
    assert_eq!(
        handed.borrow().as_slice(),
        [address_with(
            CODE,
            &super::super::ports::Ports::default().setup_url()
        )]
    );
    assert!(!layout.headless_setup_code().exists());
    Ok(())
}

#[test]
fn without_a_browser_the_code_goes_to_an_owner_only_file() -> TestResult {
    let root = tempfile::TempDir::new()?;
    let layout = Layout::at(root.path().to_path_buf());
    hand_over(&layout, CODE, &|_: &str| false, &mut Emitter::new(true))?;
    let path = layout.headless_setup_code();
    assert_eq!(std::fs::read_to_string(&path)?, CODE);
    assert_eq!(
        std::fs::metadata(&path)?.permissions().mode() & 0o777,
        0o600
    );
    Ok(())
}

#[test]
fn without_a_browser_the_json_output_names_the_code_file_never_the_code() -> TestResult {
    let root = tempfile::TempDir::new()?;
    let layout = Layout::at(root.path().to_path_buf());
    let mut emitter = Emitter::new(true);
    hand_over(&layout, CODE, &|_: &str| false, &mut emitter)?;
    let value = emitter.into_value().ok_or("JSON mode renders one object")?;
    let path = layout.headless_setup_code();
    assert_eq!(
        value["setup_code_file"],
        path.display().to_string().as_str()
    );
    assert!(!value.to_string().contains(CODE));
    Ok(())
}

#[test]
fn the_pending_file_keeps_the_purpose_and_the_digest_never_the_code() -> TestResult {
    let text = pending_text(Purpose::Password, CODE);
    assert!(!text.contains(CODE));
    let pending: serde_json::Value = serde_json::from_str(&text)?;
    assert_eq!(pending["purpose"], "password");
    assert_eq!(pending["sha256"], digest(CODE).as_str());
    assert_eq!(
        digest("abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        "the digest is SHA-256, as the directory service checks it"
    );
    Ok(())
}
