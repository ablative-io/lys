//! PKCE sign-in keeps the approving person's pass in an owner-only client file.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use zeroize::{Zeroize, Zeroizing};

use crate::cli::SignInArgs;
use crate::commands::error::CliResult;
use crate::commands::output::Emitter;
use crate::commands::seat_client::refused;
use crate::identity::install::layout::Layout;
use crate::identity::private_files;
use lys_runner::dial::HttpClient;

/// The native client's stored credential. Secrets have no Debug implementation.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pass {
    /// The exact server base this credential belongs to.
    pub server: String,
    /// Its verified person identifier.
    pub person: String,
    /// When its access authority ends.
    pub expires_at: u64,
    /// The bearer sent only to its recorded server.
    pub access_token: String,
    /// The one-use renewal credential.
    pub refresh_token: String,
}

impl Drop for Pass {
    fn drop(&mut self) {
        self.access_token.zeroize();
        self.refresh_token.zeroize();
    }
}

/// Public sign-in receipt, containing no credential.
#[derive(Serialize)]
pub struct SignedIn {
    /// The person who approved the terminal.
    pub person: String,
    /// The owner-only file written.
    pub file: PathBuf,
}

fn instant() -> CliResult<u64> {
    u64::try_from(chrono::Utc::now().timestamp()).map_err(|_| {
        refused(
            "sign_in_clock_unavailable",
            "the clock is before the Unix epoch",
        )
    })
}

/// Normalize the client's base without accepting URL credentials or query parameters.
///
/// # Errors
/// Refuses unsafe or malformed server bases.
pub fn base(server: &str) -> CliResult<String> {
    let server = server.trim_end_matches('/');
    if server.contains(['?', '#', '@']) || server.bytes().any(|byte| byte.is_ascii_control()) {
        return Err(refused(
            "server_base_invalid",
            "the server base cannot carry credentials, a query or a fragment",
        ));
    }
    HttpClient::new(server)?;
    Ok(server.to_owned())
}

fn origin(server: &str) -> CliResult<&str> {
    let scheme = server
        .find("://")
        .ok_or_else(|| refused("server_base_invalid", "name an HTTPS server"))?
        + 3;
    let end = server[scheme..]
        .find('/')
        .map_or(server.len(), |offset| scheme + offset);
    Ok(&server[..end])
}

/// The pass file is independent of an identity-service install.
#[must_use]
pub fn pass_file(root: &Path) -> PathBuf {
    root.join("client/person-pass.json")
}

/// Read the recorded pass, checking its file permissions before reading any bytes.
///
/// # Errors
/// Refuses absent, malformed, open-mode or expired passes with the sign-in act named.
pub fn load_at(root: &Path, server: Option<&str>, check_expiry: bool) -> CliResult<Pass> {
    let path = pass_file(root);
    let bytes = private_files::read(&path)?.ok_or_else(|| {
        refused(
            "person_pass_missing",
            "this terminal is not signed in; run `lys sign-in --server <base>`",
        )
    })?;
    let pass: Pass = serde_json::from_slice(&bytes).map_err(|_| {
        refused(
            "person_pass_unreadable",
            "the client pass cannot be read; run `lys sign-in --server <base>`",
        )
    })?;
    if server.is_some_and(|server| pass.server != server)
        || pass.access_token.is_empty()
        || pass.refresh_token.is_empty()
        || pass
            .access_token
            .bytes()
            .any(|byte| byte.is_ascii_control())
        || pass
            .refresh_token
            .bytes()
            .any(|byte| byte.is_ascii_control())
    {
        return Err(refused(
            "person_pass_wrong_server",
            "the pass does not belong to this server; run `lys sign-in --server <base>`",
        ));
    }
    if check_expiry && pass.expires_at <= instant()? {
        return Err(refused(
            "person_pass_expired",
            "the person's pass has expired; run `lys sign-in --server <base>`",
        ));
    }
    Ok(pass)
}

/// Read the current client's pass for a seat or attach request.
///
/// # Errors
/// Returns the named pass or layout refusal.
pub fn load(server: Option<&str>) -> CliResult<Pass> {
    load_at(&Layout::discover()?.root, server, true)
}

fn encoded(value: &str) -> String {
    value
        .bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
                char::from(byte).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect()
}

fn form(pairs: &[(&str, &str)]) -> Zeroizing<Vec<u8>> {
    Zeroizing::new(
        pairs
            .iter()
            .map(|(key, value)| format!("{}={}", encoded(key), encoded(value)))
            .collect::<Vec<_>>()
            .join("&")
            .into_bytes(),
    )
}

fn decoded(value: &str) -> CliResult<String> {
    let mut bytes = Vec::new();
    let mut input = value.bytes();
    while let Some(byte) = input.next() {
        if byte == b'%' {
            let digits: String = input.by_ref().take(2).map(char::from).collect();
            if digits.len() != 2 {
                return Err(refused(
                    "sign_in_callback_invalid",
                    "a callback field is incomplete",
                ));
            }
            bytes.push(u8::from_str_radix(&digits, 16).map_err(|_| {
                refused("sign_in_callback_invalid", "a callback field is malformed")
            })?);
        } else {
            bytes.push(if byte == b'+' { b' ' } else { byte });
        }
    }
    String::from_utf8(bytes)
        .map_err(|_| refused("sign_in_callback_invalid", "a callback field is not UTF-8"))
}

fn callback(listener: &TcpListener, expected: &str) -> CliResult<Zeroizing<String>> {
    loop {
        let (mut stream, peer) = listener.accept().map_err(|_| {
            refused(
                "sign_in_callback_unavailable",
                "the loopback receiver could not accept the browser",
            )
        })?;
        if !peer.ip().is_loopback() {
            return Err(refused(
                "sign_in_callback_invalid",
                "the browser did not arrive on loopback",
            ));
        }
        let mut request = String::new();
        let mut input = BufReader::new(&mut stream).take(16384);
        input.read_line(&mut request).map_err(|_| {
            refused(
                "sign_in_callback_invalid",
                "the browser's request could not be read",
            )
        })?;
        loop {
            let mut header = String::new();
            let size = input.read_line(&mut header).map_err(|_| {
                refused(
                    "sign_in_callback_invalid",
                    "the browser's headers could not be read",
                )
            })?;
            if size == 0 {
                return Err(refused(
                    "sign_in_callback_invalid",
                    "the browser's headers are incomplete",
                ));
            }
            if header == "\r\n" {
                break;
            }
        }
        drop(input);
        let target = request
            .split_whitespace()
            .nth(1)
            .ok_or_else(|| refused("sign_in_callback_invalid", "the browser sent no callback"))?;
        if !request.starts_with("GET ") || !target.starts_with("/lys/sign-in?") {
            stream
                .write_all(
                    b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                )
                .map_err(|_| {
                    refused(
                        "sign_in_callback_unavailable",
                        "the browser could not be answered",
                    )
                })?;
            continue;
        }
        let mut values = std::collections::BTreeMap::new();
        for pair in target.trim_start_matches("/lys/sign-in?").split('&') {
            let (key, value) = pair.split_once('=').ok_or_else(|| {
                refused(
                    "sign_in_callback_invalid",
                    "the callback fields are incomplete",
                )
            })?;
            if values.insert(decoded(key)?, decoded(value)?).is_some() {
                return Err(refused(
                    "sign_in_callback_invalid",
                    "a callback field is repeated",
                ));
            }
        }
        if values.get("state").map(String::as_str) != Some(expected) {
            return Err(refused(
                "sign_in_state_wrong",
                "the callback does not answer this sign-in",
            ));
        }
        if values.contains_key("error") {
            return Err(refused(
                "sign_in_refused",
                "the person did not approve this terminal",
            ));
        }
        let code = Zeroizing::new(
            values
                .remove("code")
                .filter(|code| !code.is_empty())
                .ok_or_else(|| {
                    refused(
                        "sign_in_callback_invalid",
                        "the callback supplied no authorization answer",
                    )
                })?,
        );
        let body = b"Signed in to Lys. You can close this tab.";
        write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nCache-Control: no-store\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len())
            .and_then(|()| stream.write_all(body)).map_err(|_| refused("sign_in_callback_unavailable", "the browser could not be answered"))?;
        return Ok(code);
    }
}

fn exchange(endpoint: &str, pairs: &[(&str, &str)]) -> CliResult<Value> {
    let body = form(pairs);
    let response = HttpClient::new(endpoint)?.send(
        "POST",
        "",
        &[("content-type", "application/x-www-form-urlencoded")],
        &body,
    )?;
    if response.status != 200 {
        return Err(refused(
            "sign_in_exchange_refused",
            "Lys refused the sign-in exchange; run `lys sign-in` again",
        ));
    }
    serde_json::from_slice(&response.body).map_err(|_| {
        refused(
            "sign_in_answer_unreadable",
            "Lys did not answer the sign-in exchange in its recorded shape",
        )
    })
}

fn save(root: &Path, server: &str, tokens: Value) -> CliResult<SignedIn> {
    let text = |name: &str| {
        tokens[name]
            .as_str()
            .filter(|value| !value.is_empty() && !value.bytes().any(|byte| byte.is_ascii_control()))
            .map(str::to_owned)
            .ok_or_else(|| {
                refused(
                    "sign_in_answer_unreadable",
                    "Lys did not supply the required sign-in fields",
                )
            })
    };
    let person = text("person")?;
    if !person
        .strip_prefix("person-")
        .is_some_and(|id| id.len() == 32 && id.bytes().all(|byte| byte.is_ascii_hexdigit()))
    {
        return Err(refused(
            "sign_in_answer_unreadable",
            "Lys did not identify a person",
        ));
    }
    let seconds = tokens["expires_in"]
        .as_u64()
        .filter(|seconds| *seconds > 0)
        .ok_or_else(|| {
            refused(
                "sign_in_answer_unreadable",
                "Lys did not name the pass lifetime",
            )
        })?;
    let pass = Pass {
        server: server.to_owned(),
        person: person.clone(),
        access_token: text("access_token")?,
        refresh_token: text("refresh_token")?,
        expires_at: instant()?.saturating_add(seconds),
    };
    let path = pass_file(root);
    private_files::ensure_dir(&root.join("client"))?;
    if let Some(previous) = private_files::read(&path)? {
        private_files::write(&path.with_extension("previous.json"), &previous)?;
    }
    let bytes = Zeroizing::new(serde_json::to_vec(&pass).map_err(|_| {
        refused(
            "sign_in_file_unwritable",
            "the client pass could not be encoded",
        )
    })?);
    private_files::write(&path, &bytes)?;
    Ok(SignedIn { person, file: path })
}

/// Perform the real code flow with an injected browser opener for acceptance fixtures.
///
/// # Errors
/// Refuses unsafe destinations, unapproved callbacks, failed exchanges and insecure files.
pub fn sign_in_at(
    server: &str,
    root: &Path,
    open: impl FnOnce(&str) -> CliResult<()>,
) -> CliResult<SignedIn> {
    let server = base(server)?;
    let origin = origin(&server)?;
    let metadata = HttpClient::new(origin)?.send(
        "GET",
        "/.well-known/oauth-authorization-server",
        &[],
        b"",
    )?;
    if metadata.status != 200 {
        return Err(refused(
            "sign_in_unavailable",
            "Lys did not serve its authorization description",
        ));
    }
    let metadata: Value = serde_json::from_slice(&metadata.body).map_err(|_| {
        refused(
            "sign_in_unavailable",
            "Lys's authorization description could not be read",
        )
    })?;
    let endpoint = |name: &str| {
        metadata[name]
            .as_str()
            .filter(|url| url.starts_with(&format!("{origin}/")))
            .ok_or_else(|| {
                refused(
                    "sign_in_unavailable",
                    "the authorization endpoint is not on this Lys server",
                )
            })
    };
    let token_endpoint = endpoint("token_endpoint")?;
    let existing = private_files::read(&pass_file(root))?;
    if existing.is_some() {
        let previous = load_at(root, Some(&server), false)?;
        match exchange(
            token_endpoint,
            &[
                ("grant_type", "refresh_token"),
                ("client_id", "lys-cli"),
                ("refresh_token", &previous.refresh_token),
            ],
        ) {
            Ok(tokens) => return save(root, &server, tokens),
            Err(error) => eprintln!(
                "the previous sign-in could not be renewed ({error}); browser approval is required"
            ),
        }
    }
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|_| {
        refused(
            "sign_in_callback_unavailable",
            "the loopback receiver could not be opened",
        )
    })?;
    let redirect = format!(
        "http://{}/lys/sign-in",
        listener.local_addr().map_err(|_| refused(
            "sign_in_callback_unavailable",
            "the loopback address could not be read"
        ))?
    );
    let verifier = Zeroizing::new(URL_SAFE_NO_PAD.encode(rand::random::<[u8; 32]>()));
    let state = URL_SAFE_NO_PAD.encode(rand::random::<[u8; 32]>());
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let query = form(&[
        ("response_type", "code"),
        ("client_id", "lys-cli"),
        ("redirect_uri", &redirect),
        ("code_challenge", &challenge),
        ("code_challenge_method", "S256"),
        ("state", &state),
    ]);
    let query = std::str::from_utf8(&query).map_err(|_| {
        refused(
            "sign_in_unavailable",
            "the authorization query could not be encoded",
        )
    })?;
    open(&format!("{}?{query}", endpoint("authorization_endpoint")?))?;
    let code = callback(&listener, &state)?;
    let tokens = exchange(
        token_endpoint,
        &[
            ("grant_type", "authorization_code"),
            ("client_id", "lys-cli"),
            ("code", &code),
            ("code_verifier", &verifier),
            ("redirect_uri", &redirect),
        ],
    )?;
    save(root, &server, tokens)
}

fn browser(address: &str) -> CliResult<()> {
    let program = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let status = Command::new(program)
        .arg(address)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|_| {
            refused(
                "sign_in_browser_unavailable",
                "the browser could not be opened",
            )
        })?;
    if !status.success() {
        return Err(refused(
            "sign_in_browser_unavailable",
            "the browser opener refused the sign-in",
        ));
    }
    Ok(())
}

/// Sign in and print only the public person and file receipt.
///
/// # Errors
/// Returns the code flow's named refusal without exposing any credential.
pub fn run(args: &SignInArgs, json: bool) -> CliResult<()> {
    let signed = sign_in_at(&args.server, &Layout::discover()?.root, browser)?;
    let mut emitter = Emitter::new(json);
    emitter.field("signed in as", "person", signed.person);
    emitter.field(
        "pass written owner-only",
        "file",
        signed.file.display().to_string(),
    );
    emitter.finish();
    Ok(())
}
