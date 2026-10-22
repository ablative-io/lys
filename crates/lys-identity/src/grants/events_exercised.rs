//! What a use exercised: the act and the resource the check that permitted
//! it was asked (AGENTS-019 R5).
//!
//! A use event written before this names its grant and its route alone, and
//! a grant carries every action of its relation, so such a use never says
//! which act it was. A use the check records now names the act and the
//! resource too, as keys `3` (the action, text) and `4` (the resource, a map
//! of `1` kind and `2` id) of its change map, under version 8,
//! [`EXERCISED_ENVELOPE`], so no earlier event's bytes change. The resource
//! is the one whose grant permits: for Lys's own kinds, which are placed in
//! no parent, the resource acted on.

use ciborium::Value;

use super::{GrantError, as_text, fields};
use crate::encoding::{map, text, uint};
use crate::grants::types::{Action, Resource};

/// Separate signed envelope for a use event that names what it exercised.
pub const EXERCISED_ENVELOPE: &str = "application/vnd.lys.grant-event.v8+cbor";

/// The act a use exercised and the resource it was asked on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exercised {
    /// The action exercised.
    pub action: Action,
    /// The resource the check that permitted it was asked on.
    pub resource: Resource,
}

/// Keys `3` and `4` of a use's change map.
pub(super) fn write(out: &mut Vec<u8>, exercised: &Exercised) {
    uint(out, 3);
    text(out, exercised.action.as_str());
    uint(out, 4);
    map(out, 2);
    uint(out, 1);
    text(out, exercised.resource.kind());
    uint(out, 2);
    text(out, exercised.resource.id());
}

/// What keys `3` and `4` of a use's change map name.
///
/// # Errors
/// Refuses an action or resource of the wrong shape, or tokens the grants
/// refuse.
pub(super) fn read(action: Value, resource: Value) -> Result<Exercised, GrantError> {
    let [kind, id] = fields::<2>(resource, "an exercised resource is a map of keys 1 and 2")?;
    Ok(Exercised {
        action: Action::new(&as_text(action, "an exercised action is text")?)?,
        resource: Resource::new(
            &as_text(kind, "a resource kind is text")?,
            &as_text(id, "a resource id is text")?,
        )?,
    })
}
