//! The served binary for the screen route tests: a broker made and seeded
//! with the product's own commands, `serve` started on a loopback port, and
//! every request signed on a person's behalf by a screen service the broker
//! trusts, with the library's own signer.

use std::error::Error;
use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use lys_core::Ed25519Identity;
use lys_secrets::{OnBehalf, new_operation_id, request_digest};
use reqwest::Method;
use serde_json::Value;
use tempfile::TempDir;

pub type Failure = Box<dyn Error>;
pub type TestResult = Result<(), Failure>;

const BINARY: &str = env!("CARGO_BIN_EXE_lys-secrets");
const SERVICE: &str = "identity";
pub const OWNER: &str = "person-owner";
pub const OTHER: &str = "person-other";
pub const OWNED: &str = "owner-key";
pub const HIDDEN: &str = "other-key";
const LISTENING: &str = "lys-secrets proxy listening on";
const CONNECT_ATTEMPTS: usize = 50;

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

/// A broker made by `init`, holding two sealed secrets: one owned by
/// `OWNER`, one owned by `OTHER` and neither owned by nor granted to
/// `OWNER`. Each owner holds a handle on its own secret, and `SERVICE` is
/// trusted to speak for a signed-in person.
pub struct Seeded {
    dir: TempDir,
    service: Ed25519Identity,
    pub owned_handle: String,
    pub hidden_handle: String,
}

impl Seeded {
    pub fn new() -> Result<Self, Failure> {
        let dir = tempfile::tempdir()?;
        let root = dir.path().join("broker");
        let keys = dir.path().join("keys");
        let service_dir = dir.path().join("service");
        let holders = dir.path().join("holders");
        std::fs::create_dir_all(&service_dir)?;
        std::fs::create_dir_all(&holders)?;
        let (root, keys) = (text(&root)?, text(&keys)?);
        let at = ["--root", root, "--keys", keys];

        lys(&[&["init"][..], &at].concat(), b"")?;
        for (name, owner) in [(OWNED, OWNER), (HIDDEN, OTHER)] {
            lys(
                &[
                    &["seal"][..],
                    &at,
                    &["--name", name, "--owner", owner],
                    &["--upstream", "http://127.0.0.1:9"],
                ]
                .concat(),
                b"a sealed test credential",
            )?;
            lys(
                &[
                    "grant",
                    "--root",
                    root,
                    "--identity",
                    owner,
                    "--secret",
                    name,
                    "--by",
                    owner,
                ],
                b"",
            )?;
        }

        let service_key = service_dir.join("identity.key");
        let public = lys(&["service-key", "--key", text(&service_key)?], b"")?;
        lys(
            &[
                &["trust-service"][..],
                &at,
                &["--name", SERVICE, "--public-key", public.trim()],
            ]
            .concat(),
            b"",
        )?;
        let service = Ed25519Identity::load(&service_key)?;
        let owned_handle = issue(&at, &holders, OWNER, OWNED)?;
        let hidden_handle = issue(&at, &holders, OTHER, HIDDEN)?;
        Ok(Self {
            dir,
            service,
            owned_handle,
            hidden_handle,
        })
    }

    fn root(&self) -> PathBuf {
        self.dir.path().join("broker")
    }

    fn keys(&self) -> PathBuf {
        self.dir.path().join("keys")
    }
}

/// Issues a handle on `secret` to `identity` with the `issue` command and
/// answers its id. The raw handle it also prints is left unread.
fn issue(at: &[&str], holders: &Path, identity: &str, secret: &str) -> Result<String, Failure> {
    let key = holders.join(format!("{identity}.key"));
    Ed25519Identity::load_or_generate(&key)?;
    let printed = lys(
        &[
            &["issue"][..],
            at,
            &["--identity", identity, "--holder-key", text(&key)?],
            &["--secret", secret],
        ]
        .concat(),
        b"",
    )?;
    printed
        .lines()
        .find_map(|line| line.strip_prefix("handle_id "))
        .map(str::to_owned)
        .ok_or_else(|| "issue printed no handle id".into())
}

/// The `serve` command running on a loopback port, stopped when dropped.
pub struct Served {
    pub seeded: Seeded,
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
    /// Starts `serve` on a port picked by binding and releasing it, and
    /// answers once the broker says it is listening and a connection
    /// reaches it.
    pub fn start(seeded: Seeded) -> Result<Self, Failure> {
        let address = TcpListener::bind("127.0.0.1:0")?.local_addr()?;
        let listen = address.to_string();
        let (root, keys) = (seeded.root(), seeded.keys());
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
            .stderr(Stdio::inherit())
            .spawn()?;
        let stdout = child.stdout.take().map(BufReader::new);
        let mut served = Self {
            seeded,
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

    fn until_listening(&mut self) -> TestResult {
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

    /// A request for `target` (path and query, sent as written) signed by
    /// the trusted service on `person`'s behalf under a fresh operation id.
    pub fn signed(
        &self,
        method: &Method,
        target: &str,
        body: &[u8],
        person: &str,
    ) -> Result<reqwest::Request, Failure> {
        let url = reqwest::Url::parse(&format!("http://{}{target}", self.address))?;
        let path = match url.query() {
            Some(query) => format!("{}?{query}", url.path()),
            None => url.path().to_owned(),
        };
        let asked = OnBehalf::sign(
            SERVICE,
            person,
            &new_operation_id()?,
            now_ms()?,
            request_digest(method.as_str(), &path, body)?,
            &self.seeded.service,
        )?;
        let [service, on_behalf_of, operation, signed_at, signature] = asked.to_wire();
        Ok(self
            .client
            .request(method.clone(), url)
            .header("lys-service", service)
            .header("lys-on-behalf-of", on_behalf_of)
            .header("lys-operation", operation)
            .header("lys-signed-at", signed_at)
            .header("lys-service-signature", signature)
            .header("content-type", "application/json")
            .body(body.to_vec())
            .build()?)
    }

    pub fn send(&self, request: reqwest::Request) -> Result<(u16, String), Failure> {
        let answered = self.runtime.block_on(async {
            let response = self.client.execute(request).await?;
            let status = response.status().as_u16();
            let body = response.text().await?;
            Ok::<_, reqwest::Error>((status, body))
        })?;
        Ok(answered)
    }

    pub fn ask(
        &self,
        method: &Method,
        target: &str,
        body: &[u8],
        person: &str,
    ) -> Result<(u16, String), Failure> {
        self.send(self.signed(method, target, body, person)?)
    }
}

fn now_ms() -> Result<i64, Failure> {
    Ok(i64::try_from(
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis(),
    )?)
}

/// Every byte of `text` percent-encoded, so only a route that decodes its
/// query reads the text back.
pub fn percent_encoded(text: &str) -> String {
    const DIGITS: &[u8; 16] = b"0123456789ABCDEF";
    let mut encoded = String::with_capacity(text.len() * 3);
    for byte in text.bytes() {
        encoded.push('%');
        encoded.push(char::from(DIGITS[usize::from(byte >> 4)]));
        encoded.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    encoded
}

pub fn names(listing: &Value) -> Vec<String> {
    listing["secrets"]
        .as_array()
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| entry["name"].as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}
