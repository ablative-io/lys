//! The secrets broker's trust in the identity service, set on every install
//! so the service's screens may ask the broker on a signed-in person's behalf.

use super::super::error::IdentityResult;
use super::layout::Layout;
use super::services::{run_to_end, sibling};

/// Trusts the screen service `name` with the secrets broker by the public
/// half of the service key, replacing any key it was trusted by before, so
/// the service may ask on a signed-in person's behalf.
pub fn trust(layout: &Layout, name: &str) -> IdentityResult<()> {
    let program = sibling("lys-secrets")?;
    let public_key = run_to_end(
        &program,
        &[
            "service-key".to_string(),
            "--key".to_string(),
            layout.service_key().display().to_string(),
        ],
        "identity service key",
    )?;
    run_to_end(
        &program,
        &[
            "trust-service".to_string(),
            "--root".to_string(),
            layout.broker_root().display().to_string(),
            "--keys".to_string(),
            layout.broker_keys().display().to_string(),
            "--name".to_string(),
            name.to_string(),
            "--public-key".to_string(),
            public_key.trim().to_string(),
        ],
        "secrets broker trust",
    )?;
    Ok(())
}
