//! The canonical encoding of a grant template and a holding record, and the
//! strict reading of them and of a role event body.
//!
//! A template is a map of exactly the grant keys it carries: `4` responsible
//! (16 bytes), `5` resource (`1` kind, `2` id), `6` relation, `7` actions
//! (ascending), `8` pass-on (`0` for use-only, or `1` actions and `2`
//! recipient kinds) and `10` window (`1` starts-at, `2` null). A template
//! carrying the grant's holder key `3` is refused `template_has_holder`, one
//! carrying its source key `9` `template_has_source`, and a window with an
//! end `template_has_end`.
//!
//! A holding is a map of `1` id, `2` holder agent id, `3` role id, `4`
//! version, `5` project, `6` grant ids, `7` end date or null and `8` move
//! policy by name. A holding with no key `8` is refused `missing_policy`; a
//! name outside the two policies is refused `unknown_policy`.

use std::collections::{BTreeMap, BTreeSet};

use ciborium::Value;

use super::{Opening, RoleChange, RoleEvent, ROLE_EVENT_VERSION, write_resource};
use crate::encoding::{MAJOR_ARRAY, bytes, head, map, text, uint};
use crate::grants::codec::{read_identity, recipient_code};
use crate::grants::{Action, GrantId, PassOn, RecipientKind, Relation, Resource, Window};
use crate::id::{AgentId, ID_LEN, PersonId};
use crate::operation::OperationId;
use crate::roles::error::RoleError;
use crate::roles::types::{
    Capacity, Holding, HoldingId, MovePolicy, RoleId, Template, TemplateParts, Timing, title,
};

const NULL: u8 = 0xf6;

fn malformed(reason: &'static str) -> RoleError {
    RoleError::EventMalformed { reason }
}

/// The one CBOR item `bytes` hold.
pub(super) fn parse(bytes: &[u8], reason: &'static str) -> Result<Value, RoleError> {
    ciborium::from_reader(bytes)
        .ok()
        .ok_or_else(|| malformed(reason))
}

/// Whether the protected header `protected` names `envelope` as its content type.
pub(super) fn names_envelope(protected: &[u8], envelope: &str) -> bool {
    let Ok(Value::Map(pairs)) = parse(protected, "a protected header is a map") else {
        return false;
    };
    pairs.iter().any(|(key, value)| {
        matches!(as_uint(key, "a header key"), Ok(3))
            && matches!(value, Value::Text(named) if named == envelope)
    })
}

fn as_uint(value: &Value, reason: &'static str) -> Result<u64, RoleError> {
    match value {
        Value::Integer(integer) => u64::try_from(*integer)
            .ok()
            .ok_or_else(|| malformed(reason)),
        _ => Err(malformed(reason)),
    }
}

fn as_text(value: Value, reason: &'static str) -> Result<String, RoleError> {
    match value {
        Value::Text(text) => Ok(text),
        _ => Err(malformed(reason)),
    }
}

fn as_id(value: Value, reason: &'static str) -> Result<[u8; ID_LEN], RoleError> {
    match value {
        Value::Bytes(raw) => <[u8; ID_LEN]>::try_from(raw)
            .ok()
            .ok_or_else(|| malformed(reason)),
        _ => Err(malformed(reason)),
    }
}

fn as_array(value: Value, reason: &'static str) -> Result<Vec<Value>, RoleError> {
    match value {
        Value::Array(items) => Ok(items),
        _ => Err(malformed(reason)),
    }
}

fn as_ids(value: Value, reason: &'static str) -> Result<Vec<GrantId>, RoleError> {
    as_array(value, reason)?
        .into_iter()
        .map(|item| as_id(item, reason).map(GrantId::from_bytes))
        .collect()
}

fn as_end(value: Value, reason: &'static str) -> Result<Option<u64>, RoleError> {
    match value {
        Value::Null => Ok(None),
        other => as_uint(&other, reason).map(Some),
    }
}

/// A map's values by key, its keys unsigned and ascending, each once.
struct Fields {
    values: BTreeMap<u64, Value>,
    reason: &'static str,
}

impl Fields {
    fn read(value: Value, reason: &'static str) -> Result<Self, RoleError> {
        let Value::Map(pairs) = value else {
            return Err(malformed(reason));
        };
        let mut values = BTreeMap::new();
        let mut last = None;
        for (key, value) in pairs {
            let key = as_uint(&key, reason)?;
            if last.is_some_and(|last| key <= last) {
                return Err(malformed(reason));
            }
            last = Some(key);
            values.insert(key, value);
        }
        Ok(Self { values, reason })
    }

    fn has(&self, key: u64) -> bool {
        self.values.contains_key(&key)
    }

    fn take(&mut self, key: u64) -> Result<Value, RoleError> {
        self.values
            .remove(&key)
            .ok_or_else(|| malformed(self.reason))
    }

    fn done(self) -> Result<(), RoleError> {
        if self.values.is_empty() {
            Ok(())
        } else {
            Err(malformed(self.reason))
        }
    }
}

fn read_resource(value: Value) -> Result<Resource, RoleError> {
    const SHAPE: &str = "a resource is a map of keys 1 and 2";
    let mut fields = Fields::read(value, SHAPE)?;
    let kind = as_text(fields.take(1)?, SHAPE)?;
    let id = as_text(fields.take(2)?, SHAPE)?;
    fields.done()?;
    Ok(Resource::new(&kind, &id)?)
}

fn read_actions(value: Value) -> Result<BTreeSet<Action>, RoleError> {
    const SHAPE: &str = "actions are an array of tokens";
    as_array(value, SHAPE)?
        .into_iter()
        .map(|item| Ok(Action::new(&as_text(item, SHAPE)?)?))
        .collect()
}

fn read_pass_on(value: Value) -> Result<PassOn, RoleError> {
    const SHAPE: &str = "a pass-on is 0 or a map of keys 1 and 2";
    if let Value::Integer(_) = value {
        return match as_uint(&value, SHAPE)? {
            0 => Ok(PassOn::UseOnly),
            _ => Err(malformed(SHAPE)),
        };
    }
    let mut fields = Fields::read(value, SHAPE)?;
    let actions = read_actions(fields.take(1)?)?;
    let recipients = as_array(fields.take(2)?, SHAPE)?
        .iter()
        .map(|code| match as_uint(code, SHAPE)? {
            1 => Ok(RecipientKind::Person),
            2 => Ok(RecipientKind::Agent),
            _ => Err(malformed("a recipient kind code is 1 or 2")),
        })
        .collect::<Result<BTreeSet<_>, RoleError>>()?;
    fields.done()?;
    Ok(PassOn::to(actions, recipients)?)
}

fn read_window(value: Value) -> Result<Window, RoleError> {
    const SHAPE: &str = "a window is a map of keys 1 and 2";
    let mut fields = Fields::read(value, SHAPE)?;
    let starts_at = as_uint(&fields.take(1)?, SHAPE)?;
    let ends_at = as_end(fields.take(2)?, SHAPE)?;
    fields.done()?;
    if ends_at.is_some() {
        return Err(RoleError::TemplateHasEnd);
    }
    Ok(Window::new(starts_at, None)?)
}

fn write_actions(out: &mut Vec<u8>, actions: &BTreeSet<Action>) {
    head(out, MAJOR_ARRAY, actions.len() as u64);
    for action in actions {
        text(out, action.as_str());
    }
}

/// Append the canonical encoding of `template`.
fn write_template(out: &mut Vec<u8>, template: &Template) {
    map(out, 6);
    uint(out, 4);
    bytes(out, template.responsible().as_bytes());
    uint(out, 5);
    write_resource(out, template.resource());
    uint(out, 6);
    text(out, template.relation().as_str());
    uint(out, 7);
    write_actions(out, template.actions());
    uint(out, 8);
    match template.pass_on() {
        PassOn::UseOnly => uint(out, 0),
        PassOn::To {
            actions,
            recipients,
        } => {
            map(out, 2);
            uint(out, 1);
            write_actions(out, actions);
            uint(out, 2);
            head(out, MAJOR_ARRAY, recipients.len() as u64);
            for kind in recipients {
                uint(out, recipient_code(*kind));
            }
        }
    }
    uint(out, 10);
    map(out, 2);
    uint(out, 1);
    uint(out, template.window().starts_at());
    uint(out, 2);
    out.push(NULL);
}

/// The canonical bytes of `template`.
pub fn encode_template(template: &Template) -> Vec<u8> {
    let mut out = Vec::new();
    write_template(&mut out, template);
    out
}

fn read_template(value: Value) -> Result<Template, RoleError> {
    const SHAPE: &str = "a template is a map of the grant keys 4, 5, 6, 7, 8 and 10";
    let mut fields = Fields::read(value, SHAPE)?;
    if fields.has(3) {
        return Err(RoleError::TemplateHasHolder);
    }
    if fields.has(9) {
        return Err(RoleError::TemplateHasSource);
    }
    let responsible = PersonId::from_bytes(as_id(fields.take(4)?, SHAPE)?);
    let resource = read_resource(fields.take(5)?)?;
    let relation = Relation::new(&as_text(fields.take(6)?, SHAPE)?)?;
    let actions = read_actions(fields.take(7)?)?;
    let pass_on = read_pass_on(fields.take(8)?)?;
    let window = read_window(fields.take(10)?)?;
    fields.done()?;
    Template::new(TemplateParts {
        relation,
        resource,
        actions,
        pass_on,
        window,
        responsible,
    })
}

/// The template `encoded` holds, refused unless it is the exact canonical encoding.
pub fn decode_template(encoded: &[u8]) -> Result<Template, RoleError> {
    let template = read_template(parse(encoded, "a template is one CBOR map")?)?;
    if encode_template(&template) != encoded {
        return Err(RoleError::EventNotCanonical);
    }
    Ok(template)
}

/// Append `ids` as an array of 16-byte grant ids.
pub(super) fn write_ids(out: &mut Vec<u8>, ids: &[GrantId]) {
    head(out, MAJOR_ARRAY, ids.len() as u64);
    for id in ids {
        bytes(out, id.as_bytes());
    }
}

/// The canonical bytes of `holding`.
pub fn encode_holding(holding: &Holding) -> Vec<u8> {
    let mut out = Vec::new();
    map(&mut out, 8);
    uint(&mut out, 1);
    bytes(&mut out, holding.id.as_bytes());
    uint(&mut out, 2);
    bytes(&mut out, holding.holder.as_bytes());
    uint(&mut out, 3);
    bytes(&mut out, holding.role.as_bytes());
    uint(&mut out, 4);
    uint(&mut out, holding.version);
    uint(&mut out, 5);
    write_resource(&mut out, &holding.project);
    uint(&mut out, 6);
    write_ids(&mut out, &holding.grants);
    uint(&mut out, 7);
    match holding.ends_at {
        Some(ends) => uint(&mut out, ends),
        None => out.push(NULL),
    }
    uint(&mut out, 8);
    text(&mut out, holding.policy.as_str());
    out
}

fn read_holding(value: Value) -> Result<Holding, RoleError> {
    const SHAPE: &str = "a holding is a map of keys 1 to 8";
    let mut fields = Fields::read(value, SHAPE)?;
    if !fields.has(8) {
        return Err(RoleError::MissingPolicy);
    }
    let policy: MovePolicy = as_text(fields.take(8)?, SHAPE)?.parse()?;
    let holding = Holding {
        id: HoldingId::from_bytes(as_id(fields.take(1)?, SHAPE)?),
        holder: AgentId::from_bytes(as_id(fields.take(2)?, SHAPE)?),
        role: RoleId::from_bytes(as_id(fields.take(3)?, SHAPE)?),
        version: as_uint(&fields.take(4)?, SHAPE)?,
        project: read_resource(fields.take(5)?)?,
        grants: as_ids(fields.take(6)?, SHAPE)?,
        ends_at: as_end(fields.take(7)?, SHAPE)?,
        policy,
    };
    fields.done()?;
    holding.check()?;
    Ok(holding)
}

/// The holding `encoded` holds, refused unless it is the exact canonical encoding.
pub fn decode_holding(encoded: &[u8]) -> Result<Holding, RoleError> {
    let holding = read_holding(parse(encoded, "a holding is one CBOR map")?)?;
    if encode_holding(&holding) != encoded {
        return Err(RoleError::EventNotCanonical);
    }
    Ok(holding)
}

fn read_policy(value: Value, reason: &'static str) -> Result<MovePolicy, RoleError> {
    as_text(value, reason)?.parse()
}

fn read_version_made(value: Value) -> Result<RoleChange, RoleError> {
    const SHAPE: &str = "a version made is a map of keys 1 to 4, and 5 and 6 for version 1";
    let mut fields = Fields::read(value, SHAPE)?;
    let role = RoleId::from_bytes(as_id(fields.take(1)?, SHAPE)?);
    let project = read_resource(fields.take(2)?)?;
    let version = as_uint(&fields.take(3)?, SHAPE)?;
    let templates = as_array(fields.take(4)?, SHAPE)?
        .into_iter()
        .map(read_template)
        .collect::<Result<Vec<_>, RoleError>>()?;
    let opening = if fields.has(5) {
        Some(Opening {
            title: title(&as_text(fields.take(5)?, SHAPE)?)?,
            default_policy: read_policy(fields.take(6)?, SHAPE)?,
        })
    } else {
        None
    };
    fields.done()?;
    if (version == 1) != opening.is_some() {
        return Err(malformed(
            "version 1 opens its role with a title and default policy, and no later version does",
        ));
    }
    Ok(RoleChange::VersionMade {
        role,
        project,
        version,
        templates,
        opening,
    })
}

fn read_change(kind: u64, value: Value) -> Result<RoleChange, RoleError> {
    const SHAPE: &str = "a role change is the map its kind names";
    if kind == 1 {
        return read_version_made(value);
    }
    if kind == 4 {
        return Ok(RoleChange::HoldingGranted(Box::new(read_holding(value)?)));
    }
    let mut fields = Fields::read(value, SHAPE)?;
    let id = as_id(fields.take(1)?, SHAPE)?;
    let change = match kind {
        2 => RoleChange::TitleChanged {
            role: RoleId::from_bytes(id),
            before: as_text(fields.take(2)?, SHAPE)?,
            after: title(&as_text(fields.take(3)?, SHAPE)?)?,
        },
        3 => RoleChange::DefaultPolicyChanged {
            role: RoleId::from_bytes(id),
            before: read_policy(fields.take(2)?, SHAPE)?,
            after: read_policy(fields.take(3)?, SHAPE)?,
        },
        5 => RoleChange::HoldingMoved {
            holding: HoldingId::from_bytes(id),
            from: as_uint(&fields.take(2)?, SHAPE)?,
            to: as_uint(&fields.take(3)?, SHAPE)?,
            timing: as_text(fields.take(4)?, SHAPE)?.parse::<Timing>()?,
            added: as_ids(fields.take(5)?, SHAPE)?,
            removed: as_ids(fields.take(6)?, SHAPE)?,
        },
        6 => RoleChange::HoldingRenewed {
            holding: HoldingId::from_bytes(id),
            version: as_uint(&fields.take(2)?, SHAPE)?,
            ends_at: as_uint(&fields.take(3)?, SHAPE)?,
            grants: as_ids(fields.take(4)?, SHAPE)?,
            replaced: as_ids(fields.take(5)?, SHAPE)?,
        },
        7 => RoleChange::HoldingPolicyChanged {
            holding: HoldingId::from_bytes(id),
            before: read_policy(fields.take(2)?, SHAPE)?,
            after: read_policy(fields.take(3)?, SHAPE)?,
        },
        _ => return Err(malformed("a role change kind is 1 to 7")),
    };
    fields.done()?;
    Ok(change)
}

/// The role event a body's CBOR value carries.
pub(super) fn read_body(value: Value) -> Result<RoleEvent, RoleError> {
    const SHAPE: &str = "a role event body is a map of keys 1 to 7";
    let mut fields = Fields::read(value, SHAPE)?;
    if as_uint(&fields.take(1)?, SHAPE)? != ROLE_EVENT_VERSION {
        return Err(malformed("this crate reads role event version 1"));
    }
    let operation = OperationId::from_bytes(as_id(fields.take(2)?, SHAPE)?);
    let actor = read_identity(fields.take(3)?)?;
    let capacity: Capacity = as_text(fields.take(4)?, SHAPE)?.parse()?;
    let recorded_at = as_uint(&fields.take(5)?, SHAPE)?;
    let kind = as_uint(&fields.take(6)?, SHAPE)?;
    let change = read_change(kind, fields.take(7)?)?;
    fields.done()?;
    RoleEvent::new(operation, actor, capacity, recorded_at, change)
}
