#![cfg(test)]
//! The script recomputes the root for every leaf in the log, and the root
//! argument, when supplied, decides the verdict in both directions.

use super::*;

#[test]
fn the_script_recomputes_the_root_for_every_leaf_in_the_log() {
    let Some(python) = python_or_skip("lys-anchor stranger verification") else {
        return;
    };

    let mut verified = 0;
    let mut path_lengths = Vec::new();
    for leaf_index in 0..TREE_SIZE {
        let case = build_case(leaf_index);
        let (code, stdout, stderr) = run(&python, &case, None);
        assert_eq!(
            code,
            Some(0),
            "the script refused a genuine proof for leaf {leaf_index}\n\
             stdout: {stdout}\nstderr: {stderr}"
        );
        // Keyed on what the script printed, not on its exit status: exit 0 is
        // what a script that did nothing at all also returns.
        assert!(
            stdout.contains("INCLUSION VERIFIED"),
            "the script exited 0 for leaf {leaf_index} without verifying anything\n\
             stdout: {stdout}"
        );
        path_lengths.push(case.artifact["hashes"].as_array().unwrap().len());
        verified += 1;
    }

    // Count what fired: a loop that ran zero times satisfies every assertion in
    // it, and a gate that never spawned looks exactly like one that passed.
    assert_eq!(verified, TREE_SIZE);

    // And the sweep covered paths of different depths rather than one shape
    // five times, which five agreements on a one-node path would also satisfy.
    path_lengths.sort_unstable();
    path_lengths.dedup();
    assert!(
        path_lengths.len() > 1,
        "every proof in the sweep had the same path length: {path_lengths:?}"
    );
}

#[test]
fn the_supplied_root_argument_decides_in_both_directions() {
    let Some(python) = python_or_skip("lys-anchor stranger verification") else {
        return;
    };
    let case = build_case(3);

    // 32 zero bytes: a root no SHA-256 tree produces, and demonstrably not this
    // log's. An argument the script silently ignored would pass this.
    let zero_root = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
    assert_ne!(
        case.root_b64, zero_root,
        "the fixture root is not all zeros"
    );
    assert_refused(
        &python,
        &case,
        Some(zero_root),
        "a root that is not the log's",
    );

    // And the same argument accepts the real root, so the refusal above is the
    // check working rather than the argument always rejecting.
    let (code, stdout, stderr) = run(&python, &case, Some(&case.root_b64));
    assert_eq!(
        code,
        Some(0),
        "the script refused the log's own root\nstdout: {stdout}\nstderr: {stderr}"
    );
    assert!(stdout.contains("INCLUSION VERIFIED"), "stdout: {stdout}");
}
