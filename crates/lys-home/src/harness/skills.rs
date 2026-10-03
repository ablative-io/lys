//! Kept skills written into a session's own config directory (HOME-037 R3).
//!
//! A skill arrives as its name, its text and the SHA-256 Lys keeps it by. It
//! is checked before anything is written: the name is one visible path
//! component and the text hashes to what Lys recorded. Each is written at
//! `skills/<name>/SKILL.md` under the config directory the session is given,
//! which must be absolute; a file already there is kept only when its bytes
//! are the same. Nothing is written anywhere else.
//!
//! The caller holds the config directory as its own for the write: no other
//! party replaces its parts while skills are written into it. Two writers of
//! the same skill may race; the first link stands and the other is kept only
//! when its bytes are the same.

use std::io::{ErrorKind, Write};
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::HomeError;
use crate::harness::launch_fields::KeptSkill;
use crate::record::blocks::Hash;

/// The longest skill name: 64 is the Agent Skills specification's bound on a
/// skill's `name`, which Claude Code reads from `skills/<name>/SKILL.md`.
const NAME_MAX: usize = 64;

/// A skill as a launch carries it: its name, its text and the hash it is
/// kept by.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillFile {
    /// Its name.
    pub name: String,
    /// Its text.
    pub text: String,
    /// The SHA-256 of the text, as lowercase hex.
    pub sha256: String,
}

fn refused(name: &str, reason: &'static str) -> HomeError {
    HomeError::SkillRefused {
        name: name.to_owned(),
        reason,
    }
}

/// `file` as the launch records it, once its name is one visible path
/// component and its text hashes to the hash it carries.
pub fn check(file: &SkillFile) -> Result<KeptSkill, HomeError> {
    let name = &file.name;
    let visible = !name.is_empty()
        && name.len() <= NAME_MAX
        && !name.starts_with(['.', '-'])
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '_' | '.'));
    if !visible {
        return Err(refused(name, "its name is not one visible path component"));
    }
    if Hash::of(file.text.as_bytes()).as_str() != file.sha256 {
        return Err(refused(
            name,
            "its text does not hash to the hash Lys keeps it by",
        ));
    }
    Ok(KeptSkill {
        name: name.clone(),
        path: format!("skills/{name}/SKILL.md"),
        len: file.text.len() as u64,
        sha256: file.sha256.clone(),
    })
}

/// The prefix of the name a skill is staged under beside its `SKILL.md`;
/// each writer's staged file is its own.
const STAGED: &str = "SKILL.md.writing";

/// `path` as a directory of the session's own: made when it is absent,
/// refused when a symlink or anything but a directory stands there.
/// Answers whether it was made here, so its parent can be synced.
fn own_dir(name: &str, path: &Path) -> Result<bool, HomeError> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.is_dir() => Ok(false),
        Ok(_) => Err(refused(name, "a symlink or a file stands in its path")),
        Err(error) if error.kind() == ErrorKind::NotFound => std::fs::create_dir(path)
            .map(|()| true)
            .map_err(|e| HomeError::io("creating a skill directory", path, e)),
        Err(error) => Err(HomeError::io("reading a skill directory", path, error)),
    }
}

/// Sync `dir`, so the entries made in it are durable.
fn sync_dir(dir: &Path) -> Result<(), HomeError> {
    std::fs::File::open(dir)
        .and_then(|dir| dir.sync_all())
        .map_err(|e| HomeError::io("syncing a skill directory", dir, e))
}

/// Whether a plain file with exactly `text` is already at `path`; refused
/// when anything else stands there.
fn kept(name: &str, path: &Path, text: &[u8]) -> Result<bool, HomeError> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.is_file() => {
            let bytes =
                std::fs::read(path).map_err(|e| HomeError::io("reading a kept skill", path, e))?;
            if bytes == text {
                Ok(true)
            } else {
                Err(refused(name, "a different SKILL.md is already there"))
            }
        }
        Ok(_) => Err(refused(name, "a symlink or a directory stands in its path")),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(false),
        Err(error) => Err(HomeError::io("reading a kept skill", path, error)),
    }
}

/// A text written whole and synced beside its `SKILL.md`, not yet linked.
pub(crate) struct Staged {
    written: PathBuf,
    path: PathBuf,
    text: Vec<u8>,
}

/// Write `text` whole under this writer's own staged name beside `path`
/// and sync it.
pub(crate) fn stage(path: &Path, text: &[u8]) -> Result<Staged, HomeError> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| {
            HomeError::io(
                "reading the clock to name a staged skill",
                path,
                std::io::Error::other(e),
            )
        })?
        .as_nanos();
    let staged = path.with_file_name(format!("{STAGED}-{}-{nanos}", std::process::id()));
    let mut out = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&staged)
        .map_err(|e| HomeError::io("staging a skill", &staged, e))?;
    out.write_all(text)
        .map_err(|e| HomeError::io("writing a staged skill", &staged, e))?;
    out.sync_all()
        .map_err(|e| HomeError::io("syncing a staged skill", &staged, e))?;
    Ok(Staged {
        written: staged,
        path: path.to_path_buf(),
        text: text.to_vec(),
    })
}

impl Staged {
    /// Link the staged text into place and clear the staged name. When
    /// another writer linked first, its `SKILL.md` stands: kept when its
    /// bytes are these, refused otherwise, and never replaced.
    pub(crate) fn publish(self, name: &str) -> Result<(), HomeError> {
        let linked = std::fs::hard_link(&self.written, &self.path);
        std::fs::remove_file(&self.written)
            .map_err(|e| HomeError::io("clearing a staged skill", &self.written, e))?;
        match linked {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                kept(name, &self.path, &self.text)?;
            }
            Err(error) => return Err(HomeError::io("publishing a skill", &self.path, error)),
        }
        match self.path.parent() {
            Some(dir) => sync_dir(dir),
            None => Ok(()),
        }
    }
}

/// Publish `text` at `path`: kept when a plain file with the same bytes is
/// there, refused when anything else is; otherwise staged and linked into
/// place, so `SKILL.md` never holds part of a text and is never replaced.
fn publish(name: &str, path: &Path, text: &[u8]) -> Result<(), HomeError> {
    if kept(name, path, text)? {
        return Ok(());
    }
    stage(path, text)?.publish(name)
}

/// `config_dir` rebuilt from its components, so a trailing slash cannot make
/// a symlink read as the directory it names; refused unless every part of
/// it that exists is already canonical, so no symlink above it is followed.
fn owned(config_dir: &Path) -> Result<PathBuf, HomeError> {
    let plain: PathBuf = config_dir.components().collect();
    let refused_dir = || HomeError::SkillDirectory {
        path: config_dir.to_path_buf(),
    };
    if !plain.is_absolute() || plain.components().any(|part| part == Component::ParentDir) {
        return Err(refused_dir());
    }
    let mut existing = plain.as_path();
    loop {
        match std::fs::symlink_metadata(existing) {
            Ok(_) => break,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                existing = existing.parent().ok_or_else(refused_dir)?;
            }
            Err(error) => {
                return Err(HomeError::io(
                    "reading the config directory",
                    existing,
                    error,
                ));
            }
        }
    }
    let canonical = existing
        .canonicalize()
        .map_err(|e| HomeError::io("resolving the config directory", existing, e))?;
    if canonical != existing {
        return Err(refused_dir());
    }
    Ok(plain)
}

/// Write each of `files` under `config_dir`, answering them as recorded.
/// Every file is checked before the first is written, and nothing is
/// written through a symlink at or below `config_dir`.
pub fn write(config_dir: &Path, files: &[SkillFile]) -> Result<Vec<KeptSkill>, HomeError> {
    let config_dir = owned(config_dir)?;
    let config_dir = config_dir.as_path();
    let kept = files.iter().map(check).collect::<Result<Vec<_>, _>>()?;
    for (file, skill) in files.iter().zip(&kept) {
        let skills = config_dir.join("skills");
        let own = skills.join(&skill.name);
        for dir in [config_dir, skills.as_path(), own.as_path()] {
            if own_dir(&skill.name, dir)?
                && let Some(parent) = dir.parent()
            {
                sync_dir(parent)?;
            }
        }
        publish(&skill.name, &own.join("SKILL.md"), file.text.as_bytes())?;
    }
    Ok(kept)
}
