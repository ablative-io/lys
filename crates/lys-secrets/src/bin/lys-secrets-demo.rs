//! A walkthrough of the secrets broker: a person grants an agent the use of
//! a credential, the agent uses it through a handle without ever seeing it,
//! the lease runs out, the handle is dropped, and the signed audit log's
//! lines since the key rotation are read from its tail and verified.

use std::path::Path;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use lys_core::Ed25519Identity;
use lys_secrets::{
    Broker, BrokerPaths, Holder, LocalGrants, Presentation, Secret, SecretRelation, SecretsError,
    Used, new_operation_id, request_digest,
};

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| {
            i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX)
        })
}

fn step(text: &str) {
    println!("\n\x1b[1;36m== {text}\x1b[0m");
}

fn refused(what: &str, result: Result<Used<usize>, SecretsError>) {
    match result {
        Err(error) => println!("  {what}: refused, {error}"),
        Ok(_) => println!("  {what}: UNEXPECTED, it was admitted"),
    }
}

fn run(root: &Path) -> Result<(), SecretsError> {
    let keys = root.join("keys");
    std::fs::create_dir_all(&keys).map_err(|source| SecretsError::Io {
        context: "creating the key folder".to_owned(),
        source,
    })?;
    let mut paths = BrokerPaths {
        store_dir: root.join("store"),
        log_dir: root.join("audit-log"),
        store_key: keys.join("store.key"),
        audit_key: keys.join("audit.key"),
        anchor: keys.join("audit.anchor"),
    };

    step("1. The broker starts: a sealed store, a signed audit log, two keys kept outside both");
    let grants = LocalGrants::new();
    let mut broker = Broker::create(&paths, grants, Box::new(now_ms))?;
    println!(
        "  store {} and audit log {}",
        paths.store_dir.display(),
        paths.log_dir.display()
    );

    step("2. Tom seals the GitHub token into the store");
    broker.seal(
        "github-token",
        "person:tom",
        &Secret::from_slice(b"ghp_demo_value_never_printed"),
    )?;
    for entry in broker.store().entries() {
        println!(
            "  entry {} ({:?}, owner {}, sequence {}): value sealed, not shown",
            entry.name, entry.class, entry.owner, entry.sequence
        );
    }

    step("3. The agent has its own key; no grant yet, so no handle");
    let agent_key = Ed25519Identity::load_or_generate(&keys.join("agent-noor.key"))?;
    let agent = Holder {
        identity: "agent:noor".to_owned(),
        key: agent_key.public_key_bytes(),
    };
    match broker.issue(&agent, "github-token", 2, now_ms() + 600_000) {
        Err(error) => println!("  refused, {error}"),
        Ok(_) => println!("  UNEXPECTED: issued without a grant"),
    }

    step("4. Tom grants Noor the use of the token; the broker issues a handle for 2 uses");
    broker.permissions().grant(SecretRelation {
        identity: "agent:noor".to_owned(),
        secret: "github-token".to_owned(),
        granted_by: Some("person:tom".to_owned()),
    })?;
    let issued = broker.issue(&agent, "github-token", 2, now_ms() + 600_000)?;
    println!(
        "  handle id {} (the handle itself is shown to the agent once, never kept)",
        issued.id
    );

    step("5. The agent uses the handle; the credential goes upstream, never to the agent");
    let operation = new_operation_id()?;
    let call = request_digest("GET", "/github-token/user/repos", b"")?;
    let presentation = Presentation::sign(&issued.id, &operation, now_ms(), call, &agent_key)?;
    match broker.use_handle(&issued.token, &presentation, Secret::len)? {
        Used::Forwarded { answer, uses_left } => println!(
            "  upstream received a {answer}-byte credential; the agent got the answer only; {uses_left} use left"
        ),
        Used::Retried { outcome } => println!("  retried: {outcome}"),
    }

    step("6. The same call retried is answered from the log, not sent again");
    match broker.use_handle(&issued.token, &presentation, Secret::len)? {
        Used::Retried { outcome } => {
            println!("  retry answered from the log: {outcome}; nothing forwarded, no use counted");
        }
        Used::Forwarded { .. } => println!("  UNEXPECTED: forwarded twice"),
    }

    step("7. Someone who stole the handle but not the agent's key");
    let thief_key = Ed25519Identity::load_or_generate(&keys.join("thief.key"))?;
    let forged = Presentation::sign(&issued.id, &new_operation_id()?, now_ms(), call, &thief_key)?;
    refused(
        "stolen handle",
        broker.use_handle(&issued.token, &forged, Secret::len),
    );

    step("8. The second use, then the lease is spent");
    let second = Presentation::sign(&issued.id, &new_operation_id()?, now_ms(), call, &agent_key)?;
    broker.use_handle(&issued.token, &second, Secret::len)?;
    println!("  second use admitted");
    let third = Presentation::sign(&issued.id, &new_operation_id()?, now_ms(), call, &agent_key)?;
    refused(
        "third use",
        broker.use_handle(&issued.token, &third, Secret::len),
    );

    step("9. A fresh handle, then Tom revokes the grant and drops it");
    let again = broker.issue(&agent, "github-token", 5, now_ms() + 600_000)?;
    broker.permissions().revoke("agent:noor", "github-token")?;
    let after_revoke =
        Presentation::sign(&again.id, &new_operation_id()?, now_ms(), call, &agent_key)?;
    refused(
        "use after revoke",
        broker.use_handle(&again.token, &after_revoke, Secret::len),
    );
    broker.drop_handle(&again.id)?;
    let after_drop =
        Presentation::sign(&again.id, &new_operation_id()?, now_ms(), call, &agent_key)?;
    refused(
        "use after drop",
        broker.use_handle(&again.token, &after_drop, Secret::len),
    );

    step(
        "10. The store key is rotated; the old key file is gone and every entry opens under the new one",
    );
    // Step 11 prints the lines written from here on, and reads no other.
    let from_rotation = broker.audit().len();
    let (old_id, new_id) = broker.rotate_store_key(&keys.join("store-2.key"))?;
    println!("  old key {old_id}\n  new key {new_id}");
    broker.permissions().grant(SecretRelation {
        identity: "agent:noor".to_owned(),
        secret: "github-token".to_owned(),
        granted_by: Some("person:tom".to_owned()),
    })?;
    let after_rotation = broker.issue(&agent, "github-token", 1, now_ms() + 600_000)?;
    let rotated_use = Presentation::sign(
        &after_rotation.id,
        &new_operation_id()?,
        now_ms(),
        call,
        &agent_key,
    )?;
    if let Used::Forwarded { answer, .. } =
        broker.use_handle(&after_rotation.token, &rotated_use, Secret::len)?
    {
        println!("  use after rotation: upstream received the same {answer}-byte credential");
    }

    step(
        "11. The broker restarts from disk; the audit log's lines since the rotation, each signature-checked, hold no credential and no handle",
    );
    paths.store_key = keys.join("store-2.key");
    drop(broker);
    let reopened = Broker::open(&paths, LocalGrants::new(), Box::new(now_ms))?;
    let since_rotation = reopened.audit().len().saturating_sub(from_rotation);
    for recorded in reopened.audit().window(None, since_rotation)? {
        let line = recorded.line;
        println!(
            "  {:>2} {:<6} handle {:<32} {:<12} {:<12} {}",
            recorded.index,
            line.kind.label(),
            line.handle.unwrap_or_default(),
            line.identity.unwrap_or_default(),
            line.secret.unwrap_or_default(),
            line.outcome
        );
    }
    println!(
        "\n\x1b[1;32mDone: sealed at rest, used by handle, never shown, every act signed in the log.\x1b[0m"
    );
    Ok(())
}

fn main() -> ExitCode {
    let Some(root) = std::env::args_os().nth(1) else {
        eprintln!("usage: lys-secrets-demo <empty folder>");
        return ExitCode::from(2);
    };
    match run(Path::new(&root)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{} {error}", error.name());
            ExitCode::FAILURE
        }
    }
}
