//! Reading canonical identity event bodies and their authenticated actors.

use super::{
    Actor, AgentId, Change, EVENT_VERSION, IdentityError, IdentityEvent, IdentityId,
    LifecycleState, LinkObservation, LoginBinding, OperationId, PersonId, Profile, Provenance,
    Value, as_id, as_text, as_uint, cbor, encode_body, fields, malformed, reporting, wire,
};

pub(super) fn decode_profile(value: Value) -> Result<Profile, IdentityError> {
    let [name] = fields::<1>(value, "a profile is a map of key 1")?;
    Profile::new(&as_text(name, "a display name is text")?)
}

fn state(value: &Value) -> Result<LifecycleState, IdentityError> {
    wire::state_from(as_uint(value, "a lifecycle state is a code")?)
        .ok_or_else(|| malformed("a lifecycle state code is 1 to 4"))
}

fn decode_change(kind: u64, value: Value) -> Result<Change, IdentityError> {
    const TEXT: &str = "a change field is text";
    match kind {
        wire::SETUP_PERSON => {
            let [shown] = fields::<1>(value, "a setup is a map of key 1")?;
            Ok(Change::SetupPerson {
                profile: decode_profile(shown)?,
            })
        }
        wire::REGISTER_PERSON => {
            let [shown] = fields::<1>(value, "a person registration is a map of key 1")?;
            Ok(Change::RegisterPerson {
                profile: decode_profile(shown)?,
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
        wire::BIND_LOGIN => {
            let [issuer, subject] = fields::<2>(value, "a login binding is a map of keys 1 and 2")?;
            Ok(Change::BindLogin {
                binding: LoginBinding::new(&as_text(issuer, TEXT)?, &as_text(subject, TEXT)?)?,
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
        _ => Err(malformed("a change kind is 1 to 10")),
    }
}

/// The actor an actor's map names: keys 1 to 4, and the agent's id under key
/// 5 for an agent signature or service-account bearer. Code 3 without key 5
/// is the operator; code 3 with a 16-byte key 5 is a service account.
pub(crate) fn decode_actor(value: Value, version: u64) -> Result<Actor, IdentityError> {
    const SHAPE: &str = "an actor is a map of keys 1 to 4, or 1 to 5 when it names a principal";
    let (value, draft) = if matches!(&value,Value::Map(pairs) if pairs.len()==6) {
        let [issuer, subject, method, at, principal, origin] = fields::<6>(value, SHAPE)?;
        let Value::Array(parts) = origin else {
            return Err(malformed("draft provenance is an array"));
        };
        let [draft, application, agent]: [Value; 3] = parts
            .try_into()
            .map_err(|_| malformed("draft provenance has three fields"))?;
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
    Ok(if let Some(draft) = draft {
        actor.with_draft(draft)
    } else {
        actor
    })
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
        _ => {
            return Err(malformed(
                "a directory event's identity kind code is 1 for a person or 2 for an agent",
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
