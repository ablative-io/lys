//! Native installers carry a private, single-use runner connection file.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::Path;
use std::process::Command;

use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::error::ServerError;

#[derive(Deserialize, Serialize, zeroize::Zeroize, zeroize::ZeroizeOnDrop)]
#[serde(deny_unknown_fields)]
pub(crate) struct Pending {
    pub(crate) server: String,
    pub(crate) machine: String,
    pub(crate) code: String,
}

fn refused(name: &str, words: impl std::fmt::Display) -> ServerError {
    ServerError::Runner {
        refusal: name.to_owned(),
        words: words.to_string(),
    }
}

fn pending_bytes(pending: &Pending) -> Result<Zeroizing<Vec<u8>>, ServerError> {
    let address = reqwest::Url::parse(&pending.server).map_err(|error| {
        refused(
            "installer_pending_invalid",
            format!("the runner server address is invalid: {error}"),
        )
    })?;
    if address.scheme() != "https"
        || address.host_str().is_none()
        || !address.username().is_empty()
        || address.password().is_some()
        || address.query().is_some()
        || address.fragment().is_some()
        || !pending.machine.starts_with("op-")
        || !(4..=128).contains(&pending.machine.len())
        || !pending
            .machine
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        || pending.code.is_empty()
        || pending.code.len() > 1024
        || pending.code.chars().any(char::is_control)
    {
        return Err(refused(
            "installer_pending_invalid",
            "the runner needs an https server, a machine id and a connection code",
        ));
    }
    let mut bytes = Zeroizing::new(Vec::new());
    serde_json::to_writer(&mut *bytes, pending).map_err(|error| {
        refused(
            "installer_pending_invalid",
            format!("the pending connection file could not be encoded: {error}"),
        )
    })?;
    Ok(bytes)
}

fn read_binary(path: &Path) -> Result<Vec<u8>, ServerError> {
    let bytes = fs::read(path).map_err(|error| refused("installer_binary_unreadable", error))?;
    if bytes.is_empty() {
        return Err(refused(
            "installer_binary_missing",
            "the installer needs both runner executables for every requested architecture",
        ));
    }
    Ok(bytes)
}

fn version(commit: &str, separator: &str) -> Result<String, ServerError> {
    if !(8..=64).contains(&commit.len()) || !commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(refused(
            "installer_commit_invalid",
            "the release needs a hexadecimal source commit of 8 to 64 characters",
        ));
    }
    Ok(format!(
        "{}{separator}g{}",
        env!("CARGO_PKG_VERSION"),
        &commit[..8]
    ))
}

fn octal(field: &mut [u8], value: u64) -> Result<(), ServerError> {
    let text = format!("{value:0width$o}\0", width = field.len() - 1);
    if text.len() != field.len() {
        return Err(refused(
            "installer_archive_limit",
            "an archive member exceeds the package format's size limit",
        ));
    }
    field.copy_from_slice(text.as_bytes());
    Ok(())
}

fn tar_file(tar: &mut Vec<u8>, path: &str, bytes: &[u8], mode: u32) -> Result<(), ServerError> {
    if path.len() > 100 || !path.is_ascii() {
        return Err(refused(
            "installer_archive_path",
            "a package member needs a short ASCII path",
        ));
    }
    let mut header = [0_u8; 512];
    header[..path.len()].copy_from_slice(path.as_bytes());
    octal(&mut header[100..108], u64::from(mode))?;
    octal(&mut header[108..116], 0)?;
    octal(&mut header[116..124], 0)?;
    octal(&mut header[124..136], bytes.len() as u64)?;
    octal(&mut header[136..148], 0)?;
    header[148..156].fill(b' ');
    header[156] = if path.ends_with('/') { b'5' } else { b'0' };
    header[257..263].copy_from_slice(b"ustar\0");
    header[263..265].copy_from_slice(b"00");
    header[265..269].copy_from_slice(b"root");
    header[297..301].copy_from_slice(b"root");
    let checksum: u64 = header.iter().map(|byte| u64::from(*byte)).sum();
    octal(&mut header[148..155], checksum)?;
    tar.extend_from_slice(&header);
    tar.extend_from_slice(bytes);
    let padded = tar.len() + (512 - bytes.len() % 512) % 512;
    tar.resize(padded, 0);
    Ok(())
}

fn gzip_tar(tar: Zeroizing<Vec<u8>>) -> Result<Vec<u8>, ServerError> {
    gzip_into(tar, Vec::new())
}

/// The ended tar, gzipped into `into`; a writer that refuses is named.
fn gzip_into<W: Write>(mut tar: Zeroizing<Vec<u8>>, into: W) -> Result<W, ServerError> {
    let ended = tar.len() + 1024;
    tar.resize(ended, 0);
    let mut gzip = flate2::GzBuilder::new()
        .mtime(0)
        .write(into, flate2::Compression::default());
    gzip.write_all(&tar)
        .map_err(|error| refused("installer_archive_failed", error))?;
    gzip.finish()
        .map_err(|error| refused("installer_archive_failed", error))
}

fn ar_file(ar: &mut Vec<u8>, name: &str, bytes: &[u8]) -> Result<(), ServerError> {
    let header = format!(
        "{name:<16}{:<12}{:<6}{:<6}{:<8}{:<10}`\n",
        0,
        0,
        0,
        "100644",
        bytes.len(),
    );
    if header.len() != 60 {
        return Err(refused(
            "installer_archive_limit",
            "an ar member exceeds the package format's size limit",
        ));
    }
    ar.extend_from_slice(header.as_bytes());
    ar.extend_from_slice(bytes);
    if bytes.len() % 2 == 1 {
        ar.push(b'\n');
    }
    Ok(())
}

const LINUX_INSTALL: &str = r#"set -eu
PATH=/usr/sbin:/usr/bin:/sbin:/bin
export PATH
pending=/usr/lib/lys/runner-pending.json
private=
fail() {
    printf '%s\n' "$1" 'The Lys package is installed, but its runner is not ready.' \
        'If this computer already joined, sign in as that person and retry: /usr/lib/lys/lys runner install' \
        'Otherwise, download and open a fresh installer from Network. A one-time connection code cannot be reused.' >&2
    exit 0
}
cleanup() {
    trap - 0 HUP INT TERM
    rm -f -- "$pending" || printf '%s\n' 'RunnerPendingRemovalFailed: the root connection file could not be removed' >&2
    if [ -n "$private" ]; then
        rm -rf -- "$private" || printf '%s\n' 'RunnerPendingRemovalFailed: the private connection folder could not be removed' >&2
    fi
    exit 0
}
trap cleanup 0
trap 'fail "RunnerInstallInterrupted: runner installation was interrupted"' HUP INT TERM
user=${SUDO_USER:-}
if [ -z "$user" ] && [ -n "${PKEXEC_UID:-}" ]; then
    case "$PKEXEC_UID" in *[!0-9]*) fail 'RunnerInstallUserLookupFailed: the graphical installer supplied an invalid user id';; esac
    record=$(getent passwd -- "$PKEXEC_UID") || fail 'RunnerInstallUserMissing: the graphical installer account was not found'
    user=${record%%:*}
fi
if [ -z "$user" ]; then
    users=$(loginctl list-users --no-legend --no-pager) || fail 'RunnerInstallSessionLookupFailed: logged-in accounts could not be read'
    count=0
    while read -r candidate_uid candidate rest; do
        case "$candidate_uid" in ''|*[!0-9]*) continue;; esac
        [ "$candidate_uid" -ge 1000 ] && [ "$candidate_uid" -lt 65534 ] || continue
        state=$(loginctl show-user "$candidate_uid" --property=State --value) || fail 'RunnerInstallSessionLookupFailed: a login session could not be read'
        case "$state" in active|online) ;; *) continue;; esac
        count=$((count + 1))
        user=$candidate
    done <<EOF
$users
EOF
    [ "$count" -le 1 ] || fail 'RunnerInstallUserAmbiguous: more than one human is logged in; run the installer from the intended account'
fi
[ -n "$user" ] || fail 'RunnerInstallUserMissing: no installing user or human local account was found'
record=$(getent passwd -- "$user") || fail 'RunnerInstallUserMissing: the installing account was not found'
IFS=: read -r account password uid gid description home shell <<EOF
$record
EOF
case "$home" in /*) ;; *) fail 'RunnerInstallHomeInvalid: the installing account has no absolute home folder';; esac
uid=$(id -u -- "$user") || fail 'RunnerInstallUserLookupFailed: the installing user id could not be read'
gid=$(id -g -- "$user") || fail 'RunnerInstallUserLookupFailed: the installing group id could not be read'
[ "$uid" -ne 0 ] || fail 'RunnerInstallUserMissing: the runner must belong to a human account, not root'
if [ -d "/run/user/$uid" ]; then
    staging="/run/user/$uid/lys-install.$$"
    [ ! -e "$staging" ] && [ ! -L "$staging" ] || fail 'RunnerPendingDirectoryExists: the private installation folder already exists'
    private=$staging
    install -d -m 700 -o "$uid" -g "$gid" "$private" || fail 'RunnerPendingDirectoryFailed: the private installation folder could not be made'
else
    private=$(runuser -u "$user" -- env HOME="$home" mktemp -d "$home/.lys-install.XXXXXXXX") \
        || fail 'RunnerPendingDirectoryFailed: the installing user could not make a private installation folder'
fi
join_file="$private/runner-pending.json"
install -m 600 -o "$uid" -g "$gid" "$pending" "$join_file" || fail 'RunnerPendingCopyFailed: the connection file could not be copied privately'
rm -f -- "$pending" || fail 'RunnerPendingRemovalFailed: the root connection file could not be removed'
if runuser -u "$user" -- env HOME="$home" USER="$user" LOGNAME="$user" \
    XDG_RUNTIME_DIR="/run/user/$uid" DBUS_SESSION_BUS_ADDRESS="unix:path=/run/user/$uid/bus" \
    /usr/lib/lys/lys runner install --pending "$join_file"; then
    exit 0
else
    result=$?
    fail "RunnerInstallFailed: connecting this computer or installing its login service failed (exit $result)"
fi
"#;

const MAC_POSTINSTALL: &str = r#"#!/bin/sh
set -eu
PATH=/usr/bin:/bin:/usr/sbin:/sbin
export PATH
pending='/Library/Application Support/Lys/runner-pending.json'
private=
fail() {
    printf '%s\n' "$1" 'The Lys package is installed, but its runner is not ready.' \
        'If this computer already joined, sign in as that person and retry: "/Library/Application Support/Lys/lys" runner install' \
        'Otherwise, download and open a fresh installer from Network. A one-time connection code cannot be reused.' >&2
    exit 0
}
cleanup() {
    trap - 0 HUP INT TERM
    rm -f -- "$pending" || printf '%s\n' 'RunnerPendingRemovalFailed: the root connection file could not be removed' >&2
    if [ -n "$private" ]; then
        rm -rf -- "$private" || printf '%s\n' 'RunnerPendingRemovalFailed: the private connection folder could not be removed' >&2
    fi
    exit 0
}
[ "${3:-/}" = / ] || fail 'RunnerInstallVolumeUnsupported: install Lys on the running system volume'
trap cleanup 0
trap 'fail "RunnerInstallInterrupted: runner installation was interrupted"' HUP INT TERM
user=$(/usr/bin/stat -f %Su /dev/console) || fail 'RunnerInstallUserLookupFailed: the console user could not be read'
case "$user" in ''|root|loginwindow|_mbsetupuser) fail 'RunnerInstallUserMissing: sign in to a Mac account before installing Lys';; esac
uid=$(/usr/bin/id -u "$user") || fail 'RunnerInstallUserLookupFailed: the console user id could not be read'
gid=$(/usr/bin/id -g "$user") || fail 'RunnerInstallUserLookupFailed: the console group id could not be read'
[ "$uid" -ne 0 ] || fail 'RunnerInstallUserMissing: the runner must belong to a human account, not root'
private=$(/usr/bin/mktemp -d "${TMPDIR:-/private/tmp}/lys-install.XXXXXXXX") || fail 'RunnerPendingDirectoryFailed: a private installation folder could not be made'
/usr/sbin/chown "$uid:$gid" "$private" || fail 'RunnerPendingOwnershipFailed: the private installation folder could not be handed to the console user'
/bin/chmod 700 "$private" || fail 'RunnerPendingPermissionsFailed: the installation folder could not be made private'
join_file="$private/runner-pending.json"
/usr/bin/install -m 600 -o "$uid" -g "$gid" "$pending" "$join_file" || fail 'RunnerPendingCopyFailed: the connection file could not be copied privately'
rm -f -- "$pending" || fail 'RunnerPendingRemovalFailed: the root connection file could not be removed'
if /bin/launchctl asuser "$uid" /usr/bin/sudo -H -u "$user" -- \
    '/Library/Application Support/Lys/lys' runner install --pending "$join_file"; then
    exit 0
else
    result=$?
    fail "RunnerInstallFailed: connecting this computer or installing its login service failed (exit $result)"
fi
"#;

fn linux_payload(
    pending: &[u8],
    lys: &[u8],
    entry: &[u8],
) -> Result<Zeroizing<Vec<u8>>, ServerError> {
    let mut tar = Zeroizing::new(Vec::new());
    for path in ["usr/", "usr/lib/", "usr/lib/lys/"] {
        tar_file(&mut tar, path, &[], 0o755)?;
    }
    tar_file(&mut tar, "usr/lib/lys/lys", lys, 0o755)?;
    tar_file(&mut tar, "usr/lib/lys/lys-containment-entry", entry, 0o755)?;
    tar_file(&mut tar, "usr/lib/lys/runner-pending.json", pending, 0o600)?;
    Ok(tar)
}

pub(crate) fn deb(
    pending: &Pending,
    lys: &Path,
    entry: &Path,
    commit: &str,
) -> Result<Vec<u8>, ServerError> {
    let version = version(commit, "+")?;
    let pending = pending_bytes(pending)?;
    let lys = read_binary(lys)?;
    let entry = read_binary(entry)?;
    let data = gzip_tar(linux_payload(&pending, &lys, &entry)?)?;
    let control = format!(
        "Package: lys\nVersion: {version}\nSection: net\nPriority: optional\nArchitecture: amd64\nMaintainer: Ablative <support@ablative.com.au>\nDepends: curl, systemd, util-linux, libc-bin\nDescription: Lys computer runner\n"
    );
    let postinst =
        format!("#!/bin/sh\ncase \"${{1:-}}\" in configure) ;; *) exit 0 ;; esac\n{LINUX_INSTALL}");
    let mut tar = Zeroizing::new(Vec::new());
    tar_file(&mut tar, "control", control.as_bytes(), 0o644)?;
    tar_file(&mut tar, "postinst", postinst.as_bytes(), 0o755)?;
    let control = gzip_tar(tar)?;
    let mut ar = b"!<arch>\n".to_vec();
    ar_file(&mut ar, "debian-binary", b"2.0\n")?;
    ar_file(&mut ar, "control.tar.gz", &control)?;
    ar_file(&mut ar, "data.tar.gz", &data)?;
    Ok(ar)
}

pub(crate) fn arch(
    pending: &Pending,
    lys: &Path,
    entry: &Path,
    commit: &str,
) -> Result<Vec<u8>, ServerError> {
    let version = version(commit, ".")?;
    let pending = pending_bytes(pending)?;
    let lys = read_binary(lys)?;
    let entry = read_binary(entry)?;
    let size = lys.len() + entry.len() + pending.len();
    let info = format!(
        "pkgname = lys\npkgbase = lys\npkgver = {version}-1\npkgdesc = Lys computer runner\nurl = https://ablative.com.au\nbuilddate = 0\npackager = Ablative\nsize = {size}\narch = x86_64\nlicense = Apache-2.0\ndepend = curl\ndepend = systemd\ndepend = util-linux\ndepend = glibc\n"
    );
    let install =
        format!("post_install() (\n{LINUX_INSTALL})\npost_upgrade() {{ post_install \"$@\"; }}\n");
    let mut tar = Zeroizing::new(Vec::new());
    tar_file(&mut tar, ".PKGINFO", info.as_bytes(), 0o644)?;
    tar_file(&mut tar, ".INSTALL", install.as_bytes(), 0o644)?;
    tar.extend_from_slice(&linux_payload(&pending, &lys, &entry)?);
    gzip_tar(tar)
}

fn private_file(path: &Path, bytes: &[u8], mode: u32) -> Result<(), ServerError> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .map_err(|error| refused("installer_staging_failed", error))?;
    file.write_all(bytes)
        .map_err(|error| refused("installer_staging_failed", error))?;
    file.set_permissions(fs::Permissions::from_mode(mode))
        .map_err(|error| refused("installer_staging_failed", error))
}

fn pkgbuild_command(root: &Path, version: &str) -> Command {
    let mut command = Command::new("/usr/bin/pkgbuild");
    command
        .arg("--root")
        .arg(root.join("payload"))
        .arg("--scripts")
        .arg(root.join("scripts"))
        .args([
            "--identifier",
            "au.com.ablative.lys-runner",
            "--version",
            version,
            "--install-location",
            "/",
            "--ownership",
            "recommended",
        ])
        .arg(root.join("component.pkg"));
    command
}

fn productbuild_command(root: &Path) -> Command {
    let mut command = Command::new("/usr/bin/productbuild");
    command
        .arg("--package")
        .arg(root.join("component.pkg"))
        .arg(root.join("lys.pkg"));
    command
}

/// The Mac installer carries the Apple silicon build only: Apple stopped selling Intel Macs years ago (Tom, 15 Oct).
pub(crate) fn macos_pkg(
    pending: &Pending,
    lys: &Path,
    entry: &Path,
    commit: &str,
) -> Result<Vec<u8>, ServerError> {
    let version = version(commit, "+")?;
    let pending = pending_bytes(pending)?;
    for name in ["pkgbuild", "productbuild"] {
        let path = format!("/usr/bin/{name}");
        let metadata = fs::metadata(&path).map_err(|error| {
            refused(
                &format!("installer_{name}_unavailable"),
                format!("the required {name} package tool is unavailable: {error}"),
            )
        })?;
        if !metadata.is_file() || metadata.permissions().mode() & 0o111 == 0 {
            return Err(refused(
                &format!("installer_{name}_unavailable"),
                format!("the required {name} package tool is not executable"),
            ));
        }
    }
    let directory = tempfile::Builder::new()
        .prefix("lys-runner-package-")
        .tempdir()
        .map_err(|error| refused("installer_staging_failed", error))?;
    let result = build_macos(
        directory.path(),
        &pending,
        [lys, entry],
        &version,
        |root, version| {
            run_tool(pkgbuild_command(root, version), "pkgbuild")?;
            run_tool(productbuild_command(root), "productbuild")
        },
    );
    settled(result, directory.close())
}

/// Stages the Mac package under `root` and has `package` (pkgbuild then
/// productbuild) write `root/lys.pkg`, which must be a flat xar installer.
fn build_macos(
    root: &Path,
    pending: &[u8],
    binaries: [&Path; 2],
    version: &str,
    package: impl FnOnce(&Path, &str) -> Result<(), ServerError>,
) -> Result<Vec<u8>, ServerError> {
    let payload = root.join("payload/Library/Application Support/Lys");
    for path in [root.join("scripts"), payload.clone()] {
        fs::create_dir_all(path).map_err(|error| refused("installer_staging_failed", error))?;
    }
    for directory in [
        "payload",
        "payload/Library",
        "payload/Library/Application Support",
        "payload/Library/Application Support/Lys",
    ] {
        fs::set_permissions(root.join(directory), fs::Permissions::from_mode(0o755))
            .map_err(|error| refused("installer_staging_failed", error))?;
    }
    private_file(
        &root.join("scripts/postinstall"),
        MAC_POSTINSTALL.as_bytes(),
        0o755,
    )?;
    private_file(&payload.join("runner-pending.json"), pending, 0o600)?;
    for (source, name) in binaries.into_iter().zip(["lys", "lys-containment-entry"]) {
        let output = payload.join(name);
        fs::copy(source, &output).map_err(|error| refused("installer_staging_failed", error))?;
        fs::set_permissions(output, fs::Permissions::from_mode(0o755))
            .map_err(|error| refused("installer_staging_failed", error))?;
    }
    package(root, version)?;
    let bytes = fs::read(root.join("lys.pkg"))
        .map_err(|error| refused("installer_package_unreadable", error))?;
    if !bytes.starts_with(b"xar!") {
        return Err(refused(
            "installer_package_invalid",
            "the package tool did not produce a flat xar installer",
        ));
    }
    Ok(bytes)
}

#[path = "packages_tools.rs"]
mod tools;
use tools::{run_tool, settled};

#[cfg(test)]
#[path = "packages_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "packages_refusals_tests.rs"]
mod refusals_tests;
