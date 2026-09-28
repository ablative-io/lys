//! The documents Claude Code gives a session at start (HOME-003 R2,
//! HOME-011), resolved in the order the harness's request gives them, each
//! as its kind, path, byte length and SHA-256 and never its content.
//!
//! The order, measured by pointing `claude -p` at a local listener and
//! reading the request it sent (PROOF-GIVEN.md), re-measured on
//! [`MEASURED_VERSION`]:
//!
//! 1. the appended instructions file the render wrote (`appended_instructions`);
//! 2. the MCP configuration the render wrote (`mcp_config`);
//! 3. the user CLAUDE.md at `<config>/CLAUDE.md` (`user_claude_md`);
//! 4. the CLAUDE.md chain: for each directory from the outermost ancestor of
//!    the working directory down to the working directory itself, its
//!    `CLAUDE.md`, `.claude/CLAUDE.md` and `CLAUDE.local.md`, in that order
//!    (`claude_md_chain`);
//! 5. the memory index at `<config>/projects/<slug>/memory/MEMORY.md`
//!    (`memory_index`), the slug being [`projects_slug`].
//!
//! The request places the appended instructions in its `system` array, ahead
//! of the first user message that carries the user CLAUDE.md, the chain and
//! the memory index in the order above. The harness reads the files in a
//! different order (the user CLAUDE.md first, and one directory's three
//! files concurrently, so their order changes from run to run); wherever the
//! request's order and the read order differ, the request's order wins,
//! since it is what reached the model. The MCP configuration contributes
//! nothing to the request when its server offers no tool, so its place
//! straight after the appended instructions comes from the read order until
//! a real server's tools position is measured.
//!
//! The config directory is the `CLAUDE_CONFIG_DIR` the template sets for the
//! session. When the template sets none it is `HOME/.claude`, with `HOME`
//! from the rendering process's environment, and the resolution says which
//! of the two it was ([`ConfigSource`]). The rendering process's own
//! `CLAUDE_CONFIG_DIR` is never read: that process's environment is not the
//! session's, and a launch from another shell is the template's to settle by
//! setting the variable. When the config directory is `D/.claude` for a
//! directory `D` on the chain, the file `D/.claude/CLAUDE.md` is both the
//! user CLAUDE.md and a chain position; it is listed once, as
//! `user_claude_md`, in the user file's place, and not again on the chain.
//!
//! Every absolute path is canonicalised before it is walked or recorded
//! (HOME-010, ADR-029, [`given_path`]): the working directory and the config
//! directory whole, each document at its parent resolved with its own name
//! kept, so one file reached through a symlinked directory, a `..` or a
//! trailing slash records as one path, and the user CLAUDE.md is told from a
//! chain position by the two canonical paths. A directory that does not
//! exist is used as given. The two written files stay relative to the out
//! directory, and the memory index's slug is taken from the working
//! directory as given.
//!
//! [`given_path`]: crate::harness::claude_code::given_path
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
use crate::harness::claude_code::given_path::{canonical_dir, canonical_document};
use crate::harness::claude_code::launch::{INSTRUCTIONS_FILE, MCP_FILE};
use crate::harness::claude_code::projects_slug;
use crate::record::blocks::Hash;

/// The Claude Code version the load order was re-measured on and the slug
/// rule was measured on.
pub const MEASURED_VERSION: &str = "2.1.283";
/// The name of the config directory under a home directory.
pub const CONFIG_DIR_NAME: &str = ".claude";
/// The variable a template sets to name the session's config directory.
pub const CONFIG_DIR_VARIABLE: &str = "CLAUDE_CONFIG_DIR";
/// The rendering process's variable the config directory falls back to.
pub const HOME_VARIABLE: &str = "HOME";
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
    /// name when neither names one (an empty `HOME` is none), and refused
    /// naming the variable and the value's shape when the value is not an
    /// absolute path: a relative one, an empty one or one beginning with
    /// `~` would resolve against lys-home's own working directory, which is
    /// never the session's.
    pub fn resolve(template: Option<&str>, process_home: Option<&Path>) -> Result<Self, HomeError> {
        if let Some(dir) = template {
            let path = absolute_or_refused(CONFIG_DIR_VARIABLE, Path::new(dir))?;
            return Ok(Self {
                path,
                source: ConfigSource::Template,
            });
        }
        let home = process_home
            .filter(|home| !home.as_os_str().is_empty())
            .ok_or(HomeError::NoConfigDir)?;
        let home = absolute_or_refused(HOME_VARIABLE, home)?;
        Ok(Self {
            path: home.join(CONFIG_DIR_NAME),
            source: ConfigSource::Home,
        })
    }
}

/// The shape of a path that is not absolute, for a refusal to name.
#[must_use]
pub fn path_shape(path: &Path) -> &'static str {
    if path.as_os_str().is_empty() {
        "empty"
    } else if path.starts_with("~") {
        "beginning with `~`, a shell expansion the harness does not perform"
    } else {
        "relative"
    }
}

/// The path when it is absolute; otherwise a refusal naming the variable it
/// came from and its shape.
fn absolute_or_refused(variable: &'static str, path: &Path) -> Result<PathBuf, HomeError> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Err(HomeError::NotAbsolute {
            what: variable,
            shape: path_shape(path),
            path: path.to_path_buf(),
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
///
/// Every absolute path is canonicalised by [`given_path`]'s rule before it
/// is walked or recorded (HOME-010 R2): the working directory whole before
/// the chain is walked, the config directory whole (recorded as
/// `config_dir.path`), and every document at its parent resolved with its
/// own name kept. A working or config directory that does not exist is used
/// as given. The memory index's slug is taken from `cwd` exactly as given,
/// and the two written files keep their paths relative to `out`.
///
/// [`given_path`]: crate::harness::claude_code::given_path
pub fn resolve_given(
    cwd: &str,
    mut config_dir: ConfigDir,
    out: &Path,
) -> Result<Resolution, HomeError> {
    let given_working = Path::new(cwd);
    if !given_working.is_absolute() {
        return Err(HomeError::NotAbsolute {
            what: "working directory",
            shape: path_shape(given_working),
            path: given_working.to_path_buf(),
        });
    }
    let working = canonical_dir(given_working)?.into_path();
    config_dir.path = canonical_dir(&config_dir.path)?.into_path();
    let mut documents = Vec::new();
    for (kind, name) in [
        (DocumentKind::AppendedInstructions, INSTRUCTIONS_FILE),
        (DocumentKind::McpConfig, MCP_FILE),
    ] {
        if let Some((length, sha256)) = measure(&out.join(name))? {
            documents.push(GivenDocument {
                kind,
                path: PathBuf::from(name),
                length,
                sha256,
            });
        }
    }
    let user_claude_md = absolute(
        DocumentKind::UserClaudeMd,
        &config_dir.path.join(CHAIN_FILES[0]),
        None,
        &mut documents,
    )?;
    let mut chain: Vec<&Path> = working.ancestors().collect();
    chain.reverse();
    for dir in chain {
        for name in CHAIN_FILES {
            absolute(
                DocumentKind::ClaudeMdChain,
                &dir.join(name),
                user_claude_md.as_deref(),
                &mut documents,
            )?;
        }
    }
    let memory = config_dir
        .path
        .join("projects")
        .join(projects_slug(cwd))
        .join("memory")
        .join("MEMORY.md");
    absolute(DocumentKind::MemoryIndex, &memory, None, &mut documents)?;
    Ok(Resolution {
        config_dir,
        documents,
    })
}

/// Add the absolute document read at `path` under its canonical path, and
/// return that path; `None` when the position is absent, or when its
/// canonical path is `listed_once`, the user CLAUDE.md already listed, so
/// one file is never listed twice nor read twice.
fn absolute(
    kind: DocumentKind,
    path: &Path,
    listed_once: Option<&Path>,
    into: &mut Vec<GivenDocument>,
) -> Result<Option<PathBuf>, HomeError> {
    let mut recorded = None;
    if let Some(listed) = listed_once {
        let canonical = canonical_document(path)?.into_path();
        if canonical == listed {
            return Ok(None);
        }
        recorded = Some(canonical);
    }
    let Some((length, sha256)) = measure(path)? else {
        return Ok(None);
    };
    let recorded = match recorded {
        Some(canonical) => canonical,
        None => canonical_document(path)?.into_path(),
    };
    into.push(GivenDocument {
        kind,
        path: recorded.clone(),
        length,
        sha256,
    });
    Ok(Some(recorded))
}

/// Read the file at `read_at` once for its length and SHA-256; an absent
/// position is `None`, a present file that cannot be read is refused by
/// path and operation.
fn measure(read_at: &Path) -> Result<Option<(u64, String)>, HomeError> {
    let bytes = match std::fs::read(read_at) {
        Ok(bytes) => bytes,
        Err(e) if matches!(e.kind(), ErrorKind::NotFound | ErrorKind::NotADirectory) => {
            return Ok(None);
        }
        Err(e) => return Err(HomeError::io("reading a given document", read_at, e)),
    };
    let measured = (bytes.len() as u64, Hash::of(&bytes).as_str().to_owned());
    drop(bytes);
    Ok(Some(measured))
}
