//! A locked, content-keyed template contains no issuer login or mutable service.

use std::collections::BTreeMap;
use std::error::Error;
use std::fs::{self, File};
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
            .and_then(|path| template_files::hash_file(&path))
            .map_err(|error| format!("service template executable hash: {error}"))
    })
    .as_ref()
    .map(String::as_str)
    .map_err(|error| error.as_str().into())
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

pub(crate) fn restore(config: &Config) -> Result<(), Failure> {
    let mut paths = template_stores::paths(config);
    // Existing application logs deliberately ignore the model file, including
    // tests that remove or corrupt that file after recording the model.
    match fs::symlink_metadata(config.apps_dir()) {
        Ok(_) => paths.retain(|(name, _)| *name != "apps"),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    let mut missing = false;
    for (_, target) in &paths {
        missing |= absent(target)?;
    }
    if !missing {
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
        for (name, _) in &paths {
            template_stores::build(name, &stores.join(name), config)?;
        }
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
