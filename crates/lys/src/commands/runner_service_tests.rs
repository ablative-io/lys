#![cfg(test)]

use super::*;
use crate::cli::{Cli, Command as CliCommand, RunnerCommand};
use clap::Parser;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn mac_text(program: &Path, log: &Path, root: &Path) -> CliResult<String> {
    plist(program, log, root, "person", Path::new("/bin/zsh"))
}

fn linux_text(program: &Path, log: &Path, root: &Path) -> CliResult<String> {
    unit(program, log, root, "person", Path::new("/bin/bash"))
}

fn connection() -> SavedJoin {
    SavedJoin {
        server: "https://lys.example.test/api".to_owned(),
        machine: "op-0123456789abcdef0123456789abcdef".to_owned(),
        server_ca: None,
    }
}

#[test]
fn launchd_text_has_the_exact_login_arguments_and_log_destinations() -> TestResult {
    let actual = mac_text(
        Path::new("/Users/person/Lys/bin/runner-service/lys"),
        Path::new("/Users/person/Lys/logs/runner-service.log"),
        Path::new("/Users/person/Lys"),
    )?;
    assert_eq!(
        actual,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>au.com.ablative.lys-runner</string>
  <key>ProgramArguments</key>
  <array><string>/Users/person/Lys/bin/runner-service/lys</string><string>runner</string><string>start</string></array>
  <key>RunAtLoad</key><true/>
  <key>KeepAlive</key><true/>
  <key>EnvironmentVariables</key>
  <dict>
    <key>LYS_IDENTITY_HOME</key><string>/Users/person/Lys</string>
    <key>SHELL</key><string>/bin/zsh</string>
    <key>USER</key><string>person</string>
    <key>LOGNAME</key><string>person</string>
  </dict>
  <key>StandardOutPath</key><string>/Users/person/Lys/logs/runner-service.log</string>
  <key>StandardErrorPath</key><string>/Users/person/Lys/logs/runner-service.log</string>
</dict>
</plist>
"#
    );
    Ok(())
}

#[test]
fn systemd_text_has_the_exact_prestart_update_and_restart_policy() -> TestResult {
    let actual = linux_text(
        Path::new("/home/person/Lys/bin/runner-service/lys"),
        Path::new("/home/person/Lys/logs/runner-service.log"),
        Path::new("/home/person/Lys"),
    )?;
    assert_eq!(
        actual,
        r#"[Unit]
Description=Lys runner
After=network-online.target
Wants=network-online.target
StartLimitIntervalSec=0

[Service]
Type=simple
Environment="LYS_IDENTITY_HOME=/home/person/Lys"
Environment="SHELL=/bin/bash"
Environment="USER=person"
Environment="LOGNAME=person"
Environment=LYS_RUNNER_SERVICE=1
ExecStartPre=-"/home/person/Lys/bin/runner-service/lys" runner update
ExecStart="/home/person/Lys/bin/runner-service/lys" runner start
Restart=always
StandardOutput=append:/home/person/Lys/logs/runner-service.log
StandardError=append:/home/person/Lys/logs/runner-service.log

[Install]
WantedBy=default.target
"#
    );
    Ok(())
}

#[test]
fn service_values_escape_xml_and_systemd_expansions() -> TestResult {
    let special = Path::new("/home/A & B/<lys>\"'%/$lys");
    let mac = mac_text(special, special, special)?;
    assert!(mac.contains("A &amp; B/&lt;lys&gt;&quot;&apos;%/$lys"));
    let linux = linux_text(special, special, special)?;
    assert!(linux.contains(r#"ExecStart="/home/A & B/<lys>\"'%%/$$lys" runner start"#));
    for path in [
        "/home/person/lys\nExecStart=/bad",
        "/home/person/lys\r",
        "/home/person/lys\0",
    ] {
        assert!(mac_text(Path::new(path), special, special).is_err());
        assert!(linux_text(Path::new(path), special, special).is_err());
    }
    Ok(())
}

#[test]
fn login_shell_is_read_from_each_platforms_account_record() -> TestResult {
    assert_eq!(
        shell_from_record(Platform::Mac, "person", "UserShell: /bin/zsh\n")?,
        Path::new("/bin/zsh")
    );
    assert_eq!(
        shell_from_record(
            Platform::Linux,
            "person",
            "person:x:1000:1000:Person:/home/person:/bin/bash\n"
        )?,
        Path::new("/bin/bash")
    );
    Ok(())
}

#[test]
fn missing_or_relative_account_shells_are_refused_in_words() {
    for (platform, record) in [
        (Platform::Mac, ""),
        (Platform::Mac, "UserShell: zsh"),
        (Platform::Mac, "UserShell: /bin/zsh\nUserShell: /bin/sh"),
        (Platform::Linux, "person:x:1000:1000:Person:/home/person:"),
        (
            Platform::Linux,
            "person:x:1000:1000:Person:/home/person:bash",
        ),
        (
            Platform::Linux,
            "another:x:1000:1000:Person:/home/person:/bin/bash",
        ),
    ] {
        let error = shell_from_record(platform, "person", record)
            .expect_err("invalid account shell accepted");
        assert!(error.to_string().contains("runner_service_shell_unknown"));
        assert!(error.to_string().contains("absolute login shell"));
    }
}

#[test]
fn saved_connection_round_trips_without_a_code_and_is_owner_only() -> TestResult {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir()?;
    let layout = Layout::at(root.path().to_path_buf());
    let saved = connection();
    saved.save(&layout)?;
    assert_eq!(SavedJoin::read(&layout)?, saved);
    let path = layout.run_dir().join("runner-join.json");
    assert_eq!(
        std::fs::metadata(&path)?.permissions().mode() & 0o777,
        0o600
    );
    let wire: serde_json::Value = serde_json::from_slice(&std::fs::read(path)?)?;
    assert_eq!(
        wire.as_object()
            .ok_or("join record is not an object")?
            .len(),
        3
    );
    assert!(wire.get("code").is_none());
    assert!(wire["server_ca"].is_null());
    Ok(())
}

#[test]
fn saved_connection_round_trips_an_absolute_certificate_authority() -> TestResult {
    let root = tempfile::tempdir()?;
    let layout = Layout::at(root.path().to_path_buf());
    let authority = root.path().join("authority.pem");
    std::fs::write(&authority, "fixture certificate")?;
    let mut saved = connection();
    saved.server_ca = Some(authority);
    saved.save(&layout)?;
    assert_eq!(SavedJoin::read(&layout)?, saved);
    Ok(())
}

fn pending_at(path: &Path) -> CliResult<()> {
    private_files::write(path, br#"{"server":"https://lys.example.test/api","machine":"op-example","code":"fixture-code"}"#)?;
    Ok(())
}

#[test]
fn pending_is_read_from_the_supplied_path_and_deleted_after_join() -> TestResult {
    let root = tempfile::tempdir()?;
    let package = tempfile::tempdir()?;
    let layout = Layout::at(root.path().to_path_buf());
    let path = package.path().join("runner-pending.json");
    pending_at(&path)?;
    let mut joined = false;
    consume_pending(&layout, &path, &mut |_, saved, code| {
        assert!(path.is_file());
        assert_eq!(saved.machine, "op-example");
        assert_eq!(code, "fixture-code");
        joined = true;
        Ok(())
    })?;
    assert!(joined);
    assert!(!path.exists());
    assert_eq!(SavedJoin::read(&layout)?.machine, "op-example");
    Ok(())
}

#[test]
fn refused_join_keeps_the_pending_file_and_does_not_save_a_connection() -> TestResult {
    let root = tempfile::tempdir()?;
    let layout = Layout::at(root.path().to_path_buf());
    let path = root.path().join("pending.json");
    pending_at(&path)?;
    let result = consume_pending(&layout, &path, &mut |_, _, _| {
        Err(refused(
            "runner_join_refused",
            "the computer was not admitted",
        ))
    });
    assert!(result.is_err());
    assert!(path.is_file());
    assert!(!layout.run_dir().join("runner-join.json").exists());
    Ok(())
}

#[test]
fn missing_and_open_mode_pending_files_have_the_named_unreadable_refusal() -> TestResult {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir()?;
    let layout = Layout::at(root.path().to_path_buf());
    for name in ["missing.json", "open.json"] {
        let path = root.path().join(name);
        if name == "open.json" {
            pending_at(&path)?;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644))?;
        }
        let error = consume_pending(&layout, &path, &mut |_, _, _| {
            panic!("unreadable pending file reached admission")
        })
        .expect_err("unreadable pending file accepted");
        assert!(error.to_string().contains("runner_pending_unreadable"));
    }
    Ok(())
}

#[test]
fn malformed_pending_errors_never_echo_the_code_value() -> TestResult {
    let root = tempfile::tempdir()?;
    let layout = Layout::at(root.path().to_path_buf());
    let path = root.path().join("pending.json");
    private_files::write(&path, br#"{"server":"https://lys.example.test/api","machine":[],"code":"fixture-private-marker"}"#)?;
    let error = consume_pending(&layout, &path, &mut |_, _, _| {
        panic!("malformed pending file reached admission")
    })
    .expect_err("malformed pending file accepted");
    assert!(error.to_string().contains("runner_pending_invalid"));
    assert!(!error.to_string().contains("fixture-private-marker"));
    assert!(path.exists());
    Ok(())
}

#[test]
fn pending_cli_requires_a_path_and_rejects_connection_arguments() -> TestResult {
    let cli = Cli::try_parse_from([
        "lys",
        "runner",
        "install",
        "--pending",
        "/usr/lib/lys/runner-pending.json",
    ])?;
    assert!(
        matches!(cli.command, CliCommand::Runner(RunnerCommand::Install { pending: Some(path), .. }) if path == Path::new("/usr/lib/lys/runner-pending.json"))
    );
    for arguments in [
        vec!["lys", "runner", "install", "--pending"],
        vec![
            "lys",
            "runner",
            "install",
            "--server",
            "https://lys.example.test/api",
        ],
        vec![
            "lys",
            "runner",
            "install",
            "--pending",
            "/pending.json",
            "--machine",
            "op-example",
        ],
        vec![
            "lys",
            "runner",
            "install",
            "--server",
            "https://lys.example.test/api",
            "--machine",
            "op-example",
            "--code",
            "fixture",
        ],
    ] {
        assert!(Cli::try_parse_from(arguments).is_err());
    }
    for command in ["start", "uninstall", "update"] {
        assert!(Cli::try_parse_from(["lys", "runner", command]).is_ok());
    }
    Ok(())
}

#[cfg(unix)]
fn worker_input(command: &mut Command) -> Result<std::os::unix::net::UnixStream, std::io::Error> {
    let (control, input) = std::os::unix::net::UnixStream::pair()?;
    command.stdin(Stdio::from(std::os::fd::OwnedFd::from(input)));
    Ok(control)
}

#[test]
#[cfg(unix)]
fn worker_ready_line_confirms_the_build_then_worker_exit_reports_its_status() -> TestResult {
    let root = tempfile::tempdir()?;
    let layout = Layout::at(root.path().to_path_buf());
    let mut command = Command::new("/bin/sh");
    command.args(["-c", "printf '%s\\n' 'starting' 'runner_service_ready' 'runner_service_ready '; printf 'runner_service_'; printf 'ready bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\\n'; IFS= read -r reply; exit 23"]);
    let mut control = worker_input(&mut command)?;
    let mut commits = Vec::new();
    let result = runner_join::supervise_command(&layout, &mut command, &mut |commit| {
        commits.push(commit.to_owned());
        control
            .write_all(b"continue\n")
            .map_err(|error| io("release ready worker", root.path(), error))
    });
    drop(control);
    assert_eq!(commits, ["bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"]);
    let error = result.expect_err("the worker exit was not reported");
    assert!(error.to_string().contains("runner_service_exited"));
    assert!(error.to_string().contains("23"));
    Ok(())
}

#[test]
#[cfg(unix)]
fn worker_exit_before_a_ready_line_never_confirms_the_build() -> TestResult {
    let root = tempfile::tempdir()?;
    let layout = Layout::at(root.path().to_path_buf());
    let mut command = Command::new("/bin/sh");
    command.args(["-c", "exit 29"]).stdin(Stdio::null());
    let mut confirmed = false;
    let result = runner_join::supervise_command(&layout, &mut command, &mut |_| {
        confirmed = true;
        Ok(())
    });
    assert!(!confirmed);
    let error = result.expect_err("a worker exit before readiness was accepted");
    let words = error.to_string();
    assert!(words.contains("runner_service_exited") || words.contains("runner_service_unready"));
    Ok(())
}

#[test]
#[cfg(unix)]
fn worker_closing_its_pipe_before_a_complete_ready_line_is_refused() -> TestResult {
    for script in [
        "exec 1>&-; IFS= read -r reply",
        "printf 'runner_service_ready bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb'; exec 1>&-; IFS= read -r reply",
    ] {
        let root = tempfile::tempdir()?;
        let layout = Layout::at(root.path().to_path_buf());
        let mut command = Command::new("/bin/sh");
        command.args(["-c", script]);
        let control = worker_input(&mut command)?;
        let mut confirmed = false;
        let result = runner_join::supervise_command(&layout, &mut command, &mut |_| {
            confirmed = true;
            Ok(())
        });
        drop(control);
        assert!(!confirmed);
        let error = result.expect_err("closing the readiness pipe was accepted");
        assert!(error.to_string().contains("runner_service_unready"));
    }
    Ok(())
}
