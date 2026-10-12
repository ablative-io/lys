//! One CA dispatcher for the ordinary executable and the creation-time fixture.

use lys_core::clock::ClockSource;

use crate::cli::CaCommand;
use crate::commands::error::CliResult;
use crate::commands::{ca, ca_log, duration};

/// Dispatch actual CA arguments using the owner's creation clock.
///
/// # Errors
/// Propagates argument validation, trust, store and output failures.
pub fn run(command: CaCommand, json: bool, clock: ClockSource) -> CliResult<()> {
    match command {
        CaCommand::Request { key, subject, out } => ca::request(&key, &subject, &out, json),
        CaCommand::Issue {
            key,
            subject,
            request,
            claims,
            validity,
            validity_days,
            out,
            issuer_out,
            log,
            log_key,
            leaf_out,
            artifact_out,
        } => {
            let ttl = duration::validity_window(validity_days, validity.as_deref())?;
            let entry = ca_log::LogEntry::from_flags(
                &log,
                &leaf_out,
                log_key.as_deref(),
                artifact_out.as_deref(),
            )?;
            let outputs = ca::IssueOutputs {
                certificate: &out,
                issuer_certificate: issuer_out.as_deref(),
                log: entry,
            };
            match clock {
                ClockSource::System => ca::issue(
                    &key,
                    &subject,
                    claims.as_deref(),
                    ttl,
                    &outputs,
                    request.as_deref(),
                    json,
                ),
                supplied @ ClockSource::Supplied(_) => ca::issue_with_clock(
                    ca::IssueOptions {
                        key: &key,
                        subject: &subject,
                        claims: claims.as_deref(),
                        ttl,
                        request_path: request.as_deref(),
                        json,
                    },
                    &outputs,
                    supplied,
                ),
            }
        }
        CaCommand::IssuerCert { key, out } => match clock {
            ClockSource::System => ca::issuer_cert(&key, &out, json),
            supplied @ ClockSource::Supplied(_) => {
                ca::issuer_cert_with_clock(&key, &out, json, supplied)
            }
        },
        CaCommand::Verify {
            cert,
            issuer_public_key,
            at,
        } => ca::verify(&cert, &issuer_public_key, at.as_deref(), json),
    }
}
