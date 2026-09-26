//! A fresh venue for one container-backed identity target: a temporary
//! directory holding a deployment config made from
//! deploy/identity/config.example.toml, with free host ports, a compose
//! project of its own and a database host that is configuration.
//!
//! Test identities only (CN2): every credential a venue uses is generated
//! by `lys identity prepare` into the venue's private directory and dies
//! with it; nothing here is a real credential.

use std::net::{IpAddr, TcpListener, UdpSocket};
use std::path::PathBuf;
use std::process::{Command, Output};

use tempfile::TempDir;

/// Every fallible support call and every test returns this.
pub type Failure = Box<dyn std::error::Error>;

/// The repository root; every repository path is spelled from here.
pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Output bytes as text.
pub fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// The single JSON object a `--json` command printed on stdout.
pub fn json_of(output: &Output) -> Result<serde_json::Value, Failure> {
    Ok(serde_json::from_slice(&output.stdout)?)
}

/// The host ports one venue publishes on.
#[derive(Debug, Clone, Copy)]
pub struct Ports {
    /// The local `database` profile's `PostgreSQL`.
    pub database: u16,
    /// Rauthy's listener.
    pub rauthy: u16,
    /// `SpiceDB`'s HTTP listener.
    pub spicedb_http: u16,
    /// `SpiceDB`'s gRPC listener.
    pub spicedb_grpc: u16,
}

/// One target's deployment directory, config and compose project.
pub struct Venue {
    dir: TempDir,
    project: String,
    ports: Ports,
    database_host: String,
    local_database: bool,
}

impl Venue {
    /// A venue for the target `tag`. With `database_host` `None` the
    /// database runs on this node under the `database` profile and is
    /// dialled at the node's own routable address; with `Some(host)` it is
    /// a network device at `host`, and nothing local stands in for it.
    pub fn fresh(tag: &str, database_host: Option<&str>) -> Result<Self, Failure> {
        let dir = tempfile::tempdir()?;
        std::fs::copy(
            repo_root().join("deploy/identity/config.example.toml"),
            dir.path().join("identity.toml"),
        )?;
        let host = match database_host {
            Some(host) => host.to_string(),
            None => node_address()?.to_string(),
        };
        let venue = Self {
            dir,
            project: format!("lysid-{tag}-{}", std::process::id()),
            ports: free_ports()?,
            database_host: host,
            local_database: database_host.is_none(),
        };
        venue.localise()?;
        Ok(venue)
    }

    /// Points the copied example at this venue's ports, project and host.
    fn localise(&self) -> Result<(), Failure> {
        let ports = self.ports();
        let rauthy = format!("127.0.0.1:{}", ports.rauthy);
        let spicedb_http = format!("127.0.0.1:{}", ports.spicedb_http);
        let database_port = if self.local_database() {
            ports.database
        } else {
            5432
        };
        let settings = [
            ("deployment", "project", quoted(&self.project)),
            ("deployment", "node", quoted("disposable-test-venue")),
            ("database", "host", quoted(&self.database_host)),
            ("database", "port", database_port.to_string()),
            ("database", "publish", quoted(&ports.database.to_string())),
            ("database", "sslmode", quoted("disable")),
            (
                "rauthy",
                "public_origin",
                quoted(&format!("http://localhost:{}", ports.rauthy)),
            ),
            (
                "rauthy",
                "admin_origin",
                quoted(&format!("http://{rauthy}")),
            ),
            ("rauthy", "publish", quoted(&rauthy)),
            (
                "spicedb",
                "http_origin",
                quoted(&format!("http://{spicedb_http}")),
            ),
            ("spicedb", "http_publish", quoted(&spicedb_http)),
            (
                "spicedb",
                "grpc_publish",
                quoted(&format!("127.0.0.1:{}", ports.spicedb_grpc)),
            ),
        ];
        for (section, key, value) in &settings {
            self.set(section, key, value)?;
        }
        Ok(())
    }

    /// Sets `key` in `[section]` of the venue config to the TOML `value`,
    /// refusing when the key is not there so an edit can never land nowhere.
    pub fn set(&self, section: &str, key: &str, value: &str) -> Result<(), Failure> {
        let path = self.config();
        let original = std::fs::read_to_string(&path)?;
        let header = format!("[{section}]");
        let prefix = format!("{key} =");
        let mut in_section = false;
        let mut replaced = false;
        let mut lines = Vec::new();
        for line in original.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') {
                in_section = trimmed == header;
            }
            if in_section && !replaced && trimmed.starts_with(&prefix) {
                lines.push(format!("{key} = {value}"));
                replaced = true;
            } else {
                lines.push(line.to_string());
            }
        }
        if !replaced {
            return Err(format!("{section}.{key} is not in the venue config").into());
        }
        std::fs::write(&path, lines.join("\n") + "\n")?;
        Ok(())
    }

    /// The venue's deployment config.
    pub fn config(&self) -> PathBuf {
        self.dir.path().join("identity.toml")
    }

    /// The venue's private directory, as the config names it.
    pub fn private_dir(&self) -> PathBuf {
        self.dir.path().join("private")
    }

    /// The compose project name.
    pub fn project(&self) -> &str {
        &self.project
    }

    /// The host ports the venue publishes on.
    pub fn ports(&self) -> Ports {
        self.ports
    }

    /// The configured database host.
    pub fn database_host(&self) -> &str {
        &self.database_host
    }

    /// Whether the database runs on this node under the `database` profile.
    pub fn local_database(&self) -> bool {
        self.local_database
    }

    /// One generated credential of the venue, read for a fixture that must
    /// hand it on (psql, `SpiceDB`'s preshared key). Never printed.
    pub fn secret(&self, name: &str) -> Result<String, Failure> {
        Ok(std::fs::read_to_string(
            self.private_dir().join("secrets").join(name),
        )?)
    }

    /// Every generated credential of the venue; empty before `prepare`.
    pub fn secrets(&self) -> Result<Vec<String>, Failure> {
        let dir = self.private_dir().join("secrets");
        if !dir.is_dir() {
            return Ok(Vec::new());
        }
        let mut secrets = Vec::new();
        for entry in std::fs::read_dir(dir)? {
            let path = entry?.path();
            if path.is_file() && !path.to_string_lossy().ends_with(".tmp") {
                secrets.push(std::fs::read_to_string(path)?);
            }
        }
        Ok(secrets)
    }

    /// Runs the built `lys` binary, and refuses the run as `secret_leaked`
    /// when anything it printed carries a generated credential of this venue.
    pub fn lys(&self, args: &[&str]) -> Result<Output, Failure> {
        let output = Command::new(env!("CARGO_BIN_EXE_lys"))
            .args(args)
            .output()?;
        let printed = text(&output.stdout) + &text(&output.stderr);
        let secrets = self.secrets()?;
        if secrets
            .iter()
            .any(|secret| printed.contains(secret.as_str()))
        {
            return Err(format!(
                "secret_leaked: `lys {}` printed a generated secret",
                args.join(" ")
            )
            .into());
        }
        Ok(output)
    }
}

fn quoted(value: &str) -> String {
    format!("\"{value}\"")
}

/// Four distinct free host ports, all held open until each is chosen.
fn free_ports() -> Result<Ports, Failure> {
    let listeners = [
        TcpListener::bind("0.0.0.0:0")?,
        TcpListener::bind("127.0.0.1:0")?,
        TcpListener::bind("127.0.0.1:0")?,
        TcpListener::bind("127.0.0.1:0")?,
    ];
    Ok(Ports {
        database: listeners[0].local_addr()?.port(),
        rauthy: listeners[1].local_addr()?.port(),
        spicedb_http: listeners[2].local_addr()?.port(),
        spicedb_grpc: listeners[3].local_addr()?.port(),
    })
}

/// This node's routable address, which both the containers and this host
/// reach. Connecting a UDP socket sends nothing: it only selects the route,
/// here towards TEST-NET-1.
fn node_address() -> Result<IpAddr, Failure> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.connect("192.0.2.1:9")?;
    let address = socket.local_addr()?.ip();
    if address.is_unspecified() || address.is_loopback() {
        return Err(
            "no_routable_node_address: the local database needs an address containers reach".into(),
        );
    }
    Ok(address)
}
