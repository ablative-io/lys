#![cfg(test)]
//! An app's schema against the rules the schema module states: every fault
//! is refused `schema_invalid` at its own JSON pointer, one test per fault,
//! and each test asserts the pointer the fault is at, so a refusal from the
//! wrong rule fails it and a fault refused by an earlier rule does too.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;

use serde_json::{Value, json};

use super::permission::parent_relation;
use super::schema::{ANY_KIND, AppSchema, LYS_APP, SchemaError, app_id, owner_of};
use super::schema_diff::{Named, Standing, diff, stranded};
use super::types::{Action, Relation, Resource};

/// A two-kind schema with a parent: a workspace whose members reach its channels.
fn workspace() -> Value {
    json!({"kinds": {
        "notes.workspace": {
            "actions": ["read", "write"],
            "relations": {"member": ["read"], "owner": ["read", "write"]}
        },
        "notes.channel": {
            "actions": ["read", "write"],
            "relations": {"poster": ["read", "write"]},
            "parents": ["notes.workspace"]
        }
    }})
}

type Outcome = Result<(), Box<dyn Error>>;

/// The pointer and words of a `schema_invalid` refusal, or an error naming
/// what was answered instead.
fn refused(app: &str, schema: &Value) -> Result<(String, String), String> {
    match AppSchema::parse(app, schema) {
        Err(SchemaError::Invalid { pointer, reason }) => Ok((pointer, reason)),
        other => Err(format!("expected schema_invalid, got {other:?}")),
    }
}

#[test]
fn a_valid_schema_reads_back_as_it_was_written() -> Outcome {
    let schema = AppSchema::parse("notes", &workspace())?;
    assert_eq!(schema.kinds().len(), 2);
    let again = AppSchema::parse("notes", &schema.to_json())?;
    assert_eq!(again, schema);
    Ok(())
}

#[test]
fn a_kind_outside_the_prefix_is_refused_naming_the_owning_prefix() -> Outcome {
    let mut schema = workspace();
    schema["kinds"]["files.doc"] = json!({"actions": ["read"], "relations": {"reader": ["read"]}});
    let (pointer, reason) = refused("notes", &schema)?;
    assert_eq!(pointer, "/kinds/files.doc");
    assert!(reason.contains("`files`"), "{reason}");
    Ok(())
}

#[test]
fn an_unprefixed_kind_is_refused_as_belonging_to_lys() -> Outcome {
    let schema =
        json!({"kinds": {"doc": {"actions": ["read"], "relations": {"reader": ["read"]}}}});
    let (pointer, reason) = refused("notes", &schema)?;
    assert_eq!(pointer, "/kinds/doc");
    assert!(reason.contains("`lys`"), "{reason}");
    Ok(())
}

#[test]
fn an_action_declared_only_on_another_kind_is_refused_at_its_relation() -> Outcome {
    let mut schema = workspace();
    schema["kinds"]["notes.workspace"]["actions"] = json!(["read", "write", "invite"]);
    schema["kinds"]["notes.channel"]["relations"]["poster"] = json!(["read", "invite"]);
    let (pointer, reason) = refused("notes", &schema)?;
    assert_eq!(pointer, "/kinds/notes.channel/relations/poster/1");
    assert!(reason.contains("not declared on the kind"), "{reason}");
    Ok(())
}

#[test]
fn a_relation_carrying_an_unknown_action_is_refused_at_the_action() -> Outcome {
    let mut schema = workspace();
    schema["kinds"]["notes.workspace"]["relations"]["member"] = json!(["read", "delete"]);
    let (pointer, reason) = refused("notes", &schema)?;
    assert_eq!(pointer, "/kinds/notes.workspace/relations/member/1");
    assert!(reason.contains("unknown action `delete`"), "{reason}");
    Ok(())
}

#[test]
fn a_parent_cycle_is_refused_at_the_parent_that_closes_it() -> Outcome {
    let mut schema = workspace();
    schema["kinds"]["notes.workspace"]["parents"] = json!(["notes.channel"]);
    let (pointer, reason) = refused("notes", &schema)?;
    assert_eq!(pointer, "/kinds/notes.channel/parents/0");
    assert!(reason.contains("cycle"), "{reason}");
    Ok(())
}

#[test]
fn a_kind_that_is_its_own_parent_is_a_cycle() -> Outcome {
    let mut schema = workspace();
    schema["kinds"]["notes.workspace"]["parents"] = json!(["notes.workspace"]);
    let (pointer, _) = refused("notes", &schema)?;
    assert_eq!(pointer, "/kinds/notes.workspace/parents/0");
    Ok(())
}

#[test]
fn a_parent_in_another_app_is_refused_naming_its_prefix() -> Outcome {
    let mut schema = workspace();
    schema["kinds"]["notes.channel"]["parents"] = json!(["files.folder"]);
    let (pointer, reason) = refused("notes", &schema)?;
    assert_eq!(pointer, "/kinds/notes.channel/parents/0");
    assert!(
        reason.contains("another app, the prefix `files`"),
        "{reason}"
    );
    Ok(())
}

#[test]
fn a_parent_the_schema_does_not_declare_is_refused() -> Outcome {
    let mut schema = workspace();
    schema["kinds"]["notes.channel"]["parents"] = json!(["notes.folder"]);
    let (pointer, reason) = refused("notes", &schema)?;
    assert_eq!(pointer, "/kinds/notes.channel/parents/0");
    assert!(reason.contains("not a kind of this schema"), "{reason}");
    Ok(())
}

#[test]
fn a_name_the_engine_does_not_take_is_refused_at_it() -> Outcome {
    let mut schema = workspace();
    schema["kinds"]["notes.workspace"]["actions"] = json!(["read", "write", "Go"]);
    let (pointer, _) = refused("notes", &schema)?;
    assert_eq!(pointer, "/kinds/notes.workspace/actions/2");
    Ok(())
}

#[test]
fn a_relation_named_as_an_action_of_its_kind_is_refused() -> Outcome {
    let mut schema = workspace();
    schema["kinds"]["notes.workspace"]["relations"]["read"] = json!(["read"]);
    let (pointer, _) = refused("notes", &schema)?;
    assert_eq!(pointer, "/kinds/notes.workspace/relations/read");
    Ok(())
}

#[test]
fn an_unknown_member_is_refused_at_it() -> Outcome {
    let mut schema = workspace();
    schema["kinds"]["notes.channel"]["colour"] = json!("blue");
    let (pointer, _) = refused("notes", &schema)?;
    assert_eq!(pointer, "/kinds/notes.channel/colour");
    Ok(())
}

#[test]
fn app_ids_take_the_permission_stores_alphabet_only() {
    let (long, longest) = ("a".repeat(41), "a".repeat(40));
    let mut refused = 0;
    for bad in ["my-app", "my.app", "ab", "9lives", "Notes", long.as_str()] {
        assert!(
            matches!(app_id(bad), Err(SchemaError::AppIdInvalid { .. })),
            "{bad} was taken"
        );
        refused += 1;
    }
    assert_eq!(refused, 6, "every bad id was tried");
    for good in ["abc", "notes", "files_2", longest.as_str()] {
        assert_eq!(app_id(good).ok(), Some(good));
    }
}

#[test]
fn lys_is_the_exception_and_declares_every_unprefixed_kind() -> Outcome {
    let model = json!({"relations": {"alpha": ["read", "write"]}});
    let schema = AppSchema::parse(LYS_APP, &model)?;
    assert!(schema.kind("doc").is_some());
    assert!(schema.kind("notes.doc").is_none());
    assert!(schema.kinds().contains_key(ANY_KIND));
    assert!(schema.kind_models(1).is_empty());
    assert_eq!(owner_of("doc"), LYS_APP);
    assert_eq!(owner_of("notes.doc"), "notes");
    Ok(())
}

#[test]
fn a_parent_relation_reaches_a_placed_child_and_no_further_than_the_schema_says() -> Outcome {
    let schema = AppSchema::parse("notes", &workspace())?;
    let channel = Resource::new("notes.channel", "general")?;
    let space = Resource::new("notes.workspace", "team")?;
    let placed_in = space.clone();
    let reached = schema.reach(&channel, |resource| {
        (resource.kind() == "notes.channel").then(|| placed_in.clone())
    });
    assert_eq!(reached, vec![channel.clone(), space.clone()]);
    let unplaced = schema.reach(&space, |_| Some(channel.clone()));
    assert_eq!(
        unplaced,
        vec![space],
        "a workspace lists no parent, so nothing flows up"
    );
    assert_eq!(parent_relation("notes.workspace"), "parent_workspace");
    Ok(())
}

#[test]
fn a_change_names_what_it_adds_and_removes_and_what_it_strands() -> Outcome {
    let old = AppSchema::parse("notes", &workspace())?;
    let mut changed = workspace();
    changed["kinds"]["notes.workspace"]["relations"] = json!({"owner": ["read", "write"]});
    changed["kinds"]["notes.channel"]["actions"] = json!(["read"]);
    changed["kinds"]["notes.channel"]["relations"]["poster"] = json!(["read"]);
    let new = AppSchema::parse("notes", &changed)?;
    let change = diff(&old, &new);
    let relation = |kind: &str, name: &str| Named {
        kind: kind.to_owned(),
        name: name.to_owned(),
    };
    assert_eq!(
        change.relations_removed,
        vec![relation("notes.workspace", "member")]
    );
    assert_eq!(
        change.actions_removed,
        vec![relation("notes.channel", "write")]
    );
    assert!(change.kinds_added.is_empty() && change.kinds_removed.is_empty());
    let (member, poster, owner) = (
        Relation::new("member")?,
        Relation::new("poster")?,
        Relation::new("owner")?,
    );
    let read: BTreeSet<Action> = [Action::new("read")?].into();
    let both: BTreeSet<Action> = ["read", "write"]
        .iter()
        .map(|name| Action::new(name))
        .collect::<Result<_, _>>()?;
    let standing = [
        ("notes.workspace", &member, &read),
        ("notes.workspace", &member, &read),
        ("notes.workspace", &owner, &both),
        ("notes.channel", &poster, &both),
        ("files.doc", &member, &read),
    ];
    let counted = stranded(
        &old,
        &new,
        standing.iter().map(|(kind, relation, actions)| Standing {
            kind,
            relation,
            actions,
            held: false,
        }),
    );
    let expected = BTreeMap::from([
        (relation("notes.channel", "poster"), 1),
        (relation("notes.workspace", "member"), 2),
    ]);
    assert_eq!(counted, expected);
    Ok(())
}
