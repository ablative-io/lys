//! ACCESS-006 R3, R5: kept cursors are looked up, bound to their asker and
//! question, expire on the operator's retention, are released whole when
//! their listing completes or resets, and are never kept beyond the
//! operator's count.

use lys_pass::membership::GrantLog;
use lys_pass::membership_pages::{
    CURSOR_EXPIRED, CURSOR_FOREIGN, CURSOR_MALFORMED, CURSOR_RESET, CURSORS_FULL,
};

use super::{Asked, CursorBook, HANDLE_BYTES};

fn log(epoch: u64) -> GrantLog {
    GrantLog {
        identity: "grants".to_owned(),
        epoch,
    }
}

fn asked<'a>(served: &'a GrantLog, question: &'a str, revision: u64) -> Asked<'a> {
    Asked {
        asker: Some("rooms"),
        question,
        served,
        revision,
    }
}

fn random(byte: u8) -> [u8; HANDLE_BYTES] {
    [byte; HANDLE_BYTES]
}

fn refusal<T>(answer: Result<T, (String, String)>) -> Option<String> {
    answer.err().map(|(name, _)| name)
}

#[test]
fn a_cursor_continues_only_its_own_question_and_asker() -> Result<(), Box<dyn std::error::Error>> {
    let served = log(0);
    let mut book = CursorBook::new(60, 4);
    let handle = book.issue(&asked(&served, "q", 3), ("ward-a", None), random(1), 100)?;
    let continued = book.take(&handle, &asked(&served, "q", 3), 101)?;
    assert_eq!(continued.after, "ward-a");
    let again = book.take(&handle, &asked(&served, "q", 4), 102)?;
    assert_eq!(again, continued, "using a cursor does not release it");
    assert_eq!(
        refusal(book.take(&handle, &asked(&served, "other", 4), 102)),
        Some(CURSOR_FOREIGN.to_owned())
    );
    let mut other_asker = asked(&served, "q", 4);
    other_asker.asker = Some("files");
    assert_eq!(
        refusal(book.take(&handle, &other_asker, 102)),
        Some(CURSOR_FOREIGN.to_owned())
    );
    assert_eq!(
        refusal(book.take("not-a-cursor", &asked(&served, "q", 4), 102)),
        Some(CURSOR_MALFORMED.to_owned())
    );
    let unknown =
        base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, random(9));
    assert_eq!(
        refusal(book.take(&unknown, &asked(&served, "q", 4), 102)),
        Some(CURSOR_EXPIRED.to_owned()),
        "a well-formed handle never issued is not kept"
    );
    Ok(())
}

#[test]
fn retention_ends_at_the_operators_seconds() -> Result<(), Box<dyn std::error::Error>> {
    let served = log(0);
    let mut book = CursorBook::new(60, 4);
    let handle = book.issue(&asked(&served, "q", 3), ("k", None), random(1), 100)?;
    assert!(book.take(&handle, &asked(&served, "q", 3), 159).is_ok());
    assert_eq!(
        refusal(book.take(&handle, &asked(&served, "q", 3), 160)),
        Some(CURSOR_EXPIRED.to_owned())
    );
    assert_eq!((book.retained(), book.released()), (0, 1));
    Ok(())
}

#[test]
fn completion_releases_the_whole_listing() -> Result<(), Box<dyn std::error::Error>> {
    let served = log(0);
    let mut book = CursorBook::new(60, 8);
    let first = book.issue(&asked(&served, "q", 3), ("a", None), random(1), 100)?;
    let chain = book.take(&first, &asked(&served, "q", 3), 100)?.chain;
    let second = book.issue(&asked(&served, "q", 3), ("b", Some(chain)), random(2), 100)?;
    let elsewhere = book.issue(&asked(&served, "r", 3), ("z", None), random(3), 100)?;
    assert_eq!(book.retained(), 3);
    book.complete(chain);
    assert_eq!((book.retained(), book.released()), (1, 2));
    for handle in [&first, &second] {
        assert_eq!(
            refusal(book.take(handle, &asked(&served, "q", 3), 101)),
            Some(CURSOR_EXPIRED.to_owned())
        );
    }
    assert!(book.take(&elsewhere, &asked(&served, "r", 3), 101).is_ok());
    Ok(())
}

#[test]
fn another_epoch_or_an_earlier_revision_resets_the_listing()
-> Result<(), Box<dyn std::error::Error>> {
    let served = log(0);
    let mut book = CursorBook::new(60, 8);
    let handle = book.issue(&asked(&served, "q", 7), ("a", None), random(1), 100)?;
    let reset = log(1);
    assert_eq!(
        refusal(book.take(&handle, &asked(&reset, "q", 7), 101)),
        Some(CURSOR_RESET.to_owned())
    );
    assert_eq!(book.retained(), 0, "a reset releases the listing");
    let handle = book.issue(&asked(&served, "q", 7), ("a", None), random(2), 100)?;
    assert_eq!(
        refusal(book.take(&handle, &asked(&served, "q", 6), 101)),
        Some(CURSOR_RESET.to_owned())
    );
    Ok(())
}

#[test]
fn no_more_than_the_operators_count_is_kept() -> Result<(), Box<dyn std::error::Error>> {
    let served = log(0);
    let mut book = CursorBook::new(60, 2);
    book.issue(&asked(&served, "q", 1), ("a", None), random(1), 100)?;
    book.issue(&asked(&served, "q", 1), ("b", None), random(2), 100)?;
    assert_eq!(
        refusal(book.issue(&asked(&served, "q", 1), ("c", None), random(3), 100)),
        Some(CURSORS_FULL.to_owned())
    );
    assert_eq!(book.retained(), 2);
    book.issue(&asked(&served, "q", 1), ("c", None), random(3), 160)?;
    assert_eq!(book.retained(), 1, "expiry made room");
    Ok(())
}
