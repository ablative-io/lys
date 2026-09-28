//! Gates on the pinned git runner: a global configuration named by the
//! inherited environment, and one under the inherited HOME, both setting a
//! name and line-ending conversion, change neither the identity a commit
//! carries nor the bytes a file is committed with. The test process's own
//! environment and current directory are left as they were.

use std::error::Error;
use std::ffi::OsString;

use crate::home_move::git::Git;

const CONFIG: &str = "[user]\n\tname = someone\n[core]\n\tautocrlf = true\n";

#[test]
fn the_runner_ignores_inherited_configuration_and_commits_bytes_unchanged()
-> Result<(), Box<dyn Error>> {
    let cwd_before = std::env::current_dir()?;
    let env_before: Vec<(OsString, OsString)> = std::env::vars_os().collect();
    let dir = tempfile::tempdir()?;
    let user_home = dir.path().join("user-home");
    std::fs::create_dir(&user_home)?;
    std::fs::write(user_home.join(".gitconfig"), CONFIG)?;
    let global = dir.path().join("global.gitconfig");
    std::fs::write(&global, CONFIG)?;
    let path = std::env::var_os("PATH").ok_or("the test process has a PATH")?;
    let inherited = vec![
        (OsString::from("PATH"), path),
        (OsString::from("GIT_CONFIG_GLOBAL"), global.into_os_string()),
        (OsString::from("HOME"), user_home.into_os_string()),
    ];
    let work = dir.path().join("work");
    std::fs::create_dir(&work)?;
    std::fs::write(work.join("f.txt"), b"a\r\nb\n")?;
    let git = Git::new(&inherited, work.join(".git"), Some(work.clone()));
    git.init(false)?;
    let tree = git.stage_exactly(&work, &["f.txt".to_owned()])?;
    let commit = git.commit_tree(&tree, None, "fixture")?;
    let author = git.text(&[&"log", &"-1", &"--format=%an <%ae>", &commit])?;
    assert_eq!(author, "lys-home <lys-home@invalid>");
    let committer = git.text(&[&"log", &"-1", &"--format=%cn <%ce>", &commit])?;
    assert_eq!(committer, "lys-home <lys-home@invalid>");
    let blob = git.output(&[&"cat-file", &"blob", &format!("{commit}:f.txt")], None)?;
    assert_eq!(blob, b"a\r\nb\n");
    assert_eq!(std::env::current_dir()?, cwd_before);
    let env_after: Vec<(OsString, OsString)> = std::env::vars_os().collect();
    assert_eq!(env_after, env_before);
    Ok(())
}
