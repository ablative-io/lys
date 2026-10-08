//! The secrets broker's command line: make a broker, seal a credential from
//! standard input, grant and revoke its use, issue and drop handles, sign a
//! presentation as an agent, page through the audit log from its tail, audit
//! every line of it, and serve the proxy.

#![warn(clippy::await_holding_lock)]

mod app_client;
mod app_client_tests;
mod args;
mod callers;
mod cli;
mod files;
mod manage;
mod oauth_proxy;
mod redact;
#[cfg(test)]
mod redact_tests;
#[cfg(test)]
mod route_write_fault_tests;
mod router;
mod save_app;
mod save_app_tests;
mod serve;
#[cfg(test)]
mod serve_tests;
mod signing;
mod spice;
mod upgrade_window;
mod values;
#[cfg(test)]
mod values_tests;
mod view;
#[cfg(test)]
mod view_tests;
mod watched;

use std::io::{Read, Write};
use std::process::ExitCode;

use clap::{CommandFactory, FromArgMatches};
use lys_core::Ed25519Identity;
use lys_secrets::{
    Broker, EntryClass, HandleId, Holder, Presentation, Recipients, RecordedLine, Relation, Scope,
    Secret, SecretsError, UpstreamRevocation, new_operation_id, request_digest, to_hex,
};

use args::{Page, RecordClass};
use cli::{Cli, Command, Where};
use files::{FileGrants, Layout, Route, now_ms};
use spice::Grants;

fn read_credential() -> Result<Secret, SecretsError> {
    let mut value = Vec::new();
    std::io::stdin()
        .read_to_end(&mut value)
        .map_err(|source| SecretsError::Io {
            context: "reading the credential from standard input".to_owned(),
            source,
        })?;
    while value.last().is_some_and(u8::is_ascii_whitespace) {
        value.pop();
    }
    Ok(Secret::new(value))
}

fn open(at: &Where) -> Result<Broker<Grants>, SecretsError> {
    let layout = Layout::new(&at.root, &at.keys);
    open_with(at, Grants::File(FileGrants::new(layout.grants())))
}

fn open_with(at: &Where, grants: Grants) -> Result<Broker<Grants>, SecretsError> {
    let layout = Layout::new(&at.root, &at.keys);
    match upgrade_window::check(&at.root) {
        Some(ready) => Broker::open_deferred(&layout.paths(), grants, Box::new(now_ms), ready),
        None => Broker::open(&layout.paths(), grants, Box::new(now_ms)),
    }
}

fn print_line(recorded: RecordedLine) {
    let line = recorded.line;
    println!(
        "{:>3} {:<8} {:<32} {:<14} {:<16} {}",
        recorded.index,
        line.kind.label(),
        line.handle.unwrap_or_default(),
        line.identity.unwrap_or_default(),
        line.secret.unwrap_or_default(),
        line.outcome
    );
}

/// Prints one page of the audit log through its window, then the index the
/// next older page ends before, or that this page reached the first line.
/// It reads the page's lines and no others.
fn print_page(broker: &Broker<Grants>, page: &Page) -> Result<(), SecretsError> {
    let lines = broker.audit().window(page.before, page.most.get())?;
    // A page holds at least one line unless it ends at line 0.
    let from = lines.first().map_or(0, |first| first.index);
    for recorded in lines {
        print_line(recorded);
    }
    if from > 0 {
        println!("older: --before {from}");
    } else {
        println!("first line reached");
    }
    Ok(())
}

fn run(command: Command) -> Result<(), SecretsError> {
    match command {
        Command::Init(at) => {
            let layout = Layout::new(&at.root, &at.keys);
            layout.prepare()?;
            Broker::create(
                &layout.paths(),
                Grants::File(FileGrants::new(layout.grants())),
                Box::new(now_ms),
            )?;
            println!(
                "broker made in {}; keys in {}",
                at.root.display(),
                at.keys.display()
            );
        }
        Command::Seal {
            at,
            name,
            owner,
            upstream,
            header,
            prefix,
            spend_header,
        } => {
            let value = read_credential()?;
            let layout = Layout::new(&at.root, &at.keys);
            let mut broker = open(&at)?;
            broker.seal(&name, &owner, &value)?;
            layout.add_route(
                &name,
                Route {
                    upstream,
                    header,
                    prefix,
                    spend_header,
                },
            )?;
            println!("sealed {name} ({} bytes, not shown)", value.len());
        }
        Command::Grant {
            root,
            identity,
            secret,
            by,
            relation,
        } => {
            let relation = Relation::from(relation);
            FileGrants::new(Layout::grants_in(&root)).set(
                relation,
                &identity,
                &secret,
                Some(&by),
            )?;
            println!(
                "{identity} holds {} on {secret}, granted by {by}",
                relation.label()
            );
        }
        Command::Revoke {
            root,
            identity,
            secret,
            relation,
        } => {
            let relation = Relation::from(relation);
            FileGrants::new(Layout::grants_in(&root)).remove(relation, &identity, &secret)?;
            println!(
                "{identity} no longer holds {} on {secret}",
                relation.label()
            );
        }
        Command::Issue {
            at,
            identity,
            holder_key,
            secret,
            uses,
            minutes,
            spend_cap,
            directory,
        } => {
            let key = Ed25519Identity::load(&holder_key)?;
            let holder = Holder {
                identity,
                key: key.public_key_bytes(),
            };
            let layout = Layout::new(&at.root, &at.keys);
            let mut broker = open_with(&at, directory.grants(&layout)?)?;
            let issued = broker.issue_capped(
                &holder,
                &secret,
                uses,
                now_ms().saturating_add(minutes.saturating_mul(60_000)),
                spend_cap,
            )?;
            println!("handle_id {}", issued.id);
            println!("handle {}", to_hex(issued.token.expose()));
        }
        Command::AddAccount {
            at,
            secret,
            account,
        } => {
            let value = read_credential()?;
            let mut broker = open(&at)?;
            broker.add_account(&secret, &account, &value)?;
            println!(
                "sealed account {account} of {secret} ({} bytes, not shown)",
                value.len()
            );
        }
        Command::NextAccount { at, secret } => {
            let mut broker = open(&at)?;
            let account = broker.next_account(&secret)?;
            println!("{secret} now uses account {account}");
        }
        Command::RestoreAccount {
            at,
            secret,
            account,
        } => {
            let mut broker = open(&at)?;
            broker.restore_account(&secret, &account)?;
            println!("account {account} of {secret} is back in service");
        }
        Command::Accounts { at, secret } => {
            let broker = open(&at)?;
            for view in broker.store().accounts(&secret) {
                let state = if view.resting {
                    "resting"
                } else if view.current {
                    "in use"
                } else {
                    "ready"
                };
                println!("{:<16} {state}", view.account);
            }
        }
        Command::Scope {
            at,
            secret,
            by,
            scope,
        } => {
            let scope = Scope::parse(&scope)?;
            let target = scope.target();
            open(&at)?.set_scope(&by, &secret, scope)?;
            println!("{secret} is now in the scope {target}");
        }
        Command::Recipients {
            at,
            secret,
            by,
            people_only,
        } => {
            let policy = if people_only {
                Recipients::PeopleOnly
            } else {
                Recipients::Anyone
            };
            open(&at)?.set_recipients(&by, &secret, policy)?;
            println!("{secret} may now be handed to {}", policy.label());
        }
        Command::TrustService {
            at,
            name,
            public_key,
        } => {
            let key = lys_secrets::from_hex(&public_key)
                .filter(|bytes| bytes.len() == 32)
                .ok_or_else(|| SecretsError::Encoding {
                    context: "service public key",
                    reason: "not 32 bytes in hex".to_owned(),
                })?;
            let layout = Layout::new(&at.root, &at.keys);
            layout.trust_service(lys_secrets::ServiceKey {
                name: name.clone(),
                public_key: to_hex(&key),
            })?;
            println!("{name} may now ask on a signed-in person's behalf");
        }
        Command::ServiceKey { key } => {
            let identity = Ed25519Identity::load_or_generate(&key)?;
            println!("{}", to_hex(&identity.public_key_bytes()));
        }
        Command::List { at, identity } => {
            for entry in open(&at)?.listing(&identity) {
                println!(
                    "{:<24} {:?} owned by {}",
                    entry.name, entry.class, entry.owner
                );
            }
        }
        Command::Revocation { at, handle_id } => {
            let state = open(&at)?.revocation_state(&HandleId::from_text(&handle_id))?;
            let here = if state.stopped_here {
                "stopped"
            } else {
                "still admitted"
            };
            println!("use here: {here}");
            match state.upstream {
                UpstreamRevocation::NotAsked => println!("provider: not asked"),
                UpstreamRevocation::Unconfirmed(reason) => {
                    println!("provider: unconfirmed ({reason})");
                }
                UpstreamRevocation::Confirmed => println!("provider: confirmed"),
            }
        }
        Command::ConfirmRevocation {
            at,
            handle_id,
            provider_subject,
        } => {
            let id = HandleId::from_text(&handle_id);
            let changed = open(&at)?.confirm_upstream_revocation(&id, &provider_subject)?;
            let said = if changed {
                "now confirmed"
            } else {
                "was already confirmed"
            };
            println!("provider revocation of {handle_id} {said}");
        }
        Command::Drop {
            at,
            handle_id,
            revoke_upstream,
        } => {
            let mut broker = open(&at)?;
            let id = HandleId::from_text(&handle_id);
            let outcome = broker.drop_handle(&id)?;
            println!("{handle_id}: {outcome:?}");
            if revoke_upstream {
                let confirmed = oauth_proxy::revoke_after_drop(&mut broker, &id)?;
                let said = if confirmed {
                    "confirmed"
                } else {
                    "not confirmed"
                };
                println!("upstream revocation {said}");
            }
        }
        Command::SealOauth {
            at,
            name,
            owner,
            upstream,
        } => {
            let layout = Layout::new(&at.root, &at.keys);
            let subject = oauth_proxy::seal(
                &mut open(&at)?,
                &layout,
                &name,
                &owner,
                upstream,
                &read_credential()?,
            )?;
            println!("sealed OAuth grant {name} for {subject} (tokens not shown)");
        }
        Command::SpawnLogin {
            at,
            seat,
            secret,
            directory,
        } => {
            let layout = Layout::new(&at.root, &at.keys);
            let mut broker = open_with(&at, directory.grants(&layout)?)?;
            let (account, login) = broker.spawn_login(&seat, &secret)?;
            std::io::stdout()
                .write_all(login.expose())
                .map_err(|source| SecretsError::Io {
                    context: "writing the login to standard output".to_owned(),
                    source,
                })?;
            eprintln!("{seat} took the login of {secret}@{account}");
        }
        Command::Sign {
            key,
            handle_id,
            method,
            path,
            body,
        } => {
            let key = Ed25519Identity::load(&key)?;
            let body = match body {
                Some(file) => std::fs::read(&file).map_err(|error| SecretsError::Io {
                    context: format!("reading {}", file.display()),
                    source: error,
                })?,
                None => Vec::new(),
            };
            let presentation = Presentation::sign(
                &HandleId::from_text(&handle_id),
                &new_operation_id()?,
                now_ms(),
                request_digest(&method, &path, &body)?,
                &key,
            )?;
            let [id, operation, signed_at, signature] = presentation.to_wire();
            println!("lys-handle-id: {id}");
            println!("lys-operation: {operation}");
            println!("lys-signed-at: {signed_at}");
            println!("lys-presentation: {signature}");
        }
        Command::Log { at, page } => print_page(&open(&at)?, &page)?,
        Command::Audit(at) => {
            let broker = open(&at)?;
            let checked = broker.audit().audit_every_line()?.len();
            println!(
                "audit sound: every one of {checked} lines is signed by {}",
                to_hex(&broker.audit().verifying_key())
            );
        }
        Command::Serve {
            at,
            listen,
            directory,
        } => {
            let layout = Layout::new(&at.root, &at.keys);
            let broker = open_with(&at, directory.grants(&layout)?)?;
            serve::serve(broker, layout, &listen)?;
        }
        Command::SealRecord {
            at,
            name,
            owner,
            class,
        } => {
            let value = read_credential()?;
            let class = match class {
                RecordClass::Memory => EntryClass::Memory,
                RecordClass::Key => EntryClass::Key,
            };
            open(&at)?.seal_record(&name, class, &owner, &value)?;
            println!("sealed {name} ({} bytes, not shown)", value.len());
        }
        Command::ReadRecord {
            at,
            identity,
            name,
            from,
            len,
            directory,
        } => {
            let layout = Layout::new(&at.root, &at.keys);
            let mut broker = open_with(&at, directory.grants(&layout)?)?;
            let piece = len.map(|len| from..from.saturating_add(len));
            let piece = piece.or((from > 0).then_some(from..usize::MAX));
            let read = broker.read_record(&identity, &name, piece)?;
            std::io::stdout()
                .write_all(read.expose())
                .map_err(|source| SecretsError::Io {
                    context: "writing the record to standard output".to_owned(),
                    source,
                })?;
        }
    }
    Ok(())
}

/// What `--version` prints after the name: the crate version and the commit
/// the binary was built from, stamped by `build.rs`.
const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), " (", env!("LYS_BUILD"), ")");

/// The arguments, with `--version` answering [`VERSION`].
fn parse() -> Cli {
    let matches = Cli::command().version(VERSION).get_matches();
    Cli::from_arg_matches(&matches).unwrap_or_else(|error| error.exit())
}

fn main() -> ExitCode {
    match run(parse().command) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
