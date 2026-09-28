//! The actor of an event is always a person, named by one of their logins.
//! The method says how the service authenticated them: their own OIDC
//! sign-in, or a request signed by an agent they are responsible for, and
//! then the event keeps that agent's id. An event written before the second
//! method existed reads back to the same event and the same bytes, and an
//! actor whose method and agent id disagree is refused.

use std::error::Error;
use std::os::unix::fs::PermissionsExt;

use lys_core::Ed25519Identity;
use lys_identity::encoding::{decode_body, encode_body};
use lys_identity::{
    Actor, AgentId, AuthMethod, Change, IdentityError, IdentityEvent, IdentityId, LinkChange,
    LinkObservation, LoginBinding, OperationId, PersonId, Provenance, sign_event, verify_event,
};

type TestResult = Result<(), Box<dyn Error>>;

const ISSUER: &str = "https://issuer.test";
const SUBJECT: &str = "link-audit-source";
const AUTHENTICATED_AT: u64 = 1_790_000_000;

/// The body of [`observed`] by an OIDC actor, as it was encoded before the
/// agent-signature method existed.
const BODY_BEFORE: &str = "a7010102500707070707070707070707070707070703a4017368747470733a2f2f6973737565722e7465737402716c696e6b2d61756469742d736f757263650301041a6ab13b8004a20101025001010101010101010101010101010101051a6ab13be4060607a6016b736f757263652d6f702d310201037568747470733a2f2f6163636f756e74732e74657374046d6164612d656c73657768657265057368747470733a2f2f6973737565722e74657374061a6ab13bb2";

/// The signed message of that body under the service key over seed 11, as it
/// was written before the agent-signature method existed.
const MESSAGE_BEFORE: &str = "d2845853a3012703782a6170706c69636174696f6e2f766e642e6c79732e6964656e746974792d6576656e742e76312b63626f7204582066be7e332c7a453332bd9d0a7f7db055f5c5ef1a06ada66d98b39fb6810c473aa058b7a7010102500707070707070707070707070707070703a4017368747470733a2f2f6973737565722e7465737402716c696e6b2d61756469742d736f757263650301041a6ab13b8004a20101025001010101010101010101010101010101051a6ab13be4060607a6016b736f757263652d6f702d310201037568747470733a2f2f6163636f756e74732e74657374046d6164612d656c73657768657265057368747470733a2f2f6973737565722e74657374061a6ab13bb25840503d2d487eb6b2a3a77584ec3e2e9b1b202e646422120813b78577f99646289ba18e04e1cd51dda7b68522c95fc3aac975f08aa0abd453425851f599a9ce4c01";

/// Where the actor's method code stands in a body whose actor is
/// [`ISSUER`] and [`SUBJECT`]: after the body's map head, the version pair,
/// the operation pair, the actor's key and map head, and the issuer and
/// subject pairs, each text under a one-byte head, comes key 3 and its code.
const METHOD_AT: usize = 1 + 2 + (2 + 16) + 2 + (2 + ISSUER.len()) + (2 + SUBJECT.len()) + 1;

fn unhex(text: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    (0..text.len())
        .step_by(2)
        .map(|at| {
            let pair = text.get(at..at + 2).ok_or("an odd number of hex digits")?;
            Ok(u8::from_str_radix(pair, 16)?)
        })
        .collect()
}

fn service_key(seed: u8) -> Result<(Ed25519Identity, tempfile::TempDir), Box<dyn Error>> {
    let dir = tempfile::TempDir::new()?;
    let path = dir.path().join("service.key");
    std::fs::write(&path, [seed; 32])?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    Ok((Ed25519Identity::load(&path)?, dir))
}

fn agent() -> AgentId {
    AgentId::from_bytes([5; 16])
}

/// A link observation about one person, recorded for the actor `provenance` attests.
fn observed(provenance: Provenance) -> Result<IdentityEvent, IdentityError> {
    IdentityEvent::new(
        OperationId::from_bytes([7; 16]),
        Actor::new(LoginBinding::new(ISSUER, SUBJECT)?, provenance),
        IdentityId::Person(PersonId::from_bytes([1; 16])),
        1_790_000_100,
        Change::LinkAudit(LinkObservation::new(
            "source-op-1",
            LinkChange::Linked,
            LoginBinding::new("https://accounts.test", "ada-elsewhere")?,
            "https://issuer.test",
            1_790_000_050,
        )?),
    )
}

fn by_oidc() -> Result<IdentityEvent, IdentityError> {
    observed(Provenance::new(AuthMethod::Oidc, AUTHENTICATED_AT))
}

fn by_agent() -> Result<IdentityEvent, IdentityError> {
    observed(Provenance::by_agent(agent(), AUTHENTICATED_AT))
}

/// `body` with its actor's method code replaced by `code`.
fn with_method(mut body: Vec<u8>, code: u8) -> Result<Vec<u8>, Box<dyn Error>> {
    assert_eq!(body.get(METHOD_AT - 1), Some(&3), "the method's key");
    *body.get_mut(METHOD_AT).ok_or("the body is too short")? = code;
    Ok(body)
}

fn refused_as_malformed(body: &[u8], names: &str) {
    let refused = decode_body(body);
    assert!(
        matches!(&refused, Err(IdentityError::EventMalformed { reason }) if reason.contains(names)),
        "expected EventMalformed naming `{names}`, read {refused:?}"
    );
}

#[test]
fn an_event_written_before_the_agent_signature_method_reads_unchanged() -> TestResult {
    let body = unhex(BODY_BEFORE)?;
    let event = decode_body(&body)?;
    assert_eq!(event, by_oidc()?);
    assert_eq!(event.actor().provenance().method(), AuthMethod::Oidc);
    assert_eq!(event.actor().provenance().agent(), None);
    assert_eq!(encode_body(&event), body, "an OIDC event encodes as before");

    let (key, _dir) = service_key(11)?;
    let message = unhex(MESSAGE_BEFORE)?;
    let read = verify_event(&message, &key.public_key_bytes())?;
    assert_eq!(read.event(), &event);
    assert_eq!(read.bytes(), message.as_slice());
    assert_eq!(
        sign_event(event, &key)?.bytes(),
        message.as_slice(),
        "an OIDC event signs to the message it signed to before"
    );
    Ok(())
}

#[test]
fn an_agent_signed_event_round_trips_and_keeps_the_agent() -> TestResult {
    let original = by_agent()?;
    let (key, _dir) = service_key(11)?;
    let signed = sign_event(original.clone(), &key)?;
    let read = verify_event(signed.bytes(), &key.public_key_bytes())?;
    assert_eq!(read.event(), &original);
    assert_eq!(read.bytes(), signed.bytes());

    let body = encode_body(&original);
    let decoded = decode_body(&body)?;
    assert_eq!(encode_body(&decoded), body);
    let actor = decoded.actor();
    assert_eq!(
        actor.binding(),
        &LoginBinding::new(ISSUER, SUBJECT)?,
        "the actor is the person, named by their login"
    );
    assert_eq!(
        actor.provenance().method(),
        AuthMethod::AgentSignature(agent())
    );
    assert_eq!(actor.provenance().agent(), Some(agent()));
    assert_eq!(actor.provenance().authenticated_at(), AUTHENTICATED_AT);
    assert_ne!(decoded, by_oidc()?, "the method is part of the event");
    Ok(())
}

#[test]
fn an_agent_signature_method_that_names_no_agent_is_refused() -> TestResult {
    let body = with_method(encode_body(&by_oidc()?), 2)?;
    refused_as_malformed(&body, "agent-signature actor");
    Ok(())
}

#[test]
fn an_oidc_method_that_names_an_agent_is_refused() -> TestResult {
    let body = with_method(encode_body(&by_agent()?), 1)?;
    refused_as_malformed(&body, "OIDC actor");
    Ok(())
}

#[test]
fn a_method_other_than_the_two_is_refused() -> TestResult {
    for body in [encode_body(&by_oidc()?), encode_body(&by_agent()?)] {
        refused_as_malformed(&with_method(body, 3)?, "method code");
    }
    Ok(())
}
