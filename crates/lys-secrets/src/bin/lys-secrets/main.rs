//! The secrets broker's command line: make a broker, seal a credential from
//! standard input, grant and revoke its use, issue and drop handles, sign a
//! presentation as an agent, read the audit log, and serve the proxy.

mod files;
mod serve;
mod spice;
mod view;

use std::io::Read;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use lys_core::Ed25519Identity;
use lys_secrets::{
    Broker, HandleId, Holder, Presentation, Secret, SecretsError, new_operation_id, to_hex,
};

use files::{FileGrants, Layout, Route, now_ms};
use spice::{Grants, SpiceGrants};

#[derive(Parser)]
#[command(name = "lys-secrets", about = "The lys secrets broker")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Args)]
struct Where {
    /// The broker's folder: the sealed store, the audit log, routes and grants.
    #[arg(long)]
    root: PathBuf,
    /// The key folder, outside the broker's folder.
    #[arg(long)]
    keys: PathBuf,
}

#[derive(Subcommand)]
enum Command {
    /// Make a new broker: store, audit log and both keys.
    Init(Where),
    /// Seal the credential read from standard input, bound to one upstream.
    Seal {
        #[command(flatten)]
        at: Where,
        #[arg(long)]
        name: String,
        #[arg(long)]
        owner: String,
        /// The only origin the credential is ever sent to.
        #[arg(long)]
        upstream: String,
        /// The request header that carries it.
        #[arg(long, default_value = "authorization")]
        header: String,
        /// Text written before the credential in that header.
        #[arg(long, default_value = "Bearer ")]
        prefix: String,
    },
    /// Grant an identity the use of a secret, in a person's name.
    Grant {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        identity: String,
        #[arg(long)]
        secret: String,
        #[arg(long)]
        by: String,
    },
    /// Revoke an identity's use of a secret; the next use is refused.
    Revoke {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        identity: String,
        #[arg(long)]
        secret: String,
    },
    /// Issue a handle to an identity whose key file is named.
    Issue {
        #[command(flatten)]
        at: Where,
        #[arg(long)]
        identity: String,
        /// The holder's key file; only its public half is registered.
        #[arg(long)]
        holder_key: PathBuf,
        #[arg(long)]
        secret: String,
        #[arg(long, default_value_t = 10)]
        uses: u64,
        #[arg(long, default_value_t = 60)]
        minutes: i64,
    },
    /// Drop a handle.
    Drop {
        #[command(flatten)]
        at: Where,
        #[arg(long)]
        handle_id: String,
    },
    /// As the holder: sign a presentation for one call and print its headers.
    Sign {
        #[arg(long)]
        key: PathBuf,
        #[arg(long)]
        handle_id: String,
    },
    /// Print the audit log, every line's signature checked.
    Log(Where),
    /// Serve the proxy on a local address.
    Serve {
        #[command(flatten)]
        at: Where,
        #[arg(long, default_value = "127.0.0.1:8472")]
        listen: String,
        /// The Lys directory's configuration; when named, every use is
        /// judged by the directory's grants on `secret/<name>`.
        #[arg(long)]
        directory_config: Option<PathBuf>,
        /// The action the directory must give on the secret for a use.
        #[arg(long, default_value = "edit")]
        use_action: String,
    },
}

fn open(at: &Where) -> Result<Broker<Grants>, SecretsError> {
    let layout = Layout::new(&at.root, &at.keys);
    open_with(at, Grants::File(FileGrants::new(layout.grants())))
}

fn open_with(at: &Where, grants: Grants) -> Result<Broker<Grants>, SecretsError> {
    let layout = Layout::new(&at.root, &at.keys);
    Broker::open(&layout.paths(), grants, Box::new(now_ms))
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
        } => {
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
            let value = Secret::new(value);
            let layout = Layout::new(&at.root, &at.keys);
            let mut broker = open(&at)?;
            broker.seal(&name, &owner, &value)?;
            layout.add_route(
                &name,
                Route {
                    upstream,
                    header,
                    prefix,
                },
            )?;
            println!("sealed {name} ({} bytes, not shown)", value.len());
        }
        Command::Grant {
            root,
            identity,
            secret,
            by,
        } => {
            FileGrants::new(Layout::grants_in(&root)).set(&identity, &secret, Some(&by))?;
            println!("{identity} may use {secret}, granted by {by}");
        }
        Command::Revoke {
            root,
            identity,
            secret,
        } => {
            FileGrants::new(Layout::grants_in(&root)).remove(&identity, &secret)?;
            println!("{identity} may no longer use {secret}");
        }
        Command::Issue {
            at,
            identity,
            holder_key,
            secret,
            uses,
            minutes,
        } => {
            let key = Ed25519Identity::load(&holder_key)?;
            let holder = Holder {
                identity,
                key: key.public_key_bytes(),
            };
            let mut broker = open(&at)?;
            let issued = broker.issue(
                &holder,
                &secret,
                uses,
                now_ms().saturating_add(minutes.saturating_mul(60_000)),
            )?;
            println!("handle_id {}", issued.id);
            println!("handle {}", to_hex(issued.token.expose()));
        }
        Command::Drop { at, handle_id } => {
            let mut broker = open(&at)?;
            let outcome = broker.drop_handle(&HandleId::from_text(&handle_id))?;
            println!("{handle_id}: {outcome:?}");
        }
        Command::Sign { key, handle_id } => {
            let key = Ed25519Identity::load(&key)?;
            let presentation = Presentation::sign(
                &HandleId::from_text(&handle_id),
                &new_operation_id()?,
                now_ms(),
                &key,
            )?;
            let [id, operation, signed_at, signature] = presentation.to_wire();
            println!("lys-handle-id: {id}");
            println!("lys-operation: {operation}");
            println!("lys-signed-at: {signed_at}");
            println!("lys-presentation: {signature}");
        }
        Command::Log(at) => {
            let broker = open(&at)?;
            for recorded in broker.audit().replay()? {
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
        }
        Command::Serve {
            at,
            listen,
            directory_config,
            use_action,
        } => {
            let layout = Layout::new(&at.root, &at.keys);
            let grants = match directory_config {
                Some(config) => {
                    Grants::Directory(SpiceGrants::from_directory(&config, &use_action)?)
                }
                None => Grants::File(FileGrants::new(layout.grants())),
            };
            let broker = open_with(&at, grants)?;
            serve::serve(broker, layout, &listen)?;
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    match run(Cli::parse().command) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
