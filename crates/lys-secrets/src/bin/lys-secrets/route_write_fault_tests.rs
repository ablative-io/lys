#![cfg(test)]
//! A partial route-file write must preserve existing routes when the layout reopens.
use std::cell::RefCell;
use std::path::{Path, PathBuf};

use lys_secrets::SecretsError;

use crate::files::{Layout, Route};

thread_local! {
    static FAIL_PATH: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}

/// Model one underlying write accepting a prefix and then returning an I/O error.
/// This seam exists only in test builds, on the actual file-write path.
pub(crate) fn partial_write(path: &Path, bytes: &[u8]) -> Result<bool, SecretsError> {
    let fired = FAIL_PATH.with(|held| {
        let mut held = held.borrow_mut();
        if held.as_deref() == Some(path) {
            held.take();
            true
        } else {
            false
        }
    });
    if fired {
        std::fs::write(path, &bytes[..bytes.len() / 2]).map_err(|source| SecretsError::Io {
            context: format!("injecting partial write to {}", path.display()),
            source,
        })?;
    }
    Ok(fired)
}

fn route(upstream: &str) -> Route {
    Route {
        upstream: upstream.to_owned(),
        header: "authorization".to_owned(),
        prefix: "Bearer ".to_owned(),
        spend_header: None,
    }
}

#[test]
fn partial_routes_write_preserves_existing_routes_after_reopen()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("broker");
    let keys = dir.path().join("keys");
    let layout = Layout::new(&root, &keys);
    layout.prepare()?;
    layout.add_route("existing-api", route("http://127.0.0.1:6010/api"))?;
    let before = serde_json::to_value(layout.routes()?.as_ref())?;
    // Match only this test's path; parallel tests cannot consume the fault.
    FAIL_PATH.with(|held| *held.borrow_mut() = Some(root.join("routes.json")));
    let answer = layout.add_route("new-api", route("http://127.0.0.1:8490/api"));
    let unconsumed = FAIL_PATH.with(|held| held.borrow_mut().take());
    assert!(unconsumed.is_none(), "partial-write seam was not exercised");
    assert!(
        matches!(answer, Err(SecretsError::Io { .. })),
        "partial write must refuse by name"
    );
    drop(layout);
    let reopened = Layout::new(&root, &keys);
    let after = reopened.routes();
    assert!(
        after.is_ok(),
        "refused new route corrupted existing routes on reopen: {after:?}"
    );
    assert_eq!(serde_json::to_value(after?.as_ref())?, before);
    Ok(())
}
