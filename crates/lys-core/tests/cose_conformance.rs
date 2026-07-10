//! D4 conformance gate: round-trip the `lys/attestation/v2` `COSE_Sign1`
//! implementation against the vendored Go `veraison/go-cose` reference
//! implementation.
//!
//! # Environment contract
//!
//! The Go scaffold in `tests/cose-conformance/` is fully vendored
//! (`go mod vendor`, pinned `github.com/veraison/go-cose v1.3.0` with its
//! MIT deps `fxamacker/cbor/v2 v2.5.0` and `x448/float16 v0.8.4`); every
//! invocation runs with `GOFLAGS=-mod=vendor GOPROXY=off GOTOOLCHAIN=local`
//! and a throwaway `GOCACHE`, so the gate needs zero network. The toolchain
//! is located via `LYS_GO_BIN`, then `/usr/local/go/bin/go`, then `go` on
//! `PATH`. If none is found, the Go round-trip tests print a skip notice
//! and return — but a toolchain that is present and BROKEN is a hard test
//! failure, deliberately.
//!
//! The pure-Rust golden assertions in this file run unconditionally, so a
//! Go-less environment never reduces byte-exact coverage (the primary
//! copies of these vectors live in the always-run unit tests as well).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use lys_core::Ed25519Identity;
use lys_core::attestation::{Attestation, verify_attestation, verify_attestation_bytes};

/// Fixed golden seed: the 32 ASCII bytes `"lys-cose-conformance-test-seed01"`.
const GOLDEN_SEED: &[u8; 32] = b"lys-cose-conformance-test-seed01";

/// Hex form of the seed, handed to the Go tool's `sign` mode.
const GOLDEN_SEED_HEX: &str = "6c79732d636f73652d636f6e666f726d616e63652d746573742d736565643031";

/// Public key for the golden seed, handed to the Go tool's `verify` mode.
const GOLDEN_PUBKEY_HEX: &str = "214e41cc8475f5bc9af68dbf33fa56e8adf5144a4a81134268f4ae379e103bfd";

const GOLDEN_PAYLOAD: &[u8] = b"lys attestation conformance payload\n";

/// SHA-256 of the golden payload.
const GOLDEN_HASH_HEX: &str = "9404f8b8cec8ad98a88b106d9345d518c273f012c1306b8af5103e865997191e";

const GOLDEN_TIMESTAMP: i64 = 1_700_000_000_000;

/// The 143-byte golden `Sig_structure` (the exact Ed25519 signing input).
const GOLDEN_SIG_STRUCTURE_HEX: &str = "846a5369676e6174757265315850a301270378276170706c69636174696f6e2f766e642e6c79732e6174746573746174696f6e2e76322b63626f72045820214e41cc8475f5bc9af68dbf33fa56e8adf5144a4a81134268f4ae379e103bfd40582ea20158209404f8b8cec8ad98a88b106d9345d518c273f012c1306b8af5103e865997191e021b0000018bcfe56800";

/// The golden 64-byte Ed25519 signature.
const GOLDEN_SIGNATURE_HEX: &str = "ba6eae66b3a04f25e6116e7278ae8b0e2174a2964bb0917ae53ab2ef326c22a706e90cae6886b20ecd086fee4b515de046d450f90de81f9aa46a9b34bd7f1a09";

/// The complete 199-byte golden artifact, byte-identical between the lys
/// hand encoder and go-cose `Sign1Message.MarshalCBOR` (Ed25519 determinism
/// plus core-deterministic CBOR make the equality exact).
const GOLDEN_ARTIFACT_HEX: &str = "d2845850a301270378276170706c69636174696f6e2f766e642e6c79732e6174746573746174696f6e2e76322b63626f72045820214e41cc8475f5bc9af68dbf33fa56e8adf5144a4a81134268f4ae379e103bfda0582ea20158209404f8b8cec8ad98a88b106d9345d518c273f012c1306b8af5103e865997191e021b0000018bcfe568005840ba6eae66b3a04f25e6116e7278ae8b0e2174a2964bb0917ae53ab2ef326c22a706e90cae6886b20ecd086fee4b515de046d450f90de81f9aa46a9b34bd7f1a09";

/// Golden artifact byte offsets used to splice mutants:
/// `d2` (0) `84` (1) `58 50` (2–3) protected (4–83) `a0` (84)
/// `58 2e` (85–86) claims (87–132) `58 40` (133–134) signature (135–198).
const OFFSET_UNPROTECTED: usize = 84;
const OFFSET_CLAIMS_HEAD: usize = 85;
const OFFSET_CLAIMS: usize = 87;

fn hex_to_bytes(hex: &str) -> Vec<u8> {
    assert_eq!(hex.len() % 2, 0);
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}

fn golden_artifact() -> Vec<u8> {
    hex_to_bytes(GOLDEN_ARTIFACT_HEX)
}

fn golden_identity() -> (tempfile::TempDir, Ed25519Identity) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("golden.key");
    std::fs::write(&path, GOLDEN_SEED).unwrap();
    let identity = Ed25519Identity::load(&path).unwrap();
    (dir, identity)
}

/// Mutant A — unprotected-header smuggling: insert `{10: 10}` into the
/// unprotected map. The signature stays valid (the unprotected bucket is
/// outside `Sig_structure`); lys rejects, vanilla go-cose accepts.
fn smuggling_mutant() -> Vec<u8> {
    let golden = golden_artifact();
    let mut mutant = golden[..OFFSET_UNPROTECTED].to_vec();
    mutant.extend_from_slice(&[0xa1, 0x0a, 0x0a]);
    mutant.extend_from_slice(&golden[OFFSET_UNPROTECTED + 1..]);
    mutant
}

/// Mutant D — oversized length head on the claims bstr (`59 00 2e` for
/// `58 2e`): the claims bytes are unchanged, so the signature stays valid;
/// lys rejects on canonicality, vanilla go-cose accepts.
fn oversized_head_mutant() -> Vec<u8> {
    let golden = golden_artifact();
    let mut mutant = golden[..OFFSET_CLAIMS_HEAD].to_vec();
    mutant.push(0x59);
    mutant.push(0x00);
    mutant.extend_from_slice(&golden[OFFSET_CLAIMS_HEAD + 1..]);
    mutant
}

/// Mutant C — indefinite-length re-encoding of the outer array (signature
/// still cryptographically valid). Rejected by BOTH: go-cose v1.3.0's
/// decode mode forbids indefinite lengths outright, and lys rejects on the
/// canonical byte-compare.
fn indefinite_length_mutant() -> Vec<u8> {
    let golden = golden_artifact();
    let mut mutant = vec![0xd2, 0x9f];
    mutant.extend_from_slice(&golden[2..]);
    mutant.push(0xff);
    mutant
}

/// Pure-Rust golden assertions — run unconditionally, Go or no Go, so the
/// gate file alone is self-evidently covered even when the round-trip
/// skips.
#[test]
fn golden_vectors_pure_rust() {
    let (_dir, identity) = golden_identity();
    assert_eq!(
        identity.public_key_bytes().to_vec(),
        hex_to_bytes(GOLDEN_PUBKEY_HEX)
    );

    // The golden signature is deterministic over the pinned Sig_structure.
    let signature = identity.sign(&hex_to_bytes(GOLDEN_SIG_STRUCTURE_HEX));
    assert_eq!(signature.to_vec(), hex_to_bytes(GOLDEN_SIGNATURE_HEX));

    // The assembled attestation reproduces the frozen golden artifact.
    let attestation = Attestation {
        payload_hash: hex_to_bytes(GOLDEN_HASH_HEX).as_slice().try_into().unwrap(),
        signature,
        signer_public_key: identity.public_key_bytes(),
        timestamp: GOLDEN_TIMESTAMP,
    };
    assert_eq!(attestation.to_cose_bytes(), golden_artifact());

    // Round trip: the golden artifact parses and verifies.
    let restored = verify_attestation_bytes(&golden_artifact(), GOLDEN_PAYLOAD).unwrap();
    assert_eq!(restored, attestation);
    verify_attestation(&restored, GOLDEN_PAYLOAD).unwrap();

    // Negative control: one flipped claims byte rejects.
    let mut tampered = golden_artifact();
    tampered[OFFSET_CLAIMS + 5] ^= 0x01;
    assert!(verify_attestation_bytes(&tampered, GOLDEN_PAYLOAD).is_err());

    // Strictness pins (the lys half of the strictness-delta assertions).
    assert!(Attestation::from_cose_bytes(&smuggling_mutant()).is_err());
    assert!(Attestation::from_cose_bytes(&oversized_head_mutant()).is_err());
    assert!(Attestation::from_cose_bytes(&indefinite_length_mutant()).is_err());
}

/// Locates the Go toolchain: `LYS_GO_BIN` override, then the pinned
/// absolute path, then `go` on PATH. `None` means "skip the round-trip".
fn find_go() -> Option<PathBuf> {
    if let Ok(overridden) = std::env::var("LYS_GO_BIN") {
        return Some(PathBuf::from(overridden));
    }
    let pinned = Path::new("/usr/local/go/bin/go");
    if pinned.exists() {
        return Some(pinned.to_path_buf());
    }
    let on_path = Command::new("go")
        .arg("version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    match on_path {
        Ok(status) if status.success() => Some(PathBuf::from("go")),
        _ => None,
    }
}

/// Runs the vendored Go tool hermetically with `input` on stdin; returns
/// `(exit_success, stdout_bytes)`. Any spawn failure with a PRESENT
/// toolchain is a hard panic — the environment contract is documented in
/// the file header.
fn run_go_tool(go: &Path, gocache: &Path, args: &[&str], input: &[u8]) -> (bool, Vec<u8>) {
    let scaffold_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/cose-conformance");
    let mut child = Command::new(go)
        .arg("run")
        .arg(".")
        .args(args)
        .current_dir(&scaffold_dir)
        .env("GOFLAGS", "-mod=vendor")
        .env("GOPROXY", "off")
        .env("GOTOOLCHAIN", "local")
        .env("GOCACHE", gocache)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("failed to spawn the Go toolchain (present but broken is a hard failure)");
    child
        .stdin
        .take()
        .expect("child stdin is piped")
        .write_all(input)
        .expect("failed to write to the Go tool's stdin");
    let output = child
        .wait_with_output()
        .expect("failed to wait for the Go tool");
    (output.status.success(), output.stdout)
}

#[test]
fn go_cose_conformance_round_trips() {
    let Some(go) = find_go() else {
        // The skip is for developer machines only. CI sets LYS_REQUIRE_GO,
        // so a missing toolchain there is a hard failure — the D4 gate can
        // never silently degrade to "passed" where it matters.
        assert!(
            std::env::var_os("LYS_REQUIRE_GO").is_none(),
            "LYS_REQUIRE_GO is set but no Go toolchain was found — \
             the COSE conformance gate must not skip in this environment"
        );
        eprintln!("skipping go-cose conformance round-trip: no Go toolchain found");
        return;
    };
    let gocache_dir = tempfile::tempdir().unwrap();
    let gocache = gocache_dir.path().join("gocache");

    let expected_verify_line = format!("{GOLDEN_HASH_HEX} {GOLDEN_TIMESTAMP}\n");

    // Round-trip A (Rust -> Go): the go-cose reference verifier accepts
    // the Rust-built artifact and reports the golden hash and timestamp.
    let (ok, stdout) = run_go_tool(
        &go,
        &gocache,
        &["verify", GOLDEN_PUBKEY_HEX],
        &golden_artifact(),
    );
    assert!(ok, "go-cose rejected the Rust-built artifact");
    assert_eq!(
        stdout,
        expected_verify_line.as_bytes(),
        "go-cose reported different claims"
    );

    // Round-trip B (Go -> Rust): the go-cose-built artifact is
    // byte-identical to the Rust one and verifies under lys.
    let (ok, go_artifact) = run_go_tool(
        &go,
        &gocache,
        &["sign", GOLDEN_SEED_HEX, &GOLDEN_TIMESTAMP.to_string()],
        GOLDEN_PAYLOAD,
    );
    assert!(ok, "go-cose sign failed");
    assert_eq!(
        go_artifact,
        golden_artifact(),
        "go-cose and lys artifacts must be byte-identical"
    );
    let restored = verify_attestation_bytes(&go_artifact, GOLDEN_PAYLOAD).unwrap();
    assert_eq!(restored.timestamp, GOLDEN_TIMESTAMP);

    // Negative parity: one flipped claims byte, rejected by BOTH.
    let mut tampered = golden_artifact();
    tampered[OFFSET_CLAIMS + 5] ^= 0x01;
    let (ok, _stdout) = run_go_tool(&go, &gocache, &["verify", GOLDEN_PUBKEY_HEX], &tampered);
    assert!(!ok, "go-cose accepted a tampered artifact");
    assert!(verify_attestation_bytes(&tampered, GOLDEN_PAYLOAD).is_err());

    // Strictness delta 1 — unprotected-header smuggling: the signature is
    // valid and vanilla go-cose ACCEPTS the mutant (the unprotected bucket
    // is outside Sig_structure); lys rejects it. This pins why the
    // canonical-strict verifier exists.
    let smuggled = smuggling_mutant();
    let (ok, stdout) = run_go_tool(&go, &gocache, &["verify", GOLDEN_PUBKEY_HEX], &smuggled);
    assert!(
        ok,
        "go-cose is expected to accept the unprotected-smuggling mutant \
         (vanilla COSE behavior); it rejected — the documented delta changed"
    );
    assert_eq!(stdout, expected_verify_line.as_bytes());
    assert!(Attestation::from_cose_bytes(&smuggled).is_err());

    // Strictness delta 2 — oversized length head (valid signature,
    // non-canonical framing): vanilla go-cose ACCEPTS it; lys rejects.
    let oversized = oversized_head_mutant();
    let (ok, stdout) = run_go_tool(&go, &gocache, &["verify", GOLDEN_PUBKEY_HEX], &oversized);
    assert!(
        ok,
        "go-cose is expected to accept the oversized-length-head mutant \
         (vanilla COSE behavior); it rejected — the documented delta changed"
    );
    assert_eq!(stdout, expected_verify_line.as_bytes());
    assert!(Attestation::from_cose_bytes(&oversized).is_err());

    // Indefinite-length parity: go-cose v1.3.0 forbids indefinite lengths
    // at decode, and lys rejects on the canonical byte-compare — rejected
    // by BOTH (empirically re-checked during this build; the D4 design
    // draft expected go-cose to accept it).
    let indefinite = indefinite_length_mutant();
    let (ok, _stdout) = run_go_tool(&go, &gocache, &["verify", GOLDEN_PUBKEY_HEX], &indefinite);
    assert!(!ok, "go-cose accepted an indefinite-length artifact");
    assert!(Attestation::from_cose_bytes(&indefinite).is_err());
}
