//! Kept skills written into a session's own config directory (HOME-037 R3).
//!
//! A skill arrives as its name, its text and the SHA-256 Lys keeps it by. It
//! is checked before anything is written: the name is one visible path
//! component and the text hashes to what Lys recorded. Each is written at
//! `skills/<name>/SKILL.md` under the config directory the session is given,
//! which must be absolute; a file already there is kept only when its bytes
//! are the same. Nothing is written anywhere else.

use std::io::Write;
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

/// Write each of `files` under `config_dir`, answering them as recorded.
/// Every file is checked before the first is written.
pub fn write(config_dir: &Path, files: &[SkillFile]) -> Result<Vec<KeptSkill>, HomeError> {
    if !config_dir.is_absolute() {
        return Err(HomeError::SkillDirectory {
            path: config_dir.to_path_buf(),
        });
    }
    let kept = files.iter().map(check).collect::<Result<Vec<_>, _>>()?;
    for (file, skill) in files.iter().zip(&kept) {
        let path = config_dir.join(&skill.path);
        match std::fs::read(&path) {
            Ok(bytes) if bytes == file.text.as_bytes() => continue,
            Ok(_) => {
                return Err(refused(
                    &skill.name,
                    "a different SKILL.md is already there",
                ));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(HomeError::io("reading a kept skill", &path, error)),
        }
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)
                .map_err(|e| HomeError::io("creating a skill directory", dir, e))?;
        }
        let mut out = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)
            .map_err(|e| HomeError::io("creating a skill", &path, e))?;
        out.write_all(file.text.as_bytes())
            .map_err(|e| HomeError::io("writing a skill", &path, e))?;
        out.sync_all()
            .map_err(|e| HomeError::io("syncing a skill", &path, e))?;
    }
    Ok(kept)
}
