#![cfg(test)]
//! An agent's use of an act a person may now give it is noted in the book as
//! it is applied, kept by the snapshot, and read without the log (AGENTS-019
//! R5); the use event naming the act is version 8 and reads back whole.

use std::error::Error;

use super::{ACTED_FOR_THEIR_PERSON, Acted};
use crate::grants::events::{
    EXERCISED_ENVELOPE, Exercised, decode_event_body, encode_event_body, sign_grant_event,
    verify_grant_event,
};
use crate::grants::projection::GrantBook;
use crate::grants::{
    AGENTS_NOT_MACHINES, Action, Grant, GrantChange, GrantEvent, GrantId, GrantParts, PassOn,
    Relation, Resource, Route, Source, WITHHELD_FROM_AGENTS, Window,
};
use crate::{AgentId, IdentityId, OperationId, PersonId};

type TestResult = Result<(), Box<dyn Error>>;

const PERSON: [u8; 16] = [0xd1; 16];
const AGENT: [u8; 16] = [0xa7; 16];
const GRANT: [u8; 16] = [0x61; 16];

fn agent() -> IdentityId {
    IdentityId::Agent(AgentId::from_bytes(AGENT))
}

/// A root grant to the agent of `actions` on Bea's account.
fn issued(actions: &[&str]) -> Result<GrantEvent, Box<dyn Error>> {
    let person = PersonId::from_bytes(PERSON);
    let operation = OperationId::from_bytes([1; 16]);
    let grant = Grant::new(GrantParts {
        id: GrantId::from_bytes(GRANT),
        issuer: IdentityId::Person(person),
        holder: agent(),
        responsible: person,
        resource: Resource::new("account", "bea")?,
        relation: Relation::new("helper")?,
        actions: actions
            .iter()
            .copied()
            .map(Action::new)
            .collect::<Result<_, _>>()?,
        pass_on: PassOn::UseOnly,
        source: Source::Root,
        window: Window::new(0, None)?,
        model_version: 1,
        operation,
    })?;
    Ok(GrantEvent::new(
        operation,
        IdentityId::Person(person),
        1,
        GrantChange::Issue(Box::new(grant)),
    )?)
}

/// `caller`'s use of the grant at `at`, naming `action` on Bea's account.
fn used(caller: IdentityId, seed: u8, at: u64, action: &str) -> Result<GrantEvent, Box<dyn Error>> {
    Ok(GrantEvent::new(
        OperationId::from_bytes([seed; 16]),
        caller,
        at,
        GrantChange::Use {
            grant: GrantId::from_bytes(GRANT),
            route: Route::Api,
        },
    )?
    .with_exercised(Exercised {
        action: Action::new(action)?,
        resource: Resource::new("account", "bea")?,
    })?)
}

#[test]
fn a_use_naming_its_act_is_version_eight_and_reads_back_whole() -> TestResult {
    let named = used(agent(), 2, 50, "person.email.set")?;
    assert_eq!(named.version(), 8);
    assert_eq!(decode_event_body(&encode_event_body(&named))?, named);
    let home = tempfile::tempdir()?;
    let key = lys_core::Ed25519Identity::load_or_generate(&home.path().join("key"))?;
    let signed = sign_grant_event(named.clone(), &key)?;
    assert!(
        signed
            .bytes()
            .windows(EXERCISED_ENVELOPE.len())
            .any(|window| window == EXERCISED_ENVELOPE),
        "the protected header names the version 8 content type"
    );
    assert_eq!(
        verify_grant_event(signed.bytes(), &key.public_key_bytes())?.event(),
        &named
    );
    // A use naming nothing keeps version 1 and its bytes.
    let bare = GrantEvent::new(
        OperationId::from_bytes([3; 16]),
        agent(),
        50,
        GrantChange::Use {
            grant: GrantId::from_bytes(GRANT),
            route: Route::Api,
        },
    )?;
    assert_eq!(bare.version(), 1);
    assert_eq!(bare.exercised(), None);
    assert_eq!(decode_event_body(&encode_event_body(&bare))?, bare);
    // Only a use names what it exercised.
    assert!(
        issued(&["read"])?
            .with_exercised(Exercised {
                action: Action::new("read")?,
                resource: Resource::new("account", "bea")?,
            })
            .is_err()
    );
    Ok(())
}

#[test]
fn the_book_notes_an_agents_moved_acts_and_the_snapshot_keeps_them() -> TestResult {
    let mut book = GrantBook::new();
    book.apply(&issued(&["read", "person.email.set"])?, 0)?;
    book.apply(&used(agent(), 2, 10, "read")?, 1)?;
    book.apply(&used(agent(), 3, 20, "person.email.set")?, 2)?;
    book.apply(&used(agent(), 4, 30, "person.email.set")?, 3)?;
    let agent_id = AgentId::from_bytes(AGENT);
    let noted = book.acted_by(agent_id);
    // A read is never noted; each email change is, oldest first.
    assert_eq!(
        noted
            .iter()
            .map(|one| (one.index, one.at))
            .collect::<Vec<_>>(),
        [(2, 20), (3, 30)]
    );
    assert_eq!(noted[0].exercised.action.as_str(), "person.email.set");
    assert_eq!(noted[0].exercised.resource.to_string(), "account:bea");
    assert_eq!(noted[0].grant, GrantId::from_bytes(GRANT));
    assert!(book.acted_by(AgentId::from_bytes([0x0b; 16])).is_empty());

    // The state names version 7 and reopens to the same book, notes and all.
    let state = crate::grants::state::encode(&book, 4)?;
    let ciborium::Value::Array(members) = ciborium::from_reader(state.as_slice())? else {
        return Err("a grant state is an array".into());
    };
    assert_eq!(members.first(), Some(&ciborium::Value::from(7_u64)));
    let reopened = crate::grants::state::decode(&state, 4)?;
    assert_eq!(reopened, book);
    assert_eq!(reopened.acted_by(agent_id), noted);
    Ok(())
}

#[test]
fn a_book_with_no_notes_writes_its_state_as_before() -> TestResult {
    let mut book = GrantBook::new();
    book.apply(&issued(&["read"])?, 0)?;
    book.apply(&used(agent(), 2, 10, "read")?, 1)?;
    let state = crate::grants::state::encode(&book, 2)?;
    let ciborium::Value::Array(members) = ciborium::from_reader(state.as_slice())? else {
        return Err("a grant state is an array".into());
    };
    assert_eq!(members.first(), Some(&ciborium::Value::from(3_u64)));
    assert_eq!(crate::grants::state::decode(&state, 2)?, book);
    assert_eq!(
        Acted::default().encode(),
        ciborium::Value::Array(Vec::new())
    );
    Ok(())
}

#[test]
fn the_listed_acts_are_the_ones_r1_moved() {
    let mut sorted = ACTED_FOR_THEIR_PERSON.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted, ACTED_FOR_THEIR_PERSON, "sorted, each once");
    assert_eq!(ACTED_FOR_THEIR_PERSON.len(), 29);
    for act in ACTED_FOR_THEIR_PERSON {
        assert!(
            AGENTS_NOT_MACHINES.contains(act),
            "{act} is an agent's, never a machine's"
        );
        assert!(
            !WITHHELD_FROM_AGENTS.contains(act),
            "{act} may be given to an agent"
        );
    }
}
