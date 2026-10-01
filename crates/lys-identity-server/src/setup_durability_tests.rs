use std::error::Error;
use std::os::unix::fs::PermissionsExt;

use lys_identity::LoginBinding;

use super::{read_administrator, record_administrator};

#[test]
fn recording_an_administrator_preserves_the_previous_file_until_publication()
-> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("administrator.json");
    let previous = LoginBinding::new("https://issuer.example.test", "previous")?;
    let next = LoginBinding::new("https://issuer.example.test", "next")?;
    record_administrator(&path, &previous)?;
    let held = directory.path().join("previous.json");
    std::fs::hard_link(&path, &held)?;
    record_administrator(&path, &next)?;
    assert_eq!(read_administrator(&path)?, Some(next));
    assert_eq!(read_administrator(&held)?, Some(previous));
    Ok(())
}

#[test]
fn a_recorded_administrator_is_private() -> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("administrator.json");
    std::fs::write(&path, b"old file")?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644))?;
    record_administrator(
        &path,
        &LoginBinding::new("https://issuer.example.test", "administrator")?,
    )?;
    assert_eq!(
        std::fs::metadata(&path)?.permissions().mode() & 0o777,
        0o600
    );
    Ok(())
}

#[test]
fn administrator_write_child() -> Result<(), Box<dyn Error>> {
    use std::io::{Read, Write};
    let Some(path) = std::env::var_os("LYS_ADMINISTRATOR_WRITE_CHILD") else {
        return Ok(());
    };
    let login = LoginBinding::new("https://issuer.example.test", "next")?;
    let file = super::prepare_administrator(std::path::Path::new(&path), &login)?;
    println!("administrator-write-prepared");
    std::io::stdout().flush()?;
    std::io::stdin().read_exact(&mut [0u8; 1])?;
    drop(file);
    Err("the parent must kill the prepared writer".into())
}

#[test]
fn a_killed_writer_leaves_the_old_administrator_readable() -> Result<(), Box<dyn Error>> {
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("administrator.json");
    let previous = LoginBinding::new("https://issuer.example.test", "previous")?;
    record_administrator(&path, &previous)?;
    let mut child = Command::new(std::env::current_exe()?)
        .args([
            "--exact",
            "setup::durability_tests::administrator_write_child",
            "--nocapture",
        ])
        .env("LYS_ADMINISTRATOR_WRITE_CHILD", &path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    let observed = (|| -> Result<(), Box<dyn Error>> {
        let output = child.stdout.take().ok_or("writer stdout missing")?;
        for line in BufReader::new(output).lines() {
            if line? == "administrator-write-prepared" {
                if read_administrator(&path)? != Some(previous.clone()) {
                    return Err("prepared writer changed the published administrator".into());
                }
                return Ok(());
            }
        }
        Err("writer exited before its prepare signal".into())
    })();
    if child.try_wait()?.is_none() {
        child.kill()?;
    }
    let exited = child.wait()?;
    observed?;
    assert!(!exited.success());
    assert_eq!(read_administrator(&path)?, Some(previous));
    Ok(())
}
