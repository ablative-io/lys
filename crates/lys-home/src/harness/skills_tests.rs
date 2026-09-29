#![cfg(test)]

use std::error::Error;
use std::path::Path;

use super::skills::{SkillFile, check, write};
use crate::error::HomeError;
use crate::record::blocks::Hash;

/// A fresh config directory, named by its canonical path.
fn config_dir() -> Result<(tempfile::TempDir, std::path::PathBuf), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().canonicalize()?;
    Ok((dir, path))
}

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
    let (_config, config) = config_dir()?;
    let files = [
        skill("review", "Read the change against its brief.\n"),
        skill("land", "Land only what the gate passed.\n"),
    ];
    let kept = write(config.as_path(), &files)?;
    assert_eq!(kept, write(config.as_path(), &files)?);
    for (file, kept) in files.iter().zip(&kept) {
        assert_eq!(kept.path, format!("skills/{}/SKILL.md", file.name));
        assert_eq!(kept.len, file.text.len() as u64);
        assert_eq!(kept.sha256, file.sha256);
        let bytes = std::fs::read(config.as_path().join(&kept.path))?;
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
    let (_config, config) = config_dir()?;
    let result = write(config.as_path(), &[skill("review", "text\n"), altered]);
    assert!(refused(result).contains("does not hash"));
    assert!(
        !config.as_path().join("skills").exists(),
        "nothing was written"
    );
    Ok(())
}

#[test]
fn a_different_skill_already_there_or_a_relative_directory_is_refused() -> Result<(), Box<dyn Error>>
{
    let (_config, config) = config_dir()?;
    write(config.as_path(), &[skill("review", "first\n")])?;
    let again = write(config.as_path(), &[skill("review", "second\n")]);
    assert!(refused(again).contains("already there"));
    let relative = write(Path::new("config"), &[skill("review", "first\n")]);
    assert!(matches!(relative, Err(HomeError::SkillDirectory { .. })));
    Ok(())
}

#[test]
fn a_symlink_below_the_config_directory_is_refused_and_nothing_is_written_through_it()
-> Result<(), Box<dyn Error>> {
    let outside = tempfile::tempdir()?;
    let (_config, config) = config_dir()?;
    std::os::unix::fs::symlink(outside.path(), config.as_path().join("skills"))?;
    let through = write(config.as_path(), &[skill("review", "text\n")]);
    assert!(refused(through).contains("symlink"));
    assert!(
        !outside.path().join("review").exists(),
        "nothing left the config directory"
    );

    let (_config, config) = config_dir()?;
    let aside = outside.path().join("SKILL.md");
    std::fs::write(&aside, "text\n")?;
    std::fs::create_dir_all(config.as_path().join("skills/review"))?;
    std::os::unix::fs::symlink(&aside, config.as_path().join("skills/review/SKILL.md"))?;
    let same = write(config.as_path(), &[skill("review", "text\n")]);
    assert!(
        refused(same).contains("symlink"),
        "a same-byte link is still refused"
    );
    Ok(())
}

#[test]
fn a_write_cut_short_is_retried_whole_and_never_leaves_part_of_a_text() -> Result<(), Box<dyn Error>>
{
    let (_config, config) = config_dir()?;
    let own = config.as_path().join("skills/review");
    std::fs::create_dir_all(&own)?;
    std::fs::write(own.join("SKILL.md.writing"), "Read the chan")?;
    let text = "Read the change against its brief.\n";
    write(config.as_path(), &[skill("review", text)])?;
    assert_eq!(std::fs::read(own.join("SKILL.md"))?, text.as_bytes());
    Ok(())
}

#[test]
fn a_config_path_through_a_symlink_is_refused_with_or_without_a_trailing_slash()
-> Result<(), Box<dyn Error>> {
    let (_real, real) = config_dir()?;
    let (_holder, holder) = config_dir()?;
    let link = holder.join("config");
    std::os::unix::fs::symlink(&real, &link)?;
    for path in [link.clone(), holder.join("config/"), link.join("below")] {
        let result = write(&path, &[skill("review", "text\n")]);
        assert!(
            matches!(result, Err(HomeError::SkillDirectory { .. })),
            "{}",
            path.display()
        );
    }
    assert!(
        !real.join("skills").exists(),
        "nothing was written through the link"
    );
    Ok(())
}
