#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

/// Every `.rs` file in this crate's `src`, as (path, contents).
fn crate_sources() -> Vec<(std::path::PathBuf, String)> {
    fn walk(dir: &Path, into: &mut Vec<(std::path::PathBuf, String)>) {
        for entry in std::fs::read_dir(dir).expect("src must be readable") {
            let path = entry.expect("a readable directory entry").path();
            if path.is_dir() {
                walk(&path, into);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                let text = std::fs::read_to_string(&path).expect("a readable source file");
                into.push((path, text));
            }
        }
    }
    let mut sources = Vec::new();
    walk(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut sources,
    );
    assert!(
        sources.len() >= 10,
        "the scan found only {} files, so it is not reading this crate",
        sources.len()
    );
    sources
}

/// This binary never claims to have authenticated anybody.
///
/// `AuthenticatedPeer::verified_by_transport` asserts that a peer demonstrated
/// possession of a private key before the call. This CLI performs no handshake
/// and observes no peer, so every use of that constructor here would be a claim
/// nobody made — and the library made it a *named* constructor precisely so the
/// mistake has to be typed out where a reviewer sees it. This is the lexical
/// gate that keeps it untyped.
///
/// The needles are assembled with `concat!` so this file contains neither
/// literal: a gate that matched itself would fail for the wrong reason, and a
/// control that matched itself would pass without ever reading another file.
#[test]
fn the_authenticated_arm_is_never_constructed_and_the_asserted_arm_is() {
    let forbidden = concat!("verified_by_", "transport(");
    let expected = concat!("AssertedBy", "Submitter(");

    let sources = crate_sources();
    let offenders: Vec<_> = sources
        .iter()
        .filter(|(_, text)| text.contains(forbidden))
        .map(|(path, _)| path.display().to_string())
        .collect();
    assert!(
        offenders.is_empty(),
        "this CLI authenticates nobody, so it must not construct the authenticated arm: {offenders:?}"
    );

    // The positive control. Without it, a scan that read no files at all — a
    // wrong path, an extension typo — would report the same clean result.
    let asserting: Vec<_> = sources
        .iter()
        .filter(|(_, text)| text.contains(expected))
        .map(|(path, _)| path.display().to_string())
        .collect();
    assert!(
        !asserting.is_empty(),
        "the scan found no use of the asserted arm either, so it is not reading source"
    );
}

/// The disclosure this CLI prints says what the plan requires it to say.
///
/// The second party is `docs/design/lys-anchor/BUILD-PLAN.md` §2.2, which is
/// where the requirement lives and which this crate cannot edit by changing its
/// own constant. A missing plan is a hard failure rather than a skip: a gate
/// that quietly does nothing when its input is absent is how the first one in
/// this repository vanished.
#[test]
fn the_standalone_disclosure_uses_the_plans_words() {
    let plan =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/design/lys-anchor/BUILD-PLAN.md");
    let text = std::fs::read_to_string(&plan)
        .unwrap_or_else(|err| panic!("the build plan must be readable at {plan:?}: {err}"));
    assert!(
        text.contains("equivocate undetectably"),
        "the plan no longer states the requirement this constant renders"
    );
    assert!(
        STANDALONE_DISCLOSURE.contains("equivocate undetectably"),
        "the disclosure dropped the phrase the plan requires: {STANDALONE_DISCLOSURE}"
    );
    assert!(
        STANDALONE_DISCLOSURE.contains("no witnesses"),
        "the disclosure must name the condition, not only the consequence"
    );
}

/// A credential travels as an assertion; its absence is `Unidentified`.
///
/// Both arms are checked, and the third is checked by the lexical gate above —
/// a match on two of three variants would look complete here and would not be.
#[cfg(feature = "unstable-anchor")]
#[test]
fn a_credential_is_asserted_and_its_absence_is_unidentified() {
    assert_eq!(submitter_context(None), SubmitterContext::Unidentified);
    let der = b"not really a certificate";
    match submitter_context(Some(der)) {
        SubmitterContext::AssertedBySubmitter(bytes) => assert_eq!(bytes, der),
        other => panic!("a CLI credential must be an assertion, got {other:?}"),
    }
}
