//! Per-user runner installation and native login services.

use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use zeroize::Zeroizing;

use crate::commands::error::{CliError, CliResult};
use crate::commands::runner_join::{self, SavedJoin};
use crate::identity::install::layout::Layout;
use crate::identity::{private_files, upgrade::swap};
use lys_runner::RunnerError;

#[path = "runner_service_account.rs"]
mod account;
#[cfg(test)]
use account::shell_from_record;
use account::{account_shell, path_text, user};

pub(crate) fn json_detail(error: &serde_json::Error) -> String {
    format!(
        "{:?} at line {}, column {}",
        error.classify(),
        error.line(),
        error.column()
    )
}

pub(crate) fn refused(name: &str, words: impl Into<String>) -> CliError {
    RunnerError::refused(name, words).into()
}

pub(crate) fn io(action: &str, path: &Path, source: std::io::Error) -> CliError {
    CliError::Io {
        context: format!("runner_service_io: {action} {}", path.display()),
        source,
    }
}

pub(crate) fn say(words: &str) -> CliResult<()> {
    writeln!(std::io::stdout().lock(), "{words}")
        .map_err(|error| io("write service outcome", Path::new("stdout"), error))
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Pending {
    server: String,
    machine: String,
    #[serde(deserialize_with = "private_code")]
    code: Zeroizing<String>,
}

fn private_code<'de, D: serde::Deserializer<'de>>(input: D) -> Result<Zeroizing<String>, D::Error> {
    <String as serde::Deserialize<'de>>::deserialize(input).map(Zeroizing::new)
}

fn consume_pending(
    layout: &Layout,
    path: &Path,
    admit: &mut dyn FnMut(&Layout, &SavedJoin, &str) -> CliResult<()>,
) -> CliResult<()> {
    let bytes = private_files::read(path).map_err(|error| {
        refused("runner_pending_unreadable", format!("the pending connection cannot be read as an owner-only file; open the installer as the person connecting this computer: {error}"))
    })?.ok_or_else(|| {
        refused("runner_pending_unreadable", "the pending connection file is missing; download this computer's installer again")
    })?;
    let pending: Pending = serde_json::from_slice(&bytes).map_err(|error| {
        refused(
            "runner_pending_invalid",
            format!(
                "the pending connection is invalid; download a fresh installer ({})",
                json_detail(&error)
            ),
        )
    })?;
    let saved = SavedJoin {
        server: pending.server.trim_end_matches('/').to_owned(),
        machine: pending.machine.clone(),
        server_ca: None,
    };
    saved.validate()?;
    if pending.code.trim().is_empty() {
        return Err(refused(
            "runner_pending_code_missing",
            "the pending connection contains no connection code",
        ));
    }
    admit(layout, &saved, pending.code.trim())?;
    saved.save(layout)?;
    std::fs::remove_file(path).map_err(|error| {
        refused(
            "runner_pending_delete_refused",
            format!(
                "this computer joined, but the used pending file could not be deleted: {error}"
            ),
        )
    })?;
    sync_dir(
        path.parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new(".")),
    )
}

pub(crate) fn sync_dir(path: &Path) -> CliResult<()> {
    std::fs::File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|error| io("sync directory", path, error))
}

/// Join once and install a login service pointing at a writable executable pair.
pub(crate) fn install(
    pending: Option<PathBuf>,
    server: Option<String>,
    machine: Option<String>,
    server_ca: Option<PathBuf>,
) -> CliResult<()> {
    let layout = Layout::discover()?;
    let service = Service::discover(&layout)?;
    let person = user("-un")?;
    let shell = account_shell(service.platform, &person)?;
    let program =
        std::env::current_exe().map_err(|error| io("locate executable", &layout.root, error))?;
    let companion = program.with_file_name("lys-containment-entry");
    if !companion.is_file() {
        return Err(refused(
            "runner_helper_missing",
            "the installer is missing lys-containment-entry beside lys; download a complete installer",
        ));
    }
    for directory in [
        layout.root.clone(),
        layout.bin_dir(),
        layout.run_dir(),
        layout.logs_dir(),
        layout.data_dir(),
    ] {
        private_files::ensure_dir(&directory)?;
    }
    if let Some(path) = pending {
        consume_pending(&layout, &path, &mut runner_join::join)?;
    } else if let (Some(server), Some(machine)) = (server, machine) {
        let server_ca = server_ca
            .map(std::fs::canonicalize)
            .transpose()
            .map_err(|error| io("read certificate authority", &layout.root, error))?;
        let saved = SavedJoin {
            server: server.trim_end_matches('/').to_owned(),
            machine,
            server_ca,
        };
        saved.validate()?;
        let code =
            runner_join::read_code(&mut std::io::stdin().lock(), std::io::stdin().is_terminal())?;
        runner_join::join(&layout, &saved, code.as_str())?;
    } else {
        SavedJoin::read(&layout)?;
    }
    for (source, name) in [(&program, "lys"), (&companion, "lys-containment-entry")] {
        if *source != layout.binary(name) {
            swap::place_binary(source, &layout.bin_dir(), name)?;
        }
    }
    private_files::ensure_dir(&layout.bin_dir().join("runner-service"))?;
    swap::place_binary(
        &layout.binary("lys"),
        &layout.bin_dir().join("runner-service"),
        "lys",
    )?;
    service.install(&person, &shell)?;
    say("This computer is connected. Its runner starts at login and restarts when it stops.")
}

/// Unload only this login service; keep the machine's admission and keys.
pub(crate) fn uninstall() -> CliResult<()> {
    Service::discover(&Layout::discover()?)?.uninstall()?;
    say("The runner login service was removed. This computer's saved connection was kept.")
}

/// The controller is replaced only after the serving executable reports ready.
pub(crate) fn start(worker: bool, restored: bool) -> CliResult<()> {
    let layout = Layout::discover()?;
    SavedJoin::read(&layout)?;
    if worker
        || !layout
            .binary("lys")
            .try_exists()
            .map_err(|error| io("find installed runner", &layout.binary("lys"), error))?
    {
        return runner_join::start_saved(&layout, 1 << 20);
    }
    let recovered = super::update::recover(&layout)?;
    if cfg!(target_os = "macos") && !restored && !recovered {
        if let Err(error) = super::update::prepare(&layout) {
            say(&format!("{error}; the installed runner will start"))?;
            super::update::recover(&layout)?;
        }
    }
    let result = runner_join::supervise(&layout);
    if let Err(error) = result {
        if super::update::rollback(&layout)? {
            say(&format!("runner_service_restarting_previous: {error}"))?;
            #[cfg(unix)]
            {
                use std::os::unix::process::CommandExt;
                let error = Command::new(layout.bin_dir().join("runner-service/lys"))
                    .args(["runner", "start", "--restored"])
                    .exec();
                return Err(io("restart previous runner", &layout.binary("lys"), error));
            }
        }
        return Err(error);
    }
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Platform {
    Mac,
    Linux,
}

pub(crate) struct Service {
    platform: Platform,
    home: PathBuf,
    program: PathBuf,
    log: PathBuf,
    identity_home: PathBuf,
}

impl Service {
    pub(crate) fn discover(layout: &Layout) -> CliResult<Self> {
        let platform = match std::env::consts::OS {
            "macos" => Platform::Mac,
            "linux" => Platform::Linux,
            _ => {
                return Err(refused(
                    "runner_service_platform_unsupported",
                    "runner login services need macOS or Linux",
                ));
            }
        };
        let home = std::env::var_os("HOME")
            .filter(|home| !home.is_empty())
            .ok_or_else(|| {
                refused(
                    "runner_service_home_missing",
                    "HOME is missing; install the runner as the person who will use this computer",
                )
            })?;
        if !layout.root.is_absolute() || !Path::new(&home).is_absolute() {
            return Err(refused(
                "runner_service_path_invalid",
                "the home and Lys data directories must be absolute paths",
            ));
        }
        Ok(Self {
            platform,
            home: home.into(),
            program: layout.bin_dir().join("runner-service/lys"),
            log: layout.logs_dir().join("runner-service.log"),
            identity_home: layout.root.clone(),
        })
    }

    fn path(&self) -> PathBuf {
        match self.platform {
            Platform::Mac => self
                .home
                .join("Library/LaunchAgents/au.com.ablative.lys-runner.plist"),
            Platform::Linux => self.home.join(".config/systemd/user/lys-runner.service"),
        }
    }

    fn text(&self, person: &str, shell: &Path) -> CliResult<String> {
        match self.platform {
            Platform::Mac => plist(&self.program, &self.log, &self.identity_home, person, shell),
            Platform::Linux => unit(&self.program, &self.log, &self.identity_home, person, shell),
        }
    }

    fn install(&self, person: &str, shell: &Path) -> CliResult<()> {
        let path = self.path();
        let parent = path.parent().ok_or_else(|| {
            refused(
                "runner_service_path_invalid",
                "the login service has no containing directory",
            )
        })?;
        std::fs::create_dir_all(parent)
            .map_err(|error| io("make login service directory", parent, error))?;
        private_files::write(&path, self.text(person, shell)?.as_bytes())?;
        match self.platform {
            Platform::Mac => {
                let domain = format!("gui/{}", user("-u")?);
                let target = format!("{domain}/au.com.ablative.lys-runner");
                if Command::new("launchctl")
                    .args(["print", &target])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .map_err(|error| io("inspect login service", &path, error))?
                    .success()
                {
                    checked(Command::new("launchctl").args(["bootout", &target]))?;
                }
                checked(
                    Command::new("launchctl")
                        .args(["bootstrap", &domain])
                        .arg(&path),
                )
            }
            Platform::Linux => {
                let active = Command::new("systemctl")
                    .args(["--user", "is-active", "--quiet", "lys-runner.service"])
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .map_err(|error| io("inspect login service", &path, error))?
                    .success();
                checked(Command::new("systemctl").args(["--user", "daemon-reload"]))?;
                checked(Command::new("systemctl").args([
                    "--user",
                    "enable",
                    "--now",
                    "lys-runner.service",
                ]))?;
                if active {
                    self.restart()?;
                }
                let linger =
                    checked(Command::new("loginctl").args(["enable-linger", &user("-un")?]));
                if let Err(error) = linger {
                    say(&format!(
                        "runner_linger_refused: the runner is installed, but starting before login was refused: {error}"
                    ))?;
                }
                Ok(())
            }
        }
    }

    pub(crate) fn restart(&self) -> CliResult<()> {
        match self.platform {
            Platform::Mac => checked(Command::new("launchctl").args([
                "kickstart",
                "-k",
                &format!("gui/{}/au.com.ablative.lys-runner", user("-u")?),
            ])),
            Platform::Linux => {
                checked(Command::new("systemctl").args(["--user", "restart", "lys-runner.service"]))
            }
        }
    }

    pub(crate) fn require_installed(&self) -> CliResult<()> {
        if !self
            .path()
            .try_exists()
            .map_err(|error| io("find login service", &self.path(), error))?
        {
            return Err(refused(
                "runner_service_not_installed",
                "this user has no runner login service; run runner install before updating it",
            ));
        }
        Ok(())
    }

    fn uninstall(&self) -> CliResult<()> {
        let path = self.path();
        if !path
            .try_exists()
            .map_err(|error| io("find login service", &path, error))?
        {
            return Err(refused(
                "runner_service_not_installed",
                "this user has no runner login service installed",
            ));
        }
        match self.platform {
            Platform::Mac => checked(Command::new("launchctl").args([
                "bootout",
                &format!("gui/{}/au.com.ablative.lys-runner", user("-u")?),
            ]))?,
            Platform::Linux => checked(Command::new("systemctl").args([
                "--user",
                "disable",
                "--now",
                "lys-runner.service",
            ]))?,
        }
        std::fs::remove_file(&path).map_err(|error| io("remove login service", &path, error))?;
        sync_dir(path.parent().ok_or_else(|| {
            refused(
                "runner_service_path_invalid",
                "the login service has no containing directory",
            )
        })?)?;
        if self.platform == Platform::Linux {
            checked(Command::new("systemctl").args(["--user", "daemon-reload"]))?;
        }
        Ok(())
    }
}

fn checked(command: &mut Command) -> CliResult<()> {
    let name = command.get_program().to_string_lossy().into_owned();
    let output = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| {
            refused(
                "runner_service_command_failed",
                format!("{name} could not run: {error}"),
            )
        })?;
    if !output.status.success() {
        return Err(refused(
            "runner_service_command_refused",
            format!(
                "{name} refused the login service operation ({})",
                output.status
            ),
        ));
    }
    Ok(())
}

fn xml(path: &Path) -> CliResult<String> {
    Ok(path_text(path)?
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;"))
}

fn plist(program: &Path, log: &Path, root: &Path, person: &str, shell: &Path) -> CliResult<String> {
    let (program, log, root) = (xml(program)?, xml(log)?, xml(root)?);
    let (person, shell) = (xml(Path::new(person))?, xml(shell)?);
    Ok(format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\">\n<dict>\n  <key>Label</key><string>au.com.ablative.lys-runner</string>\n  <key>ProgramArguments</key>\n  <array><string>{program}</string><string>runner</string><string>start</string></array>\n  <key>RunAtLoad</key><true/>\n  <key>KeepAlive</key><true/>\n  <key>EnvironmentVariables</key>\n  <dict>\n    <key>LYS_IDENTITY_HOME</key><string>{root}</string>\n    <key>SHELL</key><string>{shell}</string>\n    <key>USER</key><string>{person}</string>\n    <key>LOGNAME</key><string>{person}</string>\n  </dict>\n  <key>StandardOutPath</key><string>{log}</string>\n  <key>StandardErrorPath</key><string>{log}</string>\n</dict>\n</plist>\n"
    ))
}

fn systemd(path: &Path) -> CliResult<String> {
    Ok(path_text(path)?
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('%', "%%"))
}

fn unit(program: &Path, log: &Path, root: &Path, person: &str, shell: &Path) -> CliResult<String> {
    let (program, log, root) = (systemd(program)?, systemd(log)?, systemd(root)?);
    let (person, shell) = (systemd(Path::new(person))?, systemd(shell)?);
    let program = program.replace('$', "$$");
    // Update refusals are logged without preventing the installed runner from starting.
    Ok(format!(
        "[Unit]\nDescription=Lys runner\nAfter=network-online.target\nWants=network-online.target\nStartLimitIntervalSec=0\n\n[Service]\nType=simple\nEnvironment=\"LYS_IDENTITY_HOME={root}\"\nEnvironment=\"SHELL={shell}\"\nEnvironment=\"USER={person}\"\nEnvironment=\"LOGNAME={person}\"\nEnvironment=LYS_RUNNER_SERVICE=1\nExecStartPre=-\"{program}\" runner update\nExecStart=\"{program}\" runner start\nRestart=always\nStandardOutput=append:{log}\nStandardError=append:{log}\n\n[Install]\nWantedBy=default.target\n"
    ))
}

#[cfg(test)]
#[path = "runner_service_tests.rs"]
mod tests;
