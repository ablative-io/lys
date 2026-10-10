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

#[path = "cli/log.rs"]
mod log;
#[path = "cli/proxy.rs"]
mod proxy;
#[path = "cli/runner.rs"]
mod runner;

pub use log::{LogCommand, LogProveCommand, LogVerifyCommand};
pub use proxy::ProxyCommand;
pub use runner::{JudgeHarness, RunnerCommand};

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

    /// Lys's model proxy: every model call a Lys-started run makes goes
    /// through it to the provider unchanged, and is recorded under its run.
    #[command(subcommand)]
    Proxy(ProxyCommand),

    /// A Lys-started run's own variables: read and patch its agent's map
    /// and its session's map, as the run itself, by the run pass Lys
    /// rendered into the seat's own configuration. The pass is read from
    /// the launch's mcp.json (a Claude Code seat) or from `LYS_AGENT_PASS`
    /// and `LYS_MCP_URL` (a Codex seat), and is never printed.
    Variables(VariablesArgs),

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

    /// Seats: an agent identity's standing place on a machine, held by the
    /// installed identity server, which starts it managed through its
    /// machine's runner.
    ///
    /// Asked of the installed server over loopback as its operator, with
    /// the `lys-operator` header read from the install's operator-token
    /// file, which is never printed. A development install keeps one; a
    /// service install keeps none, and is refused `operator_token_absent`.
    Seat(SeatArgs),

    /// Show a seat's live session in this terminal, following it as it
    /// runs: user turns, assistant text, tool calls and results, and status.
    ///
    /// Read-only unless --type is given. Ctrl-C detaches, and so does the
    /// end of standard input under --type; detaching never ends the
    /// session. The server keeps the attach and each typed line as a
    /// record naming who. With --session, a session that was not started
    /// managed is shown read-only as its pseudo-terminal's exact bytes.
    Attach(AttachArgs),

    /// An agent's own acts against the installed identity server, made
    /// with the agent's own credential and never the operator token.
    Agent(AgentArgs),
}

/// `lys variables`: which map, and what to do with it.
#[derive(Debug, clap::Args)]
pub struct VariablesArgs {
    /// The launch's working folder, where its mcp.json carries the run
    /// pass; the current folder when absent.
    #[arg(long)]
    pub folder: Option<PathBuf>,

    #[command(subcommand)]
    pub command: VariablesCommand,
}

/// `lys variables` subcommands.
#[derive(Debug, Subcommand)]
pub enum VariablesCommand {
    /// Read the run's agent map, or with --session its session map: each
    /// variable with its revision, author and expiry, and the scope's
    /// revision to carry on the next set.
    Get {
        /// The session's map instead of the agent's.
        #[arg(long)]
        session: bool,
    },
    /// Patch the map: each key=value sets a key (a JSON value when it
    /// parses as one, else the text), key= alone removes it; the revision
    /// read must be given, and a stale one is refused by name.
    Set {
        /// The session's map instead of the agent's.
        #[arg(long)]
        session: bool,

        /// The scope revision read; 0 for a map never patched.
        #[arg(long)]
        revision: u64,

        /// When the keys set expire, in seconds since the Unix epoch.
        #[arg(long)]
        expires_at: Option<u64>,

        /// The keys, as key=value or key= to remove.
        #[arg(required = true)]
        values: Vec<String>,
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

/// `lys seat`: what to do, and the server it is asked of.
#[derive(Debug, clap::Args)]
pub struct SeatArgs {
    /// The identity server's base address, `http://<loopback>:<port>` with
    /// `/api` after it when the server serves its screens, over the
    /// installed service's own. The operator token is still the install's,
    /// and is sent only to a numeric loopback address.
    #[arg(long, global = true, value_name = "BASE")]
    pub server: Option<String>,

    /// What to do.
    #[command(subcommand)]
    pub command: SeatCommand,
}

/// `lys seat` subcommands. Each prints human lines, or one JSON object
/// with --json; a refusal is shown by the server's name and words.
#[derive(Debug, Subcommand)]
pub enum SeatCommand {
    /// Add a seat: a name, the agent it runs as, a reviewed profile
    /// version, and the machine whose runner holds it.
    Add {
        /// The seat's name: lowercase letters, digits and '-', 1 to 64,
        /// unique in the install.
        name: String,

        /// The agent identity the seat runs as.
        #[arg(long)]
        agent: String,

        /// The reviewed version of the agent's profile the seat starts
        /// with.
        #[arg(long)]
        profile_version: u64,

        /// The machine whose runner holds the seat.
        #[arg(long)]
        machine: String,

        /// The folder the seat works in, on that machine.
        #[arg(long)]
        working_folder: Option<String>,
    },

    /// List every seat with its state and last signal, and whether the
    /// runner could be read; a runner that could not is named with its
    /// reason.
    List,

    /// Start a seat's session through the server's own start path, managed.
    Start {
        /// The seat.
        name: String,
    },

    /// Stop a seat's session: the harness is asked to end when idle, and
    /// a turn in progress is refused `seat_turn_in_progress` unless
    /// --force.
    Stop {
        /// The seat.
        name: String,

        /// End the session even while a turn is in progress.
        #[arg(long)]
        force: bool,
    },

    /// Stop a seat's session, then start it again.
    Restart {
        /// The seat.
        name: String,

        /// End the session even while a turn is in progress.
        #[arg(long)]
        force: bool,
    },

    /// Send a message to a running seat as a user turn, never as typed
    /// keys. Text that is empty or carries control characters is refused
    /// by name.
    Send {
        /// The seat.
        name: String,

        /// The message; its words are joined with single spaces.
        #[arg(required = true, trailing_var_arg = true, allow_hyphen_values = true)]
        text: Vec<String>,
    },

    /// Import one seat's existing configuration by a confirmed,
    /// restartable plan: a dry run that changes nothing, then the person's
    /// confirmation of that exact plan. Importing never starts, stops or
    /// restarts a seat.
    Import(SeatImportArgs),
}

/// `lys attach`: the seat or session to show, and how.
#[derive(Debug, clap::Args)]
pub struct AttachArgs {
    /// The seat whose session to show.
    #[arg(required_unless_present = "session", conflicts_with = "session")]
    pub seat: Option<String>,

    /// Send each line typed on standard input to the seat as a user turn,
    /// through the same path as `lys seat send`, never as keys. An empty
    /// line sends nothing.
    #[arg(long = "type", conflicts_with = "session")]
    pub typing: bool,

    /// Show this session, one not started managed, read-only as the exact
    /// bytes of its pseudo-terminal.
    #[arg(long, value_name = "ID")]
    pub session: Option<String>,

    /// The identity server's base address, `http://<loopback>:<port>` with
    /// `/api` after it when the server serves its screens, over the
    /// installed service's own. The operator token is still the install's,
    /// and is sent only to a numeric loopback address.
    #[arg(long, value_name = "BASE")]
    pub server: Option<String>,
}

/// `lys seat import`: what to do.
#[derive(Debug, clap::Args)]
pub struct SeatImportArgs {
    /// What to do.
    #[command(subcommand)]
    pub command: SeatImportCommand,
}

/// `lys seat import` subcommands, each of one seat.
#[derive(Debug, Subcommand)]
pub enum SeatImportCommand {
    /// Read the seat's declared sources at one instant and show the whole
    /// plan: its sources and revisions, every record it writes, credential
    /// references, replacements, schedule counts, exclusions,
    /// prerequisites and refusals. Nothing is sent, started or changed.
    DryRun {
        /// The seat.
        name: String,

        /// The JSON manifest naming the seat's sources.
        #[arg(long, value_name = "PATH")]
        manifest: PathBuf,
    },

    /// Confirm the exact plan a dry run showed, as the person responsible,
    /// and apply it under one operation that resumes after a stop.
    Confirm {
        /// The seat.
        name: String,

        /// The plan id the dry run showed.
        #[arg(long, value_name = "ID")]
        plan: String,

        /// The plan revision the dry run showed.
        #[arg(long, value_name = "REVISION")]
        revision: String,
    },

    /// Show every import of the seat and the one selected.
    Status {
        /// The seat.
        name: String,
    },
}

/// `lys agent`: what to do, and the server it is asked of.
#[derive(Debug, clap::Args)]
pub struct AgentArgs {
    /// The identity server's base address, `http://<loopback>:<port>` with
    /// `/api` after it when the server serves its screens, over the
    /// installed service's own. The request is sent only to a numeric
    /// loopback address, and carries no operator token.
    #[arg(long, global = true, value_name = "BASE")]
    pub server: Option<String>,

    /// What to do.
    #[command(subcommand)]
    pub command: AgentCommand,
}

/// `lys agent` subcommands.
#[derive(Debug, Subcommand)]
pub enum AgentCommand {
    /// Ask for the agent's pass to an approved app, carrying the agent's
    /// own grants on that app's kinds, and write it to a file only its
    /// owner may read, for a tool to present to the app.
    ///
    /// The agent proves itself with a grant credential its responsible
    /// person issued for one of its grants (`POST /grants/{id}/tokens`),
    /// read from a file only its owner may read. Neither the credential nor
    /// the pass is ever printed. The pass lives the provider's pass
    /// lifetime; ask again for a new one.
    Pass {
        /// The agent identity the pass is for.
        #[arg(long, value_name = "ID")]
        agent: String,

        /// The app the pass is for: an app approved on the Apps screen.
        #[arg(long, value_name = "APP")]
        audience: String,

        /// The file holding the grant credential, owner-only.
        #[arg(long, value_name = "PATH")]
        credential_file: PathBuf,

        /// The file to write the pass to, created owner-only (mode 0600).
        #[arg(long, value_name = "PATH")]
        out: PathBuf,
    },
}

#[cfg(test)]
#[path = "cli_tests.rs"]
mod tests;
