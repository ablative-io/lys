#![cfg(test)]
//! `ID001_DEPLOY` and `ID001_PIN_CLONE`, and R2's lines that need a running
//! Rauthy, `PostgreSQL` and `SpiceDB`: health output holds no secret, configure
//! run twice leaves exactly the two managed clients beside Rauthy's own and
//! resolves a lost response by read-back, and health names each unready
//! service. Container-backed: runs only on the identity leg of
//! .land/gates.sh.

pub mod identity_support;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use identity_support::compose::{self, require_runtime};
use identity_support::fixtures::{
    Deployment, TestResult, leaks, output_text, repository_root, succeeded,
};
use identity_support::server::{LossyProxy, rauthy_json};

/// The services compose.yaml declares; no other Ablative service is among them.
const DECLARED: [&str; 4] = ["postgres", "rauthy", "spicedb", "spicedb-migrate"];

fn client_ids(deployment: &Deployment) -> TestResult<BTreeSet<String>> {
    let clients = rauthy_json(deployment, "GET", "/auth/v1/clients")?;
    Ok(clients
        .as_array()
        .ok_or("the client list is not an array")?
        .iter()
        .filter_map(|client| client["id"].as_str().map(str::to_string))
        .collect())
}

fn operation_lines(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|line| {
            let (head, operation) = line.split_once(" (operation ")?;
            let (resource, _) = head.split_once(": ")?;
            Some((
                resource.to_string(),
                operation.trim_end_matches(')').to_string(),
            ))
        })
        .collect()
}

#[test]
fn id001_deploy_fresh_install_is_ready_and_configure_is_idempotent() -> TestResult {
    require_runtime()?;
    let deployment = Deployment::new("deploy", |text| text)?;
    compose::render(&deployment)?;
    let model = compose::config_json(&deployment)?;
    let services: BTreeSet<&str> = model["services"]
        .as_object()
        .ok_or("no services")?
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(services, DECLARED.into_iter().collect());

    compose::up(&deployment, &[])?;
    let health = compose::wait_ready(&deployment)?;
    let health_text = output_text(&health);
    for service in ["postgres", "rauthy", "spicedb"] {
        assert!(
            health_text.contains(&format!("{service} ready")),
            "{health_text}"
        );
    }
    for secret in deployment.secrets()? {
        assert!(
            !leaks(&health_text, &secret),
            "health output holds a secret"
        );
    }

    let builtin_before = rauthy_json(&deployment, "GET", "/auth/v1/clients/rauthy")?;
    assert_eq!(
        client_ids(&deployment)?,
        BTreeSet::from(["rauthy".to_string()])
    );

    let proxy = LossyProxy::start(deployment.rauthy_address(), "POST /auth/v1/clients ")?;
    let through_proxy = deployment.variant("lossy", |text| {
        text.replace(
            &format!(
                "admin_url = \"http://127.0.0.1:{}\"",
                deployment.ports.rauthy
            ),
            &format!("admin_url = \"http://127.0.0.1:{}\"", proxy.port),
        )
    })?;
    let lossy = deployment.lys_with("configure", &through_proxy)?;
    assert_eq!(proxy.finish()?, 1, "the proxy lost no response");
    succeeded(&lossy, "configure through the lossy proxy")?;
    assert!(
        output_text(&lossy).contains("created (read back)"),
        "{}",
        output_text(&lossy)
    );

    let first = deployment.lys("configure")?;
    succeeded(&first, "first configure")?;
    let second = deployment.lys("configure")?;
    succeeded(&second, "second configure")?;
    let (first_text, second_text) = (output_text(&first), output_text(&second));
    let first_ops = operation_lines(&first_text);
    assert_eq!(first_ops.len(), 6, "{first_text}");
    assert_eq!(first_ops, operation_lines(&second_text));
    assert_eq!(first_ops, operation_lines(&output_text(&lossy)));
    for line in second_text
        .lines()
        .filter(|line| line.contains("(operation "))
    {
        assert!(line.contains(": unchanged ("), "{line}");
    }
    for secret in deployment.secrets()? {
        assert!(!leaks(&first_text, &secret) && !leaks(&second_text, &secret));
    }

    let expected: BTreeSet<String> = ["rauthy", "lys-platform", "cambium"]
        .into_iter()
        .map(str::to_string)
        .collect();
    assert_eq!(client_ids(&deployment)?, expected);
    assert_eq!(
        rauthy_json(&deployment, "GET", "/auth/v1/clients/rauthy")?,
        builtin_before
    );

    let platform = rauthy_json(&deployment, "GET", "/auth/v1/clients/lys-platform")?;
    assert_eq!(platform["confidential"], true);
    assert_eq!(platform["challenges"], serde_json::json!(["S256"]));
    assert_eq!(
        platform["redirect_uris"],
        serde_json::json!(["http://localhost:8490/auth/callback"])
    );
    let cambium = rauthy_json(&deployment, "GET", "/auth/v1/clients/cambium")?;
    assert_eq!(cambium["access_token_alg"], "RS256");
    assert_eq!(cambium["id_token_alg"], "RS256");
    assert_eq!(
        cambium["redirect_uris"],
        serde_json::json!(["http://localhost:8400/auth/oidc/callback"])
    );
    Ok(())
}

#[test]
fn health_names_each_unready_service_in_turn() -> TestResult {
    require_runtime()?;
    let deployment = Deployment::new("health", |text| text)?;
    compose::render(&deployment)?;
    compose::up(&deployment, &[])?;
    compose::wait_ready(&deployment)?;
    let secrets = deployment.secrets()?;
    let mut named = 0;
    for service in ["postgres", "rauthy", "spicedb"] {
        compose::stop(&deployment, service)?;
        let health = deployment.lys("health")?;
        let text = output_text(&health);
        assert!(!health.status.success(), "{text}");
        let failures = text
            .lines()
            .filter(|line| line.starts_with(&format!("{service} unready: ")))
            .count();
        assert_eq!(failures, 1, "{text}");
        assert!(secrets.iter().all(|secret| !leaks(&text, secret)));
        named += failures;
        compose::start(&deployment, service)?;
        compose::wait_ready(&deployment)?;
    }
    assert_eq!(named, 3, "one named failure per declared service");
    Ok(())
}

fn git(args: &[&str]) -> TestResult<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repository_root())
        .args(args)
        .output()?;
    succeeded(&output, &format!("git {}", args.join(" ")))?;
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

#[test]
fn id001_pin_clone_vendor_rauthy_is_the_pinned_ablative_commit() -> TestResult {
    let versions: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(
        repository_root().join("deploy/identity/versions.json"),
    )?)?;
    let pinned = versions["rauthy"]["source_commit"]
        .as_str()
        .ok_or("versions.json names no Rauthy commit")?;
    let tree = git(&["ls-tree", "HEAD", "vendor/rauthy"])?;
    assert!(tree.contains(&format!("commit {pinned}")), "{tree}");
    let url = git(&["config", "-f", ".gitmodules", "submodule.vendor/rauthy.url"])?;
    assert_eq!(url, "https://github.com/ablative-io/rauthy.git");
    let toplevel = git(&["-C", "vendor/rauthy", "rev-parse", "--show-toplevel"])?;
    if !Path::new(&toplevel).ends_with("vendor/rauthy") {
        return Err(format!(
            "submodule_uninitialised: vendor/rauthy is not initialised, so git answers for \
             {toplevel}; run git submodule update --init vendor/rauthy"
        )
        .into());
    }
    let checked_out = git(&["-C", "vendor/rauthy", "rev-parse", "HEAD"])?;
    assert_eq!(
        checked_out, pinned,
        "the recursive clone checked out another commit"
    );
    let origin = git(&["-C", "vendor/rauthy", "remote", "get-url", "origin"])?;
    assert_eq!(origin, url, "the submodule was fetched from somewhere else");
    git(&[
        "-C",
        "vendor/rauthy",
        "fetch",
        "--quiet",
        "origin",
        "ablative",
    ])?;
    git(&[
        "-C",
        "vendor/rauthy",
        "merge-base",
        "--is-ancestor",
        pinned,
        "FETCH_HEAD",
    ])?;
    Ok(())
}
