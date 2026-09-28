//! `lys ca` subcommands — issue and verify Ed25519-rooted X.509 certificates.
//!
//! Issuance wraps [`lys_core::ca::CertificateAuthority`]: the issuer identity
//! at `--key` signs a certificate for a named subject, valid from now for the
//! window `--validity` (or `--validity-days`) asks for — the library's TTL
//! model, which takes a `Duration` and does not backdate. Sub-day windows are
//! expressible because short-lived scoped grants need them; see
//! [`crate::commands::duration`].
//! Capability claims, when supplied, are validated as JSON and embedded
//! byte-for-byte as a non-critical extension under the lys OID arc with
//! sub-component `1` (`1.3.6.1.4.1.66364.1`); the library carries them as
//! opaque DER and this CLI defines no further semantics. Certificates are
//! written as PEM, the X.509 interop norm.
//!
//! Two issuance paths are offered, and they mean different things. Without
//! `--request`, the library generates the subject keypair and discards the
//! private half: the certificate names a key nobody ever held, which is fine
//! for testing the plumbing and useless for concluding anything about a
//! holder. With `--request`, the subject presents a PKCS#10 request produced by
//! `lys ca request` (or `openssl req`) and self-signed by a key they already
//! control; that proof of possession is verified before issuance and the
//! certificate binds their key. The reported `subject_key_origin` says which
//! path produced the certificate, because the distinction is the whole
//! difference in what the certificate is evidence of.
//!
//! Every issuance is entered in a transparency log before anything is
//! written (see [`crate::commands::ca_log`]), holding only the CA key.
//!
//! The issuer's self-signed certificate is built once per issuer key and
//! stored beside it, at the key file's path with `.issuer.pem` appended, so
//! `lys ca issuer-cert` and `lys ca issue --issuer-out` write the same bytes
//! on every call. The stored file is public; it is never rebuilt, overwritten
//! or deleted once it exists, and one that is not the key's own is refused by
//! name before anything is written or appended.
//!
//! Invariants: the issuer key file must already exist — only `lys key
//! generate` creates key material — and the subject keypair the library
//! generates during issuance is discarded, never written to disk or printed;
//! only its public half is reported. `ca request` writes only a public
//! artifact: a request carries a public key and a signature, never the seed.
//! Verification failures collapse to one non-oracle message, mirroring `lys
//! verify`. Claims echoed by `ca verify` are printed verbatim only when free
//! of control characters; anything else is shown as hex, so certificate
//! contents can never inject terminal escape sequences into the verification
//! output.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use lys_core::TrustError;
use lys_core::ca::{
    CertificateAuthority, LYS_OID_ARC, create_certificate_request, decode_extension,
    encode_extension, verify_certificate_chain_at,
};
use x509_parser::prelude::{FromDer, X509Certificate};

use crate::commands::ca_log::LogEntry;
use crate::commands::error::{CliError, CliResult};
use crate::commands::files::{
    StagedFile, read_file, refuse_existing, refuse_shared_paths, write_file,
};
use crate::commands::hex::{hex_lower, parse_hex_32};
use crate::commands::key::load_identity;
use crate::commands::output::Emitter;
use crate::commands::pem;

/// Sub-component appended to [`LYS_OID_ARC`] for the CLI's capability-claims
/// extension. Part of the wire contract: certificates issued by this CLI
/// carry claims under `1.3.6.1.4.1.66364.1`, and `lys ca verify` reads them
/// back from the same OID.
const CAPABILITY_CLAIMS_COMPONENT: u64 = 1;

/// The full OID under which this CLI transports capability claims.
///
/// Shared with `lys inspect cert`, which reads claims back from the same OID
/// without verifying the certificate that carries them.
pub(crate) fn capability_claims_oid() -> Vec<u64> {
    let mut oid = LYS_OID_ARC.to_vec();
    oid.push(CAPABILITY_CLAIMS_COMPONENT);
    oid
}

/// Whether claim text can be echoed to a terminal verbatim.
///
/// Certificate contents are attacker-influenceable (any issuer under the
/// trusted key can embed arbitrary bytes, and JSON strings may carry raw
/// control characters), so anything containing control characters beyond
/// newline and tab — including ANSI escape sequences that could spoof the
/// surrounding verification output — falls back to hex.
///
/// Shared with `lys inspect cert`, which reads the same fields out of
/// certificates nothing has vouched for and so needs the identical screen.
pub(crate) fn is_terminal_safe(text: &str) -> bool {
    text.chars()
        .all(|character| !character.is_control() || character == '\n' || character == '\t')
}

/// What an issuance produced, independent of which path produced it.
struct Issuance {
    der_bytes: Vec<u8>,
    subject_public_key: [u8; 32],
    issuer_public_key: [u8; 32],
    fingerprint: [u8; 32],
    expires_at: DateTime<Utc>,
    /// Human-readable provenance of the subject key, reported so an operator
    /// can tell whether the certificate binds a key its holder proved control
    /// of or one this command minted and threw away.
    subject_key_origin: &'static str,
}

/// `lys ca request --key <path> --subject <name> --out <file>`.
///
/// Produces the holder's side of a certificate exchange: a PKCS#10 request
/// carrying the identity's public key, self-signed by that identity. The
/// signature is the proof of possession `lys ca issue --request` verifies. The
/// written file is public — it contains no private material.
///
/// # Errors
///
/// Returns [`CliError::KeyFileMissing`] if the identity key file does not
/// exist, [`CliError::Io`] if the request cannot be written, and
/// [`CliError::Trust`] if the subject is empty or the library cannot sign the
/// request.
pub fn request(key: &Path, subject: &str, out: &Path, json: bool) -> CliResult<()> {
    let identity = Arc::new(load_identity(key)?);
    let der = create_certificate_request(&identity, subject)?;

    let pem_text = pem::encode_certificate_request(&der);
    write_file(out, pem_text.as_bytes(), "certificate-signing request file")?;

    let mut emit = Emitter::new(json);
    emit.field(
        "certificate-signing request for subject",
        "subject",
        subject,
    );
    emit.field(
        "subject public key (ed25519)",
        "subject_public_key",
        hex_lower(&identity.public_key_bytes()),
    );
    emit.field("request written", "request_path", out.display().to_string());
    emit.note("give this to the authority; it carries no private key material");
    emit.finish();
    Ok(())
}

/// The path of the issuer certificate stored beside an issuer key: the key
/// file's path with `.issuer.pem` appended (for `issuer.key`,
/// `issuer.key.issuer.pem`).
#[must_use]
pub fn stored_issuer_certificate_path(key: &Path) -> PathBuf {
    let mut path = key.as_os_str().to_owned();
    path.push(".issuer.pem");
    PathBuf::from(path)
}

/// What the stored issuer certificate file is called in errors.
const STORED_ISSUER: &str = "stored issuer certificate";

/// An issuer key's stored issuer certificate, as read from disk or as built
/// by this run.
struct StoredIssuer {
    /// The PEM bytes every output of the issuer certificate is written with.
    pem: Vec<u8>,
    /// The stored file, staged but not yet placed, when this run built it.
    /// `None` when it was already on disk.
    built: Option<StagedFile>,
}

/// Reads the issuer certificate stored beside `key` and checks it is the
/// authority's own, or, when there is none, builds it once and stages it at
/// the stored path for the caller to place.
///
/// # Errors
///
/// [`CliError::StoredIssuerCertificateInvalid`] if the stored file is not one
/// PEM `CERTIFICATE` block whose subject public key is the authority's,
/// [`CliError::Io`] if it cannot be read or staged, and [`CliError::Trust`]
/// if the issuer certificate cannot be built.
fn stored_issuer_certificate(
    key: &Path,
    authority: &CertificateAuthority,
) -> CliResult<StoredIssuer> {
    let path = stored_issuer_certificate_path(key);
    match std::fs::read(&path) {
        Ok(pem_bytes) => {
            check_stored_issuer(&pem_bytes, &path, key, &authority.public_key_bytes())?;
            Ok(StoredIssuer {
                pem: pem_bytes,
                built: None,
            })
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            let pem_text = pem::encode_certificate(&authority.issuer_certificate_der()?);
            let built = StagedFile::stage(&path, pem_text.as_bytes(), STORED_ISSUER)?;
            Ok(StoredIssuer {
                pem: pem_text.into_bytes(),
                built: Some(built),
            })
        }
        Err(source) => Err(CliError::Io {
            context: format!(
                "failed to read the {STORED_ISSUER} {} beside the issuer key",
                path.display()
            ),
            source,
        }),
    }
}

/// Refuses a stored issuer certificate that is not exactly one PEM
/// `CERTIFICATE` block holding one X.509 certificate whose subject public key
/// is `issuer`.
fn check_stored_issuer(
    pem_bytes: &[u8],
    path: &Path,
    key: &Path,
    issuer: &[u8; 32],
) -> CliResult<()> {
    let refuse = |reason: String| CliError::StoredIssuerCertificateInvalid {
        path: path.to_path_buf(),
        key: key.to_path_buf(),
        reason,
    };
    let der = match pem::decode_certificate(pem_bytes, path) {
        Ok(der) => der,
        Err(err) => return Err(refuse(err.to_string())),
    };
    let (rest, certificate) = match X509Certificate::from_der(&der) {
        Ok(parsed) => parsed,
        Err(err) => return Err(refuse(format!("its body is not X.509: {err:?}"))),
    };
    if !rest.is_empty() {
        let trailing = rest.len();
        return Err(refuse(format!("{trailing} bytes follow the certificate")));
    }
    let subject_key = certificate.public_key().subject_public_key.data.as_ref();
    if subject_key != issuer.as_slice() {
        let found = hex_lower(subject_key);
        return Err(refuse(format!("its subject public key is {found}")));
    }
    Ok(())
}

/// `lys ca issuer-cert --key <path> --out <file>`.
///
/// Writes the issuer's self-signed CA certificate, the one stored beside the
/// key (see [`stored_issuer_certificate_path`]), so a person who did not run
/// an issuance can still check its certificates with `openssl verify -CAfile`.
/// The stored certificate is built once, when first asked for, and every
/// later call writes its bytes unchanged. It is public: it carries the issuer
/// public key and a signature, never the seed. No log is opened.
///
/// # Errors
///
/// [`CliError::OutputExists`] if `out` already exists, refused before the
/// stored certificate is built or written; [`CliError::OutputPathShared`] if
/// `out` names the stored file; [`CliError::KeyFileMissing`] if the key file
/// does not exist; [`CliError::StoredIssuerCertificateInvalid`] if the stored
/// file is not the key's own; [`CliError::Io`] if a file cannot be read or
/// written; and [`CliError::Trust`] if the issuer certificate cannot be built.
pub fn issuer_cert(key: &Path, out: &Path, json: bool) -> CliResult<()> {
    let stored_path = stored_issuer_certificate_path(key);
    refuse_shared_paths(&[
        ("issuer certificate file", out),
        (STORED_ISSUER, stored_path.as_path()),
    ])?;
    refuse_existing(out, "issuer certificate file")?;
    let authority = CertificateAuthority::new(load_identity(key)?);
    let stored = stored_issuer_certificate(key, &authority)?;
    if let Some(built) = stored.built {
        built.place()?;
    }
    let written = StagedFile::stage(out, &stored.pem, "issuer certificate file")?;
    written.place()?;

    let mut emit = Emitter::new(json);
    emit.field(
        "issuer public key (ed25519)",
        "issuer_public_key",
        hex_lower(&authority.public_key_bytes()),
    );
    emit.field(
        "stored issuer certificate",
        "stored_issuer_certificate_path",
        stored_path.display().to_string(),
    );
    emit.field(
        "issuer certificate written",
        "issuer_certificate_path",
        out.display().to_string(),
    );
    emit.note("public: it carries no private key material");
    emit.finish();
    Ok(())
}

/// Where an issuance writes what it produces.
#[derive(Debug)]
pub struct IssueOutputs<'a> {
    /// The PEM certificate.
    pub certificate: &'a Path,
    /// Where to write the issuer's stored self-signed PEM certificate, for
    /// standard X.509 tooling.
    pub issuer_certificate: Option<&'a Path>,
    /// The transparency log the certificate is entered in before it is written.
    pub log: LogEntry<'a>,
}

impl IssueOutputs<'_> {
    /// Every output path this issuance writes, each with what it holds.
    fn named(&self) -> Vec<(&'static str, &Path)> {
        let mut named = vec![("certificate file", self.certificate)];
        if let Some(path) = self.issuer_certificate {
            named.push(("issuer certificate file", path));
        }
        named.push(("leaf file", self.log.leaf_out));
        if let Some(proof) = &self.log.proof {
            named.push(("inclusion proof artifact", proof.artifact_out));
        }
        named
    }
}

/// `lys ca issue --key <path> --subject <name> [--request <file>]
/// [--claims <file>] (--validity <window> | --validity-days <n>) --out <file>
/// [--issuer-out <file>] --log <dir> --leaf-out <file> [--log-key <path>
/// --artifact-out <file>]`.
///
/// The certificate is entered in the transparency log at `--log` as one leaf
/// whose bytes are its DER, and nothing else, before anything is written, and
/// the leaf is written at `--leaf-out`. Only the CA key is needed: the log's
/// operator makes the inclusion-proof artifact with `lys log prove inclusion`
/// from the reported leaf index, and a third party verifies the entry with
/// `scripts/verify_inclusion.py` and the certificate with `openssl verify`
/// against the issuer certificate. `--log-key` and `--artifact-out` together
/// make the artifact in this run, for one operator holding both keys. A log
/// that cannot be opened, or refuses the entry, stops the issuance before the
/// certificate is written, so no certificate this command writes is missing
/// from its log.
///
/// `--issuer-out` writes the issuer certificate stored beside the key (see
/// [`stored_issuer_certificate_path`]), building and storing it first if this
/// is the first time it is asked for; the stored file is staged with the
/// other outputs and placed only after the append.
///
/// Every output is refused, before anything is signed or appended, if it
/// already exists or shares a path with another output. Each is written to a
/// flushed temporary file and renamed into place, so none is ever torn. The
/// certificate, issuer certificate and leaf are staged before the append; a
/// failure after it names the entry and the command that recovers its
/// artifact, and never appends or signs again (see [`crate::commands::ca_log`]).
///
/// `ttl` is the already-resolved validity window; the two flags are reconciled
/// in [`crate::commands::duration::validity_window`] so this function has one
/// notion of the window rather than two.
///
/// With `--request`, the subject key comes from the holder's PKCS#10 request
/// and is certified only after its proof of possession verifies; `--subject`
/// must equal the common name the request asked for. Without `--request`, a
/// subject keypair is generated here and its private half discarded.
///
/// # Errors
///
/// Returns [`CliError::KeyFileMissing`] if the issuer key file does not
/// exist, [`CliError::Io`] if the claims file, request file, or output
/// certificate cannot be read or written, [`CliError::ClaimsJsonParse`] if the
/// claims file is not valid JSON, [`CliError::PemParse`] if the request is not
/// a PEM `CERTIFICATE REQUEST` block, and [`CliError::Trust`] if the library
/// rejects the issuance parameters, rejects the request's proof of possession,
/// or signing fails. [`CliError::OutputExists`] and
/// [`CliError::OutputPathShared`] refuse outputs before anything is signed;
/// the log's own refusal (an uninitialized or invalid log directory, or the
/// store's message) stops the run before anything is written;
/// [`CliError::StoredIssuerCertificateInvalid`] refuses a stored issuer
/// certificate that is not the key's own; and
/// [`CliError::LoggedButUnwritten`] reports a failure after the log entry.
pub fn issue(
    key: &Path,
    subject: &str,
    claims: Option<&Path>,
    ttl: Duration,
    outputs: &IssueOutputs<'_>,
    request_path: Option<&Path>,
    json: bool,
) -> CliResult<()> {
    let named = outputs.named();
    let stored_path = outputs
        .issuer_certificate
        .map(|_| stored_issuer_certificate_path(key));
    let mut distinct = named.clone();
    if let Some(path) = &stored_path {
        distinct.push((STORED_ISSUER, path.as_path()));
    }
    refuse_shared_paths(&distinct)?;
    for &(what, path) in &named {
        refuse_existing(path, what)?;
    }
    let identity = load_identity(key)?;
    let opened = outputs.log.open()?;

    let extensions = match claims {
        Some(claims_path) => {
            let claims_bytes = read_file(claims_path, "claims file")?;
            // Validate — but embed the original bytes verbatim, so the signed
            // extension is exactly what the operator reviewed on disk.
            serde_json::from_slice::<serde_json::Value>(&claims_bytes).map_err(|source| {
                CliError::ClaimsJsonParse {
                    path: claims_path.to_path_buf(),
                    source,
                }
            })?;
            vec![encode_extension(&capability_claims_oid(), claims_bytes)]
        }
        None => Vec::new(),
    };

    let authority = CertificateAuthority::new(identity);
    // Read, or built and staged, before anything is signed, so a stored
    // issuer certificate that is not this key's stops the run with nothing
    // written or appended.
    let stored = outputs
        .issuer_certificate
        .map(|_| stored_issuer_certificate(key, &authority))
        .transpose()?;

    let issued = if let Some(path) = request_path {
        let pem_bytes = read_file(path, "certificate-signing request file")?;
        let request_der = pem::decode_certificate_request(&pem_bytes, path)?;
        let certified =
            authority.issue_certificate_for_request(&request_der, subject, ttl, extensions)?;
        Issuance {
            der_bytes: certified.der_bytes,
            subject_public_key: certified.subject_public_key,
            issuer_public_key: certified.issuer_public_key,
            fingerprint: certified.fingerprint,
            expires_at: certified.expires_at,
            subject_key_origin: "presented by the holder, proof of possession verified",
        }
    } else {
        // `generated` carries the freshly generated subject signing key; it is
        // deliberately never persisted or printed and drops here.
        let generated = authority.issue_certificate(subject, ttl, extensions)?;
        Issuance {
            der_bytes: generated.der_bytes,
            subject_public_key: generated.subject_verifying_key.to_bytes(),
            issuer_public_key: generated.issuer_public_key,
            fingerprint: generated.fingerprint,
            expires_at: generated.expires_at,
            subject_key_origin: "generated by the issuer and discarded — the holder never proved \
                                 possession",
        }
    };

    let mut staged = vec![StagedFile::stage(
        outputs.certificate,
        pem::encode_certificate(&issued.der_bytes).as_bytes(),
        "certificate file",
    )?];
    if let (Some(path), Some(stored)) = (outputs.issuer_certificate, stored) {
        staged.extend(stored.built);
        staged.push(StagedFile::stage(
            path,
            &stored.pem,
            "issuer certificate file",
        )?);
    }
    let entered = opened.enter(&issued.der_bytes, staged)?;

    let mut emit = Emitter::new(json);
    emit.field("issued certificate for subject", "subject", subject);
    emit.field(
        "subject public key (ed25519)",
        "subject_public_key",
        hex_lower(&issued.subject_public_key),
    );
    emit.field(
        "subject key origin",
        "subject_key_origin",
        issued.subject_key_origin,
    );
    emit.field(
        "issuer public key (ed25519)",
        "issuer_public_key",
        hex_lower(&issued.issuer_public_key),
    );
    emit.field(
        "fingerprint (sha256)",
        "fingerprint",
        hex_lower(&issued.fingerprint),
    );
    emit.field(
        "expires at (rfc3339)",
        "expires_at",
        issued.expires_at.to_rfc3339(),
    );
    match claims {
        Some(claims_path) => emit.field(
            "capability claims embedded from",
            "capability_claims_path",
            claims_path.display().to_string(),
        ),
        None => emit.field("capability claims", "capability_claims", "none"),
    }
    emit.field(
        "certificate written",
        "certificate_path",
        outputs.certificate.display().to_string(),
    );
    if let Some(path) = outputs.issuer_certificate {
        emit.field(
            "issuer certificate written",
            "issuer_certificate_path",
            path.display().to_string(),
        );
    }
    entered.report(&mut emit);
    emit.finish();
    Ok(())
}

/// `lys ca verify --cert <file> --issuer-public-key <hex> [--at <rfc3339>]`.
///
/// # Errors
///
/// Returns [`CliError::Io`] if the certificate file cannot be read,
/// [`CliError::PemParse`] if it is not a PEM `CERTIFICATE` block,
/// [`CliError::InvalidIssuerPublicKey`] / [`CliError::InvalidTimestamp`] for
/// malformed arguments, [`CliError::CertificateVerificationFailed`] — the
/// single non-oracle message — if any verification check rejects the
/// certificate, and [`CliError::Trust`] if the DER cannot be parsed as a
/// certificate at all.
pub fn verify(cert: &Path, issuer_public_key: &str, at: Option<&str>, json: bool) -> CliResult<()> {
    let pem_bytes = read_file(cert, "certificate file")?;
    let der = pem::decode_certificate(&pem_bytes, cert)?;
    let issuer = parse_hex_32(issuer_public_key).ok_or(CliError::InvalidIssuerPublicKey)?;
    let checked_at = match at {
        Some(value) => DateTime::parse_from_rfc3339(value)
            .map(|instant| instant.with_timezone(&Utc))
            .map_err(|source| CliError::InvalidTimestamp {
                value: value.to_string(),
                source,
            })?,
        None => Utc::now(),
    };

    match verify_certificate_chain_at(&der, &issuer, checked_at) {
        Ok(()) => {}
        // Non-oracle by design: every rejected check — signature, issuer
        // key, self-signature screen, validity window — surfaces as the one
        // indistinguishable message.
        Err(TrustError::CertificateVerification { .. }) => {
            return Err(CliError::CertificateVerificationFailed);
        }
        Err(other) => return Err(CliError::Trust(other)),
    }

    // Read claims only after verification succeeded, so nothing from an
    // unverified certificate is ever echoed.
    let claims = decode_extension(&der, &capability_claims_oid())?;

    let mut emit = Emitter::new(json);
    emit.flag("certificate verified", "verified");
    emit.field(
        "issuer public key (ed25519)",
        "issuer_public_key",
        hex_lower(&issuer),
    );
    emit.field(
        "checked at (rfc3339)",
        "checked_at",
        checked_at.to_rfc3339(),
    );
    match claims {
        Some(bytes) => match String::from_utf8(bytes) {
            Ok(text) if is_terminal_safe(&text) => {
                emit.field("capability claims", "capability_claims", text);
            }
            // Non-UTF-8 or control characters (terminal escape injection):
            // echo the bytes as hex, never raw. The JSON key differs too, so
            // a consumer can tell it received hex rather than the claims
            // text and cannot mistake one encoding for the other.
            Ok(unsafe_text) => emit.field(
                "capability claims (hex)",
                "capability_claims_hex",
                hex_lower(unsafe_text.as_bytes()),
            ),
            Err(non_utf8) => emit.field(
                "capability claims (hex)",
                "capability_claims_hex",
                hex_lower(non_utf8.as_bytes()),
            ),
        },
        None => emit.field("capability claims", "capability_claims", "none"),
    }
    emit.finish();
    Ok(())
}

#[cfg(test)]
#[path = "ca_tests.rs"]
mod tests;
