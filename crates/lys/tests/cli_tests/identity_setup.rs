//! First-run setup from the install's side (DIRECTORY-047 R1): nothing about
//! a person is taken from the machine, the issuer's own bootstrap account
//! names no person, and the one-time setup code is never printed, only kept
//! as its digest, and written to an owner-only file when no browser can take
//! it.
//!
//! The deployment configuration is written here from the shipped template by
//! hand, apart from the install that renders it in use.

use std::error::Error;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Output};

use sha2::{Digest, Sha256};

type TestResult = Result<(), Box<dyn Error>>;

const TEMPLATE: &str = include_str!("../../src/identity/install/deployment.template.toml");

/// What every email-like place on the machine is set to.
const MARKER: &str = "machine-marker-7f3a@example.test";

/// Every variable a tool might read a person's email from.
const EMAIL_VARIABLES: [&str; 8] = [
    "EMAIL",
    "MAIL",
    "DEBEMAIL",
    "USER_EMAIL",
    "GIT_AUTHOR_EMAIL",
    "GIT_COMMITTER_EMAIL",
    "LYS_ADMIN_EMAIL",
    "LYS_IDENTITY_ADMIN_EMAIL",
];

fn write_deployment(root: &Path) -> TestResult {
    let text = TEMPLATE
        .replace("{{admin_email_line}}", "")
        .replace("{{rauthy_port}}", "18080")
        .replace("{{service_port}}", "8490");
    std::fs::write(root.join("deployment.toml"), text)?;
    Ok(())
}

/// Run `lys` with every email-like place on the machine set to the marker:
/// the variables, and git's global and home configuration. `PATH` names an
/// empty directory, so no browser opener is found.
fn run_marked(home: &Path, args: &[&str]) -> Result<Output, Box<dyn Error>> {
    let gitconfig = home.join(".gitconfig");
    std::fs::write(
        &gitconfig,
        format!("[user]\n\temail = {MARKER}\n\tname = Marker\n"),
    )?;
    let empty = home.join("empty-path");
    std::fs::create_dir_all(&empty)?;
    let mut command = Command::new(env!("CARGO_BIN_EXE_lys"));
    command
        .args(args)
        .env("HOME", home)
        .env("GIT_CONFIG_GLOBAL", &gitconfig)
        .env("PATH", &empty);
    for variable in EMAIL_VARIABLES {
        command.env(variable, MARKER);
    }
    Ok(command.output()?)
}

/// Files by path, each with its bytes.
type Files = Vec<(String, Vec<u8>)>;

/// Every file under `dir`, with its bytes.
fn files(dir: &Path) -> Result<Files, Box<dyn Error>> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            found.extend(files(&path)?);
        } else {
            found.push((path.display().to_string(), std::fs::read(&path)?));
        }
    }
    Ok(found)
}

fn contains(haystack: &[u8], needle: &str) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle.as_bytes())
}

#[test]
fn nothing_about_a_person_is_taken_from_the_machine() -> TestResult {
    let root = tempfile::TempDir::new()?;
    let home = tempfile::TempDir::new()?;
    write_deployment(root.path())?;
    let config = root.path().join("deployment.toml").display().to_string();
    let prepared = run_marked(home.path(), &["identity", "prepare", "--config", &config])?;
    assert!(prepared.status.success(), "{prepared:?}");
    let coded = run_marked(
        home.path(),
        &[
            "identity",
            "setup-code",
            "--root",
            &root.path().display().to_string(),
        ],
    )?;
    assert!(coded.status.success(), "{coded:?}");

    let written = files(root.path())?;
    assert!(
        written.len() >= 10,
        "state, deployment.toml and the code were written: {written:?}"
    );
    for (path, bytes) in &written {
        assert!(!contains(bytes, MARKER), "{path} holds the machine's email");
    }
    for output in [&prepared, &coded] {
        assert!(!contains(&output.stdout, MARKER));
        assert!(!contains(&output.stderr, MARKER));
    }
    let deployment = std::fs::read_to_string(root.path().join("deployment.toml"))?;
    assert!(
        !deployment.contains("admin_email"),
        "no administrator is named"
    );

    let environment = std::fs::read_to_string(root.path().join("state").join("compose.env"))?;
    let bootstrap: Vec<&str> = environment
        .lines()
        .filter(|line| line.starts_with("RAUTHY_BOOTSTRAP_ADMIN_EMAIL="))
        .collect();
    assert_eq!(
        bootstrap,
        ["RAUTHY_BOOTSTRAP_ADMIN_EMAIL=bootstrap@machine.lys.test"],
        "the issuer's own account is a machine account naming no person"
    );
    Ok(())
}

#[test]
fn the_setup_code_is_never_printed_and_is_kept_only_as_its_digest() -> TestResult {
    let root = tempfile::TempDir::new()?;
    let home = tempfile::TempDir::new()?;
    write_deployment(root.path())?;
    let config = root.path().join("deployment.toml").display().to_string();
    let prepared = run_marked(home.path(), &["identity", "prepare", "--config", &config])?;
    assert!(prepared.status.success(), "{prepared:?}");
    let coded = run_marked(
        home.path(),
        &[
            "identity",
            "setup-code",
            "--root",
            &root.path().display().to_string(),
        ],
    )?;
    assert!(coded.status.success(), "{coded:?}");

    let headless = root.path().join("setup-code");
    let code = std::fs::read_to_string(&headless)?;
    assert_eq!(code.len(), 32, "a code of 32 letters and digits");
    assert!(code.bytes().all(|byte| byte.is_ascii_alphanumeric()));
    let mode = std::fs::metadata(&headless)?.permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "only the owner reads the headless code");

    for output in [&prepared, &coded] {
        assert!(
            !contains(&output.stdout, &code),
            "the code is never printed"
        );
        assert!(
            !contains(&output.stderr, &code),
            "the code is never printed"
        );
    }
    let said = String::from_utf8(coded.stdout)?;
    let naming: Vec<&str> = said
        .lines()
        .filter(|line| line.contains(&headless.display().to_string()))
        .collect();
    assert_eq!(naming.len(), 1, "one line names the code's file: {said}");
    assert!(naming[0].contains("only you can read it"), "{said}");
    assert!(
        naming[0].contains("enter it in the Setup code field"),
        "{said}"
    );
    assert!(said.contains("http://localhost:8490/setup"), "{said}");

    let pending: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(
        root.path().join("state").join("setup-code"),
    )?)?;
    let digest: Vec<String> = Sha256::digest(code.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let digest = digest.concat();
    assert_eq!(pending["purpose"], "first-run");
    assert_eq!(pending["sha256"], digest.as_str());
    assert!(
        !std::fs::read_to_string(root.path().join("state").join("setup-code"))?.contains(&code),
        "the state keeps the digest, never the code"
    );
    Ok(())
}

#[test]
fn an_admin_email_that_is_not_an_email_is_refused() -> TestResult {
    let home = tempfile::TempDir::new()?;
    let refused = run_marked(
        home.path(),
        &["identity", "install", "--admin-email", "not an email"],
    )?;
    assert!(!refused.status.success());
    assert!(String::from_utf8(refused.stderr)?.contains("is not an email address"));
    Ok(())
}

#[test]
fn setup_over_ssh_writes_the_readable_code_without_opening_a_browser() -> TestResult {
    let root = tempfile::tempdir()?;
    let tools = tempfile::tempdir()?;
    write_deployment(root.path())?;
    let state = root.path().join("state");
    std::fs::create_dir(&state)?;
    std::fs::set_permissions(&state, std::fs::Permissions::from_mode(0o700))?;
    let opened = tools.path().join("opened");
    for name in ["open", "xdg-open"] {
        let opener = tools.path().join(name);
        std::fs::write(
            &opener,
            "#!/bin/sh\n: >\"$LYS_SETUP_TEST_OPENED\"\nexit 0\n",
        )?;
        std::fs::set_permissions(&opener, std::fs::Permissions::from_mode(0o700))?;
    }
    let output = Command::new(env!("CARGO_BIN_EXE_lys"))
        .args(["--json", "identity", "setup-code", "--root"])
        .arg(root.path())
        .env("PATH", tools.path())
        .env("SSH_CONNECTION", "127.0.0.1 12345 127.0.0.1 22")
        .env("LYS_SETUP_TEST_OPENED", &opened)
        .output()?;
    assert!(output.status.success(), "setup-code failed");
    assert!(!opened.exists(), "SSH setup opened a desktop browser");
    let path = root.path().join("setup-code");
    let code = std::fs::read_to_string(&path)?;
    assert_eq!(code.len(), 32);
    assert!(code.bytes().all(|byte| byte.is_ascii_alphanumeric()));
    assert_eq!(
        std::fs::metadata(&path)?.permissions().mode() & 0o777,
        0o600
    );
    let answer: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(answer["setup_code_file"], path.display().to_string());
    assert!(
        !answer.to_string().contains(&code),
        "setup-code printed its code"
    );
    assert!(
        !output
            .stderr
            .windows(code.len())
            .any(|window| window == code.as_bytes()),
        "setup-code printed its code on stderr"
    );
    let pending: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.path().join("state/setup-code"))?)?;
    assert_eq!(pending["purpose"], "first-run");
    assert_eq!(
        pending["sha256"],
        format!("{:x}", Sha256::digest(code.as_bytes()))
    );
    Ok(())
}

#[test]
fn a_readable_setup_file_is_reported_by_its_absolute_path() -> TestResult {
    let root = tempfile::tempdir()?;
    let tools = tempfile::tempdir()?;
    write_deployment(root.path())?;
    let state = root.path().join("state");
    std::fs::create_dir(&state)?;
    std::fs::set_permissions(&state, std::fs::Permissions::from_mode(0o700))?;
    for name in ["open", "xdg-open"] {
        let opener = tools.path().join(name);
        std::fs::write(&opener, "#!/bin/sh\nexit 1\n")?;
        std::fs::set_permissions(&opener, std::fs::Permissions::from_mode(0o700))?;
    }
    let output = Command::new(env!("CARGO_BIN_EXE_lys"))
        .args(["identity", "setup-code", "--root", "."])
        .current_dir(root.path())
        .env("PATH", tools.path())
        .env("SSH_CONNECTION", "127.0.0.1 12345 127.0.0.1 22")
        .output()?;
    assert!(output.status.success(), "setup-code failed");
    let path = root.path().join("setup-code");
    let named = format!("the setup code is in {}", path.display());
    assert!(
        String::from_utf8_lossy(&output.stdout).contains(&named),
        "the readable setup code's absolute path was not printed"
    );
    let code = std::fs::read(&path)?;
    assert!(
        !output
            .stdout
            .windows(code.len())
            .any(|window| window == code),
        "the code itself was printed"
    );
    Ok(())
}
