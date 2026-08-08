#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::cell::RefCell;

use super::*;

/// Records the concrete policy type each arm actually constructed.
///
/// Keyed on `type_name`, which comes from the compiler rather than from
/// anything this file wrote down — so an arm that silently constructed the
/// wrong policy is visible. A test that only asserted "it ran" could not tell
/// `MaxSize` from `AcceptAll`.
struct RecordPolicy<'a> {
    seen: &'a RefCell<Vec<&'static str>>,
}

impl AnchorTask for RecordPolicy<'_> {
    fn run<P: AdmissionPolicy>(self, _policy: P) -> CliResult<()> {
        self.seen.borrow_mut().push(std::any::type_name::<P>());
        Ok(())
    }
}

fn args(admit: AdmitPolicy) -> AdmissionArgs {
    AdmissionArgs {
        admit,
        max_bytes: None,
        issuer_public_key: None,
        subject_key: Vec::new(),
    }
}

/// Each `--admit` value reaches the library type it names, and the three are
/// distinct.
///
/// The distinctness assertion is the one that matters: three arms that all
/// constructed `AcceptAll` would satisfy "the task ran" three times over.
#[test]
fn each_admit_value_builds_the_policy_it_names() {
    let seen = RefCell::new(Vec::new());

    with_policy(&args(AdmitPolicy::AcceptAll), RecordPolicy { seen: &seen }).unwrap();

    let mut max_size = args(AdmitPolicy::MaxSize);
    max_size.max_bytes = Some(4096);
    with_policy(&max_size, RecordPolicy { seen: &seen }).unwrap();

    let mut recognised = args(AdmitPolicy::RecognisedCertificate);
    recognised.issuer_public_key = Some("ab".repeat(32));
    with_policy(&recognised, RecordPolicy { seen: &seen }).unwrap();

    let seen = seen.into_inner();
    assert_eq!(seen.len(), 3, "every arm must have fired");
    assert!(seen[0].ends_with("AcceptAll"), "got {}", seen[0]);
    assert!(seen[1].ends_with("MaxSize"), "got {}", seen[1]);
    assert!(
        seen[2].ends_with("RecognisedCertificate"),
        "got {}",
        seen[2]
    );
    let mut distinct = seen.clone();
    distinct.sort_unstable();
    distinct.dedup();
    assert_eq!(distinct.len(), 3, "the three arms must differ: {seen:?}");
}

/// A flag the chosen policy does not read is refused, not ignored.
///
/// Every case is counted. An operator writing `--admit accept-all --max-bytes N`
/// believes a limit is in force; admitting the invocation would run an anchor
/// under a rule its operator did not choose.
#[test]
fn a_flag_the_policy_does_not_read_is_refused() {
    let mut cases = Vec::new();

    let mut limit_without_max_size = args(AdmitPolicy::AcceptAll);
    limit_without_max_size.max_bytes = Some(4096);
    cases.push((limit_without_max_size, "--max-bytes"));

    let mut issuer_without_certificate = args(AdmitPolicy::AcceptAll);
    issuer_without_certificate.issuer_public_key = Some("ab".repeat(32));
    cases.push((issuer_without_certificate, "--issuer-public-key"));

    let mut subject_without_certificate = args(AdmitPolicy::MaxSize);
    subject_without_certificate.max_bytes = Some(1);
    subject_without_certificate.subject_key = vec!["ab".repeat(32)];
    cases.push((subject_without_certificate, "--subject-key"));

    let mut refusals = 0;
    for (case, expected_flag) in cases {
        let seen = RefCell::new(Vec::new());
        let err = with_policy(&case, RecordPolicy { seen: &seen }).unwrap_err();
        assert!(
            matches!(err, CliError::AdmissionArgumentIgnored { flag, .. } if flag == expected_flag),
            "got: {err}"
        );
        assert!(
            seen.into_inner().is_empty(),
            "the task must not have run for {expected_flag}"
        );
        refusals += 1;
    }
    assert_eq!(refusals, 3, "every case must have been exercised");
}

/// The positive control for the refusals above: each flag is accepted by the
/// policy that does read it.
#[test]
fn each_flag_is_accepted_by_the_policy_that_reads_it() {
    let seen = RefCell::new(Vec::new());

    let mut max_size = args(AdmitPolicy::MaxSize);
    max_size.max_bytes = Some(4096);
    with_policy(&max_size, RecordPolicy { seen: &seen }).unwrap();

    let mut recognised = args(AdmitPolicy::RecognisedCertificate);
    recognised.issuer_public_key = Some("ab".repeat(32));
    recognised.subject_key = vec!["cd".repeat(32)];
    with_policy(&recognised, RecordPolicy { seen: &seen }).unwrap();

    assert_eq!(seen.into_inner().len(), 2);
}

/// A key that is not 64 hexadecimal characters is refused, and the subject-key
/// refusal echoes which one.
///
/// An operator passing several `--subject-key` values needs to know which was
/// rejected; a bare "invalid key" would leave them to compare hex strings by
/// eye. The issuer refusal names no value because there is only ever one.
#[test]
fn malformed_keys_are_refused() {
    let seen = RefCell::new(Vec::new());

    let mut bad_issuer = args(AdmitPolicy::RecognisedCertificate);
    bad_issuer.issuer_public_key = Some("not-hex".to_string());
    let err = with_policy(&bad_issuer, RecordPolicy { seen: &seen }).unwrap_err();
    assert!(
        matches!(err, CliError::InvalidIssuerPublicKey),
        "got: {err}"
    );

    let mut bad_subject = args(AdmitPolicy::RecognisedCertificate);
    bad_subject.issuer_public_key = Some("ab".repeat(32));
    bad_subject.subject_key = vec!["cd".repeat(32), "too-short".to_string()];
    let err = with_policy(&bad_subject, RecordPolicy { seen: &seen }).unwrap_err();
    assert!(
        matches!(&err, CliError::InvalidSubjectKey { value } if value == "too-short"),
        "got: {err}"
    );

    assert!(
        seen.into_inner().is_empty(),
        "no task may run on a malformed key"
    );
}

/// No `--subject-key` means an unrestricted subject, never an empty allow-list.
///
/// The library documents an empty set as admitting nothing, deliberately, so
/// mapping "the operator gave no list" onto "the operator gave an empty list"
/// would turn an unrestricted policy into a deny-all. Keyed on the value the
/// library reports back rather than on which constructor this file called.
#[test]
fn an_absent_subject_list_is_unrestricted_and_a_present_one_is_not() {
    let mut unrestricted = args(AdmitPolicy::RecognisedCertificate);
    unrestricted.issuer_public_key = Some("ab".repeat(32));
    let policy = recognised_certificate(&unrestricted).unwrap();
    assert!(
        policy.subject_keys().is_none(),
        "an absent list must not become an empty set"
    );

    let mut restricted = args(AdmitPolicy::RecognisedCertificate);
    restricted.issuer_public_key = Some("ab".repeat(32));
    restricted.subject_key = vec!["cd".repeat(32)];
    let policy = recognised_certificate(&restricted).unwrap();
    assert_eq!(policy.subject_keys().map(BTreeSet::len), Some(1));
}
