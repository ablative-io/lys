//! The containment helper travels in the runner's binary transaction.

use std::path::Path;

use super::super::error::{ErrorKind, IdentityError, IdentityResult};
use super::super::install::layout::Layout;

const NAME: &str = "lys-containment-entry";

/// A helper need not have a version command; its bytes are checked before stopping.
pub(super) fn check(layout: &Layout, from: &Path) -> IdentityResult<()> {
    let source = from.join(NAME);
    let bytes = match std::fs::read(&source) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound
            && !layout.binary(NAME).try_exists().map_err(|error| super::io("find containment helper", &source, &error))? => return Ok(()),
        Err(error) => return Err(IdentityError::new(
            ErrorKind::BinaryMissing,
            "prepare runner helper",
            NAME,
            format!("runner_helper_missing: the incoming build must include the installed containment helper: {error}"),
        ).at(&source)),
    };
    if bytes.is_empty() {
        return Err(IdentityError::new(
            ErrorKind::BinaryMissing,
            "prepare runner helper",
            NAME,
            "runner_helper_empty: the incoming containment helper is empty",
        )
        .at(&source));
    }
    Ok(())
}

pub(super) fn place(layout: &Layout, from: &Path, names: &[&str]) -> IdentityResult<()> {
    if names.contains(&"lys")
        && from
            .join(NAME)
            .try_exists()
            .map_err(|error| super::io("find containment helper", &from.join(NAME), &error))?
    {
        super::swap::place_binary(&from.join(NAME), &layout.bin_dir(), NAME)?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "companion_tests.rs"]
mod tests;
