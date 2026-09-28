#![cfg(test)]
//! DIRECTORY-029 R4, CONFORMANCE 5.2: its profile version is reviewed reads
//! the review record the roles card `Ink1H1Os` makes, and nothing else counts
//! as a review.

use lys_identity::start::profile_review::{ProfileReviews, Review, check};

/// The review record `Ink1H1Os` keeps, or its absence.
struct Reviews(Option<Vec<String>>);

impl ProfileReviews for Reviews {
    fn review(&self, profile_version: &str) -> Option<Review> {
        let reviewed = self.0.as_ref()?;
        Some(if reviewed.iter().any(|held| held == profile_version) {
            Review::Reviewed
        } else {
            Review::NotReviewed
        })
    }
}

#[test]
fn a_reviewed_version_passes() {
    assert_eq!(check(&Reviews(Some(vec!["pv-fixture-1".to_owned()])), "pv-fixture-1"), Ok(()));
}

#[test]
fn a_version_with_no_review_is_refused_by_name() {
    let refusal = check(&Reviews(Some(vec!["pv-fixture-1".to_owned()])), "pv-fixture-2")
        .expect_err("no review is on record");
    assert_eq!(refusal.name(), "profile_version_not_reviewed");
    assert!(refusal.to_string().contains("pv-fixture-2"), "{refusal}");
}

#[test]
fn with_no_review_record_the_check_names_its_card() {
    let refusal = check(&Reviews(None), "pv-fixture-1").expect_err("the record is missing");
    assert_eq!(refusal.name(), "check_record_missing");
    let words = refusal.to_string();
    assert!(words.contains("its profile version is reviewed"), "{words}");
    assert!(words.contains("Ink1H1Os"), "{words}");
}
