//! Candidate admission is scoped to the test thread and absent from production.

use crate::error::RunnerError;
use std::cell::RefCell;
use std::path::PathBuf;

#[path = "../../../tests/qualify_launched.rs"]
mod launched;

pub(super) struct Candidate {
    pub(super) adapter: &'static str,
    pub(super) version: &'static str,
    pub(super) entry: PathBuf,
}

thread_local! {
    static CANDIDATE: RefCell<Option<Candidate>> = const { RefCell::new(None) };
}

pub(super) struct Admission;

pub(super) fn admit(candidate: Candidate) -> Result<Admission, RunnerError> {
    CANDIDATE.with(|slot| {
        let mut slot = slot
            .try_borrow_mut()
            .map_err(|error| super::failed("qualification_seam_busy", error))?;
        if slot.is_some() {
            return Err(super::failed(
                "qualification_seam_busy",
                "this thread already admits a candidate",
            ));
        }
        *slot = Some(candidate);
        Ok(Admission)
    })
}

impl Drop for Admission {
    fn drop(&mut self) {
        CANDIDATE.with(|slot| match slot.try_borrow_mut() {
            Ok(mut slot) => {
                drop(slot.take());
            }
            Err(error) => {
                crate::error::said(&format!("qualification_seam_cleanup_failed: {error}"));
            }
        });
    }
}

pub(super) fn approved(adapter: &str, version: &str) -> Result<bool, RunnerError> {
    CANDIDATE.with(|slot| {
        let slot = slot
            .try_borrow()
            .map_err(|error| super::failed("qualification_seam_busy", error))?;
        Ok(slot
            .as_ref()
            .is_some_and(|candidate| candidate.adapter == adapter && candidate.version == version))
    })
}

pub(super) fn entry() -> Result<Option<PathBuf>, RunnerError> {
    CANDIDATE.with(|slot| {
        let slot = slot
            .try_borrow()
            .map_err(|error| super::failed("qualification_seam_busy", error))?;
        Ok(slot.as_ref().map(|candidate| candidate.entry.clone()))
    })
}
