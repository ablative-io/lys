//! Outputs: refused before anything is signed when they would overwrite a file
//! or each other, and a failure after the append names the entry it leaves.

use crate::support::{
    Bench, assert_success, hex_to_base64, path_str, report, run_lys, said, verify_inclusion,
    with_flag,
};

/// The names in the bench directory other than the fixtures, sorted. The
/// issuer certificate stored beside the issuer key belongs with the key.
fn written(bench: &Bench) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(bench.dir())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| !["issuer.key", "operator.key", "log"].contains(&name.as_str()))
        .filter(|name| name != "issuer.key.issuer.pem")
        .collect();
    names.sort();
    names
}

#[test]
fn a_failure_after_the_append_names_the_entry_and_the_command_that_recovers_it() {
    let bench = Bench::new();
    let unreachable = bench.path("no-such-dir").join("agent-late.inclusion.json");
    let args = with_flag(
        bench.issue_args(&bench.log_dir, "agent-late"),
        "--artifact-out",
        &unreachable,
    );
    let failed = run_lys(&args);
    assert_ne!(failed.status.code(), Some(0));

    // The log grew by exactly one, and the error names that entry.
    assert_eq!(bench.log_size(), 1);
    let status = run_lys(&["--json", "log", "status", "--dir", path_str(&bench.log_dir)]);
    let root = hex_to_base64(report(&status)["root_hash"].as_str().unwrap());
    let message = said(&failed);
    assert!(message.contains("leaf index 0"), "{message}");
    assert!(message.contains("tree size 1"), "{message}");
    assert!(message.contains(&root), "the root in base64: {message}");
    let recover = format!(
        "lys log prove inclusion --dir {} --key {} --leaf-index 0 --out {}",
        path_str(&bench.log_dir),
        path_str(&bench.log_key),
        path_str(&unreachable)
    );
    assert!(message.contains(&recover), "{message}");
    assert!(message.contains("do not issue again"), "{message}");

    // The certificate and the leaf were placed before the artifact failed,
    // and nothing temporary was left behind.
    assert_eq!(
        written(&bench),
        ["agent-late.issuer.pem", "agent-late.leaf", "agent-late.pem"]
    );

    // Issuing again is refused before anything is signed, so no second leaf.
    let again = bench.issue_into(&bench.log_dir, "agent-late");
    assert_ne!(again.status.code(), Some(0));
    assert!(said(&again).contains("agent-late.pem"), "{}", said(&again));
    assert_eq!(bench.log_size(), 1);

    // The named command, pointed where the artifact can go, recovers it.
    let recovered = bench.path("agent-late.inclusion.json");
    assert_success(&run_lys(&[
        "log",
        "prove",
        "inclusion",
        "--dir",
        path_str(&bench.log_dir),
        "--key",
        path_str(&bench.log_key),
        "--leaf-index",
        "0",
        "--out",
        path_str(&recovered),
    ]));
    assert_success(&verify_inclusion(
        &recovered,
        &bench.path("agent-late.leaf"),
        Some(&root),
    ));
}

#[test]
fn an_existing_output_is_refused_by_name_before_anything_is_signed_or_appended() {
    let bench = Bench::new();
    let existing = bench.path("agent-kept.leaf");
    std::fs::write(&existing, b"someone else's file").unwrap();

    let refused = bench.issue_into(&bench.log_dir, "agent-kept");
    assert_ne!(refused.status.code(), Some(0));
    let message = said(&refused);
    assert!(message.contains("leaf file"), "{message}");
    assert!(message.contains(path_str(&existing)), "{message}");
    assert_eq!(std::fs::read(&existing).unwrap(), b"someone else's file");
    assert_eq!(bench.log_size(), 0);
    assert_eq!(written(&bench), ["agent-kept.leaf"]);
}

#[test]
fn an_existing_certificate_is_refused_with_the_ca_key_only_too() {
    let bench = Bench::new();
    let existing = bench.path("agent-plain.pem");
    std::fs::write(&existing, b"an older certificate").unwrap();
    let refused = run_lys(&[
        "ca",
        "issue",
        "--key",
        path_str(&bench.issuer_key),
        "--subject",
        "agent-plain",
        "--validity",
        "1h",
        "--out",
        path_str(&existing),
        "--log",
        path_str(&bench.log_dir),
        "--leaf-out",
        path_str(&bench.path("agent-plain.leaf")),
    ]);
    assert_ne!(refused.status.code(), Some(0));
    assert!(
        said(&refused).contains("certificate file"),
        "{}",
        said(&refused)
    );
    assert_eq!(std::fs::read(&existing).unwrap(), b"an older certificate");
    assert!(!bench.path("agent-plain.leaf").exists());
    assert_eq!(bench.log_size(), 0);
}

#[test]
fn two_outputs_sharing_a_path_are_refused_by_name_before_anything_is_appended() {
    let bench = Bench::new();
    let shared = bench.path("agent-shared.pem");
    let args = with_flag(
        bench.issue_args(&bench.log_dir, "agent-shared"),
        "--leaf-out",
        &shared,
    );
    let refused = run_lys(&args);
    assert_ne!(refused.status.code(), Some(0));
    let message = said(&refused);
    assert!(message.contains("certificate file"), "{message}");
    assert!(message.contains("leaf file"), "{message}");
    assert!(message.contains(path_str(&shared)), "{message}");
    assert_eq!(bench.log_size(), 0);
    assert!(written(&bench).is_empty(), "{:?}", written(&bench));
}
