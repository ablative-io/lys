#![cfg(test)]
#![cfg(unix)]
//! Cache failures propagate, release the lock, and leave no partial publication.

#[path = "harness/cache.rs"]
mod cache;

use std::cell::Cell;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

type Outcome = Result<(), Box<dyn std::error::Error>>;

struct Fixture {
    home: tempfile::TempDir,
    go: PathBuf,
    scaffold: PathBuf,
    target: PathBuf,
    output: PathBuf,
}

impl Fixture {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let home = tempfile::tempdir()?;
        let go = home.path().join("go");
        fs::write(
            &go,
            b"#!/bin/sh\nprintf '%s\\n' 'cache environment fixture'\n",
        )?;
        fs::set_permissions(&go, fs::Permissions::from_mode(0o700))?;
        let scaffold = home.path().join("scaffold");
        fs::create_dir(&scaffold)?;
        fs::write(scaffold.join("main.go"), b"package main\n")?;
        let target = home.path().join("cache");
        let output = home.path().join("output");
        Ok(Self {
            home,
            go,
            scaffold,
            target,
            output,
        })
    }

    fn build(
        &self,
        compile: impl FnOnce(&Path, &Path) -> cache::CacheResult<()>,
    ) -> cache::CacheResult<()> {
        cache::build(
            &self.go,
            &self.scaffold,
            &self.output,
            &self.target,
            compile,
        )
    }

    fn published(&self) -> Result<PathBuf, Box<dyn std::error::Error>> {
        for entry in fs::read_dir(self.target.join("lys-go-conformance-v1"))? {
            let entry = entry?;
            if entry.file_type()?.is_dir() && entry.file_name() != "gocache" {
                return Ok(entry.path());
            }
        }
        Err("cache entry was not published".into())
    }
}

fn compile(executable: &Path, compiler_cache: &Path) -> cache::CacheResult<()> {
    assert!(compiler_cache.is_dir());
    fs::write(executable, b"compiled fixture")?;
    Ok(())
}

#[test]
fn compiler_failure_returns_its_error_cleans_staging_and_releases_the_lock() -> Outcome {
    let fixture = Fixture::new()?;
    let failure = fixture
        .build(|_, _| Err("injected compiler failure".into()))
        .unwrap_err();
    assert_eq!(failure.to_string(), "injected compiler failure");
    assert!(!fixture.output.try_exists()?);
    assert!(fixture.published().is_err());
    fixture.build(compile)?;
    assert_eq!(fs::read(&fixture.output)?, b"compiled fixture");
    let published = fixture.published()?;
    assert!(published.join("sha256").is_file());
    assert_eq!(
        fs::read_dir(fixture.target.join("lys-go-conformance-v1"))?.count(),
        3
    );
    Ok(())
}

#[test]
fn corrupt_executable_is_a_named_error_and_is_not_rebuilt_or_copied() -> Outcome {
    let fixture = Fixture::new()?;
    fixture.build(compile)?;
    fs::write(fixture.published()?.join("tool"), b"corrupt")?;
    fs::remove_file(&fixture.output)?;
    let calls = Cell::new(0);
    let failure = fixture
        .build(|executable, compiler_cache| {
            calls.set(calls.get() + 1);
            compile(executable, compiler_cache)
        })
        .unwrap_err();
    assert!(failure.to_string().contains("go_cache_checksum_mismatch"));
    assert_eq!(calls.get(), 0);
    assert!(!fixture.output.try_exists()?);
    Ok(())
}

#[test]
fn changed_source_is_refused_before_publication_and_the_next_build_can_succeed() -> Outcome {
    let fixture = Fixture::new()?;
    let failure = fixture
        .build(|executable, compiler_cache| {
            fs::write(fixture.scaffold.join("main.go"), b"package changed\n")?;
            compile(executable, compiler_cache)
        })
        .unwrap_err();
    assert_eq!(failure.to_string(), "go_cache_source_changed_during_build");
    assert!(!fixture.output.try_exists()?);
    assert!(fixture.published().is_err());
    fixture.build(compile)?;
    assert_eq!(fs::read(&fixture.output)?, b"compiled fixture");
    Ok(())
}

#[test]
fn missing_toolchain_returns_a_named_error_without_calling_the_compiler() -> Outcome {
    let fixture = Fixture::new()?;
    let calls = Cell::new(0);
    let failure = cache::build(
        &fixture.home.path().join("missing-go"),
        &fixture.scaffold,
        &fixture.output,
        &fixture.target,
        |_, _| {
            calls.set(calls.get() + 1);
            Ok(())
        },
    )
    .unwrap_err();
    assert!(
        failure
            .to_string()
            .contains("go_cache_toolchain_spawn_failed")
    );
    assert_eq!(calls.get(), 0);
    assert!(!fixture.output.try_exists()?);
    Ok(())
}
