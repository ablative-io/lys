//! The container engine Lys's services run in: whether it is here and
//! answering, what the page tells a person when it is not, and the wait
//! for it to answer.
//!
//! The engine is asked at its known socket paths, the places the engines
//! this install supports listen on a Mac, with the engine's own `/_ping`
//! over the socket. When none answers, the page shows [`Guidance`] in plain
//! words: what the engine is for, the official download for this Mac, and
//! that the install continues by itself.
//!
//! The wait for the engine to answer is [`crate::engine_wait`]: on the
//! kernel's notice of a socket or state folder changing and on the engine's
//! own answer to a held request, never a clock.
//!
//! Invariants: nothing here reads a clock or sleeps; and no word of the
//! guidance names the issuer inside Lys.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use lys_install::install::engine_path;
use serde::Serialize;

/// The official page for Docker Desktop on a Mac.
pub const DOWNLOAD_PAGE: &str = "https://docs.docker.com/desktop/setup/install/mac-install/";

/// Where the engines this install supports listen, for the home `home`:
/// Docker Desktop's own socket, then Colima's, `OrbStack`'s and Rancher
/// Desktop's, then the system-wide link Docker Desktop makes.
pub fn socket_paths(home: Option<&Path>) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(home) = home {
        paths.push(home.join(".docker").join("run").join("docker.sock"));
        paths.push(home.join(".colima").join("default").join("docker.sock"));
        paths.push(home.join(".orbstack").join("run").join("docker.sock"));
        paths.push(home.join(".rd").join("docker.sock"));
    }
    paths.push(PathBuf::from("/var/run/docker.sock"));
    paths
}

/// Whether the engine behind `socket` answers its `/_ping` with `200`.
pub fn answers(socket: &Path) -> bool {
    ping(socket).is_ok_and(|answer| {
        answer.starts_with("HTTP/1.1 200") || answer.starts_with("HTTP/1.0 200")
    })
}

#[cfg(unix)]
fn ping(socket: &Path) -> std::io::Result<String> {
    let mut stream = std::os::unix::net::UnixStream::connect(socket)?;
    stream.write_all(b"GET /_ping HTTP/1.0\r\nHost: docker\r\n\r\n")?;
    let mut answer = String::new();
    stream.read_to_string(&mut answer)?;
    Ok(answer)
}

#[cfg(not(unix))]
fn ping(socket: &Path) -> std::io::Result<String> {
    Err(std::io::Error::other(format!(
        "{} is a Unix socket and this is not a Unix host",
        socket.display()
    )))
}

/// The folders Docker Desktop's engine writes as it starts, under the home
/// `home`: its data folder, where its sockets to the engine are made as
/// the engine comes up.
pub fn state_folders(home: Option<&Path>) -> Vec<PathBuf> {
    home.map(|home| {
        let data = home
            .join("Library")
            .join("Containers")
            .join("com.docker.docker")
            .join("Data");
        vec![data.clone(), data.join("vms").join("0")]
    })
    .unwrap_or_default()
}

/// The first of `sockets` that answers.
pub fn answering(sockets: &[PathBuf]) -> Option<PathBuf> {
    sockets.iter().find(|socket| answers(socket)).cloned()
}

/// Whether a container engine's command-line tools are on this Mac.
pub fn installed(dirs: &[PathBuf]) -> bool {
    engine_path::find(engine_path::DOCKER, dirs).is_file()
}

/// Which download this Mac needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Chip {
    /// An Apple processor.
    Apple,
    /// An Intel processor.
    Intel,
}

impl Chip {
    /// The chip of the Mac running `arch` (`std::env::consts::ARCH`).
    pub fn of(arch: &str) -> Self {
        if arch == "aarch64" {
            Chip::Apple
        } else {
            Chip::Intel
        }
    }

    /// The official download of Docker Desktop for this chip.
    pub fn download(self) -> &'static str {
        match self {
            Chip::Apple => "https://desktop.docker.com/mac/main/arm64/Docker.dmg",
            Chip::Intel => "https://desktop.docker.com/mac/main/amd64/Docker.dmg",
        }
    }

    /// The chip's name as a person reads it.
    pub fn words(self) -> &'static str {
        match self {
            Chip::Apple => "a Mac with an Apple chip",
            Chip::Intel => "a Mac with an Intel chip",
        }
    }
}

/// What the page tells a person whose engine is missing or stopped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Guidance {
    /// `missing` or `stopped`.
    pub state: &'static str,
    /// The heading.
    pub title: &'static str,
    /// What the engine is for.
    pub why: &'static str,
    /// What to do.
    pub what: String,
    /// The official download for this Mac, when the engine is missing.
    pub download: Option<&'static str>,
    /// The page the download is described on.
    pub page: &'static str,
    /// That the install continues by itself.
    pub then: &'static str,
}

const WHY: &str = "Lys keeps its sign-in, its directory and its permissions in a few small \
                   services that run inside a container engine on this Mac. Docker Desktop is \
                   the one Lys is tested with.";

const THEN: &str = "Leave this page open. Lys continues by itself as soon as the engine \
                    answers; there is nothing to press here.";

impl Guidance {
    /// The guidance for an engine that is not on this Mac.
    pub fn missing(chip: Chip) -> Self {
        Self {
            state: "missing",
            title: "Lys needs a container engine",
            why: WHY,
            what: format!(
                "Download Docker Desktop for {}, open the download, drag Docker to Applications \
                 and open it once. Leave it running.",
                chip.words()
            ),
            download: Some(chip.download()),
            page: DOWNLOAD_PAGE,
            then: THEN,
        }
    }

    /// The guidance for an engine that is here but not answering.
    pub fn stopped() -> Self {
        Self {
            state: "stopped",
            title: "The container engine is not running",
            why: WHY,
            what: "Open Docker Desktop from your Applications folder and leave it running."
                .to_string(),
            download: None,
            page: DOWNLOAD_PAGE,
            then: THEN,
        }
    }

    /// The guidance for this Mac: missing when the engine's tools are not in
    /// `dirs`, stopped otherwise.
    pub fn for_mac(dirs: &[PathBuf]) -> Self {
        if installed(dirs) {
            Self::stopped()
        } else {
            Self::missing(Chip::of(std::env::consts::ARCH))
        }
    }
}

/// The folder watched for `socket`: its own folder when that exists, its
/// nearest existing ancestor otherwise.
pub fn watched_folder(socket: &Path) -> Option<PathBuf> {
    socket
        .ancestors()
        .skip(1)
        .find(|dir| dir.is_dir())
        .map(Path::to_path_buf)
}

/// The folders watched for `sockets`, each once.
pub fn watched_folders(sockets: &[PathBuf]) -> Vec<PathBuf> {
    let mut folders: Vec<PathBuf> = Vec::new();
    for folder in sockets.iter().filter_map(|socket| watched_folder(socket)) {
        if !folders.contains(&folder) {
            folders.push(folder);
        }
    }
    folders
}

#[cfg(test)]
#[path = "engine_tests.rs"]
mod tests;
