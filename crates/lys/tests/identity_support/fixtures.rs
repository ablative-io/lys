//! A fresh venue directory per test: a copy of
//! `deploy/identity/config.example.toml` edited only where a parallel,
//! disposable install must differ (its compose project, its free ports, and
//! when a test asks, its database host), and the `lys` binary run against it.
//! Every credential comes from `lys identity prepare`; none is written here.

use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use super::TestResult;

/// The shortest run of a secret's bytes that counts as a leak.
pub const LEAK_WINDOW: usize = 8;

/// The repository root.
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The ports a stack publishes on loopback.
#[derive(Debug, Clone, Copy)]
pub struct Ports {
    /// Rauthy's listener.
    pub rauthy: u16,
    /// SpiceDB's HTTP gateway.
    pub spicedb_http: u16,
    /// SpiceDB's gRPC API.
    pub spicedb_grpc: u16,
    /// The local PostgreSQL service.
    pub postgres: u16,
}

/// One disposable deployment directory.
pub struct Fixture {
    /// The venue directory; removed on drop.
    pub dir: tempfile::TempDir,
    /// The edited configuration file inside it.
    pub config: PathBuf,
    /// The compose project name.
    pub project: String,
    /// The published ports.
    pub ports: Ports,
}

impl Fixture {
    /// A fresh venue whose database host is `database_host` (`postgres` for
    /// the compose file's own service).
    pub fn new(label: &str, database_host: &str) -> TestResult<Self> {
        let dir = tempfile::tempdir()?;
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.subsec_nanos();
        let project = format!("lysid-{label}-{}-{nanos}", std::process::id());
        let ports = Ports {
            rauthy: free_port()?,
            spicedb_http: free_port()?,
            spicedb_grpc: free_port()?,
            postgres: free_port()?,
        };
        let example = std::fs::read_to_string(repo_root().join("deploy/identity/config.example.toml"))?;
        let text = [
            ("compose_project = \"lys-identity\"", format!("compose_project = \"{project}\"")),
            ("host = \"postgres\"", format!("host = \"{database_host}\"")),
            ("local_publish = \"127.0.0.1:55432\"", format!("local_publish = \"127.0.0.1:{}\"", ports.postgres)),
            ("public_origin = \"http://localhost:8080\"", format!("public_origin = \"http://localhost:{}\"", ports.rauthy)),
            ("publish = \"127.0.0.1:8080\"", format!("publish = \"127.0.0.1:{}\"", ports.rauthy)),
            ("http_publish = \"127.0.0.1:8443\"", format!("http_publish = \"127.0.0.1:{}\"", ports.spicedb_http)),
            ("grpc_publish = \"127.0.0.1:50051\"", format!("grpc_publish = \"127.0.0.1:{}\"", ports.spicedb_grpc)),
        ]
        .iter()
        .try_fold(example, |text, (from, to)| edit(&text, from, to))?;
        let config = dir.path().join("config.toml");
        std::fs::write(&config, text)?;
        Ok(Self {
            dir,
            config,
            project,
            ports,
        })
    }

    /// The state directory `prepare` writes into.
    pub fn state_dir(&self) -> PathBuf {
        self.dir.path().join("state")
    }

    /// Rauthy's published address.
    pub fn rauthy(&self) -> String {
        format!("127.0.0.1:{}", self.ports.rauthy)
    }

    /// SpiceDB's published HTTP address.
    pub fn spicedb(&self) -> String {
        format!("127.0.0.1:{}", self.ports.spicedb_http)
    }

    /// Run `lys` with `args`.
    pub fn lys(&self, args: &[&str]) -> TestResult<Output> {
        Ok(Command::new(env!("CARGO_BIN_EXE_lys"))
            .args(args)
            .current_dir(repo_root())
            .output()?)
    }

    /// Run `lys identity <subcommand> --config <config>` plus `extra`.
    pub fn identity(&self, subcommand: &str, extra: &[&str]) -> TestResult<Output> {
        let config = self.config.to_string_lossy().to_string();
        let mut args = vec!["identity", subcommand, "--config", config.as_str()];
        args.extend_from_slice(extra);
        self.lys(&args)
    }

    /// Rewrite the configuration file, replacing `from` with `to` once.
    pub fn edit_config(&self, from: &str, to: &str) -> TestResult {
        let text = std::fs::read_to_string(&self.config)?;
        std::fs::write(&self.config, edit(&text, from, to)?)?;
        Ok(())
    }

    /// Every credential `prepare` and `configure` generated or kept.
    pub fn secrets(&self) -> TestResult<Vec<String>> {
        let mut secrets = Vec::new();
        for entry in std::fs::read_dir(self.state_dir().join("credentials"))? {
            secrets.push(std::fs::read_to_string(entry?.path())?);
        }
        if secrets.len() < 9 {
            return Err(format!("only {} credentials were found", secrets.len()).into());
        }
        Ok(secrets)
    }

    /// Whether `text` holds any run of any generated credential.
    pub fn leaks(&self, text: &str) -> TestResult<bool> {
        Ok(self.secrets()?.iter().any(|secret| {
            secret
                .as_bytes()
                .windows(LEAK_WINDOW)
                .any(|window| text.as_bytes().windows(LEAK_WINDOW).any(|run| run == window))
        }))
    }

    /// The `Authorization` header value of the bootstrap API key.
    pub fn api_key_header(&self) -> TestResult<String> {
        let secret = std::fs::read_to_string(self.state_dir().join("credentials/rauthy_api_key"))?;
        Ok(format!("API-Key lys_identity${secret}"))
    }

    /// SpiceDB's preshared key as a bearer header value.
    pub fn spicedb_bearer(&self) -> TestResult<String> {
        let secret = std::fs::read_to_string(self.state_dir().join("credentials/spicedb_preshared_key"))?;
        Ok(format!("Bearer {secret}"))
    }
}

/// `text` with the single occurrence of `from` replaced by `to`.
pub fn edit(text: &str, from: &str, to: &str) -> TestResult<String> {
    match text.matches(from).count() {
        1 => Ok(text.replacen(from, to, 1)),
        count => Err(format!("expected one {from:?}, found {count}").into()),
    }
}

/// Text of a process's stdout and stderr together.
pub fn output_text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn free_port() -> TestResult<u16> {
    Ok(TcpListener::bind("127.0.0.1:0")?.local_addr()?.port())
}
