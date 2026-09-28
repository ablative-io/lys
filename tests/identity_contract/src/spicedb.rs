//! A disposable `SpiceDB`: the release deploy/identity/versions.json pins,
//! started in its in-memory testing mode on a free loopback port, and killed
//! when its holder drops it. Every test that needs a permission engine,
//! including every service the harness starts, gets one of these and no
//! other: no real credentials, no production `SpiceDB` and no shared store.
//!
//! The binary is the release archive for this platform, fetched once into
//! the workspace's `target/tmp` and refused unless its SHA-256 is the one
//! pinned here for that release; `LYS_SPICEDB_BINARY` names a binary to use
//! instead, refused unless it reports the pinned version. One fetch runs at a
//! time in a process. Readiness is read from the server's own log line, never
//! from a clock: a server that stops before serving is reported with
//! everything it said. In its testing mode `SpiceDB` keeps a store of its own
//! for each preshared key, so a key no other caller uses is a store no other
//! caller reads.

use std::error::Error;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};

use lys_identity_server::spicedb::ClientConfig;
use sha2::{Digest, Sha256};

/// The release archives' SHA-256, from each release's checksums.txt.
const ARCHIVES: [(&str, &str, &str); 4] = [
    (
        "v1.56.2",
        "darwin_arm64",
        "1421ff9226202862d423ad18279cc21cc7621420f5984e9f2cf87d1702e8879b",
    ),
    (
        "v1.56.2",
        "darwin_amd64",
        "45c92177f2c1413bae0c46ebb4c9f24e95de43d7202f5e9b40c709467d45d831",
    ),
    (
        "v1.56.2",
        "linux_amd64",
        "490dd2f91b5ef3a7afab600949f605a3db4d72eaa34e8b4172b38f20d1a3e477",
    ),
    (
        "v1.56.2",
        "linux_arm64",
        "095c39c58a0e4338db6b7fa785ce8e2431a90759df4f7cf04d7c0fb42f3f7281",
    ),
];

/// The line `SpiceDB` logs once its gRPC listener serves.
const SERVING: &str = "grpc server started serving";

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Held while the binary is looked for and fetched, so one fetch runs at a
/// time in a process.
static FETCH: Mutex<()> = Mutex::new(());

/// A name no other download or key of this process uses.
pub fn unique(prefix: &str) -> String {
    format!(
        "{prefix}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst)
    )
}

/// The `SpiceDB` release deploy/identity/versions.json pins.
fn pinned_version() -> Result<String, Box<dyn Error>> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../deploy/identity/versions.json");
    let versions: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path)?)?;
    Ok(versions["spicedb"]["version"]
        .as_str()
        .ok_or("deploy/identity/versions.json names no SpiceDB version")?
        .to_owned())
}

fn platform() -> Result<&'static str, Box<dyn Error>> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => Ok("darwin_arm64"),
        ("macos", "x86_64") => Ok("darwin_amd64"),
        ("linux", "x86_64") => Ok("linux_amd64"),
        ("linux", "aarch64") => Ok("linux_arm64"),
        (os, arch) => Err(format!("no SpiceDB release archive is pinned for {os} {arch}").into()),
    }
}

fn run(command: &mut Command) -> Result<(), Box<dyn Error>> {
    let status = command.status()?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{command:?} exited {status}").into())
    }
}

/// The pinned release's binary, fetched and verified when it is not held.
fn binary() -> Result<PathBuf, Box<dyn Error>> {
    let version = pinned_version()?;
    if let Ok(named) = std::env::var("LYS_SPICEDB_BINARY") {
        let output = Command::new(&named).arg("version").output()?;
        let said = String::from_utf8_lossy(&output.stdout);
        if !said.contains(&version) {
            return Err(format!("{named} reports {said}, not the pinned SpiceDB {version}").into());
        }
        return Ok(PathBuf::from(named));
    }
    let platform = platform()?;
    let fetching = FETCH.lock().unwrap_or_else(PoisonError::into_inner);
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/tmp/spicedb")
        .join(&version)
        .join(platform);
    let held = dir.join("spicedb");
    if held.exists() {
        return Ok(held);
    }
    let digest = ARCHIVES
        .iter()
        .find(|(pinned, on, _)| *pinned == version && *on == platform)
        .map(|(_, _, digest)| *digest)
        .ok_or_else(|| {
            format!("no archive digest is pinned for SpiceDB {version} on {platform}; pin it from the release's checksums.txt")
        })?;
    std::fs::create_dir_all(&dir)?;
    let work = dir.join(unique("download"));
    std::fs::create_dir_all(&work)?;
    let archive = work.join("spicedb.tar.gz");
    let number = version.trim_start_matches('v');
    let url = format!(
        "https://github.com/authzed/spicedb/releases/download/{version}/spicedb_{number}_{platform}.tar.gz"
    );
    run(Command::new("curl")
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--proto",
            "=https",
        ])
        .arg("--output")
        .arg(&archive)
        .arg(&url))?;
    let mut bytes = Vec::new();
    std::fs::File::open(&archive)?.read_to_end(&mut bytes)?;
    let found: String = Sha256::digest(&bytes)
        .iter()
        .fold(String::new(), |text, byte| format!("{text}{byte:02x}"));
    if found != digest {
        return Err(format!("{url} has SHA-256 {found}, not the pinned {digest}").into());
    }
    run(Command::new("tar")
        .arg("-xzf")
        .arg(&archive)
        .arg("-C")
        .arg(&work)
        .arg("spicedb"))?;
    std::fs::rename(work.join("spicedb"), &held)?;
    std::fs::remove_dir_all(&work)?;
    drop(fetching);
    Ok(held)
}

/// One running disposable `SpiceDB`.
pub struct SpiceDb {
    child: Child,
    /// Its gRPC address, `http://127.0.0.1:port`.
    pub address: String,
}

impl Drop for SpiceDb {
    fn drop(&mut self) {
        if self.child.kill().is_ok() {
            self.child.wait().ok();
        }
    }
}

impl SpiceDb {
    /// Start the pinned release in its testing mode, taking at most
    /// `max_updates_per_write` updates in one write when a cap is named, and
    /// return once it serves.
    pub fn start(max_updates_per_write: Option<u16>) -> Result<Self, Box<dyn Error>> {
        let port = std::net::TcpListener::bind("127.0.0.1:0")?
            .local_addr()?
            .port();
        let mut command = Command::new(binary()?);
        command
            .arg("serve-testing")
            .arg("--grpc-addr")
            .arg(format!("127.0.0.1:{port}"))
            .args([
                "--readonly-grpc-enabled=false",
                "--http-enabled=false",
                "--skip-release-check",
                "--log-format",
                "json",
                "--log-level",
                "info",
            ]);
        if let Some(cap) = max_updates_per_write {
            command
                .arg("--write-relationships-max-updates-per-call")
                .arg(cap.to_string());
        }
        let mut child = command
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()?;
        let said = child.stderr.take().ok_or("SpiceDB's log is not readable")?;
        let mut lines = BufReader::new(said).lines();
        let mut before = Vec::new();
        loop {
            let Some(line) = lines.next() else {
                child.wait()?;
                return Err(
                    format!("SpiceDB stopped before serving: {}", before.join("\n")).into(),
                );
            };
            let line = line?;
            if line.contains(SERVING) {
                break;
            }
            before.push(line);
        }
        std::thread::spawn(move || lines.map_while(Result::ok).for_each(drop));
        Ok(Self {
            child,
            address: format!("http://127.0.0.1:{port}"),
        })
    }

    /// A client configuration presenting `key`, so the client reaches the
    /// store this `SpiceDB` keeps for that key.
    pub fn client_config(&self, key: &str) -> ClientConfig {
        ClientConfig::new(&self.address, key)
    }
}
