//! Verified executable-pair updates, durable rollback and failed-release retention.

use std::collections::BTreeMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::service::{self, io, refused, sync_dir};
use crate::commands::error::{CliError, CliResult};
use crate::commands::runner_join::SavedJoin;
use crate::identity::install::layout::{ClientRelease, Layout, host_platform};
use crate::identity::{private_files, upgrade::swap};

const NAMES: [&str; 2] = ["lys", "lys-containment-entry"];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Clients {
    platforms: BTreeMap<String, ClientRelease>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PendingUpdate {
    commit: String,
    files: BTreeMap<String, String>,
    previous: BTreeMap<String, String>,
    rejected: bool,
}

fn record_path(layout: &Layout) -> PathBuf {
    layout.run_dir().join("runner-update.json")
}

fn read(layout: &Layout) -> CliResult<Option<PendingUpdate>> {
    private_files::read(&record_path(layout))?
        .map(|bytes| {
            let record: PendingUpdate = serde_json::from_slice(&bytes).map_err(|error| {
                refused(
                    "runner_update_record_invalid",
                    format!(
                        "the saved runner update record cannot be read ({})",
                        service::json_detail(&error)
                    ),
                )
            })?;
            validate_files(&record.files)?;
            validate_files(&record.previous)?;
            if !commit_valid(&record.commit) {
                return Err(refused(
                    "runner_update_record_invalid",
                    "the saved runner update commit is invalid",
                ));
            }
            Ok(record)
        })
        .transpose()
}

fn write(layout: &Layout, record: &PendingUpdate) -> CliResult<()> {
    let bytes = serde_json::to_vec(record).map_err(|error| {
        refused(
            "runner_update_record_invalid",
            format!(
                "the runner update record could not be encoded ({})",
                service::json_detail(&error)
            ),
        )
    })?;
    private_files::write(&record_path(layout), &bytes)?;
    Ok(())
}

fn commit_valid(commit: &str) -> bool {
    commit.len() == 40
        && commit
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn stamp_commit(stamp: &str) -> CliResult<&str> {
    let (commit, suffix) = stamp.split_once(';').unwrap_or((stamp, ""));
    if !commit_valid(commit) || !matches!(suffix, "" | " stated") {
        return Err(refused(
            "runner_update_build_untracked",
            "automatic runner updates require a clean build stamped with its source commit",
        ));
    }
    Ok(commit)
}

fn validate_files(files: &BTreeMap<String, String>) -> CliResult<()> {
    if files.len() != NAMES.len()
        || !NAMES.iter().all(|name| {
            files.get(*name).is_some_and(|digest| {
                digest.len() == 64
                    && digest
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            })
        })
    {
        return Err(refused(
            "runner_update_manifest_invalid",
            "the release must record lys and lys-containment-entry with their SHA-256 digests",
        ));
    }
    Ok(())
}

fn digest(path: &Path) -> CliResult<String> {
    use std::io::Read;
    let mut file = File::open(path).map_err(|error| io("open executable", path, error))?;
    let mut hash = Sha256::new();
    let mut buffer = vec![0_u8; 32_768].into_boxed_slice();
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|error| io("read executable", path, error))?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

fn verify(path: &Path, expected: &str) -> CliResult<()> {
    if digest(path)? != expected {
        return Err(refused(
            "runner_update_digest_mismatch",
            format!("{} differs from its recorded SHA-256", path.display()),
        ));
    }
    Ok(())
}

fn sync_file(path: &Path) -> CliResult<()> {
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|error| io("sync executable", path, error))
}

fn lock(layout: &Layout) -> CliResult<File> {
    private_files::ensure_dir(&layout.run_dir())?;
    let path = layout.run_dir().join("runner-update.lock");
    let mut options = std::fs::OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options
        .open(&path)
        .map_err(|error| io("open update lock", &path, error))?;
    drop(private_files::read(&path)?);
    #[cfg(unix)]
    rustix::fs::flock(&file, rustix::fs::FlockOperation::NonBlockingLockExclusive).map_err(
        |error| {
            refused(
                "runner_update_busy",
                format!("another runner update holds the update lock: {error}"),
            )
        },
    )?;
    #[cfg(not(unix))]
    return Err(refused(
        "runner_update_platform_unsupported",
        "runner updates need macOS or Linux",
    ));
    Ok(file)
}

fn fetch(saved: &SavedJoin, route: &str, path: &Path, limit: u64) -> CliResult<()> {
    private_files::write(path, b"")?;
    let mut command = Command::new("/usr/bin/curl");
    command
        .args([
            "--disable",
            "--fail",
            "--silent",
            "--show-error",
            "--proto",
            "=https",
            "--tlsv1.2",
            "--max-filesize",
        ])
        .arg(limit.to_string())
        .arg("--output")
        .arg(path);
    if let Some(authority) = &saved.server_ca {
        command.arg("--cacert").arg(authority);
    }
    let url = format!("{}/{route}", saved.server.trim_end_matches('/'));
    let output = command.arg("--url").arg(url).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::piped()).output()
        .map_err(|error| refused("runner_update_unreachable", format!("the update downloader could not start; the installer needs /usr/bin/curl: {error}")))?;
    if !output.status.success() {
        return Err(download_failure(output.status.code()));
    }
    let size = std::fs::metadata(path)
        .map_err(|error| io("measure download", path, error))?
        .len();
    if size == 0 || size > limit {
        return Err(refused(
            "runner_update_download_invalid",
            "the download is empty or exceeds the release size limit",
        ));
    }
    Ok(())
}

fn download_failure(code: Option<i32>) -> CliError {
    if code == Some(63) {
        return refused(
            "runner_update_download_invalid",
            "the runner download exceeds its size limit",
        );
    }
    let reason = match code {
        Some(5 | 6) => "the network could not find the Lys server",
        Some(7) => "the Lys server did not accept a connection",
        Some(22) => "the Lys server did not serve the requested release",
        Some(28) => "the connection to the Lys server expired",
        Some(35 | 60) => "a verified TLS connection to the Lys server could not be established",
        _ => "the transfer from the Lys server did not finish",
    };
    refused(
        "runner_update_unreachable",
        format!("{reason}; the installed runner was kept (download exit {code:?})"),
    )
}

fn release(bytes: &[u8], platform: &str) -> CliResult<ClientRelease> {
    let clients: Clients = serde_json::from_slice(bytes).map_err(|error| {
        refused(
            "runner_update_manifest_invalid",
            format!(
                "the server's client list is invalid ({})",
                service::json_detail(&error)
            ),
        )
    })?;
    let release = clients.platforms.get(platform).ok_or_else(|| {
        refused(
            "runner_update_platform_missing",
            format!("the server has no runner release for {platform}"),
        )
    })?;
    if !commit_valid(&release.commit) {
        return Err(refused(
            "runner_update_manifest_invalid",
            "the release does not name a source commit",
        ));
    }
    validate_files(&release.files)?;
    Ok(release.clone())
}

fn stage(layout: &Layout, release: &ClientRelease, source: &Path) -> CliResult<()> {
    validate_files(&release.files)?;
    let mut previous = BTreeMap::new();
    for name in NAMES {
        verify(&source.join(name), &release.files[name])?;
        previous.insert(name.to_owned(), digest(&layout.binary(name))?);
    }
    for name in NAMES {
        swap::place_binary(
            &layout.binary(name),
            &layout.bin_dir(),
            &format!("{name}.previous"),
        )?;
        verify(
            &layout.bin_dir().join(format!("{name}.previous")),
            &previous[name],
        )?;
        sync_file(&layout.bin_dir().join(format!("{name}.previous")))?;
    }
    sync_dir(&layout.bin_dir())?;
    let record = PendingUpdate {
        commit: release.commit.clone(),
        files: release.files.clone(),
        previous,
        rejected: false,
    };
    write(layout, &record)?;
    let placed = (|| {
        for name in NAMES {
            swap::place_binary(&source.join(name), &layout.bin_dir(), name)?;
            sync_file(&layout.binary(name))?;
        }
        sync_dir(&layout.bin_dir())
    })();
    if let Err(error) = placed {
        return match rollback_locked(layout) {
            Ok(_) => Err(error),
            Err(again) => Err(refused(
                "runner_update_rollback_failed",
                format!("{error}; restoring the previous runner also failed: {again}"),
            )),
        };
    }
    Ok(())
}

fn installed_matches(layout: &Layout, files: &BTreeMap<String, String>) -> CliResult<bool> {
    for name in NAMES {
        let path = layout.binary(name);
        if !path
            .try_exists()
            .map_err(|error| io("find executable", &path, error))?
            || digest(&path)? != files[name]
        {
            return Ok(false);
        }
    }
    Ok(true)
}

fn rollback_locked(layout: &Layout) -> CliResult<bool> {
    let Some(mut record) = read(layout)? else {
        return Ok(false);
    };
    if record.rejected {
        return Ok(false);
    }
    for name in NAMES {
        verify(
            &layout.bin_dir().join(format!("{name}.previous")),
            &record.previous[name],
        )?;
    }
    for name in NAMES {
        swap::place_binary(
            &layout.bin_dir().join(format!("{name}.previous")),
            &layout.bin_dir(),
            name,
        )?;
        sync_file(&layout.binary(name))?;
    }
    sync_dir(&layout.bin_dir())?;
    record.rejected = true;
    write(layout, &record)?;
    service::say(
        "runner_update_rolled_back: the new runner did not become ready; the previous executable pair was restored",
    )?;
    Ok(true)
}

pub(crate) fn rollback(layout: &Layout) -> CliResult<bool> {
    let _lock = lock(layout)?;
    rollback_locked(layout)
}

/// Finish an interrupted pair swap before either executable is started.
pub(crate) fn recover(layout: &Layout) -> CliResult<bool> {
    let _lock = lock(layout)?;
    let Some(record) = read(layout)? else {
        return Ok(false);
    };
    if !record.rejected && !installed_matches(layout, &record.files)? {
        return rollback_locked(layout);
    }
    Ok(false)
}

/// Accept readiness only from the release that was downloaded and verified.
pub(crate) fn ready(layout: &Layout, stamp: &str) -> CliResult<()> {
    let _lock = lock(layout)?;
    let Some(record) = read(layout)? else {
        return Ok(());
    };
    if record.rejected {
        return Ok(());
    }
    if stamp_commit(stamp)? != record.commit || !installed_matches(layout, &record.files)? {
        return Err(refused(
            "runner_update_ready_mismatch",
            "the ready runner is not the verified release; restoring the previous runner",
        ));
    }
    let controller = layout.bin_dir().join("runner-service");
    private_files::ensure_dir(&controller)?;
    swap::place_binary(&layout.binary("lys"), &controller, "lys")?;
    sync_file(&controller.join("lys"))?;
    sync_dir(&controller)?;
    let path = record_path(layout);
    std::fs::remove_file(&path).map_err(|error| io("complete runner update", &path, error))?;
    sync_dir(&layout.run_dir())
}

/// Fetch and verify both executables before changing either installed file.
pub(crate) fn prepare(layout: &Layout) -> CliResult<bool> {
    prepare_with(layout, &mut fetch)
}

fn prepare_with(
    layout: &Layout,
    download: &mut dyn FnMut(&SavedJoin, &str, &Path, u64) -> CliResult<()>,
) -> CliResult<bool> {
    let _lock = lock(layout)?;
    let pending = read(layout)?;
    if pending.as_ref().is_some_and(|record| !record.rejected) {
        service::say("The verified runner update is waiting for service readiness.")?;
        return Ok(true);
    }
    let saved = SavedJoin::read(layout)?;
    let next = layout.bin_dir().join("runner-next");
    private_files::ensure_dir(&next)?;
    let result = (|| {
        let manifest = next.join("clients.json");
        download(&saved, "clients", &manifest, 1_048_576)?;
        let bytes =
            std::fs::read(&manifest).map_err(|error| io("read client list", &manifest, error))?;
        let release = release(&bytes, &host_platform())?;
        if pending
            .as_ref()
            .is_some_and(|record| record.rejected && record.commit == release.commit)
        {
            service::say(
                "runner_update_release_rejected: this release failed readiness; the previous runner is retained until a different release is available",
            )?;
            return Ok(false);
        }
        let installed = crate::identity::upgrade::version(&layout.binary("lys"), "lys")?;
        if stamp_commit(&installed)? == release.commit && installed_matches(layout, &release.files)?
        {
            service::say("The runner is already on the current release.")?;
            return Ok(false);
        }
        for name in NAMES {
            download(
                &saved,
                &format!("clients/{}/{name}", host_platform()),
                &next.join(name),
                536_870_912,
            )?;
            verify(&next.join(name), &release.files[name])?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(next.join(name), std::fs::Permissions::from_mode(0o700))
                    .map_err(|error| io("make runner executable", &next.join(name), error))?;
            }
        }
        stage(layout, &release, &next)?;
        Ok(true)
    })();
    let cleanup =
        std::fs::remove_dir_all(&next).map_err(|error| io("remove update downloads", &next, error));
    match (result, cleanup) {
        (Err(error), Err(cleanup)) => Err(refused(
            "runner_update_cleanup_failed",
            format!("{error}; {cleanup}"),
        )),
        (Err(error), Ok(())) if matches!(&error, CliError::Runner(error) if error.name() == "runner_update_unreachable") =>
        {
            service::say(&error.to_string())?;
            Ok(false)
        }
        (Err(error), Ok(())) | (Ok(_), Err(error)) => Err(error),
        (Ok(updated), Ok(())) => Ok(updated),
    }
}

/// The service's pre-start update never restarts the service containing it.
pub(crate) fn run() -> CliResult<()> {
    let layout = Layout::discover()?;
    let service = service::Service::discover(&layout)?;
    service.require_installed()?;
    recover(&layout)?;
    if !prepare(&layout)? {
        return Ok(());
    }
    if std::env::var_os("LYS_RUNNER_SERVICE").is_none() {
        if let Err(error) = service.restart() {
            return match rollback(&layout) {
                Ok(_) => Err(error),
                Err(again) => Err(refused(
                    "runner_update_rollback_failed",
                    format!("{error}; {again}"),
                )),
            };
        }
    }
    service::say(
        "The verified runner update is installed. The service will restore the previous runner if readiness fails.",
    )
}

#[cfg(test)]
#[path = "runner_update_tests.rs"]
mod tests;
