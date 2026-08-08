//! Turning `--admit` into the policy object the library insists on being given.
//!
//! # This module builds policies; it does not implement one
//!
//! Every value it can produce is a type `lys-anchor` exports —
//! [`AcceptAll`], [`MaxSize`], [`RecognisedCertificate`]. There is no enum here
//! implementing [`AdmissionPolicy`] by dispatching to those three, and that
//! omission is the point of the [`AnchorTask`] trait below. Such an enum would
//! be a fourth admission policy, defined in a binary, that no library consumer
//! could ever obtain — and this repository's rule is that logic lives in the
//! library and the binary parses and formats. A rule implemented in the binary
//! is a rule nobody else gets.
//!
//! So the runtime choice is made once, here, by matching and calling a
//! *generic* function three times. Each command supplies its own [`AnchorTask`]
//! and is monomorphised over the concrete policy, exactly as a library consumer
//! writing the same three arms by hand would be.
//!
//! # An argument the chosen policy does not read is refused, not ignored
//!
//! `--admit accept-all --max-bytes 4096` is rejected. An operator who writes it
//! believes they have configured a limit, and an anchor that shrugged and
//! admitted everything would be running an admission rule its operator did not
//! choose — which is precisely what DP23 spent a type parameter preventing.

use std::collections::BTreeSet;

use lys_anchor::{AcceptAll, AdmissionPolicy, MaxSize, RecognisedCertificate};

use crate::cli::{AdmissionArgs, AdmitPolicy};
use crate::commands::error::{CliError, CliResult};
use crate::commands::hex::parse_hex_32;

/// One command's work, expressed so it can be monomorphised over whichever
/// concrete policy the operator named.
///
/// A closure cannot be generic over a type parameter, so the work is a trait
/// with a generic method instead. Implementors hold their own arguments and
/// receive the policy by value, the same way `Anchor::create` and
/// `Anchor::open` take it.
pub trait AnchorTask {
    /// Runs the command under `policy`.
    ///
    /// # Errors
    ///
    /// Whatever the command itself fails with.
    fn run<P: AdmissionPolicy>(self, policy: P) -> CliResult<()>;
}

/// Builds the policy named by `args` and hands it to `task`.
///
/// # Errors
///
/// [`CliError::AdmissionArgumentMissing`] if a flag the named policy requires
/// is absent, [`CliError::AdmissionArgumentIgnored`] if a flag it does not read
/// was supplied anyway, and [`CliError::InvalidIssuerPublicKey`] /
/// [`CliError::InvalidSubjectKey`] for a key that is not 64 hexadecimal
/// characters. Otherwise whatever `task` returns.
pub fn with_policy<T: AnchorTask>(args: &AdmissionArgs, task: T) -> CliResult<()> {
    refuse_unread_arguments(args)?;
    match args.admit {
        AdmitPolicy::AcceptAll => task.run(AcceptAll),
        AdmitPolicy::MaxSize => task.run(MaxSize::new(max_bytes(args)?)),
        AdmitPolicy::RecognisedCertificate => task.run(recognised_certificate(args)?),
    }
}

/// Refuses any policy-specific flag the chosen policy does not read.
///
/// Written as a table of `(policy that reads it, flag, was it supplied)` rather
/// than as a match per policy, so adding a flag means adding one row and cannot
/// mean forgetting an arm.
fn refuse_unread_arguments(args: &AdmissionArgs) -> CliResult<()> {
    let supplied: [(AdmitPolicy, &'static str, bool); 3] = [
        (
            AdmitPolicy::MaxSize,
            "--max-bytes",
            args.max_bytes.is_some(),
        ),
        (
            AdmitPolicy::RecognisedCertificate,
            "--issuer-public-key",
            args.issuer_public_key.is_some(),
        ),
        (
            AdmitPolicy::RecognisedCertificate,
            "--subject-key",
            !args.subject_key.is_empty(),
        ),
    ];
    for (reader, flag, was_supplied) in supplied {
        if was_supplied && reader != args.admit {
            return Err(CliError::AdmissionArgumentIgnored {
                policy: args.admit.as_str(),
                flag,
            });
        }
    }
    Ok(())
}

/// The limit for `--admit max-size`.
fn max_bytes(args: &AdmissionArgs) -> CliResult<usize> {
    args.max_bytes.ok_or(CliError::AdmissionArgumentMissing {
        policy: AdmitPolicy::MaxSize.as_str(),
        flag: "--max-bytes",
    })
}

/// The policy for `--admit recognised-certificate`.
///
/// No `--subject-key` means an unrestricted subject, which is
/// `RecognisedCertificate::issued_by` and not `issued_by_to` with an empty set:
/// the library documents an empty set as admitting nothing, on purpose, and
/// mapping "the operator gave no allow-list" onto "the operator gave an empty
/// allow-list" would turn an unrestricted policy into a deny-all — or, read the
/// other way round, would be the accident that makes an emptied allow-list mean
/// everyone.
fn recognised_certificate(args: &AdmissionArgs) -> CliResult<RecognisedCertificate> {
    let issuer_hex =
        args.issuer_public_key
            .as_deref()
            .ok_or(CliError::AdmissionArgumentMissing {
                policy: AdmitPolicy::RecognisedCertificate.as_str(),
                flag: "--issuer-public-key",
            })?;
    let issuer = parse_hex_32(issuer_hex).ok_or(CliError::InvalidIssuerPublicKey)?;
    if args.subject_key.is_empty() {
        return Ok(RecognisedCertificate::issued_by(issuer));
    }
    let mut subjects = BTreeSet::new();
    for supplied in &args.subject_key {
        let key = parse_hex_32(supplied).ok_or_else(|| CliError::InvalidSubjectKey {
            value: supplied.clone(),
        })?;
        subjects.insert(key);
    }
    Ok(RecognisedCertificate::issued_by_to(issuer, subjects))
}

#[cfg(test)]
#[path = "policy_tests.rs"]
mod tests;
