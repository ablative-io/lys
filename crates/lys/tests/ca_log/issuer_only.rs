//! Issuance with the CA key only: the issuer enters every certificate in the
//! log and never holds the log's key, the log's operator makes each
//! certificate's inclusion-proof artifact, and a stranger holding four files
//! checks the certificate and its entry offline.
//!
//! The stranger runs `openssl` and `python3` and nothing else: a cleared
//! environment whose `PATH` is one directory holding only those two, each
//! started under a network-denying wrapper. The wrapper is proved to deny the
//! network by a connection the same command makes without it and cannot make
//! under it, counted at the listener.

use std::io::ErrorKind;
use std::net::TcpListener;
use std::path::Path;

use crate::support::{
    Bench, Stranger, assert_success, copy_verify_inclusion, corrupt_first_leaf, dir_bytes,
    leaf_files, missing_required, path_str, pem_to_der, report, run_lys, said,
};

/// What `lys ca issue` reported for one certificate.
struct Issued {
    subject: String,
    leaf_index: u64,
    tree_size: u64,
    root_base64: String,
}

/// The file `subject`'s issuance wrote with `extension`.
fn output(bench: &Bench, subject: &str, extension: &str) -> std::path::PathBuf {
    bench.path(&format!("{subject}.{extension}"))
}

/// Issues `subject` with the CA key only, checks the report and the leaf,
/// and has the operator make the artifact before anything else enters the log.
fn issue_and_prove(bench: &Bench, subject: &str, request: Option<&Path>) -> Issued {
    let args = bench.issuer_only_args(&bench.log_dir, subject, request);
    let issued = report(&run_lys(&args));
    let no_artifact = issued.get("artifact_path").is_none();
    assert!(no_artifact, "the issuer wrote an artifact: {issued}");
    let root_base64 = issued["root_base64"].as_str().unwrap().to_string();
    assert_eq!(root_base64.len(), 44);
    let leaf_name = format!("{subject}.leaf");
    let leaf_path = issued["leaf_path"].as_str().unwrap();
    assert!(leaf_path.ends_with(&leaf_name), "{leaf_path}");
    let log_dir = issued["log_dir"].as_str().unwrap();
    assert_eq!(log_dir, path_str(&bench.log_dir));

    let certificate = output(bench, subject, "pem");
    let pem = std::fs::read_to_string(certificate).unwrap();
    let leaf = std::fs::read(output(bench, subject, "leaf")).unwrap();
    assert_eq!(leaf, pem_to_der(&pem), "the leaf is the certificate's DER");

    let leaf_index = issued["leaf_index"].as_u64().unwrap();
    let leaves = bench.log_dir.join("leaves");
    let name = format!("{leaf_index:020}");
    let logged = std::fs::read(leaves.join(name)).unwrap();
    assert_eq!(logged, leaf, "the log holds the leaf");

    let artifact = output(bench, subject, "inclusion.json");
    assert_success(&bench.prove(leaf_index, &artifact));
    Issued {
        subject: subject.to_string(),
        leaf_index,
        tree_size: issued["tree_size"].as_u64().unwrap(),
        root_base64,
    }
}

/// The stranger's check of one certificate, in a fresh directory holding only
/// the four handed files and a copy of `scripts/verify_inclusion.py`.
fn stranger_checks(bench: &Bench, stranger: &Stranger, issued: &Issued) {
    let desk = tempfile::tempdir().unwrap();
    let dir = desk.path();
    let subject = issued.subject.as_str();
    let artifact = output(bench, subject, "inclusion.json");
    let handed = [
        (bench.path("issuer.pem"), "issuer.pem"),
        (output(bench, subject, "pem"), "agent.pem"),
        (output(bench, subject, "leaf"), "agent.leaf"),
        (artifact, "agent.inclusion.json"),
    ];
    for (from, to) in handed {
        std::fs::copy(from, dir.join(to)).unwrap();
    }
    copy_verify_inclusion(dir);
    let root = issued.root_base64.as_str();

    // The certificate, under the issuer certificate, with openssl alone.
    let verify = ["verify", "-CAfile", "issuer.pem", "agent.pem"];
    let verified = stranger.offline(dir, "openssl", &verify);
    assert_success(&verified);
    let said_ok = String::from_utf8_lossy(&verified.stdout);
    assert_eq!(said_ok.trim_end(), "agent.pem: OK");

    // The certificate is the leaf.
    let to_der = ["x509", "-in", "agent.pem", "-outform", "DER"];
    let der = stranger.offline(dir, "openssl", &to_der);
    assert_success(&der);
    assert_eq!(der.stdout, std::fs::read(dir.join("agent.leaf")).unwrap());

    // The artifact describes the tree issue reported.
    let text = std::fs::read(dir.join("agent.inclusion.json")).unwrap();
    let artifact: serde_json::Value = serde_json::from_slice(&text).unwrap();
    assert_eq!(artifact["leaf_index"].as_u64(), Some(issued.leaf_index));
    assert_eq!(artifact["tree_size"].as_u64(), Some(issued.tree_size));

    // The entry, under the root issue reported.
    let check = [
        "verify_inclusion.py",
        "agent.inclusion.json",
        "agent.leaf",
        root,
    ];
    let included = stranger.offline(dir, "python3", &check);
    assert_success(&included);
    let first = String::from_utf8_lossy(&included.stdout);
    assert_eq!(first.lines().next(), Some("INCLUSION VERIFIED"));

    // A leaf changed by one byte is refused.
    let mut tampered = std::fs::read(dir.join("agent.leaf")).unwrap();
    let last = tampered.len() - 1;
    tampered[last] ^= 0x01;
    std::fs::write(dir.join("agent.tampered.leaf"), tampered).unwrap();
    let check = [
        "verify_inclusion.py",
        "agent.inclusion.json",
        "agent.tampered.leaf",
        root,
    ];
    let refused = stranger.offline(dir, "python3", &check);
    assert_eq!(refused.status.code(), Some(2), "{}", said(&refused));
    let stderr = String::from_utf8_lossy(&refused.stderr);
    assert!(stderr.starts_with("VERIFICATION FAILED"), "{stderr}");
}

/// The listener's pending connections, counted until none is left.
fn accepted(listener: &TcpListener) -> usize {
    listener.set_nonblocking(true).unwrap();
    let mut count = 0;
    loop {
        match listener.accept() {
            Ok(_) => count += 1,
            Err(err) if err.kind() == ErrorKind::WouldBlock => return count,
            Err(err) => panic!("the listener failed: {err}"),
        }
    }
}

#[test]
fn the_issuer_holds_only_the_ca_key_and_a_stranger_checks_both_certificates_offline() {
    let bench = Bench::new();
    assert_success(&run_lys(&[
        "ca",
        "issuer-cert",
        "--key",
        path_str(&bench.issuer_key),
        "--out",
        path_str(&bench.path("issuer.pem")),
    ]));

    // Without --request, then with it; each proved before the next entry.
    let generated = issue_and_prove(&bench, "agent-one", None);
    assert_eq!((generated.leaf_index, generated.tree_size), (0, 1));

    let holder = bench.path("agent.key");
    let request = bench.path("agent.csr.pem");
    assert_success(&run_lys(&["key", "generate", "--out", path_str(&holder)]));
    assert_success(&run_lys(&[
        "ca",
        "request",
        "--key",
        path_str(&holder),
        "--subject",
        "agent-two",
        "--out",
        path_str(&request),
    ]));
    let presented = issue_and_prove(&bench, "agent-two", Some(&request));
    assert_eq!((presented.leaf_index, presented.tree_size), (1, 2));

    let stranger = Stranger::new();
    assert_eq!(stranger.tools(), ["openssl", "python3"]);
    stranger_checks(&bench, &stranger, &generated);
    stranger_checks(&bench, &stranger, &presented);

    // The wrapper denies the network: the same connection succeeds without
    // it and fails under it, and the listener saw exactly one.
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port().to_string();
    let connect = "import socket, sys; \
                   socket.create_connection(('127.0.0.1', int(sys.argv[1])))";
    let probe = ["-c", connect, port.as_str()];
    let scratch = tempfile::tempdir().unwrap();
    assert_success(&stranger.online(scratch.path(), "python3", &probe));
    let offline = stranger.offline(scratch.path(), "python3", &probe);
    assert_ne!(offline.status.code(), Some(0), "{}", said(&offline));
    assert_eq!(accepted(&listener), 1);
}

#[test]
fn issuance_without_the_log_flags_is_refused_by_argument_parsing() {
    let bench = Bench::new();
    let out = bench.path("agent-x.pem");
    let base = [
        "ca",
        "issue",
        "--key",
        path_str(&bench.issuer_key),
        "--subject",
        "agent-x",
        "--validity",
        "1h",
        "--out",
        path_str(&out),
    ];

    let neither = run_lys(&base);
    assert_eq!(neither.status.code(), Some(2), "{}", said(&neither));
    let missing = missing_required(&neither);
    let named = |flag: &str| missing.iter().any(|line| line.starts_with(flag));
    assert!(named("--log "), "{missing:?}");
    assert!(named("--leaf-out"), "{missing:?}");
    assert!(!out.exists());

    let mut log_only = base.to_vec();
    log_only.extend(["--log", path_str(&bench.log_dir)]);
    let refused = run_lys(&log_only);
    assert_eq!(refused.status.code(), Some(2), "{}", said(&refused));
    let missing = missing_required(&refused);
    let named = |flag: &str| missing.iter().any(|line| line.starts_with(flag));
    assert!(named("--leaf-out"), "{missing:?}");
    assert!(!named("--log "), "{missing:?}");
    assert!(!out.exists());
    assert_eq!(leaf_files(&bench.log_dir), 0);
}

#[test]
fn an_artifact_without_the_log_key_is_refused_and_nothing_is_appended() {
    let bench = Bench::new();
    let first = bench.issuer_only_args(&bench.log_dir, "agent-one", None);
    assert_success(&run_lys(&first));
    let before = leaf_files(&bench.log_dir);

    let artifact = bench.path("agent-three.inclusion.json");
    let mut args = bench.issuer_only_args(&bench.log_dir, "agent-three", None);
    args.push("--artifact-out".to_string());
    args.push(path_str(&artifact).to_string());
    let refused = run_lys(&args);
    assert_eq!(refused.status.code(), Some(1), "{}", said(&refused));
    let expected = "--log-key and --artifact-out come together or not at all: \
                    missing --log-key";
    let stderr = String::from_utf8_lossy(&refused.stderr);
    assert!(stderr.contains(expected), "{stderr}");
    for extension in ["pem", "inclusion.json", "leaf"] {
        let path = output(&bench, "agent-three", extension);
        assert!(!path.exists(), "{} was written", path.display());
    }
    assert_eq!(leaf_files(&bench.log_dir), before);
}

#[test]
fn a_log_that_fails_its_integrity_check_stops_the_issuance_with_nothing_written() {
    let bench = Bench::new();
    let first = bench.issuer_only_args(&bench.log_dir, "agent-one", None);
    assert_success(&run_lys(&first));
    let key = bench.path("issuer4.key");
    assert_success(&run_lys(&["key", "generate", "--out", path_str(&key)]));
    let before = leaf_files(&bench.log_dir);
    corrupt_first_leaf(&bench.log_dir);
    let damaged = dir_bytes(&bench.log_dir);

    let refused = run_lys(&[
        "ca",
        "issue",
        "--key",
        path_str(&key),
        "--subject",
        "agent-four",
        "--validity",
        "1h",
        "--out",
        path_str(&bench.path("agent-four.pem")),
        "--log",
        path_str(&bench.log_dir),
        "--leaf-out",
        path_str(&bench.path("agent-four.leaf")),
        "--issuer-out",
        path_str(&bench.path("issuer4.pem")),
    ]);
    assert_eq!(refused.status.code(), Some(1), "{}", said(&refused));
    let stderr = String::from_utf8_lossy(&refused.stderr);
    let refusal = "error: log directory invalid:";
    assert!(stderr.starts_with(refusal), "{stderr}");
    for name in [
        "agent-four.pem",
        "agent-four.leaf",
        "issuer4.pem",
        "issuer4.key.issuer.pem",
    ] {
        assert!(!bench.path(name).exists(), "{name} was written");
    }
    assert_eq!(
        dir_bytes(&bench.log_dir),
        damaged,
        "a refused issuance writes nothing to the log"
    );
    assert_eq!(before, 1);
}
