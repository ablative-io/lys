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

mod log;
mod runner;

pub use log::{LogCommand, LogProveCommand, LogVerifyCommand};
pub use runner::RunnerCommand;

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

    /// The standalone identity product's dependencies: prepare the private
    /// deployment artifacts, configure Rauthy's clients and themes, and
    /// check readiness.
    #[command(subcommand)]
    Identity(crate::identity::IdentityCommand),

    /// Transparency-log operations: append-only logs with C2SP signed-note
    /// checkpoints and self-contained RFC 6962 proof artifacts.
    ///
    /// Every leaf hash is exactly SHA-256(0x00 || leaf-file-bytes), so a
    /// third party holding only the raw leaf file reproduces it with
    /// standard tooling: (printf '\x00'; cat leaf-file) | shasum -a 256.
    #[command(subcommand)]
    Log(LogCommand),

    /// Lys's own runner: holds each agent the server starts in its own
    /// background pseudo-terminal, reached on a Unix socket only, and the
    /// bridge a runner on another machine dials the server through.
    #[command(subcommand)]
    Runner(RunnerCommand),

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
    /// The certificate is valid from now for exactly one of --validity
    /// (a suffixed window, e.g. 30m, 12h, 7d) or --validity-days (whole days).
    /// Prefer --validity: short-lived scoped grants want minutes to hours, and
    /// --validity-days cannot express less than a day.
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
    ///
    /// Every certificate is entered in the transparency log at --log before
    /// it is written, as one leaf whose bytes are the certificate's DER and
    /// nothing else, and the leaf is written at --leaf-out. Only the CA key is
    /// needed: the log's operator makes the inclusion-proof artifact with
    /// `lys log prove inclusion --leaf-index <reported leaf index>`. One
    /// operator holding both keys may give --log-key and --artifact-out
    /// together to make it in the same run.
    ///
    /// --issuer-out writes the issuer certificate stored beside the key, at
    /// the key file's path with .issuer.pem appended (for issuer.key,
    /// issuer.key.issuer.pem), building and storing it the first time it is
    /// asked for; `lys ca issuer-cert` writes the same bytes.
    #[command(group(
        clap::ArgGroup::new("validity_window")
            .required(true)
            .args(["validity", "validity_days"])
    ))]
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

        /// Validity window from now as a count and one unit: s, m, h, or d
        /// (notBefore = now, notAfter = now + this). For example 30m, 12h, 7d.
        /// Exactly one unit — compound windows like 1h30m are refused rather
        /// than partially read.
        #[arg(long)]
        validity: Option<String>,

        /// Validity window length in whole days from now (notBefore = now,
        /// notAfter = now + this many days). Must be at least 1.
        ///
        /// Retained unchanged; --validity covers the same windows and finer
        /// ones.
        #[arg(long, value_parser = clap::value_parser!(u32).range(1..))]
        validity_days: Option<u32>,

        /// Path to write the PEM-encoded certificate to.
        #[arg(long)]
        out: PathBuf,

        /// Path to write the issuer's self-signed PEM certificate to
        /// (optional). With it, `openssl verify -CAfile <this> <cert>` checks
        /// the issued certificate with nothing from lys. The bytes are those
        /// of the issuer certificate stored beside the key, at the key file's
        /// path with .issuer.pem appended, built and stored the first time.
        #[arg(long)]
        issuer_out: Option<PathBuf>,

        /// Transparency log directory to enter the certificate in before it is
        /// written, as one leaf whose bytes are the certificate's DER. If the
        /// log cannot take it, no certificate is written.
        #[arg(long)]
        log: PathBuf,

        /// Path to write the leaf to: the certificate's DER bytes exactly.
        #[arg(long)]
        leaf_out: PathBuf,

        /// The log operator's identity key file, which signs the checkpoint
        /// in the inclusion proof (optional). Only with --artifact-out, for
        /// one operator holding both the CA key and the log's key.
        #[arg(long)]
        log_key: Option<PathBuf>,

        /// Path to write the `lys/log-inclusion-proof/v1` artifact to, which
        /// `lys log verify inclusion` and `scripts/verify_inclusion.py` check
        /// (optional). Only with --log-key.
        #[arg(long)]
        artifact_out: Option<PathBuf>,
    },

    /// Write the issuer's self-signed CA certificate as PEM.
    ///
    /// The certificate is the one stored beside the key, at the key file's
    /// path with .issuer.pem appended (for issuer.key, issuer.key.issuer.pem).
    /// It is built and stored the first time it is asked for, here or by
    /// `lys ca issue --issuer-out`, and every later call writes the same
    /// bytes. It is public: it carries the issuer public key and a signature,
    /// never private key material. With it, `openssl verify -CAfile <this>
    /// <cert>` checks a certificate the key issued with nothing from lys.
    IssuerCert {
        /// Path to the issuing authority's identity key file (raw 32-byte
        /// Ed25519 seed). Must already exist — run `lys key generate` first.
        #[arg(long)]
        key: PathBuf,

        /// Path to write the PEM-encoded issuer certificate to. Refused if it
        /// already exists.
        #[arg(long)]
        out: PathBuf,
    },

    /// Verify a PEM certificate against a trusted issuer public key at a
    /// given instant.
    ///
    /// Exits 0 if the certificate verifies (printing issuer key, checked-at
    /// instant, and any embedded capability claims), 1 otherwise with a
    /// single generic failure message.
    ///
    /// Verification without a log does not check revocation.
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
