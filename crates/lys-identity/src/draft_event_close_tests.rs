#![cfg(test)]
use crate::draft_event::{
    Approved, Created, DraftEvent, Executed, Refused, RefusedOnExecution, Target, decode, encode,
};
use crate::projection::Projection;
use crate::{
    Actor, AgentId, AuthMethod, IdentityError, IdentityId, LoginBinding, OperationId, PersonId,
    Provenance,
};
use std::error::Error;
use std::sync::Arc;

fn op(byte: u8) -> OperationId {
    OperationId::from_bytes([byte; 16])
}
fn actor() -> Result<Actor, IdentityError> {
    Ok(Actor::new(
        LoginBinding::new("https://issuer.test", "person")?,
        Provenance::new(AuthMethod::Oidc, 1),
    ))
}
/// An agent admitted on its own draft.decide grant, as the server records
/// the approver of a draft it decided.
fn agent_actor() -> Result<Actor, IdentityError> {
    let agent = AgentId::from_bytes([9; 16]);
    Ok(Actor::new(
        LoginBinding::new("https://issuer.test", &agent.to_string())?,
        Provenance::new(AuthMethod::AgentSignature(agent), 1),
    ))
}
fn created() -> Result<DraftEvent, IdentityError> {
    Ok(DraftEvent::Created(Arc::new(Created {
        operation: op(1),
        actor: actor()?,
        recorded_at: 1,
        target: Target {
            kind: "agent".into(),
            id: "one".into(),
            action: "agent.provisioning.set".into(),
        },
        method: "POST".into(),
        path: "/agents/one/provisioning".into(),
        body: b"{}".to_vec(),
        note: String::new(),
        reviewer: IdentityId::Person(PersonId::from_bytes([2; 16])),
        corrects: None,
        evidence: None,
        request_signature: None,
    })))
}
fn approved(hash: [u8; 32]) -> Result<DraftEvent, IdentityError> {
    Ok(DraftEvent::Approved(Arc::new(Approved {
        operation: op(3),
        actor: actor()?,
        recorded_at: 2,
        draft: op(1),
        draft_hash: hash,
        application: op(4),
        evidence: None,
    })))
}
fn close(hash: [u8; 32], refused: bool) -> Result<DraftEvent, IdentityError> {
    Ok(if refused {
        DraftEvent::RefusedOnExecution(Arc::new(RefusedOnExecution {
            operation: op(5),
            actor: actor()?,
            recorded_at: 3,
            draft: op(1),
            draft_hash: hash,
            application: op(4),
            refusal: "NotHeld".into(),
            reason: "the person does not hold this scope".into(),
        }))
    } else {
        DraftEvent::Executed(Arc::new(Executed {
            operation: op(5),
            actor: actor()?,
            recorded_at: 3,
            draft: op(1),
            draft_hash: hash,
            application: op(4),
            status: 200,
            receipt_index: 7,
        }))
    })
}

#[test]
fn closing_events_round_trip_and_fold_only_once_after_approval() -> Result<(), Box<dyn Error>> {
    for refused in [false, true] {
        let creation = created()?;
        let bytes = encode(&creation);
        assert_eq!(decode(&bytes)?, creation);
        let hash = crate::encoding::payload_commitment(&bytes);
        let mut projection = Projection::new();
        projection.apply_draft(&creation, 0)?;
        assert!(matches!(
            projection.check_draft(&close(hash, refused)?),
            Err(IdentityError::DraftNotPending { .. })
        ));
        projection.apply_draft(&approved(hash)?, 1)?;
        let closing = close(hash, refused)?;
        assert_eq!(encode(&decode(&encode(&closing))?), encode(&closing));
        projection.apply_draft(&closing, 8)?;
        let held = projection.draft(op(1)).ok_or("draft absent")?;
        assert_eq!(
            held.state(),
            if refused {
                "refused_on_execution"
            } else {
                "executed"
            }
        );
        if !refused {
            assert_eq!(
                held.executed
                    .as_ref()
                    .ok_or("execution absent")?
                    .0
                    .receipt_index,
                7
            );
        }
        assert!(matches!(
            projection.check_draft(&closing),
            Err(IdentityError::DraftNotPending { .. })
        ));
    }
    Ok(())
}

#[test]
fn a_refused_draft_cannot_close_as_executed() -> Result<(), Box<dyn Error>> {
    let creation = created()?;
    let hash = crate::encoding::payload_commitment(&encode(&creation));
    let mut projection = Projection::new();
    projection.apply_draft(&creation, 0)?;
    projection.apply_draft(
        &DraftEvent::Refused(Arc::new(Refused {
            operation: op(6),
            actor: actor()?,
            recorded_at: 2,
            draft: op(1),
            draft_hash: hash,
            reason: "no".into(),
            evidence: None,
        })),
        1,
    )?;
    assert!(matches!(
        projection.check_draft(&close(hash, false)?),
        Err(IdentityError::DraftNotPending { .. })
    ));
    Ok(())
}

#[test]
fn an_old_draft_log_reopens_and_its_closing_event_survives_a_snapshot() -> Result<(), Box<dyn Error>>
{
    let home = tempfile::tempdir()?;
    let key = home.path().join("key");
    lys_core::Ed25519Identity::load_or_generate(&key)?;
    let log = home.path().join("log");
    lys_log_store::FileLeafStore::create(&log, "example.test/drafts")?;
    let open = || -> Result<crate::Directory<lys_log_store::FileLeafStore>, Box<dyn Error>> {
        let log = log.clone();
        Ok(crate::Directory::open_with(
            Box::new(move || lys_log_store::FileLeafStore::open(&log)),
            lys_core::Ed25519Identity::load(&key)?,
            std::num::NonZeroU64::new(1).ok_or("snapshot interval absent")?,
        )?)
    };
    let creation = created()?;
    let hash = crate::encoding::payload_commitment(&encode(&creation));
    {
        let mut directory = open()?;
        directory.record_draft(creation)?;
        directory.record_draft(approved(hash)?)?;
    }
    let mut directory = open()?;
    assert_eq!(
        directory
            .projection()?
            .draft(op(1))
            .ok_or("draft absent")?
            .state(),
        "approved"
    );
    directory.record_draft(close(hash, false)?)?;
    assert!(matches!(
        directory.record_draft(close(hash, false)?),
        Err(IdentityError::DraftNotPending { .. })
    ));
    drop(directory);
    assert_eq!(
        open()?
            .projection()?
            .draft(op(1))
            .ok_or("draft absent")?
            .state(),
        "executed"
    );
    Ok(())
}

#[test]
fn a_correction_and_a_mismatched_application_cannot_close() -> Result<(), Box<dyn Error>> {
    let creation = created()?;
    let hash = crate::encoding::payload_commitment(&encode(&creation));
    let mut projection = Projection::new();
    projection.apply_draft(&creation, 0)?;
    let DraftEvent::Created(original) = creation else {
        return Err("creation absent".into());
    };
    let mut own = original.as_ref().clone();
    own.operation = op(8);
    own.recorded_at = 2;
    own.corrects = Some(op(1));
    let own = Arc::new(own);
    let corrected_hash =
        crate::encoding::payload_commitment(&encode(&DraftEvent::Created(Arc::clone(&own))));
    projection.apply_draft(
        &DraftEvent::Correction(Arc::new(crate::draft_event::Correction {
            operation: op(7),
            actor: actor()?,
            recorded_at: 2,
            draft: op(1),
            draft_hash: hash,
            reason: "corrected".into(),
            corrected: own,
            corrected_hash,
        })),
        1,
    )?;
    for refused in [false, true] {
        assert!(matches!(
            projection.check_draft(&close(hash, refused)?),
            Err(IdentityError::DraftNotPending { .. })
        ));
    }
    let mut projection = Projection::new();
    projection.apply_draft(&created()?, 0)?;
    projection.apply_draft(&approved(hash)?, 1)?;
    let DraftEvent::Executed(mut event) = close(hash, false)? else {
        return Err("execution absent".into());
    };
    Arc::make_mut(&mut event).application = op(9);
    assert!(matches!(
        projection.check_draft(&DraftEvent::Executed(event)),
        Err(IdentityError::DraftNotPending { .. })
    ));
    Ok(())
}

#[test]
fn an_application_is_reserved_even_before_its_first_execution() -> Result<(), Box<dyn Error>> {
    let creation = created()?;
    let hash = crate::encoding::payload_commitment(&encode(&creation));
    let mut projection = Projection::new();
    projection.apply_draft(&creation, 0)?;
    projection.apply_draft(&approved(hash)?, 1)?;
    let DraftEvent::Created(mut next) = created()? else {
        return Err("creation absent".into());
    };
    Arc::make_mut(&mut next).operation = op(10);
    let next = DraftEvent::Created(next);
    let next_hash = crate::encoding::payload_commitment(&encode(&next));
    projection.apply_draft(&next, 2)?;
    let DraftEvent::Approved(mut approval) = approved(next_hash)? else {
        return Err("approval absent".into());
    };
    Arc::make_mut(&mut approval).operation = op(11);
    Arc::make_mut(&mut approval).draft = op(10);
    assert!(matches!(
        projection.check_draft(&DraftEvent::Approved(approval)),
        Err(IdentityError::OperationReused { .. })
    ));
    Ok(())
}

/// An agent that decided a draft closes it (24 October 2026): the close
/// needs only its three operations kept apart, never a person's authority.
#[test]
fn an_agent_approver_closes_the_draft_it_decided() -> Result<(), Box<dyn Error>> {
    let hash = [4; 32];
    let executed = DraftEvent::Executed(Arc::new(Executed {
        operation: op(5),
        actor: agent_actor()?,
        recorded_at: 3,
        draft: op(1),
        draft_hash: hash,
        application: op(4),
        status: 200,
        receipt_index: 7,
    }));
    executed.validate()?;
    assert_eq!(encode(&decode(&encode(&executed))?), encode(&executed));
    let refused = DraftEvent::RefusedOnExecution(Arc::new(RefusedOnExecution {
        operation: op(5),
        actor: agent_actor()?,
        recorded_at: 3,
        draft: op(1),
        draft_hash: hash,
        application: op(4),
        refusal: "NotHeld".into(),
        reason: "the agent does not hold this scope".into(),
    }));
    refused.validate()?;
    let same = DraftEvent::Executed(Arc::new(Executed {
        operation: op(4),
        actor: agent_actor()?,
        recorded_at: 3,
        draft: op(1),
        draft_hash: hash,
        application: op(4),
        status: 200,
        receipt_index: 7,
    }));
    assert!(matches!(
        same.validate(),
        Err(IdentityError::DraftChangeInvalid { .. })
    ));
    Ok(())
}
