#![cfg(test)]
//! DIRECTORY-047 R6 and R2 on a real install: `lys identity install` on a
//! scratch root, its own compose project and free ports, with the screens
//! built from this tree; then the first-run setup a person does with the
//! setup code, a password sign-in, and a refused one. Everything a person
//! reads is kept: the install's stdout and stderr, every served page and
//! asset, every answer body and every redirect. None names the issuer's
//! product, none gives the issuer's loopback port, and every redirect is on
//! Lys's origin. Then a fixture product is registered in the service
//! configuration the install wrote and signs a person in through Lys
//! (`product`). Container-backed: runs only on the identity leg.
//!
//! The directory service and the secrets broker answer on the install's
//! fixed ports, so the test refuses by name when another install holds them
//! rather than sharing one. The binaries are the workspace test build's
//! siblings, so the CLI and services come from the same build without
//! compiling them again inside a running test.

pub mod identity_support;
#[path = "identity_install/product.rs"]
mod product;

use std::fmt::Write as _;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use identity_support::compose::require_runtime;
use identity_support::fixtures::{
    TestResult, free_network, free_port, output_text, repository_root, succeeded,
};
use sha2::{Digest, Sha256};

/// The deployment configuration the install writes, as shipped.
const TEMPLATE: &str = include_str!("../src/identity/install/deployment.template.toml");
/// The directory service's fixed loopback port.
const SERVICE_PORT: u16 = 8490;
/// The secrets broker's fixed loopback port.
const BROKER_PORT: u16 = 8472;
/// Lys's origin as a browser uses it.
const ORIGIN: &str = "http://localhost:8490";
const EMAIL: &str = "ada@example.test";
const PASSWORD: &str = "Analytical-Engine-1843";

/// One installed estate, stopped and removed with its volumes when dropped.
struct Estate {
    root: tempfile::TempDir,
    project: String,
    rauthy_port: u16,
}

impl Drop for Estate {
    fn drop(&mut self) {
        for pid in ["runner.pid", "identity.pid", "secrets.pid"] {
            if let Ok(text) = std::fs::read_to_string(self.root.path().join("run").join(pid)) {
                let stopped = Command::new("kill").arg(text.trim()).status();
                if let Err(error) = stopped {
                    eprintln!("stopping {pid} failed: {error}");
                }
            }
        }
        let down = Command::new("docker")
            .arg("compose")
            .arg("-f")
            .arg(self.root.path().join("deploy/compose.yaml"))
            .arg("--env-file")
            .arg(self.root.path().join("state/compose.env"))
            .args(["-p", &self.project, "--profile", "bundled-db"])
            .args(["down", "-v", "--remove-orphans"])
            .output();
        if let Err(error) = down {
            eprintln!("teardown of {} failed: {error}", self.project);
        }
    }
}

/// Refuses by name when anything listens on `port` already.
fn port_free(port: u16, what: &str) -> TestResult {
    TcpListener::bind(("127.0.0.1", port))
        .map(drop)
        .map_err(|error| {
            format!("port_in_use: {what} port 127.0.0.1:{port} is taken ({error}); stop the install holding it").into()
        })
}

/// Reuse the workspace's compiled binaries, refusing an incomplete build.
fn binaries() -> TestResult<PathBuf> {
    let directory = Path::new(env!("CARGO_BIN_EXE_lys"))
        .parent()
        .ok_or("install_binary_directory_missing: the compiled CLI has no parent")?;
    for name in ["lys", "lys-identity-server", "lys-secrets"] {
        let path = directory.join(name);
        let metadata = std::fs::metadata(&path)
            .map_err(|error| format!("install_binary_missing: {}: {error}", path.display()))?;
        if !metadata.is_file() || metadata.permissions().mode() & 0o111 == 0 {
            return Err(format!("install_binary_not_executable: {}", path.display()).into());
        }
    }
    Ok(directory.to_path_buf())
}

/// The screens, built from this tree and packaged with the manifest the
/// install verifies, written here from the manifest's documented format.
fn screens(at: &Path) -> TestResult<PathBuf> {
    let prepared = Command::new("python3")
        .arg(repository_root().join("scripts/identity-gates/surface_fixture.py"))
        .arg(env!("CARGO_TARGET_TMPDIR"))
        .output()?;
    succeeded(&prepared, "prepare the shared screens")?;
    let surface = PathBuf::from(String::from_utf8(prepared.stdout)?.trim());
    let package = at.join("screens");
    let mut files = Vec::new();
    let dist = surface.join("dist");
    collect(&dist, &dist, &mut files)?;
    std::fs::create_dir(&package)?;
    for (relative, bytes) in &files {
        let path = package.join(relative);
        let parent = path.parent().ok_or("screen_asset_parent_missing")?;
        std::fs::create_dir_all(parent)?;
        std::fs::write(path, bytes)?;
    }
    let entries: Vec<serde_json::Value> = files
        .iter()
        .map(|(path, bytes)| {
            serde_json::json!({
                "path": path,
                "bytes": bytes.len(),
                "sha256": hex(&Sha256::digest(bytes)),
            })
        })
        .collect();
    let commit = Command::new("git")
        .current_dir(repository_root())
        .args(["rev-parse", "HEAD"])
        .output()?;
    succeeded(&commit, "read the screens' source commit")?;
    let manifest = serde_json::json!({
        "format": "lys-identity-surface/v1",
        "commit": String::from_utf8(commit.stdout)?.trim(),
        "files": entries,
    });
    std::fs::write(package.join("surface-manifest.json"), manifest.to_string())?;
    Ok(package)
}

fn collect(base: &Path, dir: &Path, files: &mut Vec<(String, Vec<u8>)>) -> TestResult {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect(base, &path, files)?;
        } else {
            let relative = path
                .strip_prefix(base)?
                .to_string_lossy()
                .replace('\\', "/");
            files.push((relative, std::fs::read(&path)?));
        }
    }
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    let digits: Vec<String> = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    digits.concat()
}

/// A PATH whose `open` and `xdg-open` find no browser, so the install
/// hands the setup code over as a headless one does.
fn headless_path(at: &Path) -> TestResult<String> {
    let bin = at.join("no-browser");
    std::fs::create_dir(&bin)?;
    for name in ["open", "xdg-open"] {
        let script = bin.join(name);
        std::fs::write(&script, "#!/bin/sh\nexit 1\n")?;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))?;
    }
    let inherited = std::env::var("PATH")?;
    Ok(format!("{}:{inherited}", bin.display()))
}

/// The estate's deployment configuration: the shipped template on free
/// ports, a free network range and a compose project of its own.
fn deployment(project: &str, rauthy_port: u16) -> TestResult<String> {
    let network = free_network()?;
    Ok(TEMPLATE
        .replace("{{admin_email_line}}", "")
        .replace("{{rauthy_port}}", &rauthy_port.to_string())
        .replace("{{service_port}}", &SERVICE_PORT.to_string())
        .replace(
            "project = \"lys-identity\"",
            &format!("project = \"{project}\""),
        )
        .replace("55432", &free_port()?.to_string())
        .replace("58051", &free_port()?.to_string())
        .replace("58443", &free_port()?.to_string())
        .replace("172.29.47.", &network))
}

/// One answer as a browser sees it.
struct Seen {
    status: u16,
    location: Option<String>,
    cookie: Option<String>,
    body: String,
}

/// One HTTP/1.1 request to the service, as a browser at Lys's origin sends it.
fn ask(method: &str, path: &str, cookie: Option<&str>, body: Option<&str>) -> TestResult<Seen> {
    let cookie = cookie.map(|cookie| format!("Cookie: {cookie}"));
    let json = body.map(|body| ("application/json", body));
    send(method, path, cookie.as_slice(), json)
}

/// One HTTP/1.1 request to the service with `headers`, each a whole header
/// line, and a body of the content type it names.
fn send(
    method: &str,
    path: &str,
    headers: &[String],
    body: Option<(&str, &str)>,
) -> TestResult<Seen> {
    let mut stream = TcpStream::connect(("127.0.0.1", SERVICE_PORT))?;
    let mut head = format!(
        "{method} {path} HTTP/1.1\r\nHost: localhost:{SERVICE_PORT}\r\nConnection: close\r\n"
    );
    for header in headers {
        write!(head, "{header}\r\n")?;
    }
    let payload = body.map_or("", |(_, payload)| payload);
    if let Some((content_type, _)) = body {
        write!(head, "Content-Type: {content_type}\r\n")?;
    }
    write!(head, "Content-Length: {}\r\n\r\n", payload.len())?;
    stream.write_all(head.as_bytes())?;
    stream.write_all(payload.as_bytes())?;
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw)?;
    let text = String::from_utf8_lossy(&raw).into_owned();
    let (head, rest) = text.split_once("\r\n\r\n").ok_or("no header terminator")?;
    let status = head.split(' ').nth(1).ok_or("no status")?.parse::<u16>()?;
    let header = |name: &str| {
        head.lines()
            .filter_map(|line| line.split_once(':'))
            .find(|(key, _)| key.trim().eq_ignore_ascii_case(name))
            .map(|(_, value)| value.trim().to_owned())
    };
    let chunked = header("transfer-encoding").is_some_and(|value| value.contains("chunked"));
    Ok(Seen {
        status,
        location: header("location"),
        cookie: header("set-cookie").and_then(|value| value.split(';').next().map(str::to_owned)),
        body: if chunked {
            dechunk(rest)?
        } else {
            rest.to_owned()
        },
    })
}

fn dechunk(mut rest: &str) -> TestResult<String> {
    let mut body = String::new();
    loop {
        let (size, tail) = rest.split_once("\r\n").ok_or("chunk size line")?;
        let size = usize::from_str_radix(size.trim(), 16)?;
        if size == 0 {
            return Ok(body);
        }
        body.push_str(tail.get(..size).ok_or("short chunk")?);
        rest = tail.get(size + 2..).ok_or("chunk terminator")?;
    }
}

/// Everything a person read, by where it came from.
#[derive(Default)]
struct Heard {
    texts: Vec<(String, String)>,
    redirects: Vec<String>,
    /// The redirects that hand a code back to a product, at its own origin.
    to_products: Vec<String>,
}

impl Heard {
    fn install(&mut self, output: &Output) {
        self.texts.push((
            "install stdout".to_owned(),
            String::from_utf8_lossy(&output.stdout).into_owned(),
        ));
        self.texts.push((
            "install stderr".to_owned(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }

    /// An answer that sends the browser back to a product, whose redirect
    /// is kept apart from those that stay on Lys's origin.
    fn handed_back(&mut self, what: &str, seen: &Seen) {
        self.texts.push((what.to_owned(), seen.body.clone()));
        if let Some(location) = &seen.location {
            self.texts
                .push((format!("{what} redirect"), location.clone()));
            self.to_products.push(location.clone());
        }
    }

    fn answer(&mut self, what: &str, seen: &Seen) {
        self.texts.push((what.to_owned(), seen.body.clone()));
        if let Some(location) = &seen.location {
            self.texts
                .push((format!("{what} redirect"), location.clone()));
            self.redirects.push(location.clone());
        }
    }
}

fn operation() -> String {
    let bytes: [u8; 16] = rand::random();
    format!("op-{}", hex(&bytes))
}

#[test]
fn a_first_install_and_a_sign_in_never_name_the_issuer() -> TestResult {
    require_runtime()?;
    port_free(SERVICE_PORT, "identity service")?;
    port_free(BROKER_PORT, "secrets broker")?;
    let bin = binaries()?;
    let work = tempfile::TempDir::new()?;
    let package = screens(work.path())?;
    let path = headless_path(work.path())?;
    let estate = Estate {
        root: tempfile::TempDir::new()?,
        project: format!("lys-identity-test-install-{}", std::process::id()),
        rauthy_port: free_port()?,
    };
    std::fs::set_permissions(estate.root.path(), std::fs::Permissions::from_mode(0o700))?;
    std::fs::write(
        estate.root.path().join("deployment.toml"),
        deployment(&estate.project, estate.rauthy_port)?,
    )?;
    let installed = Command::new(bin.join("lys"))
        .args(["identity", "install", "--root"])
        .arg(estate.root.path())
        .arg("--surface")
        .arg(&package)
        .env("PATH", &path)
        .output()?;
    succeeded(&installed, "lys identity install")?;
    let mut read = Heard::default();
    read.install(&installed);

    let code_file = estate.root.path().join("setup-code");
    let code = std::fs::read_to_string(&code_file)?;
    assert_eq!(
        std::fs::metadata(&code_file)?.permissions().mode() & 0o777,
        0o600
    );
    assert!(
        !output_text(&installed).contains(code.trim()),
        "the setup code is never printed"
    );

    let page = ask("GET", "/", None, None)?;
    assert_eq!(page.status, 200, "{}", page.body);
    assert!(page.body.contains("<title>Lys</title>"), "{}", page.body);
    let mut assets = 0;
    for (at, _) in page.body.match_indices("=\"/assets/") {
        let rest = page.body.get(at + 2..).ok_or("an asset reference")?;
        let target = rest.split('"').next().ok_or("an asset path")?;
        let asset = ask("GET", target, None, None)?;
        assert_eq!(asset.status, 200, "{target}");
        read.answer(target, &asset);
        assets += 1;
    }
    assert!(assets >= 2, "the page's script and stylesheet were read");
    read.answer("the page", &page);
    read.answer("a first visit", &ask("GET", "/api/login", None, None)?);
    let setup = ask("GET", "/setup", None, None)?;
    assert_eq!(setup.status, 200);
    read.answer("the setup page", &setup);

    let opened = serde_json::json!({ "code": code.trim() }).to_string();
    let open = ask("POST", "/api/setup/open", None, Some(&opened))?;
    assert_eq!(open.status, 200, "{}", open.body);
    read.answer("the opened setup", &open);
    let wrong = ask(
        "POST",
        "/api/setup/open",
        None,
        Some(r#"{"code":"not-the-code"}"#),
    )?;
    assert_eq!(wrong.status, 401, "{}", wrong.body);
    read.answer("a wrong setup code", &wrong);
    let weak = serde_json::json!({
        "code": code.trim(), "operation": operation(), "display_name": "Ada Lovelace",
        "email": EMAIL, "password": "short",
    });
    let refused = ask(
        "POST",
        "/api/setup/administrator",
        None,
        Some(&weak.to_string()),
    )?;
    assert!(refused.status >= 400, "{}", refused.body);
    read.answer("a weak password", &refused);
    let administrator = serde_json::json!({
        "code": code.trim(), "operation": operation(), "display_name": "Ada Lovelace",
        "email": EMAIL, "password": PASSWORD,
    });
    let made = ask(
        "POST",
        "/api/setup/administrator",
        None,
        Some(&administrator.to_string()),
    )?;
    assert_eq!(made.status, 200, "{}", made.body);
    read.answer("the finished setup", &made);

    let credentials = serde_json::json!({ "email": EMAIL, "password": PASSWORD }).to_string();
    let signed_in = ask("POST", "/api/sign-in", None, Some(&credentials))?;
    assert_eq!(signed_in.status, 200, "{}", signed_in.body);
    read.answer("a sign-in", &signed_in);
    let cookie = signed_in.cookie.ok_or("the sign-in began a session")?;
    let me = ask("GET", "/api/me", Some(&cookie), None)?;
    assert_eq!(me.status, 200, "{}", me.body);
    read.answer("the signed-in person", &me);
    let mistyped =
        serde_json::json!({ "email": EMAIL, "password": "Not-The-Password-1" }).to_string();
    let refused = ask("POST", "/api/sign-in", None, Some(&mistyped))?;
    assert_eq!(refused.status, 401, "{}", refused.body);
    read.answer("a refused sign-in", &refused);
    read.answer(
        "the sign-in providers",
        &ask("GET", "/api/sign-in/providers", None, None)?,
    );
    product::signs_in_through_lys(
        &product::Installed {
            root: estate.root.path(),
            lys: &bin.join("lys"),
            package: &package,
            path: &path,
        },
        &mut read,
    )?;

    let port = format!(":{}", estate.rauthy_port);
    let mut scanned = 0;
    for (what, text) in &read.texts {
        let lower = text.to_ascii_lowercase();
        assert!(!lower.contains("rauthy"), "{what} names the issuer: {text}");
        assert!(
            !text.contains(&port),
            "{what} gives the issuer's port: {text}"
        );
        scanned += 1;
    }
    assert!(scanned >= 27, "every answer was scanned: {scanned}");
    assert!(!read.redirects.is_empty(), "a redirect was read");
    for location in &read.redirects {
        assert!(
            location.starts_with('/') || location.starts_with(ORIGIN),
            "{location} is not on Lys's origin"
        );
    }
    assert!(!read.to_products.is_empty(), "a code was handed back");
    for location in &read.to_products {
        assert!(
            location.starts_with(product::CALLBACK),
            "{location} is not the product's registered address"
        );
    }
    drop(estate);
    Ok(())
}
