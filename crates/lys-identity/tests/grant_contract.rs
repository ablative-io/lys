//! R1: the grant contract, its model and its wire boundary
//! (`GRANT_CONTRACT`, `GRANT_MODEL` and `GRANT_WIRE_BOUNDARY`).

use std::collections::BTreeSet;
use std::error::Error;

use ciborium::Value;
use lys_identity::grants::{
    Action, GRANT_ENVELOPE, Grant, GrantError, GrantId, GrantParts, MEMBERS, Model, PassOn,
    RecipientKind, Relation, Resource, Source, Window, decode_grant, encode_grant,
};
use lys_identity::{AgentId, IdentityId, OperationId, PersonId};

type TestResult = Result<(), Box<dyn Error>>;

fn actions(names: &[&str]) -> Result<BTreeSet<Action>, GrantError> {
    names.iter().map(|name| Action::new(name)).collect()
}

const DANA: [u8; 16] = [0xd1; 16];
const AGENT: [u8; 16] = [0xa7; 16];
const SOURCE: [u8; 16] = [0x51; 16];
const ID: [u8; 16] = [0x61; 16];
const OPERATION: [u8; 16] = [0x0e; 16];

/// A derived grant carrying every member: a source, pass-on to both kinds and an end.
fn derived() -> Result<Grant, GrantError> {
    Grant::new(GrantParts {
        id: GrantId::from_bytes(ID),
        issuer: IdentityId::Person(PersonId::from_bytes(DANA)),
        holder: IdentityId::Agent(AgentId::from_bytes(AGENT)),
        responsible: PersonId::from_bytes(DANA),
        resource: Resource::new("project", "alpha")?,
        relation: Relation::new("heron")?,
        actions: actions(&["read", "write"])?,
        pass_on: PassOn::to(
            actions(&["read"])?,
            [RecipientKind::Person, RecipientKind::Agent].into(),
        )?,
        source: Source::Grant(GrantId::from_bytes(SOURCE)),
        window: Window::new(1_000, Some(2_000))?,
        model_version: 3,
        operation: OperationId::from_bytes(OPERATION),
    })
}

/// A root grant: held by its responsible person, use-only, with no end of its own.
fn root() -> Result<Grant, GrantError> {
    let mut parts = derived()?.parts().clone();
    parts.holder = IdentityId::Person(PersonId::from_bytes(DANA));
    parts.pass_on = PassOn::UseOnly;
    parts.source = Source::Root;
    parts.window = Window::new(1_000, None)?;
    Grant::new(parts)
}

fn int(value: u64) -> Value {
    Value::Integer(value.into())
}

fn identity(kind: u64, id: [u8; 16]) -> Value {
    Value::Map(vec![
        (int(1), int(kind)),
        (int(2), Value::Bytes(id.to_vec())),
    ])
}

fn texts(names: &[&str]) -> Value {
    Value::Array(
        names
            .iter()
            .map(|name| Value::Text((*name).to_owned()))
            .collect(),
    )
}

/// The derived grant's members, written from the contract document's table
/// with ciborium, independently of the crate's encoder.
fn fixture() -> Vec<(Value, Value)> {
    vec![
        (int(1), Value::Bytes(ID.to_vec())),
        (int(2), identity(1, DANA)),
        (int(3), identity(2, AGENT)),
        (int(4), Value::Bytes(DANA.to_vec())),
        (
            int(5),
            Value::Map(vec![
                (int(1), Value::Text("project".to_owned())),
                (int(2), Value::Text("alpha".to_owned())),
            ]),
        ),
        (int(6), Value::Text("heron".to_owned())),
        (int(7), texts(&["read", "write"])),
        (
            int(8),
            Value::Map(vec![
                (int(1), texts(&["read"])),
                (int(2), Value::Array(vec![int(1), int(2)])),
            ]),
        ),
        (int(9), Value::Bytes(SOURCE.to_vec())),
        (
            int(10),
            Value::Map(vec![(int(1), int(1_000)), (int(2), int(2_000))]),
        ),
        (int(11), int(3)),
        (int(12), Value::Bytes(OPERATION.to_vec())),
    ]
}

fn bytes_of(pairs: Vec<(Value, Value)>) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut out = Vec::new();
    ciborium::into_writer(&Value::Map(pairs), &mut out)?;
    Ok(out)
}

/// The fixture with member `key` replaced by `value`.
fn with(key: u64, value: &Value) -> Result<Vec<u8>, Box<dyn Error>> {
    let pairs = fixture()
        .into_iter()
        .map(|(k, v)| {
            if k == int(key) {
                (k, value.clone())
            } else {
                (k, v)
            }
        })
        .collect();
    bytes_of(pairs)
}

#[test]
fn grant_contract_every_member_round_trips_and_matches_the_fixture() -> TestResult {
    let derived = derived()?;
    let encoded = encode_grant(&derived);
    assert_eq!(encoded, bytes_of(fixture())?, "the fixture's bytes");
    assert_eq!(decode_grant(&encoded)?, derived);
    let root = root()?;
    let mut root_fixture = fixture();
    root_fixture[2].1 = identity(1, DANA);
    root_fixture[7].1 = int(0);
    root_fixture[8].1 = int(0);
    root_fixture[9].1 = Value::Map(vec![(int(1), int(1_000)), (int(2), Value::Null)]);
    assert_eq!(encode_grant(&root), bytes_of(root_fixture)?);
    assert_eq!(decode_grant(&encode_grant(&root))?, root);
    Ok(())
}

#[test]
fn grant_contract_refuses_each_case_by_name() -> TestResult {
    let mut cases = 0;
    let mut refused = |bytes: &[u8], expected: &GrantError| {
        cases += 1;
        assert_eq!(decode_grant(bytes).as_ref(), Err(expected), "case {cases}");
    };
    for key in [0, 14, 40] {
        let mut pairs = fixture();
        pairs.push((int(key), int(1)));
        refused(&bytes_of(pairs)?, &GrantError::MemberUnknown { key });
    }
    let mut pairs = fixture();
    pairs.push((int(13), int(1)));
    refused(
        &bytes_of(pairs)?,
        &GrantError::GrantMalformed {
            reason: "a one-time grant names key 13 once, as true",
        },
    );
    for (key, member) in (1..).zip(MEMBERS) {
        let pairs = fixture()
            .into_iter()
            .filter(|(k, _)| *k != int(key))
            .collect();
        refused(&bytes_of(pairs)?, &GrantError::MemberMissing { member });
    }
    refused(
        &with(7, &texts(&[]))?,
        &GrantError::AuthorityAbsent { member: "actions" },
    );
    let empty_pass_on =
        |actions: Value, kinds: Value| Value::Map(vec![(int(1), actions), (int(2), kinds)]);
    refused(
        &with(8, &empty_pass_on(texts(&[]), Value::Array(vec![int(1)])))?,
        &GrantError::AuthorityAbsent {
            member: "pass-on actions",
        },
    );
    refused(
        &with(8, &empty_pass_on(texts(&["read"]), Value::Array(vec![])))?,
        &GrantError::AuthorityAbsent {
            member: "pass-on recipients",
        },
    );
    refused(
        &with(
            8,
            &empty_pass_on(texts(&["delete"]), Value::Array(vec![int(2)])),
        )?,
        &GrantError::PassOnOutside,
    );
    for code in [0, 4, 99] {
        refused(
            &with(
                8,
                &empty_pass_on(texts(&["read"]), Value::Array(vec![int(code)])),
            )?,
            &GrantError::RecipientKindUnknown { code },
        );
    }
    let root_held_by_agent = with(9, &int(0))?;
    refused(
        &root_held_by_agent,
        &GrantError::LineageMalformed {
            reason: "a root grant is held by its responsible person",
        },
    );
    let mut pairs = fixture();
    pairs[2].1 = identity(1, [0xee; 16]);
    pairs[8].1 = int(0);
    refused(
        &bytes_of(pairs)?,
        &GrantError::LineageMalformed {
            reason: "a root grant is held by its responsible person",
        },
    );
    refused(
        &with(9, &Value::Bytes(ID.to_vec()))?,
        &GrantError::LineageMalformed {
            reason: "a grant does not derive from itself",
        },
    );
    refused(
        &with(9, &Value::Text("root".to_owned()))?,
        &GrantError::LineageMalformed {
            reason: "a source is 0 for a root or a 16-byte grant id",
        },
    );
    refused(
        &with(9, &int(1))?,
        &GrantError::LineageMalformed {
            reason: "a source is 0 for a root or a 16-byte grant id",
        },
    );
    let mut reordered = fixture();
    reordered.swap(0, 1);
    refused(&bytes_of(reordered)?, &GrantError::GrantNotCanonical);
    assert_eq!(
        cases,
        3 + 1 + 12 + 4 + 3 + 5 + 1,
        "every refusal case was counted"
    );
    Ok(())
}

#[test]
fn row_2_2_grant_contract_supplies_no_permission_by_default() -> TestResult {
    let null_pass_on = with(8, &Value::Null)?;
    assert!(matches!(
        decode_grant(&null_pass_on),
        Err(GrantError::GrantMalformed { .. })
    ));
    let other_code = with(8, &int(1))?;
    assert!(matches!(
        decode_grant(&other_code),
        Err(GrantError::GrantMalformed { .. })
    ));
    let absent_end = with(10, &Value::Map(vec![(int(1), int(1_000))]))?;
    assert!(matches!(
        decode_grant(&absent_end),
        Err(GrantError::GrantMalformed { .. })
    ));
    Ok(())
}

fn relation(name: &str) -> Result<Relation, GrantError> {
    Relation::new(name)
}

#[test]
fn row_2_1_grant_model_judges_by_action_sets_and_keeps_its_version() -> TestResult {
    let model = Model::new(
        4,
        [
            (
                relation("aardvark")?,
                actions(&["delete", "read", "write"])?,
            ),
            (relation("zebra")?, actions(&["read"])?),
            (relation("mole")?, actions(&["share", "write"])?),
        ],
    )?;
    let broad = model.actions(&relation("aardvark")?)?.clone();
    let narrow = model.actions(&relation("zebra")?)?.clone();
    let within = model.within(&relation("zebra")?, &broad)?;
    assert_eq!((within.actions, within.model_version), (narrow.clone(), 4));
    assert_eq!(
        model.within(&relation("aardvark")?, &narrow),
        Err(GrantError::ActionsOutside {
            relation: "aardvark".to_owned(),
            outside: "delete, write".to_owned(),
            model_version: 4,
        }),
        "a name earlier in the alphabet is not a lower rank"
    );
    assert_eq!(
        model.within(&relation("mole")?, &broad),
        Err(GrantError::ActionsOutside {
            relation: "mole".to_owned(),
            outside: "share".to_owned(),
            model_version: 4,
        }),
        "overlapping sets are not ranked"
    );
    assert_eq!(
        model.within(&relation("owner")?, &broad),
        Err(GrantError::RelationUnknown {
            relation: "owner".to_owned(),
            model_version: 4
        }),
        "a label outside the model carries no authority"
    );
    let next = Model::new(5, [(relation("zebra")?, actions(&["read", "write"])?)])?;
    assert_eq!(next.within(&relation("zebra")?, &broad)?.model_version, 5);
    assert_eq!(
        Model::new(0, [(relation("zebra")?, narrow)]),
        Err(GrantError::ModelInvalid {
            reason: "a model version is 1 or more"
        })
    );
    assert_eq!(
        Model::new(1, [(relation("zebra")?, BTreeSet::new())]),
        Err(GrantError::ModelInvalid {
            reason: "every relation carries at least one action"
        })
    );
    Ok(())
}

#[test]
fn grant_wire_boundary_names_its_own_envelope_and_reads_no_other_format() -> TestResult {
    assert_eq!(GRANT_ENVELOPE, "application/vnd.lys.grant-event.v1+cbor");
    assert_ne!(GRANT_ENVELOPE, lys_identity::signer::CONTENT_TYPE);
    assert_ne!(GRANT_ENVELOPE, "application/vnd.lys.delegation.v1+cbor");
    let contract = include_str!("../../../docs/design/identity/GRANT-CONTRACT.md");
    assert!(
        contract.contains(GRANT_ENVELOPE),
        "the contract document names the envelope"
    );
    let protected = Value::Map(vec![
        (int(1), Value::Integer((-8).into())),
        (
            int(3),
            Value::Text("application/vnd.lys.delegation.v1+cbor".to_owned()),
        ),
        (int(4), Value::Bytes(vec![0x03; 32])),
    ]);
    let mut protected_bytes = Vec::new();
    ciborium::into_writer(&protected, &mut protected_bytes)?;
    let message = Value::Tag(
        18,
        Box::new(Value::Array(vec![
            Value::Bytes(protected_bytes),
            Value::Map(vec![]),
            Value::Bytes(bytes_of(fixture())?),
            Value::Bytes(vec![0; 64]),
        ])),
    );
    let mut delegation = Vec::new();
    ciborium::into_writer(&message, &mut delegation)?;
    assert!(
        matches!(
            decode_grant(&delegation),
            Err(GrantError::GrantMalformed { .. })
        ),
        "a lys/delegation/v1 message, even one carrying a grant's bytes, is never read as a grant"
    );
    Ok(())
}
