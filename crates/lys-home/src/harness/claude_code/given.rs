//! The documents Claude Code gives a session at start (HOME-003 R2),
//! resolved in the order measured on the harness version
//! [`MEASURED_VERSION`], each as its kind, path, byte length and SHA-256 and
//! never its content.
//!
//! The order, measured by pointing `claude -p` at a local listener and
//! reading the request it sent (PROOF-GIVEN.md):
//!
//! 1. the user CLAUDE.md at `<config>/CLAUDE.md` (`user_claude_md`);
//! 2. the appended instructions file the render wrote (`appended_instructions`);
//! 3. the MCP configuration the render wrote (`mcp_config`);
//! 4. the CLAUDE.md chain: for each directory from the outermost ancestor of
//!    the working directory down to the working directory itself, its
//!    `CLAUDE.md`, `.claude/CLAUDE.md` and `CLAUDE.local.md`, in that order
//!    (`claude_md_chain`). That is the order the request gives them; the
//!    harness reads one directory's three files concurrently, so the order
//!    it happens to read them in changes from run to run and is not what is
//!    recorded;
//! 5. the memory index at `<config>/projects/<slug>/memory/MEMORY.md`
//!    (`memory_index`), the slug being [`projects_slug`].
//!
//! The config directory is the `CLAUDE_CONFIG_DIR` the template sets for the
//! session. When the template sets none it is `HOME/.claude`, with `HOME`
//! from the rendering process's environment, and the resolution says which
//! of the two it was ([`ConfigSource`]). The rendering process's own
//! `CLAUDE_CONFIG_DIR` is never read: that process's environment is not the
//! session's, and a launch from another shell is the template's to settle by
//! setting the variable. When the config directory is `D/.claude` for a
//! directory `D` on the chain, the file `D/.claude/CLAUDE.md` is both the
//! user CLAUDE.md and a chain position; the request gives it once, first, so
//! it is listed once, as `user_claude_md`.
//!
//! A position whose file is absent is omitted. A file that exists and cannot
//! be read fails the resolution by path and operation, since a document the
//! harness would read must never be dropped from the record. Each document
//! is read once, for its length and hash, and no byte of it is kept, returned
//! or put in an error. Nothing is written under the config directory or the
//! working directory, and no document is parsed: a file reaching the request
//! by an `@`-import or from `.claude/rules` is not found here and the record
//! names those two kinds as unlisted.

use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::HomeError;
use crate::harness::claude_code::launch::{INSTRUCTIONS_FILE, MCP_FILE};
use crate::harness::claude_code::projects_slug;
use crate::record::blocks::Hash;

/// The Claude Code version the load order and the slug rule were measured on.
pub const MEASURED_VERSION: &str = "2.1.283";
/// The name of the config directory under a home directory.
pub const CONFIG_DIR_NAME: &str = ".claude";
/// The variable a template sets to name the session's config directory.
pub const CONFIG_DIR_VARIABLE: &str = "CLAUDE_CONFIG_DIR";
/// The three instruction files of one directory on the chain, in the order
/// the request gives them.
pub const CHAIN_FILES: [&str; 3] = ["CLAUDE.md", ".claude/CLAUDE.md", "CLAUDE.local.md"];

/// Which position of the load order a document holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentKind {
    /// `<config>/CLAUDE.md`.
    UserClaudeMd,
    /// The instructions file the render wrote, appended to the system prompt.
    AppendedInstructions,
    /// The MCP configuration the render wrote.
    McpConfig,
    /// One directory's `CLAUDE.md`, `.claude/CLAUDE.md` or `CLAUDE.local.md`.
    ClaudeMdChain,
    /// `<config>/projects/<slug>/memory/MEMORY.md`.
    MemoryIndex,
}

impl DocumentKind {
    /// The kind's name as the record spells it.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::UserClaudeMd => "user_claude_md",
            Self::AppendedInstructions => "appended_instructions",
            Self::McpConfig => "mcp_config",
            Self::ClaudeMdChain => "claude_md_chain",
            Self::MemoryIndex => "memory_index",
        }
    }
}

/// Where the session's config directory came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigSource {
    /// The template's env slot set `CLAUDE_CONFIG_DIR`.
    Template,
    /// The template set none, so it is `HOME/.claude` from the rendering
    /// process's `HOME`.
    Home,
}

/// The session's config directory and where it came from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigDir {
    /// The directory.
    pub path: PathBuf,
    /// Which of the two rules named it.
    pub source: ConfigSource,
}

impl ConfigDir {
    /// The config directory from what the template sets, or from the
    /// rendering process's `HOME` when the template sets none. Refused by
    /// name when neither names one.
    pub fn resolve(template: Option<&str>, process_home: Option<&Path>) -> Result<Self, HomeError> {
        if let Some(dir) = template {
            return Ok(Self {
                path: PathBuf::from(dir),
                source: ConfigSource::Template,
            });
        }
        let home = process_home.ok_or(HomeError::NoConfigDir)?;
        Ok(Self {
            path: home.join(CONFIG_DIR_NAME),
            source: ConfigSource::Home,
        })
    }
}

/// One document the session is given: its kind, its path (relative to the
/// render's out directory for a file the render wrote, absolute otherwise),
/// its byte length and the SHA-256 of its bytes. Never its content.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GivenDocument {
    /// Its position in the load order.
    pub kind: DocumentKind,
    /// Where it is.
    pub path: PathBuf,
    /// Its length in bytes.
    pub length: u64,
    /// The SHA-256 of its bytes, 64 lowercase hex characters.
    pub sha256: String,
}

/// What a resolution found: the config directory it read under, and the
/// documents in the measured order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resolution {
    /// The config directory and its source.
    pub config_dir: ConfigDir,
    /// The documents, in the order the request gives them.
    pub documents: Vec<GivenDocument>,
}

/// Resolve the documents a session started in `cwd` under `config_dir` is
/// given, with the appended instructions and MCP configuration the render
/// wrote under `out`. Reads only; never a byte kept.
pub fn resolve_given(
    cwd: &str,
    config_dir: ConfigDir,
    out: &Path,
) -> Result<Resolution, HomeError> {
    let working = Path::new(cwd);
    if !working.is_absolute() {
        return Err(HomeError::NotAbsolute {
            what: "working directory",
            path: working.to_path_buf(),
        });
    }
    let mut documents = Vec::new();
    let user_claude_md = config_dir.path.join(CHAIN_FILES[0]);
    document(
        DocumentKind::UserClaudeMd,
        &user_claude_md,
        &user_claude_md,
        &mut documents,
    )?;
    for (kind, name) in [
        (DocumentKind::AppendedInstructions, INSTRUCTIONS_FILE),
        (DocumentKind::McpConfig, MCP_FILE),
    ] {
        document(kind, &out.join(name), Path::new(name), &mut documents)?;
    }
    let mut chain: Vec<&Path> = working.ancestors().collect();
    chain.reverse();
    for dir in chain {
        for name in CHAIN_FILES {
            let path = dir.join(name);
            if path == user_claude_md {
                continue;
            }
            document(DocumentKind::ClaudeMdChain, &path, &path, &mut documents)?;
        }
    }
    let memory = config_dir
        .path
        .join("projects")
        .join(projects_slug(cwd))
        .join("memory")
        .join("MEMORY.md");
    document(DocumentKind::MemoryIndex, &memory, &memory, &mut documents)?;
    Ok(Resolution {
        config_dir,
        documents,
    })
}

/// Read the file at `read_at` once and add it as a document named `record_as`;
/// an absent position is omitted, a present file that cannot be read is
/// refused by path and operation.
fn document(
    kind: DocumentKind,
    read_at: &Path,
    record_as: &Path,
    into: &mut Vec<GivenDocument>,
) -> Result<(), HomeError> {
    let bytes = match std::fs::read(read_at) {
        Ok(bytes) => bytes,
        Err(e) if matches!(e.kind(), ErrorKind::NotFound | ErrorKind::NotADirectory) => {
            return Ok(());
        }
        Err(e) => return Err(HomeError::io("reading a given document", read_at, e)),
    };
    into.push(GivenDocument {
        kind,
        path: record_as.to_path_buf(),
        length: bytes.len() as u64,
        sha256: Hash::of(&bytes).as_str().to_owned(),
    });
    drop(bytes);
    Ok(())
}
