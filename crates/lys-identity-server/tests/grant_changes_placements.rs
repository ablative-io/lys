#![cfg(test)]
//! Placements in the grant change stream over HTTP (DIRECTORY-089 R1): a
//! placement widens what a grant reaches without being a grant event, so a
//! read following placements is given each one in order with the grant
//! changes, at the revision it was kept after, resumable from either half
//! of its cursor, and applied by the consumer index (ACCESS-006 R2). Every
//! frame is asserted exactly, at the applied page.

use std::error::Error;
use std::num::NonZeroUsize;

use identity_contract::apps::{
    Auth, FILES, NOTES, TestResult, login, ok, op, post, refused, registered, root,
};
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::grants::change_stream::{ChangeFrame, ChangeKind, ChangesPage, ResetReason};
use lys_identity::grants::channel_membership_index::{Applied, MembershipIndex};
use lys_identity_server::apps_state::{By, Placed};
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

async fn read(world: &World, auth: Auth<'_>, body: &Value) -> Result<ChangesPage, Box<dyn Error>> {
    let answer = ok(post(&world.service, "/grants/changes", auth, body).await?)?;
    Ok(serde_json::from_value(answer)?)
}

async fn place(
    world: &World,
    (app, credential): (&str, &str),
    child: &str,
    parent: &str,
) -> TestResult {
    let body = json!({
        "operation": op()?,
        "child": {"kind": format!("{app}.channel"), "id": child},
        "parent": {"kind": format!("{app}.workspace"), "id": parent},
    });
    ok(post(
        &world.service,
        &format!("/apps/{app}/placements"),
        Auth::Bearer(credential),
        &body,
    )
    .await?)?;
    Ok(())
}

fn service_key(world: &World) -> Result<[u8; 32], Box<dyn Error>> {
    let key =
        lys_identity::signer::load_service_key(&world.service.dir.path().join("service.key"))?;
    Ok(key.public_key_bytes())
}

fn channel(id: &str) -> String {
    format!("{NOTES}.channel:{id}")
}

fn team() -> String {
    format!("{NOTES}.workspace:team")
}

fn placement(revision: u64, position: u64, child: &str, widens: &[&str]) -> ChangeFrame {
    ChangeFrame::Placement {
        revision,
        placement: position,
        child: channel(child),
        parent: team(),
        restricted: false,
        widens: widens.iter().map(|grant| (*grant).to_owned()).collect(),
    }
}

fn mark(revision: u64, placement: u64) -> ChangeFrame {
    ChangeFrame::PlacementMark {
        revision,
        placement,
    }
}

/// The world after: Bea holds `member` on the notes workspace `team` at
/// revision `r`; the notes app places `general` in `team` and the files app
/// places one of its own channels, both after `r`; the workspace grant is
/// revoked at `r + 1`; and the notes app places `random` in `team` after it.
struct Placed3 {
    world: World,
    first: ChangesPage,
    member: String,
    r: u64,
}

async fn placed3() -> Result<Placed3, Box<dyn Error>> {
    let world = world().await?;
    let issued = ok(root(
        &world.service,
        &world.admin,
        &world.bea,
        (&format!("{NOTES}.workspace"), "team"),
        "member",
    )
    .await?)?;
    let member = issued["grant"]
        .as_str()
        .ok_or("the issue names its grant")?
        .to_owned();
    let first = read(
        &world,
        Auth::Bearer(&world.notes),
        &json!({"limit": 16, "placements": true}),
    )
    .await?;
    let Some(ChangeFrame::Baseline { revision: r, .. }) = first.frames.first().cloned() else {
        return Err(format!("a first read starts with a baseline: {first:?}").into());
    };
    assert_eq!(
        first.frames[1..],
        [mark(r, 0), ChangeFrame::Ready { revision: r }],
        "a first read following placements stands after every one"
    );
    place(&world, (NOTES, &world.notes), "general", "team").await?;
    place(&world, (FILES, &world.files), "elsewhere", "theirs").await?;
    let body = json!({"operation": op()?, "route": "api", "reason": "left the team"});
    let revoked = ok(post(
        &world.service,
        &format!("/grants/{member}/revoke"),
        Auth::Cookie(&world.admin),
        &body,
    )
    .await?)?;
    assert_eq!(revoked["receipt"]["revision"].as_u64(), Some(r + 1));
    place(&world, (NOTES, &world.notes), "random", "team").await?;
    Ok(Placed3 {
        world,
        first,
        member,
        r,
    })
}

/// The revoke of the workspace grant at `r + 1`, as the page carries it.
fn is_revoke(frame: &ChangeFrame, revision: u64, grant: &str) -> bool {
    matches!(
        frame,
        ChangeFrame::Change { revision: at, grant: named, change: ChangeKind::Revoke, .. }
            if *at == revision && named == grant
    )
}

/// One read after the cursor gives the placements in order with the
/// revoke: the first widened the workspace grant, the other app's is a
/// mark, the one after the revoke widened nothing, and readiness follows
/// the mark naming the head and every placement. The index applies it all
/// once and is ready at both halves of the cursor.
#[tokio::test]
async fn placements_are_ordered_with_the_grant_changes() -> TestResult {
    let Placed3 {
        world,
        first,
        member,
        r,
    } = placed3().await?;
    let mut index = MembershipIndex::new(
        first.log.clone(),
        service_key(&world)?,
        NonZeroUsize::new(16).ok_or("a window")?,
    )
    .following_placements();
    index.apply_page(&first.log, &first.frames)?;
    assert_eq!((index.placed(), index.is_ready()), (Some(0), true));

    let cursor =
        json!({"log": first.log, "after": r, "placed": 0, "limit": 16, "placements": true});
    let page = read(&world, Auth::Bearer(&world.notes), &cursor).await?;
    let [widening, hidden, revoke, narrow, barrier, ready] = page.frames.as_slice() else {
        return Err(format!("six frames: {page:?}").into());
    };
    assert_eq!(*widening, placement(r, 1, "general", &[&member]));
    assert_eq!(
        *hidden,
        mark(r, 2),
        "the files app's placement is not shown"
    );
    assert!(is_revoke(revoke, r + 1, &member), "{revoke:?}");
    assert_eq!(
        *narrow,
        placement(r + 1, 3, "random", &[]),
        "a revoked grant widens nothing"
    );
    assert_eq!(*barrier, mark(r + 1, 3));
    assert_eq!(*ready, ChangeFrame::Ready { revision: r + 1 });

    let applied = index.apply_page(&page.log, &page.frames)?;
    assert_eq!(
        applied
            .iter()
            .filter(|done| matches!(done, Applied::Placement { .. }))
            .count(),
        2
    );
    assert_eq!(applied.last(), Some(&Applied::Ready { revision: r + 1 }));
    assert_eq!(
        (index.cursor(), index.placed(), index.revoked_count()),
        (Some(r + 1), Some(3), 1)
    );

    // The administrator sees the other app's placement itself.
    let all = read(&world, Auth::Cookie(&world.admin), &cursor).await?;
    assert!(
        matches!(&all.frames[1], ChangeFrame::Placement { placement: 2, revision, .. } if *revision == r),
        "{all:?}"
    );

    // Nothing follows the cursor at the head: the mark and Ready alone.
    let idle =
        json!({"log": first.log, "after": r + 1, "placed": 3, "limit": 16, "placements": true});
    let again = read(&world, Auth::Bearer(&world.notes), &idle).await?;
    assert_eq!(
        again.frames,
        vec![mark(r + 1, 3), ChangeFrame::Ready { revision: r + 1 }]
    );
    Ok(())
}

/// Read one position at a time, the stream resumes from each half of the
/// cursor across every placement, a placement read again is the same
/// frame and applies nothing, and the index ends where one read leaves it.
#[tokio::test]
async fn a_cursor_resumes_across_each_placement() -> TestResult {
    let Placed3 {
        world,
        first,
        member,
        r,
    } = placed3().await?;
    let mut index = MembershipIndex::new(
        first.log.clone(),
        service_key(&world)?,
        NonZeroUsize::new(16).ok_or("a window")?,
    )
    .following_placements();
    index.apply_page(&first.log, &first.frames)?;
    let step = |after: u64, placed: u64| json!({"log": first.log, "after": after, "placed": placed, "limit": 1, "placements": true});

    let one = read(&world, Auth::Bearer(&world.notes), &step(r, 0)).await?;
    assert_eq!(one.frames, vec![placement(r, 1, "general", &[&member])]);
    index.apply_page(&one.log, &one.frames)?;
    let twice = read(&world, Auth::Bearer(&world.notes), &step(r, 0)).await?;
    assert_eq!(
        twice.frames, one.frames,
        "a placement read again is the same frame"
    );
    assert_eq!(
        index.apply_page(&twice.log, &twice.frames)?,
        vec![Applied::PlacementDuplicate { placement: 1 }]
    );

    let two = read(&world, Auth::Bearer(&world.notes), &step(r, 1)).await?;
    assert_eq!(two.frames, vec![mark(r, 2)]);
    index.apply_page(&two.log, &two.frames)?;
    assert_eq!(
        index.placed(),
        Some(2),
        "a mark covers the other app's placement"
    );

    let three = read(&world, Auth::Bearer(&world.notes), &step(r, 2)).await?;
    let [revoke] = three.frames.as_slice() else {
        return Err(format!("one frame: {three:?}").into());
    };
    assert!(is_revoke(revoke, r + 1, &member), "{revoke:?}");
    index.apply_page(&three.log, &three.frames)?;

    let four = read(&world, Auth::Bearer(&world.notes), &step(r + 1, 2)).await?;
    assert_eq!(
        four.frames,
        vec![
            placement(r + 1, 3, "random", &[]),
            mark(r + 1, 3),
            ChangeFrame::Ready { revision: r + 1 },
        ]
    );
    index.apply_page(&four.log, &four.frames)?;
    assert_eq!(
        (index.cursor(), index.placed(), index.is_ready()),
        (Some(r + 1), Some(3), true)
    );
    Ok(())
}

/// A read not following placements is answered exactly as before them; a
/// cursor that is no position of the order is a named reset; a placement
/// cursor without its half, or given unasked, is refused.
#[tokio::test]
async fn an_unordered_cursor_is_a_reset_and_an_unasked_read_is_unchanged() -> TestResult {
    let Placed3 {
        world,
        first,
        member,
        r,
    } = placed3().await?;
    let plain = read(
        &world,
        Auth::Bearer(&world.notes),
        &json!({"log": first.log, "after": r, "limit": 16}),
    )
    .await?;
    let [revoke, ready] = plain.frames.as_slice() else {
        return Err(format!("two frames: {plain:?}").into());
    };
    assert!(is_revoke(revoke, r + 1, &member), "{revoke:?}");
    assert_eq!(*ready, ChangeFrame::Ready { revision: r + 1 });

    for (after, placed, reason) in [
        // Past the placements held.
        (r + 1, 9, ResetReason::Rollback),
        // Past the revoke, before the placements kept ahead of it.
        (r + 1, 0, ResetReason::Unordered),
        // Placements applied that stand after the revision applied.
        (r, 3, ResetReason::Unordered),
    ] {
        let cursor = json!({"log": first.log, "after": after, "placed": placed, "limit": 16, "placements": true});
        let page = read(&world, Auth::Bearer(&world.notes), &cursor).await?;
        assert!(
            matches!(page.frames.as_slice(), [ChangeFrame::Reset { reason: named, .. }] if *named == reason),
            "{after}/{placed}: {page:?}"
        );
    }

    for body in [
        json!({"log": first.log, "after": r, "placed": 0, "limit": 16}),
        json!({"log": first.log, "after": r, "limit": 16, "placements": true}),
        json!({"placed": 0, "limit": 16, "placements": true}),
    ] {
        refused(
            &post(
                &world.service,
                "/grants/changes",
                Auth::Bearer(&world.notes),
                &body,
            )
            .await?,
            400,
            "RequestMalformed",
        )?;
    }
    Ok(())
}

/// A placement kept before placements were ordered keeps its bytes and
/// reads with no revision; one kept since names its revision.
#[test]
fn an_old_placement_line_keeps_its_bytes() -> TestResult {
    let old = Placed {
        operation: "op-0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b".to_owned(),
        app: "notes".to_owned(),
        child_kind: "notes_page".to_owned(),
        child_id: "first".to_owned(),
        parent_kind: "notes_book".to_owned(),
        parent_id: "shelf".to_owned(),
        restricted: false,
        revision: None,
        by: By::Start,
        at: 11,
    };
    let bytes = serde_json::to_vec(&old)?;
    assert!(
        !String::from_utf8(bytes.clone())?.contains("revision"),
        "an unordered placement is written as before"
    );
    let read: Placed = serde_json::from_slice(&bytes)?;
    assert_eq!(read, old);
    assert_eq!(serde_json::to_vec(&read)?, bytes);

    let ordered = Placed {
        revision: Some(4),
        ..old
    };
    let bytes = serde_json::to_vec(&ordered)?;
    assert_eq!(serde_json::from_slice::<Placed>(&bytes)?.revision, Some(4));
    Ok(())
}
