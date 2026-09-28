//! Certificate revocation as one appended leaf, and the live set folded from
//! the log.
//!
//! One certificate log per issuing authority holds three kinds of leaf under
//! the domain tag `lys-identity/certificate-log/v1`: an issuance leaf carrying
//! a certificate's DER, a revocation leaf carrying the certificate's hash and
//! the issuing authority's signature over `lys-identity/certificate-revocation/v1`
//! bytes that bind the log's origin, and an attestation entry that places an
//! attestation by the certificate's key in log order.
//!
//! # Invariants
//!
//! - A revocation is a leaf appended at the store's extent, never a removal,
//!   a rewrite or a truncation; the `LeafStore` trait is used as written.
//! - Only the issuing authority's key signs a revocation, and the fold refuses
//!   a revocation signed by any other key or for any other log's origin.
//! - A revocation is permanent: no leaf reinstates a revoked certificate, and
//!   a replay of its issuance leaf after the revocation is refused by name.
//! - A revocation names a certificate the log already holds issued; one that
//!   does not is refused and revokes nothing.
//! - The fold reads no clock: an issuance leaf is checked at the certificate's
//!   own notBefore instant, and every other instant is an input.
//! - Verification without a log, lys-core's `verify_certificate_chain` and
//!   `verify_certificate_chain_at` and the `lys ca verify` command, does not
//!   check revocation; only [`verify_certificate`] against a log does.
//! - A certificate's history keeps verifying after its revocation: its
//!   issuance leaf, the log's proofs and the attestations whose entries
//!   precede the revocation leaf.
//!
//! The contract is `docs/design/identity/CERTIFICATE-REVOCATION.md`.

pub mod error;
pub mod leaf;

pub use error::{AppendOperation, LeafRefusal, RevocationError};
pub use leaf::{
    CertificateHash, CertificateLeaf, LEAF_DOMAIN_TAG, REVOCATION_DOMAIN_TAG, REVOCATION_LEAF_LEN,
    Revocation, revocation_signed_bytes,
};
