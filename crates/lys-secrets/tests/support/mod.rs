//! What the integration tests share.
//!
//! The in-process world of CONFORMANCE rows 7.6 and 7.8 lives beside this
//! module in `fixture.rs` and `leases.rs`. The tests that use it reach each
//! file by its path, so a test compiles only the support it uses and the
//! served binary's support is never compiled into a test that does not
//! start it.

pub mod served;
