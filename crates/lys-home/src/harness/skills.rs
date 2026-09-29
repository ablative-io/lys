//! Kept skills written into a session's own config directory (HOME-037 R3).
//!
//! A skill arrives as its name, its text and the SHA-256 Lys keeps it by. It
//! is checked before anything is written: the name is one visible path
//! component and the text hashes to what Lys recorded. Each is written at
//! `skills/<name>/SKILL.md` under the config directory the session is given,
//! which must be absolute; a file already there is kept only when its bytes
//! are the same. Nothing is written anywhere else.

use std::io::{ErrorKind, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::HomeError;
use crate::harness::launch_fields::KeptSkill;
use crate::record::blocks::Hash;

/// The longest skill name.
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

/// The name a skill is staged under beside its `SKILL.md` before it is
/// published.
const STAGED: &str = "SKILL.md.writing";

/// `path` as a directory of the session's own: made when it is absent,
/// refused when a symlink or anything but a directory stands there.
fn own_dir(name: &str, path: &Path) -> Result<(), HomeError> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.is_dir() => Ok(()),
        Ok(_) => Err(refused(name, "a symlink or a file stands in its path")),
        Err(error) if error.kind() == ErrorKind::NotFound => std::fs::create_dir(path)
            .map_err(|e| HomeError::io("creating a skill directory", path, e)),
        Err(error) => Err(HomeError::io("reading a skill directory", path, error)),
    }
}

/// Publish `text` at `path`: kept when a plain file with the same bytes is
/// there, refused when anything else is; otherwise written whole and synced
/// beside it, then linked into place, so `SKILL.md` never holds part of a
/// text and is never replaced.
fn publish(name: &str, path: &Path, text: &[u8]) -> Result<(), HomeError> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.is_file() => {
            let bytes =
                std::fs::read(path).map_err(|e| HomeError::io("reading a kept skill", path, e))?;
            return if bytes == text {
                Ok(())
            } else {
                Err(refused(name, "a different SKILL.md is already there"))
            };
        }
        Ok(_) => return Err(refused(name, "a symlink or a directory stands in its path")),
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Err(error) => return Err(HomeError::io("reading a kept skill", path, error)),
    }
    let staged = path.with_file_name(STAGED);
    match std::fs::remove_file(&staged) {
        Err(error) if error.kind() != ErrorKind::NotFound => {
            return Err(HomeError::io("clearing a staged skill", &staged, error));
        }
        _ => {}
    }
    let mut out = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&staged)
        .map_err(|e| HomeError::io("staging a skill", &staged, e))?;
    out.write_all(text)
        .map_err(|e| HomeError::io("writing a staged skill", &staged, e))?;
    out.sync_all()
        .map_err(|e| HomeError::io("syncing a staged skill", &staged, e))?;
    drop(out);
    std::fs::hard_link(&staged, path).map_err(|e| HomeError::io("publishing a skill", path, e))?;
    std::fs::remove_file(&staged).map_err(|e| HomeError::io("clearing a staged skill", &staged, e))
}

/// Write each of `files` under `config_dir`, answering them as recorded.
/// Every file is checked before the first is written, and nothing is
/// written through a symlink at or below `config_dir`.
pub fn write(config_dir: &Path, files: &[SkillFile]) -> Result<Vec<KeptSkill>, HomeError> {
    if !config_dir.is_absolute() {
        return Err(HomeError::SkillDirectory {
            path: config_dir.to_path_buf(),
        });
    }
    let kept = files.iter().map(check).collect::<Result<Vec<_>, _>>()?;
    for (file, skill) in files.iter().zip(&kept) {
        let skills = config_dir.join("skills");
        let own = skills.join(&skill.name);
        for dir in [config_dir, skills.as_path(), own.as_path()] {
            own_dir(&skill.name, dir)?;
        }
        publish(&skill.name, &own.join("SKILL.md"), file.text.as_bytes())?;
    }
    Ok(kept)
}
