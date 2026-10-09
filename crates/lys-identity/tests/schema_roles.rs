//! Roles in an app's schema (ACCESS-004 R1): a role is a name and a set of
//! its kind's actions; a schema without roles writes back exactly as before;
//! a change names each role it adds, removes, widens or narrows; a narrowing
//! strands the standing grants naming the role; and a grant naming a role is
//! judged as the role's actions under the version current when it is judged.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;

use lys_identity::grants::{
    Action, AppSchema, Grant, GrantId, GrantParts, Model, Named, PassOn, Relation, Resource,
    SchemaError, Source, Standing, Window, diff, stranded,
};
use lys_identity::{IdentityId, OperationId, PersonId};
use serde_json::{Value, json};

type Outcome = Result<(), Box<dyn Error>>;

/// Cambium's workspace, its administrator a role over three of its actions.
fn cambium(administrator: &[&str]) -> Value {
    json!({"kinds": {
        "cambium.workspace": {
            "actions": ["read", "post", "seat_add", "seat_retire"],
            "relations": {"member": ["read", "post"]},
            "roles": {"administrator": administrator, "observer": ["read"]}
        }
    }})
}

fn refused(schema: &Value) -> Result<(String, String), String> {
    match AppSchema::parse("cambium", schema) {
        Err(SchemaError::Invalid { pointer, reason }) => Ok((pointer, reason)),
        other => Err(format!("expected schema_invalid, got {other:?}")),
    }
}

fn actions(names: &[&str]) -> Result<BTreeSet<Action>, Box<dyn Error>> {
    Ok(names
        .iter()
        .map(|name| Action::new(name))
        .collect::<Result<_, _>>()?)
}

#[test]
fn a_role_is_read_as_its_name_and_its_kinds_actions_and_written_back() -> Outcome {
    let schema = AppSchema::parse("cambium", &cambium(&["read", "seat_add"]))?;
    let administrator = Relation::new("administrator")?;
    assert_eq!(
        schema.role("cambium.workspace", &administrator),
        Some(&actions(&["read", "seat_add"])?)
    );
    let written = schema.to_json();
    assert_eq!(
        written["kinds"]["cambium.workspace"]["roles"],
        json!({"administrator": ["read", "seat_add"], "observer": ["read"]})
    );
    assert_eq!(AppSchema::parse("cambium", &written)?, schema);
    Ok(())
}

#[test]
fn a_schema_without_roles_writes_back_byte_for_byte_as_before() -> Outcome {
    let schema = json!({"kinds": {
        "notes.workspace": {
            "actions": ["read", "write"],
            "relations": {"member": ["read"], "owner": ["read", "write"]}
        }
    }});
    let read = AppSchema::parse("notes", &schema)?;
    // The form every schema was written in before roles: no `roles` member.
    let before = json!({"kinds": {
        "notes.workspace": {
            "actions": ["read", "write"],
            "relations": {"member": ["read"], "owner": ["read", "write"]},
            "parents": []
        }
    }});
    assert_eq!(
        serde_json::to_vec(&read.to_json())?,
        serde_json::to_vec(&before)?
    );
    assert!(read.kinds().values().all(|kind| kind.roles.is_empty()));
    Ok(())
}

#[test]
fn a_role_carrying_an_action_its_kind_does_not_declare_is_refused_at_it() -> Outcome {
    let (pointer, reason) = refused(&cambium(&["read", "launch"]))?;
    assert_eq!(pointer, "/kinds/cambium.workspace/roles/administrator/1");
    assert!(reason.contains("`launch`"), "{reason}");
    Ok(())
}

#[test]
fn a_role_named_as_a_relation_or_an_action_of_its_kind_is_refused() -> Outcome {
    let mut schema = cambium(&["read"]);
    schema["kinds"]["cambium.workspace"]["roles"] = json!({"member": ["read"]});
    let (pointer, reason) = refused(&schema)?;
    assert_eq!(pointer, "/kinds/cambium.workspace/roles/member");
    assert!(reason.contains("relation"), "{reason}");
    schema["kinds"]["cambium.workspace"]["roles"] = json!({"post": ["read"]});
    let (pointer, reason) = refused(&schema)?;
    assert_eq!(pointer, "/kinds/cambium.workspace/roles/post");
    assert!(reason.contains("action"), "{reason}");
    Ok(())
}

#[test]
fn an_empty_role_and_an_action_named_twice_are_refused() -> Outcome {
    let (pointer, _) = refused(&cambium(&[]))?;
    assert_eq!(pointer, "/kinds/cambium.workspace/roles/administrator");
    let (pointer, reason) = refused(&cambium(&["read", "read"]))?;
    assert_eq!(pointer, "/kinds/cambium.workspace/roles/administrator/1");
    assert!(reason.contains("twice"), "{reason}");
    Ok(())
}

#[test]
fn a_change_names_the_widening_of_a_role_in_words() -> Outcome {
    let old = AppSchema::parse("cambium", &cambium(&["read", "seat_add"]))?;
    let new = AppSchema::parse("cambium", &cambium(&["read", "seat_add", "seat_retire"]))?;
    let change = diff(&old, &new);
    assert_eq!(change.roles_changed.len(), 1);
    let widened = &change.roles_changed[0];
    assert_eq!(widened.added, vec!["seat_retire".to_owned()]);
    assert!(widened.removed.is_empty());
    assert_eq!(
        widened.words(),
        vec!["adds seat_retire to administrator".to_owned()]
    );
    assert!(!change.is_empty());
    let mut gone = cambium(&["read"]);
    gone["kinds"]["cambium.workspace"]["roles"] = json!({"administrator": ["read"]});
    let removed = diff(&old, &AppSchema::parse("cambium", &gone)?);
    assert_eq!(
        removed.roles_removed,
        vec![Named {
            kind: "cambium.workspace".to_owned(),
            name: "observer".to_owned(),
        }]
    );
    Ok(())
}

#[test]
fn a_narrowing_strands_the_grants_naming_the_role_and_nothing_else() -> Outcome {
    let old = AppSchema::parse("cambium", &cambium(&["read", "seat_add"]))?;
    let narrowed = AppSchema::parse("cambium", &cambium(&["read"]))?;
    let widened = AppSchema::parse("cambium", &cambium(&["read", "seat_add", "post"]))?;
    let administrator = Relation::new("administrator")?;
    let member = Relation::new("member")?;
    let held = actions(&["read", "seat_add"])?;
    let member_actions = actions(&["read", "post"])?;
    let by_role = Standing {
        kind: "cambium.workspace",
        relation: &administrator,
        actions: &held,
        held: false,
        role: true,
    };
    let by_relation = Standing {
        kind: "cambium.workspace",
        relation: &member,
        actions: &member_actions,
        held: false,
        role: false,
    };
    let counted = stranded(&old, &narrowed, [by_role, by_relation]);
    assert_eq!(
        counted,
        BTreeMap::from([(
            Named {
                kind: "cambium.workspace".to_owned(),
                name: "administrator".to_owned(),
            },
            1
        )])
    );
    assert!(
        stranded(&old, &narrowed, [by_relation]).is_empty(),
        "a narrowing no standing grant names strands nothing"
    );
    assert!(stranded(&old, &widened, [by_role, by_relation]).is_empty());
    Ok(())
}

/// A root grant on `workspace` naming the role `administrator`, issued
/// under version 1 with the role's actions then.
fn administrator_grant(issued: &BTreeSet<Action>) -> Result<Grant, Box<dyn Error>> {
    let person = PersonId::from_bytes([0xd1; 16]);
    Ok(Grant::new(GrantParts {
        id: GrantId::from_bytes([0x61; 16]),
        issuer: IdentityId::Person(person),
        holder: IdentityId::Person(person),
        responsible: person,
        resource: Resource::new("cambium.workspace", "ward")?,
        relation: Relation::new("administrator")?,
        actions: issued.clone(),
        pass_on: PassOn::UseOnly,
        source: Source::Root,
        window: Window::new(1_000, None)?,
        model_version: 1,
        operation: OperationId::from_bytes([0x0e; 16]),
    })?
    .as_role())
}

fn model(schema: &AppSchema, version: u64) -> Result<Model, Box<dyn Error>> {
    let lys = [(Relation::new("viewer")?, actions(&["read"])?)];
    Ok(Model::new(1, lys)?.with_kinds(schema.kind_models(version)))
}

#[test]
fn a_grant_naming_a_role_is_judged_as_the_roles_actions_at_the_current_version() -> Outcome {
    let first = AppSchema::parse("cambium", &cambium(&["read", "seat_add"]))?;
    let issued = actions(&["read", "seat_add"])?;
    let grant = administrator_grant(&issued)?;
    assert!(grant.names_role());
    assert_eq!(model(&first, 1)?.actions_of(&grant), &issued);
    let widened = AppSchema::parse("cambium", &cambium(&["read", "seat_add", "seat_retire"]))?;
    assert_eq!(
        model(&widened, 2)?.actions_of(&grant),
        &actions(&["read", "seat_add", "seat_retire"])?,
        "the role's actions at version 2, not those it was issued with"
    );
    assert_eq!(
        grant.actions(),
        &issued,
        "the grant keeps what it first gave"
    );
    let mut gone = cambium(&["read"]);
    gone["kinds"]["cambium.workspace"]["roles"] = json!({"observer": ["read"]});
    let without = AppSchema::parse("cambium", &gone)?;
    assert!(model(&without, 3)?.actions_of(&grant).is_empty());
    Ok(())
}

#[test]
fn a_grant_naming_a_relation_keeps_the_actions_it_was_issued_with() -> Outcome {
    let schema = AppSchema::parse("cambium", &cambium(&["read"]))?;
    let person = PersonId::from_bytes([0xd1; 16]);
    let issued = actions(&["read"])?;
    let grant = Grant::new(GrantParts {
        id: GrantId::from_bytes([0x62; 16]),
        issuer: IdentityId::Person(person),
        holder: IdentityId::Person(person),
        responsible: person,
        resource: Resource::new("cambium.workspace", "ward")?,
        relation: Relation::new("member")?,
        actions: issued.clone(),
        pass_on: PassOn::UseOnly,
        source: Source::Root,
        window: Window::new(1_000, None)?,
        model_version: 1,
        operation: OperationId::from_bytes([0x0e; 16]),
    })?;
    assert!(!grant.names_role());
    assert_eq!(model(&schema, 2)?.actions_of(&grant), &issued);
    Ok(())
}

#[test]
fn a_held_grant_under_a_role_carrying_a_hot_action_is_named() -> Outcome {
    let mut schema = cambium(&["read", "seat_add"]);
    schema["kinds"]["cambium.workspace"]["hot"] = json!(["read"]);
    let read = AppSchema::parse("cambium", &schema)?;
    let hot = read.hot_action("cambium.workspace", &Relation::new("administrator")?);
    assert_eq!(hot.map(Action::as_str), Some("read"));
    Ok(())
}
