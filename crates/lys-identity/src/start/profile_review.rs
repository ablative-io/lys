//! Its profile version is reviewed: read from the review record the roles
//! card `Ink1H1Os` makes for profile versions.
//!
//! A version with no review on record fails as
//! `profile_version_not_reviewed`. While `Ink1H1Os`'s record does not exist
//! the check answers `check_record_missing`, naming `Ink1H1Os`. A sample
//! review, a mock-up record or the absence of a refusal is never a review:
//! only the owner's record answering [`Review::Reviewed`] passes.

use crate::start::checks::Check;
use crate::start::error::Refusal;

/// What the review record holds for one profile version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Review {
    /// A review of the version is on record.
    Reviewed,
    /// The record holds no review of the version.
    NotReviewed,
}

/// The review record `Ink1H1Os` keeps for profile versions.
pub trait ProfileReviews {
    /// What the record holds for `profile_version`, or `None` when the
    /// review record does not exist.
    fn review(&self, profile_version: &str) -> Option<Review>;
}

/// Its profile version is reviewed: passes only on a review on record.
pub fn check(reviews: &dyn ProfileReviews, profile_version: &str) -> Result<(), Refusal> {
    match reviews.review(profile_version) {
        None => Err(Check::ProfileVersionIsReviewed.record_missing()),
        Some(Review::Reviewed) => Ok(()),
        Some(Review::NotReviewed) => Err(Refusal::ProfileVersionNotReviewed {
            profile_version: profile_version.to_owned(),
        }),
    }
}
