//! `ID001_DEPLOY` and `ID001_PIN_CLONE`, with the three DIRECTORY-002 R2 lines
//! that need a running Rauthy, `PostgreSQL` and `SpiceDB`: the captured health
//! output, configure run twice, and health per service.
//!
//! Container-backed: declared `test = false`, run only on the identity leg of
//! .land/gates.sh, and refused as `container_runtime_missing` without a
//! runtime. Test identities only (CN2).

mod identity_support;

use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use identity_support::fixtures::{Failure, Venue, repo_root, text};
use identity_support::server::Stack;
use serde_json::Value;

/// The services health declares, each with the failure its absence names.
const SERVICES: [(&str, &str); 3] = [
    ("postgres", "database_unreachable"),
    ("rauthy", "rauthy_unreachable"),
    ("spicedb", "spicedb_unreachable"),
];

/// `ID001_DEPLOY`: from a fresh venue the dependencies become ready with no
/// other Ablative service running; configure is idempotent and resolves a
/// lost answer by read-back; health prints no secret and names each
/// unready service.
#[test]
fn id001_deploy_ready_configured_and_named() -> Result<(), Failure> {
    let venue = Venue::fresh("deploy", None)?;
    let stack = Stack::up(&venue)?;
    let ready = stack.wait_ready()?;
    assert_eq!(ready["ready"], Value::Bool(true));

    // Three dependency processes and nothing else: no Cambium, Manifold,
    // Argus or Aion service is declared or running.
    let config = stack.compose().config()?;
    let declared: BTreeSet<&str> = config["services"]
        .as_object()
        .ok_or("compose config has no services")?
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        declared,
        BTreeSet::from(["postgres", "rauthy", "spicedb", "spicedb-migrate"])
    );
    let running = text(
        &stack
            .compose()
            .ok(&["ps", "--services", "--status", "running"])?
            .stdout,
    );
    let running: BTreeSet<&str> = running.lines().collect();
    assert_eq!(running, BTreeSet::from(["postgres", "rauthy", "spicedb"]));

    // Configure through a proxy that forwards the first create and drops
    // its answer: the outcome is uncertain and must be resolved by reading
    // the client back, never by creating it twice.
    let proxy = CuttingProxy::start(venue.ports().rauthy)?;
    venue.set(
        "rauthy",
        "admin_origin",
        &format!("\"http://127.0.0.1:{}\"", proxy.port),
    )?;
    let first = stack.configure()?;
    assert_eq!(
        proxy.cut.load(Ordering::SeqCst),
        1,
        "the create's answer was not cut"
    );
    venue.set(
        "rauthy",
        "admin_origin",
        &format!("\"http://127.0.0.1:{}\"", venue.ports().rauthy),
    )?;
    assert_eq!(
        outcome(&first, "client/platform")?,
        "created_resolved_by_read_back"
    );
    assert_eq!(outcome(&first, "client/cambium")?, "created");

    let second = stack.configure()?;
    let mut unchanged = 0;
    for resource in [
        "client/platform",
        "client/cambium",
        "theme/platform",
        "theme/cambium",
    ] {
        assert_eq!(outcome(&second, resource)?, "unchanged", "{resource}");
        assert_eq!(
            operation_id(&first, resource)?,
            operation_id(&second, resource)?,
            "{resource}"
        );
        unchanged += 1;
    }
    assert_eq!(unchanged, 4);
    check_clients(&first, &second)?;

    // The captured health output of the configured deployment carries no
    // byte of a generated secret; `Venue::lys` refuses any run that does,
    // and this counts that there were secrets to find.
    let health = stack.health()?;
    assert!(health.status.success(), "{}", text(&health.stderr));
    let printed = text(&health.stdout) + &text(&health.stderr);
    let secrets = venue.secrets()?;
    assert_eq!(secrets.len(), 9);
    assert!(
        secrets
            .iter()
            .all(|secret| !printed.contains(secret.as_str()))
    );

    // Each declared service made unavailable in turn is named by health.
    let mut named = 0;
    for (service, failure) in SERVICES {
        stack.compose().ok(&["stop", service])?;
        let output = stack.health()?;
        let stderr = text(&output.stderr);
        assert!(!output.status.success(), "{service} stopped, health passed");
        assert!(stderr.contains(failure), "{service} stopped: {stderr}");
        named += 1;
        stack.compose().ok(&["start", service])?;
        stack.wait_ready()?;
    }
    assert_eq!(named, SERVICES.len());
    assert_eq!(named, ready["checks"].as_array().map_or(0, Vec::len));
    Ok(())
}

/// `ID001_PIN_CLONE`: this checkout carries the vendor/rauthy submodule at the
/// commit deploy/identity/versions.json records, on the fork's `ablative`
/// branch, as a clone of its own rather than a borrowed reading copy.
#[test]
fn id001_pin_clone() -> Result<(), Failure> {
    let root = repo_root();
    let versions: Value = serde_json::from_str(&std::fs::read_to_string(
        root.join("deploy/identity/versions.json"),
    )?)?;
    let expected = versions["rauthy"]["source_commit"]
        .as_str()
        .ok_or("versions.json has no rauthy.source_commit")?;
    let gitlink = git(&root, &["ls-tree", "HEAD", "vendor/rauthy"])?;
    assert!(
        gitlink.contains(expected),
        "the pinned gitlink is {gitlink}"
    );
    let vendor = root.join("vendor/rauthy");
    let head = git(&vendor, &["rev-parse", "HEAD"])?;
    assert_eq!(
        head.trim(),
        expected,
        "vendor/rauthy is not checked out at the pin"
    );
    let modules = std::fs::read_to_string(root.join(".gitmodules"))?;
    assert!(modules.contains("url = https://github.com/ablative-io/rauthy.git"));
    assert!(modules.contains("branch = ablative"));
    let remote = git(
        &vendor,
        &[
            "ls-remote",
            "https://github.com/ablative-io/rauthy.git",
            "refs/heads/ablative",
        ],
    )?;
    let tip = remote
        .split_whitespace()
        .next()
        .ok_or("the fork has no ablative branch")?;
    git(
        &vendor,
        &[
            "fetch",
            "--quiet",
            "https://github.com/ablative-io/rauthy.git",
            "refs/heads/ablative",
        ],
    )?;
    git(&vendor, &["merge-base", "--is-ancestor", expected, tip])?;
    let alternates = git(
        &vendor,
        &["rev-parse", "--git-path", "objects/info/alternates"],
    )?;
    assert!(
        !vendor.join(alternates.trim()).exists(),
        "vendor/rauthy borrows objects from another clone"
    );
    Ok(())
}

fn git(dir: &std::path::Path, args: &[&str]) -> Result<String, Failure> {
    let output = Command::new("git").current_dir(dir).args(args).output()?;
    if output.status.success() {
        Ok(text(&output.stdout))
    } else {
        Err(format!("git {} failed: {}", args.join(" "), text(&output.stderr)).into())
    }
}

fn operation<'r>(report: &'r Value, resource: &str) -> Result<&'r Value, Failure> {
    report["operations"]
        .as_array()
        .and_then(|operations| operations.iter().find(|op| op["resource"] == resource))
        .ok_or_else(|| format!("configure reported no {resource}").into())
}

fn outcome<'r>(report: &'r Value, resource: &str) -> Result<&'r str, Failure> {
    operation(report, resource)?["outcome"]
        .as_str()
        .ok_or_else(|| format!("{resource} has no outcome").into())
}

fn operation_id<'r>(report: &'r Value, resource: &str) -> Result<&'r str, Failure> {
    operation(report, resource)?["operation_id"]
        .as_str()
        .ok_or_else(|| format!("{resource} has no operation id").into())
}

/// Exactly the built-in rauthy client and the two managed clients exist,
/// each once; the built-in one is as Rauthy's migration left it; the
/// managed ones hold the configured values.
fn check_clients(first: &Value, second: &Value) -> Result<(), Failure> {
    let clients = second["clients"].as_array().ok_or("no clients")?;
    let ids: Vec<&str> = clients
        .iter()
        .filter_map(|client| client["id"].as_str())
        .collect();
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    assert_eq!(
        sorted,
        ["cambium", "platform", "rauthy"],
        "clients: {ids:?}"
    );
    let find = |list: &Value, id: &str| {
        list.as_array()
            .and_then(|all| all.iter().find(|client| client["id"] == id).cloned())
            .ok_or(format!("no client {id}"))
    };
    let before = find(&first["clients_before"], "rauthy")?;
    assert_eq!(
        find(&second["clients"], "rauthy")?,
        before,
        "the built-in client changed"
    );
    assert_eq!(first["clients_before"].as_array().map_or(0, Vec::len), 1);
    let platform = find(&second["clients"], "platform")?;
    assert_eq!(
        platform["redirect_uris"],
        serde_json::json!(["http://localhost:8081/auth/callback"])
    );
    assert_eq!(platform["access_token_alg"], "RS256");
    assert_eq!(platform["id_token_alg"], "RS256");
    assert_eq!(platform["challenges"], serde_json::json!(["S256"]));
    assert_eq!(platform["confidential"], true);
    let cambium = find(&second["clients"], "cambium")?;
    assert_eq!(
        cambium["redirect_uris"],
        serde_json::json!(["http://localhost:8082/auth/callback"])
    );
    assert_eq!(cambium["access_token_alg"], "RS256");
    assert_eq!(cambium["challenges"], Value::Null);
    assert_eq!(cambium["confidential"], true);
    Ok(())
}

/// A TCP proxy to Rauthy that forwards every request and drops the answer
/// to the first client create, so the caller sees a request that was sent
/// and never answered.
struct CuttingProxy {
    port: u16,
    cut: Arc<AtomicUsize>,
}

impl CuttingProxy {
    fn start(upstream: u16) -> Result<Self, Failure> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();
        let cut = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&cut);
        std::thread::spawn(move || {
            for client in listener.incoming().flatten() {
                if let Err(error) = relay(client, upstream, &counter) {
                    eprintln!("proxy relay: {error}");
                }
            }
        });
        Ok(Self { port, cut })
    }
}

fn relay(mut client: TcpStream, upstream: u16, cut: &AtomicUsize) -> Result<(), Failure> {
    let request = read_request(&mut client)?;
    let mut server = TcpStream::connect(("127.0.0.1", upstream))?;
    server.write_all(&request)?;
    let mut response = Vec::new();
    server.read_to_end(&mut response)?;
    let is_create = request.starts_with(b"POST /auth/v1/clients ");
    if is_create
        && cut
            .compare_exchange(0, 1, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    {
        // Rauthy has acted on the request; its answer is dropped here.
        return Ok(());
    }
    client.write_all(&response)?;
    Ok(())
}

fn read_request(client: &mut TcpStream) -> Result<Vec<u8>, Failure> {
    let mut request = Vec::new();
    let mut buffer = [0_u8; 4096];
    loop {
        let read = client.read(&mut buffer)?;
        if read == 0 {
            return Ok(request);
        }
        request.extend_from_slice(&buffer[..read]);
        let Some(end) = request.windows(4).position(|window| window == b"\r\n\r\n") else {
            continue;
        };
        let head = text(&request[..end]).to_ascii_lowercase();
        let length = head
            .lines()
            .find_map(|line| line.strip_prefix("content-length:"))
            .map_or(Ok(0), |value| value.trim().parse::<usize>())?;
        if request.len() >= end + 4 + length {
            return Ok(request);
        }
    }
}
