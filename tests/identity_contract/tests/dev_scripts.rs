#![cfg(test)]
//! The development scripts seed exactly the grants they name through a fresh service.

use std::error::Error;
use std::fs::{File, OpenOptions};
use std::io::Read;
use std::net::TcpListener;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

fn succeeded(output: &Output, action: &str) -> TestResult {
    if !output.status.success() {
        return Err(format!(
            "{action} exited {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(())
}

struct Held {
    dir: tempfile::TempDir,
    surface: PathBuf,
    binaries: PathBuf,
    tools: PathBuf,
    finished: Vec<(String, File, File)>,
}

impl Held {
    fn open() -> Result<Self, Box<dyn Error>> {
        let dir = tempfile::tempdir()?;
        let surface = dir.path().join("surface/identity");
        let binaries = dir.path().join("binaries");
        let tools = dir.path().join("tools");
        for path in [&surface.join("dev"), &binaries, &tools] {
            std::fs::create_dir_all(path)?;
        }
        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../surface/identity")
            .canonicalize()?;
        for name in [
            "package.json",
            "package-lock.json",
            "dev/start.sh",
            "dev/seed-grants.sh",
        ] {
            std::fs::copy(source.join(name), surface.join(name))?;
        }
        succeeded(
            &Command::new("npm")
                .arg("ci")
                .current_dir(&surface)
                .output()?,
            "install the locked preview tool",
        )?;
        std::fs::create_dir(surface.join("dist"))?;
        std::fs::write(
            surface.join("dist/index.html"),
            "<!doctype html><title>Development script fixture</title>",
        )?;
        let compiled = Path::new(env!("CARGO_BIN_EXE_dev_issuer"))
            .parent()
            .ok_or("the issuer binary has no directory")?;
        for name in ["dev_issuer", "lys-identity-server", "lys-identity-dev-seed"] {
            let binary = compiled.join(name);
            if !binary.is_file() {
                return Err(
                    format!("development_binary_missing: {name}; run the workspace gate").into(),
                );
            }
            symlink(binary, binaries.join(name))?;
        }
        let mut finished = Vec::new();
        for name in ["issuer", "service", "app"] {
            let started = dir.path().join(format!("{name}.started"));
            let ended = dir.path().join(format!("{name}.finished"));
            succeeded(
                &Command::new("mkfifo").arg(&started).arg(&ended).output()?,
                "make child lifecycle signals",
            )?;
            finished.push((
                name.to_owned(),
                OpenOptions::new().read(true).write(true).open(started)?,
                OpenOptions::new().read(true).write(true).open(ended)?,
            ));
        }
        let observer = tools.join("nohup");
        std::fs::write(
            &observer,
            r#"#!/bin/sh
trap '' HUP
case "$1" in
  */dev_issuer) name=issuer ;;
  */lys-identity-server) name=service ;;
  node) name=app ;;
  *) echo "unexpected development child" >&2; exit 2 ;;
esac
finished="$LYS_DEV_TEST_SIGNALS/$name.finished"
"$@" &
child=$!
trap 'if kill -TERM "$child"; then wait "$child"; fi; printf x >"$finished"; exit 0' TERM INT
printf x >"$LYS_DEV_TEST_SIGNALS/$name.started"
wait "$child"
code=$?
printf x >"$finished"
exit "$code"
"#,
        )?;
        std::fs::set_permissions(observer, std::fs::Permissions::from_mode(0o700))?;
        Ok(Self {
            dir,
            surface,
            binaries,
            tools,
            finished,
        })
    }

    fn state(&self) -> PathBuf {
        self.dir.path().join("state")
    }

    fn logs(&self) -> PathBuf {
        self.dir.path().join("logs")
    }

    fn stop(&mut self) -> TestResult {
        for (name, started, signal) in self.finished.iter_mut().rev() {
            let pid_file = self.dir.path().join(format!("state/{name}.pid"));
            if !pid_file.exists() {
                continue;
            }
            let mut ready = [0];
            started.read_exact(&mut ready)?;
            assert_eq!(ready, [b'x']);
            let pid = std::fs::read_to_string(&pid_file)?.trim().parse::<u32>()?;
            let killed = Command::new("kill")
                .args(["-TERM", &pid.to_string()])
                .output()?;
            // A child that ended before readiness already sent its exit signal.
            if !killed.status.success()
                && !String::from_utf8_lossy(&killed.stderr).contains("No such process")
            {
                return Err(format!("development_child_stop_failed: {name}").into());
            }
            let mut ended = [0];
            signal.read_exact(&mut ended)?;
            assert_eq!(ended, [b'x']);
            std::fs::remove_file(pid_file)?;
        }
        Ok(())
    }

    fn start(&self, ports: [u16; 3]) -> Result<Output, Box<dyn Error>> {
        let search = std::env::var_os("PATH").ok_or("PATH is missing")?;
        let mut dirs = vec![self.tools.clone()];
        dirs.extend(std::env::split_paths(&search));
        Ok(Command::new("sh")
            .arg(self.surface.join("dev/start.sh"))
            .arg(self.state())
            .arg(self.logs())
            .env("HOST", "127.0.0.1")
            .env("APP_PORT", ports[0].to_string())
            .env("SERVICE_PORT", ports[1].to_string())
            .env("ISSUER_PORT", ports[2].to_string())
            .env("LYS_DEV_BIN", &self.binaries)
            .env("LYS_DEV_SKIP_BUILD", "1")
            .env("LYS_DEV_TEST_SIGNALS", self.dir.path())
            .env("PATH", std::env::join_paths(dirs)?)
            .output()?)
    }
}

impl Drop for Held {
    fn drop(&mut self) {
        if let Err(error) = self.stop() {
            eprintln!("development fixture cleanup failed: {error}");
        }
    }
}

#[tokio::test]
async fn the_real_development_scripts_seed_their_named_grants_on_fresh_state() -> TestResult {
    let mut held = Held::open()?;
    let listeners = [
        TcpListener::bind("127.0.0.1:0")?,
        TcpListener::bind("127.0.0.1:0")?,
        TcpListener::bind("127.0.0.1:0")?,
    ];
    let ports = [
        listeners[0].local_addr()?.port(),
        listeners[1].local_addr()?.port(),
        listeners[2].local_addr()?.port(),
    ];
    drop(listeners);
    let start = held.start(ports)?;
    assert!(
        start.status.success(),
        "start.sh exited {}; seed-grants log: {}",
        start.status,
        std::fs::read_to_string(held.logs().join("seed-grants.log"))?
    );
    assert!(held.state().join("grants.seeded").is_file());
    assert!(!held.logs().join("app-build.log").exists());
    let service = format!("http://127.0.0.1:{}", ports[1]);
    let http = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let login = http
        .post(format!("{service}/sign-in"))
        .json(&json!({ "email": "ada@example.test", "password": "development-only" }))
        .send()
        .await?;
    assert!(login.status().is_success());
    let cookie = login
        .headers()
        .get(reqwest::header::SET_COOKIE)
        .ok_or("sign-in answered no session cookie")?
        .to_str()?
        .split(';')
        .next()
        .ok_or("sign-in cookie is empty")?
        .to_owned();
    let answer = http
        .get(format!("{service}/grants"))
        .header(reqwest::header::COOKIE, cookie)
        .send()
        .await?
        .error_for_status()?
        .json::<Value>()
        .await?;
    let grants = answer["grants"].as_array().ok_or("no grants listed")?;
    assert_eq!(grants.len(), 3, "{answer}");
    let seed = std::fs::read_to_string(held.logs().join("seed.log"))?;
    let first = |suffix: &str| -> Result<&str, Box<dyn Error>> {
        seed.lines()
            .find(|line| line.ends_with(suffix))
            .and_then(|line| line.split_whitespace().next())
            .ok_or_else(|| format!("seed did not name {suffix}").into())
    };
    let ada = first("signs in as ada")?;
    let scribe = seed
        .lines()
        .find(|line| line.split_whitespace().nth(1) == Some("Scribe"))
        .and_then(|line| line.split_whitespace().next())
        .ok_or("seed did not name Scribe")?;
    for (holder, resource, relation) in [
        (ada, "identity", "owner"),
        (ada, "ledger", "viewer"),
        (scribe, "identity", "viewer"),
    ] {
        assert_eq!(
            grants
                .iter()
                .filter(|grant| grant["holder"] == holder
                    && grant["resource"] == json!({"kind": "project", "id": resource})
                    && grant["relation"] == relation)
                .count(),
            1,
            "{answer}"
        );
    }
    held.stop()
}
