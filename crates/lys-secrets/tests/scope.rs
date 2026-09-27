//! Scopes: a personal secret belongs to its person, whatever grants exist;
//! another person, or that person's agent, can neither discover, read, be
//! issued nor be lent it, and knowing its name grants nothing. A team
//! secret is open to the team's members and to no one else.

use lys_core::Ed25519Identity;
use lys_secrets::{
    Broker, BrokerPaths, EntryClass, Holder, IssuedHandle, LocalGrants, Presentation, Relation,
    Scope, Secret, SecretRelation, SecretsError, new_operation_id,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn refusal<T>(result: Result<T, SecretsError>) -> &'static str {
    match result {
        Ok(_) => "admitted",
        Err(error) => error.name(),
    }
}

fn relation(identity: &str, secret: &str, by: &str) -> SecretRelation {
    SecretRelation {
        identity: identity.to_owned(),
        secret: secret.to_owned(),
        granted_by: Some(by.to_owned()),
    }
}

struct Party {
    key: Ed25519Identity,
    holder: Holder,
}

fn party(dir: &std::path::Path, identity: &str) -> Result<Party, Box<dyn std::error::Error>> {
    let key = Ed25519Identity::load_or_generate(&dir.join(format!("{identity}.key")))?;
    let holder = Holder {
        identity: identity.to_owned(),
        key: key.public_key_bytes(),
    };
    Ok(Party { key, holder })
}

fn sign(issued: &IssuedHandle, party: &Party) -> Result<Presentation, SecretsError> {
    Presentation::sign(&issued.id, &new_operation_id()?, 1_000, [1; 32], &party.key)
}

fn broker(root: &std::path::Path) -> Result<Broker<LocalGrants>, Box<dyn std::error::Error>> {
    let keys = root.join("keys");
    std::fs::create_dir_all(&keys)?;
    let paths = BrokerPaths {
        store_dir: root.join("store"),
        log_dir: root.join("log"),
        store_key: keys.join("store.key"),
        audit_key: keys.join("audit.key"),
        anchor: keys.join("audit.anchor"),
    };
    Ok(Broker::create(
        &paths,
        LocalGrants::new(),
        Box::new(|| 1_000),
    )?)
}

fn names(broker: &Broker<LocalGrants>, identity: &str) -> Vec<String> {
    broker
        .listing(identity)
        .into_iter()
        .map(|entry| entry.name)
        .collect()
}

#[test]
fn a_personal_secret_is_closed_to_every_other_person_and_their_agents() -> TestResult {
    let root = tempfile::tempdir()?;
    let keys = root.path().join("keys");
    let mut broker = broker(root.path())?;
    let people = [("tom", "dana"), ("dana", "tom")];
    for (person, _other) in people {
        let owner = format!("person:{person}");
        broker.seal(
            &format!("{person}-token"),
            &owner,
            &Secret::from_slice(b"value"),
        )?;
        broker.seal_record(
            &format!("{person}-notes"),
            EntryClass::Memory,
            &owner,
            &Secret::from_slice(b"private"),
        )?;
        let personal = Scope::parse(&format!("personal:{owner}"))?;
        broker.set_scope(&owner, &format!("{person}-token"), personal.clone())?;
        broker.set_scope(&owner, &format!("{person}-notes"), personal)?;
    }
    let grants = broker.permissions();
    for identity in [
        "person:tom",
        "person:dana",
        "agent:tom-bot",
        "agent:dana-bot",
    ] {
        for secret in ["tom-token", "dana-token"] {
            grants.grant(relation(identity, secret, "person:tom"));
        }
        for record in ["tom-notes", "dana-notes"] {
            grants.grant_as(Relation::Read, relation(identity, record, "person:tom"));
        }
    }
    for person in ["tom", "dana"] {
        grants.grant_as(
            Relation::Member,
            relation(
                &format!("agent:{person}-bot"),
                &format!("person/person:{person}"),
                &format!("person:{person}"),
            ),
        );
    }

    for (person, other) in people {
        let token = format!("{person}-token");
        let notes = format!("{person}-notes");
        let owner = party(&keys, &format!("person:{person}"))?;
        let own_bot = party(&keys, &format!("agent:{person}-bot"))?;
        let own = broker.issue(&owner.holder, &token, 5, 50_000)?;
        assert_eq!(
            names(&broker, &format!("agent:{person}-bot")),
            vec![notes.clone(), token.clone()]
        );
        assert_eq!(
            broker
                .read_record(&format!("agent:{person}-bot"), &notes, None)?
                .len(),
            7
        );
        broker.derive(
            &own.token,
            &sign(&own, &owner)?,
            &own_bot.holder,
            (1, 40_000),
            None,
        )?;

        for outsider in [format!("person:{other}"), format!("agent:{other}-bot")] {
            let stranger = party(&keys, &outsider)?;
            assert!(!names(&broker, &outsider).contains(&token));
            assert!(!names(&broker, &outsider).contains(&notes));
            assert_eq!(refusal(broker.metadata(&outsider, &token)), "SecretUnknown");
            assert_eq!(
                refusal(broker.metadata(&outsider, &token)),
                refusal(broker.metadata(&outsider, "never-sealed"))
            );
            assert_eq!(
                refusal(broker.read_record(&outsider, &notes, None)),
                "NotFound"
            );
            assert_eq!(
                refusal(broker.issue(&stranger.holder, &token, 1, 50_000)),
                "SecretUnknown"
            );
            assert_eq!(
                refusal(broker.derive(
                    &own.token,
                    &sign(&own, &owner)?,
                    &stranger.holder,
                    (1, 40_000),
                    None
                )),
                "PermissionDenied"
            );
        }
    }
    Ok(())
}

#[test]
fn leaving_a_scope_ends_the_use_and_a_team_secret_is_open_to_its_members_only() -> TestResult {
    let root = tempfile::tempdir()?;
    let keys = root.path().join("keys");
    let mut broker = broker(root.path())?;
    broker.seal("ledger", "person:dana", &Secret::from_slice(b"value"))?;
    broker.set_scope("person:dana", "ledger", Scope::parse("team:accounts")?)?;
    assert_eq!(
        refusal(broker.set_scope("person:tom", "ledger", Scope::parse("personal:person:tom")?)),
        "LendingNotPermitted"
    );
    assert_eq!(refusal(Scope::parse("guild:x")), "InvalidScope");
    let grants = broker.permissions();
    for identity in ["person:tom", "agent:tom-bot"] {
        grants.grant(relation(identity, "ledger", "person:dana"));
    }
    grants.grant_as(
        Relation::Member,
        relation("person:tom", "team/accounts", "person:dana"),
    );
    let tom = party(&keys, "person:tom")?;
    let tom_bot = party(&keys, "agent:tom-bot")?;

    broker.add_account("ledger", "spare", &Secret::from_slice(b"second"))?;
    let both = vec!["ledger".to_owned(), "ledger@spare".to_owned()];
    assert_eq!(names(&broker, "person:tom"), both);
    assert_eq!(names(&broker, "person:dana"), both);
    assert!(names(&broker, "agent:tom-bot").is_empty());
    assert_eq!(
        refusal(broker.issue(&tom_bot.holder, "ledger", 1, 50_000)),
        "SecretUnknown"
    );

    let handle = broker.issue(&tom.holder, "ledger", 5, 50_000)?;
    broker.use_handle(&handle.token, &sign(&handle, &tom)?, Secret::len)?;
    broker
        .permissions()
        .revoke_as(Relation::Member, "person:tom", "team/accounts");
    assert_eq!(
        refusal(broker.use_handle(&handle.token, &sign(&handle, &tom)?, Secret::len)),
        "PermissionDenied"
    );
    assert!(names(&broker, "person:tom").is_empty());
    Ok(())
}
