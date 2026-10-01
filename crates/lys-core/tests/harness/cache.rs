use std::error::Error;
use std::fs::{self, File};
use std::path::Path;
use std::process::Command;
use std::time::Instant;

use sha2::{Digest, Sha256};

fn field(hash: &mut Sha256, bytes: &[u8]) {
    hash.update(bytes.len().to_le_bytes());
    hash.update(bytes);
}

/// A cache failure retains the operation's named reason.
pub type CacheResult<T> = Result<T, Box<dyn Error>>;

fn sources(hash: &mut Sha256, root: &Path, dir: &Path) -> CacheResult<()> {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .map_err(|error| format!("go_cache_sources_unreadable: {}: {error}", dir.display()))?
        .collect::<Result<_, _>>()
        .map_err(|error| {
            format!(
                "go_cache_directory_entry_unreadable: {}: {error}",
                dir.display()
            )
        })?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        let kind = entry.file_type().map_err(|error| {
            format!(
                "go_cache_source_type_unreadable: {}: {error}",
                path.display()
            )
        })?;
        if kind.is_symlink() {
            return Err(format!("go_cache_source_symlink: {}", path.display()).into());
        }
        if kind.is_dir() {
            sources(hash, root, &path)?;
        } else {
            if !kind.is_file() {
                return Err(format!("go_cache_source_not_regular: {}", path.display()).into());
            }
            field(
                hash,
                path.strip_prefix(root)
                    .map_err(|error| {
                        format!(
                            "go_cache_source_outside_scaffold: {}: {error}",
                            path.display()
                        )
                    })?
                    .as_os_str()
                    .as_encoded_bytes(),
            );
            field(
                hash,
                &fs::read(&path).map_err(|error| {
                    format!("go_cache_source_unreadable: {}: {error}", path.display())
                })?,
            );
        }
    }
    Ok(())
}

fn source_hash(scaffold: &Path) -> CacheResult<Vec<u8>> {
    let mut hash = Sha256::new();
    sources(&mut hash, scaffold, scaffold)?;
    Ok(hash.finalize().to_vec())
}

fn executable_hash(path: &Path) -> CacheResult<Vec<u8>> {
    let bytes = fs::read(path).map_err(|error| {
        format!(
            "go_cache_executable_unreadable: {}: {error}",
            path.display()
        )
    })?;
    Ok(Sha256::digest(bytes).to_vec())
}

fn environment(go: &Path, scaffold: &Path, gocache: &Path) -> CacheResult<Vec<u8>> {
    let result = Command::new(go)
        .arg("env")
        .args([
            "GOVERSION",
            "GOROOT",
            "GOOS",
            "GOARCH",
            "GOAMD64",
            "GOARM",
            "GOARM64",
            "GOMIPS",
            "GOMIPS64",
            "GOPPC64",
            "GORISCV64",
            "GOWASM",
            "GOEXPERIMENT",
            "CGO_ENABLED",
            "CC",
            "CXX",
            "CGO_CFLAGS",
            "CGO_CPPFLAGS",
            "CGO_CXXFLAGS",
            "CGO_LDFLAGS",
            "GOFIPS140",
            "GOWORK",
        ])
        .current_dir(scaffold)
        .env("GOFLAGS", "-mod=vendor")
        .env("GOPROXY", "off")
        .env("GOTOOLCHAIN", "local")
        .env("GOWORK", "off")
        .env("GOCACHE", gocache)
        .output()
        .map_err(|error| format!("go_cache_toolchain_spawn_failed: {}: {error}", go.display()))?;
    if !result.status.success() {
        return Err(format!(
            "go_cache_toolchain_environment_failed: {}: {}",
            result.status,
            String::from_utf8_lossy(&result.stderr)
        )
        .into());
    }
    Ok(result.stdout)
}

/// Reuses a source-addressed executable without sharing a test's mutable files.
///
/// # Errors
///
/// Returns a named error on a broken toolchain, changed inputs, corrupt cache or I/O
/// error. The lock covers publication and copying, so readers see a full build.
pub fn build(
    go: &Path,
    scaffold: &Path,
    out: &Path,
    target_tmp: &Path,
    compile: impl FnOnce(&Path, &Path) -> CacheResult<()>,
) -> CacheResult<()> {
    let started = Instant::now();
    let root = target_tmp.join("lys-go-conformance-v1");
    let gocache = root.join("gocache");
    fs::create_dir_all(&gocache).map_err(|error| {
        format!(
            "go_cache_directory_create_failed: {}: {error}",
            gocache.display()
        )
    })?;
    let environment = environment(go, scaffold, &gocache)?;
    let source = source_hash(scaffold)?;
    let mut hash = Sha256::new();
    field(&mut hash, b"offline-vendor-build-v1");
    field(&mut hash, &source);
    field(&mut hash, go.as_os_str().as_encoded_bytes());
    field(&mut hash, &environment);
    let key = format!("{:x}", hash.finalize());
    let lock = File::options()
        .create(true)
        .truncate(false)
        .write(true)
        .open(root.join(format!("{key}.lock")))
        .map_err(|error| format!("go_cache_lock_open_failed: {error}"))?;
    let waiting = Instant::now();
    lock.lock()
        .map_err(|error| format!("go_cache_lock_failed: {error}"))?;
    profile("lock", waiting);
    let entry = root.join(key);
    if !entry
        .try_exists()
        .map_err(|error| format!("go_cache_entry_unreadable: {}: {error}", entry.display()))?
    {
        let staging = tempfile::tempdir_in(&root)
            .map_err(|error| format!("go_cache_staging_create_failed: {error}"))?;
        let published: CacheResult<()> = (|| {
            let executable = staging.path().join("tool");
            let compiling = Instant::now();
            compile(&executable, &gocache)?;
            profile("build", compiling);
            if source_hash(scaffold)? != source {
                return Err("go_cache_source_changed_during_build".into());
            }
            fs::write(staging.path().join("sha256"), executable_hash(&executable)?)
                .map_err(|error| format!("go_cache_checksum_write_failed: {error}"))?;
            fs::rename(staging.path(), &entry).map_err(|error| {
                format!("go_cache_publish_failed: {}: {error}", entry.display())
            })?;
            Ok(())
        })();
        match published {
            Ok(()) => drop(staging.keep()),
            Err(failure) => {
                staging.close().map_err(|error| {
                    format!("{failure}; go_cache_staging_cleanup_failed: {error}")
                })?;
                return Err(failure);
            }
        }
    }
    let executable = entry.join("tool");
    let held = fs::read(entry.join("sha256"))
        .map_err(|error| format!("go_cache_checksum_unreadable: {error}"))?;
    if executable_hash(&executable)? != held {
        return Err(format!("go_cache_checksum_mismatch: {}", executable.display()).into());
    }
    if source_hash(scaffold)? != source {
        return Err("go_cache_source_changed_before_use".into());
    }
    fs::copy(&executable, out)
        .map_err(|error| format!("go_cache_copy_failed: {}: {error}", out.display()))?;
    profile("cache", started);
    Ok(())
}

/// Prints phase costs only when the diagnostic explicitly requests them.
pub fn profile(phase: &str, started: Instant) {
    if std::env::var_os("LYS_GO_PROFILE").is_some() {
        eprintln!(
            "go_fixture phase={phase} elapsed_us={}",
            started.elapsed().as_micros()
        );
    }
}
