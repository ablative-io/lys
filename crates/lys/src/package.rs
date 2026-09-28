//! `lys package app`: Lys.app and Lys.dmg for macOS, one universal build
//! for Apple and Intel processors, signed, notarised and stapled.
//!
//! The command is given two folders of Lys's binaries, one built for each
//! processor, and the compiled screens package. It reads the build stamp
//! each binary's `--version` prints from the binary itself (so a binary
//! built for the other processor is read without running it) and refuses,
//! naming the binary, unless every one names the same commit. It joins
//! each pair into one universal binary with `lipo` and refuses unless each
//! holds both processors; assembles the bundle (`Info.plist`, icon, the
//! binaries beside `lys-app`, the bundle's executable, and the screens,
//! verified and placed as the install places them); signs every binary and
//! the bundle with the hardened runtime; submits the app for notarisation
//! and staples the ticket; and makes the disk image, the app beside a link
//! to Applications.
//!
//! Without a Developer ID it builds a locally signed app named
//! `Lys (development)` when `--development` asks, and is refused
//! `signing_identity_missing` otherwise, before anything is written.
//!
//! Invariants: the release name is given only to an app signed with the
//! Developer ID, verified, notarised and stapled; everything is assembled
//! in a staging folder and moved to its name only when it is finished, and
//! the staging folder is removed on any refusal; every tool is named by
//! its absolute path, and every refusal names what it refused.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use clap::Subcommand;
use lys_install::install::surface;

use crate::commands::output::Emitter;

/// The binaries the app carries, the bundle's executable first.
pub const BINARIES: [&str; 5] = [
    "lys-app",
    "lys",
    "lys-identity-server",
    "lys-secrets",
    "lys-home",
];

/// The release name, given only to a signed, notarised app.
pub const RELEASE_NAME: &str = "Lys";

/// The name of an app signed only on this machine.
pub const DEVELOPMENT_NAME: &str = "Lys (development)";

/// The processors the universal build holds, as `lipo` names them.
pub const ARCHS: [&str; 2] = ["arm64", "x86_64"];

/// The words a build with no commit carries.
const NO_COMMIT: &str = "not built from a git commit";

/// The bundle's property list, with `{{name}}`, `{{bundle_id}}`,
/// `{{version}}` and `{{build}}` to fill.
pub const INFO_PLIST: &str = include_str!("../../../packaging/macos/Info.plist");

/// The app's icon.
const ICON: &[u8] = include_bytes!("../../../packaging/macos/Lys.icns");

/// `lys package` subcommands.
#[derive(Debug, Subcommand)]
pub enum PackageCommand {
    /// Build Lys.app and Lys.dmg: a universal app holding every Lys binary
    /// and the screens, signed with the Developer ID and the hardened
    /// runtime, notarised and stapled.
    App {
        /// The folder the app and the disk image are written to.
        #[arg(long)]
        out: PathBuf,
        /// A folder holding every Lys binary built for Apple processors.
        #[arg(long)]
        arm64: PathBuf,
        /// A folder holding every Lys binary built for Intel processors.
        #[arg(long = "x86-64")]
        x86_64: PathBuf,
        /// The compiled screens package.
        #[arg(long)]
        surface: PathBuf,
        /// The Developer ID Application identity the keychain holds.
        #[arg(long, env = "LYS_DEVELOPER_ID")]
        developer_id: Option<String>,
        /// The keychain profile the notarisation credentials are kept under.
        #[arg(long, env = "LYS_NOTARY_PROFILE")]
        notary_profile: Option<String>,
        /// Build a locally signed app named "Lys (development)" instead.
        #[arg(long)]
        development: bool,
    },
}

/// A refusal of `lys package`, by name.
#[derive(Debug, thiserror::Error)]
#[error("{name}: {detail}")]
pub struct PackageError {
    /// The refusal's stable name.
    pub name: &'static str,
    /// What was refused.
    pub detail: String,
}

fn refuse(name: &'static str, detail: impl Into<String>) -> PackageError {
    PackageError {
        name,
        detail: detail.into(),
    }
}

/// `value` when it names something.
fn named(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

/// How the app is signed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Signing {
    /// With the Developer ID, notarised under the keychain profile.
    Release {
        /// The Developer ID Application identity.
        identity: String,
        /// The notarisation credentials' keychain profile.
        profile: String,
    },
    /// Locally, under the development name.
    Development,
}

impl Signing {
    /// The signing the flags ask for, refused by name when a release has
    /// no Developer ID or no notarisation profile.
    pub fn from_flags(
        developer_id: Option<&str>,
        notary_profile: Option<&str>,
        development: bool,
    ) -> Result<Self, PackageError> {
        if development {
            return Ok(Signing::Development);
        }
        let identity = named(developer_id).ok_or_else(|| {
            refuse(
                "signing_identity_missing",
                "no Developer ID is named: pass --developer-id or set LYS_DEVELOPER_ID, or \
                 build a locally signed app with --development",
            )
        })?;
        let profile = named(notary_profile).ok_or_else(|| {
            refuse(
                "notary_profile_missing",
                "no notarisation profile is named: pass --notary-profile or set \
                 LYS_NOTARY_PROFILE",
            )
        })?;
        Ok(Signing::Release {
            identity: identity.to_string(),
            profile: profile.to_string(),
        })
    }

    /// The app's name.
    pub fn name(&self) -> &'static str {
        match self {
            Signing::Release { .. } => RELEASE_NAME,
            Signing::Development => DEVELOPMENT_NAME,
        }
    }

    /// The bundle identifier.
    pub fn bundle_id(&self) -> &'static str {
        match self {
            Signing::Release { .. } => "au.com.ablative.lys",
            Signing::Development => "au.com.ablative.lys.development",
        }
    }

    /// The `codesign` arguments before the path.
    fn codesign_args(&self) -> Vec<String> {
        let mut args = vec!["--force".to_string()];
        match self {
            Signing::Release { identity, .. } => args.extend([
                "--options".to_string(),
                "runtime".to_string(),
                "--timestamp".to_string(),
                "--sign".to_string(),
                identity.clone(),
            ]),
            Signing::Development => args.extend(["--sign".to_string(), "-".to_string()]),
        }
        args
    }
}

/// The build stamp `NAME VERSION (BUILD)` holds, read from a binary's
/// bytes: the one text after `VERSION (` that is a 40-character commit,
/// perhaps with `; dirty`, or the words of a build with no commit.
pub fn stamp_in(bytes: &[u8], version: &str) -> Option<String> {
    let marker = format!("{version} (");
    let marker = marker.as_bytes();
    let mut found: Vec<String> = Vec::new();
    let mut at = 0;
    while let Some(offset) = find(&bytes[at..], marker) {
        let start = at + offset + marker.len();
        let rest = &bytes[start..bytes.len().min(start + 64)];
        if let Some(end) = rest.iter().position(|byte| *byte == b')') {
            let inside = String::from_utf8_lossy(&rest[..end]).into_owned();
            if is_stamp(&inside) && !found.contains(&inside) {
                found.push(inside);
            }
        }
        at = start;
    }
    match found.as_slice() {
        [one] => Some(one.clone()),
        _ => None,
    }
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn is_stamp(text: &str) -> bool {
    let commit = text.strip_suffix("; dirty").unwrap_or(text);
    text == NO_COMMIT || (commit.len() == 40 && commit.bytes().all(|b| b.is_ascii_hexdigit()))
}

/// The one build every reading names, or a refusal naming the first binary
/// whose build differs from the first one read.
pub fn same_build(readings: &[(String, String)]) -> Result<String, PackageError> {
    let Some((first, build)) = readings.first() else {
        return Err(refuse("binary_missing", "no binary was read"));
    };
    match readings.iter().find(|(_, other)| other != build) {
        None => Ok(build.clone()),
        Some((binary, other)) => Err(refuse(
            "build_mismatch",
            format!("{binary} is built from {other}, but {first} is built from {build}"),
        )),
    }
}

/// The bundle's property list for `signing` at `version` and `build`.
pub fn render_info(signing: &Signing, version: &str, build: &str) -> String {
    INFO_PLIST
        .replace("{{name}}", signing.name())
        .replace("{{bundle_id}}", signing.bundle_id())
        .replace("{{version}}", version)
        .replace("{{build}}", build)
}

/// Runs `program` with `args`, answering its output, refused as `name`
/// when it cannot start or exits unsuccessfully.
fn tool(program: &str, args: &[&str], name: &'static str) -> Result<String, PackageError> {
    let output = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| refuse(name, format!("{program} could not start: {error}")))?;
    let said = String::from_utf8_lossy(&output.stdout).into_owned();
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(refuse(
            name,
            format!(
                "{program} {} exited {}: {} {}",
                args.join(" "),
                output.status,
                said.trim(),
                error.trim()
            ),
        ));
    }
    Ok(said)
}

fn text(path: &Path) -> String {
    path.display().to_string()
}

fn io(name: &'static str, path: &Path, error: &std::io::Error) -> PackageError {
    refuse(name, format!("{}: {error}", path.display()))
}

/// Refuses `signing_identity_missing` unless the keychain holds `identity`
/// as a valid code-signing identity.
fn require_identity(identity: &str) -> Result<(), PackageError> {
    let found = tool(
        "/usr/bin/security",
        &["find-identity", "-v", "-p", "codesigning"],
        "signing_identity_missing",
    )?;
    if found.contains(&format!("\"{identity}\"")) {
        return Ok(());
    }
    Err(refuse(
        "signing_identity_missing",
        format!("the keychain holds no valid code-signing identity named {identity}"),
    ))
}

/// Reads every binary's build in both folders.
fn read_builds(folders: &[(&str, &Path)]) -> Result<String, PackageError> {
    let mut readings = Vec::new();
    for (arch, folder) in folders {
        for name in BINARIES {
            let path = folder.join(name);
            let bytes =
                std::fs::read(&path).map_err(|error| io("binary_missing", &path, &error))?;
            let build = stamp_in(&bytes, env!("CARGO_PKG_VERSION")).ok_or_else(|| {
                let detail = format!("{arch}/{name} carries no one build stamp");
                refuse("build_unreadable", detail)
            })?;
            readings.push((format!("{arch}/{name}"), build));
        }
    }
    same_build(&readings)
}

/// Joins each binary into one universal binary in `macos`, refused unless
/// it holds both processors.
fn universal(folders: &[(&str, &Path)], macos: &Path) -> Result<(), PackageError> {
    for name in BINARIES {
        let out = macos.join(name);
        let mut args = vec!["-create".to_string(), "-output".to_string(), text(&out)];
        args.extend(folders.iter().map(|(_, folder)| text(&folder.join(name))));
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        tool("/usr/bin/lipo", &args, "universal_incomplete")?;
        let archs = tool(
            "/usr/bin/lipo",
            &["-archs", &text(&out)],
            "universal_incomplete",
        )?;
        let held: Vec<&str> = archs.split_whitespace().collect();
        if let Some(missing) = ARCHS.iter().find(|arch| !held.contains(arch)) {
            let detail = format!("{name} holds {held:?}, not {missing}");
            return Err(refuse("universal_incomplete", detail));
        }
    }
    Ok(())
}

/// Signs every binary the bundle's executable sits beside, then the
/// bundle, and verifies the bundle as Gatekeeper would.
fn sign(signing: &Signing, app: &Path) -> Result<(), PackageError> {
    let args = signing.codesign_args();
    let macos = app.join("Contents").join("MacOS");
    let mut paths: Vec<PathBuf> = BINARIES
        .iter()
        .skip(1)
        .map(|name| macos.join(name))
        .collect();
    paths.push(app.to_path_buf());
    for path in paths {
        let mut all: Vec<&str> = args.iter().map(String::as_str).collect();
        let path = text(&path);
        all.push(&path);
        tool("/usr/bin/codesign", &all, "signing_failed")?;
    }
    let verify = ["--verify", "--deep", "--strict", &text(app)];
    tool("/usr/bin/codesign", &verify, "signature_invalid").map(drop)
}

/// Submits `submit` (a zip of the app, or the disk image itself) for
/// notarisation under `profile`, then staples the ticket to `staple`.
fn notarise_as(submit: &Path, staple: &Path, profile: &str) -> Result<(), PackageError> {
    let said = tool(
        "/usr/bin/xcrun",
        &[
            "notarytool",
            "submit",
            &text(submit),
            "--keychain-profile",
            profile,
            "--wait",
        ],
        "notarisation_refused",
    )?;
    if !said.contains("status: Accepted") {
        return Err(refuse("notarisation_refused", said.trim().to_string()));
    }
    tool(
        "/usr/bin/xcrun",
        &["stapler", "staple", &text(staple)],
        "staple_failed",
    )
    .map(drop)
}

/// Notarises `app`, staples the ticket and asks Gatekeeper to assess it.
fn notarise(app: &Path, profile: &str, staging: &Path) -> Result<(), PackageError> {
    let zip = staging.join("notarise.zip");
    let pack = ["-c", "-k", "--keepParent", &text(app), &text(&zip)];
    tool("/usr/bin/ditto", &pack, "notarisation_refused")?;
    notarise_as(&zip, app, profile)?;
    let assess = ["--assess", "--type", "execute", &text(app)];
    tool("/usr/sbin/spctl", &assess, "gatekeeper_refused").map(drop)
}

/// Notarises the disk image `image`, staples its own ticket and asks
/// Gatekeeper to assess it as a download is assessed, so the image passes
/// on its own before the app inside it is ever opened.
fn notarise_image(image: &Path, profile: &str) -> Result<(), PackageError> {
    notarise_as(image, image, profile)?;
    let assess = [
        "--assess",
        "--type",
        "open",
        "--context",
        "context:primary-signature",
        &text(image),
    ];
    tool("/usr/sbin/spctl", &assess, "gatekeeper_refused").map(drop)
}

/// Builds the app in `staging`, answering its path there.
fn assemble(
    signing: &Signing,
    folders: &[(&str, &Path)],
    screens: &Path,
    build: &str,
    staging: &Path,
) -> Result<PathBuf, PackageError> {
    let app = staging.join(format!("{}.app", signing.name()));
    let contents = app.join("Contents");
    let macos = contents.join("MacOS");
    let resources = contents.join("Resources");
    for dir in [&macos, &resources] {
        std::fs::create_dir_all(dir).map_err(|error| io("assemble_failed", dir, &error))?;
    }
    universal(folders, &macos)?;
    let info = contents.join("Info.plist");
    std::fs::write(
        &info,
        render_info(signing, env!("CARGO_PKG_VERSION"), build),
    )
    .map_err(|error| io("assemble_failed", &info, &error))?;
    let icon = resources.join("Lys.icns");
    std::fs::write(&icon, ICON).map_err(|error| io("assemble_failed", &icon, &error))?;
    surface::place(screens, &resources.join("surface"))
        .map_err(|error| refuse("screens_invalid", error.to_string()))?;
    sign(signing, &app)?;
    if let Signing::Release { profile, .. } = signing {
        notarise(&app, profile, staging)?;
    }
    Ok(app)
}

/// Makes the disk image holding `app` beside a link to Applications in
/// `staging`, answering its path there; a release image is signed,
/// notarised and stapled.
fn disk_image(signing: &Signing, app: &Path, staging: &Path) -> Result<PathBuf, PackageError> {
    let folder = staging.join("image");
    std::fs::create_dir_all(&folder).map_err(|error| io("disk_image_failed", &folder, &error))?;
    let name = format!("{}.app", signing.name());
    tool(
        "/usr/bin/ditto",
        &[&text(app), &text(&folder.join(&name))],
        "disk_image_failed",
    )?;
    let link = folder.join("Applications");
    std::os::unix::fs::symlink("/Applications", &link)
        .map_err(|error| io("disk_image_failed", &link, &error))?;
    let image = staging.join("image.dmg");
    let create = [
        "create",
        "-volname",
        signing.name(),
        "-srcfolder",
        &text(&folder),
        "-format",
        "UDZO",
        &text(&image),
    ];
    tool("/usr/bin/hdiutil", &create, "disk_image_failed")?;
    if let Signing::Release { identity, profile } = signing {
        let sign = ["--force", "--timestamp", "--sign", identity, &text(&image)];
        tool("/usr/bin/codesign", &sign, "signing_failed")?;
        notarise_image(&image, profile)?;
    }
    Ok(image)
}

/// Moves `from` to `to`, replacing an earlier `to`, file or folder.
fn replace(from: &Path, to: &Path) -> Result<(), PackageError> {
    if to.is_dir() {
        std::fs::remove_dir_all(to).map_err(|error| io("assemble_failed", to, &error))?;
    }
    std::fs::rename(from, to).map_err(|error| io("assemble_failed", to, &error))
}

/// The options of `lys package app`.
#[derive(Debug)]
pub struct Options<'a> {
    /// Where the app and the disk image go.
    pub out: &'a Path,
    /// The binaries built for Apple processors.
    pub arm64: &'a Path,
    /// The binaries built for Intel processors.
    pub x86_64: &'a Path,
    /// The screens package.
    pub surface: &'a Path,
    /// How the app is signed.
    pub signing: Signing,
}

/// Runs `lys package app`.
pub fn run(options: &Options<'_>, json: bool) -> Result<(), PackageError> {
    let signing = &options.signing;
    if let Signing::Release { identity, .. } = signing {
        require_identity(identity)?;
    }
    let folders = [("arm64", options.arm64), ("x86_64", options.x86_64)];
    let build = read_builds(&folders)?;
    let (manifest, _) = surface::verify(options.surface)
        .map_err(|error| refuse("screens_invalid", error.to_string()))?;
    if matches!(signing, Signing::Release { .. }) && manifest.commit != build {
        return Err(refuse(
            "build_mismatch",
            format!(
                "the screens are built from {}, the binaries from {build}",
                manifest.commit
            ),
        ));
    }
    std::fs::create_dir_all(options.out)
        .map_err(|error| io("assemble_failed", options.out, &error))?;
    let staging = options.out.join(format!(".{}.building", signing.name()));
    if staging.exists() {
        std::fs::remove_dir_all(&staging)
            .map_err(|error| io("assemble_failed", &staging, &error))?;
    }
    let app = options.out.join(format!("{}.app", signing.name()));
    let dmg = options.out.join(format!("{}.dmg", signing.name()));
    let outcome = assemble(signing, &folders, options.surface, &build, &staging)
        .and_then(|built| disk_image(signing, &built, &staging).map(|image| (built, image)))
        .and_then(|(built, image)| replace(&built, &app).map(|()| image))
        .and_then(|image| replace(&image, &dmg));
    let cleared = match std::fs::remove_dir_all(&staging) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
            Err(io("assemble_failed", &staging, &error))
        }
        _ => Ok(()),
    };
    match (outcome, cleared) {
        (Err(mut refused), Err(also)) => {
            refused.detail = format!("{}; the staging folder stays: {also}", refused.detail);
            return Err(refused);
        }
        (Err(refused), Ok(())) | (Ok(()), Err(refused)) => return Err(refused),
        (Ok(()), Ok(())) => {}
    }
    let mut emitter = Emitter::new(json);
    emitter.field("app", "app", text(&app));
    emitter.field("disk image", "disk_image", text(&dmg));
    emitter.field("build", "build", build);
    emitter.field("screens", "screens", manifest.commit);
    let signed = match signing {
        Signing::Release { identity, .. } => format!("{identity}, notarised and stapled"),
        Signing::Development => "locally, for development only".to_string(),
    };
    emitter.field("signed", "signed", signed);
    emitter.finish();
    Ok(())
}

#[cfg(test)]
#[path = "package_tests.rs"]
mod tests;
