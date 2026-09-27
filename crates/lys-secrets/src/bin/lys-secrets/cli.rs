//! The command line: every command and its arguments.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::args::{PermissionSource, RecordClass, RelationArg};

#[derive(Parser)]
#[command(name = "lys-secrets", about = "The lys secrets broker")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(clap::Args)]
pub struct Where {
    /// The broker's folder: the sealed store, the audit log, routes and grants.
    #[arg(long)]
    pub root: PathBuf,
    /// The key folder, outside the broker's folder.
    #[arg(long)]
    pub keys: PathBuf,
}

#[derive(Subcommand)]
pub enum Command {
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
        /// The upstream answer header that reports what a call spent.
        #[arg(long)]
        spend_header: Option<String>,
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
        #[arg(long, value_enum, default_value = "use")]
        relation: RelationArg,
    },
    /// Revoke an identity's use of a secret; the next use is refused.
    Revoke {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        identity: String,
        #[arg(long)]
        secret: String,
        #[arg(long, value_enum, default_value = "use")]
        relation: RelationArg,
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
        /// A hard cap on what the handle's calls may spend; each call then
        /// sends `lys-reserve` with what it may spend.
        #[arg(long)]
        spend_cap: Option<u64>,
        #[command(flatten)]
        directory: PermissionSource,
    },
    /// Seal another account of a secret, read from standard input.
    AddAccount {
        #[command(flatten)]
        at: Where,
        #[arg(long)]
        secret: String,
        #[arg(long)]
        account: String,
    },
    /// Rest the account in use and move every handle to the next one.
    NextAccount {
        #[command(flatten)]
        at: Where,
        #[arg(long)]
        secret: String,
    },
    /// Return a resting account to service.
    RestoreAccount {
        #[command(flatten)]
        at: Where,
        #[arg(long)]
        secret: String,
        #[arg(long)]
        account: String,
    },
    /// List a secret's accounts.
    Accounts {
        #[command(flatten)]
        at: Where,
        #[arg(long)]
        secret: String,
    },
    /// Drop a handle.
    Drop {
        #[command(flatten)]
        at: Where,
        #[arg(long)]
        handle_id: String,
        /// Also ask the provider to revoke the OAuth grant behind the handle.
        #[arg(long)]
        revoke_upstream: bool,
    },
    /// Seal an OAuth service grant, read from standard input as JSON with
    /// its provenance, access token, expiry and refresh token.
    SealOauth {
        #[command(flatten)]
        at: Where,
        #[arg(long)]
        name: String,
        #[arg(long)]
        owner: String,
        /// The only origin the access token is ever sent to.
        #[arg(long)]
        upstream: String,
    },
    /// As the holder: sign a presentation for one request and print its
    /// headers. The presentation is good for that method, path and body only.
    Sign {
        #[arg(long)]
        key: PathBuf,
        #[arg(long)]
        handle_id: String,
        #[arg(long, default_value = "GET")]
        method: String,
        /// The path with its query, as the proxy is called (`/<secret>/...`).
        #[arg(long)]
        path: String,
        /// A file holding the request body; none means an empty body.
        #[arg(long)]
        body: Option<PathBuf>,
    },
    /// Print the audit log, every line's signature checked.
    Log(Where),
    /// Serve the proxy on a local address.
    Serve {
        #[command(flatten)]
        at: Where,
        #[arg(long, default_value = "127.0.0.1:8472")]
        listen: String,
        #[command(flatten)]
        directory: PermissionSource,
    },
    /// Seal a memory or key record, read from standard input.
    SealRecord {
        #[command(flatten)]
        at: Where,
        #[arg(long)]
        name: String,
        #[arg(long)]
        owner: String,
        #[arg(long, value_enum)]
        class: RecordClass,
    },
    /// Read a memory record as an identity holding the read relation, and
    /// write it to standard output; `--from` and `--len` read a piece.
    ReadRecord {
        #[command(flatten)]
        at: Where,
        #[arg(long)]
        identity: String,
        #[arg(long)]
        name: String,
        #[arg(long, default_value_t = 0)]
        from: usize,
        #[arg(long)]
        len: Option<usize>,
        #[command(flatten)]
        directory: PermissionSource,
    },
}
