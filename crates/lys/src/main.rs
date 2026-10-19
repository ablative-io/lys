// The install renders one json! document of the whole server configuration;
// its macro recursion passes the default limit of 128.
#![recursion_limit = "256"]
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

use lys::{cli, commands, identity};

use std::process::ExitCode;

use clap::{CommandFactory, FromArgMatches};

use crate::cli::{
    Cli, Command, InspectCommand, KeyCommand, LogCommand, LogProveCommand, LogVerifyCommand,
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
    commands::started::mark();
    let cli = parse();
    let json = cli.json;
    let result = match cli.command {
        Command::SignIn(args) => commands::sign_in::run(&args, json),
        Command::Claude(args) => commands::launch::run("claude", &args, json),
        Command::Codex(args) => commands::launch::run("codex", &args, json),
        Command::Run(args) => commands::launch::run(
            &args.command,
            &cli::HarnessArgs {
                options: args.options,
                arguments: args.arguments,
            },
            json,
        ),
        Command::Agents(args) => commands::launch::agents(&args, json),
        Command::Key(key_command) => dispatch_key(key_command, json),
        Command::Identity(identity_command) => {
            dispatch_identity(identity_command, json).map_err(commands::error::CliError::from)
        }
        Command::Log(log_command) => dispatch_log(log_command, json),
        Command::Ca(ca_command) => {
            commands::ca_dispatch::run(ca_command, json, lys_core::clock::ClockSource::System)
        }
        Command::Runner(runner_command) => commands::runner::run(runner_command),
        Command::Proxy(proxy_command) => commands::proxy::run(proxy_command),
        Command::Variables(variables_args) => commands::variables::run(variables_args, json),
        Command::Mcp {
            url,
            agent,
            key_file,
        } => commands::mcp_bridge::run(&url, &agent, &key_file),
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
        Command::Seat(seat_args) => commands::seat::run(seat_args, json),
        Command::Attach(attach_args) => commands::attach::run(&attach_args, json),
        Command::Agent(agent_args) => commands::agent_pass::run(agent_args, json),
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

fn dispatch_identity(
    identity_command: identity::IdentityCommand,
    json: bool,
) -> identity::error::IdentityResult<()> {
    match identity_command {
        identity::IdentityCommand::Import {
            file,
            root,
            credential_file,
            address,
        } => identity::import::run(&file, root, credential_file, address.as_deref(), json),
        identity::IdentityCommand::Prepare { config } => identity::prepare::run(&config, json),
        identity::IdentityCommand::Configure { config } => identity::configure::run(&config, json),
        identity::IdentityCommand::Health { config } => identity::health::run(&config, json),
        identity::IdentityCommand::SetupCode { root } => {
            identity::install::setup_code::run(root, json)
        }
        identity::IdentityCommand::Stop { reason, kill, root } => {
            identity::stop::run(&identity::stop::StopOptions { root, reason, kill }, json)
        }
        identity::IdentityCommand::Start { root } => {
            identity::stop::start(&identity::stop::StartOptions { root }, json)
        }
        identity::IdentityCommand::AcceptLoss { store, root } => {
            identity::accept_loss::run(root, &store, json)
        }
        identity::IdentityCommand::Install {
            from,
            clients,
            service_port,
            broker_port,
            proxy_port,
            root,
            admin_email,
            surface,
            message_service,
            profile,
        } => identity::install::run(
            &identity::install::Options {
                from,
                clients,
                root,
                profile,
                admin_email,
                surface,
                message_service,
                service_port,
                broker_port,
                proxy_port,
            },
            json,
        ),
        identity::IdentityCommand::Upgrade {
            clients,
            from,
            surface,
            root,
            message_service,
        } => identity::upgrade::run(
            &identity::upgrade::Options {
                clients,
                from,
                surface,
                root,
                message_service,
            },
            json,
        ),
    }
}

fn dispatch_log(log_command: LogCommand, json: bool) -> commands::error::CliResult<()> {
    match log_command {
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
    }
}

fn dispatch_key(key_command: KeyCommand, json: bool) -> commands::error::CliResult<()> {
    match key_command {
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
    }
}
