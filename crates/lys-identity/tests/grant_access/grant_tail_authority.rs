#![cfg(test)]

//! DIRECTORY-090 R6: a complete authenticated tail's authority effects,
//! compared to replay (`TAIL_AUTHORITY`), refused when any owned input is
//! changed (`TAIL_INTEGRITY`), refused after a later append (`TAIL_RACE`), and
//! refused by its provider's original failure (`TAIL_FAILURE`).

mod integrity;

use std::error::Error;
use std::sync::Arc;

use crate::support::{World, alpha, pass};
use lys_core::Ed25519Identity;
use lys_identity::grants::{
    Action, ExerciseRequest, GrantChange, GrantError, GrantEvent, GrantId, GrantLedger, Grants,
    MemoryRelationships, PassOn, Permit, RecipientKind, Route, Source, TailBearing, TailEffect,
    TailEffectAt, sign_grant_event,
};
use lys_identity::restart::SNAPSHOT_EVERY;
use lys_identity::{IdentityId, OperationId};
use lys_log_store::FileLeafStore;
use lys_log_store::witness::{FileTailProvider, TailWitnessProvider};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

const BOTH: [RecipientKind; 2] = [RecipientKind::Person, RecipientKind::Agent];

/// Two chains on one resource, a one-time hop, and an independent root.
#[derive(Debug, Clone, Copy)]
struct Chain {
    root: GrantId,
    first: GrantId,
    second: GrantId,
    independent: GrantId,
    once_root: GrantId,
    once: GrantId,
}

fn chain(world: &mut World) -> TestResult<Chain> {
    let (dana, tom, tom_agent, lee) = (
        IdentityId::Person(world.dana),
        IdentityId::Person(world.tom),
        IdentityId::Agent(world.tom_agent),
        IdentityId::Person(world.lee),
    );
    let root = world.root(world.dana, "kite", pass(&["read"], &BOTH)?, None)?;
    let first = world.request(
        dana,
        root,
        tom,
        "tern",
        pass(&["read"], &[RecipientKind::Agent])?,
        None,
    )?;
    let first = world.delegate(&first)?.event.grant();
    let second = world.request(tom, first, tom_agent, "tern", PassOn::UseOnly, None)?;
    let second = world.delegate(&second)?.event.grant();
    let independent = world.root(world.lee, "tern", PassOn::UseOnly, None)?;
    let once_root = world.root(
        world.dana,
        "heron",
        pass(&["read"], &[RecipientKind::Person])?,
        None,
    )?;
    let once = world.request(dana, once_root, lee, "tern", PassOn::UseOnly, None)?;
    let directory = world.directory.projection()?;
    let once = world
        .grants
        .delegate_once(directory, &once, world.now)?
        .event
        .grant();
    Ok(Chain {
        root,
        first,
        second,
        independent,
        once_root,
        once,
    })
}

/// Reopen the world's grants with `provider` as their explicit capability.
fn witnessed(
    world: &mut World,
    provider: Arc<dyn TailWitnessProvider + Send + Sync>,
) -> TestResult {
    let path = world.dir.path().join("grants");
    world.grants = Grants::open_with_tail_provider(
        Box::new(move || FileLeafStore::open(&path)),
        Ed25519Identity::load(&world.dir.path().join("service.key"))?,
        MemoryRelationships::default(),
        world.grants.model().clone(),
        world.admin,
        SNAPSHOT_EVERY,
        Some(provider),
    )?;
    Ok(())
}

fn file_provider(world: &World) -> TestResult<FileTailProvider> {
    Ok(FileTailProvider::new(FileLeafStore::open_read_only(
        &world.dir.path().join("grants"),
    )?))
}

/// Append `change` by `caller` through a second, independent handle on the
/// same log.
fn append_by(world: &World, caller: IdentityId, change: GrantChange) -> TestResult {
    let path = world.dir.path().join("grants");
    let key = Ed25519Identity::load(&world.dir.path().join("service.key"))?;
    let (mut writer, opening) = GrantLedger::open(
        Box::new(move || FileLeafStore::open(&path)),
        &key,
        SNAPSHOT_EVERY,
    )?;
    drop(opening);
    let event = GrantEvent::new(OperationId::generate()?, caller, world.now, change)?;
    writer.append(&sign_grant_event(event, &key)?)?;
    drop(writer);
    Ok(())
}

/// Append `change` by the root authority through a second handle.
fn append_elsewhere(world: &World, change: GrantChange) -> TestResult {
    append_by(world, IdentityId::Person(world.admin), change)
}

fn revoke(grant: GrantId) -> GrantChange {
    GrantChange::Revoke {
        grant,
        reason: "withdrawn through another handle".to_owned(),
    }
}

/// What a full replay of every leaf decides for `holder` resting on `grant`.
fn replayed(
    world: &mut World,
    holder: IdentityId,
    grant: GrantId,
) -> TestResult<Result<Permit, GrantError>> {
    world.reopen(MemoryRelationships::default())?;
    let request = ExerciseRequest {
        caller: holder,
        route: Route::Api,
        resource: alpha()?,
        action: Action::new("read")?,
    };
    let directory = world.directory.projection()?;
    let now = world.now;
    Ok(world
        .grants
        .explain_by(directory, &request, Some(grant), now, None))
}

struct Case {
    name: &'static str,
    holder: fn(&World) -> IdentityId,
    selected: fn(&Chain) -> GrantId,
    /// The event, and whether its holder (rather than the root authority)
    /// records it.
    change: fn(&Chain) -> (GrantChange, bool),
    lineage: fn(&Chain) -> Vec<GrantId>,
    effect: fn(&Chain) -> TailEffect,
    /// The grant replay names revoked, or `None` when the lineage stands.
    ends: fn(&Chain) -> Option<GrantId>,
}

fn agent(world: &World) -> IdentityId {
    IdentityId::Agent(world.tom_agent)
}

fn lee(world: &World) -> IdentityId {
    IdentityId::Person(world.lee)
}

fn agent_path(chain: &Chain) -> Vec<GrantId> {
    vec![chain.second, chain.first, chain.root]
}

fn cases() -> [Case; 6] {
    [
        Case {
            name: "direct grant revoked",
            holder: agent,
            selected: |chain| chain.second,
            change: |chain| (revoke(chain.second), false),
            lineage: agent_path,
            effect: |chain| TailEffect::Revoked {
                grant: chain.second,
            },
            ends: |chain| Some(chain.second),
        },
        Case {
            name: "middle ancestor revoked",
            holder: agent,
            selected: |chain| chain.second,
            change: |chain| (revoke(chain.first), false),
            lineage: agent_path,
            effect: |chain| TailEffect::Revoked { grant: chain.first },
            ends: |chain| Some(chain.first),
        },
        Case {
            name: "root ancestor revoked",
            holder: agent,
            selected: |chain| chain.second,
            change: |chain| (revoke(chain.root), false),
            lineage: agent_path,
            effect: |chain| TailEffect::Revoked { grant: chain.root },
            ends: |chain| Some(chain.root),
        },
        Case {
            name: "one-time hop spent",
            holder: lee,
            selected: |chain| chain.once,
            change: |chain| {
                (
                    GrantChange::Use {
                        grant: chain.once,
                        route: Route::Api,
                    },
                    true,
                )
            },
            lineage: |chain| vec![chain.once, chain.once_root],
            effect: |chain| TailEffect::Spent { grant: chain.once },
            ends: |chain| Some(chain.once),
        },
        Case {
            name: "fully independent chain revoked",
            holder: agent,
            selected: |chain| chain.second,
            change: |chain| (revoke(chain.independent), false),
            lineage: agent_path,
            effect: |chain| TailEffect::Revoked {
                grant: chain.independent,
            },
            ends: |_| None,
        },
        Case {
            name: "a lineage grant used without spending",
            holder: agent,
            selected: |chain| chain.second,
            change: |chain| {
                (
                    GrantChange::Use {
                        grant: chain.second,
                        route: Route::Tool,
                    },
                    true,
                )
            },
            lineage: agent_path,
            effect: |chain| TailEffect::Used {
                grant: chain.second,
            },
            ends: |_| None,
        },
    ]
}

#[test]
fn tail_authority_agrees_with_replay_for_every_hop_and_an_independent_chain() -> TestResult {
    for case in &cases() {
        let mut world = World::new()?;
        let chain = chain(&mut world)?;
        let provider = file_provider(&world)?;
        witnessed(&mut world, Arc::new(provider))?;
        let settled = world.grants.revision();
        let holder = (case.holder)(&world);
        let (change, by_holder) = (case.change)(&chain);
        if by_holder {
            append_by(&world, holder, change)?;
        } else {
            append_elsewhere(&world, change)?;
        }
        let witness = world.grants.ledger().acquire_tail()?;
        let selected = (case.selected)(&chain);
        let authority = world.grants.tail_authority(&witness, selected)?;
        let effect = (case.effect)(&chain);
        assert_eq!(authority.lineage, (case.lineage)(&chain), "{}", case.name);
        assert_eq!(
            authority.effects,
            vec![TailEffectAt {
                index: settled,
                effect: effect.clone()
            }],
            "{}",
            case.name
        );
        assert_eq!(authority.tail_len, 1, "{}: the tail length", case.name);
        assert_eq!(
            authority.lineage_visits, 1,
            "{}: one lineage visit per tail event",
            case.name
        );
        let replay = replayed(&mut world, holder, selected)?;
        if let Some(ended) = (case.ends)(&chain) {
            assert_eq!(
                authority.bearing,
                TailBearing::Relevant {
                    index: settled,
                    effect,
                },
                "{}",
                case.name
            );
            assert_eq!(
                replay.map(|permit| permit.grant),
                Err(GrantError::Revoked {
                    grant: ended.to_string()
                }),
                "{}: replay refuses what the tail named relevant",
                case.name
            );
        } else {
            assert_eq!(authority.bearing, TailBearing::Disjoint, "{}", case.name);
            assert_eq!(
                replay.map(|permit| permit.grant),
                Ok(selected),
                "{}: replay keeps what the tail proved disjoint",
                case.name
            );
        }
    }
    Ok(())
}

#[test]
fn an_effect_on_a_grant_neither_the_book_nor_the_tail_names_is_unclassifiable() -> TestResult {
    let mut world = World::new()?;
    let chain = chain(&mut world)?;
    let provider = file_provider(&world)?;
    witnessed(&mut world, Arc::new(provider))?;
    let settled = world.grants.revision();
    let stranger = GrantId::generate()?;
    append_elsewhere(&world, revoke(chain.independent))?;
    append_elsewhere(&world, revoke(stranger))?;
    append_elsewhere(&world, revoke(chain.root))?;
    let witness = world.grants.ledger().acquire_tail()?;
    let authority = world.grants.tail_authority(&witness, chain.second)?;
    assert_eq!(
        authority.bearing,
        TailBearing::Unclassifiable {
            index: settled + 1,
            effect: TailEffect::Unknown { grant: stranger },
        },
        "an unknown effect cannot prove disjointness, and it is met first"
    );
    assert_eq!(authority.effects.len(), 3, "every effect is still named");
    assert_eq!(
        authority.effects[2],
        TailEffectAt {
            index: settled + 2,
            effect: TailEffect::Revoked { grant: chain.root },
        }
    );
    assert_eq!((authority.tail_len, authority.lineage_visits), (3, 3));
    Ok(())
}

#[test]
fn a_tail_issue_names_its_source_and_a_later_spend_of_it_is_classified() -> TestResult {
    let mut world = World::new()?;
    let chain = chain(&mut world)?;
    let provider = file_provider(&world)?;
    witnessed(&mut world, Arc::new(provider))?;
    let settled = world.grants.revision();
    let request = world.request(
        IdentityId::Person(world.dana),
        chain.once_root,
        IdentityId::Person(world.tom),
        "tern",
        PassOn::UseOnly,
        None,
    )?;
    let path = world.dir.path().join("grants");
    let mut elsewhere = Grants::open(
        Box::new(move || FileLeafStore::open(&path)),
        Ed25519Identity::load(&world.dir.path().join("service.key"))?,
        MemoryRelationships::default(),
        world.grants.model().clone(),
        world.admin,
    )?;
    let directory = world.directory.projection()?;
    let issued = elsewhere
        .delegate_once(directory, &request, world.now)?
        .event
        .grant();
    let exercise = ExerciseRequest {
        caller: IdentityId::Person(world.tom),
        route: Route::Api,
        resource: alpha()?,
        action: Action::new("read")?,
    };
    let spent = elsewhere.check_by(directory, &exercise, Some(issued), world.now, None)?;
    drop(elsewhere);
    assert_eq!(spent.use_event, Some(Ok(settled + 1)));
    let witness = world.grants.ledger().acquire_tail()?;
    let authority = world.grants.tail_authority(&witness, chain.second)?;
    assert_eq!(
        authority.effects,
        vec![
            TailEffectAt {
                index: settled,
                effect: TailEffect::Issued {
                    grant: issued,
                    source: Source::Grant(chain.once_root),
                },
            },
            TailEffectAt {
                index: settled + 1,
                effect: TailEffect::Spent { grant: issued },
            },
        ]
    );
    assert_eq!(authority.bearing, TailBearing::Disjoint);
    Ok(())
}
