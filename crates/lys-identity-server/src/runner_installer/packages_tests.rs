#![cfg(test)]

use std::collections::BTreeMap;
use std::error::Error;
use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(target_os = "macos")]
use super::run_tool;
use super::{
    LINUX_INSTALL, MAC_POSTINSTALL, Pending, arch, deb, lipo_command, pending_bytes,
    pkgbuild_command, private_file, productbuild_command, version,
};

const COMMIT: &str = "0123456789abcdef";

fn pending() -> Pending {
    Pending {
        server: "https://lys.example/api".to_owned(),
        machine: "op-fixture".to_owned(),
        code: "unused-fixture".to_owned(),
    }
}

fn binaries() -> Result<(tempfile::TempDir, PathBuf, PathBuf), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let lys = directory.path().join("lys");
    let entry = directory.path().join("lys-containment-entry");
    std::fs::write(&lys, b"runner")?;
    std::fs::write(&entry, b"containment")?;
    Ok((directory, lys, entry))
}

struct Member {
    mode: u32,
    bytes: Vec<u8>,
}

fn ar_members(bytes: &[u8]) -> Result<Vec<(String, Vec<u8>)>, Box<dyn Error>> {
    assert!(bytes.starts_with(b"!<arch>\n"));
    let mut offset = 8;
    let mut members = Vec::new();
    while offset < bytes.len() {
        let header = bytes.get(offset..offset + 60).ok_or("short ar header")?;
        assert_eq!(&header[58..], b"`\n");
        let name = std::str::from_utf8(&header[..16])?
            .trim_end()
            .trim_end_matches('/')
            .to_owned();
        let size: usize = std::str::from_utf8(&header[48..58])?.trim().parse()?;
        offset += 60;
        let member = bytes.get(offset..offset + size).ok_or("short ar member")?;
        members.push((name, member.to_vec()));
        offset += size;
        if size % 2 == 1 {
            assert_eq!(bytes.get(offset), Some(&b'\n'));
            offset += 1;
        }
    }
    Ok(members)
}

fn tar_members(bytes: &[u8]) -> Result<BTreeMap<String, Member>, Box<dyn Error>> {
    let mut tar = Vec::new();
    flate2::read::GzDecoder::new(bytes).read_to_end(&mut tar)?;
    assert_eq!(tar.len() % 512, 0);
    let mut members = BTreeMap::new();
    let mut offset = 0;
    loop {
        let header = tar.get(offset..offset + 512).ok_or("short tar header")?;
        if header.iter().all(|byte| *byte == 0) {
            assert!(tar[offset..].iter().all(|byte| *byte == 0));
            assert!(tar.len() - offset >= 1024);
            break;
        }
        assert_eq!(&header[257..263], b"ustar\0");
        assert_eq!(&header[263..265], b"00");
        let field = |start, end| -> Result<&str, Box<dyn Error>> {
            Ok(std::str::from_utf8(&header[start..end])?.trim_matches(['\0', ' ']))
        };
        let stored = u32::from_str_radix(field(148, 156)?, 8)?;
        let sum: u32 = header
            .iter()
            .enumerate()
            .map(|(index, byte)| {
                if (148..156).contains(&index) {
                    32
                } else {
                    u32::from(*byte)
                }
            })
            .sum();
        assert_eq!(stored, sum);
        assert_eq!(field(108, 116)?, "0000000");
        assert_eq!(field(116, 124)?, "0000000");
        let path = field(0, 100)?.to_owned();
        assert_eq!(header[156], if path.ends_with('/') { b'5' } else { b'0' });
        let mode = u32::from_str_radix(field(100, 108)?, 8)?;
        let size = usize::from_str_radix(field(124, 136)?, 8)?;
        offset += 512;
        let contents = tar.get(offset..offset + size).ok_or("short tar member")?;
        assert!(
            members
                .insert(
                    path,
                    Member {
                        mode,
                        bytes: contents.to_vec()
                    }
                )
                .is_none()
        );
        offset += size.div_ceil(512) * 512;
    }
    Ok(members)
}

#[test]
fn deb_has_ordered_members_metadata_executables_and_private_pending() -> Result<(), Box<dyn Error>>
{
    let (directory, lys, entry) = binaries()?;
    let bytes = deb(&pending(), &lys, &entry, COMMIT)?;
    let ar = ar_members(&bytes)?;
    assert_eq!(
        ar.iter().map(|(name, _)| name.as_str()).collect::<Vec<_>>(),
        ["debian-binary", "control.tar.gz", "data.tar.gz"]
    );
    assert_eq!(ar[0].1, b"2.0\n");
    let control = tar_members(&ar[1].1)?;
    let fields = std::str::from_utf8(&control["control"].bytes)?;
    assert!(fields.contains(&format!(
        "Version: {}+g01234567\n",
        env!("CARGO_PKG_VERSION")
    )));
    for expected in [
        "Package: lys\n",
        "Architecture: amd64\n",
        "Depends: curl, systemd, util-linux, libc-bin\n",
        "Description: Lys computer runner\n",
    ] {
        assert!(fields.contains(expected));
    }
    assert_eq!(control["postinst"].mode, 0o755);
    let script = std::str::from_utf8(&control["postinst"].bytes)?;
    assert!(script.starts_with("#!/bin/sh\n"));
    assert!(script.contains("configure)"));
    assert_linux_install_script(script);
    let data = tar_members(&ar[2].1)?;
    assert_eq!(data["usr/lib/lys/lys"].mode, 0o755);
    assert!(data["usr/lib/lys/lys"].bytes == b"runner");
    assert_eq!(data["usr/lib/lys/lys-containment-entry"].mode, 0o755);
    assert!(data["usr/lib/lys/lys-containment-entry"].bytes == b"containment");
    let saved = &data["usr/lib/lys/runner-pending.json"];
    assert_eq!(saved.mode, 0o600);
    let document: serde_json::Value = serde_json::from_slice(&saved.bytes)?;
    assert_eq!(
        document.as_object().ok_or("pending object missing")?.len(),
        3
    );
    assert_eq!(document["server"], "https://lys.example/api");
    assert_eq!(document["machine"], "op-fixture");
    assert!(
        document["code"]
            .as_str()
            .is_some_and(|code| code == pending().code)
    );
    directory.close()?;
    Ok(())
}

fn assert_linux_install_script(script: &str) {
    for expected in [
        "${SUDO_USER:-}",
        "${PKEXEC_UID:-}",
        "getent passwd",
        "loginctl list-users",
        "loginctl show-user",
        "--property=State --value",
        "active|online",
        "\"$count\" -le 1",
        "RunnerInstallUserMissing:",
        "runuser -u \"$user\"",
        "HOME=\"$home\"",
        "XDG_RUNTIME_DIR=\"/run/user/$uid\"",
        "DBUS_SESSION_BUS_ADDRESS=\"unix:path=/run/user/$uid/bus\"",
        "install -d -m 700 -o \"$uid\" -g \"$gid\" \"$private\"",
        "mktemp -d \"$home/.lys-install.XXXXXXXX\"",
        "install -m 600 -o \"$uid\" -g \"$gid\" \"$pending\" \"$join_file\"",
        "runner install --pending \"$join_file\"",
        "rm -rf -- \"$private\"",
        "rm -f -- \"$pending\"",
        "trap cleanup 0",
        "exit 0",
    ] {
        assert!(
            script.contains(expected),
            "missing installer step: {expected}"
        );
    }
    assert!(!script.contains("unused-fixture"));
    assert!(!script.contains("set -x"));
}

#[test]
fn arch_is_gzip_with_pacman_metadata_install_hook_and_private_pending() -> Result<(), Box<dyn Error>>
{
    let (directory, lys, entry) = binaries()?;
    let bytes = arch(&pending(), &lys, &entry, COMMIT)?;
    assert_eq!(&bytes[..2], &[0x1f, 0x8b]);
    let members = tar_members(&bytes)?;
    let info = std::str::from_utf8(&members[".PKGINFO"].bytes)?;
    assert!(info.contains(&format!(
        "pkgver = {}.g01234567-1\n",
        env!("CARGO_PKG_VERSION")
    )));
    for field in [
        "pkgname = lys\n",
        "pkgbase = lys\n",
        "pkgdesc = Lys computer runner\n",
        "arch = x86_64\n",
        "size = ",
        "depend = curl\n",
        "depend = systemd\n",
        "depend = util-linux\n",
        "depend = glibc\n",
    ] {
        assert!(info.contains(field));
    }
    let script = std::str::from_utf8(&members[".INSTALL"].bytes)?;
    assert!(script.contains("post_install() ("));
    assert!(script.contains("post_upgrade()"));
    assert_linux_install_script(script);
    assert_eq!(members["usr/lib/lys/runner-pending.json"].mode, 0o600);
    assert_eq!(members["usr/lib/lys/lys"].mode, 0o755);
    assert_eq!(members["usr/lib/lys/lys-containment-entry"].mode, 0o755);
    directory.close()?;
    Ok(())
}

#[test]
fn pending_and_binary_validation_refuse_without_disclosing_values() -> Result<(), Box<dyn Error>> {
    let (directory, lys, entry) = binaries()?;
    for field in ["server", "machine", "code"] {
        let mut input = pending();
        match field {
            "server" => input.server = "https://secret:secret@lys.example/api".to_owned(),
            "machine" => input.machine = "op-fixture\nforged".to_owned(),
            _ => input.code.clear(),
        }
        let error = deb(&input, &lys, &entry, COMMIT)
            .err()
            .ok_or("invalid input admitted")?;
        let words = error.to_string();
        assert!(words.contains("installer_pending_invalid"));
        assert!(!words.contains("secret"));
        assert!(!words.contains("unused-fixture"));
    }
    std::fs::write(&lys, b"")?;
    let error = deb(&pending(), &lys, &entry, COMMIT)
        .err()
        .ok_or("empty executable admitted")?;
    assert!(error.to_string().contains("installer_binary_missing"));
    std::fs::remove_file(&lys)?;
    let error = arch(&pending(), &lys, &entry, COMMIT)
        .err()
        .ok_or("missing executable admitted")?;
    assert!(error.to_string().contains("installer_binary_unreadable"));
    directory.close()?;
    Ok(())
}

#[test]
fn package_versions_carry_only_a_valid_source_commit() -> Result<(), Box<dyn Error>> {
    assert_eq!(
        version(COMMIT, "+")?,
        format!("{}+g01234567", env!("CARGO_PKG_VERSION"))
    );
    assert_eq!(
        version(COMMIT, ".")?,
        format!("{}.g01234567", env!("CARGO_PKG_VERSION"))
    );
    for commit in ["short", "01234567\nArchitecture: arm64", "xyz123456789"] {
        let error = version(commit, "+")
            .err()
            .ok_or("invalid commit admitted")?;
        assert!(error.to_string().contains("installer_commit_invalid"));
    }
    Ok(())
}

#[test]
fn pending_memory_and_staging_file_are_private() -> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("runner-pending.json");
    let mut pending = pending();
    private_file(&path, &pending_bytes(&pending)?, 0o600)?;
    assert_eq!(
        std::fs::metadata(&path)?.permissions().mode() & 0o777,
        0o600
    );
    let error = private_file(&path, b"replacement", 0o600)
        .err()
        .ok_or("pending file replaced")?;
    assert!(error.to_string().contains("installer_staging_failed"));
    zeroize::Zeroize::zeroize(&mut pending);
    assert!(pending.code.is_empty());
    assert!(pending.server.is_empty());
    assert!(pending.machine.is_empty());
    directory.close()?;
    Ok(())
}

fn hook_fixture() -> Result<tempfile::TempDir, Box<dyn Error>> {
    let directory = tempfile::Builder::new()
        .prefix("lys package fixture ")
        .tempdir()?;
    let root = directory.path();
    let stubs = [
        (
            "getent",
            r#"#!/bin/sh
case "$3" in
    alice) printf 'alice:x:1001:1001::%s/alice:/bin/sh\n' "$FIXTURE_HOME";;
    1002|graphical) printf 'graphical:x:1002:1002::%s/graphical:/bin/sh\n' "$FIXTURE_HOME";;
    *) exit 2;;
esac
"#,
        ),
        (
            "id",
            r#"#!/bin/sh
case "${3:-$2}" in alice) printf '1001\n';; graphical) printf '1002\n';; *) exit 1;; esac
"#,
        ),
        (
            "loginctl",
            r#"#!/bin/sh
case "$1" in
    list-users) printf '%s\n' "$FIXTURE_USERS";;
    show-user) printf 'active\n';;
    *) exit 1;;
esac
"#,
        ),
        ("stat", "#!/bin/sh\nprintf 'alice\\n'\n"),
        (
            "chown",
            "#!/bin/sh\nprintf '%s\\n' \"$@\" >> \"$FIXTURE_OWNERSHIP\"\n",
        ),
        (
            "install",
            r#"#!/bin/sh
set -eu
printf '%s\n' "$@" >> "$FIXTURE_OWNERSHIP"
if [ "$1" = -d ]; then
    for target in "$@"; do :; done
    mkdir -m 700 "$target"
else
    while [ "$#" -gt 2 ]; do shift 2; done
    cp "$1" "$2"
    chmod 600 "$2"
fi
"#,
        ),
    ];
    for (name, script) in stubs {
        private_file(&root.join(name), script.as_bytes(), 0o700)?;
    }
    let runner = r#"#!/bin/sh
set -eu
for argument in "$@"; do
    if [ "$argument" = mktemp ]; then
        while [ "$1" != mktemp ]; do shift; done
        exec "$@"
    fi
done
printf '%s\n' "$@" > "$FIXTURE_CALL"
while [ "$#" -gt 0 ] && [ "$1" != --pending ]; do shift; done
[ "$#" -eq 2 ] || exit 90
join_file=$2
[ "$join_file" != "$FIXTURE_PENDING" ] || exit 91
[ ! -e "$FIXTURE_PENDING" ] || exit 92
[ -f "$join_file" ] || exit 93
private=${join_file%/*}
case "$(ls -ld "$private")" in drwx------*) ;; *) exit 94;; esac
case "$(ls -ld "$join_file")" in -rw-------*) ;; *) exit 95;; esac
if [ "$FIXTURE_CONSUME" = yes ]; then rm "$join_file"; fi
printf '%s\n' "$join_file" > "$FIXTURE_HANDOFF"
exit "$FIXTURE_RESULT"
"#;
    for name in ["runuser", "launchctl"] {
        private_file(&root.join(name), runner.as_bytes(), 0o700)?;
    }
    for path in ["home/alice", "home/graphical", "tmp"] {
        std::fs::create_dir_all(root.join(path))?;
    }
    Ok(directory)
}

fn exercise_hook(macos: bool) -> Result<(), Box<dyn Error>> {
    let directory = hook_fixture()?;
    let root = directory.path();
    let script = if macos {
        MAC_POSTINSTALL
            .replace(
                "PATH=/usr/bin:/bin:/usr/sbin:/sbin",
                "PATH=\"$FIXTURE_BIN:/usr/bin:/bin:/usr/sbin:/sbin\"",
            )
            .replace(
                "pending='/Library/Application Support/Lys/runner-pending.json'",
                "pending=\"$FIXTURE_PENDING\"",
            )
            .replace("/usr/bin/stat", "stat")
            .replace("/usr/bin/id", "id")
            .replace("/usr/bin/mktemp", "mktemp")
            .replace("/usr/sbin/chown", "chown")
            .replace("/bin/chmod", "chmod")
            .replace("/usr/bin/install", "install")
            .replace("/bin/launchctl", "launchctl")
    } else {
        LINUX_INSTALL
            .replace(
                "PATH=/usr/sbin:/usr/bin:/sbin:/bin",
                "PATH=\"$FIXTURE_BIN:/usr/sbin:/usr/bin:/sbin:/bin\"",
            )
            .replace(
                "pending=/usr/lib/lys/runner-pending.json",
                "pending=\"$FIXTURE_PENDING\"",
            )
            .replace("/run/user/$uid", "$FIXTURE_RUNTIME")
    };
    let cases = if macos {
        vec![
            ("", "", "alice", 0, true, "yes"),
            ("", "", "alice", 9, true, "no"),
            ("", "", "alice", 9, true, "yes"),
        ]
    } else {
        vec![
            ("alice", "1002", "alice", 0, true, "yes"),
            ("", "1002", "graphical", 0, true, "yes"),
            ("", "", "alice", 0, true, "yes"),
            ("alice", "", "alice", 9, true, "no"),
            ("alice", "", "alice", 9, false, "yes"),
            ("", "", "", 0, false, "no"),
        ]
    };
    for (index, (sudo, graphical, expected, result, runtime, consume)) in
        cases.into_iter().enumerate()
    {
        let pending = root.join("runner-pending.json");
        let call = root.join("call");
        let handoff = root.join("handoff");
        let ownership = root.join("ownership");
        let runtime_dir = root.join(format!("runtime-{index}"));
        if runtime {
            std::fs::create_dir(&runtime_dir)?;
        }
        private_file(&pending, b"{}", 0o600)?;
        let output = Command::new("/bin/sh")
            .args(["-c", &script])
            .env("SUDO_USER", sudo)
            .env("PKEXEC_UID", graphical)
            .env("FIXTURE_BIN", root)
            .env("FIXTURE_HOME", root.join("home"))
            .env("TMPDIR", root.join("tmp"))
            .env("FIXTURE_RUNTIME", &runtime_dir)
            .env(
                "FIXTURE_USERS",
                if expected.is_empty() {
                    ""
                } else {
                    "1001 alice no"
                },
            )
            .env("FIXTURE_PENDING", &pending)
            .env("FIXTURE_CALL", &call)
            .env("FIXTURE_HANDOFF", &handoff)
            .env("FIXTURE_OWNERSHIP", &ownership)
            .env("FIXTURE_RESULT", result.to_string())
            .env("FIXTURE_CONSUME", consume)
            .output()?;
        assert_eq!(output.status.code(), Some(0));
        assert!(!pending.try_exists()?);
        let words = String::from_utf8(output.stderr)?;
        if expected.is_empty() {
            assert!(words.contains("RunnerInstallUserMissing:"));
            assert!(words.contains("fresh installer"));
            continue;
        }
        let join_file = std::fs::read_to_string(&handoff)?;
        let join_file = Path::new(join_file.trim());
        assert!(
            !join_file
                .parent()
                .ok_or("private parent missing")?
                .try_exists()?
        );
        if !macos && !runtime {
            assert!(join_file.starts_with(root.join("home")));
        }
        let args = std::fs::read_to_string(&call)?;
        let uid = if expected == "alice" { 1001 } else { 1002 };
        if macos {
            assert!(args.starts_with("asuser\n1001\n/usr/bin/sudo\n-H\n-u\nalice\n--\n"));
        } else {
            assert!(args.starts_with(&format!("-u\n{expected}\n--\nenv\n")));
            assert!(args.contains(&format!(
                "HOME={}\n",
                root.join("home").join(expected).display()
            )));
            assert!(args.contains(&format!("XDG_RUNTIME_DIR={}\n", runtime_dir.display())));
            assert!(args.contains(&format!(
                "DBUS_SESSION_BUS_ADDRESS=unix:path={}/bus\n",
                runtime_dir.display()
            )));
        }
        let owner = std::fs::read_to_string(&ownership)?;
        assert!(owner.contains(&format!("-o\n{uid}\n-g\n{uid}\n")));
        if result != 0 {
            assert!(words.contains("RunnerInstallFailed:"));
            assert!(words.contains("exit 9"));
            assert!(words.contains("runner install"));
            assert!(words.contains("fresh installer"));
            assert!(!words.contains("--pending"));
        }
        std::fs::remove_file(call)?;
        std::fs::remove_file(handoff)?;
        std::fs::remove_file(ownership)?;
    }
    directory.close()?;
    Ok(())
}

#[test]
fn linux_hook_hands_off_a_deletable_private_copy_and_keeps_the_package_installed()
-> Result<(), Box<dyn Error>> {
    exercise_hook(false)
}

#[test]
fn macos_hook_hands_off_a_deletable_private_copy_and_keeps_the_package_installed()
-> Result<(), Box<dyn Error>> {
    exercise_hook(true)
}

fn args(command: &Command) -> Vec<&std::ffi::OsStr> {
    command.get_args().collect()
}

#[test]
fn macos_commands_preserve_paths_and_select_both_architectures() {
    let root = Path::new("/private/tmp/package fixture");
    let lipo = lipo_command(
        &root.join("arm64/lys"),
        &root.join("x86_64/lys"),
        &root.join("payload/lys"),
    );
    assert_eq!(lipo.get_program(), "/usr/bin/lipo");
    assert_eq!(
        args(&lipo),
        [
            "-create",
            "-arch",
            "arm64",
            "/private/tmp/package fixture/arm64/lys",
            "-arch",
            "x86_64",
            "/private/tmp/package fixture/x86_64/lys",
            "-output",
            "/private/tmp/package fixture/payload/lys"
        ]
    );
    let version = format!("{}+g01234567", env!("CARGO_PKG_VERSION"));
    let pkg = pkgbuild_command(root, &version);
    assert_eq!(pkg.get_program(), "/usr/bin/pkgbuild");
    assert_eq!(
        args(&pkg),
        [
            "--root",
            "/private/tmp/package fixture/payload",
            "--scripts",
            "/private/tmp/package fixture/scripts",
            "--identifier",
            "au.com.ablative.lys-runner",
            "--version",
            &version,
            "--install-location",
            "/",
            "--ownership",
            "recommended",
            "/private/tmp/package fixture/component.pkg"
        ]
    );
    let product = productbuild_command(root);
    assert_eq!(product.get_program(), "/usr/bin/productbuild");
    assert_eq!(
        args(&product),
        [
            "--package",
            "/private/tmp/package fixture/component.pkg",
            "/private/tmp/package fixture/lys.pkg"
        ]
    );
    for step in [
        "stat -f %Su /dev/console",
        "launchctl asuser \"$uid\"",
        "/usr/bin/sudo -H -u \"$user\"",
        "runner install --pending \"$join_file\"",
        "RunnerInstallUserMissing:",
        "loginwindow",
        "chown \"$uid:$gid\" \"$private\"",
        "chmod 700 \"$private\"",
        "rm -rf -- \"$private\"",
        "install -m 600 -o \"$uid\" -g \"$gid\" \"$pending\" \"$join_file\"",
    ] {
        assert!(
            MAC_POSTINSTALL.contains(step),
            "missing installer step: {step}"
        );
    }
}

#[cfg(target_os = "macos")]
#[test]
fn pkgbuild_produces_a_flat_package_without_running_install_hooks() -> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let root = directory.path();
    std::fs::create_dir_all(root.join("payload/Library/Application Support/Lys"))?;
    std::fs::create_dir(root.join("scripts"))?;
    private_file(
        &root.join("payload/Library/Application Support/Lys/runner-pending.json"),
        &pending_bytes(&pending())?,
        0o600,
    )?;
    private_file(
        &root.join("scripts/postinstall"),
        MAC_POSTINSTALL.as_bytes(),
        0o755,
    )?;
    run_tool(pkgbuild_command(root, &version(COMMIT, "+")?), "pkgbuild")?;
    run_tool(productbuild_command(root), "productbuild")?;
    let bytes = std::fs::read(root.join("lys.pkg"))?;
    assert!(bytes.starts_with(b"xar!"));
    directory.close()?;
    Ok(())
}
