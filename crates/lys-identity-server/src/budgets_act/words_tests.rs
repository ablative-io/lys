//! A context warning's figures in percent and in tokens.

use super::{compaction_figures, context_figures, grouped};

#[test]
fn tokens_are_grouped_by_thousands() {
    assert_eq!(grouped(0), "0");
    assert_eq!(grouped(999), "999");
    assert_eq!(grouped(1_000), "1,000");
    assert_eq!(grouped(400_000), "400,000");
    assert_eq!(grouped(1_000_000), "1,000,000");
}

#[test]
fn a_reported_window_shows_the_tokens_beside_the_percentage() {
    assert_eq!(
        context_figures(&20.into(), Some(200_000), Some(1_000_000)),
        "20% of your context window (200,000 of 1,000,000 tokens)"
    );
    assert_eq!(
        compaction_figures(&40.into(), Some(1_000_000)),
        Ok("40% (400,000 tokens)".to_owned())
    );
    assert_eq!(
        compaction_figures(&40.into(), Some(200_000)),
        Ok("40% (80,000 tokens)".to_owned())
    );
}

#[test]
fn an_unreported_window_shows_the_percentage_alone() {
    assert_eq!(
        context_figures(&38.into(), None, None),
        "38% of your context window"
    );
    assert_eq!(compaction_figures(&40.into(), None), Ok("40%".to_owned()));
}
