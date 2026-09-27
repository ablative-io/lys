//! What a translation leaves as it was: a second translation gives equal bytes,
//! and the Claude Code render is unchanged by hash.

use std::error::Error;
use std::path::PathBuf;

use lys_home::cli::{Cli, Command, run};

use super::{Gate, add_beside_and_at_head, every_kind_fixture, import, sha, translate};

#[test]
fn second_translation_gives_equal_bytes() -> Gate {
    let (records, head) = every_kind_fixture();
    let (dir, home) = import(&records)?;
    add_beside_and_at_head(&home, &head)?;
    let (first_lines, first, first_path) = translate(&home, &dir.path().join("o1"))?;
    let (second_lines, mut second, second_path) = translate(&home, &dir.path().join("o2"))?;
    assert_eq!(
        serde_json::to_vec(&first_lines)?,
        serde_json::to_vec(&second_lines)?
    );
    let lost = second["lost"].as_array_mut().ok_or("no lost")?;
    let before = lost.len();
    lost.retain(|row| row["kind"] != "lys.translation");
    assert_eq!(lost.len() + 1, before);
    assert_eq!(serde_json::to_vec(&first)?, serde_json::to_vec(&second)?);
    assert_eq!(sha(&first_path)?, sha(&second_path)?);
    Ok(())
}

#[test]
fn claude_code_render_is_unchanged_by_hash() -> Gate {
    let (records, head) = every_kind_fixture();
    let (dir, home) = import(&records)?;
    add_beside_and_at_head(&home, &head)?;
    let render = |name: &str| -> Result<PathBuf, Box<dyn Error>> {
        let out = dir.path().join(name);
        run(Cli {
            command: Command::Render {
                home: home.clone(),
                session: "s1".to_owned(),
                uuid: "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa".to_owned(),
                cwd: "/w".to_owned(),
                model: "claude-fixture".to_owned(),
                out: Some(out.clone()),
                version: "2.1.281".to_owned(),
                canon: None,
            },
        })?;
        Ok(out)
    };
    let a = render("a.jsonl")?;
    translate(&home, &dir.path().join("o"))?;
    let b = render("b.jsonl")?;
    assert_eq!(sha(&a)?, sha(&b)?);
    Ok(())
}
