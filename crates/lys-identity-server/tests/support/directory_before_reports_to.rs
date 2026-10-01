//! A frozen directory snapshot writer keeps the upgrade proof independent of new fields.

use std::error::Error;

use ciborium::Value;
use identity_contract::harness::ADMINISTRATOR;
use lys_identity::event::{Change, IdentityEvent};
use lys_identity::projection::{Projection, Record};
use lys_identity::signer::{load_service_key, sign_event};
use lys_identity::{
    Actor, AgentId, AuthMethod, IdentityId, LifecycleState, LoginBinding, OperationId, PersonId,
    Profile, Provenance,
};
use lys_identity_server::config::Config;
use lys_log_store::{FileLeafStore, FrontierLog};

/// The revision whose snapshot encoding is frozen here.
pub const SOURCE_COMMIT: &str = "33b80e3ada3667471af846701f222397f6297919";
const STATE_VERSION: u64 = 2;
const DOMAIN: &str = "lys/identity-directory/v1";

/// The identities written in the old directory format.
pub struct Legacy {
    /// The active person responsible for both agents.
    pub owner: PersonId,
    /// Every agent in the fixture.
    pub agents: Vec<AgentId>,
}

fn uint(value: u64) -> Value {
    Value::Integer(value.into())
}

fn bytes(value: &[u8]) -> Value {
    Value::Bytes(value.to_vec())
}

fn text(value: &str) -> Value {
    Value::Text(value.to_owned())
}

fn array(items: Vec<Value>) -> Value {
    Value::Array(items)
}

fn binding(value: &LoginBinding) -> Value {
    array(vec![text(value.issuer()), text(value.subject())])
}

fn identity(value: IdentityId) -> Value {
    let (kind, id) = match value {
        IdentityId::Person(id) => (1, *id.as_bytes()),
        IdentityId::Agent(id) => (2, *id.as_bytes()),
        IdentityId::ServiceAccount(id) => (3, *id.as_bytes()),
    };
    array(vec![uint(kind), bytes(&id)])
}

fn record(id: IdentityId, held: &Record) -> Value {
    let state = match held.state() {
        LifecycleState::Registered => 1,
        LifecycleState::Active => 2,
        LifecycleState::Suspended => 3,
        LifecycleState::Retired => 4,
    };
    array(vec![
        identity(id),
        text(held.profile().display_name()),
        uint(state),
        held.responsible()
            .map_or(Value::Null, |person| bytes(person.as_bytes())),
        array(held.bindings().iter().map(binding).collect()),
        binding(held.registered_by()),
        array(held.events().iter().map(|index| uint(*index)).collect()),
    ])
}

fn snapshot(
    projection: &Projection,
    operations: &mut [(OperationId, u64)],
    folded: u64,
) -> Result<Vec<u8>, Box<dyn Error>> {
    operations.sort_by_key(|operation| operation.0);
    let projection = array(vec![
        array(
            projection
                .records()
                .map(|(id, held)| record(*id, held))
                .collect(),
        ),
        array(
            operations
                .iter()
                .map(|(operation, index)| array(vec![bytes(operation.as_bytes()), uint(*index)]))
                .collect(),
        ),
        array(Vec::new()),
    ]);
    let state = array(vec![uint(STATE_VERSION), uint(folded), projection]);
    let mut owner = Vec::new();
    ciborium::into_writer(&state, &mut owner)?;
    // The fixture has fewer leaves than the first checkpoint interval, so
    // its only checkpoint is the empty frontier at zero.
    if folded >= 1024 {
        return Err("legacy fixture exceeds its frozen checkpoint interval".into());
    }
    let wrapped = array(vec![uint(1), array(vec![bytes(&[])]), bytes(&owner)]);
    let mut encoded = Vec::new();
    ciborium::into_writer(&wrapped, &mut encoded)?;
    Ok(encoded)
}

/// Write old registration events and seal their version-two snapshot.
pub fn write(config: &Config) -> Result<Legacy, Box<dyn Error>> {
    let key = load_service_key(&config.event_key_file)?;
    FileLeafStore::create(&config.log_dir, &config.log_origin)?;
    let (mut log, _) = FrontierLog::open(FileLeafStore::open(&config.log_dir)?)?;
    let actor = Actor::new(
        LoginBinding::new(&config.issuer, ADMINISTRATOR)?,
        Provenance::new(AuthMethod::Oidc, 1),
    );
    let owner = PersonId::generate()?;
    let agents = vec![AgentId::generate()?, AgentId::generate()?];
    let mut changes = vec![(
        IdentityId::Person(owner),
        Change::SetupPerson {
            profile: Profile::new("Earlier owner")?,
        },
    )];
    for (index, agent) in agents.iter().enumerate() {
        changes.push((
            IdentityId::Agent(*agent),
            Change::RegisterAgent {
                responsible: owner,
                profile: Profile::new(&format!("Earlier agent {index}"))?,
            },
        ));
    }
    let mut projection = Projection::new();
    let mut operations = Vec::new();
    for (index, (id, change)) in (0_u64..).zip(changes) {
        let operation = OperationId::generate()?;
        let event = IdentityEvent::new(operation, actor.clone(), id, 1, change)?;
        projection.check(&event)?;
        let signed = sign_event(event.clone(), &key)?;
        log.append(signed.bytes())?;
        projection.apply(&event, index)?;
        operations.push((operation, index));
    }
    let folded = u64::try_from(operations.len())?;
    log.write_snapshot(
        DOMAIN,
        &snapshot(&projection, &mut operations, folded)?,
        &key,
    )?;
    Ok(Legacy { owner, agents })
}
