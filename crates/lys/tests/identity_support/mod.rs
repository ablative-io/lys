//! Support for the container-backed identity targets (DIRECTORY-002 R1 to
//! R3): [`fixtures`] a fresh venue per target, [`compose`] the venue's
//! `docker compose`, [`server`] a running deployment.
//!
//! Every target that includes this module needs a container runtime and
//! refuses by name (`container_runtime_missing`) without one. None of them
//! runs in `cargo test --workspace --all-features`: each is declared
//! `test = false` in crates/lys/Cargo.toml and runs only on the identity leg
//! of .land/gates.sh.

pub mod compose;
pub mod fixtures;
pub mod server;
