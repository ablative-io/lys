//! A locked, content-keyed template contains no issuer login or mutable service.

use std::collections::BTreeMap;
use std::error::Error;
use std::fs::{self, File};
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Instant;

use lys_identity_server::Config;
use sha2::{Digest, Sha256};

use crate::{template_files, template_stores};

type Failure = Box<dyn Error>;

fn executable_hash() -> Result<&'static str, Failure> {
    static HASH: OnceLock<Result<String, String>> = OnceLock::new();
    HASH.get_or_init(|| {
        std::env::current_exe()
            .and_then(|path| program_hash(&path))
            .map_err(|error| format!("service template executable hash: {error}"))
    })
    .as_ref()
    .map(String::as_str)
    .map_err(|error| error.as_str().into())
}

fn program_hash(path: &Path) -> io::Result<String> {
    let mut input = io::BufReader::with_capacity(256 * 1024, File::open(path)?);
    let mut hash = ring::digest::Context::new(&ring::digest::SHA256);
    loop {
        let bytes = input.fill_buf()?;
        if bytes.is_empty() {
            break;
        }
        hash.update(bytes);
        let count = bytes.len();
        input.consume(count);
    }
    let digest = hash.finish();
    let mut encoded = String::with_capacity(64);
    for byte in digest.as_ref() {
        std::fmt::Write::write_fmt(&mut encoded, format_args!("{byte:02x}"))
            .map_err(io::Error::other)?;
    }
    Ok(encoded)
}

fn cache_dir() -> Result<PathBuf, Failure> {
    let executable = std::env::current_exe()?;
    let deps = executable
        .parent()
        .ok_or("test executable has no directory")?;
    if deps.file_name().is_none_or(|name| name != "deps") {
        return Err("service template requires a Cargo test executable under deps".into());
    }
    let target = deps
        .parent()
        .and_then(Path::parent)
        .ok_or("test executable has no target directory")?;
    Ok(target.join("tmp").join("service-templates"))
}

fn fingerprint(config: &Config, paths: &[(&str, PathBuf)]) -> Result<String, Failure> {
    let mut hash = Sha256::new();
    hash.update(executable_hash()?);
    hash.update(include_bytes!("harness.rs"));
    hash.update(include_bytes!("harness_serve.rs"));
    hash.update(include_bytes!("service_template.rs"));
    hash.update(include_bytes!("template_stores.rs"));
    hash.update(include_bytes!("template_files.rs"));
    let zone = jiff::tz::TimeZone::try_system()?;
    hash.update(serde_json::to_vec(&(
        &config.log_origin,
        paths.iter().map(|(name, _)| name).collect::<Vec<_>>(),
        zone.iana_name()
            .ok_or("service template requires a named host time zone")?,
        if paths.iter().any(|(name, _)| *name == "apps") {
            Some(fs::read(&config.grant_model_file)?)
        } else {
            None
        },
        fs::read(&config.event_key_file)?,
    ))?);
    Ok(format!("{:x}", hash.finalize()))
}

fn absent(path: &Path) -> std::io::Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(false),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(true),
        Err(error) => Err(error),
    }
}

fn missing_targets(paths: Vec<(&str, PathBuf)>) -> std::io::Result<Vec<(&str, PathBuf)>> {
    let mut missing = Vec::with_capacity(paths.len());
    for (name, target) in paths {
        if absent(&target)? {
            missing.push((name, target));
        }
    }
    Ok(missing)
}

fn build_stores(paths: &[(&str, PathBuf)], stores: &Path, config: &Config) -> Result<(), Failure> {
    let lanes = paths.len().min(4);
    let mut failures = Vec::new();
    std::thread::scope(|scope| {
        let mut workers = Vec::with_capacity(lanes);
        for lane in 0..lanes {
            let worker = std::thread::Builder::new()
                .name(format!("fixture-store-{lane}"))
                .spawn_scoped(scope, move || {
                    let mut failures = Vec::new();
                    for (name, _) in paths.iter().skip(lane).step_by(lanes) {
                        if let Err(error) = template_stores::build(name, &stores.join(name), config)
                        {
                            failures.push(format!("service_template_store_failed {name}: {error}"));
                        }
                    }
                    failures
                });
            match worker {
                Ok(worker) => {
                    workers.push(worker);
                }
                Err(error) => {
                    failures.push(format!("service_template_worker_start_failed: {error}"));
                }
            }
        }
        for worker in workers {
            match worker.join() {
                Ok(errors) => {
                    failures.extend(errors);
                }
                Err(error) => {
                    failures.push(format!("service_template_worker_panicked: {error:?}"));
                }
            }
        }
    });
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("; ").into())
    }
}

pub(crate) fn restore(config: &Config) -> Result<(), Failure> {
    let mut paths = template_stores::paths(config);
    // Existing application logs deliberately ignore the model file, including
    // tests that remove or corrupt that file after recording the model.
    match fs::symlink_metadata(config.apps_dir()) {
        Ok(_) => paths.retain(|(name, _)| *name != "apps"),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    let paths = missing_targets(paths)?;
    if paths.is_empty() {
        return Ok(());
    }
    let key = fingerprint(config, &paths)?;
    let cache = cache_dir()?;
    fs::create_dir_all(&cache)?;
    let lock = File::options()
        .create(true)
        .truncate(false)
        .write(true)
        .open(cache.join(format!("{key}.lock")))?;
    rustix::fs::flock(&lock, rustix::fs::FlockOperation::LockExclusive)?;
    let ready = cache.join(&key);
    if !ready.try_exists()? {
        let started = Instant::now();
        let stage = tempfile::Builder::new()
            .prefix("building-")
            .tempdir_in(&cache)?;
        let stores = stage.path().join("stores");
        fs::create_dir(&stores)?;
        build_stores(&paths, &stores, config)?;
        let files = template_files::inventory(&stores)?;
        fs::write(
            stage.path().join("manifest.json"),
            serde_json::to_vec(&files)?,
        )?;
        fs::rename(stage.path(), &ready)?;
        eprintln!(
            "service template: built {} stores in {:?}",
            paths.len(),
            started.elapsed()
        );
    }
    let expected: BTreeMap<String, String> =
        serde_json::from_slice(&fs::read(ready.join("manifest.json"))?)?;
    if template_files::inventory(&ready.join("stores"))? != expected {
        return Err(format!("service template {key} failed its file inventory check").into());
    }
    drop(lock);
    #[cfg(target_os = "macos")]
    eprintln!("service template: clonefile copies; unsupported filesystems report a fallback");
    #[cfg(not(target_os = "macos"))]
    eprintln!("service template: recursive-copy fallback on this platform");
    for (name, target) in paths {
        // Preparation owns every path it creates, including deliberately broken
        // stores and old formats used by recovery tests.
        if absent(&target)? {
            let parent = target
                .parent()
                .ok_or("template destination has no parent")?;
            fs::create_dir_all(parent)?;
            template_files::copy_tree(&ready.join("stores").join(name), &target)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn program_hash_reads_every_byte_across_its_buffer_boundary() -> io::Result<()> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("program");
        let mut bytes = vec![13; 256 * 1024 * 2 + 1];
        for content in [&b""[..], &b"abc"[..], &bytes] {
            fs::write(&path, content)?;
            assert_eq!(program_hash(&path)?, template_files::hash_file(&path)?);
        }
        bytes[256 * 1024 * 2] = 17;
        fs::write(&path, bytes)?;
        assert_eq!(program_hash(&path)?, template_files::hash_file(&path)?);
        Ok(())
    }

    #[test]
    fn caller_prepared_stores_are_not_rebuilt() -> std::io::Result<()> {
        let dir = tempfile::tempdir()?;
        let prepared = dir.path().join("prepared");
        fs::create_dir(&prepared)?;
        fs::write(prepared.join("snapshot.bin"), b"old format")?;
        let broken = dir.path().join("broken");
        fs::write(&broken, b"invalid store")?;
        let link = dir.path().join("link");
        std::os::unix::fs::symlink("missing", &link)?;
        let missing = dir.path().join("new");
        let paths = missing_targets(vec![
            ("prepared", prepared.clone()),
            ("broken", broken.clone()),
            ("link", link.clone()),
            ("new", missing.clone()),
        ])?;
        assert_eq!(paths, vec![("new", missing)]);
        assert_eq!(fs::read(prepared.join("snapshot.bin"))?, b"old format");
        assert_eq!(fs::read(broken)?, b"invalid store");
        assert_eq!(fs::read_link(link)?, Path::new("missing"));
        Ok(())
    }

    #[test]
    fn target_metadata_failure_is_not_an_absent_store() -> std::io::Result<()> {
        let dir = tempfile::tempdir()?;
        let file = dir.path().join("file");
        fs::write(&file, b"regular file")?;
        let result = missing_targets(vec![
            ("new", dir.path().join("new")),
            ("invalid", file.join("child")),
        ]);
        assert_eq!(
            result.err().map(|error| error.kind()),
            Some(std::io::ErrorKind::NotADirectory)
        );
        Ok(())
    }

    #[test]
    fn prepared_install_needs_no_template() -> std::io::Result<()> {
        let dir = tempfile::tempdir()?;
        assert!(missing_targets(vec![("prepared", dir.path().to_owned())])?.is_empty());
        Ok(())
    }
}
