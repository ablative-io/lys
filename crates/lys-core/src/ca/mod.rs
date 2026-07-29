//! Certificate authority operations: Ed25519-rooted X.509 issuance, proof of
//! possession for holder-controlled subject keys, custom extension transport,
//! and chain verification.

pub mod authority;
pub mod certificate;
pub mod extensions;
pub(crate) mod rcgen_bridge;
pub mod request;

pub use authority::{CertificateAuthority, verify_certificate_chain, verify_certificate_chain_at};
pub use certificate::{CertifiedKey, IssuedCertificate, certificate_subject_public_key};
pub use extensions::{CustomExtension, LYS_OID_ARC, decode_extension, encode_extension};
pub use request::{VerifiedRequest, create_certificate_request, verify_certificate_request};
