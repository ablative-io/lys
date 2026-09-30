#![cfg(test)]
//! DIRECTORY-051 R6: a launch carries its agent's policy and the digest the
//! server admitted it under, inside the signed start act. The runner encodes
//! the policy again, and starts the session only when the digest it computes
//! is the one admitted; a policy changed on the way, or a digest that names
//! another policy, is refused by name and nothing runs.

use std::collections::BTreeMap;
use std::error::Error;
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_runner::admitted::{Admitted, ENCODING, digest};
use lys_runner::judge::{Authority, Policy, Rule, RuleKind};
use lys_runner::protocol::hex;
use lys_runner::{Act, Answer, Client, Launch, Options, Runner, RunnerError};
use sha2::{Digest, Sha256};

type TestResult = Result<(), Box<dyn Error>>;

fn policy(target: &str) -> Policy {
    Policy {
        version: 1,
        agent: "agent-one".to_owned(),
        rules: vec![Rule {
            id: "no-secrets".to_owned(),
            tool: "Read".to_owned(),
            kind: RuleKind::PathPrefix,
            target: Some(target.to_owned()),
            authority: Authority::Hard,
        }],
    }
}

fn launch(session: &str, admitted: Option<Admitted>) -> Launch {
    Launch {
        session: session.to_owned(),
        program: "/bin/sh".to_owned(),
        arguments: vec!["-c".to_owned(), "cat".to_owned()],
        directory: "/".to_owned(),
        environment: BTreeMap::new(),
        config: None,
        columns: 80,
        rows: 24,
        rotation: None,
        policy: admitted.map(Box::new),
    }
}

fn start(client: &Client, launch: Launch) -> Result<Result<Answer, String>, RunnerError> {
    match client.ask(&Act::Start {
        launch: Box::new(launch),
    }) {
        Ok(answer) => Ok(Ok(answer)),
        Err(RunnerError::Refused { refusal, .. }) => Ok(Err(refusal)),
        Err(other) => Err(other),
    }
}

#[test]
fn the_digest_is_the_hash_of_the_domain_line_and_the_policy() -> TestResult {
    let policy = policy("/secrets");
    let mut encoded = format!("{ENCODING}\n").into_bytes();
    encoded.extend(serde_json::to_vec(&policy)?);
    let expected = hex(&Sha256::digest(&encoded));
    assert_eq!(digest(&policy)?, expected);
    assert_eq!(Admitted::of(policy)?.digest, expected);
    Ok(())
}

#[test]
fn only_the_admitted_policy_starts_a_session() -> TestResult {
    let dir = tempfile::tempdir()?;
    let key = Arc::new(Ed25519Identity::load_or_generate(
        &dir.path().join("server.key"),
    )?);
    let options = Options {
        socket: dir.path().join("runner.sock"),
        state: dir.path().join("state"),
        server_key: key.public_key_bytes(),
        scrollback: 1 << 16,
    };
    let serving = Runner::open(&options)?.spawn();
    let client = Client::new(options.socket, Arc::clone(&key));

    let admitted = Admitted::of(policy("/secrets"))?;
    match start(&client, launch("admitted", Some(admitted.clone())))? {
        Ok(Answer::Started { .. }) => {}
        other => return Err(format!("the admitted policy answered {other:?}").into()),
    }
    let Answer::Status { status } = client.ask(&Act::Status {
        session: Some("admitted".to_owned()),
    })?
    else {
        return Err("the runner answered no status".into());
    };
    assert_eq!(
        status.sessions[0].policy,
        Some(admitted.judged_under()),
        "{status:?}"
    );

    let changed = Admitted {
        policy: policy("/elsewhere"),
        ..admitted.clone()
    };
    assert_eq!(
        start(&client, launch("changed", Some(changed)))?,
        Err("policy_digest_mismatch".to_owned())
    );

    let other_digest = Admitted {
        digest: digest(&policy("/elsewhere"))?,
        ..admitted
    };
    assert_eq!(
        start(&client, launch("other-digest", Some(other_digest)))?,
        Err("policy_digest_mismatch".to_owned())
    );

    let unversioned = Admitted::of(Policy {
        version: 0,
        ..policy("/secrets")
    })?;
    assert_eq!(
        start(&client, launch("unversioned", Some(unversioned)))?,
        Err("policy_invalid".to_owned())
    );

    for refused in ["changed", "other-digest", "unversioned"] {
        match client.ask(&Act::Status {
            session: Some(refused.to_owned()),
        }) {
            Err(RunnerError::Refused { refusal, .. }) if refusal == "session_unknown" => {}
            other => return Err(format!("{refused} is held: {other:?}").into()),
        }
    }
    serving.stop()?;
    Ok(())
}
