//! One canonical variable scope is mapped without reading secret values.

use super::{
    CREDENTIAL_INLINE, Completeness, DestinationEntry, Excluded, Fragment, Identity,
    MEMBER_UNSUPPORTED, NANOS, SCOPE_AMBIGUOUS, Scope, SourceEntry, VARIABLES, Value, empty,
    incomplete, json, reached, refusal, refused_api, secret_name, secret_shaped,
};

/// One canonical scope's variables, each live one mapped at the matching
/// Lys scope with its author and absolute expiry.
pub(super) fn variables(
    body: &Value,
    scope: &str,
    who: &Identity<'_>,
    captured_at: u64,
    base: &str,
) -> Fragment {
    let member = format!("{base}{VARIABLES} {scope}");
    if body.get("id").and_then(Value::as_str) != Some(scope) {
        let detail = "the monitor answered for another scope than the canonical one asked";
        let refused = refusal(SCOPE_AMBIGUOUS, &member, detail);
        return refused_api(base, VARIABLES, Some(body), refused);
    }
    let revision = body.get("revision").and_then(Value::as_u64);
    let held = body.get("entries").and_then(Value::as_object);
    let (Some(revision), Some(held)) = (revision, held) else {
        return incomplete(
            base,
            VARIABLES,
            body,
            "the answer carries no revision or entries",
        );
    };
    let lys = match who.session.filter(|_| scope.starts_with("session:")) {
        Some(session) => Scope::Session {
            id: session.to_owned(),
        },
        None => Scope::Agent {
            id: who.lys_agent.to_owned(),
        },
    };
    let source_id = format!("monitor:variables:{scope}");
    let mut fragment = empty();
    fragment.sources.push(SourceEntry {
        id: source_id.clone(),
        kind: "monitor_variables".to_owned(),
        locator: member,
        scope: scope.to_owned(),
        revision_kind: "native".to_owned(),
        source_revision: revision.to_string(),
        completeness: Completeness::Complete,
    });
    for (name, entry) in held {
        variable(
            &mut fragment,
            (name, entry),
            scope,
            who,
            &lys,
            (&source_id, revision),
            captured_at,
        );
    }
    fragment
}

fn variable(
    fragment: &mut Fragment,
    (name, entry): (&str, &Value),
    scope: &str,
    who: &Identity<'_>,
    lys: &Scope,
    (source_id, revision): (&str, u64),
    captured_at: u64,
) {
    let at = format!("{scope}.{name}");
    if crate::variables_state::checked_name(name).is_err() {
        let refused = refusal(MEMBER_UNSUPPORTED, &at, "not a Lys variable name");
        fragment.refusals.push(refused);
        return;
    }
    let value = entry.get("value").unwrap_or(&Value::Null);
    let shaped = if secret_name(name) && !value.is_null() {
        Some(at.clone())
    } else {
        secret_shaped(&at, value)
    };
    if let Some(path) = shaped {
        let detail = "a secret-shaped variable; its value is not shown";
        fragment
            .refusals
            .push(refusal(CREDENTIAL_INLINE, &path, detail));
        return;
    }
    let expiry = entry.get("expires_at").and_then(Value::as_str);
    let expires_at = match expiry
        .map(crate::seat_import_schedules::instant)
        .transpose()
    {
        Ok(nanos) => nanos,
        Err(reason) => {
            let member = format!("{at}.expires_at");
            fragment
                .refusals
                .push(refusal(MEMBER_UNSUPPORTED, &member, reason));
            return;
        }
    };
    if expires_at.is_some_and(|nanos| reached(nanos, captured_at)) {
        fragment.excluded.push(Excluded {
            source_id: format!("{source_id}/{name}"),
            revision: revision.to_string(),
            reason: "expired at or before captured_at".to_owned(),
        });
        return;
    }
    let seconds = expires_at.map(|nanos| u64::try_from(nanos.div_euclid(NANOS)));
    let expires_at = match seconds.transpose() {
        Ok(seconds) => seconds,
        Err(error) => {
            let member = format!("{at}.expires_at");
            let detail = format!("not an instant Lys can hold: {error}");
            fragment
                .refusals
                .push(refusal(MEMBER_UNSUPPORTED, &member, detail));
            return;
        }
    };
    fragment.destinations.push(DestinationEntry {
        record_kind: "variable".to_owned(),
        record_id: format!("{}/{name}", lys.key()),
        expected_revision: None,
        change: json!({
            "scope": lys,
            "name": name,
            "value": value,
            "author": entry.get("author"),
            "expires_at": expires_at,
            "source_scope": scope,
            "seat": who.seat,
        }),
        source_entry_ids: vec![source_id.to_owned()],
    });
}
