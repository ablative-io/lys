//! The brief's fixture home forked five times and refused five times (HOME-006
//! R7): the block store unchanged, the children's lines hash-equal, and both
//! ancestry sides on the parent.

use std::collections::BTreeSet;
use std::error::Error;
use std::path::Path;

use serde_json::Value;

use lys_home::{Entry, EntryBody, Hash, Home};

use super::{Gate, PARENT, SENTINELS, fixture_home, lys_home, one_object, refused};

/// The count of files and the total bytes under a directory, recursively.
fn files_and_bytes(dir: &Path) -> Result<(u64, u64), Box<dyn Error>> {
    let mut files = 0;
    let mut bytes = 0;
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            let (f, b) = files_and_bytes(&entry.path())?;
            files += f;
            bytes += b;
        } else {
            files += 1;
            bytes += entry.metadata()?.len();
        }
    }
    Ok((files, bytes))
}

/// The SHA-256 of each line of a session file after its header.
fn line_hashes(file: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    Ok(std::fs::read(file)?
        .split_inclusive(|byte| *byte == b'\n')
        .skip(1)
        .map(|line| Hash::of(line).to_string())
        .collect())
}

/// One fork through the binary, returning its report.
fn fork_ok(home: &str, args: &[&str]) -> Result<Value, Box<dyn Error>> {
    let mut all = vec!["fork", "--home", home];
    all.extend_from_slice(args);
    let report = one_object(&lys_home(&all)?)?;
    assert_eq!(report["command"], "fork");
    Ok(report["report"].clone())
}

#[test]
fn five_forks_succeed_and_five_are_refused_with_the_ancestry_on_the_parent() -> Gate {
    let dir = tempfile::tempdir()?;
    let (home, lanterns) = fixture_home(dir.path())?;
    let home_arg = home.to_str().ok_or("a UTF-8 path")?;
    let blocks = home.join("blocks");
    let store_before = files_and_bytes(&blocks)?;
    let mut children: Vec<String> = Vec::new();

    let first = fork_ok(home_arg, &["--lantern", &lanterns.l5])?;
    let second = fork_ok(home_arg, &["--lantern", &lanterns.l5])?;
    let first_child = first["child"].as_str().ok_or("a child")?.to_owned();
    let second_child = second["child"].as_str().ok_or("a child")?.to_owned();
    assert_ne!(first_child, second_child);
    assert_eq!(files_and_bytes(&blocks)?, store_before);
    let first_lines = line_hashes(&home.join("sessions").join(format!("{first_child}.jsonl")))?;
    let second_lines = line_hashes(&home.join("sessions").join(format!("{second_child}.jsonl")))?;
    assert_eq!(first_lines.len(), 8);
    assert_eq!(first_lines[..7], second_lines[..7]);
    assert_ne!(first_lines[7], second_lines[7]);
    children.push(first_child.clone());
    children.push(second_child);

    let carried = fork_ok(home_arg, &["--lantern", &lanterns.l6])?;
    assert_eq!(carried["cut_at"], "e5");
    assert_eq!(carried["point"], "e6");
    assert_ne!(carried["cut_at"], carried["point"]);
    assert_eq!(carried["coordinate_carried"], true);
    children.push(carried["child"].as_str().ok_or("a child")?.to_owned());

    let lit_in = fork_ok(home_arg, &["--lantern", &lanterns.l2])?;
    assert_eq!(lit_in["parent"], PARENT);
    children.push(lit_in["child"].as_str().ok_or("a child")?.to_owned());

    let named = fork_ok(home_arg, &["--lantern", &lanterns.o2, "--session", PARENT])?;
    assert_eq!(named["parent"], PARENT);
    children.push(named["child"].as_str().ok_or("a child")?.to_owned());
    assert_eq!(children.len(), 5);
    assert_eq!(children.iter().collect::<BTreeSet<_>>().len(), 5);

    let mut refusals = 0;
    for (args, id, word) in [
        (
            vec!["--lantern", "no-such-lantern"],
            "no-such-lantern",
            None,
        ),
        (vec!["--lantern", "e5"], "e5", None),
        (
            vec!["--lantern", &lanterns.l1],
            lanterns.l1.as_str(),
            Some("nothing_to_fork"),
        ),
        (
            vec!["--lantern", &lanterns.o2],
            lanterns.o2.as_str(),
            Some("lantern_ambiguous"),
        ),
        (
            vec!["--lantern", &lanterns.l2, "--session", &first_child],
            lanterns.l2.as_str(),
            Some("lantern_not_lit_here"),
        ),
    ] {
        let mut all = vec!["fork", "--home", home_arg];
        all.extend_from_slice(&args);
        let stderr = refused(&lys_home(&all)?, 1)?;
        assert!(stderr.contains(id), "{stderr}");
        if let Some(word) = word {
            assert!(stderr.contains(word), "{stderr}");
        }
        for sentinel in SENTINELS {
            assert!(!stderr.contains(sentinel));
        }
        refusals += 1;
    }
    assert_eq!(refusals, 5);
    assert_eq!(files_and_bytes(&blocks)?, store_before);

    let parent_home = Home::open(&home)?;
    let marks = parent_home
        .read_session(PARENT)?
        .customs_everywhere("lys.fork")?;
    assert_eq!(marks.len(), 5);
    let named_children: Vec<String> = marks
        .iter()
        .map(|mark| match &mark.body {
            EntryBody::Custom {
                data: Some(data), ..
            } => Ok(data["child"].as_str().ok_or("a child")?.to_owned()),
            _ => Err("not a custom entry".into()),
        })
        .collect::<Result<_, Box<dyn Error>>>()?;
    assert_eq!(named_children, children);
    let head = parent_home.open_session(PARENT)?.head()?.map(str::to_owned);
    assert_eq!(head.as_deref(), marks.last().map(Entry::id));
    Ok(())
}
