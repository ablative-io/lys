//! The `lys log` subcommands, with their proofs and verifications.

use std::path::PathBuf;

use clap::Subcommand;

/// `lys log` subcommands.
#[derive(Debug, Subcommand)]
pub enum LogCommand {
    /// Initialize a new log directory, pinning its origin.
    ///
    /// The origin is the log's unique identity and the signed-note key name
    /// its checkpoints are signed under. It is set exactly once, here;
    /// refuses to re-initialize an existing log directory.
    Init {
        /// Path to the log directory to create.
        #[arg(long)]
        dir: PathBuf,

        /// Log origin (e.g. example.com/my-log). Must be non-empty and
        /// contain no whitespace and no '+' — it doubles as the signed-note
        /// key name.
        #[arg(long)]
        origin: String,
    },

    /// Report a log's origin, size and current root hash — without its key.
    ///
    /// Read-only. Every other route to a log's current size and root goes
    /// through `lys log checkpoint`, which signs and so requires custody of
    /// the signing key; observing your own append-only log should not.
    /// Prints no verifier key, because that is derived from the signing key
    /// this command never touches — use `lys log checkpoint` when a third
    /// party needs it. The root is recomputed from local bytes the local
    /// operator controls, so it reports what this directory contains, not
    /// that anyone has attested to it.
    Status {
        /// Path to the log directory to inspect.
        #[arg(long)]
        dir: PathBuf,
    },

    /// Append a leaf file's raw bytes to the log.
    ///
    /// The bytes are hashed verbatim per RFC 6962: leaf hash =
    /// SHA-256(0x00 || file-bytes), reproducible with
    /// (printf '\x00'; cat leaf-file) | shasum -a 256. Appending never
    /// requires the signing key.
    Append {
        /// Path to an initialized log directory.
        #[arg(long)]
        dir: PathBuf,

        /// Path to the leaf file whose raw bytes are appended.
        #[arg(long)]
        leaf: PathBuf,
    },

    /// Sign a C2SP signed-note checkpoint over the log's current root and
    /// write it to a file.
    ///
    /// The checkpoint is signed under the log's origin as the note key
    /// name. Also prints the verifier key string a third party needs.
    Checkpoint {
        /// Path to an initialized log directory.
        #[arg(long)]
        dir: PathBuf,

        /// Path to the log operator's identity key file (raw 32-byte
        /// Ed25519 seed). Must already exist — run `lys key generate`
        /// first.
        #[arg(long)]
        key: PathBuf,

        /// Path to write the signed checkpoint note to.
        #[arg(long)]
        out: PathBuf,
    },

    /// Build a self-contained, signed JSON proof artifact.
    #[command(subcommand)]
    Prove(LogProveCommand),

    /// Verify a proof artifact as a third party.
    ///
    /// Requires only the artifact, the verifier key string, and (for
    /// inclusion) the leaf file — never the log directory.
    #[command(subcommand)]
    Verify(LogVerifyCommand),
}

/// `lys log prove` subcommands.
#[derive(Debug, Subcommand)]
pub enum LogProveCommand {
    /// Build an inclusion-proof artifact for one leaf, embedding a freshly
    /// signed checkpoint of the current tree.
    Inclusion {
        /// Path to an initialized log directory.
        #[arg(long)]
        dir: PathBuf,

        /// Path to the log operator's identity key file (raw 32-byte
        /// Ed25519 seed). Must already exist — run `lys key generate`
        /// first.
        #[arg(long)]
        key: PathBuf,

        /// Zero-based index of the leaf to prove.
        #[arg(long)]
        leaf_index: u64,

        /// Path to write the JSON inclusion-proof artifact to.
        #[arg(long)]
        out: PathBuf,
    },

    /// Build a consistency-proof artifact from an earlier tree size to the
    /// current tree, embedding freshly signed checkpoints of both.
    Consistency {
        /// Path to an initialized log directory.
        #[arg(long)]
        dir: PathBuf,

        /// Path to the log operator's identity key file (raw 32-byte
        /// Ed25519 seed). Must already exist — run `lys key generate`
        /// first.
        #[arg(long)]
        key: PathBuf,

        /// The earlier tree size to prove consistency from. Must be at
        /// least 1 and strictly below the current tree size.
        #[arg(long, value_parser = clap::value_parser!(u64).range(1..))]
        old_size: u64,

        /// Path to write the JSON consistency-proof artifact to.
        #[arg(long)]
        out: PathBuf,
    },
}

/// `lys log verify` subcommands — the third-party path. These take no log
/// directory at all: verification runs from the artifact, the verifier key
/// string, and (for inclusion) the leaf file alone.
#[derive(Debug, Subcommand)]
pub enum LogVerifyCommand {
    /// Verify an inclusion-proof artifact against a leaf file.
    ///
    /// Exits 0 if the artifact verifies, 1 otherwise with a single generic
    /// failure message.
    Inclusion {
        /// Path to the JSON inclusion-proof artifact produced by
        /// `lys log prove inclusion`.
        #[arg(long)]
        artifact: PathBuf,

        /// Path to the raw leaf file the artifact should prove.
        #[arg(long)]
        leaf: PathBuf,

        /// Trusted verifier key string in the signed-note format
        /// `ORIGIN+KEYID_HEX+KEY_BASE64`, as printed by
        /// `lys log checkpoint` and `lys key inspect --note-name`.
        #[arg(long)]
        verifier_key: String,
    },

    /// Verify a consistency-proof artifact.
    ///
    /// Exits 0 if the artifact verifies, 1 otherwise with a single generic
    /// failure message.
    Consistency {
        /// Path to the JSON consistency-proof artifact produced by
        /// `lys log prove consistency`.
        #[arg(long)]
        artifact: PathBuf,

        /// Trusted verifier key string in the signed-note format
        /// `ORIGIN+KEYID_HEX+KEY_BASE64`, as printed by
        /// `lys log checkpoint` and `lys key inspect --note-name`.
        #[arg(long)]
        verifier_key: String,
    },
}
