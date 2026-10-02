#![cfg(test)]
//! Compile the actual stamp once per source and compiler, across test processes.

use std::error::Error;
use std::ffi::{OsStr, OsString};
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;

use sha2::{Digest, Sha256};

struct Compiler {
    executable: OsString,
    version: Vec<u8>,
}

fn compiler() -> Result<&'static Compiler, Box<dyn Error>> {
    static COMPILER: OnceLock<Result<Compiler, String>> = OnceLock::new();
    COMPILER
        .get_or_init(|| {
            let executable = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
            let version = Command::new(&executable)
                .arg("-vV")
                .current_dir(env!("CARGO_MANIFEST_DIR"))
                .output()
                .map_err(|error| format!("stamp_compiler_unavailable: {error}"))?;
            if !version.status.success() {
                return Err(format!(
                    "stamp_compiler_version_failed: {}",
                    String::from_utf8_lossy(&version.stderr)
                ));
            }
            Ok(Compiler {
                executable,
                version: version.stdout,
            })
        })
        .as_ref()
        .map_err(|error| error.clone().into())
}

fn executable(source: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let compiler = compiler()?;
    let source = fs::read(source)?;
    let mut hash = Sha256::new();
    hash.update(&compiler.version);
    hash.update(&source);
    let key = format!("{:x}", hash.finalize());
    let cache = Path::new(env!("CARGO_TARGET_TMPDIR")).join("version-stamps");
    fs::create_dir_all(&cache)?;
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(cache.join(format!("{key}.lock")))?;
    lock.lock()?;
    let binary = cache.join(format!("{key}{}", std::env::consts::EXE_SUFFIX));
    if !binary.is_file() {
        let temporary = tempfile::tempdir_in(&cache)?;
        let input = temporary.path().join("stamp.rs");
        fs::write(&input, source)?;
        let output = temporary.path().join("stamp");
        let compiled = Command::new(&compiler.executable)
            .args(["--edition=2024", "--crate-name", "stamp_probe"])
            .arg(&input)
            .arg("-o")
            .arg(&output)
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()?;
        if !compiled.status.success() {
            return Err(format!(
                "stamp_compile_failed: {}",
                String::from_utf8_lossy(&compiled.stderr)
            )
            .into());
        }
        fs::rename(output, &binary)?;
    }
    Ok(binary)
}

pub(super) fn run(tree: &Path, commit: Option<&OsStr>) -> Result<Output, Box<dyn Error>> {
    let mut command = Command::new(executable(&tree.join("build.rs"))?);
    command
        .current_dir(tree)
        .env("CARGO_MANIFEST_DIR", tree)
        .env_remove("LYS_BUILD_COMMIT")
        .env("GIT_CEILING_DIRECTORIES", tree.parent().ok_or("no parent")?);
    if let Some(commit) = commit {
        command.env("LYS_BUILD_COMMIT", commit);
    }
    let mut output = command.output()?;
    if output.status.success() {
        let stdout = String::from_utf8(output.stdout)?;
        let value = stdout
            .lines()
            .find_map(|line| line.strip_prefix("cargo:rustc-env=LYS_BUILD="))
            .ok_or("stamp_output_missing: build script emitted no provenance")?;
        output.stdout = value.as_bytes().to_vec();
    }
    Ok(output)
}
