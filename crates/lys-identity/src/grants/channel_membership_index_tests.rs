//! The consumer index's frame order, evidence and readiness (ACCESS-006 R2,
//! DIRECTORY-089 R1): every count is asserted at the applied frame.

use std::error::Error;
use std::num::NonZeroUsize;

use lys_core::Ed25519Identity;

use super::{
    Applied, DEPENDENCY_REVOKED, MembershipIndex, STREAM_BEHIND, STREAM_FOREIGN_LOG, STREAM_GAP,
    STREAM_INTEGRITY, STREAM_NOT_READY, STREAM_READY_DISAGREES, STREAM_RESET,
};
use crate::grants::change_stream::{ChangeFrame, ChangeKind, LogName, ResetReason, event_hex};
use crate::grants::{GrantChange, GrantEvent, GrantId, Route, sign_grant_event};
use crate::{IdentityId, OperationId, PersonId};

type TestResult = Result<(), Box<dyn Error>>;

fn log() -> LogName {
    LogName {
        identity: "example.test/lys/grants#one".to_owned(),
        epoch: 0,
    }
}

struct Signer {
    _home: tempfile::TempDir,
    key: Ed25519Identity,
}

fn signer() -> Result<Signer, Box<dyn Error>> {
    let home = tempfile::tempdir()?;
    let key = Ed25519Identity::load_or_generate(&home.path().join("key"))?;
    Ok(Signer { _home: home, key })
}

fn caller() -> IdentityId {
    IdentityId::Person(PersonId::from_bytes([7; 16]))
}

/// The change frame of `change` to `grant` at `revision`, signed by `signer`.
fn frame(
    signer: &Signer,
    revision: u64,
    grant: GrantId,
    change: GrantChange,
) -> Result<ChangeFrame, Box<dyn Error>> {
    let kind = match change {
        GrantChange::Revoke { .. } => ChangeKind::Revoke,
        GrantChange::Issue(_) | GrantChange::Use { .. } => ChangeKind::Issue,
    };
    let event = GrantEvent::new(OperationId::generate()?, caller(), 1, change)?;
    let evidence = sign_grant_event(event, &signer.key)?;
    Ok(ChangeFrame::Change {
        revision,
        grant: grant.to_string(),
        change: kind,
        event: event_hex(evidence.bytes()),
    })
}

fn revoke(signer: &Signer, revision: u64, grant: GrantId) -> Result<ChangeFrame, Box<dyn Error>> {
    frame(
        signer,
        revision,
        grant,
        GrantChange::Revoke {
            grant,
            reason: "withdrawn".to_owned(),
        },
    )
}

fn index(signer: &Signer) -> Result<MembershipIndex, Box<dyn Error>> {
    Ok(MembershipIndex::new(
        log(),
        signer.key.public_key_bytes(),
        NonZeroUsize::new(4).ok_or("a window")?,
    ))
}

fn ready_at(index: &mut MembershipIndex, revision: u64) -> TestResult {
    index.apply(
        &log(),
        &ChangeFrame::Baseline {
            revision,
            revoked: Vec::new(),
        },
    )?;
    index.apply(&log(), &ChangeFrame::Ready { revision })?;
    Ok(())
}

/// `d089_r1_revision_and_ready_counts`, index half: a pass bound at r+1 is
/// refused at r, one revoke at r+1 is applied once, and the Ready covering
/// it admits only the independent path.
#[test]
fn one_revoke_is_applied_once_and_ready_covers_it() -> TestResult {
    let signer = signer()?;
    let mut index = index(&signer)?;
    ready_at(&mut index, 5)?;
    let (revoked, independent) = (GrantId::generate()?, GrantId::generate()?);
    let path = vec![revoked.to_string()];
    index.admit(5, &path)?;
    assert_eq!(
        index.admit(6, &path).map_err(|r| r.name),
        Err(STREAM_BEHIND)
    );

    let change = revoke(&signer, 6, revoked)?;
    assert_eq!(
        index.apply(&log(), &change)?,
        Applied::Change {
            revision: 6,
            change: ChangeKind::Revoke
        }
    );
    index.apply(&log(), &ChangeFrame::Ready { revision: 6 })?;
    assert_eq!(index.revoked_count(), 1);
    assert_eq!(
        index.admit(6, &path).map_err(|r| r.name),
        Err(DEPENDENCY_REVOKED)
    );
    index.admit(6, &[independent.to_string()])?;
    Ok(())
}

/// `d089_r1_replay_live_race`, index half: omission is a gap with its length,
/// an identical duplicate applies nothing, a conflicting one is refused.
#[test]
fn omission_duplicate_and_conflict_are_told_apart() -> TestResult {
    let signer = signer()?;
    let mut index = index(&signer)?;
    ready_at(&mut index, 2)?;
    let grant = GrantId::generate()?;
    let first = revoke(&signer, 3, grant)?;
    index.apply(&log(), &first)?;
    assert_eq!(
        index.apply(&log(), &first)?,
        Applied::Duplicate { revision: 3 }
    );
    assert_eq!(index.revoked_count(), 1, "a duplicate applies nothing");
    let conflicting = revoke(&signer, 3, grant)?;
    let refused = index.apply(&log(), &conflicting).err().ok_or("accepted")?;
    assert_eq!(refused.name, STREAM_INTEGRITY);
    assert!(!index.is_ready());

    let omitted = revoke(&signer, 7, GrantId::generate()?)?;
    let gap = index.apply(&log(), &omitted).err().ok_or("accepted")?;
    assert_eq!((gap.name, gap.gap), (STREAM_GAP, Some(3)));
    assert_eq!(index.cursor(), Some(3), "a gap moves nothing");
    Ok(())
}

/// A watermark moves the cursor over hidden changes; Ready must name the
/// cursor exactly.
#[test]
fn watermarks_advance_and_ready_must_agree() -> TestResult {
    let signer = signer()?;
    let mut index = index(&signer)?;
    ready_at(&mut index, 1)?;
    index.apply(&log(), &ChangeFrame::Watermark { revision: 9 })?;
    assert_eq!(index.cursor(), Some(9));
    let early = index
        .apply(&log(), &ChangeFrame::Ready { revision: 8 })
        .err()
        .ok_or("accepted")?;
    assert_eq!(early.name, STREAM_READY_DISAGREES);
    assert_eq!(
        index.admit(1, &[]).map_err(|r| r.name),
        Err(STREAM_NOT_READY)
    );
    index.apply(&log(), &ChangeFrame::Ready { revision: 9 })?;
    index.admit(9, &[])?;
    Ok(())
}

/// A foreign log, a reset, a bad signature, a substituted grant and a use
/// each withdraw readiness by name.
#[test]
fn foreign_reset_and_forged_frames_withdraw_ready() -> TestResult {
    let other = signer()?;
    let signer = signer()?;
    let mut index = index(&signer)?;
    let grant = GrantId::generate()?;
    let foreign = LogName { epoch: 1, ..log() };
    ready_at(&mut index, 1)?;
    let refused = index.apply(&foreign, &ChangeFrame::Ready { revision: 1 });
    assert_eq!(refused.map_err(|r| r.name), Err(STREAM_FOREIGN_LOG));

    ready_at(&mut index, 1)?;
    let forged = revoke(&other, 2, grant)?;
    assert_eq!(
        index.apply(&log(), &forged).map_err(|r| r.name),
        Err(STREAM_INTEGRITY)
    );
    let ChangeFrame::Change { event, .. } = revoke(&signer, 2, grant)? else {
        return Err("a change frame".into());
    };
    let substituted = ChangeFrame::Change {
        revision: 2,
        grant: GrantId::generate()?.to_string(),
        change: ChangeKind::Revoke,
        event,
    };
    assert_eq!(
        index.apply(&log(), &substituted).map_err(|r| r.name),
        Err(STREAM_INTEGRITY)
    );
    let used = frame(
        &signer,
        2,
        grant,
        GrantChange::Use {
            grant,
            route: Route::Api,
        },
    )?;
    assert_eq!(
        index.apply(&log(), &used).map_err(|r| r.name),
        Err(STREAM_INTEGRITY)
    );
    assert_eq!(index.cursor(), Some(1));

    ready_at(&mut index, 1)?;
    let reset = ChangeFrame::Reset {
        reason: ResetReason::Rollback,
        from: Some(log()),
    };
    assert_eq!(
        index.apply(&log(), &reset).map_err(|r| r.name),
        Err(STREAM_RESET)
    );
    assert_eq!(index.cursor(), None);
    assert!(!index.is_ready());
    Ok(())
}

/// A transport failure withdraws readiness until Ready is applied again,
/// and a revoked grant stays revoked across a baseline.
#[test]
fn withdrawal_and_baselines_never_readmit_a_revoked_grant() -> TestResult {
    let signer = signer()?;
    let mut index = index(&signer)?;
    let grant = GrantId::generate()?;
    ready_at(&mut index, 1)?;
    index.apply(&log(), &revoke(&signer, 2, grant)?)?;
    index.withdraw();
    assert_eq!(
        index.admit(0, &[]).map_err(|r| r.name),
        Err(STREAM_NOT_READY)
    );
    ready_at(&mut index, 2)?;
    assert_eq!(index.revoked_at(&grant.to_string()), Some(2));
    assert_eq!(
        index.admit(2, &[grant.to_string()]).map_err(|r| r.name),
        Err(DEPENDENCY_REVOKED)
    );
    Ok(())
}

fn placement(revision: u64, placement: u64, widens: &[&str]) -> ChangeFrame {
    ChangeFrame::Placement {
        revision,
        placement,
        child: "notes.channel:general".to_owned(),
        parent: "notes.workspace:team".to_owned(),
        restricted: false,
        widens: widens.iter().map(|grant| (*grant).to_owned()).collect(),
    }
}

/// `d089_r1_replay_live_race`, placement half: an index following
/// placements applies each one in turn at the revision it was kept after,
/// tells an identical duplicate from a conflict, an omitted placement from
/// an omitted revision, and is ready only once a mark names its revision.
#[test]
fn placements_apply_in_order_with_the_changes() -> TestResult {
    let signer = signer()?;
    let mut index = index(&signer)?.following_placements();
    index.apply(
        &log(),
        &ChangeFrame::Baseline {
            revision: 5,
            revoked: Vec::new(),
        },
    )?;
    let unmarked = index.apply(&log(), &ChangeFrame::Ready { revision: 5 });
    assert_eq!(unmarked.map_err(|r| r.name), Err(STREAM_READY_DISAGREES));
    let mark = ChangeFrame::PlacementMark {
        revision: 5,
        placement: 2,
    };
    assert_eq!(
        index.apply(&log(), &mark)?,
        Applied::PlacementMark { placement: 2 }
    );
    index.apply(&log(), &ChangeFrame::Ready { revision: 5 })?;
    assert_eq!((index.placed(), index.is_ready()), (Some(2), true));

    let widening = placement(5, 3, &["grant-a"]);
    assert_eq!(
        index.apply(&log(), &widening)?,
        Applied::Placement {
            placement: 3,
            widens: 1
        }
    );
    assert_eq!(
        index.apply(&log(), &widening)?,
        Applied::PlacementDuplicate { placement: 3 }
    );
    assert_eq!(index.placed(), Some(3), "a duplicate applies nothing");
    let conflicting = index.apply(&log(), &placement(5, 3, &["grant-b"]));
    assert_eq!(conflicting.map_err(|r| r.name), Err(STREAM_INTEGRITY));
    let omitted = index
        .apply(&log(), &placement(5, 5, &[]))
        .err()
        .ok_or("accepted")?;
    assert_eq!((omitted.name, omitted.gap), (STREAM_GAP, Some(1)));

    index.apply(&log(), &revoke(&signer, 6, GrantId::generate()?)?)?;
    let before = index.apply(&log(), &placement(5, 4, &[]));
    assert_eq!(before.map_err(|r| r.name), Err(STREAM_INTEGRITY));
    let ahead = index
        .apply(&log(), &placement(7, 4, &[]))
        .err()
        .ok_or("accepted")?;
    assert_eq!((ahead.name, ahead.gap), (STREAM_GAP, Some(1)));
    assert_eq!(index.placed(), Some(3), "a refused placement moves nothing");
    index.apply(&log(), &placement(6, 4, &[]))?;
    let unmarked = index.apply(&log(), &ChangeFrame::Ready { revision: 6 });
    assert_eq!(unmarked.map_err(|r| r.name), Err(STREAM_READY_DISAGREES));
    let early_mark = ChangeFrame::PlacementMark {
        revision: 7,
        placement: 4,
    };
    let early = index.apply(&log(), &early_mark);
    assert_eq!(early.map_err(|r| r.name), Err(STREAM_GAP));
    let barrier = ChangeFrame::PlacementMark {
        revision: 6,
        placement: 4,
    };
    index.apply(&log(), &barrier)?;
    index.apply(&log(), &ChangeFrame::Ready { revision: 6 })?;
    assert_eq!((index.placed(), index.is_ready()), (Some(4), true));

    let reset = ChangeFrame::Reset {
        reason: ResetReason::Unordered,
        from: Some(log()),
    };
    assert_eq!(
        index.apply(&log(), &reset).map_err(|r| r.name),
        Err(STREAM_RESET)
    );
    assert_eq!(index.placed(), None, "a reset forgets the placement cursor");
    Ok(())
}

/// An index that does not follow placements refuses a placement frame by
/// name, and its readiness needs no mark.
#[test]
fn an_index_not_following_placements_refuses_them() -> TestResult {
    let signer = signer()?;
    let mut index = index(&signer)?;
    ready_at(&mut index, 2)?;
    let refused = index.apply(&log(), &placement(2, 1, &[]));
    assert_eq!(refused.map_err(|r| r.name), Err(STREAM_INTEGRITY));
    assert!(!index.is_ready());
    Ok(())
}
