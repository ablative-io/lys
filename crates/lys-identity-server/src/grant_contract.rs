//! The grant routes' typed wire: every request body, and the JSON each answer
//! is written as.
//!
//! Every member of every body is required, and an unknown member is refused.
//! A body is only a request: parsing it grants nothing, and every decision is
//! made by the grants' one authority owner behind the route.

use std::collections::BTreeSet;
use std::str::FromStr;

use lys_identity::grants::{
    Action, DelegateRequest, GrantId, GrantRecord, PassOn, Permit, RecipientKind, Recorded,
    Relation, Resource, RevokeRequest, RootRequest, Route, Source, Window,
};
use lys_identity::{IdentityId, OperationId, PersonId};
use serde::{Deserialize, Deserializer};
use serde_json::{Value, json};

use crate::error::ServerError;
use crate::routes::{hex, identity_id};

/// The most holders one page of a who-can answer carries.
pub const PAGE_MAX: usize = 100;

fn nullable<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> Result<Option<T>, D::Error> {
    Option::deserialize(deserializer)
}

/// How a request says it arrived. It is recorded, and never changes a decision.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteWire {
    /// The browser screens.
    Browser,
    /// The API, called directly.
    Api,
    /// An agent's tool.
    Tool,
}

impl From<RouteWire> for Route {
    fn from(route: RouteWire) -> Self {
        match route {
            RouteWire::Browser => Self::Browser,
            RouteWire::Api => Self::Api,
            RouteWire::Tool => Self::Tool,
        }
    }
}

/// A resource, by kind and id.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceWire {
    kind: String,
    id: String,
}

impl ResourceWire {
    fn resource(&self) -> Result<Resource, ServerError> {
        Ok(Resource::new(&self.kind, &self.id)?)
    }
}

/// What a grant lets its holder pass on, stated affirmatively.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PassOnWire {
    /// Exercise only.
    UseOnly,
    /// These actions, to these kinds of recipient.
    To {
        /// The actions that may be passed on.
        actions: Vec<String>,
        /// The kinds of recipient, `person` or `agent`.
        recipients: Vec<String>,
    },
}

fn recipient_kind(text: &str) -> Result<RecipientKind, ServerError> {
    match text {
        "person" => Ok(RecipientKind::Person),
        "agent" => Ok(RecipientKind::Agent),
        other => Err(ServerError::RequestMalformed {
            reason: format!("{other} is not a recipient kind, which is person or agent"),
        }),
    }
}

impl PassOnWire {
    fn pass_on(&self) -> Result<PassOn, ServerError> {
        match self {
            Self::UseOnly => Ok(PassOn::UseOnly),
            Self::To {
                actions,
                recipients,
            } => {
                let actions = actions
                    .iter()
                    .map(|action| Action::new(action))
                    .collect::<Result<BTreeSet<_>, _>>()?;
                let recipients = recipients
                    .iter()
                    .map(|kind| recipient_kind(kind))
                    .collect::<Result<BTreeSet<_>, _>>()?;
                Ok(PassOn::to(actions, recipients)?)
            }
        }
    }
}

/// When a grant may be exercised. `ends_at` is required, and null means no end of its own.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WindowWire {
    starts_at: u64,
    #[serde(deserialize_with = "nullable")]
    ends_at: Option<u64>,
}

impl WindowWire {
    fn window(self) -> Result<Window, ServerError> {
        Ok(Window::new(self.starts_at, self.ends_at)?)
    }
}

fn operation(text: &str) -> Result<OperationId, ServerError> {
    Ok(OperationId::from_str(text)?)
}

/// A grant id as a path or body gives it.
pub fn grant_id(text: &str) -> Result<GrantId, ServerError> {
    Ok(GrantId::from_str(text)?)
}

/// A request to issue a root grant to a person.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RootBody {
    operation: String,
    route: RouteWire,
    holder: String,
    resource: ResourceWire,
    relation: String,
    pass_on: PassOnWire,
    window: WindowWire,
}

impl RootBody {
    /// The request, made by `caller`.
    pub fn request(&self, caller: IdentityId) -> Result<RootRequest, ServerError> {
        Ok(RootRequest {
            operation: operation(&self.operation)?,
            caller,
            route: self.route.into(),
            holder: PersonId::from_str(&self.holder)?,
            resource: self.resource.resource()?,
            relation: Relation::new(&self.relation)?,
            pass_on: self.pass_on.pass_on()?,
            window: self.window.window()?,
        })
    }
}

/// A request to pass on part of a grant the caller holds.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DelegateBody {
    operation: String,
    route: RouteWire,
    source: String,
    recipient: String,
    responsible: String,
    resource: ResourceWire,
    relation: String,
    pass_on: PassOnWire,
    window: WindowWire,
}

impl DelegateBody {
    /// The request, made by `caller`.
    pub fn request(&self, caller: IdentityId) -> Result<DelegateRequest, ServerError> {
        Ok(DelegateRequest {
            operation: operation(&self.operation)?,
            caller,
            route: self.route.into(),
            source: grant_id(&self.source)?,
            recipient: identity_id(&self.recipient)?,
            responsible: PersonId::from_str(&self.responsible)?,
            resource: self.resource.resource()?,
            relation: Relation::new(&self.relation)?,
            pass_on: self.pass_on.pass_on()?,
            window: self.window.window()?,
        })
    }
}

/// A request to revoke a grant, and everything derived from it.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RevokeBody {
    operation: String,
    route: RouteWire,
    reason: String,
}

impl RevokeBody {
    /// The request to revoke `grant`, made by `caller`.
    pub fn request(
        &self,
        caller: IdentityId,
        grant: GrantId,
    ) -> Result<RevokeRequest, ServerError> {
        Ok(RevokeRequest {
            operation: operation(&self.operation)?,
            caller,
            route: self.route.into(),
            grant,
            reason: self.reason.clone(),
        })
    }
}

/// A question about one action on one resource: why the caller may or may not
/// exercise it, or who can.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionBody {
    route: RouteWire,
    resource: ResourceWire,
    action: String,
}

impl ActionBody {
    /// The route, resource and action asked about.
    pub fn parts(&self) -> Result<(Route, Resource, Action), ServerError> {
        Ok((
            self.route.into(),
            self.resource.resource()?,
            Action::new(&self.action)?,
        ))
    }
}

/// A page of the who-can question. `after` is required, null for the first page.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WhoBody {
    /// The action and resource asked about.
    #[serde(flatten)]
    pub question: ActionBody,
    /// How many holders one page carries, 1 to [`PAGE_MAX`].
    pub page_size: usize,
    /// The last grant of the previous page, or null.
    #[serde(deserialize_with = "nullable")]
    pub after: Option<String>,
}

fn pass_on_json(pass_on: &PassOn) -> Value {
    match pass_on {
        PassOn::UseOnly => json!({ "kind": "use_only" }),
        PassOn::To {
            actions,
            recipients,
        } => json!({
            "kind": "to",
            "actions": actions.iter().map(Action::as_str).collect::<Vec<_>>(),
            "recipients": recipients.iter().map(ToString::to_string).collect::<Vec<_>>(),
        }),
    }
}

/// A grant, as a caller who may inspect it reads it.
pub fn grant_json(record: &GrantRecord) -> Value {
    let grant = record.grant();
    let parts = grant.parts();
    json!({
        "id": grant.id().to_string(),
        "issuer": parts.issuer.to_string(),
        "holder": parts.holder.to_string(),
        "responsible": parts.responsible.to_string(),
        "resource": { "kind": parts.resource.kind(), "id": parts.resource.id() },
        "relation": parts.relation.as_str(),
        "actions": parts.actions.iter().map(Action::as_str).collect::<Vec<_>>(),
        "pass_on": pass_on_json(&parts.pass_on),
        "source": match parts.source {
            Source::Root => Value::Null,
            Source::Grant(id) => Value::String(id.to_string()),
        },
        "window": { "starts_at": parts.window.starts_at(), "ends_at": parts.window.ends_at() },
        "model_version": parts.model_version,
        "operation": parts.operation.to_string(),
        "revoked": record.revoked().is_some(),
    })
}

/// A recorded change and its receipt.
pub fn recorded_json(recorded: &Recorded) -> Value {
    let receipt = &recorded.receipt;
    let coordinate = receipt.coordinate;
    json!({
        "operation": receipt.operation.to_string(),
        "grant": receipt.grant.to_string(),
        "index": recorded.index,
        "receipt": {
            "version": receipt.version,
            "caller": receipt.caller.to_string(),
            "change_kind": receipt.change_kind,
            "payload_commitment": hex(&receipt.payload_commitment),
            "payload_commitment_hash": "sha-256",
            "revision": receipt.revision(),
            "log": {
                "index": coordinate.index,
                "tree_size": coordinate.tree_size,
                "root": hex(&coordinate.root),
                "leaf_hash": hex(&coordinate.leaf_hash),
            },
        },
    })
}

/// A permitted decision, with the authority path it rests on.
pub fn permit_json(permit: &Permit) -> Value {
    json!({
        "permitted": true,
        "grant": permit.grant.to_string(),
        "path": permit.path.iter().map(ToString::to_string).collect::<Vec<_>>(),
        "responsible": permit.root_person.to_string(),
        "scope": permit.actions.iter().map(Action::as_str).collect::<Vec<_>>(),
        "model_version": permit.model_version,
        "revision": permit.revision,
    })
}
