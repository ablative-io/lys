//! Every refusal a loss causes says so (DIRECTORY-107 R4): once an
//! operator accepted a loss, the refusals it causes carry when Lys lost the
//! records and what to do again; an install with no loss says nothing of
//! one; and an accepted loss on the grant log tells every consumer that
//! read the earlier log that it was rolled back.

use std::sync::Arc;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use identity_contract::apps::{Auth, NOTES, ok, post, registered, root};
use lys_core::Ed25519Identity;
use lys_core::attestation::sign_attestation;
use lys_core::ca::create_certificate_request;
use lys_identity::grants::change_stream::{ChangeFrame, ChangesPage, ResetReason};
use lys_identity_server::agent_signature::{HEADER, payload};
use serde_json::{Value, json};

use super::{
    ADMINISTRATOR, BEA, Failure, Service, TestResult, accept, agent, copy_tree, grant, kept, login,
    machine, operation, put_back, refused_health, seeded,
};

/// The grant check Bea asks of `doc:<id>`.
async fn check(service: &Service, bea: &str, id: &str) -> Result<(u16, Value), Failure> {
    let question =
        json!({ "route": "api", "resource": { "kind": "doc", "id": id }, "action": "read" });
    service.post("/grants/check", Some(bea), &question).await
}

#[tokio::test]
async fn a_grant_lost_in_an_accepted_window_is_refused_not_held_saying_when() -> TestResult {
    let (mut service, seeded) = seeded().await?;
    let ada = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = service.sign_in(login(BEA)).await?;
    let (status, plain) = check(&service, &bea, "2").await?;
    assert_eq!(status, 403, "{plain}");
    assert_eq!(plain["refusal"], "NotHeld", "{plain}");
    assert!(
        !plain.to_string().contains("Lys lost"),
        "no loss, no sentence: {plain}"
    );
    assert!(plain.get("lost_at").is_none(), "{plain}");

    let copy = kept(&service, "grant-log");
    service
        .restart_between(None, |config| Ok(copy_tree(&config.grant_log_dir, &copy)?))
        .await?;
    grant(&service, &ada, &seeded.people[1].id.to_string(), "1").await?;
    let (status, held) = check(&service, &bea, "1").await?;
    assert_eq!(status, 200, "{held}");
    service
        .restart_between(None, |config| Ok(put_back(&config.grant_log_dir, &copy)?))
        .await?;
    refused_health(&service).await?;
    let mut at = String::new();
    service
        .restart_between(None, |config| {
            at = accept(config, "grant-log")?;
            Ok(())
        })
        .await?;
    let (status, lost) = check(&service, &bea, "1").await?;
    assert_eq!(status, 403, "{lost}");
    assert_eq!(
        lost["refusal"], "NotHeld",
        "the refusal keeps its name: {lost}"
    );
    let reason = lost["reason"].as_str().ok_or("no reason")?;
    assert!(
        reason.ends_with(&format!(
            "Lys lost its grant records at {at}; ask for the grant again."
        )),
        "{reason}"
    );
    assert_eq!(lost["lost_at"], at, "{lost}");
    Ok(())
}

async fn changes(service: &Service, app: &str, body: &Value) -> Result<ChangesPage, Failure> {
    let answer = ok(post(service, "/grants/changes", Auth::Bearer(app), body).await?)?;
    Ok(serde_json::from_value(answer)?)
}

fn ready(page: &ChangesPage) -> Result<u64, Failure> {
    match page.frames.last() {
        Some(ChangeFrame::Ready { revision }) => Ok(*revision),
        other => Err(format!("a page ends Ready: {other:?}").into()),
    }
}

async fn give(service: &Service, admin: &str, holder: &str, id: &str) -> Result<(), Failure> {
    ok(root(
        service,
        admin,
        holder,
        (&format!("{NOTES}.doc"), id),
        "reader",
    )
    .await?)?;
    Ok(())
}

#[tokio::test]
async fn an_accepted_grant_log_loss_answers_the_old_epoch_rolled_back_after_the_head_grows()
-> TestResult {
    let (mut service, seeded) = identity_contract::apps::seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let notes = registered(&service, &admin, NOTES).await?;
    let bea = seeded.people[1].id.to_string();
    give(&service, &admin, &bea, "1").await?;
    let first = changes(&service, &notes, &json!({"limit": 64})).await?;
    let before = ready(&first)?;
    let copy = kept(&service, "grant-log");
    service
        .restart_between(None, |config| Ok(copy_tree(&config.grant_log_dir, &copy)?))
        .await?;
    give(&service, &admin, &bea, "2").await?;
    let window = changes(
        &service,
        &notes,
        &json!({"log": first.log, "after": before, "limit": 64}),
    )
    .await?;
    let asked = ready(&window)?;
    assert!(asked > before);
    service
        .restart_between(None, |config| Ok(put_back(&config.grant_log_dir, &copy)?))
        .await?;
    refused_health(&service).await?;
    service
        .restart_between(None, |config| {
            accept(config, "grant-log")?;
            Ok(())
        })
        .await?;
    for id in ["3", "4", "5"] {
        give(&service, &admin, &bea, id).await?;
    }
    let old = changes(
        &service,
        &notes,
        &json!({"log": first.log, "after": asked, "limit": 64}),
    )
    .await?;
    assert_eq!(
        old.frames.first(),
        Some(&ChangeFrame::Reset {
            reason: ResetReason::Rollback,
            from: Some(first.log.clone())
        }),
        "{old:?}"
    );
    assert_eq!(
        old.log.identity, first.log.identity,
        "the same log, rolled back"
    );
    assert_eq!(old.log.epoch, first.log.epoch + 1, "at the next epoch");
    let new = changes(
        &service,
        &notes,
        &json!({"log": old.log, "after": before, "limit": 64}),
    )
    .await?;
    let applied = new
        .frames
        .iter()
        .filter(|frame| matches!(frame, ChangeFrame::Change { .. }))
        .count();
    assert_eq!(applied, 3, "the three appends since the cursor: {new:?}");
    assert!(ready(&new)? > asked, "the head grew past the asked cursor");
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    bytes
        .iter()
        .flat_map(|byte| {
            [
                DIGITS[usize::from(byte >> 4)],
                DIGITS[usize::from(byte & 0x0f)],
            ]
        })
        .map(char::from)
        .collect()
}

#[tokio::test]
async fn a_certificate_lost_in_an_accepted_window_is_refused_saying_when() -> TestResult {
    let (mut service, seeded) = seeded().await?;
    let ada = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = service.sign_in(login(BEA)).await?;
    let agent = agent(&seeded)?;
    let desk = machine(&service, &ada, "Laptop", &agent).await?;
    let copy = kept(&service, "certificates");
    service
        .restart_between(None, |config| {
            Ok(copy_tree(
                config
                    .certificates_dir
                    .as_deref()
                    .ok_or("no certificates")?,
                &copy,
            )?)
        })
        .await?;
    let key = Arc::new(Ed25519Identity::from_seed(&zeroize::Zeroizing::new(
        [9; 32],
    )));
    let body = json!({
        "operation": operation()?,
        "request": STANDARD.encode(create_certificate_request(&key, &agent)?),
    });
    let (status, issued) = service
        .post(&format!("/agents/{agent}/certificates"), Some(&bea), &body)
        .await?;
    assert_eq!(status, 200, "{issued}");
    service
        .restart_between(None, |config| {
            Ok(put_back(
                config
                    .certificates_dir
                    .as_deref()
                    .ok_or("no certificates")?,
                &copy,
            )?)
        })
        .await?;
    refused_health(&service).await?;
    let mut at = String::new();
    service
        .restart_between(None, |config| {
            at = accept(config, "certificates")?;
            Ok(())
        })
        .await?;
    let path = format!("/agents/{agent}/runtime/sessions/{}/reports", operation()?);
    let report = json!({
        "operation": operation()?, "state": "starting", "machine": desk,
        "what": "the agent's own session",
    })
    .to_string()
    .into_bytes();
    let signed_at = u64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis(),
    )?;
    let nonce = hex(&[7; 16]);
    let signed = payload("POST", &path, &report, signed_at, &nonce);
    let cose = sign_attestation(&signed, &key).to_cose_bytes();
    let header = format!("{agent} {signed_at} {nonce} {}", hex(&cose));
    let (status, refused) = service
        .post_signed(&path, (HEADER, &header), report)
        .await?;
    assert_eq!(status, 401, "{refused}");
    assert_eq!(refused["refusal"], "AgentSignatureRefused", "{refused}");
    let reason = refused["reason"].as_str().ok_or("no reason")?;
    assert!(reason.contains("the signature does not verify"), "{reason}");
    assert!(
        reason.ends_with(&format!(
            "Lys lost its certificate records at {at}; issue the agent's certificate again."
        )),
        "{reason}"
    );
    Ok(())
}

#[tokio::test]
async fn a_machine_lost_in_an_accepted_window_answers_lost_at_in_its_runner_view() -> TestResult {
    let (mut service, seeded) = seeded().await?;
    let ada = service.sign_in(login(ADMINISTRATOR)).await?;
    let runs = agent(&seeded)?;
    let kept_desk = machine(&service, &ada, "Kept desk", &runs).await?;
    let copy = kept(&service, "network.json");
    service
        .restart_between(None, |config| {
            std::fs::create_dir_all(copy.parent().ok_or("no folder")?)?;
            Ok(copy_tree(
                config.network_file.as_deref().ok_or("no network")?,
                &copy,
            )?)
        })
        .await?;
    let lost_desk = machine(&service, &ada, "Joined in the window", &runs).await?;
    let mut at = String::new();
    service
        .restart_between(None, |config| {
            put_back(config.network_file.as_deref().ok_or("no network")?, &copy)?;
            at = accept(config, "network")?;
            Ok(())
        })
        .await?;
    let (status, view) = service
        .get(&format!("/network/machines/{kept_desk}/runner"), Some(&ada))
        .await?;
    assert_eq!(status, 200, "{view}");
    assert_eq!(view["lost_at"], at, "{view}");
    let (status, refused) = service
        .get(&format!("/network/machines/{lost_desk}/runner"), Some(&ada))
        .await?;
    assert_eq!(refused["refusal"], "MachineUnknown", "{status} {refused}");
    assert_eq!(refused["lost_at"], at, "{refused}");
    let reason = refused["reason"].as_str().ok_or("no reason")?;
    assert!(
        reason.ends_with(&format!(
            "Lys lost its computer records at {at}; connect this computer again."
        )),
        "{reason}"
    );
    Ok(())
}
