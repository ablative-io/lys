//! A bench question answered by the real check against a scratch copy of the
//! draft, in a throwaway namespace of the permission service.
//!
//! The namespace holds memory-only apps and grant logs with ephemeral signing
//! keys made for it alone, and the
//! permission engine the service runs its grants on: when `SpiceDB` is
//! configured, a scratch scope of it (`spicedb_scope`) that the draft's
//! kinds, the example placements and the example grants are written into,
//! and otherwise the in-process engine. Nothing in it is the service's, and
//! no standing check reads it. The draft is loaded into it as the one approved
//! app beside the app `lys`, whose schema is the service's own model, so a
//! kind of any other app is refused there exactly as the service refuses an
//! unregistered kind. The example people and agents are active people of a
//! scratch directory, the example holdings root grants issued by its root,
//! and the example placements placements of the draft's kinds.
//!
//! The question is then asked through `grants_batch::one`, the decision the
//! batch route makes for every check: the kind and the action admitted by
//! the apps, and the grants' `decide` reaching from the resource to each
//! parent it is placed in. The answer is that decision's, the path read back
//! from the grant it names. Its scratch engine scope is removed before the
//! answer is given; cleanup failures are refused by name.

use std::collections::BTreeMap;
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_identity::grants::{
    AppSchema, GrantChange, Grants, MemoryRelationships, Model, PassOn, Relation, Resource,
    RootRequest, Route, Window,
};
use lys_identity::projection::Projection;
use lys_identity::{
    Actor, AuthMethod, Change, IdentityEvent, IdentityId, LoginBinding, OperationId, PersonId,
    Profile, Provenance,
};
use serde_json::Value;

use crate::apps_bench::{BenchAnswer, Example, Holding, Placement, Question};
use crate::apps_binding::new_secret;
use crate::apps_error::AppError;
use crate::apps_state::{Approved, By, Client, Line, LysRecorded, Placed, Registered};
use crate::apps_store::AppStore;
use crate::error::ServerError;
use crate::grants::Judged;
use crate::grants_batch::{CheckWire, one};
use crate::session::now;
use crate::spicedb::scope::scope_for;
use crate::spicedb::{Relationships, SpiceDb, SpiceDbConnection};

/// The issuer the scratch directory's example logins are bound at. It names
/// no real issuer and no one signs in through it.
const EXAMPLE_ISSUER: &str = "https://bench.lys.invalid";

#[path = "apps_bench_memory.rs"]
pub(crate) mod memory;
use memory::MemoryStore;
type ScratchGrants = Grants<MemoryStore, Relationships>;

/// The origin the scratch grant log is created with.
const GRANT_ORIGIN: &str = "lys/identity/bench/grants";

/// What one question is asked with: the draft and who opened the bench.
pub struct Draft<'a> {
    /// The app the draft is of.
    pub app: &'a str,
    /// The draft as it was sent, already checked.
    pub schema: &'a Value,
    /// Who opened the bench.
    pub by: &'a By,
    /// Lys's own model, the app `lys`'s schema in the namespace.
    pub lys: &'a Model,
    /// The permission engine the service runs its grants on, when it is
    /// `SpiceDB`; none for the in-process engine.
    pub engine: Option<&'a SpiceDbConnection>,
}

/// The examples one question is asked over.
pub struct Examples<'a> {
    /// Who holds which relation on which resource.
    pub holdings: &'a [Holding],
    /// Which resource is placed in which parent.
    pub placements: &'a [Placement],
    /// The question.
    pub question: &'a Question,
}

fn unavailable(what: impl std::fmt::Display) -> ServerError {
    AppError::AppsUnavailable {
        reason: format!("the bench's scratch namespace: {what}"),
    }
    .into()
}

fn operation() -> Result<String, ServerError> {
    Ok(OperationId::generate()?.to_string())
}

fn resource(example: &Example) -> Result<Resource, ServerError> {
    Ok(Resource::new(&example.kind, &example.id)?)
}

/// Answer the question in process-only stores and remove its engine scope
/// before returning. No scratch key, leaf or snapshot reaches disk.
pub fn answer_in(draft: &Draft<'_>, examples: &Examples<'_>) -> Result<BenchAnswer, ServerError> {
    let (random, _) = new_secret()?;
    let scope = scope_for(&random);
    let answered = answer(draft, examples, &scope);
    let scratch = draft
        .engine
        .map_or(Ok(()), |settings| SpiceDb::remove_scratch(settings, &scope));
    scratch.map_err(unavailable)?;
    answered
}

/// The scratch apps store: the app `lys` with the service's own model, and
/// the draft approved.
fn apps(draft: &Draft<'_>, at: u64) -> Result<AppStore<MemoryStore>, ServerError> {
    let key = Arc::new(Ed25519Identity::ephemeral());
    let leaves = MemoryStore::empty();
    let mut apps = AppStore::over(
        Box::new(move || MemoryStore::open(Arc::clone(&leaves), crate::apps_store::ORIGIN)),
        key,
    )?;
    let relations = draft
        .lys
        .relations()
        .map(|(relation, actions)| (relation.clone(), actions.clone()))
        .collect();
    apps.keep(Line::Lys(LysRecorded {
        operation: operation()?,
        version: draft.lys.version(),
        schema: AppSchema::lys(relations).to_json(),
        at,
    }))?;
    apps.keep(Line::Registered(Registered {
        operation: operation()?,
        app: draft.app.to_owned(),
        name: format!("the draft of {}", draft.app),
        redirects: Vec::new(),
        schema: draft.schema.clone(),
        service_account: None,
        by: draft.by.clone(),
        at,
    }))?;
    apps.keep(Line::Approved(Approved {
        operation: operation()?,
        app: draft.app.to_owned(),
        client: Client {
            client_id: format!("bench-{}", draft.app),
            secret_sha256: String::new(),
        },
        binding: None,
        by: draft.by.clone(),
        at,
    }))?;
    Ok(apps)
}

/// Add an active person to the scratch directory, bound to `subject` at the
/// example issuer.
fn person(directory: &mut Projection, subject: &str, at: u64) -> Result<PersonId, ServerError> {
    let person = PersonId::generate()?;
    let binding = LoginBinding::new(EXAMPLE_ISSUER, subject)?;
    let event = IdentityEvent::new(
        OperationId::generate()?,
        Actor::new(binding, Provenance::new(AuthMethod::Oidc, at)),
        IdentityId::Person(person),
        at,
        Change::SetupPerson {
            profile: Profile::new("An example on the test bench")?,
        },
    )?;
    let index = directory.records().count() as u64;
    directory.apply(&event, index)?;
    Ok(person)
}

/// The scratch directory: its root, and a person for each example subject.
fn directory(
    examples: &Examples<'_>,
    at: u64,
) -> Result<(Projection, PersonId, BTreeMap<String, PersonId>), ServerError> {
    let mut directory = Projection::new();
    let root = person(&mut directory, "the bench's root", at)?;
    let mut people = BTreeMap::new();
    let subjects = examples
        .holdings
        .iter()
        .map(|holding| holding.subject.as_str())
        .chain([examples.question.subject.as_str()]);
    for subject in subjects {
        if !people.contains_key(subject) {
            let id = person(&mut directory, &format!("example:{subject}"), at)?;
            people.insert(subject.to_owned(), id);
        }
    }
    Ok((directory, root, people))
}

/// The scratch grants, on the scratch scope `scope` of the service's
/// `SpiceDB` when the draft names one, as the service opens its own.
fn grants(
    draft: &Draft<'_>,
    (model, root): (Model, PersonId),
    scope: &str,
) -> Result<ScratchGrants, ServerError> {
    let leaves = MemoryStore::empty();
    let key = Ed25519Identity::ephemeral();
    let relationships = match draft.engine {
        Some(settings) => Relationships::SpiceDb(SpiceDb::open_scratch(settings, &model, scope)?),
        None => Relationships::Memory(MemoryRelationships::default()),
    };
    let mut grants = Grants::open(
        Box::new(move || MemoryStore::open(Arc::clone(&leaves), GRANT_ORIGIN)),
        key,
        relationships,
        model,
        root,
    )?;
    if draft.engine.is_some() {
        grants.project()?;
    }
    Ok(grants)
}

/// Place each example resource in its parent, as the placements route does:
/// in the apps log, and in the permission engine when it is `SpiceDB`.
fn place(
    (apps, grants): (&mut AppStore<MemoryStore>, &ScratchGrants),
    draft: &Draft<'_>,
    placements: &[Placement],
    at: u64,
) -> Result<(), ServerError> {
    for placement in placements {
        let (child, parent) = (resource(&placement.child)?, resource(&placement.parent)?);
        for kind in [child.kind(), parent.kind()] {
            apps.admit_kind(Some(draft.app), kind)?;
        }
        let flows = apps
            .schema(draft.app)
            .and_then(|schema| schema.kind(child.kind()))
            .is_some_and(|kind| kind.parents.contains(parent.kind()));
        if !flows {
            return Err(AppError::PlacementInvalid {
                reason: format!(
                    "the kind {} does not list {} among its parents",
                    child.kind(),
                    parent.kind()
                ),
            }
            .into());
        }
        apps.keep(Line::Placed(Placed {
            operation: operation()?,
            app: draft.app.to_owned(),
            child_kind: child.kind().to_owned(),
            child_id: child.id().to_owned(),
            parent_kind: parent.kind().to_owned(),
            parent_id: parent.id().to_owned(),
            restricted: false,
            revision: None,
            by: draft.by.clone(),
            at,
        }))?;
        if let Relationships::SpiceDb(engine) = grants.relationships() {
            engine.place(&child, &parent)?;
        }
    }
    Ok(())
}

fn answer(
    draft: &Draft<'_>,
    examples: &Examples<'_>,
    scope: &str,
) -> Result<BenchAnswer, ServerError> {
    let at = now();
    let mut apps = apps(draft, at)?;
    let (directory, root, people) = directory(examples, at)?;
    let mut grants = grants(draft, (apps.model()?, root), scope)?;
    place((&mut apps, &grants), draft, examples.placements, at)?;
    let mut held = BTreeMap::new();
    for holding in examples.holdings {
        let on = resource(&holding.resource)?;
        apps.admit_kind(None, on.kind())?;
        let holder = *people
            .get(&holding.subject)
            .ok_or_else(|| unavailable("an example subject has no person"))?;
        let recorded = grants.issue_root(
            &directory,
            &RootRequest {
                operation: OperationId::generate()?,
                caller: IdentityId::Person(root),
                route: Route::Api,
                holder,
                resource: on,
                relation: Relation::new(&holding.relation)?,
                pass_on: PassOn::UseOnly,
                window: Window::new(0, None)?,
            },
            at,
        )?;
        if let GrantChange::Issue(grant) = recorded.event.change() {
            held.insert(grant.id().to_string(), holding);
        }
    }
    let question = examples.question;
    let subject = people
        .get(&question.subject)
        .ok_or_else(|| unavailable("the asked subject has no person"))?;
    let asked = resource(&question.resource)?;
    let reached = apps.reach(&asked);
    let mut judged = Judged {
        directory: &directory,
        grants: &mut grants,
        root,
        apps: &mut apps,
    };
    let check = CheckWire {
        subject: IdentityId::Person(*subject).to_string(),
        kind: asked.kind().to_owned(),
        id: asked.id().to_owned(),
        action: question.action.clone(),
    };
    let decided = one(&mut judged, None, &check, at, None);
    let via = decided.via.as_deref().unwrap_or_default();
    let upto = reached
        .iter()
        .position(|on| on.to_string() == via)
        .unwrap_or(reached.len().saturating_sub(1));
    let mut path: Vec<String> = reached
        .windows(2)
        .take(upto)
        .map(|pair| format!("{} is placed in {}", pair[0], pair[1]))
        .collect();
    let holding = decided.grant.as_deref().and_then(|grant| held.get(grant));
    match (decided.allowed, holding) {
        (true, Some(holding)) => {
            path.push(format!(
                "{} holds {} on {via}",
                holding.subject, holding.relation
            ));
            path.push(format!("{} carries {}", holding.relation, question.action));
        }
        (true, None) => return Err(unavailable("the check allowed by a grant no example holds")),
        (false, _) if decided.refusal.as_deref() == Some("NotHeld") => path.push(format!(
            "{} holds no relation carrying {} on {asked} or a parent it is placed in",
            question.subject, question.action
        )),
        (false, _) => path.push(decided.reason.clone().unwrap_or_default()),
    }
    Ok(BenchAnswer {
        allowed: decided.allowed,
        path,
        refusal: decided.refusal,
    })
}

#[cfg(test)]
#[path = "apps_bench_memory_tests.rs"]
mod memory_tests;
