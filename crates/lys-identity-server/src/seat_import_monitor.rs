//! The monitor's records of one seat, read for its import plan (AGENTS-003
//! R2) through the monitor's authenticated HTTP API and never through its
//! journals:
//! `GET /api/budgets`, `GET /api/prompt-settings`,
//! `POST /api/agent-variables/get` at the exact canonical agent and session
//! scopes, `GET /api/scheduled-messages` and `GET /api/rules`.
//!
//! Each answer's own error, invalid, skipped-line and held fields are
//! evidence: an answer that is unavailable, or says it is incomplete, is a
//! source entry marked incomplete beside a refusal by name
//! (`import_source_unavailable`, `import_source_incomplete`), never an
//! empty list. Zero rows with no such field is complete and says so.
//!
//! Budgets map into DIRECTORY-051's limits for the seat's agent (each
//! inform level a context notice, `compact_at` a context compaction, a
//! Codex session's notices information-only) and its context window;
//! prompt settings into AGENTS-001's words; variables into its variables at
//! the matching scope; schedules and rules through
//! [`crate::seat_import_schedules`] and [`crate::seat_import_rules`]. The
//! monitor's transport target is kept only as provenance, never a
//! destination. The collector secret rides in the header the manifest names
//! (`secret_header`) from the variable it names (`secret_env`); a secret
//! named with no header refuses by name, no header name is ever assumed, and
//! the secret's value is never written anywhere.

use std::collections::BTreeMap;
use std::env::VarError;
use std::time::Duration;

use reqwest::header::{HeaderName, HeaderValue};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use crate::seat_import_monitor_maps as maps;
use crate::seat_import_plan::{
    Completeness, DestinationEntry, Excluded, Fragment, MonitorSource, Refusal, SourceEntry,
};
use crate::variables_state::Scope;

/// A monitor answer that could not be had.
pub const SOURCE_UNAVAILABLE: &str = "import_source_unavailable";
/// A monitor answer that says it is incomplete.
pub const SOURCE_INCOMPLETE: &str = "import_source_incomplete";
/// A monitor scope that does not resolve to exactly one record.
pub const SCOPE_AMBIGUOUS: &str = "import_scope_ambiguous";
/// A member Lys cannot hold as it is.
pub const MEMBER_UNSUPPORTED: &str = "import_member_unsupported";
/// A secret-shaped member, refused without its value.
pub const CREDENTIAL_INLINE: &str = "import_credential_inline";
/// An input past a declared bound.
pub const BOUND_EXCEEDED: &str = "import_bound_exceeded";
/// The largest answer read from one monitor API, in bytes.
pub const ANSWER_BOUND: usize = 4 * 1024 * 1024;

const ASKED_FOR: Duration = Duration::from_secs(10);
const NANOS: i128 = 1_000_000_000;
pub(crate) const BUDGETS: &str = "/api/budgets";
pub(crate) const PROMPTS: &str = "/api/prompt-settings";
const VARIABLES: &str = "/api/agent-variables/get";
const SCHEDULES: &str = "/api/scheduled-messages";
const RULES: &str = "/api/rules";
const SECRET_WORDS: [&str; 8] = [
    "secret",
    "token",
    "password",
    "passwd",
    "apikey",
    "credential",
    "authorization",
    "bearer",
];
const SECRET_PREFIXES: [&str; 7] = [
    "bearer ",
    "-----begin",
    "sk-",
    "ghp_",
    "github_pat_",
    "xoxb-",
    "xoxp-",
];

/// Who the monitor's records are for: the monitor's agent key and session,
/// and the Lys seat and agent they land on.
pub(crate) struct Identity<'a> {
    pub(crate) agent: &'a str,
    pub(crate) session: Option<&'a str>,
    pub(crate) seat: &'a str,
    pub(crate) lys_agent: &'a str,
}

/// How the monitor is asked: one client, its base, and the secret with the
/// header it rides in.
struct Asking {
    client: reqwest::Client,
    base: String,
    secret: Option<(HeaderName, HeaderValue)>,
}

/// Read every monitor record of the seat `source` names, at `captured_at`,
/// for a Lys seat and agent both named by the source's agent key; the
/// source's own session is the one legacy recipient mapped to it. An import
/// whose Lys seat or agent is named otherwise reads through
/// [`read_monitor_mapped`].
pub async fn read_monitor(source: &MonitorSource, captured_at: u64) -> Fragment {
    let mut recipients = BTreeMap::new();
    if let Some(session) = &source.session {
        recipients.insert(session.clone(), source.agent.clone());
    }
    let seat = source.agent.as_str();
    read_monitor_mapped(source, captured_at, seat, seat, &recipients).await
}

/// Read every monitor record of `source` at `captured_at` for the Lys seat
/// `seat` of the Lys agent `lys_agent`, with the manifest's map of legacy
/// recipient sessions to Lys seats.
pub async fn read_monitor_mapped(
    source: &MonitorSource,
    captured_at: u64,
    seat: &str,
    lys_agent: &str,
    recipient_map: &BTreeMap<String, String>,
) -> Fragment {
    let mut fragment = empty();
    let asking = match asking(source) {
        Ok(asking) => asking,
        Err(refused) => {
            fragment.refusals.push(refused);
            return fragment;
        }
    };
    let base = asking.base.as_str();
    let who = Identity {
        agent: &source.agent,
        session: source.session.as_deref(),
        seat,
        lys_agent,
    };
    match ask(&asking, BUDGETS, None).await {
        Ok(body) => fragment.merge(maps::budgets(&body, &who, base)),
        Err(refused) => fragment.merge(refused_api(base, BUDGETS, None, refused)),
    }
    match ask(&asking, PROMPTS, None).await {
        Ok(body) => fragment.merge(maps::prompts(&body, &who, base)),
        Err(refused) => fragment.merge(refused_api(base, PROMPTS, None, refused)),
    }
    let mut asked_scopes = vec![format!("agent:{}", who.agent)];
    asked_scopes.extend(who.session.map(|session| format!("session:{session}")));
    for scope in asked_scopes {
        let asked = json!({ "scope": scope });
        match ask(&asking, VARIABLES, Some(&asked)).await {
            Ok(body) => fragment.merge(variables(&body, &scope, &who, captured_at, base)),
            Err(refused) => fragment.merge(refused_api(base, VARIABLES, None, refused)),
        }
    }
    match ask(&asking, SCHEDULES, None).await {
        Ok(body) => {
            let (rows, read) = listed(&body, "schedules", base, SCHEDULES);
            fragment.merge(read);
            if let Some(rows) = rows {
                let mut classified = crate::seat_import_schedules::classify_for(
                    rows,
                    captured_at,
                    recipient_map,
                    seat,
                    lys_agent,
                );
                for entry in &mut classified.sources {
                    entry.locator = format!("{base}{SCHEDULES}#{}", entry.scope);
                }
                fragment.merge(classified);
            }
        }
        Err(refused) => fragment.merge(refused_api(base, SCHEDULES, None, refused)),
    }
    match ask(&asking, RULES, None).await {
        Ok(body) => {
            let (rows, read) = listed(&body, "rules", base, RULES);
            fragment.merge(read);
            if let Some(rows) = rows {
                let mut transferred = crate::seat_import_rules::transfer_rules(rows, who.agent);
                for entry in &mut transferred.sources {
                    entry.locator = format!("{base}{RULES}#{}", entry.scope);
                }
                fragment.merge(transferred);
            }
        }
        Err(refused) => fragment.merge(refused_api(base, RULES, None, refused)),
    }
    fragment
}

/// A fragment holding nothing.
pub(crate) fn empty() -> Fragment {
    Fragment {
        sources: Vec::new(),
        destinations: Vec::new(),
        references: Vec::new(),
        excluded: Vec::new(),
        refusals: Vec::new(),
        schedule_counts: None,
        replacements: Vec::new(),
        prerequisites: Vec::new(),
    }
}

/// A refusal by `name` of `member`.
pub(crate) fn refusal(name: &str, member: &str, detail: impl Into<String>) -> Refusal {
    Refusal {
        name: name.to_owned(),
        member: member.to_owned(),
        detail: detail.into(),
    }
}

/// The SHA-256 of `value`'s JSON, its object keys in order.
pub(crate) fn content_revision(value: &Value) -> String {
    crate::routes::hex(&Sha256::digest(value.to_string().as_bytes()))
}

/// Whether `captured_at`, in seconds, is at or after the instant `nanos`.
pub(crate) fn reached(nanos: i128, captured_at: u64) -> bool {
    nanos <= i128::from(captured_at) * NANOS
}

/// Whether a member named `key` holds a secret by its name.
fn secret_name(key: &str) -> bool {
    let lowered = key.to_lowercase();
    lowered.contains("private_key")
        || lowered.contains("api_key")
        || lowered
            .split(['_', '-', '.'])
            .any(|word| SECRET_WORDS.contains(&word))
}

/// The path of the first secret-shaped member under `value`, named from
/// `path`; the value itself is never returned.
pub(crate) fn secret_shaped(path: &str, value: &Value) -> Option<String> {
    match value {
        Value::String(text) => {
            let lowered = text.trim_start().to_lowercase();
            let userinfo = lowered.split_once("://").is_some_and(|(_, rest)| {
                rest.split(['/', '?', '#'])
                    .next()
                    .is_some_and(|host| host.contains('@'))
            });
            let shaped = SECRET_PREFIXES
                .iter()
                .any(|prefix| lowered.starts_with(prefix))
                || lowered.contains("private key")
                || userinfo;
            shaped.then(|| path.to_owned())
        }
        Value::Array(items) => items
            .iter()
            .enumerate()
            .find_map(|(index, item)| secret_shaped(&format!("{path}[{index}]"), item)),
        Value::Object(members) => members.iter().find_map(|(key, item)| {
            let member = format!("{path}.{key}");
            if secret_name(key) && !item.is_null() {
                Some(member)
            } else {
                secret_shaped(&member, item)
            }
        }),
        Value::Null | Value::Bool(_) | Value::Number(_) => None,
    }
}

fn asking(source: &MonitorSource) -> Result<Asking, Refusal> {
    let base = source.base.trim_end_matches('/').to_owned();
    let secret = match &source.secret_env {
        None => None,
        Some(name) => {
            let member = format!("monitor.secret_env {name}");
            let Some(given) = &source.secret_header else {
                let detail = format!(
                    "the secret in {name} is named with no secret_header for it to ride in; no header name is assumed"
                );
                return Err(refusal(SOURCE_UNAVAILABLE, "monitor.secret_header", detail));
            };
            let header_name = HeaderName::from_bytes(given.as_bytes()).map_err(|error| {
                let detail = format!("`{given}` is not a header name: {error}");
                refusal(SOURCE_UNAVAILABLE, "monitor.secret_header", detail)
            })?;
            let value = match std::env::var(name) {
                Ok(value) => Zeroizing::new(value),
                Err(VarError::NotPresent) => {
                    let detail = format!("the variable {name} is not set");
                    return Err(refusal(SOURCE_UNAVAILABLE, &member, detail));
                }
                Err(VarError::NotUnicode(_)) => {
                    let detail = format!("the variable {name} is not text");
                    return Err(refusal(SOURCE_UNAVAILABLE, &member, detail));
                }
            };
            let mut header = HeaderValue::from_str(value.as_str()).map_err(|error| {
                let detail = format!("the variable {name} is not a header value: {error}");
                refusal(SOURCE_UNAVAILABLE, &member, detail)
            })?;
            header.set_sensitive(true);
            Some((header_name, header))
        }
    };
    let client = reqwest::Client::builder()
        .timeout(ASKED_FOR)
        .build()
        .map_err(|error| {
            let detail = format!("the client that asks the monitor could not be built: {error}");
            refusal(SOURCE_UNAVAILABLE, &base, detail)
        })?;
    Ok(Asking {
        client,
        base,
        secret,
    })
}

/// Ask the monitor `path`, posting `body` when there is one, and read its JSON
/// answer within [`ANSWER_BOUND`].
async fn ask(asking: &Asking, path: &str, body: Option<&Value>) -> Result<Value, Refusal> {
    let address = format!("{}{path}", asking.base);
    let method = if body.is_some() { "POST" } else { "GET" };
    let member = format!("{method} {address}");
    let mut request = match body {
        Some(body) => asking.client.post(&address).json(body),
        None => asking.client.get(&address),
    };
    if let Some((header, secret)) = &asking.secret {
        request = request.header(header.clone(), secret.clone());
    }
    let mut answered = request.send().await.map_err(|error| {
        let detail = format!("the monitor could not be reached: {error}");
        refusal(SOURCE_UNAVAILABLE, &member, detail)
    })?;
    let status = answered.status();
    if !status.is_success() {
        let detail = format!("the monitor answered {status}");
        return Err(refusal(SOURCE_UNAVAILABLE, &member, detail));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = answered.chunk().await.map_err(|error| {
        refusal(
            SOURCE_UNAVAILABLE,
            &member,
            format!("the answer could not be read: {error}"),
        )
    })? {
        if bytes.len() + chunk.len() > ANSWER_BOUND {
            let detail = format!("the answer is larger than {ANSWER_BOUND} bytes");
            return Err(refusal(BOUND_EXCEEDED, &member, detail));
        }
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes).map_err(|error| {
        let detail = format!("the answer does not read as JSON: {error}");
        refusal(SOURCE_UNAVAILABLE, &member, detail)
    })
}

/// The source entry for one monitor API, complete or not.
pub(crate) fn api_entry(
    base: &str,
    path: &str,
    revision: String,
    completeness: Completeness,
) -> SourceEntry {
    SourceEntry {
        id: format!("monitor:{path}"),
        kind: "monitor_api".to_owned(),
        locator: format!("{base}{path}"),
        scope: String::new(),
        revision_kind: "content_sha256".to_owned(),
        source_revision: revision,
        completeness,
    }
}

/// An API that refused or could not be read, `body` what it answered when
/// it answered: incomplete, with the refusal beside it.
fn refused_api(base: &str, path: &str, body: Option<&Value>, refused: Refusal) -> Fragment {
    let mut fragment = empty();
    let reason = format!("{}: {}", refused.name, refused.detail);
    let revision = body.map(content_revision).unwrap_or_default();
    let completeness = Completeness::Incomplete { reason };
    fragment
        .sources
        .push(api_entry(base, path, revision, completeness));
    fragment.refusals.push(refused);
    fragment
}

/// An API whose answer says it is incomplete.
pub(crate) fn incomplete(base: &str, path: &str, body: &Value, reason: &str) -> Fragment {
    let refused = refusal(SOURCE_INCOMPLETE, &format!("{base}{path}"), reason);
    refused_api(base, path, Some(body), refused)
}

/// The rows of a schedules or rules listing, none when its own fields say
/// rows were skipped, held or not read; the fragment says which.
fn listed<'a>(
    body: &'a Value,
    key: &str,
    base: &str,
    path: &str,
) -> (Option<&'a [Value]>, Fragment) {
    let Some(rows) = body.get(key).and_then(Value::as_array) else {
        let reason = format!("the answer names no {key} list");
        return (None, incomplete(base, path, body, &reason));
    };
    let health = body.get("health");
    let said = [
        (
            body.get("error").is_some_and(|error| !error.is_null()),
            "the answer carries an error",
        ),
        (
            body.get("skipped_lines")
                .is_some_and(|lines| lines.as_u64() != Some(0)),
            "the journal skipped lines it could not read",
        ),
        (
            nonempty(body.get("skips")),
            "the journal names skipped lines",
        ),
        (
            nonempty(health.and_then(|health| health.get("skips"))),
            "the journal names skipped lines",
        ),
        (
            nonempty(health.and_then(|health| health.get("held"))),
            "schedules are held for a lost journal page",
        ),
    ];
    if let Some((_, reason)) = said.iter().find(|(said, _)| *said) {
        return (None, incomplete(base, path, body, reason));
    }
    let revision = content_revision(&Value::Array(rows.clone()));
    let mut fragment = empty();
    fragment
        .sources
        .push(api_entry(base, path, revision, Completeness::Complete));
    (Some(rows.as_slice()), fragment)
}

fn nonempty(value: Option<&Value>) -> bool {
    value
        .and_then(Value::as_array)
        .is_some_and(|items| !items.is_empty())
}

/// One canonical scope's variables, each live one mapped at the matching
/// Lys scope with its author and absolute expiry.
fn variables(
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
        let at = format!("{scope}.{name}");
        if crate::variables_state::checked_name(name).is_err() {
            let refused = refusal(MEMBER_UNSUPPORTED, &at, "not a Lys variable name");
            fragment.refusals.push(refused);
            continue;
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
            continue;
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
                continue;
            }
        };
        if expires_at.is_some_and(|nanos| reached(nanos, captured_at)) {
            fragment.excluded.push(Excluded {
                source_id: format!("{source_id}/{name}"),
                revision: revision.to_string(),
                reason: "expired at or before captured_at".to_owned(),
            });
            continue;
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
                continue;
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
            source_entry_ids: vec![source_id.clone()],
        });
    }
    fragment
}
