#![cfg(test)]
//! Counting gates on the screen views (SECRETS-005 R6). A 200-line audit
//! window naming three secrets asks each permission once per secret and
//! reads the grants file once, where each line once asked again and read
//! the file again. What it answers is held against the filter the view used
//! to run, line by line, over the same window.

use std::sync::atomic::Ordering;

use lys_core::Ed25519Identity;
use lys_secrets::{Broker, Holder, IssuedHandle, Presentation, Relation, Secret, new_operation_id};

use crate::files::{FileGrants, Layout, now_ms};
use crate::spice::Grants;
use crate::view::audit_window;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const SECRETS: [&str; 3] = ["first", "second", "third"];
const HOLDER: &str = "agent:noor";
const VIEWER: &str = "agent:vee";
const PARTIAL: &str = "agent:pat";
const PERSON: &str = "person:tom";
const WINDOW: u64 = 200;

/// The indexes of the window's lines `identity` may read, as the view
/// found them when it asked for each line afresh.
fn asked_per_line(broker: &Broker<Grants>, identity: &str) -> TestResult<Vec<u64>> {
    Ok(broker
        .audit()
        .window(None, WINDOW)?
        .into_iter()
        .filter(|recorded| {
            recorded
                .line
                .secret
                .as_deref()
                .is_some_and(|secret| broker.discovers(identity, secret))
        })
        .map(|recorded| recorded.index)
        .collect())
}

/// The checks asked and the grants file's parses, as the view for
/// `identity` counts them, and the indexes it shows.
fn viewed(broker: &Broker<Grants>, identity: &str) -> TestResult<(u64, u64, Vec<u64>)> {
    let Grants::File(file) = broker.permissions() else {
        return Err("the broker reads no grants file".into());
    };
    file.checks.store(0, Ordering::Relaxed);
    file.parses.store(0, Ordering::Relaxed);
    let answer = audit_window(broker, identity, None)
        .map_err(|(status, text)| format!("{status}: {text}"))?
        .0;
    let shown = answer["lines"]
        .as_array()
        .ok_or("the view answered no lines")?
        .iter()
        .filter_map(|line| line["index"].as_u64())
        .collect();
    Ok((
        file.checks.load(Ordering::Relaxed),
        file.parses.load(Ordering::Relaxed),
        shown,
    ))
}

#[test]
fn a_two_hundred_line_audit_view_asks_each_permission_once() -> TestResult {
    let dir = tempfile::tempdir()?;
    let layout = Layout::new(&dir.path().join("broker"), &dir.path().join("keys"));
    layout.prepare()?;
    let grants = FileGrants::new(layout.grants());
    for secret in SECRETS {
        grants.set(Relation::Use, HOLDER, secret, Some(PERSON))?;
        grants.set(Relation::Lend, VIEWER, secret, Some(PERSON))?;
    }
    grants.set(Relation::Lend, PARTIAL, SECRETS[0], Some(PERSON))?;
    let file = Grants::File(FileGrants::new(layout.grants()));
    let mut broker = Broker::create(&layout.paths(), file, Box::new(now_ms))?;
    for secret in SECRETS {
        broker.seal(secret, PERSON, &Secret::from_slice(b"test value"))?;
    }
    let agent = Ed25519Identity::load_or_generate(&dir.path().join("agent.key"))?;
    let stranger = Ed25519Identity::load_or_generate(&dir.path().join("stranger.key"))?;
    let holder = Holder {
        identity: HOLDER.to_owned(),
        key: agent.public_key_bytes(),
    };
    let mut issued: Vec<IssuedHandle> = Vec::new();
    for secret in SECRETS {
        issued.push(broker.issue(&holder, secret, 1, now_ms() + 600_000)?);
    }
    let mut refusals = 0;
    for handle in issued.iter().cycle() {
        if broker.audit().len() >= WINDOW {
            break;
        }
        let forged = Presentation::sign(
            &handle.id,
            &new_operation_id()?,
            now_ms(),
            [7; 32],
            &stranger,
        )?;
        let refused = broker.admit_use(&handle.token, &forged, 0).err();
        let refused = refused.map(|error| error.name());
        assert_eq!(refused, Some("PresentationInvalid"));
        refusals += 1;
    }
    assert_eq!(broker.audit().len(), WINDOW);
    assert_eq!(refusals, WINDOW - 6);

    let (checks, parses, shown) = viewed(&broker, VIEWER)?;
    assert_eq!(
        checks, 9,
        "three checks for each of three secrets, where each of 200 lines asked up to three"
    );
    assert_eq!(
        parses, 1,
        "one parse of grants.json, where each check parsed it"
    );
    assert_eq!(shown.len(), 200);
    assert_eq!(shown, asked_per_line(&broker, VIEWER)?);

    let (checks, parses, shown) = viewed(&broker, PARTIAL)?;
    assert_eq!(checks, 9);
    assert_eq!(parses, 1);
    assert!(!shown.is_empty() && shown.len() < 200);
    assert_eq!(shown, asked_per_line(&broker, PARTIAL)?);
    dir.close()?;
    Ok(())
}
