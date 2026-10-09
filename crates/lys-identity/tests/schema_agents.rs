//! The acts of an app's kind an agent may hold (ACCESS-001, the schema's
//! `agents`): none unless the kind names them, each named one the kind
//! declares, written back only when named, and the shipped rule unchanged on
//! Lys's own kinds.

use std::collections::BTreeSet;
use std::error::Error;

use lys_identity::grants::{Action, AppSchema, Model, Relation, SchemaError, agent_may_hold};
use serde_json::{Value, json};

type Outcome = Result<(), Box<dyn Error>>;

/// A channel kind of the app `rooms`, naming `agents` when given.
fn rooms(agents: Option<Value>) -> Value {
    let mut schema = json!({"kinds": {
        "rooms.channel": {
            "actions": ["read", "post"],
            "relations": {"reader": ["read"], "poster": ["post"]}
        }
    }});
    if let Some(agents) = agents {
        schema["kinds"]["rooms.channel"]["agents"] = agents;
    }
    schema
}

fn model(schema: &AppSchema) -> Result<Model, Box<dyn Error>> {
    let read: BTreeSet<Action> = [Action::new("read")?].into();
    Ok(Model::new(1, [(Relation::new("viewer")?, read)])?.with_kinds(schema.kind_models(1)))
}

fn refused(schema: &Value) -> Result<(String, String), String> {
    match AppSchema::parse("rooms", schema) {
        Err(SchemaError::Invalid { pointer, reason }) => Ok((pointer, reason)),
        other => Err(format!("expected schema_invalid, got {other:?}")),
    }
}

#[test]
fn a_kind_naming_no_agents_gives_an_agent_none_of_its_acts() -> Outcome {
    let schema = AppSchema::parse("rooms", &rooms(None))?;
    let model = model(&schema)?;
    for act in ["read", "post"] {
        assert!(
            !model.agent_may_hold("rooms.channel", &Action::new(act)?),
            "{act}"
        );
    }
    assert!(
        schema.to_json()["kinds"]["rooms.channel"]
            .get("agents")
            .is_none()
    );
    Ok(())
}

#[test]
fn a_kind_naming_read_gives_an_agent_read_and_nothing_else() -> Outcome {
    let schema = AppSchema::parse("rooms", &rooms(Some(json!(["read"]))))?;
    let model = model(&schema)?;
    assert!(model.agent_may_hold("rooms.channel", &Action::new("read")?));
    assert!(!model.agent_may_hold("rooms.channel", &Action::new("post")?));
    assert!(!model.agent_may_hold("rooms.unknown", &Action::new("read")?));
    assert_eq!(
        schema.to_json()["kinds"]["rooms.channel"]["agents"],
        json!(["read"])
    );
    Ok(())
}

#[test]
fn an_agent_act_the_kind_does_not_declare_or_named_twice_is_refused_at_it() -> Outcome {
    let (pointer, reason) = refused(&rooms(Some(json!(["write"]))))?;
    assert_eq!(pointer, "/kinds/rooms.channel/agents/0");
    assert!(
        reason.contains("`write` is not an action of its kind"),
        "{reason}"
    );
    let (pointer, reason) = refused(&rooms(Some(json!(["read", "read"]))))?;
    assert_eq!(pointer, "/kinds/rooms.channel/agents/1");
    assert!(reason.contains("named twice"), "{reason}");
    Ok(())
}

#[test]
fn lys_own_kinds_keep_the_shipped_rule() -> Outcome {
    let schema = AppSchema::parse("rooms", &rooms(Some(json!(["read"]))))?;
    let model = model(&schema)?;
    for (kind, act) in [
        ("directory", "view"),
        ("person", "person.retire"),
        ("directory", "made.up"),
    ] {
        assert_eq!(
            model.agent_may_hold(kind, &Action::new(act)?),
            agent_may_hold(kind, act),
            "{kind} {act}"
        );
    }
    Ok(())
}
