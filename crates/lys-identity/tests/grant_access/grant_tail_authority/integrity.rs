//! `TAIL_INTEGRITY`, `TAIL_RACE` and `TAIL_FAILURE` at the grant owner: every
//! changed owned input is refused, a witness older than an append is refused
//! by the original coherence error, and a provider's own failure is returned
//! unchanged, with no reading and no permission following.

use std::error::Error;
use std::sync::Arc;

use lys_identity::grants::{GrantError, TailAuthority, TailBearing, TailEffect};
use lys_log_store::witness::{FaultTailProvider, TailFaultStep, TailWitness};
use lys_log_store::{FileLeafStore, Frontier, LeafStore, PinnedRoot, StoreError};

use super::{append_elsewhere, chain, file_provider, revoke, witnessed};
use crate::support::World;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

/// The frontier of the first `size` leaves of the world's grant log.
fn frontier_at(world: &World, size: u64) -> TestResult<Frontier> {
    let store = FileLeafStore::open_read_only(&world.dir.path().join("grants"))?;
    let mut frontier = Frontier::new();
    for index in 0..size {
        frontier.push(&store.leaf(index)?.ok_or("a settled leaf is missing")?);
    }
    Ok(frontier)
}

/// Re-certify `witness`'s upper bound over its (changed) leaves, so only the
/// grant owner's own checks can refuse it.
fn recertified(world: &World, mut witness: TailWitness) -> TestResult<TailWitness> {
    let mut frontier = frontier_at(world, witness.lower.tree_size)?;
    for leaf in &witness.leaves {
        frontier.push(&leaf.bytes);
    }
    witness.upper = PinnedRoot {
        tree_size: frontier.size(),
        root: frontier.root(),
    };
    Ok(witness)
}

#[test]
fn every_changed_owned_input_is_refused_and_the_real_witness_verifies() -> TestResult {
    let mut world = World::new()?;
    let chain = chain(&mut world)?;
    let provider = file_provider(&world)?;
    witnessed(&mut world, Arc::new(provider))?;
    append_elsewhere(&world, revoke(chain.independent))?;
    append_elsewhere(&world, revoke(chain.first))?;
    let witness = world.grants.ledger().acquire_tail()?;
    let ledger = world.grants.ledger();
    assert_eq!(ledger.authenticate_tail(&witness)?.len(), 2);
    let mut changed = Vec::new();
    let mut case = witness.clone();
    case.origin.push_str("/other");
    changed.push(("log identity", case));
    let mut case = witness.clone();
    case.lower.tree_size -= 1;
    changed.push(("lower size", case));
    let mut case = witness.clone();
    case.lower.root[0] ^= 1;
    changed.push(("lower root", case));
    let mut case = witness.clone();
    case.upper.tree_size += 1;
    changed.push(("upper size", case));
    let mut case = witness.clone();
    case.upper.root[0] ^= 1;
    changed.push(("upper root", case));
    let mut case = witness.clone();
    case.leaves.remove(0);
    changed.push(("omitted index", recertified(&world, case)?));
    let mut case = witness.clone();
    case.leaves.push(case.leaves[1].clone());
    changed.push(("duplicated index", recertified(&world, case)?));
    let mut case = witness.clone();
    case.leaves.swap(0, 1);
    changed.push(("reordered index", recertified(&world, case)?));
    let mut case = witness.clone();
    let last = case.leaves[0].bytes.len() - 1;
    case.leaves[0].bytes[last] ^= 1;
    let signature = recertified(&world, case)?;
    assert_eq!(
        ledger.authenticate_tail(&signature).err(),
        Some(GrantError::SignatureInvalid),
        "a changed signature under a recertified root is the signature's refusal"
    );
    changed.push(("event signature", signature));
    for (name, case) in &changed {
        assert!(ledger.authenticate_tail(case).is_err(), "{name}");
        let mut called = false;
        let read = ledger.with_verified_tail(case, |_| {
            called = true;
            Ok(())
        });
        assert!(read.is_err(), "{name}: the provider reading refuses it");
        assert!(!called, "{name}: nothing is read from it");
    }
    let claimed = world.grants.tail_authority(&witness, chain.second)?;
    world
        .grants
        .confirm_tail_authority(&witness, chain.second, &claimed)?;
    let mut effects: Vec<TailAuthority> = Vec::new();
    let mut case = claimed.clone();
    case.effects[1].effect = TailEffect::Revoked { grant: chain.root };
    effects.push(case);
    let mut case = claimed.clone();
    case.bearing = TailBearing::Disjoint;
    effects.push(case);
    let mut case = claimed.clone();
    case.effects.pop();
    effects.push(case);
    let mut case = claimed.clone();
    case.tail_len = 1;
    effects.push(case);
    let mut case = claimed;
    case.lineage.pop();
    effects.push(case);
    for case in &effects {
        assert!(
            world
                .grants
                .confirm_tail_authority(&witness, chain.second, case)
                .is_err(),
            "a changed claimed effect is refused: {case:?}"
        );
    }
    Ok(())
}

#[test]
fn a_relevant_append_after_acquisition_refuses_the_older_witness() -> TestResult {
    let mut world = World::new()?;
    let chain = chain(&mut world)?;
    let provider = file_provider(&world)?;
    witnessed(&mut world, Arc::new(provider))?;
    let folded = world.grants.revision();
    append_elsewhere(&world, revoke(chain.independent))?;
    let witness = world.grants.ledger().acquire_tail()?;
    append_elsewhere(&world, revoke(chain.root))?;
    let refused = world.grants.tail_authority(&witness, chain.second);
    let reason = match refused {
        Err(GrantError::LogUnavailable { reason }) => reason,
        other => return Err(format!("the older witness was not refused: {other:?}").into()),
    };
    assert_eq!(
        reason,
        StoreError::LeafAlreadyWritten { index: folded + 1 }.to_string(),
        "the original coherence refusal, by name"
    );
    assert_eq!(
        world.grants.revision(),
        folded,
        "no permission or use followed"
    );
    let fresh = world.grants.ledger().acquire_tail()?;
    let authority = world.grants.tail_authority(&fresh, chain.second)?;
    assert_eq!(
        authority.bearing,
        TailBearing::Relevant {
            index: folded + 1,
            effect: TailEffect::Revoked { grant: chain.root },
        }
    );
    Ok(())
}

#[test]
fn a_fault_provider_refuses_by_its_original_error_and_is_complete_without_one() -> TestResult {
    let mut world = World::new()?;
    let chain = chain(&mut world)?;
    let (provider, faults) = FaultTailProvider::new(file_provider(&world)?);
    witnessed(&mut world, Arc::new(provider))?;
    append_elsewhere(&world, revoke(chain.root))?;
    let sentinel = |context: &'static str| {
        move || StoreError::Io {
            context: context.to_owned(),
            source: std::io::Error::other("tail-authority-sentinel"),
        }
    };
    faults
        .refuse(TailFaultStep::Acquire, sentinel("tail-authority-acquire"))
        .unwrap();
    let refused = world.grants.ledger().acquire_tail();
    assert!(
        matches!(&refused, Err(GrantError::LogUnavailable { reason }) if reason.contains("tail-authority-acquire")),
        "the provider's original failure, and no witness: {refused:?}"
    );
    faults.clear(TailFaultStep::Acquire).unwrap();
    let witness = world.grants.ledger().acquire_tail()?;
    faults
        .refuse(TailFaultStep::Verify, sentinel("tail-authority-verify"))
        .unwrap();
    let refused = world.grants.tail_authority(&witness, chain.second);
    assert!(
        matches!(&refused, Err(GrantError::LogUnavailable { reason }) if reason.contains("tail-authority-verify")),
        "the provider's original failure, and no classification: {refused:?}"
    );
    assert_eq!(faults.readings(), 0, "no reading reached the grant owner");
    faults.clear(TailFaultStep::Verify).unwrap();
    let authority = world.grants.tail_authority(&witness, chain.second)?;
    assert_eq!(
        authority.bearing,
        TailBearing::Relevant {
            index: witness.lower.tree_size,
            effect: TailEffect::Revoked { grant: chain.root },
        }
    );
    assert_eq!(
        (
            faults.acquisitions(),
            faults.verifications(),
            faults.readings()
        ),
        (2, 2, 1)
    );
    Ok(())
}
