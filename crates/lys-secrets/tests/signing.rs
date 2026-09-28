#![cfg(test)]
//! The broker signs with a key it holds and returns the signature only
//! (SECRETS-006). Each acceptance line of R2 and R3, and each route line of
//! R1, is one test here. Tests of a route run the served binary over a
//! broker seeded by its own commands (`support/keyed.rs`); tests of the
//! library drive `Broker::sign_for` in process (`support/world.rs`), on a
//! clock the test moves by hand. Every signature is checked against the key
//! the test's own seed makes, never against the key an answer names. A
//! served test signs for the wall clock's time, which the served broker
//! reads; an in-process test signs for its broker's fixed time.

#[path = "support/keyed.rs"]
mod keyed;
#[path = "support/signer.rs"]
mod signer;
#[path = "support/world.rs"]
mod world;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use keyed::{CREDENTIAL, Failure, KEY, Keyed, OWNER, SIGNATURE, Served};
use lys_core::Ed25519Identity;
use lys_core::attestation::verify_attestation_bytes_by_signer;
use lys_secrets::{
    AgentRequest, AuditKind, Broker, IssuedHandle, LocalGrants, Secret, SecretsError, Signable,
    Signing, SigningPurpose, from_hex, to_hex,
};
use reqwest::Method;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use signer::Signer;
use world::{GRANTOR, NOW_MS, TestResult, World, granted, refusal};
use zeroize::Zeroizing;

const AGENT: &str = "agent:noor";
const METHOD: &str = "POST";
const PATH: &str = "/v1/agents/whoami";
const BODY: &[u8] = b"{\"agent\":\"noor\"}";
/// The signing time of the in-process tests: their broker's fixed clock.
const SIGNED_AT_MS: u64 = 1_800_000_000_000;
const NONCE: &str = "00112233445566778899aabbccddeeff";
/// The seed of the key the in-process tests seal. It is no credential.
const SEED: [u8; 32] = [0x5c; 32];

/// The wall clock, the time a served broker accepts a signature for.
fn now_ms() -> Result<u64, Failure> {
    Ok(u64::try_from(
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis(),
    )?)
}

fn body_digest() -> String {
    to_hex(&Sha256::digest(BODY))
}

/// The members of the one agent request every test signs, signed at `at`.
fn members(at: u64) -> Value {
    json!({
        "method": METHOD,
        "path": PATH,
        "body_digest": body_digest(),
        "signed_at_ms": at,
        "nonce": NONCE,
    })
}

/// The body of a signature request for `key` with `members`.
fn asked(key: &str, members: &Value) -> Result<Vec<u8>, serde_json::Error> {
    let asked = json!({ "key": key, "purpose": "agent_request", "members": members });
    serde_json::to_vec(&asked)
}

/// A signature request for the key, signed now.
fn fresh() -> Result<Vec<u8>, Failure> {
    Ok(asked(KEY, &members(now_ms()?))?)
}

/// The bytes the verifier builds for the members signed at `at`, by the
/// shared function.
fn payload(at: u64) -> Vec<u8> {
    lys_core::agent_request::payload(METHOD, PATH, &body_digest(), at, NONCE)
}

/// The members signed at `at`, as the library takes them.
fn signable(at: u64) -> Result<Signable, SecretsError> {
    AgentRequest::new(METHOD, PATH, &body_digest(), at, NONCE).map(Signable::AgentRequest)
}

/// The public key `seed` makes: the key a signature must verify against.
fn public_key_of(seed: [u8; 32]) -> [u8; 32] {
    Ed25519Identity::from_seed(&Zeroizing::new(seed)).public_key_bytes()
}

fn holds(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

/// Whether `bytes` hold the seed's bytes, or their hex in either case.
fn holds_seed(bytes: &[u8], seed: &[u8; 32]) -> bool {
    let hex = to_hex(seed);
    let upper = hex.to_uppercase();
    [&seed[..], hex.as_bytes(), upper.as_bytes()]
        .iter()
        .any(|needle| holds(bytes, needle))
}

/// The owner's handle on the key, as the handles route shows it.
fn key_handle(served: &Served, keyed: &Keyed) -> Result<Value, Failure> {
    let target = format!("/_lys/handles?holder={OWNER}");
    let (status, body) = served.ask(keyed, &Method::GET, &target, b"")?;
    assert_eq!(status, 200, "{body}");
    let answer: Value = serde_json::from_str(&body)?;
    let id = keyed.on_key.id.as_str();
    let handles = answer["handles"].as_array().ok_or("no handles")?;
    let found = handles.iter().find(|handle| handle["id"] == id);
    Ok(found.ok_or("no handle on the key")?.clone())
}

/// The key as the owner's listing shows it, and the whole listing.
fn listed_key(served: &Served, keyed: &Keyed) -> Result<(Value, String), Failure> {
    let (status, body) = served.ask(keyed, &Method::GET, "/_lys/secrets", b"")?;
    assert_eq!(status, 200, "{body}");
    let answer: Value = serde_json::from_str(&body)?;
    let entries = answer["secrets"].as_array().ok_or("no secrets")?;
    let entry = entries.iter().find(|entry| entry["name"] == KEY);
    Ok((entry.ok_or("the listing holds no key")?.clone(), body))
}

/// How many lines the owner reads at `GET /_lys/audit` of `kind` whose
/// outcome is `outcome`.
fn audit_count(
    served: &Served,
    keyed: &Keyed,
    kind: &str,
    outcome: &str,
) -> Result<usize, Failure> {
    let (status, body) = served.ask(keyed, &Method::GET, "/_lys/audit", b"")?;
    assert_eq!(status, 200, "{body}");
    let answer: Value = serde_json::from_str(&body)?;
    let lines = answer["lines"].as_array().ok_or("no lines")?;
    let matching = |line: &&Value| line["kind"] == kind && line["outcome"] == outcome;
    Ok(lines.iter().filter(matching).count())
}

/// Revokes the owner's lease on the key through its route.
fn revoke(served: &Served, keyed: &Keyed) -> TestResult {
    let target = format!("/_lys/leases/{}/revoke", keyed.on_key.id);
    let (status, body) = served.ask(keyed, &Method::POST, &target, b"")?;
    assert_eq!(status, 200, "the revoke: {body}");
    Ok(())
}

/// A broker in process holding `KEY`, sealed from `SEED`, and a handle on
/// it for `AGENT` good for `uses` uses until `not_after_ms`.
fn signing_world(
    world: &World,
    broker: &mut Broker<LocalGrants>,
    (uses, not_after_ms): (u64, i64),
) -> Result<(Signer, IssuedHandle), Failure> {
    let seed = Secret::from_slice(&SEED);
    broker.seal_signing_key(KEY, GRANTOR, SigningPurpose::AgentRequest, &seed)?;
    let signer = Signer::new(&world.keys(), AGENT)?;
    let issued = broker.issue(&signer.holder, KEY, uses, not_after_ms)?;
    Ok((signer, issued))
}

/// One signing use of `issued` by `signer` through the library broker.
fn sign(
    broker: &mut Broker<LocalGrants>,
    signer: &Signer,
    issued: &IssuedHandle,
) -> Result<Signing, SecretsError> {
    let presented = signer.present(&issued.id)?;
    broker.sign_for(&issued.token, &presented, KEY, &signable(SIGNED_AT_MS)?)
}

/// Whether one signing use of `issued` by `signer` made a signature.
fn signs(
    broker: &mut Broker<LocalGrants>,
    signer: &Signer,
    issued: &IssuedHandle,
) -> Result<bool, SecretsError> {
    let made = sign(broker, signer, issued)?;
    Ok(matches!(made, Signing::Made(_)))
}

#[test]
fn the_credential_proxy_refuses_a_signing_key_not_a_value_secret() -> TestResult {
    let keyed = Keyed::new(5)?;
    let served = Served::start(&keyed)?;
    let target = format!("/{KEY}/v1/models");
    let proxied = (&Method::GET, target.as_str(), &b""[..]);
    let (status, body) = served.held(&keyed, &keyed.on_key, proxied, true)?;
    assert_eq!(status, 403, "{body}");
    assert!(body.starts_with("not_a_value_secret"), "{body}");
    let handle = key_handle(&served, &keyed)?;
    assert_eq!(handle["used"], 0, "no use was admitted");
    Ok(())
}

#[test]
fn the_listing_of_a_signing_key_holds_its_public_key() -> TestResult {
    let keyed = Keyed::new(5)?;
    let served = Served::start(&keyed)?;
    let entry = listed_key(&served, &keyed)?.0;
    let expected = to_hex(&public_key_of(keyed.seed));
    assert_eq!(entry["public_key"], expected, "{entry}");
    Ok(())
}

#[test]
fn the_listing_of_a_signing_key_holds_its_purpose() -> TestResult {
    let keyed = Keyed::new(5)?;
    let served = Served::start(&keyed)?;
    let entry = listed_key(&served, &keyed)?.0;
    assert_eq!(entry["purpose"], "agent_request", "{entry}");
    assert_eq!(entry["class"], "signing_key", "{entry}");
    Ok(())
}

#[test]
fn the_listing_of_a_signing_key_holds_neither_the_seed_nor_its_hex() -> TestResult {
    let keyed = Keyed::new(5)?;
    let served = Served::start(&keyed)?;
    let (entry, listing) = listed_key(&served, &keyed)?;
    assert_eq!(entry["name"], KEY, "the key searched for is listed");
    assert!(!holds_seed(listing.as_bytes(), &keyed.seed), "{listing}");
    Ok(())
}

#[test]
fn a_signature_from_the_route_verifies_over_the_payload_of_its_members() -> TestResult {
    let keyed = Keyed::new(5)?;
    let served = Served::start(&keyed)?;
    let at = now_ms()?;
    let (status, body) = served.sign(&keyed, &keyed.on_key, &asked(KEY, &members(at))?)?;
    assert_eq!(status, 200, "{body}");
    let answer: Value = serde_json::from_str(&body)?;
    let cose = answer["signature"]
        .as_str()
        .and_then(from_hex)
        .ok_or("the answer holds no signature in hex")?;
    let expected = public_key_of(keyed.seed);
    let verifier = lys_identity_server::agent_signature::payload(METHOD, PATH, BODY, at, NONCE);
    assert_eq!(verifier, payload(at), "one payload for both sides");
    verify_attestation_bytes_by_signer(&cose, &verifier, &expected)?;
    assert_eq!(answer["public_key"], to_hex(&expected));
    let other = lys_core::agent_request::payload(METHOD, "/v1/other", &body_digest(), 1, NONCE);
    assert!(
        verify_attestation_bytes_by_signer(&cose, &other, &expected).is_err(),
        "the signature covers the members it was asked for and no others"
    );
    Ok(())
}

#[test]
fn the_agent_request_domain_label_is_written_in_one_source_file_under_crates() -> TestResult {
    let label = ["lys-identity", "agent-request", "v1"].join("/");
    assert_eq!(lys_core::agent_request::DOMAIN, label);
    assert_eq!(SigningPurpose::AgentRequest.domain(), label);
    let crates = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("the crate has no parent folder")?;
    let mut folders = vec![crates.to_path_buf()];
    let mut read = 0_usize;
    let mut holding: Vec<PathBuf> = Vec::new();
    while let Some(folder) = folders.pop() {
        for found in std::fs::read_dir(&folder)? {
            let path = found?.path();
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");
            if path.is_dir() {
                if name != "target" && !name.starts_with('.') {
                    folders.push(path);
                }
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                read += 1;
                if holds(&std::fs::read(&path)?, label.as_bytes()) {
                    holding.push(path);
                }
            }
        }
    }
    assert!(read > 100, "the walk read {read} source files");
    assert_eq!(holding, vec![crates.join("lys-core/src/agent_request.rs")]);
    Ok(())
}

#[test]
fn a_purpose_other_than_the_keys_own_is_refused_signing_purpose_mismatch() -> TestResult {
    let world = World::new()?;
    let mut broker = world.broker(granted(AGENT, &[CREDENTIAL]))?;
    let credential = Secret::from_slice(b"not a signing key");
    broker.seal(CREDENTIAL, GRANTOR, &credential)?;
    let signer = Signer::new(&world.keys(), AGENT)?;
    let issued = broker.issue(&signer.holder, CREDENTIAL, 5, NOW_MS + 60_000)?;
    let presented = signer.present(&issued.id)?;
    let asked = signable(SIGNED_AT_MS)?;
    let answered = broker.sign_for(&issued.token, &presented, CREDENTIAL, &asked);
    let refused = refusal(answered);
    assert!(refused.starts_with("signing_purpose_mismatch"), "{refused}");
    assert!(refused.contains("signs for none"), "{refused}");
    assert!(refused.contains("asked for agent_request"), "{refused}");
    let settled: Vec<String> = broker
        .audit()
        .audit_every_line()?
        .into_iter()
        .filter(|recorded| recorded.line.kind == AuditKind::Settlement)
        .map(|recorded| recorded.line.outcome)
        .collect();
    assert_eq!(settled, vec!["upstream_failed"], "the use was settled");
    Ok(())
}

#[test]
fn a_member_outside_the_typed_members_is_refused_400_and_uses_nothing() -> TestResult {
    let keyed = Keyed::new(5)?;
    let served = Served::start(&keyed)?;
    let at = now_ms()?;
    let digest = to_hex(&Sha256::digest(payload(at)));
    let mut inner = members(at);
    inner["digest"] = json!(digest);
    let outer = json!({
        "key": KEY,
        "purpose": "agent_request",
        "members": members(at),
        "bytes": to_hex(&payload(at)),
    });
    for body in [asked(KEY, &inner)?, serde_json::to_vec(&outer)?] {
        let (status, answer) = served.sign(&keyed, &keyed.on_key, &body)?;
        assert_eq!(status, 400, "{answer}");
        assert!(answer.starts_with("Encoding"), "{answer}");
    }
    assert_eq!(key_handle(&served, &keyed)?["used"], 0);
    let (status, answer) = served.sign(&keyed, &keyed.on_key, &fresh()?)?;
    assert_eq!(status, 200, "{answer}");
    let handle = key_handle(&served, &keyed)?;
    assert_eq!(handle["used"], 1, "a well-formed call counts");
    Ok(())
}

#[test]
fn an_unsigned_presentation_is_refused_presentation_unsigned() -> TestResult {
    let keyed = Keyed::new(5)?;
    let served = Served::start(&keyed)?;
    let body = fresh()?;
    let unsigned = (&Method::POST, SIGNATURE, body.as_slice());
    let (status, answer) = served.held(&keyed, &keyed.on_key, unsigned, false)?;
    assert_eq!(status, 400, "{answer}");
    assert!(answer.starts_with("PresentationUnsigned"), "{answer}");
    Ok(())
}

#[test]
fn a_lease_that_does_not_cover_the_key_is_refused_outside_scope() -> TestResult {
    let keyed = Keyed::new(5)?;
    let served = Served::start(&keyed)?;
    let body = fresh()?;
    let (status, answer) = served.sign(&keyed, &keyed.on_credential, &body)?;
    assert_eq!(status, 403, "{answer}");
    assert!(answer.starts_with("OutsideScope"), "{answer}");
    Ok(())
}

#[test]
fn a_caller_that_is_not_permitted_is_refused_permission_denied() -> TestResult {
    let world = World::new()?;
    let mut broker = world.broker(granted(AGENT, &[KEY]))?;
    let (signer, issued) = signing_world(&world, &mut broker, (5, NOW_MS + 60_000))?;
    assert!(signs(&mut broker, &signer, &issued)?);
    assert!(broker.permissions().revoke(AGENT, KEY));
    let refused = refusal(sign(&mut broker, &signer, &issued));
    assert!(refused.starts_with("PermissionDenied"), "{refused}");
    Ok(())
}

#[test]
fn a_signing_time_away_from_the_brokers_clock_is_refused_before_admission() -> TestResult {
    let world = World::new()?;
    let mut broker = world.broker(granted(AGENT, &[KEY]))?;
    let (signer, issued) = signing_world(&world, &mut broker, (5, NOW_MS + 60_000))?;
    let presented = signer.present(&issued.id)?;
    let ahead = signable(SIGNED_AT_MS + 86_400_000)?;
    let refused = refusal(broker.sign_for(&issued.token, &presented, KEY, &ahead));
    assert!(refused.starts_with("PresentationStale"), "{refused}");
    let uses = broker
        .audit()
        .audit_every_line()?
        .into_iter()
        .filter(|recorded| recorded.line.kind == AuditKind::Use)
        .count();
    assert_eq!(uses, 0, "nothing was admitted");
    assert!(signs(&mut broker, &signer, &issued)?);
    Ok(())
}

#[test]
fn no_answer_of_the_route_holds_the_seed_or_its_hex() -> TestResult {
    let keyed = Keyed::new(5)?;
    let served = Served::start(&keyed)?;
    let body = fresh()?;
    let mut answers = vec![served.sign(&keyed, &keyed.on_key, &body)?];
    let unsigned = (&Method::POST, SIGNATURE, body.as_slice());
    answers.push(served.held(&keyed, &keyed.on_key, unsigned, false)?);
    answers.push(served.sign(&keyed, &keyed.on_credential, &body)?);
    let mut inner = members(now_ms()?);
    inner["seed"] = json!(true);
    answers.push(served.sign(&keyed, &keyed.on_key, &asked(KEY, &inner)?)?);
    let statuses: Vec<u16> = answers.iter().map(|answer| answer.0).collect();
    assert_eq!(statuses, vec![200, 400, 403, 400], "{answers:?}");
    for answer in &answers {
        assert!(!holds_seed(answer.1.as_bytes(), &keyed.seed), "{answer:?}");
    }
    Ok(())
}

#[test]
fn no_log_line_the_broker_writes_holds_the_seed_or_its_hex() -> TestResult {
    let keyed = Keyed::new(5)?;
    let served = Served::start_logged(&keyed)?;
    let body = fresh()?;
    let (status, answer) = served.sign(&keyed, &keyed.on_key, &body)?;
    assert_eq!(status, 200, "{answer}");
    let unsigned = (&Method::POST, SIGNATURE, body.as_slice());
    let (status, answer) = served.held(&keyed, &keyed.on_key, unsigned, false)?;
    assert_eq!(status, 400, "{answer}");
    revoke(&served, &keyed)?;
    let (status, answer) = served.sign(&keyed, &keyed.on_key, &body)?;
    assert_eq!(status, 403, "{answer}");
    drop(served);
    let log = keyed.log()?;
    let text = String::from_utf8_lossy(&log);
    assert!(!holds_seed(&log, &keyed.seed), "{text}");
    Ok(())
}

#[test]
fn one_admitted_signing_use_is_one_use_line_admitted_at_the_audit_route() -> TestResult {
    let keyed = Keyed::new(5)?;
    let served = Served::start(&keyed)?;
    assert_eq!(audit_count(&served, &keyed, "use", "admitted")?, 0);
    let (status, answer) = served.sign(&keyed, &keyed.on_key, &fresh()?)?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(audit_count(&served, &keyed, "use", "admitted")?, 1);
    Ok(())
}

#[test]
fn one_refused_signing_use_is_one_use_line_naming_its_refusal() -> TestResult {
    let keyed = Keyed::new(5)?;
    let served = Served::start(&keyed)?;
    let body = fresh()?;
    let unsigned = (&Method::POST, SIGNATURE, body.as_slice());
    let (status, answer) = served.held(&keyed, &keyed.on_key, unsigned, false)?;
    assert_eq!(status, 400, "{answer}");
    let count = audit_count(&served, &keyed, "use", "PresentationUnsigned")?;
    assert_eq!(count, 1);
    assert_eq!(audit_count(&served, &keyed, "use", "admitted")?, 0);
    Ok(())
}

#[test]
fn after_a_reopen_a_lease_of_two_uses_signed_twice_refuses_lease_exhausted() -> TestResult {
    let world = World::new()?;
    let mut broker = world.broker(granted(AGENT, &[KEY]))?;
    let (signer, issued) = signing_world(&world, &mut broker, (2, NOW_MS + 60_000))?;
    assert!(signs(&mut broker, &signer, &issued)?);
    assert!(signs(&mut broker, &signer, &issued)?);
    drop(broker);
    let grants = granted(AGENT, &[KEY]);
    let mut reopened = Broker::open(&world.paths(), grants, Box::new(|| NOW_MS))?;
    let refused = refusal(sign(&mut reopened, &signer, &issued));
    assert!(refused.starts_with("LeaseExhausted"), "{refused}");
    Ok(())
}

#[test]
fn after_the_lease_is_revoked_the_next_signing_call_is_refused_handle_dropped() -> TestResult {
    let keyed = Keyed::new(5)?;
    let served = Served::start(&keyed)?;
    revoke(&served, &keyed)?;
    let (status, answer) = served.sign(&keyed, &keyed.on_key, &fresh()?)?;
    assert_eq!(status, 403, "{answer}");
    assert!(answer.starts_with("HandleDropped"), "{answer}");
    Ok(())
}

#[test]
fn after_the_lease_is_revoked_the_next_signing_call_answers_no_signature() -> TestResult {
    let keyed = Keyed::new(5)?;
    let served = Served::start(&keyed)?;
    let body = fresh()?;
    let (status, before) = served.sign(&keyed, &keyed.on_key, &body)?;
    assert_eq!(status, 200, "{before}");
    assert!(before.contains("\"signature\""), "{before}");
    revoke(&served, &keyed)?;
    let (status, after) = served.sign(&keyed, &keyed.on_key, &body)?;
    assert_ne!(status, 200, "{after}");
    assert!(!after.contains("\"signature\""), "{after}");
    assert!(serde_json::from_str::<Value>(&after).is_err(), "{after}");
    Ok(())
}

#[test]
fn after_the_window_closes_the_next_sign_for_is_refused_lease_window_closed() -> TestResult {
    let world = World::new()?;
    let clock = Arc::new(AtomicI64::new(NOW_MS));
    let read = Arc::clone(&clock);
    let grants = granted(AGENT, &[KEY]);
    let mut broker = world.broker_on(grants, Box::new(move || read.load(Ordering::SeqCst)))?;
    let window_ends = NOW_MS + 20_000;
    let (signer, issued) = signing_world(&world, &mut broker, (5, window_ends))?;
    assert!(signs(&mut broker, &signer, &issued)?);
    clock.store(window_ends + 1, Ordering::SeqCst);
    let refused = refusal(sign(&mut broker, &signer, &issued));
    assert!(refused.starts_with("LeaseWindowClosed"), "{refused}");
    Ok(())
}

#[test]
fn after_the_handle_is_dropped_the_next_signing_call_is_refused_handle_dropped() -> TestResult {
    let keyed = Keyed::new(5)?;
    let served = Served::start(&keyed)?;
    let ending = serde_json::to_vec(&json!({
        "handle": keyed.on_key.id, "operation": "signing-key-drop-operation-01"
    }))?;
    let (status, answer) = served.ask(&keyed, &Method::POST, "/_lys/drop", &ending)?;
    assert_eq!(status, 200, "the drop: {answer}");
    let (status, answer) = served.sign(&keyed, &keyed.on_key, &fresh()?)?;
    assert_eq!(status, 403, "{answer}");
    assert!(answer.starts_with("HandleDropped"), "{answer}");
    Ok(())
}

/// The audit log of a served broker that signed once, refused a signature
/// once and had the lease revoked, every file of it read whole, and the
/// signing time the signature was made for.
fn audited(keyed: &Keyed) -> Result<(Vec<Vec<u8>>, u64), Failure> {
    let served = Served::start(keyed)?;
    let at = now_ms()?;
    let body = asked(KEY, &members(at))?;
    let (status, answer) = served.sign(keyed, &keyed.on_key, &body)?;
    assert_eq!(status, 200, "{answer}");
    let unsigned = (&Method::POST, SIGNATURE, body.as_slice());
    let (status, answer) = served.held(keyed, &keyed.on_key, unsigned, false)?;
    assert_eq!(status, 400, "{answer}");
    revoke(&served, keyed)?;
    drop(served);
    let files = keyed.audit_bytes()?;
    let named = files.iter().any(|file| holds(file, KEY.as_bytes()));
    assert!(named, "the log names the key");
    Ok((files, at))
}

#[test]
fn no_audit_line_holds_the_keys_public_key_in_hex() -> TestResult {
    let keyed = Keyed::new(5)?;
    let public = to_hex(&public_key_of(keyed.seed));
    let upper = public.to_uppercase();
    for file in audited(&keyed)?.0 {
        assert!(!holds(&file, public.as_bytes()));
        assert!(!holds(&file, upper.as_bytes()));
    }
    Ok(())
}

#[test]
fn no_audit_line_holds_the_hex_sha256_of_the_bytes_signed() -> TestResult {
    let keyed = Keyed::new(5)?;
    let (files, at) = audited(&keyed)?;
    let signed = to_hex(&Sha256::digest(payload(at)));
    let upper = signed.to_uppercase();
    for file in files {
        assert!(!holds(&file, signed.as_bytes()));
        assert!(!holds(&file, upper.as_bytes()));
    }
    Ok(())
}

#[test]
fn no_audit_line_holds_the_seed_or_its_hex() -> TestResult {
    let keyed = Keyed::new(5)?;
    for file in audited(&keyed)?.0 {
        assert!(!holds_seed(&file, &keyed.seed));
    }
    Ok(())
}
