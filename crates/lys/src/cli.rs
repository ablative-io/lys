//! Clap argument definitions for the `lys` binary.
//!
//! Pure declaration — no logic. Doc comments double as `--help` text.
//!
//! One piece of that text is load-bearing: `ca issue` quotes the
//! capability-claims OID, which must stay identical to the OID the CLI
//! actually writes into certificates. The literal below is pinned to
//! `capability_claims_oid()` by a test that renders the help, so the two
//! cannot drift.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// Cryptographic trust infrastructure for AI agents — identity, attestation,
/// and verification.
#[derive(Debug, Parser)]
#[command(name = "lys", version, propagate_version = true)]
pub struct Cli {
    /// Emit one JSON object on stdout instead of human-readable lines.
    ///
    /// Global, and honoured by every subcommand — a caller never has to
    /// discover which commands support it. Success is
    /// `{"ok":true,...}`; failure is `{"ok":false,"error":"..."}` on stdout
    /// with the diagnostic still on stderr, so a pipeline gating on this
    /// output never receives something it cannot parse. Human-only prose,
    /// such as the `UNVERIFIED` banners, is represented as fields rather
    /// than sentences.
    #[arg(long, global = true)]
    pub json: bool,

    /// Subcommand to run.
    #[command(subcommand)]
    pub command: Command,
}

/// Top-level `lys` subcommands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Identity key management.
    #[command(subcommand)]
    Key(KeyCommand),

    /// Certificate-authority operations: issue and verify Ed25519-signed
    /// X.509 certificates.
    #[command(subcommand)]
    Ca(CaCommand),

    /// Transparency-log operations: append-only logs with C2SP signed-note
    /// checkpoints and self-contained RFC 6962 proof artifacts.
    ///
    /// Every leaf hash is exactly SHA-256(0x00 || leaf-file-bytes), so a
    /// third party holding only the raw leaf file reproduces it with
    /// standard tooling: (printf '\x00'; cat leaf-file) | shasum -a 256.
    #[command(subcommand)]
    Log(LogCommand),

    /// Sign an attestation over a payload file and write the `COSE_Sign1`
    /// artifact.
    Attest {
        /// Path to the identity key file (raw 32-byte Ed25519 seed).
        #[arg(long)]
        key: PathBuf,

        /// Path to the payload file to attest.
        #[arg(long)]
        payload: PathBuf,

        /// Path to write the `COSE_Sign1` attestation artifact to (raw
        /// bytes; conventional extension .cose).
        #[arg(long)]
        out: PathBuf,
    },

    /// Verify a `COSE_Sign1` attestation artifact against a payload file, and
    /// optionally against the certificate that vouches for its signer.
    ///
    /// Without --cert this proves only that *some* key signed the payload, and
    /// prints which key. With --cert and --issuer-public-key it additionally
    /// requires that the certificate verifies against that issuer and that the
    /// attestation's signer is the key the certificate certifies — so the
    /// answer covers "did this named subject, holding these capabilities, make
    /// this statement?" rather than leaving you to compare two hex strings by
    /// eye. Capability claims are printed only when all three checks pass.
    ///
    /// Exits 0 if everything asked for verifies, 1 otherwise with a single
    /// generic failure message.
    Verify {
        /// Path to the `COSE_Sign1` attestation artifact produced by
        /// `lys attest`.
        #[arg(long)]
        attestation: PathBuf,

        /// Path to the payload file the attestation should cover.
        #[arg(long)]
        payload: PathBuf,

        /// Path to the PEM certificate vouching for the attestation's signer
        /// (optional). Requires --issuer-public-key.
        #[arg(long, requires = "issuer_public_key")]
        cert: Option<PathBuf>,

        /// Trusted issuer Ed25519 public key as 64 hexadecimal characters.
        /// Requires --cert.
        #[arg(long, requires = "cert")]
        issuer_public_key: Option<String>,

        /// RFC 3339 instant to evaluate the certificate's validity window at
        /// (default: now). Only meaningful with --cert.
        #[arg(long, requires = "cert")]
        at: Option<String>,
    },

    /// Read-only viewers: print what an artifact or certificate file says,
    /// WITHOUT verifying any of it.
    ///
    /// Every output opens with an UNVERIFIED banner naming the command that
    /// does verify — `lys verify` for attestations, `lys ca verify` for
    /// certificates. For local files only: never expose these through a
    /// network service, where a decode-success signal is a parsing oracle.
    #[command(subcommand)]
    Inspect(InspectCommand),

    /// Seal a payload for a recipient and sign the envelope with the
    /// sender's identity (X25519 + HKDF-SHA256 + AES-256-GCM, Ed25519
    /// attestation over the sealed bytes).
    ///
    /// Writes two files: the JSON sealed envelope to --out and the sender
    /// `COSE_Sign1` attestation binding it to --attestation-out. `lys open`
    /// requires both.
    Seal {
        /// Path to the sender's identity key file (raw 32-byte Ed25519
        /// seed). Must already exist — run `lys key generate` first.
        #[arg(long)]
        key: PathBuf,

        /// Recipient X25519 public key as 64 hexadecimal characters — the
        /// `public key (x25519)` line printed by `lys key inspect`.
        #[arg(long)]
        recipient_public_key: String,

        /// Path to the payload file to seal.
        #[arg(long)]
        payload: PathBuf,

        /// Path to write the JSON sealed envelope to.
        #[arg(long)]
        out: PathBuf,

        /// Path to write the sender's `COSE_Sign1` attestation to (raw
        /// bytes; conventional extension .cose).
        #[arg(long)]
        attestation_out: PathBuf,
    },

    /// Open a sealed envelope, verifying the sender attestation before
    /// decrypting. The plaintext is written to --out, never to stdout.
    ///
    /// Exits 0 on success, 1 otherwise with a single generic failure
    /// message.
    Open {
        /// Path to the recipient's identity key file (raw 32-byte Ed25519
        /// seed). Must already exist — run `lys key generate` first.
        #[arg(long)]
        key: PathBuf,

        /// Expected sender Ed25519 public key as 64 hexadecimal characters
        /// — the `public key (ed25519)` line printed by `lys key inspect`.
        #[arg(long)]
        sender_public_key: String,

        /// Path to the JSON sealed envelope produced by `lys seal`.
        #[arg(long)]
        envelope: PathBuf,

        /// Path to the `COSE_Sign1` sender attestation produced by
        /// `lys seal`.
        #[arg(long)]
        attestation: PathBuf,

        /// Path to write the decrypted payload to (created mode 0600 on
        /// Unix).
        #[arg(long)]
        out: PathBuf,
    },
}

/// `lys ca` subcommands.
#[derive(Debug, Subcommand)]
pub enum CaCommand {
    /// Create a PKCS#10 certificate-signing request for your own identity key
    /// and write it as PEM.
    ///
    /// This is the holder's side of issuance: the request carries your public
    /// key and is self-signed by it, which is the proof of possession an
    /// authority checks before certifying the key. The written file is public;
    /// it contains no private key material. Give it to whoever runs the
    /// authority, who issues with `lys ca issue --request`.
    Request {
        /// Path to your identity key file (raw 32-byte Ed25519 seed). Must
        /// already exist — run `lys key generate` first.
        #[arg(long)]
        key: PathBuf,

        /// Common name to request a certificate under.
        #[arg(long)]
        subject: String,

        /// Path to write the PEM-encoded certificate-signing request to.
        #[arg(long)]
        out: PathBuf,
    },

    /// Issue an Ed25519-signed X.509 certificate for a named subject and
    /// write it as PEM.
    ///
    /// The certificate is valid from now for --validity-days whole days.
    /// With --request, the subject key comes from the holder's request and is
    /// certified only after its proof of possession verifies, so the
    /// certificate binds a key its holder demonstrably controls. Without
    /// --request, a fresh subject keypair is generated by the library and its
    /// private half discarded — convenient, but the certificate then names a
    /// key nobody ever held and proves nothing about any holder. Capability
    /// claims, if given, must be a valid JSON file and are embedded
    /// byte-for-byte as a non-critical X.509 extension under OID
    /// 1.3.6.1.4.1.66364.1 (the lys arc); claims always come from the
    /// authority, never from the request.
    Issue {
        /// Path to the issuing authority's identity key file (raw 32-byte
        /// Ed25519 seed). Must already exist — run `lys key generate` first.
        #[arg(long)]
        key: PathBuf,

        /// Subject common name to issue the certificate for. With --request,
        /// this must equal the common name the request asked for.
        #[arg(long)]
        subject: String,

        /// Path to a PEM certificate-signing request from the subject
        /// (optional). Supply it to certify a key the holder controls.
        #[arg(long)]
        request: Option<PathBuf>,

        /// Path to a JSON file of capability claims to embed in the
        /// certificate (optional).
        #[arg(long)]
        claims: Option<PathBuf>,

        /// Validity window length in whole days from now (notBefore = now,
        /// notAfter = now + this many days). Must be at least 1.
        #[arg(long, value_parser = clap::value_parser!(u32).range(1..))]
        validity_days: u32,

        /// Path to write the PEM-encoded certificate to.
        #[arg(long)]
        out: PathBuf,
    },

    /// Verify a PEM certificate against a trusted issuer public key at a
    /// given instant.
    ///
    /// Exits 0 if the certificate verifies (printing issuer key, checked-at
    /// instant, and any embedded capability claims), 1 otherwise with a
    /// single generic failure message.
    Verify {
        /// Path to the PEM certificate produced by `lys ca issue`.
        #[arg(long)]
        cert: PathBuf,

        /// Trusted issuer Ed25519 public key as 64 hexadecimal characters,
        /// as printed by `lys key inspect`.
        #[arg(long)]
        issuer_public_key: String,

        /// RFC 3339 instant to evaluate the validity window at, e.g.
        /// 2026-07-10T12:00:00Z (default: now).
        #[arg(long)]
        at: Option<String>,
    },
}

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

/// `lys inspect` subcommands — read-only viewers over local files. They
/// decode and print; they verify nothing, and every output says so on its
/// first line.
#[derive(Debug, Subcommand)]
pub enum InspectCommand {
    /// Print the fields of a `COSE_Sign1` attestation artifact WITHOUT
    /// checking its signature.
    ///
    /// Exits 0 if the artifact decodes, 1 otherwise with the same generic
    /// message `lys verify` prints. Everything printed is UNVERIFIED — run
    /// `lys verify` to check the signature against a payload.
    Attestation {
        /// Path to the `COSE_Sign1` attestation artifact produced by
        /// `lys attest`.
        #[arg(long)]
        attestation: PathBuf,
    },

    /// Print a PEM certificate's fields and capability claims WITHOUT
    /// checking it against any issuer.
    ///
    /// Exits 0 if the certificate decodes, 1 otherwise. Everything printed —
    /// subject, validity window, and especially the capability claims — is
    /// UNVERIFIED and attacker-choosable; run `lys ca verify` to check the
    /// signature and the validity window before believing any of it.
    Cert {
        /// Path to the PEM certificate produced by `lys ca issue`.
        #[arg(long)]
        cert: PathBuf,
    },
}

/// `lys key` subcommands.
#[derive(Debug, Subcommand)]
pub enum KeyCommand {
    /// Generate a new Ed25519 identity key, or load the existing one if the
    /// file is already present. Prints the public key; never prints private
    /// key material.
    Generate {
        /// Path to write the identity key file to (raw 32-byte seed,
        /// mode 0600 on Unix).
        #[arg(long)]
        out: PathBuf,
    },

    /// Inspect an existing identity key file: print the Ed25519 public key
    /// and the derived X25519 public key. Never prints private key material.
    Inspect {
        /// Path to the identity key file (raw 32-byte Ed25519 seed).
        #[arg(long)]
        key: PathBuf,

        /// Signed-note key name to additionally print the verifier key
        /// string for. Must equal the log origin this key signs
        /// checkpoints for (the `--origin` given to `lys log init`).
        #[arg(long)]
        note_name: Option<String>,

        /// Also print the OpenSSH public-key line for this identity.
        ///
        /// A lys identity is an Ed25519 keypair, which is what an SSH
        /// signing key is. Point Git at this key with `gpg.format = ssh`
        /// and it can sign commits that `git verify-commit` checks on
        /// plain GitHub — no accounts and no lys on the verifier's machine.
        /// Public material only; there is no private-key export, because
        /// an OpenSSH private key file would be a second at-rest copy of
        /// the seed.
        #[arg(long)]
        ssh: bool,

        /// Also print an `allowed_signers` line binding this principal to
        /// the key for Git signatures.
        ///
        /// Write it to a file and set
        /// `git config gpg.ssh.allowedSignersFile <path>`. The entry is
        /// scoped to `namespaces="git"` deliberately: an unscoped entry
        /// authorises the key for every SSH signature namespace, and a
        /// commit-signing key is not automatically a signing key for
        /// anything else. Must contain no whitespace — the file format is
        /// whitespace-separated.
        #[arg(long, value_name = "PRINCIPAL")]
        allowed_signers: Option<String>,
    },
}

#[cfg(test)]
#[path = "cli_tests.rs"]
mod tests;
