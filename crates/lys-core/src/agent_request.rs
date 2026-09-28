//! The bytes an agent signs for a request to the identity service.
//!
//! **Invariant:** one function builds them, and both sides call it: the
//! verifier in `lys-identity-server` and the secrets broker, which signs for
//! an agent with a key it holds. The domain label lives here and in no other
//! source file, so the signer and the verifier cannot drift apart; a change
//! to the layout is a change to [`payload`] alone.
//!
//! The payload is the domain label, the request's method, its path, the
//! SHA-256 digest of its body as 64 lower-case hex characters, the signing
//! time in milliseconds since the Unix epoch and the nonce in hex, each on a
//! line of its own. It is a wire format: a signature made over it is checked
//! for as long as it is kept, so it changes only by a new domain label
//! beside this one, never by an edit of this one.

/// The domain every agent request signature is made under.
pub const DOMAIN: &str = "lys-identity/agent-request/v1";

/// The bytes an agent signs for the request `method` `path` whose body has
/// the SHA-256 digest `body_digest` (64 lower-case hex characters), signed
/// at `signed_at_ms` under `nonce`.
pub fn payload(
    method: &str,
    path: &str,
    body_digest: &str,
    signed_at_ms: u64,
    nonce: &str,
) -> Vec<u8> {
    format!("{DOMAIN}\n{method}\n{path}\n{body_digest}\n{signed_at_ms}\n{nonce}").into_bytes()
}
