#![cfg(test)]
//! The grant change stream over HTTP (DIRECTORY-089 R1): an app reads the
//! actual signed grant log after its cursor, applies it to the consumer
//! index (ACCESS-006 R2), and is admitted only once Ready covers the
//! revision it needs. Counts are taken at the applied frame, never at a
//! request sent.

use std::error::Error;
use std::num::NonZeroUsize;

use identity_contract::apps::{
    Auth, FILES, NOTES, TestResult, login, ok, op, post, refused, registered, root,
};
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::grants::change_stream::{ChangeFrame, ChangeKind, ChangesPage, ResetReason};
use lys_identity::grants::channel_membership_index::{
    Applied, DEPENDENCY_REVOKED, MembershipIndex, STREAM_BEHIND,
};
use serde_json::{Value, json};

struct World {
    service: Service,
    admin: String,
    notes: String,
    files: String,
    bea: String,
}

async fn world() -> Result<World, Box<dyn Error>> {
    let (service, seeded) = identity_contract::apps::seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let notes = registered(&service, &admin, NOTES).await?;
    let files = registered(&service, &admin, FILES).await?;
    Ok(World {
        service,
        admin,
        notes,
        files,
        bea: seeded.people[1].id.to_string(),
    })
}

async fn read(
    world: &World,
    credential: &str,
    body: &Value,
) -> Result<ChangesPage, Box<dyn Error>> {
    let answer = ok(post(
        &world.service,
        "/grants/changes",
        Auth::Bearer(credential),
        body,
    )
    .await?)?;
    Ok(serde_json::from_value(answer)?)
}

async fn grant(
    world: &World,
    kind: &str,
    id: &str,
    relation: &str,
) -> Result<String, Box<dyn Error>> {
    let issued = ok(root(
        &world.service,
        &world.admin,
        &world.bea,
        (kind, id),
        relation,
    )
    .await?)?;
    Ok(issued["grant"]
        .as_str()
        .ok_or("the issue names its grant")?
        .to_owned())
}

async fn revoke(world: &World, grant: &str) -> Result<u64, Box<dyn Error>> {
    let body = json!({"operation": op()?, "route": "api", "reason": "left the ward"});
    let revoked = ok(post(
        &world.service,
        &format!("/grants/{grant}/revoke"),
        Auth::Cookie(&world.admin),
        &body,
    )
    .await?)?;
    Ok(revoked["receipt"]["revision"]
        .as_u64()
        .ok_or("the revoke names its revision")?)
}

/// The service key the stream's events are signed with, as a consumer is given it.
fn service_key(world: &World) -> Result<[u8; 32], Box<dyn Error>> {
    let key =
        lys_identity::signer::load_service_key(&world.service.dir.path().join("service.key"))?;
    Ok(key.public_key_bytes())
}

fn revocations(page: &ChangesPage) -> usize {
    page.frames
        .iter()
        .filter(|frame| {
            matches!(
                frame,
                ChangeFrame::Change {
                    change: ChangeKind::Revoke,
                    ..
                }
            )
        })
        .count()
}

/// d089_r1_revision_and_ready_counts: issue at r, revoke at r+1; the app
/// sees one revocation and one Ready covering r+1, its index refuses
/// admission from r once r+1 is required, and an independent grant stays
/// admitted.
#[tokio::test]
async fn one_revocation_one_ready_and_no_admission_from_the_older_revision() -> TestResult {
    let world = world().await?;
    let doc = format!("{NOTES}.doc");
    let revoked = grant(&world, &doc, "1", "reader").await?;
    let independent = grant(&world, &doc, "2", "reader").await?;

    let first = read(&world, &world.notes, &json!({"limit": 16})).await?;
    let ChangeFrame::Baseline {
        revision: r,
        revoked: none,
    } = &first.frames[0]
    else {
        return Err(format!("a first read starts with a baseline: {first:?}").into());
    };
    assert!(none.is_empty());
    assert_eq!(
        first.frames.last(),
        Some(&ChangeFrame::Ready { revision: *r })
    );
    let mut index = MembershipIndex::new(
        first.log.clone(),
        service_key(&world)?,
        NonZeroUsize::new(16).ok_or("a window")?,
    );
    index.apply_page(&first.log, &first.frames)?;
    index.admit(*r, std::slice::from_ref(&revoked))?;

    let fence = revoke(&world, &revoked).await?;
    assert_eq!(fence, r + 1);
    assert_eq!(
        index
            .admit(fence, std::slice::from_ref(&revoked))
            .map_err(|refusal| refusal.name),
        Err(STREAM_BEHIND),
        "no admission from the cache at r once r+1 is required"
    );
    let next = read(
        &world,
        &world.notes,
        &json!({"log": first.log, "after": r, "limit": 16}),
    )
    .await?;
    assert_eq!(revocations(&next), 1, "{next:?}");
    let applied = index.apply_page(&next.log, &next.frames)?;
    assert_eq!(
        applied
            .iter()
            .filter(|done| matches!(done, Applied::Change { .. }))
            .count(),
        1
    );
    assert_eq!(applied.last(), Some(&Applied::Ready { revision: fence }));
    assert_eq!(
        index
            .admit(fence, std::slice::from_ref(&revoked))
            .map_err(|refusal| refusal.name),
        Err(DEPENDENCY_REVOKED)
    );
    index.admit(fence, std::slice::from_ref(&independent))?;

    // Replay strictly after the cursor: the same read again applies nothing new.
    let again = read(
        &world,
        &world.notes,
        &json!({"log": first.log, "after": fence, "limit": 16}),
    )
    .await?;
    assert_eq!(again.frames, vec![ChangeFrame::Ready { revision: fence }]);
    Ok(())
}

/// An app sees only changes on its own kinds; the rest are watermarks.
#[tokio::test]
async fn another_apps_changes_are_watermarks() -> TestResult {
    let world = world().await?;
    let files_doc = format!("{FILES}.doc");
    let theirs = grant(&world, &files_doc, "9", "reader").await?;
    let head = revoke(&world, &theirs).await?;
    let page = read(&world, &world.notes, &json!({"log": null, "limit": 16})).await?;
    let baseline = read(&world, &world.notes, &json!({"limit": 1})).await?;
    assert_eq!(page.log, baseline.log);
    let replay = read(
        &world,
        &world.notes,
        &json!({"log": page.log, "after": 0, "limit": head}),
    )
    .await?;
    assert_eq!(revocations(&replay), 0, "{replay:?}");
    assert!(
        replay
            .frames
            .contains(&ChangeFrame::Watermark { revision: head }),
        "{replay:?}"
    );
    let owner = read(
        &world,
        &world.files,
        &json!({"log": page.log, "after": 0, "limit": head}),
    )
    .await?;
    assert_eq!(revocations(&owner), 1, "{owner:?}");
    Ok(())
}

/// A foreign log and a cursor past the head are resets, never empty
/// success; a cursor without its log and a zero limit are refused.
#[tokio::test]
async fn a_foreign_log_or_rolled_back_cursor_is_a_reset() -> TestResult {
    let world = world().await?;
    let first = read(&world, &world.notes, &json!({"limit": 4})).await?;
    let foreign = json!({"log": {"identity": "elsewhere", "epoch": 0}, "after": 0, "limit": 4});
    let page = read(&world, &world.notes, &foreign).await?;
    assert!(matches!(
        page.frames.as_slice(),
        [ChangeFrame::Reset {
            reason: ResetReason::OtherLog,
            ..
        }]
    ));
    let ahead = json!({"log": first.log, "after": 1_000_000, "limit": 4});
    let page = read(&world, &world.notes, &ahead).await?;
    assert!(matches!(
        page.frames.as_slice(),
        [ChangeFrame::Reset {
            reason: ResetReason::Rollback,
            ..
        }]
    ));
    let unnamed = json!({"after": 0, "limit": 4});
    refused(
        &post(
            &world.service,
            "/grants/changes",
            Auth::Bearer(&world.notes),
            &unnamed,
        )
        .await?,
        400,
        "RequestMalformed",
    )?;
    let zero = json!({"limit": 0});
    refused(
        &post(
            &world.service,
            "/grants/changes",
            Auth::Bearer(&world.notes),
            &zero,
        )
        .await?,
        400,
        "RequestMalformed",
    )?;
    Ok(())
}

/// d089_r1_old_install_and_bounded_replay, identity half: the identity is
/// recorded once beside the grant log, survives a restart, and is the log
/// every membership decision names.
#[tokio::test]
async fn the_log_identity_is_recorded_once_and_named_by_membership() -> TestResult {
    let mut world = world().await?;
    let before = read(&world, &world.notes, &json!({"limit": 1})).await?;
    assert!(before.log.identity.contains('#'), "{:?}", before.log);
    world.service.restart().await?;
    let admin = world.service.sign_in(login(ADMINISTRATOR)).await?;
    world.admin = admin;
    let after = read(&world, &world.notes, &json!({"limit": 1})).await?;
    assert_eq!(before.log, after.log);
    let question = json!({
        "contract": 1,
        "log": {"identity": "not-this-log", "epoch": 0},
        "workspace": {"kind": format!("{NOTES}.workspace"), "id": "w"},
        "subject": {"id": world.bea, "kind": "person"},
        "resource": {"kind": format!("{NOTES}.channel"), "id": "c"},
        "action": "read",
        "at_least": 0,
    });
    let decided = ok(post(
        &world.service,
        "/grants/membership",
        Auth::Bearer(&world.notes),
        &question,
    )
    .await?)?;
    assert_eq!(decided["log"], serde_json::to_value(&after.log)?);
    Ok(())
}

/// d089_r1_replay_live_race, wait half: a waiting read is released by the
/// grants' own commit, and the revoke appears exactly once whether it
/// committed before the wait began or after.
#[tokio::test]
async fn a_waiting_read_is_released_by_the_commit_and_sees_it_once() -> TestResult {
    let world = world().await?;
    let doc = format!("{NOTES}.doc");
    let held = grant(&world, &doc, "1", "reader").await?;
    let first = read(&world, &world.notes, &json!({"limit": 16})).await?;
    let Some(ChangeFrame::Ready { revision: head }) = first.frames.last().cloned() else {
        return Err("a baseline ends ready".into());
    };
    let body = json!({"log": first.log, "after": head, "limit": 16, "wait": true});
    let base = world.service.base.clone();
    let credential = world.notes.clone();
    let waiting = tokio::spawn(async move {
        reqwest::Client::new()
            .post(format!("{base}/grants/changes"))
            .bearer_auth(credential)
            .json(&body)
            .send()
            .await?
            .json::<ChangesPage>()
            .await
    });
    let fence = revoke(&world, &held).await?;
    let mut page = waiting.await??;
    if revocations(&page) == 0 {
        // The wait was released before the revoke's frame was read: the
        // next read from the same cursor carries it.
        page = read(
            &world,
            &world.notes,
            &json!({"log": first.log, "after": head, "limit": 16}),
        )
        .await?;
    }
    assert_eq!(revocations(&page), 1, "{page:?}");
    assert_eq!(
        page.frames.last(),
        Some(&ChangeFrame::Ready { revision: fence })
    );
    Ok(())
}
