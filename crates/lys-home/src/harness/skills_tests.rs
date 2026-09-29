#![cfg(test)]

use std::error::Error;
use std::path::Path;

use super::skills::{SkillFile, check, write};
use crate::error::HomeError;
use crate::record::blocks::Hash;

fn skill(name: &str, text: &str) -> SkillFile {
    SkillFile {
        name: name.to_owned(),
        text: text.to_owned(),
        sha256: Hash::of(text.as_bytes()).as_str().to_owned(),
    }
}

fn refused(result: Result<impl std::fmt::Debug, HomeError>) -> String {
    match result {
        Err(HomeError::SkillRefused { reason, .. }) => reason.to_owned(),
        other => format!("not a skill refusal: {other:?}"),
    }
}

#[test]
fn two_kept_skills_are_written_where_the_harness_reads_them_twice_identical()
-> Result<(), Box<dyn Error>> {
    let config = tempfile::tempdir()?;
    let files = [
        skill("review", "Read the change against its brief.\n"),
        skill("land", "Land only what the gate passed.\n"),
    ];
    let kept = write(config.path(), &files)?;
    assert_eq!(kept, write(config.path(), &files)?);
    for (file, kept) in files.iter().zip(&kept) {
        assert_eq!(kept.path, format!("skills/{}/SKILL.md", file.name));
        assert_eq!(kept.len, file.text.len() as u64);
        assert_eq!(kept.sha256, file.sha256);
        let bytes = std::fs::read(config.path().join(&kept.path))?;
        assert_eq!(bytes, file.text.as_bytes());
    }
    Ok(())
}

#[test]
fn a_name_off_one_component_or_a_text_off_its_hash_is_refused_before_anything_is_written()
-> Result<(), Box<dyn Error>> {
    for name in ["", ".hidden", "-flag", "two/parts", "../up", "Upper"] {
        assert!(
            refused(check(&skill(name, "text\n"))).contains("one visible path component"),
            "{name}"
        );
    }
    let mut altered = skill("review", "text\n");
    altered.text.push('!');
    assert!(refused(check(&altered)).contains("does not hash"));
    let config = tempfile::tempdir()?;
    let result = write(config.path(), &[skill("review", "text\n"), altered]);
    assert!(refused(result).contains("does not hash"));
    assert!(
        !config.path().join("skills").exists(),
        "nothing was written"
    );
    Ok(())
}

#[test]
fn a_different_skill_already_there_or_a_relative_directory_is_refused() -> Result<(), Box<dyn Error>>
{
    let config = tempfile::tempdir()?;
    write(config.path(), &[skill("review", "first\n")])?;
    let again = write(config.path(), &[skill("review", "second\n")]);
    assert!(refused(again).contains("already there"));
    let relative = write(Path::new("config"), &[skill("review", "first\n")]);
    assert!(matches!(relative, Err(HomeError::SkillDirectory { .. })));
    Ok(())
}
