#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use clap::CommandFactory;

/// The minimal argument list for each subcommand, WITHOUT `--admit`.
///
/// Kept in one place so the "required" test and its positive control cannot
/// drift apart by exercising different invocations — which is the way a control
/// stops controlling anything.
fn invocations_without_admit() -> Vec<Vec<&'static str>> {
    let all = vec![
        vec![
            "lys-anchor",
            "init",
            "--dir",
            "/tmp/a",
            "--origin",
            "example.com/a",
            "--key",
            "/tmp/k",
            "--genesis",
            "/tmp/g",
        ],
        vec!["lys-anchor", "status", "--dir", "/tmp/a", "--key", "/tmp/k"],
        vec![
            "lys-anchor",
            "checkpoint",
            "--dir",
            "/tmp/a",
            "--key",
            "/tmp/k",
            "--out",
            "/tmp/o",
        ],
        vec![
            "lys-anchor",
            "prove",
            "--dir",
            "/tmp/a",
            "--key",
            "/tmp/k",
            "--leaf-index",
            "1",
            "--out",
            "/tmp/o",
        ],
    ];
    // Shadowed rather than pushed into a `mut` binding, because `submit` does
    // not exist in a default build and a `mut` that is never written is itself
    // a lint failure there. The subcommand's absence is the point, so the shape
    // of this helper has to survive it.
    #[cfg(feature = "unstable-anchor")]
    let all = {
        let mut all = all;
        all.push(vec![
            "lys-anchor",
            "submit",
            "--dir",
            "/tmp/a",
            "--key",
            "/tmp/k",
            "--statement",
            "/tmp/s",
            "--receipt-out",
            "/tmp/r",
            "--artifact-out",
            "/tmp/f",
        ]);
        all
    };
    all
}

/// Clap's own validation of the argument graph.
///
/// The second party is clap: `debug_assert` checks the declarations for
/// contradictions this crate cannot see in itself — a `required_if_eq` naming a
/// value the enum does not have, a duplicate long flag, an id that does not
/// resolve. It fails at test time rather than at an operator's first
/// invocation of a rarely used flag.
#[test]
fn the_argument_graph_is_internally_consistent() {
    Cli::command().debug_assert();
}

/// No subcommand can open or create an anchor without the operator naming an
/// admission policy.
///
/// The second party is `lys-anchor`'s own rule, stated where this crate cannot
/// edit it: no policy implements `Default`, the absence is checked there by
/// `compile_fail` doctests, and `Anchor::create`/`Anchor::open` have no overload
/// that omits a policy. A CLI that supplied one would undo all of it in a line,
/// and the only visible symptom would be that this test started passing without
/// `--admit`.
#[test]
fn every_subcommand_requires_admit() {
    let mut refusals = 0;
    for invocation in invocations_without_admit() {
        let parsed = Cli::try_parse_from(&invocation);
        assert!(
            parsed.is_err(),
            "parsed without --admit: {:?}",
            invocation.get(1)
        );
        refusals += 1;
    }
    assert_eq!(
        refusals,
        invocations_without_admit().len(),
        "every subcommand must have been exercised"
    );
    assert!(refusals >= 4, "the four core subcommands at minimum");
}

/// The positive control for the refusals above: the same invocations parse once
/// `--admit` is present.
///
/// Without this, "every subcommand requires --admit" would also pass for an
/// invocation that was malformed for some entirely different reason.
#[test]
fn the_same_invocations_parse_with_admit() {
    let mut parsed_count = 0;
    for mut invocation in invocations_without_admit() {
        invocation.extend_from_slice(&["--admit", "accept-all"]);
        Cli::try_parse_from(&invocation)
            .unwrap_or_else(|err| panic!("{:?} failed to parse: {err}", invocation.get(1)));
        parsed_count += 1;
    }
    assert_eq!(parsed_count, invocations_without_admit().len());
}

/// `--admit max-size` without `--max-bytes` is refused by clap, with the
/// positive control that supplying it parses.
///
/// A limit is the operator's number and there is no default for it — a policy
/// that silently admitted everything because the number was forgotten would be
/// running a rule nobody chose.
#[test]
fn max_size_requires_its_limit() {
    let base = ["lys-anchor", "status", "--dir", "/tmp/a", "--key", "/tmp/k"];
    let without: Vec<&str> = base
        .iter()
        .copied()
        .chain(["--admit", "max-size"])
        .collect();
    assert!(Cli::try_parse_from(&without).is_err(), "{without:?}");

    let with: Vec<&str> = base
        .iter()
        .copied()
        .chain(["--admit", "max-size", "--max-bytes", "4096"])
        .collect();
    Cli::try_parse_from(&with).expect("the control must parse");
}

/// `--admit recognised-certificate` without `--issuer-public-key` is refused by
/// clap, with the positive control that supplying it parses.
///
/// There is no authority this CLI could name, and a policy that defaulted to
/// trusting one would be the single worst default in the workspace.
#[test]
fn recognised_certificate_requires_its_authority() {
    let base = ["lys-anchor", "status", "--dir", "/tmp/a", "--key", "/tmp/k"];
    let without: Vec<&str> = base
        .iter()
        .copied()
        .chain(["--admit", "recognised-certificate"])
        .collect();
    assert!(Cli::try_parse_from(&without).is_err(), "{without:?}");

    let issuer = "ab".repeat(32);
    let with: Vec<&str> = base
        .iter()
        .copied()
        .chain([
            "--admit",
            "recognised-certificate",
            "--issuer-public-key",
            issuer.as_str(),
        ])
        .collect();
    Cli::try_parse_from(&with).expect("the control must parse");
}

/// The value strings an operator types are the ones diagnostics quote back.
///
/// `AdmitPolicy::as_str` feeds `--admit {policy} does not read {flag}`, so a
/// rename that changed one and not the other would produce a message telling an
/// operator to use a flag value that does not exist.
#[test]
fn policy_names_match_the_accepted_values() {
    let issuer = "ab".repeat(32);
    for (policy, spelled) in [
        (AdmitPolicy::AcceptAll, "accept-all"),
        (AdmitPolicy::MaxSize, "max-size"),
        (AdmitPolicy::RecognisedCertificate, "recognised-certificate"),
    ] {
        assert_eq!(policy.as_str(), spelled);
        let invocation = [
            "lys-anchor",
            "status",
            "--dir",
            "/tmp/a",
            "--key",
            "/tmp/k",
            "--admit",
            spelled,
            "--max-bytes",
            "1",
            "--issuer-public-key",
            issuer.as_str(),
        ];
        // Parsing must accept the spelling; whether the policy READS those two
        // flags is `policy::with_policy`'s refusal, not clap's.
        let parsed = Cli::try_parse_from(invocation);
        assert!(
            parsed.is_ok(),
            "{spelled} was not an accepted --admit value"
        );
    }
}
