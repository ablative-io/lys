#![cfg(test)]
//! The one `OpenAPI` document against the router it describes: it reads as
//! `OpenAPI` 3.1; every route the source declares has an entry and every
//! entry a route, and a planted route without an entry is named; every
//! entry is served by the running service; and every refusal the document
//! names is produced by some test. The routes are read from the source the
//! router is built from, a second party to the table in `src/openapi.rs`.

use std::collections::BTreeSet;
use std::error::Error;
use std::path::Path;

use identity_contract::apps::{Auth, TestResult, get, send};
use identity_contract::harness::Service;
use lys_identity_server::openapi::{api, document};
use serde_json::Value;

/// A route as `method path`.
type Declared = BTreeSet<(String, String)>;

/// Every route the source text `text` declares, by `.route(path, method(...))`.
fn declared_in(text: &str) -> Declared {
    let mut found = Declared::new();
    let mut rest = text;
    while let Some(start) = rest.find(".route(") {
        let call = &rest[start + ".route".len()..];
        let mut depth = 0usize;
        let mut end = call.len();
        for (index, byte) in call.bytes().enumerate() {
            match byte {
                b'(' => depth += 1,
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        end = index;
                        break;
                    }
                }
                _ => {}
            }
        }
        let body = &call[..end];
        if let Some(path) = body.split('"').nth(1) {
            for method in ["get", "post", "put", "delete", "patch"] {
                let mut from = 0;
                while let Some(at) = body[from..].find(&format!("{method}(")) {
                    let at = from + at;
                    let before = body[..at].chars().next_back();
                    if before.is_none_or(|char| !char.is_alphanumeric() && char != '_') {
                        found.insert((method.to_owned(), path.to_owned()));
                    }
                    from = at + method.len();
                }
            }
        }
        rest = &call[end..];
    }
    found
}

/// Every route the service's source declares, outside the screens' own router.
fn declared() -> Result<Declared, Box<dyn Error>> {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut found = Declared::new();
    let mut read = 0;
    for entry in std::fs::read_dir(&src)? {
        let path = entry?.path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        let rust = path.extension().is_some_and(|extension| extension == "rs");
        if !rust || name.starts_with("surface") {
            continue;
        }
        found.extend(declared_in(&std::fs::read_to_string(&path)?));
        read += 1;
    }
    if read == 0 {
        return Err(format!("no source was read from {}", src.display()).into());
    }
    Ok(found)
}

fn documented() -> Declared {
    api()
        .routes()
        .iter()
        .map(|route| (route.method.word().to_owned(), route.path.to_owned()))
        .collect()
}

/// Each route of `routes` that `table` does not hold, as `method path`.
fn missing(routes: &Declared, table: &Declared) -> Vec<String> {
    routes
        .difference(table)
        .map(|(method, path)| format!("{method} {path}"))
        .collect()
}

#[test]
fn the_document_reads_as_openapi_3_1() -> TestResult {
    let document = document()?;
    let faults = lys_openapi::validate(&document);
    assert!(faults.is_empty(), "{faults:#?}");
    assert_eq!(document["openapi"], "3.1.0");
    let schemes = document["components"]["securitySchemes"]
        .as_object()
        .ok_or("no schemes")?;
    for scheme in ["session_cookie", "lys_bearer", "agent_signature"] {
        assert!(schemes.contains_key(scheme), "{scheme} is not described");
    }
    let refusal = &document["components"]["schemas"]["Refusal"]["properties"];
    for member in ["refusal", "reason", "fields"] {
        assert!(
            refusal.get(member).is_some(),
            "every refusal carries {member}"
        );
    }
    let mut broken = document.clone();
    broken["paths"]["/apps"]["post"]["requestBody"]["content"]["application/json"]["schema"]["$ref"] =
        Value::from("#/components/schemas/Nothing");
    assert_eq!(
        lys_openapi::validate(&broken).len(),
        1,
        "a broken reference is a fault"
    );
    Ok(())
}

#[test]
fn every_declared_route_has_an_entry_and_every_entry_a_route() -> TestResult {
    let (routes, table) = (declared()?, documented());
    assert!(
        routes.len() > 90,
        "only {} routes were read from the source",
        routes.len()
    );
    let undocumented = missing(&routes, &table);
    assert!(
        undocumented.is_empty(),
        "routes without an entry: {undocumented:?}"
    );
    let unrouted = missing(&table, &routes);
    assert!(unrouted.is_empty(), "entries with no route: {unrouted:?}");
    assert_eq!(
        api().routes().len(),
        table.len(),
        "no route has two entries"
    );
    Ok(())
}

#[test]
fn a_route_added_without_an_entry_fails_naming_the_route() {
    let planted = declared_in(r#"Router::new().route("/planted/{id}", get(read).post(write))"#);
    let named = missing(&planted, &documented());
    assert_eq!(named, vec!["get /planted/{id}", "post /planted/{id}"]);
    let wrapped = declared_in(
        "        .route(\n            \"/grants/which\",\n            post(which),\n        )",
    );
    assert_eq!(wrapped.len(), 1, "a route across lines is read");
    assert!(missing(&wrapped, &documented()).is_empty());
}

#[tokio::test]
async fn every_entry_is_served_by_the_running_service() -> TestResult {
    let service = Service::start().await?;
    let mut asked = 0;
    for route in api().routes() {
        let path = route
            .path
            .split('/')
            .map(|segment| {
                if segment.starts_with('{') {
                    "x"
                } else {
                    segment
                }
            })
            .collect::<Vec<_>>()
            .join("/");
        let method = match route.method {
            lys_openapi::Method::Get => reqwest::Method::GET,
            lys_openapi::Method::Post => reqwest::Method::POST,
            lys_openapi::Method::Put => reqwest::Method::PUT,
        };
        let body = (method != reqwest::Method::GET).then(|| serde_json::json!({}));
        let (status, answer) = send(&service, method, &path, Auth::Nobody, body.as_ref()).await?;
        assert_ne!(
            status,
            405,
            "{} {} is not served for that method",
            route.method.word(),
            route.path
        );
        assert!(
            status != 404 || answer.get("refusal").is_some(),
            "{} {} is not served: {status} {answer}",
            route.method.word(),
            route.path
        );
        asked += 1;
    }
    assert_eq!(asked, api().routes().len());
    let served = get(&service, "/openapi.json", Auth::Nobody).await?;
    assert_eq!(served.0, 200);
    assert_eq!(
        served.1,
        document()?,
        "the served document is the generated one"
    );
    Ok(())
}

/// Every test source the refusals are produced in.
fn test_sources() -> Result<String, Box<dyn Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut text = String::new();
    for dir in [
        root.join("tests"),
        root.join("../../tests/identity_contract/tests"),
    ] {
        for entry in std::fs::read_dir(&dir)? {
            let path = entry?.path();
            let is_openapi = path.file_name().is_some_and(|name| name == "openapi.rs");
            if path.extension().is_some_and(|extension| extension == "rs") && !is_openapi {
                text.push_str(&std::fs::read_to_string(&path)?);
            }
        }
    }
    Ok(text)
}

#[test]
fn every_refusal_the_document_names_is_produced_by_a_test() -> TestResult {
    let document = document()?;
    let named: BTreeSet<String> = document["paths"]
        .as_object()
        .ok_or("no paths")?
        .values()
        .filter_map(Value::as_object)
        .flat_map(|item| item.values())
        .filter_map(|operation| operation["x-refusals"].as_array())
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();
    assert!(named.len() > 20, "only {} refusals are named", named.len());
    let sources = test_sources()?;
    let untested: Vec<&String> = named
        .iter()
        .filter(|name| !sources.contains(&format!("\"{name}\"")))
        .collect();
    assert!(
        untested.is_empty(),
        "refusals no test produces: {untested:?}"
    );
    Ok(())
}

/// Every route whose answer the document describes by its words alone, with
/// why. `every_route_names_the_types_it_takes_and_answers` holds this list
/// exact from both sides: a route named here that in fact names an answer
/// fails, and a route that names none and is not here fails.
const OPEN_ANSWERS: &[(&str, &str)] = &[
    ("get /authority", "answers text/plain, not JSON"),
    ("get /login", "answers a 303 to the issuer, with no body"),
    (
        "get /callback",
        "answers through Response, since it must set the session cookie",
    ),
    (
        "get /configuration",
        "answers the startup settings as a dump that follows the configuration",
    ),
    ("get /secrets", "the secrets broker's own answer, forwarded"),
    (
        "get /secrets/grants",
        "the secrets broker's own answer, forwarded",
    ),
    (
        "get /secrets/audit",
        "the secrets broker's own answer, forwarded",
    ),
    (
        "get /secrets/revocation",
        "the secrets broker's own answer, forwarded",
    ),
    (
        "get /secrets/settings",
        "the secrets broker's own answer, forwarded",
    ),
    (
        "get /secrets/handles",
        "the secrets broker's own answer, forwarded",
    ),
    (
        "post /secrets/scope",
        "the secrets broker's own answer, forwarded",
    ),
    (
        "post /secrets/recipients",
        "the secrets broker's own answer, forwarded",
    ),
    (
        "post /secrets/drop",
        "the secrets broker's own answer, forwarded",
    ),
    (
        "post /agents/{id}/start",
        "the start owners' own rendered JSON, which this crate holds no type for",
    ),
    (
        "post /launch-records/{id}/start-again",
        "the start owners' own rendered JSON, which this crate holds no type for",
    ),
    (
        "post /launch-records/{id}/withdraw",
        "the start owners' own rendered JSON, which this crate holds no type for",
    ),
    (
        "get /launch-records/{id}/state",
        "the start owners' own rendered JSON, which this crate holds no type for",
    ),
    ("get /openapi.json", "answers this document"),
    (
        "post /apps/{app}/placements",
        "answers the placement made, with nothing of its own to say",
    ),
    ("post /apps/bench", "answers the bench opened"),
    ("post /apps/bench/{id}/close", "answers the bench closed"),
];

/// Every POST or PUT route the document gives no request body schema, with
/// why. Each either takes no body at all or takes one it does not own.
const NO_BODY: &[(&str, &str)] = &[
    ("post /requests/{id}/reconcile", "takes no body"),
    ("post /network/machines/{id}/retire", "takes no body"),
    ("post /sessions/{id}/end", "takes no body"),
    (
        "post /directory/people/{id}/sessions/{session}/end",
        "takes no body",
    ),
    ("post /launch-records/{id}/start-again", "takes no body"),
    ("post /launch-records/{id}/withdraw", "takes no body"),
    ("post /apps/bench/{id}/close", "takes no body"),
    (
        "post /secrets/scope",
        "forwards its bytes to the secrets broker unread",
    ),
    (
        "post /secrets/recipients",
        "forwards its bytes to the secrets broker unread",
    ),
    (
        "post /secrets/drop",
        "forwards its bytes to the secrets broker unread",
    ),
    (
        "post /agents/{id}/start",
        "takes the start grammar's members as they came, not a Rust type",
    ),
];

/// The schema an operation names at `at`, when it names one by reference.
fn reference<'a>(operation: &'a Value, at: &str) -> Option<&'a str> {
    operation.pointer(at)?.get("$ref")?.as_str()
}

#[test]
fn every_route_names_the_types_it_takes_and_answers() -> TestResult {
    let document = document()?;
    let paths = document["paths"].as_object().ok_or("no paths")?;
    let open: BTreeSet<&str> = OPEN_ANSWERS.iter().map(|(route, _)| *route).collect();
    let bodiless: BTreeSet<&str> = NO_BODY.iter().map(|(route, _)| *route).collect();
    assert_eq!(open.len(), OPEN_ANSWERS.len(), "an answer is listed twice");
    assert_eq!(bodiless.len(), NO_BODY.len(), "a body is listed twice");
    let (mut answers, mut bodies) = (0, 0);
    let mut served = BTreeSet::new();
    for (path, item) in paths {
        for (method, operation) in item.as_object().into_iter().flatten() {
            let at = format!("{method} {path}");
            served.insert(at.clone());
            let answer = reference(operation, "/responses/200/content/application~1json/schema");
            if open.contains(at.as_str()) {
                assert!(
                    answer.is_none(),
                    "{at} names the answer {answer:?}: take it out of OPEN_ANSWERS"
                );
            } else {
                assert!(answer.is_some(), "{at} names no answer schema");
                answers += 1;
            }
            if method != "post" && method != "put" {
                continue;
            }
            let body = reference(operation, "/requestBody/content/application~1json/schema");
            if bodiless.contains(at.as_str()) {
                assert!(
                    body.is_none(),
                    "{at} names the body {body:?}: take it out of NO_BODY"
                );
            } else {
                assert!(body.is_some(), "{at} names no request body schema");
                bodies += 1;
            }
        }
    }
    for (route, _why) in OPEN_ANSWERS.iter().chain(NO_BODY) {
        assert!(
            served.contains(*route),
            "{route} is listed but is no route of the document"
        );
    }
    assert!(answers > 90, "only {answers} routes name an answer");
    assert!(bodies > 50, "only {bodies} routes name a request body");
    Ok(())
}
