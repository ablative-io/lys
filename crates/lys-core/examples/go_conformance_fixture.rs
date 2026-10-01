#[path = "../tests/harness/cache.rs"]
mod cache;

use std::error::Error;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

fn target_tmp() -> Result<PathBuf, Box<dyn Error>> {
    let metadata = Command::new(env!("CARGO"))
        .args(["metadata", "--offline", "--no-deps", "--format-version=1"])
        .output()
        .map_err(|error| format!("go_fixture_metadata_spawn_failed: {error}"))?;
    if !metadata.status.success() {
        return Err(format!(
            "go_fixture_metadata_failed: {}",
            String::from_utf8_lossy(&metadata.stderr)
        )
        .into());
    }
    let value: serde_json::Value = serde_json::from_slice(&metadata.stdout)
        .map_err(|error| format!("go_fixture_metadata_invalid: {error}"))?;
    let target = value["target_directory"]
        .as_str()
        .ok_or("go_fixture_target_directory_missing")?;
    Ok(Path::new(target).join("tmp"))
}

fn toolchain() -> Result<Option<PathBuf>, Box<dyn Error>> {
    if let Some(overridden) = std::env::var_os("LYS_GO_BIN") {
        return Ok(Some(PathBuf::from(
            overridden
                .into_string()
                .map_err(|_| "go_fixture_toolchain_not_utf8")?,
        )));
    }
    if Path::new("/usr/local/go/bin/go")
        .try_exists()
        .map_err(|error| format!("go_fixture_toolchain_path_unreadable: {error}"))?
    {
        return Ok(Some(PathBuf::from("/usr/local/go/bin/go")));
    }
    match Command::new("go")
        .arg("version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
    {
        Ok(status) if status.success() => Ok(Some(PathBuf::from("go"))),
        Ok(status) => {
            eprintln!("go_fixture_path_probe_failed: {status}");
            Ok(None)
        }
        Err(error) => {
            eprintln!("go_fixture_path_probe_failed: {error}");
            Ok(None)
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let Some(go) = toolchain()? else {
        if std::env::var_os("LYS_REQUIRE_GO").is_some() {
            return Err("go_fixture_toolchain_required_but_missing".into());
        }
        eprintln!("skipping Go fixture: no Go toolchain found");
        return Ok(());
    };
    let target_tmp = target_tmp()?;
    let output = tempfile::tempdir()
        .map_err(|error| format!("go_fixture_output_directory_failed: {error}"))?;
    for name in ["go-conformance", "cose-conformance"] {
        let started = Instant::now();
        let scaffold = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join(name);
        cache::build(
            &go,
            &scaffold,
            &output.path().join(name),
            &target_tmp,
            |executable, gocache| {
                let status = Command::new(&go)
                    .args(["build", "-o"])
                    .arg(executable)
                    .arg(".")
                    .current_dir(&scaffold)
                    .env("GOFLAGS", "-mod=vendor")
                    .env("GOPROXY", "off")
                    .env("GOTOOLCHAIN", "local")
                    .env("GOWORK", "off")
                    .env("GOCACHE", gocache)
                    .status()
                    .map_err(|error| format!("go_fixture_spawn_failed: {name}: {error}"))?;
                if !status.success() {
                    return Err(format!("go_fixture_build_failed: {name}: {status}").into());
                }
                Ok(())
            },
        )?;
        eprintln!(
            "go_fixture scaffold={name} elapsed_us={}",
            started.elapsed().as_micros()
        );
    }
    if std::env::var_os("LYS_GO_PROFILE").is_some() {
        let environment =
            std::env::var_os("NEXTEST_ENV").ok_or("go_fixture_nextest_environment_missing")?;
        OpenOptions::new()
            .append(true)
            .open(environment)
            .and_then(|mut file| file.write_all(b"LYS_GO_PROFILE=1\n"))
            .map_err(|error| format!("go_fixture_nextest_environment_failed: {error}"))?;
    }
    Ok(())
}
