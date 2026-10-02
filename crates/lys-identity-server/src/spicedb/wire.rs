//! The engine's JSON for an object and a relationship, written and read
//! back the one way every call uses.

use lys_identity::grants::{ObjectRef, Relationship};
use serde_json::{Value, json};

use super::{definition, engine_ident_on, lys_name_on};

pub(super) fn object_json(object: &ObjectRef) -> Value {
    json!({"objectType": definition(&object.kind), "objectId": object.id.replace('.', "|")})
}

pub(super) fn relationship_json(relationship: &Relationship) -> Value {
    let mut subject = json!({"object": object_json(&relationship.subject)});
    if let Some(relation) = &relationship.subject_relation {
        subject["optionalRelation"] = json!(engine_ident_on(&relationship.subject.kind, relation));
    }
    let mut value = json!({
        "resource": object_json(&relationship.resource),
        "relation": engine_ident_on(&relationship.resource.kind, &relationship.relation),
        "subject": subject,
    });
    if let Some(ends_at) = relationship.ends_at {
        value["optionalCaveat"] =
            json!({"caveatName": "unexpired", "context": {"ends_at": ends_at}});
    }
    value
}

pub(super) fn object_of(value: &Value) -> Option<ObjectRef> {
    Some(ObjectRef {
        kind: value.get("objectType")?.as_str()?.replacen('/', ".", 1),
        id: value.get("objectId")?.as_str()?.replace('|', "."),
    })
}

pub(super) fn relationship_of(value: &Value) -> Option<Relationship> {
    let subject = value.get("subject")?;
    let ends_at = match value.pointer("/optionalCaveat/context/ends_at") {
        Some(end) => Some(end.as_u64()?),
        None => None,
    };
    let resource = object_of(value.get("resource")?)?;
    let relation = lys_name_on(&resource.kind, value.get("relation")?.as_str()?)?;
    let object = object_of(subject.get("object")?)?;
    let subject_relation = match subject
        .get("optionalRelation")
        .and_then(Value::as_str)
        .filter(|relation| !relation.is_empty())
    {
        Some(relation) => Some(lys_name_on(&object.kind, relation)?),
        None => None,
    };
    Some(Relationship {
        resource,
        relation,
        subject: object,
        subject_relation,
        ends_at,
    })
}
