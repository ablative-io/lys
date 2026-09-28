//! `lys identity`: prepare, configure, install, upgrade and check the
//! standalone identity product. The work is `lys-install`'s, the one install
//! Lys.app runs too; this module declares the arguments and re-exports.

pub mod cli;
pub mod install;

pub use cli::IdentityCommand;
pub use lys_install::{IdentityError, configure, health, prepare, upgrade};
