#![cfg(test)]

use std::error::Error;
use std::os::unix::fs::PermissionsExt;

use super::{KEPT, fixture, login, login_from};

/// A variable the invoking shell could carry and no service may: the
/// shape of a harness's config folder.
const LEAK: &str = "LYS_TEST_HARNESS_CONFIG_DIR";

/// The test login's variables with the leak added; never this process's.
fn process_with_leak() -> Result<Vec<(std::ffi::OsString, std::ffi::OsString)>, Box<dyn Error>> {
    let mut process = fixture::variables()?;
    process.push((LEAK.into(), "/leaked".into()));
    Ok(process)
}

#[test]
fn the_login_environment_keeps_the_login_and_nothing_of_the_invoking_process()
-> Result<(), Box<dyn Error>> {
    let environment = login_from(process_with_leak()?)?;
    let variables: Vec<(&str, &str)> = environment.variables().collect();
    assert!(
        !variables.iter().any(|(name, _)| *name == LEAK),
        "the leak must not be kept"
    );
    for (name, _) in &variables {
        assert!(
            *name == "PATH" || KEPT.contains(name),
            "{name} is not a login variable"
        );
    }
    let home = fixture::home();
    assert!(
        variables.contains(&("HOME", home.to_str().ok_or("a home in text")?)),
        "HOME is the login's"
    );
    let (_, path) = variables
        .iter()
        .find(|(name, _)| *name == "PATH")
        .ok_or("PATH must be answered")?;
    assert!(!path.is_empty(), "PATH must not be empty");
    let record = environment.record();
    assert_eq!(record.path, *path);
    assert!(!record.kept.iter().any(|name| name == "PATH"));
    assert!(record.kept.iter().any(|name| name == "HOME"));
    Ok(())
}

#[test]
fn a_login_profile_that_prints_does_not_become_part_of_path() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::TempDir::new()?;
    let shell = dir.path().join("noisy-shell");
    // A login shell whose profile prints first, answers as /bin/sh would, and
    // whose logout file prints after.
    std::fs::write(
        &shell,
        "#!/bin/sh\necho 'Last login: noise from a profile'\n/bin/sh \"$@\"\necho 'bye from a logout file'\n",
    )?;
    std::fs::set_permissions(&shell, std::fs::Permissions::from_mode(0o700))?;
    let mut process = process_with_leak()?;
    process.retain(|(name, _)| name != "SHELL");
    process.push(("SHELL".into(), shell.into()));
    let environment = login_from(process)?;
    let (_, path) = environment
        .variables()
        .find(|(name, _)| *name == "PATH")
        .ok_or("PATH must be answered")?;
    assert!(
        !path.contains("noise"),
        "profile output became PATH: {path}"
    );
    assert!(!path.contains("bye"), "logout output became PATH: {path}");
    assert!(!path.contains('\n'), "PATH carries a line break: {path}");
    assert!(
        path.split(':')
            .any(|entry| entry == "/bin" || entry == "/usr/bin"),
        "{path}"
    );

    // A shell that never answers its PATH is refused by name.
    let mute = dir.path().join("mute-shell");
    std::fs::write(&mute, "#!/bin/sh\necho 'nothing'\n")?;
    std::fs::set_permissions(&mute, std::fs::Permissions::from_mode(0o700))?;
    let mut process = process_with_leak()?;
    process.retain(|(name, _)| name != "SHELL");
    process.push(("SHELL".into(), mute.into()));
    let refusal = login_from(process)
        .err()
        .ok_or("a mute shell must be refused")?;
    assert!(
        refusal.to_string().contains("did not answer its PATH"),
        "{refusal}"
    );

    // A shell that opens its answer and never closes it is refused by name too.
    let open = dir.path().join("open-shell");
    std::fs::write(&open, "#!/bin/sh\nprintf 'LYS_LOGIN_PATH:/bin'\n")?;
    std::fs::set_permissions(&open, std::fs::Permissions::from_mode(0o700))?;
    let mut process = process_with_leak()?;
    process.retain(|(name, _)| name != "SHELL");
    process.push(("SHELL".into(), open.into()));
    let refusal = login_from(process)
        .err()
        .ok_or("an unclosed answer must be refused")?;
    assert!(
        refusal
            .to_string()
            .contains("did not close its PATH answer"),
        "{refusal}"
    );

    // A shell that answers its PATH but not the login's ANTHROPIC_BASE_URL is
    // refused by name, never taken as naming none.
    let half = dir.path().join("half-shell");
    std::fs::write(
        &half,
        "#!/bin/sh\nprintf 'LYS_LOGIN_PATH:/bin:LYS_LOGIN_PATH_END'\n",
    )?;
    std::fs::set_permissions(&half, std::fs::Permissions::from_mode(0o700))?;
    let mut process = process_with_leak()?;
    process.retain(|(name, _)| name != "SHELL");
    process.push(("SHELL".into(), half.into()));
    let refusal = login_from(process)
        .err()
        .ok_or("an answer without the base must be refused")?;
    assert!(
        refusal
            .to_string()
            .contains("did not answer its ANTHROPIC_BASE_URL"),
        "{refusal}"
    );
    Ok(())
}

#[test]
fn a_login_without_a_shell_is_refused_in_the_product_s_words() -> Result<(), Box<dyn Error>> {
    let mut process = process_with_leak()?;
    process.retain(|(name, _)| name != "SHELL");
    let refusal = login_from(process)
        .err()
        .ok_or("a login without SHELL must be refused")?;
    assert_eq!(refusal.kind(), super::ErrorKind::Unready, "{refusal}");
    assert!(
        refusal
            .to_string()
            .contains("SHELL is not set; start the install from a login"),
        "{refusal}"
    );
    Ok(())
}

#[test]
fn a_test_build_starts_the_test_login_s_shell_and_never_the_runner_s() -> Result<(), Box<dyn Error>>
{
    let environment = login()?;
    let variables: Vec<(&str, &str)> = environment.variables().collect();
    let shell = fixture::shell();
    assert!(
        variables.contains(&("SHELL", shell.to_str().ok_or("a shell in text")?)),
        "{variables:?}"
    );
    let home = fixture::home();
    assert!(
        variables.contains(&("HOME", home.to_str().ok_or("a home in text")?)),
        "{variables:?}"
    );
    // Only the test login's shell answers this PATH: the runner's own login
    // shell would answer the runner's.
    assert!(
        variables.contains(&("PATH", fixture::PATH)),
        "{variables:?}"
    );
    assert_eq!(environment.anthropic_base(), None);
    Ok(())
}

#[test]
fn service_path_is_the_interactive_login_path_including_user_programmes()
-> Result<(), Box<dyn Error>> {
    let dir = tempfile::TempDir::new()?;
    let shell = dir.path().join("login-shell");
    std::fs::write(
        &shell,
        "#!/bin/sh\nif [ \"$1\" = -l ]; then shift; fi\nif [ \"$1\" = -i ]; then PATH=/fixture/bun:/usr/bin:/bin; shift; else PATH=/usr/bin:/bin; fi\nexport PATH\nexec /bin/sh \"$@\"\n",
    )?;
    std::fs::set_permissions(&shell, std::fs::Permissions::from_mode(0o700))?;
    let mut process = process_with_leak()?;
    process.retain(|(name, _)| name != "SHELL");
    process.push(("SHELL".into(), shell.into()));
    let environment = login_from(process)?;
    assert_eq!(environment.record().path, "/fixture/bun:/usr/bin:/bin");
    Ok(())
}

/// Which part of the terminal regression a re-invoked test binary plays; unset
/// in an ordinary run, where the helper does nothing.
const TERMINAL_PART: &str = "LYS_TEST_LOGIN_TERMINAL_PART";
/// What the foreground part prints before the PATH its background part read.
const TERMINAL_ANSWER: &str = "LYS_TEST_LOGIN_TERMINAL_ANSWER:";
/// The helper's full name, as the re-invoked test binary is asked for it.
const TERMINAL_HELPER: &str = "identity::install::login::tests::login_terminal_helper";

/// The test binary asked to run [`login_terminal_helper`] as `part`.
fn terminal_helper(part: &str) -> Result<std::process::Command, Box<dyn Error>> {
    let mut helper = std::process::Command::new(std::env::current_exe()?);
    helper
        .args(["--exact", TERMINAL_HELPER, "--nocapture"])
        .env(TERMINAL_PART, part);
    Ok(helper)
}

/// A login read from outside the foreground process group of the terminal
/// that controls it, as a test runner that gives each test a group of its own
/// does, or a background job. The system's `script` gives the helper a
/// terminal of its own; the helper's foreground part starts its background
/// part in a new process group on that terminal, and the background part
/// reads the login. An interactive login shell that kept the terminal would
/// be stopped by `SIGTTIN` and never answer; the foreground part sees the stop
/// and says so.
#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn a_login_read_outside_the_terminal_s_foreground_group_still_answers() -> Result<(), Box<dyn Error>>
{
    let foreground = terminal_helper("foreground")?;
    let mut script = std::process::Command::new("script");
    #[cfg(target_os = "linux")]
    {
        let mut line = String::new();
        for word in std::iter::once(foreground.get_program()).chain(foreground.get_args()) {
            let word = word.to_str().ok_or("a helper word in text")?;
            line.push_str(&format!(" '{}'", word.replace('\'', r"'\''")));
        }
        script.args(["-q", "-e", "-c", &line, "/dev/null"]);
    }
    #[cfg(target_os = "macos")]
    script
        .args(["-q", "/dev/null"])
        .arg(foreground.get_program())
        .args(foreground.get_args());
    let output = script
        .env(TERMINAL_PART, "foreground")
        .stdin(std::process::Stdio::null())
        .output()?;
    let said = String::from_utf8_lossy(&output.stdout);
    assert!(
        said.contains(&format!("{TERMINAL_ANSWER}{}", fixture::PATH)),
        "the login read from a background group did not answer: {said}{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

/// The test login, with a shell that asks for the terminal when it is
/// started interactive from outside the terminal's foreground group. On Linux
/// that is the test login's own shell, which runs `/bin/sh`. On macOS
/// `/bin/sh` is a bash that gives up job control there, and so does `zsh`
/// when it does not lead its process group, so the shell runs `/bin/dash`,
/// which the system carries and which stops its group with `SIGTTIN` there as
/// Linux's shells do. It reads no profile, so it answers the test login's
/// `PATH`.
fn terminal_login(
    dir: &std::path::Path,
) -> Result<Vec<(std::ffi::OsString, std::ffi::OsString)>, Box<dyn Error>> {
    let mut process = fixture::variables()?;
    if cfg!(target_os = "macos") {
        let shell = dir.join("dash-login");
        std::fs::write(
            &shell,
            format!(
                "#!/bin/sh\nPATH='{}'\nexport PATH\nif [ \"$1\" = -l ]; then shift; fi\nexec /bin/dash \"$@\"\n",
                fixture::PATH
            ),
        )?;
        std::fs::set_permissions(&shell, std::fs::Permissions::from_mode(0o700))?;
        process.retain(|(name, _)| name != "SHELL");
        process.push(("SHELL".into(), shell.into()));
    }
    Ok(process)
}

/// The two parts of the terminal regression, chosen by [`TERMINAL_PART`].
#[test]
fn login_terminal_helper() -> Result<(), Box<dyn Error>> {
    use std::io::Read;
    use std::os::unix::process::CommandExt;

    use rustix::process::{Pid, WaitOptions, waitpid};

    match std::env::var(TERMINAL_PART).as_deref() {
        Ok("background") => {
            let dir = tempfile::TempDir::new()?;
            let environment = login_from(terminal_login(dir.path())?)?;
            println!("{TERMINAL_ANSWER}{}", environment.record().path);
            Ok(())
        }
        Ok("foreground") => {
            let mut background = terminal_helper("background")?
                .process_group(0)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::piped())
                .spawn()?;
            let mut stdout = background.stdout.take().ok_or("background stdout")?;
            let reading = std::thread::spawn(move || {
                let mut said = String::new();
                stdout.read_to_string(&mut said).map(|_| said)
            });
            let pid = Pid::from_child(&background);
            let (_, status) =
                waitpid(Some(pid), WaitOptions::UNTRACED)?.ok_or("the background part's wait")?;
            if let Some(signal) = status.stopping_signal() {
                background.kill()?;
                background.wait()?;
                return Err(format!("the background part was stopped by signal {signal}").into());
            }
            let said = reading
                .join()
                .map_err(|panic| format!("the background reader panicked: {panic:?}"))??;
            assert_eq!(status.exit_status(), Some(0), "{said}");
            let answer = said
                .lines()
                .find(|line| line.starts_with(TERMINAL_ANSWER))
                .ok_or_else(|| format!("the background part did not answer: {said}"))?;
            println!("{answer}");
            Ok(())
        }
        _ => Ok(()),
    }
}
