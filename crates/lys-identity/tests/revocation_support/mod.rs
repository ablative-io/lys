//! Shared fixtures for the certificate revocation tests.

pub mod fixtures;

pub use fixtures::{Issuer, TestResult, issue, new_issuer, open_log, seed};
