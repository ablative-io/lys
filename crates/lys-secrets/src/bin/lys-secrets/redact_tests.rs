#![cfg(test)]
//! Counting gates on the upstream answer (SECRETS-005 R5). Five hidden
//! values in a mebibyte answer are hidden in one pass, exactly as one pass
//! per value hid them; and an answer past the cap is refused by name, never
//! cut short.

use lys_secrets::Secret;

use crate::redact::{REDACTED, Redactor, capped};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const MIB: usize = 1024 * 1024;
/// Where the answer would be split, were it read in pieces this long.
const PIECE: usize = 64 * 1024;

/// The redaction as it was: one pass over the whole answer for each hidden
/// value in turn, and how many places the passes stood on together.
fn one_pass_each(haystack: &[u8], hidden: &[&[u8]]) -> (Vec<u8>, usize) {
    let mut text = haystack.to_vec();
    let mut stood = 0;
    for needle in hidden {
        let mut out = Vec::with_capacity(text.len());
        let mut at = 0;
        while at < text.len() {
            stood += 1;
            if text[at..].starts_with(needle) {
                out.extend_from_slice(REDACTED);
                at += needle.len();
            } else {
                out.push(text[at]);
                at += 1;
            }
        }
        text = out;
    }
    (text, stood)
}

#[test]
fn five_hidden_values_in_a_mebibyte_answer_are_hidden_in_one_pass() {
    let values: [&[u8]; 5] = [
        b"first-hidden-value",
        b"second-hidden-value-longer",
        b"third-hv",
        b"fourth-hidden",
        b"fifth-hidden-value-x",
    ];
    let secrets: Vec<Secret> = values
        .iter()
        .map(|value| Secret::from_slice(value))
        .collect();
    let hidden: Vec<&Secret> = secrets.iter().collect();
    let mut answer = Vec::with_capacity(MIB);
    let mut placed = 0;
    let mut row = 0usize;
    let mut straddled = false;
    while answer.len() < MIB - 64 {
        if !straddled && answer.len() + 64 >= PIECE - 5 {
            answer.resize(PIECE - 5, b'.');
            answer.extend_from_slice(values[1]);
            placed += 1;
            straddled = true;
        }
        answer.extend_from_slice(format!("{{\"row\":{row},\"text\":\"").as_bytes());
        if row % 97 == 0 {
            answer.extend_from_slice(values[row % values.len()]);
            placed += 1;
        }
        answer.extend_from_slice(b"\"}\n");
        row += 1;
    }
    answer.resize(MIB, b'.');
    assert_eq!(answer.len(), MIB);
    assert!(straddled);

    let redactor = Redactor::new(&hidden);
    let (redacted, stood) = redactor.redact_counted(&answer);
    let (before, stood_before) = one_pass_each(&answer, &values);
    assert!(
        stood <= answer.len(),
        "the answer is scanned once: {stood} places for {} bytes",
        answer.len()
    );
    assert!(
        stood_before > 4 * (MIB - placed * 32),
        "the redaction as it was scanned the answer once per hidden value"
    );
    assert_eq!(redacted, before, "every value hidden as before");
    let shown = redacted
        .windows(REDACTED.len())
        .filter(|window| *window == REDACTED)
        .count();
    assert_eq!(shown, placed);
    for value in values {
        assert!(!redacted.windows(value.len()).any(|piece| piece == value));
    }
    assert!(!redactor.contains(&redacted));
}

#[tokio::test]
async fn an_answer_past_the_cap_is_refused_by_name_and_never_cut_short() -> TestResult {
    let redactor = Redactor::new(&[]);
    let over = reqwest::Response::from(axum::http::Response::new(vec![b'a'; 1025]));
    let refused = capped(over, 1024, &redactor).await.err();
    assert_eq!(refused.map(|error| error.name()), Some("AnswerTooLarge"));
    let whole = reqwest::Response::from(axum::http::Response::new(vec![b'a'; 1024]));
    assert_eq!(capped(whole, 1024, &redactor).await?.len(), 1024);
    Ok(())
}
