//! `lys` — command-line surface for the lys trust primitives.
//!
//! This binary is a thin surface over [`lys_core`]: it parses arguments,
//! dispatches to the subcommand implementations in [`commands`], and maps
//! their results to process exit codes. All logic lives in the library and
//! the per-subcommand modules — this file stays parse-and-dispatch only.
//!
//! Exit codes: `0` on success, `1` on any operational or verification
//! failure (with a diagnostic on stderr), `2` for argument-parsing errors
//! (clap's convention).

mod cli;
mod commands;
mod identity;

use std::process::ExitCode;

use clap::{CommandFactory, FromArgMatches};

use crate::cli::{
    CaCommand, Cli, Command, InspectCommand, KeyCommand, LogCommand, LogProveCommand,
    LogVerifyCommand,
};

/// What `--version` prints after the name: the crate version and the commit
/// the binary was built from, stamped by `build.rs`.
const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), " (", env!("LYS_BUILD"), ")");

/// The arguments, with `--version` answering [`VERSION`].
fn parse() -> Cli {
    let matches = Cli::command().version(VERSION).get_matches();
    Cli::from_arg_matches(&matches).unwrap_or_else(|error| error.exit())
}

/// Entry point: parse arguments, dispatch, and translate the outcome into an
/// exit code. Every failure path prints a diagnostic to stderr.
fn main() -> ExitCode {
    let cli = parse();
    let json = cli.json;
    let result = match cli.command {
        Command::Key(key_command) => match key_command {
            KeyCommand::Generate { out } => commands::key::generate(&out, json),
            KeyCommand::Inspect {
                key,
                note_name,
                ssh,
                allowed_signers,
            } => commands::key::inspect(
                &key,
                note_name.as_deref(),
                ssh,
                allowed_signers.as_deref(),
                json,
            ),
        },
        Command::Identity(identity_command) => match identity_command {
            identity::IdentityCommand::Import {
                file,
                root,
                credential_file,
                address,
            } => identity::import::run(&file, root, credential_file, address.as_deref(), json),
            identity::IdentityCommand::Prepare { config } => identity::prepare::run(&config, json),
            identity::IdentityCommand::Configure { config } => {
                identity::configure::run(&config, json)
            }
            identity::IdentityCommand::Health { config } => identity::health::run(&config, json),
            identity::IdentityCommand::SetupCode { root } => {
                identity::install::setup_code::run(root, json)
            }
            identity::IdentityCommand::Install {
                service_port,
                broker_port,
                root,
                admin_email,
                surface,
                message_service,
            } => identity::install::run(
                &identity::install::Options {
                    root,
                    admin_email,
                    surface,
                    message_service,
                    service_port,
                    broker_port,
                },
                json,
            ),
            identity::IdentityCommand::Upgrade {
                from,
                surface,
                root,
                message_service,
            } => identity::upgrade::run(
                &identity::upgrade::Options {
                    from,
                    surface,
                    root,
                    message_service,
                },
                json,
            ),
        }
        .map_err(commands::error::CliError::from),
        Command::Log(log_command) => match log_command {
            LogCommand::Init { dir, origin } => commands::log::init::run(&dir, &origin, json),
            LogCommand::Status { dir } => commands::log::status::run(&dir, json),
            LogCommand::Append { dir, leaf } => commands::log::append::run(&dir, &leaf, json),
            LogCommand::Checkpoint { dir, key, out } => {
                commands::log::checkpoint::run(&dir, &key, &out, json)
            }
            LogCommand::Prove(prove_command) => match prove_command {
                LogProveCommand::Inclusion {
                    dir,
                    key,
                    leaf_index,
                    out,
                } => commands::log::prove::inclusion(&dir, &key, leaf_index, &out, json),
                LogProveCommand::Consistency {
                    dir,
                    key,
                    old_size,
                    out,
                } => commands::log::prove::consistency(&dir, &key, old_size, &out, json),
            },
            LogCommand::Verify(verify_command) => match verify_command {
                LogVerifyCommand::Inclusion {
                    artifact,
                    leaf,
                    verifier_key,
                } => commands::log::verify::inclusion(&artifact, &leaf, &verifier_key, json),
                LogVerifyCommand::Consistency {
                    artifact,
                    verifier_key,
                } => commands::log::verify::consistency(&artifact, &verifier_key, json),
            },
        },
        Command::Ca(ca_command) => match ca_command {
            CaCommand::Request { key, subject, out } => {
                commands::ca::request(&key, &subject, &out, json)
            }
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
            } => commands::duration::validity_window(validity_days, validity.as_deref()).and_then(
                |ttl| {
                    let entry = commands::ca_log::LogEntry::from_flags(
                        &log,
                        &leaf_out,
                        log_key.as_deref(),
                        artifact_out.as_deref(),
                    )?;
                    let outputs = commands::ca::IssueOutputs {
                        certificate: &out,
                        issuer_certificate: issuer_out.as_deref(),
                        log: entry,
                    };
                    commands::ca::issue(
                        &key,
                        &subject,
                        claims.as_deref(),
                        ttl,
                        &outputs,
                        request.as_deref(),
                        json,
                    )
                },
            ),
            CaCommand::IssuerCert { key, out } => commands::ca::issuer_cert(&key, &out, json),
            CaCommand::Verify {
                cert,
                issuer_public_key,
                at,
            } => commands::ca::verify(&cert, &issuer_public_key, at.as_deref(), json),
        },
        Command::Runner(runner_command) => commands::runner::run(runner_command),
        Command::Attest { key, payload, out } => commands::attest::run(&key, &payload, &out, json),
        Command::Verify {
            attestation,
            payload,
            cert,
            issuer_public_key,
            at,
        } => commands::verify::run(
            &attestation,
            &payload,
            cert.as_deref(),
            issuer_public_key.as_deref(),
            at.as_deref(),
            json,
        ),
        Command::Inspect(inspect_command) => match inspect_command {
            InspectCommand::Attestation { attestation } => {
                commands::inspect::attestation(&attestation, json)
            }
            InspectCommand::Cert { cert } => commands::inspect::cert(&cert, json),
        },
        Command::Seal {
            key,
            recipient_public_key,
            payload,
            out,
            attestation_out,
        } => commands::seal::seal(
            &key,
            &recipient_public_key,
            &payload,
            &out,
            &attestation_out,
            json,
        ),
        Command::Open {
            key,
            sender_public_key,
            envelope,
            attestation,
            out,
        } => commands::seal::open(
            &key,
            &sender_public_key,
            &envelope,
            &attestation,
            &out,
            json,
        ),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            // The diagnostic always goes to stderr, so an operator watching a
            // terminal sees it in the usual place. Under `--json` the failure
            // is ALSO emitted as an object on stdout: a caller that asked for
            // parseable output must not receive unparseable output at exactly
            // the moment it matters most. The message is the CLI's existing
            // text, already collapsed to non-specific wording for
            // verification failures — JSON mode reformats, it never widens.
            eprintln!("error: {error}");
            if json {
                commands::output::emit_json_error(&error.to_string());
            }
            ExitCode::FAILURE
        }
    }
}
