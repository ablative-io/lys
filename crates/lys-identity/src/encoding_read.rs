//! Reading canonical identity event bodies and their authenticated actors.

use super::{
    Actor, AgentId, Change, EVENT_VERSION, IdentityError, IdentityEvent, IdentityId,
    LifecycleState, LinkObservation, LoginBinding, OperationId, PersonId, Profile, Provenance,
    Value, as_id, as_text, as_uint, cbor, encode_body, fields, malformed, reporting, wire,
};
use crate::id::OrganisationId;

pub(super) fn decode_profile(value: Value) -> Result<Profile, IdentityError> {
    let [name] = fields::<1>(value, "a profile is a map of key 1")?;
    Profile::new(&as_text(name, "a display name is text")?)
}

/// A person registration's profile and organisation: a map of key 1 alone
/// for a leaf written before organisations, of keys 1 and 2 after.
fn decode_registration(
    value: Value,
    reason: &'static str,
) -> Result<(Profile, Option<OrganisationId>), IdentityError> {
    if matches!(&value, Value::Map(pairs) if pairs.len() == 2) {
        let [shown, organisation] = fields::<2>(value, reason)?;
        return Ok((
            decode_profile(shown)?,
            Some(OrganisationId::from_bytes(as_id(
                organisation,
                "an organisation is 16 bytes",
            )?)),
        ));
    }
    let [shown] = fields::<1>(value, reason)?;
    Ok((decode_profile(shown)?, None))
}

fn state(value: &Value) -> Result<LifecycleState, IdentityError> {
    wire::state_from(as_uint(value, "a lifecycle state is a code")?)
        .ok_or_else(|| malformed("a lifecycle state code is 1 to 4"))
}

fn decode_change(kind: u64, value: Value) -> Result<Change, IdentityError> {
    const TEXT: &str = "a change field is text";
    match kind {
        wire::SETUP_PERSON => {
            let (profile, organisation) =
                decode_registration(value, "a setup is a map of key 1, or keys 1 and 2")?;
            Ok(Change::SetupPerson {
                profile,
                organisation,
            })
        }
        wire::REGISTER_PERSON => {
            let (profile, organisation) = decode_registration(
                value,
                "a person registration is a map of key 1, or keys 1 and 2",
            )?;
            Ok(Change::RegisterPerson {
                profile,
                organisation,
            })
        }
        wire::FOUND_ORGANISATION => {
            let [shown, founder] = fields::<2>(value, "a founding is a map of keys 1 and 2")?;
            Ok(Change::FoundOrganisation {
                profile: decode_profile(shown)?,
                founder: PersonId::from_bytes(as_id(founder, "a founder is 16 bytes")?),
            })
        }
        wire::ATTACH_ORGANISATION => {
            let [organisation] = fields::<1>(value, "an attachment is a map of key 1")?;
            Ok(Change::AttachOrganisation {
                organisation: OrganisationId::from_bytes(as_id(
                    organisation,
                    "an organisation is 16 bytes",
                )?),
            })
        }
        wire::REGISTER_AGENT => {
            let [responsible, shown] =
                fields::<2>(value, "an agent registration is a map of keys 1 and 2")?;
            Ok(Change::RegisterAgent {
                responsible: PersonId::from_bytes(as_id(
                    responsible,
                    "a responsible person is 16 bytes",
                )?),
                profile: decode_profile(shown)?,
            })
        }
        wire::CHANGE_PROFILE => {
            let [shown] = fields::<1>(value, "a profile change is a map of key 1")?;
            Ok(Change::ChangeProfile {
                profile: decode_profile(shown)?,
            })
        }
        wire::BIND_LOGIN | wire::UNBIND_LOGIN => {
            let [issuer, subject] = fields::<2>(value, "a login binding is a map of keys 1 and 2")?;
            let binding = LoginBinding::new(&as_text(issuer, TEXT)?, &as_text(subject, TEXT)?)?;
            Ok(if kind == wire::UNBIND_LOGIN {
                Change::UnbindLogin { binding }
            } else {
                Change::BindLogin { binding }
            })
        }
        wire::TRANSITION => {
            let [transition, from, to, reason] =
                fields::<4>(value, "a transition is a map of keys 1 to 4")?;
            let transition = wire::transition_from(as_uint(&transition, "a transition is a code")?)
                .ok_or_else(|| malformed("a transition code is 1 to 4"))?;
            Ok(Change::Transition {
                transition,
                from: state(&from)?,
                to: state(&to)?,
                reason: as_text(reason, TEXT)?,
            })
        }
        wire::LINK_AUDIT => {
            let [source, change, issuer, subject, observer, observed_at] =
                fields::<6>(value, "a link observation is a map of keys 1 to 6")?;
            let change = wire::link_from(as_uint(&change, "a link change is a code")?)
                .ok_or_else(|| malformed("a link change code is 1 or 2"))?;
            let binding = LoginBinding::new(&as_text(issuer, TEXT)?, &as_text(subject, TEXT)?)?;
            Ok(Change::LinkAudit(LinkObservation::new(
                &as_text(source, TEXT)?,
                change,
                binding,
                &as_text(observer, TEXT)?,
                as_uint(&observed_at, "an observation time is seconds")?,
            )?))
        }
        wire::REPORTING_REGISTRATION | wire::REPORTS_TO_CHANGED => reporting::decode(kind, value),
        wire::AGENT_CALL => {
            let [method, path, digest, signature] =
                fields::<4>(value, "an agent call is a map of keys 1 to 4")?;
            Ok(Change::AgentCall(crate::agent_call::AgentCall::new(
                &as_text(method, TEXT)?,
                &as_text(path, TEXT)?,
                &as_text(digest, TEXT)?,
                &as_text(signature, TEXT)?,
            )?))
        }
        wire::ADMISSION_WITHDRAWN => {
            let [digest] = fields::<1>(value, "an admission withdrawal is a map of key 1")?;
            Ok(Change::AdmissionWithdrawn {
                digest: as_text(digest, TEXT)?,
            })
        }
        _ => Err(malformed("a change kind is 1 to 13")),
    }
}

/// The actor an actor's map names: keys 1 to 4, and the agent's id under key
/// 5 for an agent signature or service-account bearer. Code 3 without key 5
/// is the operator; code 3 with a 16-byte key 5 is a service account.
pub(crate) fn decode_actor(value: Value, version: u64) -> Result<Actor, IdentityError> {
    const SHAPE: &str = "an actor is a map of keys 1 to 4, or 1 to 5 when it names a principal";
    let (value, through) = through_app(value)?;
    let (value, draft) = if matches!(&value,Value::Map(pairs) if pairs.len()==6) {
        let [issuer, subject, method, at, principal, origin] = fields::<6>(value, SHAPE)?;
        let Value::Array(parts) = origin else {
            return Err(malformed("draft provenance is an array"));
        };
        let [draft, application, agent]: [Value; 3] = parts
            .try_into()
            .ok()
            .ok_or_else(|| malformed("draft provenance has three fields"))?;
        let origin = crate::provenance::DraftProvenance {
            draft: as_text(draft, "a draft id is text")?.parse()?,
            application: as_text(application, "an application id is text")?.parse()?,
            drafted_by: if agent == Value::Null {
                None
            } else {
                Some(as_text(agent, "a draft author is text")?.parse()?)
            },
        };
        let mut pairs = vec![
            (1.into(), issuer),
            (2.into(), subject),
            (3.into(), method),
            (4.into(), at),
        ];
        if principal != Value::Null {
            pairs.push((5.into(), principal));
        }
        (Value::Map(pairs), Some(origin))
    } else {
        (value, None)
    };
    let named = matches!(&value, Value::Map(pairs) if pairs.len() == 5);
    let ([issuer, subject, method, authenticated_at], agent) = if named {
        let [issuer, subject, method, authenticated_at, agent] = fields::<5>(value, SHAPE)?;
        let agent = AgentId::from_bytes(as_id(
            agent,
            "actor key 5 must hold a 16-byte principal id",
        )?);
        ([issuer, subject, method, authenticated_at], Some(agent))
    } else {
        (fields::<4>(value, SHAPE)?, None)
    };
    let code = as_uint(&method, "an authentication method is a code")?;
    if code == wire::OPERATOR_OR_SERVICE_ACCOUNT && version == EVENT_VERSION && named {
        return Err(malformed(
            "an operator actor carries no principal id under key 5",
        ));
    }
    let method = wire::method_from(code, agent).map_err(malformed)?;
    let provenance = match authenticated_at {
        Value::Array(run) if code == 4 => {
            let Ok([launch, session]) = <[Value; 2]>::try_from(run) else {
                return Err(malformed("pass provenance is a launch and session pair"));
            };
            Provenance::by_pass(
                agent.ok_or_else(|| malformed("pass provenance requires an agent"))?,
                &as_text(launch, "a launch is text")?,
                &as_text(session, "a session is text")?,
            )?
        }
        time => Provenance::new(method, as_uint(&time, "an authentication time is seconds")?),
    };
    let actor = Actor::new(
        LoginBinding::new(
            &as_text(issuer, "an actor's issuer is text")?,
            &as_text(subject, "an actor's subject is text")?,
        )?,
        provenance,
    );
    let actor = if let Some(draft) = draft {
        actor.with_draft(draft)
    } else {
        actor
    };
    Ok(match through {
        Some(app) => actor.with_through_app(&app),
        None => actor,
    })
}

/// An actor's map with the app it came through under key 7 (DIRECTORY-113
/// Amendment 2) answered apart, and the map as it reads without it: keys 5
/// and 6 kept only as the shapes before it write them.
fn through_app(value: Value) -> Result<(Value, Option<String>), IdentityError> {
    const SHAPE: &str = "an actor through an app is a map of keys 1 to 7";
    if !matches!(&value, Value::Map(pairs) if pairs.len() == 7) {
        return Ok((value, None));
    }
    let [issuer, subject, method, at, principal, origin, app] = fields::<7>(value, SHAPE)?;
    let app = as_text(app, "an app is text")?;
    if app.is_empty() {
        return Err(malformed("an app is named"));
    }
    let mut pairs = vec![
        (1.into(), issuer),
        (2.into(), subject),
        (3.into(), method),
        (4.into(), at),
    ];
    if origin != Value::Null {
        pairs.push((5.into(), principal));
        pairs.push((6.into(), origin));
    } else if principal != Value::Null {
        pairs.push((5.into(), principal));
    }
    Ok((Value::Map(pairs), Some(app)))
}

/// The highest change kind the build before organisations (043cc007) read:
/// its decoder answered any higher kind `EventMalformed` at the arm every
/// kind it did not know fell to, and read a person registration as a map
/// of exactly key 1.
pub const HIGHEST_KIND_BEFORE_ORGANISATIONS: u64 = 10;

/// The event `body` names as the build before organisations read it
/// (DIRECTORY-109 R2): a founding or an attachment, a change kind above
/// [`HIGHEST_KIND_BEFORE_ORGANISATIONS`], is refused `EventMalformed` with
/// that build's words, and so is a person registration or setup naming an
/// organisation, a map of two keys where it read one. So a store brought
/// under an organisation is unreadable to a put-back, never silently read
/// as narrower than it is; the copy the upgrade kept is the put-back.
///
/// # Errors
/// - `IdentityError::EventMalformed` if the previous build would have refused the body, or this one does.
/// - Every other refusal of [`decode_body`].
pub fn decode_body_before_organisations(body: &[u8]) -> Result<IdentityEvent, IdentityError> {
    let event = decode_body(body)?;
    match event.change() {
        Change::FoundOrganisation { .. }
        | Change::AttachOrganisation { .. }
        | Change::AdmissionWithdrawn { .. }
        | Change::UnbindLogin { .. } => Err(malformed("a change kind is 1 to 10")),
        Change::RegisterPerson {
            organisation: Some(_),
            ..
        } => Err(malformed("a person registration is a map of key 1")),
        Change::SetupPerson {
            organisation: Some(_),
            ..
        } => Err(malformed("a setup is a map of key 1")),
        _ => Ok(event),
    }
}

/// The event an event body's bytes name, refused unless the bytes are its canonical encoding.
///
/// # Errors
/// - `IdentityError::EventMalformed` if CBOR fields, wire codes or provenance have the wrong shape.
/// - `IdentityError::VersionUnsupported` if the event version is not supported.
/// - `IdentityError::BindingMalformed` if an actor or changed login is invalid.
/// - `IdentityError::ProfileInvalid` if a decoded display profile is invalid.
/// - `IdentityError::ChangeMismatch` if a change does not fit its identity or required fields.
/// - `IdentityError::TransitionRefused` if a decoded lifecycle transition is undefined.
/// - `IdentityError::ReasonRequired` if a decoded suspension or retirement has no reason.
/// - `IdentityError::EventNotCanonical` if re-encoding does not reproduce the input bytes.
pub fn decode_body(body: &[u8]) -> Result<IdentityEvent, IdentityError> {
    const SHAPE: &str = "the body is a map of keys 1 to 7";
    let value = cbor(body, SHAPE)?;
    if let Value::Map(pairs) = &value
        && let Some((_, version)) = pairs.first()
    {
        let version = as_uint(version, SHAPE)?;
        if version != EVENT_VERSION && version != 2 {
            return Err(IdentityError::VersionUnsupported { version });
        }
    }
    let [
        version,
        operation,
        actor,
        identity,
        recorded_at,
        kind,
        change,
    ] = fields::<7>(value, SHAPE)?;
    let version = as_uint(&version, SHAPE)?;
    let actor = decode_actor(actor, version)?;
    let [identity_kind, identity_id] =
        fields::<2>(identity, "an identity is a map of keys 1 and 2")?;
    let identity_id = as_id(identity_id, "an identity id is 16 bytes")?;
    let identity = match as_uint(&identity_kind, "an identity kind is a code")? {
        wire::PERSON => IdentityId::Person(PersonId::from_bytes(identity_id)),
        wire::AGENT => IdentityId::Agent(AgentId::from_bytes(identity_id)),
        wire::ORGANISATION => IdentityId::Organisation(OrganisationId::from_bytes(identity_id)),
        _ => {
            return Err(malformed(
                "a directory event's identity kind code is 1 for a person, 2 for an agent or 6 for an organisation",
            ));
        }
    };
    let event = IdentityEvent::new(
        OperationId::from_bytes(as_id(operation, "an operation id is 16 bytes")?),
        actor,
        identity,
        as_uint(&recorded_at, "a recorded time is seconds")?,
        decode_change(as_uint(&kind, "a change kind is a code")?, change)?,
    )?;
    if event.version() != version {
        return Err(malformed(
            "event version does not match actor authentication method",
        ));
    }
    if encode_body(&event) != body {
        return Err(IdentityError::EventNotCanonical);
    }
    Ok(event)
}
