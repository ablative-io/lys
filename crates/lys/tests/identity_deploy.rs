//! ID001_DEPLOY and ID001_PIN_CLONE, and the three `lys identity` lines that
//! need a running Rauthy, `PostgreSQL` and `SpiceDB`: health output carries no
//! secret, configure run twice is idempotent (and settles a lost response by
//! reading back), and health names each unready service.
//!
//! Container-backed: declared `test = false`, run only on the identity leg of
//! `.land/gates.sh`.

pub mod identity_support;

use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use identity_support::fixtures::{edit, output_text, repo_root};
use identity_support::{Stack, TestResult};
use serde_json::Value;

/// The services the compose file defines: the three dependency processes
/// and SpiceDB's one-shot migration. Nothing else runs.
const COMPOSE_SERVICES: [&str; 4] = ["postgres", "rauthy", "spicedb", "spicedb-migrate"];

/// Parse configure's operation lines: `outcome kind resource id`.
fn operations(stdout: &str) -> Vec<(String, String, String, String)> {
    stdout
        .lines()
        .filter_map(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();
            match fields.as_slice() {
                [outcome, kind, resource, id] if id.starts_with("op-") => Some((
                    (*outcome).to_string(),
                    (*kind).to_string(),
                    (*resource).to_string(),
                    (*id).to_string(),
                )),
                _ => None,
            }
        })
        .collect()
}

/// A proxy in front of Rauthy that forwards every request and, for the first
/// `POST /auth/v1/clients`, closes the connection after Rauthy has answered
/// instead of relaying the answer: a transport failure after the request.
fn lossy_proxy(upstream: String, dropped: Arc<AtomicUsize>) -> TestResult<String> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let address = listener.local_addr()?.to_string();
    std::thread::spawn(move || {
        for mut client in listener.incoming().flatten() {
            let request = read_request(&mut client);
            let Ok(mut rauthy) = TcpStream::connect(&upstream) else {
                continue;
            };
            let mut response = Vec::new();
            if rauthy.write_all(&request).is_err() || rauthy.read_to_end(&mut response).is_err() {
                continue;
            }
            let is_create = request.starts_with(b"POST /auth/v1/clients ");
            if is_create && dropped.fetch_add(1, Ordering::SeqCst) == 0 {
                continue;
            }
            if client.write_all(&response).is_err() {
                eprintln!("proxy: the client went away before the response");
            }
        }
    });
    Ok(address)
}

fn read_request(stream: &mut TcpStream) -> Vec<u8> {
    let mut request = Vec::new();
    let mut buffer = [0_u8; 4096];
    loop {
        if let Some(end) = request.windows(4).position(|window| window == b"\r\n\r\n") {
            let head = String::from_utf8_lossy(&request[..end]).to_ascii_lowercase();
            let length = head
                .lines()
                .find_map(|line| line.strip_prefix("content-length:"))
                .and_then(|value| value.trim().parse::<usize>().ok())
                .unwrap_or(0);
            if request.len() >= end + 4 + length {
                return request;
            }
        }
        match stream.read(&mut buffer) {
            Ok(0) | Err(_) => return request,
            Ok(read) => request.extend_from_slice(&buffer[..read]),
        }
    }
}

fn client_ids(stack: &Stack) -> TestResult<Vec<String>> {
    let reply = stack.rauthy_admin("GET", "/auth/v1/clients")?;
    let clients: Vec<Value> = serde_json::from_str(&reply.body)?;
    let mut ids: Vec<String> = clients
        .iter()
        .filter_map(|client| client["id"].as_str().map(str::to_string))
        .collect();
    ids.sort();
    Ok(ids)
}

#[test]
fn deploy_becomes_ready_and_configure_twice_is_idempotent() -> TestResult {
    // ID001_DEPLOY: a fresh venue directory, nothing else running.
    let stack = Stack::up("deploy")?;
    let services: BTreeSet<String> = stack
        .compose
        .run_ok(&["config", "--services"])?
        .lines()
        .map(str::to_string)
        .collect();
    let expected: BTreeSet<String> = COMPOSE_SERVICES.iter().map(|name| (*name).to_string()).collect();
    assert_eq!(services, expected);

    let builtin_before = stack.rauthy_admin("GET", "/auth/v1/clients/rauthy")?;
    assert_eq!(builtin_before.status, 200);

    // First run through a proxy that loses the response to the first create.
    let dropped = Arc::new(AtomicUsize::new(0));
    let proxy = lossy_proxy(stack.fixture.rauthy(), Arc::clone(&dropped))?;
    let proxied = stack.fixture.dir.path().join("proxied.toml");
    let text = std::fs::read_to_string(&stack.fixture.config)?;
    let rauthy_publish = format!("publish = \"{}\"", stack.fixture.rauthy());
    std::fs::write(&proxied, edit(&text, &rauthy_publish, &format!("publish = \"{proxy}\""))?)?;
    let themes = repo_root().join("deploy/identity/rauthy-themes.json");
    let first = stack.fixture.lys(&[
        "identity",
        "configure",
        "--config",
        &proxied.to_string_lossy(),
        "--themes",
        &themes.to_string_lossy(),
    ])?;
    assert!(first.status.success(), "{}", output_text(&first));
    assert!(dropped.load(Ordering::SeqCst) >= 1, "the proxy never lost a response");
    let first_ops = operations(&String::from_utf8_lossy(&first.stdout));
    assert_eq!(first_ops.len(), 6, "{first_ops:?}");
    let (outcome, kind, resource, _) = &first_ops[0];
    assert_eq!(
        (outcome.as_str(), kind.as_str(), resource.as_str()),
        ("recovered", "client", "platform"),
        "the lost create was not settled by read-back: {first_ops:?}"
    );

    // Second run, direct: every operation unchanged, under the same ids.
    let second_ops = operations(&stack.configure()?);
    assert_eq!(second_ops.len(), 6, "{second_ops:?}");
    let mut compared = 0;
    for (first_op, second_op) in first_ops.iter().zip(&second_ops) {
        assert_eq!(first_op.1, second_op.1);
        assert_eq!(first_op.2, second_op.2);
        assert_eq!(first_op.3, second_op.3, "operation id of {} {} changed", first_op.1, first_op.2);
        let expected = if second_op.1 == "client_secret" { "reused" } else { "unchanged" };
        assert_eq!(second_op.0, expected, "{second_op:?}");
        compared += 1;
    }
    assert_eq!(compared, 6);

    // Exactly the built-in client and the two managed ones, each once.
    assert_eq!(client_ids(&stack)?, ["cambium", "platform", "rauthy"]);
    let builtin_after = stack.rauthy_admin("GET", "/auth/v1/clients/rauthy")?;
    let before: Value = serde_json::from_str(&builtin_before.body)?;
    let after: Value = serde_json::from_str(&builtin_after.body)?;
    assert_eq!(before, after, "the built-in rauthy client changed");
    for (id, redirect, challenges) in [
        ("platform", "http://localhost:3000/auth/callback", Value::from(vec!["S256"])),
        ("cambium", "http://localhost:4000/auth/callback", Value::Null),
    ] {
        let client: Value = serde_json::from_str(&stack.rauthy_admin("GET", &format!("/auth/v1/clients/{id}"))?.body)?;
        assert_eq!(client["redirect_uris"], Value::from(vec![redirect]), "{id}");
        assert_eq!(client["access_token_alg"], "RS256", "{id}");
        assert_eq!(client["id_token_alg"], "RS256", "{id}");
        assert_eq!(client["confidential"], true, "{id}");
        assert_eq!(client.get("challenges").cloned().unwrap_or(Value::Null), challenges, "{id}");
    }

    // Health of the configured deployment, captured: ready, and no secret.
    let health = stack.fixture.identity("health", &[])?;
    let captured = output_text(&health);
    assert!(health.status.success(), "{captured}");
    assert_eq!(captured.matches(": ready at ").count(), 3, "{captured}");
    assert!(!stack.fixture.leaks(&captured)?, "health output carried a generated secret");
    assert!(!stack.fixture.leaks(&output_text(&first))?, "configure output carried a secret");
    Ok(())
}

#[test]
fn health_names_each_unready_service_in_turn() -> TestResult {
    let stack = Stack::up("health")?;
    let ready = stack.fixture.identity("health", &[])?;
    let declared = output_text(&ready).matches("service ").count();
    assert!(ready.status.success(), "{}", output_text(&ready));
    assert_eq!(declared, 3);
    let cases = [("postgres", "database"), ("rauthy", "rauthy"), ("spicedb", "spicedb")];
    let mut named = 0;
    for (container, service) in cases {
        stack.compose.run_ok(&["stop", container])?;
        let unready = stack.fixture.identity("health", &["--timeout-secs", "2"])?;
        let text = output_text(&unready);
        assert!(!unready.status.success(), "{service} stopped but health passed: {text}");
        assert!(text.contains(&format!("service {service}: unready")), "{text}");
        assert!(text.contains("services_unready:"), "{text}");
        assert!(!stack.fixture.leaks(&text)?);
        named += 1;
        stack.compose.run_ok(&["start", container])?;
        stack.wait_ready()?;
    }
    assert_eq!(named, declared, "one named failure per declared service");
    Ok(())
}

/// ID001_PIN_CLONE: fetch the exact pushed Lys commit fresh, with its
/// submodules, and verify the Rauthy pin is the expected commit and is on the
/// fork's ablative branch, without reading this checkout's submodule copy.
#[test]
fn pin_clone_verifies_the_rauthy_commit_on_ablative() -> TestResult {
    let git = |dir: &std::path::Path, args: &[&str]| -> TestResult<String> {
        let output = Command::new("git").arg("-C").arg(dir).args(args).output()?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(format!("git {} failed: {}", args.join(" "), output_text(&output)).into())
        }
    };
    let root = repo_root();
    let head = git(root.as_path(), &["rev-parse", "HEAD"])?;
    let origin = git(root.as_path(), &["remote", "get-url", "origin"])?;
    let versions: Value = serde_json::from_str(&std::fs::read_to_string(root.join("deploy/identity/versions.json"))?)?;
    let expected = versions["rauthy"]["commit"].as_str().ok_or("versions.json names no rauthy commit")?;
    let fork = versions["rauthy"]["fork"].as_str().ok_or("versions.json names no fork")?;

    let fresh = tempfile::tempdir()?;
    let lys = fresh.path().join("lys");
    std::fs::create_dir_all(&lys)?;
    git(lys.as_path(), &["init", "-q"])?;
    git(lys.as_path(), &["fetch", "-q", "--depth", "1", origin.as_str(), head.as_str()])?;
    git(lys.as_path(), &["checkout", "-q", "FETCH_HEAD"])?;
    git(lys.as_path(), &["submodule", "update", "-q", "--init", "--recursive", "--depth", "1"])?;
    let pinned = git(lys.join("vendor/rauthy").as_path(), &["rev-parse", "HEAD"])?;
    assert_eq!(pinned, expected, "the fresh clone's vendor/rauthy is not the recorded pin");

    let fork_dir = fresh.path().join("rauthy");
    git(fresh.path(), &["clone", "-q", "--filter=tree:0", "--no-checkout", "--single-branch", "--branch", "ablative", fork, "rauthy"])?;
    git(fork_dir.as_path(), &["merge-base", "--is-ancestor", expected, "origin/ablative"])?;
    println!("ID001_PIN_CLONE: {head} pins vendor/rauthy {pinned}, on {fork} ablative");
    Ok(())
}

#[test]
fn no_other_service_is_a_runtime_dependency() -> TestResult {
    let compose = std::fs::read_to_string(repo_root().join("deploy/identity/compose.yaml"))?;
    let images: Vec<String> = compose
        .lines()
        .filter(|line| line.trim_start().starts_with("image:"))
        .map(str::to_ascii_lowercase)
        .collect();
    assert_eq!(images.len(), 4, "{images:?}");
    for absent in ["cambium", "manifold", "argus", "aion"] {
        assert!(images.iter().all(|line| !line.contains(absent)), "{absent} is an image");
    }
    Ok(())
}
