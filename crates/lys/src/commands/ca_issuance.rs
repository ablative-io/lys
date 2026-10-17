//! Certificate issuance validates inputs and stages outputs before entering the log.

use std::path::Path;
use std::time::Duration;

use chrono::{DateTime, Utc};
use lys_core::ca::encode_extension;
use lys_core::clock::ClockSource;

use super::{
    STORED_ISSUER, authority_with_clock, capability_claims_oid, stored_issuer_certificate,
    stored_issuer_certificate_path,
};
use crate::commands::ca_log::LogEntry;
use crate::commands::error::{CliError, CliResult};
use crate::commands::files::{StagedFile, read_file, refuse_existing, refuse_shared_paths};
use crate::commands::hex::hex_lower;
use crate::commands::key::load_identity;
use crate::commands::output::Emitter;
use crate::commands::pem;

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

/// The unchanged issuance inputs, grouped for clock-supplied dispatch.
#[derive(Clone, Copy)]
pub struct IssueOptions<'a> {
    /// The issuer's identity key file.
    pub key: &'a Path,
    /// The subject this authority chooses to certify.
    pub subject: &'a str,
    /// Optional custom capability claims read from this file.
    pub claims: Option<&'a Path>,
    /// The certificate's requested validity duration.
    pub ttl: Duration,
    /// Optional proof-of-possession certificate-signing request file.
    pub request_path: Option<&'a Path>,
    /// Whether to emit the existing JSON result.
    pub json: bool,
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
    issue_with_clock(
        IssueOptions {
            key,
            subject,
            claims,
            ttl,
            request_path,
            json,
        },
        outputs,
        ClockSource::System,
    )
}

/// Issues through the existing file and log path with an owned clock.
///
/// # Errors
///
/// See [`issue`]. Clock failures propagate without publishing a certificate
/// or appending it to the log.
pub fn issue_with_clock(
    options: IssueOptions<'_>,
    outputs: &IssueOutputs<'_>,
    clock: ClockSource,
) -> CliResult<()> {
    let IssueOptions {
        key,
        subject,
        claims,
        ttl,
        request_path,
        json,
    } = options;
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

    let authority = authority_with_clock(identity, clock);
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

    let mut emit = issuance_report(&issued, subject, claims, outputs, json);
    entered.report(&mut emit);
    emit.finish();
    Ok(())
}

fn issuance_report(
    issued: &Issuance,
    subject: &str,
    claims: Option<&Path>,
    outputs: &IssueOutputs<'_>,
    json: bool,
) -> Emitter {
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
    emit
}
