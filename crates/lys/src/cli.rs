//! Clap argument definitions for the `lys` binary.
//!
//! Pure declaration — no logic. Doc comments double as `--help` text.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// Cryptographic trust infrastructure for AI agents — identity, attestation,
/// and verification.
#[derive(Debug, Parser)]
#[command(name = "lys", version, propagate_version = true)]
pub struct Cli {
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

    /// Sign an attestation over a payload file and write the JSON envelope.
    Attest {
        /// Path to the identity key file (raw 32-byte Ed25519 seed).
        #[arg(long)]
        key: PathBuf,

        /// Path to the payload file to attest.
        #[arg(long)]
        payload: PathBuf,

        /// Path to write the JSON attestation envelope to.
        #[arg(long)]
        out: PathBuf,
    },

    /// Verify a JSON attestation envelope against a payload file.
    ///
    /// Exits 0 if the attestation verifies, 1 otherwise.
    Verify {
        /// Path to the JSON attestation envelope produced by `lys attest`.
        #[arg(long)]
        attestation: PathBuf,

        /// Path to the payload file the attestation should cover.
        #[arg(long)]
        payload: PathBuf,
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
    },
}
