//! Independent Git provenance and explicit reasons for a moving checkout.

use std::error::Error;
use std::process::{Command, Output};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

const NO_COMMIT: &str = "not built from a git commit";

fn git(args: &[&str]) -> TestResult<Output> {
    Command::new("git")
        .arg("--no-optional-locks")
        .args(args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .map_err(|error| format!("git {args:?} could not read build provenance: {error}").into())
}

fn refused(args: &[&str], output: &Output) -> Box<dyn Error> {
    format!(
        "git {args:?} refused build provenance with {}: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr).trim()
    )
    .into()
}

fn head() -> TestResult<String> {
    let args = ["ls-files", "--error-unmatch", "Cargo.toml"];
    let tracked = git(&args)?;
    if !tracked.status.success() {
        let stderr = String::from_utf8(tracked.stderr.clone())?;
        if tracked.status.code() == Some(1)
            || (tracked.status.code() == Some(128) && stderr.contains("not a git repository"))
        {
            return Ok(NO_COMMIT.to_string());
        }
        return Err(refused(&args, &tracked));
    }
    let args = ["rev-parse", "HEAD"];
    let output = git(&args)?;
    if !output.status.success() {
        return Err(refused(&args, &output));
    }
    let commit = String::from_utf8(output.stdout)?.trim().to_string();
    if commit.len() != 40
        || !commit
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
    {
        return Err(format!("git HEAD is not a 40-character lowercase commit: {commit}").into());
    }
    Ok(commit)
}

/// The stamp describes build time; a changed checkout invalidates this run.
pub fn at_build() -> TestResult<String> {
    let current = head()?;
    let stamp = env!("LYS_BUILD");
    let stamped = stamp.split_once(';').map_or(stamp, |(commit, _)| commit);
    if current != stamped {
        return Err(format!(
            "git_head_moved_since_build: test build was stamped `{stamp}`, current Git HEAD is \
             `{current}`; the build stamp names build time, so this run cannot compare it with \
             a later HEAD"
        )
        .into());
    }
    Ok(current)
}

/// Re-read after the answer so movement within a test gets its own reason.
pub fn unchanged(expected: &str) -> TestResult {
    let current = head()?;
    if current != expected {
        return Err(format!(
            "git_head_moved_during_test: Git HEAD was `{expected}` before the answer and is \
             `{current}` after it; this run cannot compare builds across a moving HEAD"
        )
        .into());
    }
    Ok(())
}
