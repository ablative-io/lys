//! `lys identity install`: a thin call into `lys-install`, whose install
//! Lys.app runs as well. The steps are reported through the install's own
//! lines, so this caller hears nothing more.

use lys_install::error::IdentityResult;
pub use lys_install::install::Options;

/// Runs `lys identity install`: the install's own lines say each step, so
/// the step notices are not needed here.
pub fn run(options: &Options, json: bool) -> IdentityResult<()> {
    lys_install::install::run(options, json, &mut |_| {})
}
