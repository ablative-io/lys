//! The served binary holding a signing key, for the signature route tests.
//! A broker is made and seeded with the product's own commands: the key
//! sealed by `seal-signing-key` with its seed on standard input, a
//! credential beside it, a handle on each issued to their owner, and a
//! screen service trusted to speak for a signed-in person. `serve` runs on a
//! loopback port, its standard error inherited or written to a file in the
//! served root. A request is signed as the holder of a handle, with a
//! presentation bound to that very request, or as the owner by the trusted
//! service. The seed is generated here and is no credential.
//!
//! This lives apart from `served.rs`, which the screen route tests use
//! whole: a test crate uses every item of each support file it includes, or
//! the build refuses the unused ones as dead code.

use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use lys_core::Ed25519Identity;
use lys_secrets::{HandleId, OnBehalf, Presentation, new_operation_id, request_digest};
use rand::TryRngCore;
use rand::rngs::OsRng;
use reqwest::Method;
use serde_json::{Value, json};
use tempfile::TempDir;

pub type Failure = Box<dyn Error>;

const BINARY: &str = env!("CARGO_BIN_EXE_lys-secrets");
const SERVICE: &str = "identity";
/// The person who owns the key and the credential and holds a handle on
/// each.
pub const OWNER: &str = "person-owner";
/// The signing key.
pub const KEY: &str = "agent-key";
/// The credential beside it.
pub const CREDENTIAL: &str = "owner-credential";
/// The route the broker signs at.
pub const SIGNATURE: &str = "/_lys/signature";
const LISTENING: &str = "lys-secrets proxy listening on";
const CONNECT_ATTEMPTS: usize = 50;
/// The file in the served root the broker's standard error is written to.
const LOG: &str = "broker.log";

/// Runs one command of the binary, feeding `input` on standard input, and
/// answers its standard output; a failed command is an error carrying its
/// standard error.
fn lys(args: &[&str], input: &[u8]) -> Result<String, Failure> {
    let mut child = Command::new(BINARY)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or("the command's standard input was not piped")?;
    stdin.write_all(input)?;
    drop(stdin);
    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(format!(
            "lys-secrets {} failed ({}): {}",
            args.first().copied().unwrap_or_default(),
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(String::from_utf8(output.stdout)?)
}

fn text(path: &Path) -> Result<&str, Failure> {
    path.to_str()
        .ok_or_else(|| format!("{} is not UTF-8", path.display()).into())
}

fn now_ms() -> Result<i64, Failure> {
    Ok(i64::try_from(
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis(),
    )?)
}

/// A handle as `issue` printed it: its id and the raw handle in hex.
pub struct Handle {
    pub id: String,
    token: String,
}

/// A broker made and seeded by the product's commands, holding the signing
/// key sealed from `seed`.
pub struct Keyed {
    dir: TempDir,
    service: Ed25519Identity,
    holder: Ed25519Identity,
    /// The seed the key was sealed from.
    pub seed: [u8; 32],
    /// The owner's handle on the signing key.
    pub on_key: Handle,
    /// The owner's handle on the credential.
    pub on_credential: Handle,
}

impl Keyed {
    /// The world, with a handle on the key good for `uses` uses. A route is
    /// set for the key by hand, as an operator might, so a proxied request
    /// for it reaches the broker's admission.
    pub fn new(uses: u64) -> Result<Self, Failure> {
        let dir = tempfile::tempdir()?;
        let root = dir.path().join("broker");
        let keys = dir.path().join("keys");
        let service_dir = dir.path().join("service");
        std::fs::create_dir_all(&service_dir)?;
        let (root_text, keys_text) = (text(&root)?, text(&keys)?);
        let at = ["--root", root_text, "--keys", keys_text];

        lys(&[&["init"][..], &at].concat(), b"")?;
        let mut seed = [0u8; 32];
        OsRng
            .try_fill_bytes(&mut seed)
            .map_err(|error| error.to_string())?;
        seal_signing_key(&at, &seed)?;
        lys(
            &[
                &["seal"][..],
                &at,
                &["--name", CREDENTIAL, "--owner", OWNER],
                &["--upstream", "http://127.0.0.1:9"],
            ]
            .concat(),
            b"a sealed test credential",
        )?;
        let routes = root.join("routes.json");
        let mut table: Value = serde_json::from_slice(&std::fs::read(&routes)?)?;
        table[KEY] = json!({
            "upstream": "http://127.0.0.1:9", "header": "authorization", "prefix": "Bearer "
        });
        std::fs::write(&routes, serde_json::to_vec_pretty(&table)?)?;
        for secret in [KEY, CREDENTIAL] {
            let grant = ["--identity", OWNER, "--secret", secret, "--by", OWNER];
            lys(&[&["grant", "--root", root_text][..], &grant].concat(), b"")?;
        }

        let service_key = service_dir.join("identity.key");
        let public = lys(&["service-key", "--key", text(&service_key)?], b"")?;
        let trusted = ["--name", SERVICE, "--public-key", public.trim()];
        lys(&[&["trust-service"][..], &at, &trusted].concat(), b"")?;
        let service = Ed25519Identity::load(&service_key)?;

        let holder_key = dir.path().join("holder.key");
        let holder = Ed25519Identity::load_or_generate(&holder_key)?;
        let on_key = issue(&at, &holder_key, KEY, uses)?;
        let on_credential = issue(&at, &holder_key, CREDENTIAL, 10)?;
        Ok(Self {
            dir,
            service,
            holder,
            seed,
            on_key,
            on_credential,
        })
    }

    fn root(&self) -> PathBuf {
        self.dir.path().join("broker")
    }

    fn keys(&self) -> PathBuf {
        self.dir.path().join("keys")
    }

    /// Every byte the broker wrote to its standard error while it ran
    /// under [`Served::start_logged`].
    pub fn log(&self) -> Result<Vec<u8>, Failure> {
        Ok(std::fs::read(self.root().join(LOG))?)
    }

    /// Every byte of every file of the audit log, the log's own directory
    /// read whole.
    pub fn audit_bytes(&self) -> Result<Vec<Vec<u8>>, Failure> {
        let mut files = Vec::new();
        let mut folders = vec![self.root().join("audit-log")];
        while let Some(folder) = folders.pop() {
            for found in std::fs::read_dir(&folder)? {
                let path = found?.path();
                if path.is_dir() {
                    folders.push(path);
                } else {
                    files.push(std::fs::read(&path)?);
                }
            }
        }
        Ok(files)
    }
}

/// Runs `seal-signing-key` for `KEY`, owned by `OWNER` and signing for
/// `agent_request`, with `seed` on standard input.
fn seal_signing_key(at: &[&str], seed: &[u8]) -> Result<(), Failure> {
    let named = ["--name", KEY, "--owner", OWNER];
    let purpose = ["--purpose", "agent_request"];
    lys(
        &[&["seal-signing-key"][..], at, &named, &purpose].concat(),
        seed,
    )?;
    Ok(())
}

/// Issues a handle on `secret` to `OWNER`, bound to the key in `holder_key`,
/// good for `uses` uses.
fn issue(at: &[&str], holder_key: &Path, secret: &str, uses: u64) -> Result<Handle, Failure> {
    let uses = uses.to_string();
    let asked = ["--identity", OWNER, "--holder-key", text(holder_key)?];
    let printed = lys(
        &[
            &["issue"][..],
            at,
            &asked,
            &["--secret", secret, "--uses", &uses],
        ]
        .concat(),
        b"",
    )?;
    let value = |label: &str| {
        printed
            .lines()
            .find_map(|line| line.strip_prefix(label))
            .map(str::to_owned)
            .ok_or_else(|| format!("issue printed no {label}"))
    };
    Ok(Handle {
        id: value("handle_id ")?,
        token: value("handle ")?,
    })
}

/// The `serve` command running on a loopback port over a [`Keyed`] broker,
/// stopped when dropped. The broker may be started again over the same
/// folders once this one is dropped.
pub struct Served {
    child: Child,
    stdout: Option<BufReader<ChildStdout>>,
    address: SocketAddr,
    runtime: tokio::runtime::Runtime,
    client: reqwest::Client,
}

impl Drop for Served {
    fn drop(&mut self) {
        if let Err(error) = self.child.kill() {
            eprintln!("stopping the served broker: {error}");
        }
        if let Err(error) = self.child.wait() {
            eprintln!("reaping the served broker: {error}");
        }
    }
}

impl Served {
    /// Starts `serve` over `keyed`, its standard error inherited.
    pub fn start(keyed: &Keyed) -> Result<Self, Failure> {
        Self::launch(keyed, Stdio::inherit())
    }

    /// Starts `serve` over `keyed` with its standard error written to a file
    /// in the served root, read back by [`Keyed::log`].
    pub fn start_logged(keyed: &Keyed) -> Result<Self, Failure> {
        let log = File::create(keyed.root().join(LOG))?;
        Self::launch(keyed, Stdio::from(log))
    }

    /// Starts `serve` on a port picked by binding and releasing it, and
    /// answers once the broker says it is listening and a connection
    /// reaches it.
    fn launch(keyed: &Keyed, stderr: Stdio) -> Result<Self, Failure> {
        let address = TcpListener::bind("127.0.0.1:0")?.local_addr()?;
        let listen = address.to_string();
        let (root, keys) = (keyed.root(), keyed.keys());
        let mut child = Command::new(BINARY)
            .args([
                "serve",
                "--root",
                text(&root)?,
                "--keys",
                text(&keys)?,
                "--listen",
                &listen,
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(stderr)
            .spawn()?;
        let stdout = child.stdout.take().map(BufReader::new);
        let mut served = Self {
            child,
            stdout,
            address,
            runtime: tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?,
            client: reqwest::Client::new(),
        };
        served.until_listening()?;
        Ok(served)
    }

    fn until_listening(&mut self) -> Result<(), Failure> {
        let stdout = self
            .stdout
            .as_mut()
            .ok_or("the broker's standard output was not piped")?;
        let mut line = String::new();
        loop {
            line.clear();
            if stdout.read_line(&mut line)? == 0 {
                let status = self.child.wait()?;
                return Err(format!("serve ended before listening: {status}").into());
            }
            if line.starts_with(LISTENING) {
                break;
            }
        }
        (0..CONNECT_ATTEMPTS)
            .find_map(|_attempt| TcpStream::connect(self.address).ok())
            .map(drop)
            .ok_or_else(|| format!("no connection reached {}", self.address).into())
    }

    fn url(&self, target: &str) -> Result<reqwest::Url, Failure> {
        let url = format!("http://{}{target}", self.address);
        Ok(reqwest::Url::parse(&url)?)
    }

    fn send(&self, request: reqwest::Request) -> Result<(u16, String), Failure> {
        let answered = self.runtime.block_on(async {
            let response = self.client.execute(request).await?;
            let status = response.status().as_u16();
            let body = response.text().await?;
            Ok::<_, reqwest::Error>((status, body))
        })?;
        Ok(answered)
    }

    /// `method` `target` with `body`, as the holder of `handle` under a
    /// fresh operation id, the presentation bound to this very request; an
    /// unsigned one when `signs` is false.
    pub fn held(
        &self,
        keyed: &Keyed,
        handle: &Handle,
        (method, target, body): (&Method, &str, &[u8]),
        signs: bool,
    ) -> Result<(u16, String), Failure> {
        let presented = Presentation::sign(
            &HandleId::from_text(&handle.id),
            &new_operation_id()?,
            now_ms()?,
            request_digest(method.as_str(), target, body)?,
            &keyed.holder,
        )?;
        let [id, operation, signed_at, signature] = presented.to_wire();
        let mut request = self
            .client
            .request(method.clone(), self.url(target)?)
            .header("lys-handle", &handle.token)
            .header("lys-handle-id", id)
            .header("lys-operation", operation)
            .header("lys-signed-at", signed_at)
            .header("content-type", "application/json")
            .body(body.to_vec());
        if signs {
            request = request.header("lys-presentation", signature);
        }
        self.send(request.build()?)
    }

    /// `POST /_lys/signature` with `body`, as the holder of `handle`.
    pub fn sign(
        &self,
        keyed: &Keyed,
        handle: &Handle,
        body: &[u8],
    ) -> Result<(u16, String), Failure> {
        self.held(keyed, handle, (&Method::POST, SIGNATURE, body), true)
    }

    /// `method` `target` with `body`, signed by the trusted service on the
    /// owner's behalf under a fresh operation id.
    pub fn ask(
        &self,
        keyed: &Keyed,
        method: &Method,
        target: &str,
        body: &[u8],
    ) -> Result<(u16, String), Failure> {
        let asked = OnBehalf::sign(
            SERVICE,
            OWNER,
            &new_operation_id()?,
            now_ms()?,
            request_digest(method.as_str(), target, body)?,
            &keyed.service,
        )?;
        let [service, on_behalf_of, operation, signed_at, signature] = asked.to_wire();
        let request = self
            .client
            .request(method.clone(), self.url(target)?)
            .header("lys-service", service)
            .header("lys-on-behalf-of", on_behalf_of)
            .header("lys-operation", operation)
            .header("lys-signed-at", signed_at)
            .header("lys-service-signature", signature)
            .header("content-type", "application/json")
            .body(body.to_vec())
            .build()?;
        self.send(request)
    }
}
