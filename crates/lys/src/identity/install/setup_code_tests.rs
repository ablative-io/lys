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

/// Hand the code over with a browser that takes it, answering every address
/// the browser was given and the JSON output.
fn handed_over(layout: &Layout) -> Result<(Vec<String>, serde_json::Value), Box<dyn Error>> {
    let handed = RefCell::new(Vec::new());
    let open = |address: &str| {
        handed.borrow_mut().push(address.to_owned());
        true
    };
    let mut emitter = Emitter::new(true);
    hand_over(layout, CODE, &open, &mut emitter)?;
    let value = emitter.into_value().ok_or("JSON mode renders one object")?;
    Ok((handed.into_inner(), value))
}

#[test]
fn a_browser_that_takes_the_address_leaves_no_file() -> TestResult {
    let root = tempfile::TempDir::new()?;
    let layout = Layout::at(root.path().to_path_buf());
    std::fs::write(
        layout.deployment_config(),
        "[issuer]\npublic_origin = \"https://lys.example.test\"\n",
    )?;
    let (handed, _) = handed_over(&layout)?;
    assert_eq!(handed.len(), 1);
    assert!(!layout.headless_setup_code().exists());
    Ok(())
}

#[test]
fn a_named_public_origin_is_the_setup_page_the_browser_opens() -> TestResult {
    let root = tempfile::TempDir::new()?;
    let layout = Layout::at(root.path().to_path_buf());
    std::fs::write(
        layout.deployment_config(),
        "[issuer]\npublic_origin = \"https://lys.example.test/\"\nlisten_port = 8443\n",
    )?;
    let (handed, value) = handed_over(&layout)?;
    assert_eq!(
        handed,
        [format!("https://lys.example.test/setup#code={CODE}")]
    );
    assert!(handed.iter().all(|address| !address.contains("localhost")));
    assert!(value.get("address_fallback").is_none());
    Ok(())
}

#[test]
fn a_loopback_public_origin_is_used_as_named_without_a_fallback() -> TestResult {
    let root = tempfile::TempDir::new()?;
    let layout = Layout::at(root.path().to_path_buf());
    std::fs::write(
        layout.deployment_config(),
        "[issuer]\npublic_origin = \"http://127.0.0.1:19001\"\n",
    )?;
    let (handed, value) = handed_over(&layout)?;
    assert_eq!(
        handed,
        [format!("http://127.0.0.1:19001/setup#code={CODE}")]
    );
    assert!(value.get("address_fallback").is_none());
    Ok(())
}

#[test]
fn without_a_deployment_the_local_address_is_opened_and_the_reason_said() -> TestResult {
    let root = tempfile::TempDir::new()?;
    let layout = Layout::at(root.path().to_path_buf());
    let (handed, value) = handed_over(&layout)?;
    assert_eq!(
        handed,
        [address_with(
            CODE,
            &super::super::ports::Ports::default().setup_url()
        )]
    );
    assert_eq!(handed, [format!("http://localhost:8490/setup#code={CODE}")]);
    let reason = value["address_fallback"]
        .as_str()
        .ok_or("the fallback is named in the output")?;
    assert!(reason.contains("using the local address http://localhost:8490"));
    assert!(reason.contains("names no public origin"));
    assert!(reason.contains(&layout.deployment_config().display().to_string()));
    Ok(())
}

#[test]
fn a_deployment_without_a_public_origin_says_so() -> TestResult {
    let root = tempfile::TempDir::new()?;
    let layout = Layout::at(root.path().to_path_buf());
    std::fs::write(
        layout.deployment_config(),
        "[deployment]\nproject = \"lys\"\n",
    )?;
    let address =
        super::super::ports::PersonAddress::load(&layout, super::super::ports::Ports::default())?;
    assert_eq!(address.origin, "http://localhost:8490");
    assert!(
        address
            .fallback
            .as_deref()
            .is_some_and(|reason| reason.ends_with("names no issuer.public_origin"))
    );
    Ok(())
}

#[test]
fn an_unreadable_deployment_is_refused_never_guessed_local() -> TestResult {
    let root = tempfile::TempDir::new()?;
    let layout = Layout::at(root.path().to_path_buf());
    std::fs::write(layout.deployment_config(), "[issuer\n")?;
    let refused =
        super::super::ports::PersonAddress::load(&layout, super::super::ports::Ports::default());
    assert!(
        refused.is_err(),
        "a broken deployment.toml is refused by name"
    );
    Ok(())
}

#[test]
fn without_a_browser_the_line_names_the_public_setup_page() -> TestResult {
    let root = tempfile::TempDir::new()?;
    let layout = Layout::at(root.path().to_path_buf());
    std::fs::write(
        layout.deployment_config(),
        "[issuer]\npublic_origin = \"https://lys.example.test\"\n",
    )?;
    let address = super::super::ports::PersonAddress::load(
        &layout,
        super::super::ports::Ports::load(&layout)?,
    )?;
    let line = super::headless_line(&layout.headless_setup_code(), &address.setup_url());
    assert!(line.contains("open https://lys.example.test/setup and enter it"));
    assert!(!line.contains("localhost"));
    let mut emitter = Emitter::new(true);
    hand_over(&layout, CODE, &|_: &str| false, &mut emitter)?;
    let value = emitter.into_value().ok_or("JSON mode renders one object")?;
    assert!(value.get("address_fallback").is_none());
    assert!(!value.to_string().contains(CODE));
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
