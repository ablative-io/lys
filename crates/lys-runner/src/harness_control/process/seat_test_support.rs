#![cfg(test)]
//! A scoped entry fixture exercises the production process proof.
use super::{RunnerError, qualification};
use std::path::PathBuf;

pub(crate) fn entry(path: PathBuf) -> Result<impl Drop, RunnerError> {
    qualification::admit(qualification::Candidate {
        adapter: "acp/1",
        version: "seat-program/v1",
        entry: path,
    })
}
