//! Lys's relation and action names as the permission engine takes them.
//!
//! A Lys name the engine already takes is its own engine name, so every
//! relationship an engine holds keeps its name. Any other Lys name, such as
//! the dotted `only.person.profile.set`, becomes [`MAPPED`] and the name with
//! each `_` as `_u`, `.` as `_d` and `-` as `_h`. The engine name is read back
//! by the inverse. A Lys name that itself starts with [`MAPPED`], or whose
//! engine name would be over the engine's length, is refused by name.

use lys_identity::grants::GrantError;

/// The prefix of an engine name that stands for an escaped Lys name.
pub(super) const MAPPED: &str = "x0_";

/// Whether the engine takes `name`: three to sixty-four lowercase letters,
/// digits and underscores, starting with a letter and not ending with an
/// underscore.
pub(super) fn engine_takes(name: &str) -> bool {
    let bytes = name.as_bytes();
    let inner = |byte: &u8| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'_';
    (3..=64).contains(&bytes.len())
        && bytes.first().is_some_and(u8::is_ascii_lowercase)
        && bytes.last().is_some_and(|byte| *byte != b'_')
        && bytes.iter().all(inner)
}

/// The engine's name for the Lys name `name`.
pub(super) fn engine_ident(name: &str) -> String {
    if engine_takes(name) && !name.starts_with(MAPPED) {
        return name.to_owned();
    }
    let mut mapped = String::from(MAPPED);
    for character in name.chars() {
        match character {
            '_' => mapped.push_str("_u"),
            '.' => mapped.push_str("_d"),
            '-' => mapped.push_str("_h"),
            other => mapped.push(other),
        }
    }
    mapped
}

/// The Lys name the engine name `name` stands for, or none when `name`
/// holds an escape Lys never writes, which only a corrupt engine holds.
pub(super) fn lys_name(name: &str) -> Option<String> {
    let Some(escaped) = name.strip_prefix(MAPPED) else {
        return Some(name.to_owned());
    };
    let mut lys = String::with_capacity(escaped.len());
    let mut characters = escaped.chars();
    while let Some(character) = characters.next() {
        if character != '_' {
            lys.push(character);
            continue;
        }
        match characters.next() {
            Some('u') => lys.push('_'),
            Some('d') => lys.push('.'),
            Some('h') => lys.push('-'),
            _ => return None,
        }
    }
    // Only a name engine_ident writes is read back: x0_foo for foo is not.
    (engine_ident(&lys) == name && !lys.starts_with(MAPPED)).then_some(lys)
}

/// The engine's name for `name` on an object of `kind`. The map belongs to
/// Lys's own grant model alone: an approved app's kind, `{app}.{kind}`, keeps
/// its names as they stand, since the app's schema already took them.
pub(super) fn engine_ident_on(kind: &str, name: &str) -> String {
    if kind.contains('.') {
        name.to_owned()
    } else {
        engine_ident(name)
    }
}

/// The name `name` read from the engine on an object of `kind` stands for.
pub(super) fn lys_name_on(kind: &str, name: &str) -> Option<String> {
    if kind.contains('.') {
        Some(name.to_owned())
    } else {
        lys_name(name)
    }
}

/// The engine name for the Lys `what` named `name`, refused by name when
/// the engine cannot take one for it.
pub(super) fn engine_name_for(what: &str, name: &str) -> Result<String, GrantError> {
    if name.starts_with(MAPPED) {
        return Err(GrantError::PermissionEngineUnavailable {
            reason: format!(
                "the {what} {name} starts with {MAPPED}, which the permission engine's names keep for escaped Lys names"
            ),
        });
    }
    let engine = engine_ident(name);
    if engine_takes(&engine) {
        return Ok(engine);
    }
    Err(GrantError::PermissionEngineUnavailable {
        reason: format!(
            "the {what} {name} has no name the permission engine takes: {engine} is not three to sixty-four lowercase letters, digits and underscores, starting with a letter"
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::super::{relationship_json, relationship_of};
    use super::{MAPPED, engine_ident, engine_name_for, lys_name};
    use lys_identity::grants::shipped_model;
    use lys_identity::grants::{ObjectRef, Relationship};
    use serde_json::Value;

    #[test]
    fn every_shipped_name_maps_to_an_engine_name_and_back() -> Result<(), Box<dyn std::error::Error>>
    {
        let model: Value = serde_json::from_str(&shipped_model())?;
        let relations = model["relations"].as_object().ok_or("no relations")?;
        let mut seen = std::collections::BTreeMap::new();
        for (relation, actions) in relations {
            let actions = actions.as_array().ok_or("no actions")?;
            let names =
                std::iter::once(relation.as_str()).chain(actions.iter().filter_map(Value::as_str));
            for name in names.map(str::to_owned) {
                let engine = engine_name_for("name", &name)?;
                assert_eq!(lys_name(&engine), Some(name.clone()), "{engine}");
                if let Some(other) = seen.insert(engine.clone(), name.clone()) {
                    assert_eq!(other, name, "two names become {engine}");
                }
            }
        }
        assert_eq!(
            engine_ident("only.agent.mcp-request.approve"),
            "x0_only_dagent_dmcp_hrequest_dapprove"
        );
        Ok(())
    }

    #[test]
    fn a_name_the_engine_already_takes_is_its_own_engine_name()
    -> Result<(), Box<dyn std::error::Error>> {
        for name in ["owner", "view", "can_edit", "parent_project"] {
            assert_eq!(engine_name_for("relation", name)?, name);
            assert_eq!(lys_name(name).as_deref(), Some(name));
        }
        Ok(())
    }

    #[test]
    fn a_name_that_cannot_map_is_refused_by_name() {
        let reserved = format!("{MAPPED}owner");
        let refusal = engine_name_for("relation", &reserved).map_err(|error| error.to_string());
        assert!(
            refusal.as_ref().is_err_and(|text| text.contains(&reserved)),
            "{refusal:?}"
        );
        let long = format!("only.{}", "a".repeat(60));
        let refusal = engine_name_for("relation", &long).map_err(|error| error.to_string());
        assert!(
            refusal.as_ref().is_err_and(|text| text.contains(&long)),
            "{refusal:?}"
        );
        assert_eq!(
            lys_name(&engine_ident("a_b.c-d")).as_deref(),
            Some("a_b.c-d")
        );
        assert_eq!(lys_name("x0_a_qb"), None, "an escape Lys never writes");
        assert_eq!(lys_name("x0_ab_"), None, "an escape cut short");
        assert_eq!(lys_name("x0_owner"), None, "owner is never written escaped");
        assert_eq!(
            lys_name("x0_x0_ufoo"),
            None,
            "a Lys name never starts with x0_"
        );
    }

    #[test]
    fn an_app_relation_is_written_and_read_back_as_it_stands() {
        let object = |kind: &str, id: &str| ObjectRef {
            kind: kind.to_owned(),
            id: id.to_owned(),
        };
        for (kind, relation, engine) in [
            ("fixture.doc", "x0_foo", "x0_foo"),
            (
                "document",
                "only.person.profile.set",
                "x0_only_dperson_dprofile_dset",
            ),
        ] {
            let relationship = Relationship {
                resource: object(kind, "one"),
                relation: relation.to_owned(),
                subject: object("grant", "g"),
                subject_relation: Some("holder".to_owned()),
                ends_at: None,
            };
            let json = relationship_json(&relationship);
            assert_eq!(json["relation"], engine, "{kind}");
            assert_eq!(relationship_of(&json), Some(relationship), "{kind}");
        }
    }
}
