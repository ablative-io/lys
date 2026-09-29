//! The `OpenAPI` 3.1 document of a lys service, generated and never written
//! by hand.
//!
//! A service states its route table once: each route's method, path, words,
//! how a caller authenticates to it, the types it takes and answers, and the
//! name of every refusal it answers with. The types' schemas are generated
//! from the types themselves by their `ToSchema` derives, so the document
//! says what the code takes and answers, and cannot say anything else.
//! Every refusal shares one shape, [`Refusal`]: its name, its words and the
//! fields at fault.
//!
//! [`validate`] reads a document back as `OpenAPI` 3.1 and checks what a
//! reader of it relies on: the version, that every reference resolves, that
//! every path parameter is declared, that every operation has an id of its
//! own and answers, and that every security scheme it names is defined.

use std::borrow::Cow;
use std::collections::BTreeSet;

use serde::Serialize;
use serde_json::{Map, Value, json};
use utoipa::ToSchema;

/// The `OpenAPI` version every document is written in, and the one
/// [`validate`] reads it as.
pub const VERSION: &str = "3.1.0";

/// An HTTP method a route answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Method {
    /// GET.
    Get,
    /// POST.
    Post,
    /// PUT.
    Put,
}

impl Method {
    /// The method's word in a document, lowercase.
    pub fn word(self) -> &'static str {
        match self {
            Self::Get => "get",
            Self::Post => "post",
            Self::Put => "put",
        }
    }
}

/// How a caller authenticates to a route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Auth {
    /// Anyone: the route answers without authentication.
    Public,
    /// A person's signed-in session, carried in its cookie.
    Session,
    /// An app's or registrar's Lys bearer credential.
    Bearer,
    /// An agent's signature over the request.
    AgentSignature,
}

impl Auth {
    /// The security scheme's name in a document; none for a public route.
    pub fn scheme(self) -> Option<&'static str> {
        match self {
            Self::Public => None,
            Self::Session => Some("session_cookie"),
            Self::Bearer => Some("lys_bearer"),
            Self::AgentSignature => Some("agent_signature"),
        }
    }
}

/// One field a refusal names as at fault.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RefusalField {
    /// The JSON pointer of the field within the request body.
    pub at: String,
    /// The count the fault concerns, when it concerns one.
    pub count: Option<u64>,
}

/// Every refusal's one shape.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Refusal {
    /// The refusal's name, which a caller codes against.
    pub refusal: String,
    /// The refusal in words.
    pub reason: String,
    /// The fields at fault; empty when the refusal is not about a field.
    pub fields: Vec<RefusalField>,
}

/// One route of the table.
#[derive(Debug, Clone)]
pub struct Route {
    /// Its method.
    pub method: Method,
    /// Its path, with each parameter as `{name}`.
    pub path: &'static str,
    /// What it does, in words.
    pub summary: &'static str,
    /// How a caller authenticates to it: any one of these.
    pub auth: &'static [Auth],
    /// The schema of the body it takes, by name; none when it takes none.
    pub request: Option<Cow<'static, str>>,
    /// The schema of the body it answers, by name; none when its answer is
    /// described only by its words.
    pub response: Option<Cow<'static, str>>,
    /// The name of every refusal it answers with.
    pub refusals: Vec<&'static str>,
}

/// A service's route table and the schemas its routes name.
#[derive(Debug, Clone)]
pub struct Api {
    title: String,
    version: String,
    description: String,
    routes: Vec<Route>,
    schemas: Map<String, Value>,
    faults: Vec<String>,
}

fn reference(name: &str) -> Value {
    json!({"$ref": format!("#/components/schemas/{name}")})
}

/// Each parameter a path template names, in order.
pub fn parameters(path: &str) -> Vec<&str> {
    path.split('/')
        .filter_map(|segment| segment.strip_prefix('{')?.strip_suffix('}'))
        .collect()
}

impl Api {
    /// An empty table titled `title`, at `version`, described by `description`.
    pub fn new(title: &str, version: &str, description: &str) -> Self {
        let mut api = Self {
            title: title.to_owned(),
            version: version.to_owned(),
            description: description.to_owned(),
            routes: Vec::new(),
            schemas: Map::new(),
            faults: Vec::new(),
        };
        api.schema::<Refusal>();
        api
    }

    /// Hold the schema of `T` and of every type it names, answering the
    /// name routes refer to it by.
    pub fn schema<T: ToSchema>(&mut self) -> Cow<'static, str> {
        let mut named = Vec::new();
        T::schemas(&mut named);
        named.push((T::name().into_owned(), T::schema()));
        for (name, schema) in named {
            match serde_json::to_value(&schema) {
                Ok(value) => self.hold(name, value),
                Err(error) => self
                    .faults
                    .push(format!("the schema {name} does not serialise: {error}")),
            }
        }
        Cow::Owned(T::name().into_owned())
    }

    /// Hold `value` under `name`, faulting when a different schema is
    /// already held under it. Two types of one name would otherwise leave
    /// whichever was registered last standing for both, and every route
    /// naming either would point at the survivor without saying so.
    fn hold(&mut self, name: String, value: Value) {
        match self.schemas.get(&name) {
            Some(held) if held == &value => {}
            Some(_) => self.faults.push(format!(
                "two different schemas are named {name}: rename one with #[schema(as = ...)]"
            )),
            None => {
                self.schemas.insert(name, value);
            }
        }
    }

    /// Add `route` to the table.
    pub fn route(&mut self, route: Route) {
        self.routes.push(route);
    }

    /// Every route of the table, in the order added.
    pub fn routes(&self) -> &[Route] {
        &self.routes
    }

    fn operation(route: &Route) -> Value {
        let security: Vec<Value> = route
            .auth
            .iter()
            .filter_map(|auth| auth.scheme())
            .map(|scheme| json!({ scheme: [] }))
            .collect();
        let parameters: Vec<Value> = parameters(route.path)
            .into_iter()
            .map(|name| json!({"name": name, "in": "path", "required": true, "schema": {"type": "string"}}))
            .collect();
        let answered = route
            .response
            .as_deref()
            .map_or_else(|| json!({"type": "object"}), reference);
        let refused = if route.refusals.is_empty() {
            "A refusal, by name.".to_owned()
        } else {
            format!("A refusal, by name: {}.", route.refusals.join(", "))
        };
        let mut operation = json!({
            "operationId": format!("{}{}", route.method.word(), route.path.replace(['/', '{', '}'], "_")),
            "summary": route.summary,
            "security": security,
            "parameters": parameters,
            "responses": {
                "200": {"description": route.summary, "content": {"application/json": {"schema": answered}}},
                "default": {"description": refused, "content": {"application/json": {"schema": reference("Refusal")}}},
            },
            "x-refusals": route.refusals,
        });
        if let Some(request) = route.request.as_deref() {
            operation["requestBody"] = json!({
                "required": true,
                "content": {"application/json": {"schema": reference(request)}},
            });
        }
        operation
    }

    /// The document, or every fault met while generating it.
    pub fn document(&self) -> Result<Value, Vec<String>> {
        if !self.faults.is_empty() {
            return Err(self.faults.clone());
        }
        let mut paths = Map::new();
        for route in &self.routes {
            let item = paths
                .entry(route.path.to_owned())
                .or_insert_with(|| json!({}));
            item[route.method.word()] = Self::operation(route);
        }
        Ok(json!({
            "openapi": VERSION,
            "info": {"title": self.title, "version": self.version, "description": self.description},
            "paths": paths,
            "components": {
                "schemas": self.schemas,
                "securitySchemes": {
                    "session_cookie": {"type": "apiKey", "in": "cookie", "name": "lys_directory_session", "description": "A person's signed-in session."},
                    "lys_bearer": {"type": "http", "scheme": "bearer", "description": "An app's credential, `lys-app.{app}.{secret}`, or a registrar's, `lys-registrar.{account}.{secret}`."},
                    "agent_signature": {"type": "apiKey", "in": "header", "name": "lys-agent-signature", "description": "An agent's signature over the request's method, path, body digest, time and nonce."},
                },
            },
        }))
    }
}

/// Every `$ref` the value holds, anywhere within it.
fn references<'a>(value: &'a Value, found: &mut Vec<&'a str>) {
    match value {
        Value::Object(members) => {
            for (key, member) in members {
                match (key.as_str(), member.as_str()) {
                    ("$ref", Some(target)) => found.push(target),
                    _ => references(member, found),
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                references(item, found);
            }
        }
        _ => {}
    }
}

/// Every fault `document` holds as `OpenAPI` 3.1; none when it holds none.
pub fn validate(document: &Value) -> Vec<String> {
    let mut faults = Vec::new();
    if let Err(error) = serde_json::from_value::<utoipa::openapi::OpenApi>(document.clone()) {
        faults.push(format!(
            "the document does not read as OpenAPI 3.1: {error}"
        ));
    }
    if document.get("openapi").and_then(Value::as_str) != Some(VERSION) {
        faults.push(format!("the document's openapi is not {VERSION}"));
    }
    let schemas = document
        .pointer("/components/schemas")
        .and_then(Value::as_object);
    let mut found = Vec::new();
    references(document, &mut found);
    for target in found {
        let resolves = target
            .strip_prefix("#/components/schemas/")
            .is_some_and(|name| schemas.is_some_and(|schemas| schemas.contains_key(name)));
        if !resolves {
            faults.push(format!("the reference {target} resolves to nothing"));
        }
    }
    let defined = document
        .pointer("/components/securitySchemes")
        .and_then(Value::as_object);
    let mut ids = BTreeSet::new();
    let paths = document.get("paths").and_then(Value::as_object);
    for (path, item) in paths.into_iter().flatten() {
        for (method, operation) in item.as_object().into_iter().flatten() {
            let at = format!("{method} {path}");
            let id = operation.get("operationId").and_then(Value::as_str);
            if !id.is_some_and(|id| ids.insert(id.to_owned())) {
                faults.push(format!("{at} has no operation id of its own"));
            }
            if operation.pointer("/responses/200").is_none() {
                faults.push(format!("{at} has no answer"));
            }
            let declared: BTreeSet<&str> = operation
                .get("parameters")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|parameter| parameter.get("name").and_then(Value::as_str))
                .collect();
            for name in parameters(path) {
                if !declared.contains(name) {
                    faults.push(format!("{at} does not declare its parameter {name}"));
                }
            }
            let named = operation
                .get("security")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_object)
                .flat_map(Map::keys);
            for scheme in named {
                if !defined.is_some_and(|defined| defined.contains_key(scheme)) {
                    faults.push(format!(
                        "{at} names the security scheme {scheme}, which is not defined"
                    ));
                }
            }
        }
    }
    faults
}
