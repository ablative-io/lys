//! Clap argument definitions for the `lys-anchor` binary.
//!
//! Pure declaration — no logic. Doc comments double as `--help` text.
//!
//! # Why every subcommand asks for `--admit`, including the ones that only read
//!
//! `Anchor::open` takes an admission policy by value and has no overload that
//! omits it, because *"an admission rule is not a property of the stored log
//! and is not recovered from it"* — two processes may open one store under two
//! different policies (`crates/lys-anchor/src/anchor/open.rs`). This CLI
//! therefore cannot open an anchor at all without naming one, and the only two
//! ways to spare an operator the flag on `status`, `checkpoint` and `prove`
//! would be to pick a policy for them or to bypass `Anchor` and open the raw
//! log. The first is the default DP23 exists to forbid. The second would make
//! `lys-anchor status` report on a log without ever checking it is an anchor.
//!
//! So the flag is required everywhere, and the friction is the library's shape
//! showing through rather than a choice made here. See the increment report:
//! the missing piece is a read-only open.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

/// A lys transparency anchor: an append-only log with a genesis leaf, that
/// publishes signed checkpoints and proofs about itself.
#[derive(Debug, Parser)]
#[command(name = "lys-anchor", version, propagate_version = true)]
pub struct Cli {
    /// Emit one JSON object on stdout instead of human-readable lines.
    ///
    /// Global, and honoured by every subcommand — a caller never has to
    /// discover which commands support it. Success is `{"ok":true,...}`;
    /// failure is `{"ok":false,"error":"..."}` on stdout with the diagnostic
    /// still on stderr, so a pipeline gating on this output never receives
    /// something it cannot parse.
    #[arg(long, global = true)]
    pub json: bool,

    /// Subcommand to run.
    #[command(subcommand)]
    pub command: Command,
}

/// Top-level `lys-anchor` subcommands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Create an anchor: a new log directory, its pinned origin, and its
    /// genesis leaf.
    ///
    /// The origin is the anchor's identity and the signed-note key name its
    /// checkpoints are signed under; it is set exactly once, here. The genesis
    /// file's raw bytes become leaf 0 and are never interpreted — an anchor's
    /// leaf 0 is written at creation or never, because storage offers no
    /// insert, no rewrite and no fork.
    Init {
        /// Path to the anchor directory to create.
        #[arg(long)]
        dir: PathBuf,

        /// Anchor origin (e.g. example.com/my-anchor). Must be non-empty and
        /// contain no whitespace and no '+' — it doubles as the signed-note
        /// key name.
        #[arg(long)]
        origin: String,

        /// Path to the anchor's signing key file (raw 32-byte Ed25519 seed).
        /// Must already exist — run `lys key generate` first. It is never
        /// created here: an anchor that minted a key on a missing file would
        /// publish under an identity nobody was told about.
        #[arg(long)]
        key: PathBuf,

        /// Path to the file whose raw bytes become the genesis leaf.
        #[arg(long)]
        genesis: PathBuf,

        /// The admission policy this invocation runs the anchor under. See
        /// [`AdmissionArgs`] for why every subcommand asks for one.
        #[command(flatten)]
        admission: AdmissionArgs,
    },

    /// Report an anchor's origin, size and identity.
    ///
    /// Reads the anchor's own log and reports what this directory contains —
    /// not that anybody else has attested to it. Also prints the
    /// standalone-equivocation disclosure, which is unconditional in this
    /// version.
    Status {
        /// Path to an initialized anchor directory.
        #[arg(long)]
        dir: PathBuf,

        /// Path to the anchor's signing key file (raw 32-byte Ed25519 seed).
        ///
        /// Required even though this command signs nothing: an anchor holds its
        /// signer from construction, so there is no key-free way to open one.
        #[arg(long)]
        key: PathBuf,

        /// The admission policy this invocation runs the anchor under. See
        /// [`AdmissionArgs`] for why every subcommand asks for one.
        #[command(flatten)]
        admission: AdmissionArgs,
    },

    /// Sign a C2SP signed-note checkpoint over the anchor's current root and
    /// write it to a file.
    ///
    /// The note is signed under the anchor's own origin as the signed-note key
    /// name, which a verifier binds — so one anchor's checkpoint can never be
    /// accepted for another. Publishing is not an append: the log is untouched.
    Checkpoint {
        /// Path to an initialized anchor directory.
        #[arg(long)]
        dir: PathBuf,

        /// Path to the anchor's signing key file (raw 32-byte Ed25519 seed).
        #[arg(long)]
        key: PathBuf,

        /// Path to write the signed checkpoint note to.
        #[arg(long)]
        out: PathBuf,

        /// The admission policy this invocation runs the anchor under. See
        /// [`AdmissionArgs`] for why every subcommand asks for one.
        #[command(flatten)]
        admission: AdmissionArgs,
    },

    /// Build the self-contained JSON inclusion proof for one leaf.
    ///
    /// The `lys/log-inclusion-proof/v1` artifact — an RFC 6962 inclusion path
    /// plus a freshly signed checkpoint over the root it leads to. A reader
    /// needs the leaf bytes and this anchor's verifier key and nothing else,
    /// and checks it with `lys log verify inclusion`. This is the artifact
    /// stock tooling can verify, which is why it is available in a default
    /// build while `submit`'s draft receipt is not.
    Prove {
        /// Path to an initialized anchor directory.
        #[arg(long)]
        dir: PathBuf,

        /// Path to the anchor's signing key file (raw 32-byte Ed25519 seed).
        #[arg(long)]
        key: PathBuf,

        /// Zero-based index of the leaf to prove. Index 0 is the genesis leaf.
        #[arg(long)]
        leaf_index: u64,

        /// Path to write the JSON inclusion-proof artifact to.
        #[arg(long)]
        out: PathBuf,

        /// The admission policy this invocation runs the anchor under. See
        /// [`AdmissionArgs`] for why every subcommand asks for one.
        #[command(flatten)]
        admission: AdmissionArgs,
    },

    /// Offer a statement to the anchor, and — if its admission policy admits it
    /// — append it and write the anchor's signed receipt for it.
    ///
    /// The statement file's raw bytes become the leaf, verbatim: the anchor
    /// does not parse, canonicalize or interpret them, and two identical
    /// submissions are two events at two indices with two receipts.
    ///
    /// A JSON inclusion-proof artifact is written alongside the receipt and is
    /// not optional — a receipt only specialised tooling can check would
    /// violate "verification must outlive the vendor".
    ///
    /// This subcommand exists only in a build with the `unstable-anchor`
    /// feature, because the receipt format is a draft that is not ratified.
    #[cfg(feature = "unstable-anchor")]
    Submit {
        /// Path to an initialized anchor directory.
        #[arg(long)]
        dir: PathBuf,

        /// Path to the anchor's signing key file (raw 32-byte Ed25519 seed).
        #[arg(long)]
        key: PathBuf,

        /// Path to the file whose raw bytes are submitted as the statement.
        #[arg(long)]
        statement: PathBuf,

        /// Path to a DER certificate to present to the admission policy
        /// (optional).
        ///
        /// Presented as ASSERTED BY THE SUBMITTER, never as authenticated: this
        /// CLI performs no handshake and authenticates nobody, so it cannot
        /// honestly claim a peer demonstrated possession of anything. A policy
        /// may still admit on it — `recognised-certificate` does — but
        /// admission on this route is recognition of a credential and not
        /// authentication of whoever presented it.
        #[arg(long)]
        credential: Option<PathBuf>,

        /// Path to write the anchor's `COSE_Sign1` receipt to (raw bytes;
        /// conventional extension .cose).
        #[arg(long)]
        receipt_out: PathBuf,

        /// Path to write the JSON inclusion-proof artifact to.
        #[arg(long)]
        artifact_out: PathBuf,

        /// The admission policy this invocation runs the anchor under. See
        /// [`AdmissionArgs`] for why every subcommand asks for one.
        #[command(flatten)]
        admission: AdmissionArgs,
    },
}

/// The admission policy this invocation runs the anchor under.
///
/// There is no default and there must not be one. An anchor's admission rule is
/// an operator decision: `lys-anchor` ships no policy implementing `Default`,
/// its absence is checked by `compile_fail` doctests rather than intended, and
/// a CLI that filled the gap in would undo all of that in one line. So `--admit`
/// is required, always, and the value an operator types is the whole of the
/// rule.
#[derive(Debug, Args)]
pub struct AdmissionArgs {
    /// Admission policy to run this anchor under. Required — there is no
    /// default, and an anchor must not come into existence under a rule nobody
    /// named.
    #[arg(long, value_enum)]
    pub admit: AdmitPolicy,

    /// Maximum statement length in bytes, inclusive. Required by, and read only
    /// by, `--admit max-size`.
    ///
    /// The limit bounds the statement — the bytes that become a leaf and are
    /// stored forever — and not the credential, which is admission-time only.
    #[arg(long, required_if_eq("admit", "max-size"))]
    pub max_bytes: Option<usize>,

    /// Certificate-authority Ed25519 public key as 64 hexadecimal characters.
    /// Required by, and read only by, `--admit recognised-certificate`.
    #[arg(long, required_if_eq("admit", "recognised-certificate"))]
    pub issuer_public_key: Option<String>,

    /// Restrict admission to certificates over this subject Ed25519 public key,
    /// as 64 hexadecimal characters. Repeatable. Read only by
    /// `--admit recognised-certificate`.
    ///
    /// Omitted entirely, the policy admits any subject the authority certified.
    /// There is no way to express an empty allow-list here, and that is
    /// deliberate: an empty set admits nothing, and a flag that produced it by
    /// being left out would be an allow-list that silently became a deny-all.
    #[arg(long)]
    pub subject_key: Vec<String>,
}

/// The admission policies `lys-anchor` ships, as command-line values.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum AdmitPolicy {
    /// Admit every submission, whatever it carries and whoever sent it.
    ///
    /// The honest policy for an anchor that takes all comers, and it has to be
    /// written out to get one — which is the entire difference between an
    /// operator who decided to take all comers and one who never thought about
    /// it.
    AcceptAll,

    /// Admit a submission whose statement is at most `--max-bytes` long.
    MaxSize,

    /// Admit a submission presenting a certificate issued by
    /// `--issuer-public-key`, currently within its validity window, and — if
    /// any `--subject-key` is given — over one of those subjects.
    ///
    /// Recognising a credential is not authenticating a submitter. A
    /// certificate is a public artifact, so anyone who has seen one can present
    /// it, and this CLI authenticates nobody.
    RecognisedCertificate,
}

impl AdmitPolicy {
    /// The value an operator typed, for use in a diagnostic.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AcceptAll => "accept-all",
            Self::MaxSize => "max-size",
            Self::RecognisedCertificate => "recognised-certificate",
        }
    }
}

#[cfg(test)]
#[path = "cli_tests.rs"]
mod tests;
