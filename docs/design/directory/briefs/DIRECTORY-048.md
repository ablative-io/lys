---
type: brief
id: DIRECTORY-048
cluster: directory
title: Every app registers its sign-in and permission schema with Lys through one published API; Lys depends on no app
---

# DIRECTORY-048: Every app registers its sign-in and permission schema with Lys through one published API; Lys depends on no app

> **Cluster:** directory
> **Depends on:** DIRECTORY-047
> **Design anchor:**
> - ADR-116 — Every app registers with Lys through one published API; Lys depends on no app — An app is a record in Lys: an id, a name, its sign-in client, and a permission schema it owns (resource kinds under the app's own prefix, each kind's actions, relations carrying actions, and parent kinds whose relations flow down). An app registers and changes its schema only through the API, is approved by an administrator on a Lys screen before it has any effect, and cannot touch another app's kinds. Lys's own model is the schema of the app 'lys'. The API is described by one OpenAPI document generated from the routes and their types, never written by hand. The MCP server is a face over that same API with three tools, the caller's own identity on every call, and no credential or authority of its own. Lys depends on no app. It holds no app's name, kind, schema or code; it never calls an app, waits on one or reads one's store. Every app depends on Lys through this API alone, and Lys runs the same with no apps registered as with twenty.
> - ADR-126 — Applications have connector identities; people register them with explicit permission — An application connector is its own third identity and grant-holder kind. Apps do not act as agents and do not use service accounts, which are accounts people use to service things. Only a signed-in person with an explicit ordinary register_app grant registers an app. The super administrator may grant that permission to a person; an agent, connector or service account cannot register, even if presented with such a grant. Approval creates a connector identity and binds the app to it. No app authority is implicit in approval, binding or ownership of a kind prefix: every permission is an explicit grant with a chain tracing to the super administrator. Own-kind checks, schema acts, batch and which use that connector identity and the ordinary grant engine. Extend IdentityId, grant admission and the permission-store holder, plus event and snapshot representation with compatibility tests. Preserve historical person/agent bytes and signatures; version a representation when necessary, never rewrite history or coerce identities. This supersedes DIRECTORY-048 wording allowing a service account or connector holding register_app to register, and forbids a separate registrar credential as substitute authority.
> **Checklist:**
> - C361 — Only a person holding an explicit register_app grant registers an app; approval creates its distinct application-connector identity and binding, and every app permission follows an explicit grant chain back to the super administrator (ADR-126, DIRECTORY-048 R1).
> - C362 — An app's schema declares kinds under its own prefix, their actions, relations carrying actions and parent kinds; an invalid schema is refused naming the line of the fault (DIRECTORY-048 R2).
> - C363 — An app cannot define, change or grant on another app's kinds; the refusal names the owning prefix (DIRECTORY-048 R2).
> - C364 — A schema change that would strand standing grants is refused naming the relation and the count; a dry run answers the same without writing (DIRECTORY-048 R3).
> - C365 — Lys's own model is the schema of the app 'lys'; existing grants and checks answer the same after the move (DIRECTORY-048 R4).
> - C366 — Apps check many permissions and list permitted resources as their own application connector, under explicit grants and their own-kind boundary; neither binding nor prefix ownership grants authority (ADR-126, DIRECTORY-048 R5).
> - C367 — One OpenAPI document, generated from the routes and types, describes every route; a route without an entry fails the build (DIRECTORY-048 R6).
> - C368 — Lys holds no app's name or schema in code or configuration and makes no call to any app (DIRECTORY-048 R7).
> - C386 — A person builds an app's whole permission template on the Apps screen from templates, tests it on example people and resources against the real check, and saves it; built and uploaded schemas are the same record (DIRECTORY-048 R8).
> **Stories:**
> - S151 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As a developer of an app, I want to register my app and its permission schema through a documented API, so that my app's resources and actions are checked by Lys without anyone changing Lys.
> - S152 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As an administrator, I want to approve each app and see its schema on a Lys screen before it takes effect, so that no app gives itself power I have not seen.

## Purpose

Tom, 28 September 2026 evening: 'a quality API as well, because we're going to need a way of getting in the permission schema and everything for every one of the apps that it covers', and 'this has to be able to handle sign in and authentication and permissions for every single one of our apps, but not depend on them.' Lys must be the sign-in, authentication and permissions for every product we make, and for products we do not make, without any of them being built into it. Today the permission model is one file read at start, any grant may name any kind, apps are not records, and there is no description of the API an app would code against. Tom, 19:53: 'I want a nice builder for designing the permissions. Both: for you guys to upload them, but the permission schema, however that's done, needs to be done properly.' Tom, 19:55, correcting: 'we need to be able to build the permissions template, which is the harder part. Saying it in words after the fact doesn't change anything.' Amended 28 September 2026 20:45 by Waffles after Archie's review of the brief against the tree.

## Task

Make an app a record in Lys with its sign-in client and a permission schema it owns, registered and changed only through the API and approved by an administrator on a Lys screen; move Lys's own model into the schema of the app 'lys'; give apps batch checks and resource lookups; publish one generated OpenAPI document. Lys depends on no app. It holds no app's name, kind, schema or code; it never calls an app, waits on one or reads one's store. Every app depends on Lys through this API alone, and Lys runs the same with no apps registered as with twenty.

## Requirements

### R1: An app is a record, registered through the API and approved on a Lys screen

Behavioural. Add apps as log events and a projection (apps_api.rs, apps_state.rs, apps_store.rs, following the *_api/_state/_store shape of teams and service accounts). POST /apps takes an id (3 to 40 lowercase letters, digits and underscores, starting with a letter, not 'lys'), a display name, redirect addresses and a permission schema (R2); it is accepted only from a signed-in person holding an explicit ordinary register_app grant whose authority traces to the super administrator, and creates the app as pending with its sign-in client prepared but not enabled. An administrator approves or declines it on a new Apps screen (surface/identity/src/features/apps) that shows the name, the redirect addresses and the full schema in words before the button; only on approval is the sign-in client enabled at Lys's provider (DIRECTORY-047 R3) and the schema written to the permission store. GET /apps and GET /apps/{app} answer what the caller may see. Retiring an app disables its client and refuses every check on its kinds by name; its grants stay in the log. An app id is 3 to 40 characters of lowercase letters, digits and underscores, starting with a letter, the same alphabet the permission store's names allow, so the id is the store prefix unchanged; a hyphen or dot is refused app_id_invalid. Registering writes only the pending registration: the sign-in client is created at the provider on approval and never before. On approval the app's client secret is shown once, to the approving administrator, on the approval screen and in the approval's API answer, and is never written to a log, a receipt or a stored record in the clear. ADR-126 records Tom's 29 September 2026 06:47, 06:48 and 06:50 Melbourne rulings. An application connector is a distinct third identity/holder kind beside person and agent, never a service account or an agent identity. On approval create that connector identity and bind the app to it; registration, identity creation and prefix ownership confer no implicit permissions. No agent, connector or service account can register an app, even if offered a register_app grant or registrar bearer. Every authority an app exercises is an explicit ordinary grant whose derivation reaches the super administrator; authenticate the connector, then judge its grant on each protected act. Add the connector holder kind to IdentityId, grant types/admission and the permission store, and carry it through events, snapshot encoding/restore and server identity dispatch. Preserve existing person/agent encoded bytes, signatures, IDs and history, introduce a versioned representation wherever adding a member otherwise changes a frozen format, and refuse unsupported formats by name. Do not rewrite historical records or silently map connector identities to people, agents or service accounts. Against DIRECTORY-047 R3's complete green provider tree, wire production client lookup to apps_binding::sign_in_client; an app bearer GET /apps/me does not prove an app-client OIDC sign-in. Remove the registrar-credential substitute. Under ADR-111, grants.rs is already near the 500-code-line limit. Move connector-specific holder conversion and authorization helpers into the named module crates/lys-identity-server/src/grants_connector.rs, declared from lib.rs, rather than growing grants.rs. The repository file-length leg is the measure, not raw physical lines.

**Acceptance:**
- A pending app's client cannot complete a sign-in and checks on its kinds answer app_not_approved.
- Approval on the screen enables the client and the schema; a sign-in and a check then pass.
- A second registration of the same id is refused app_exists.
- A retired app's checks answer app_retired and its grants remain readable.
- An id with a hyphen, a dot or under three characters is refused app_id_invalid.
- Registering creates nothing at the provider; approving creates the client and shows its secret once to the approver; no log line holds it.
- The production provider client lookup reaches apps_binding::sign_in_client: a pending app is refused, approval permits an OIDC round trip using that app's own client and registered redirect, and retirement refuses another round trip; GET /apps/me does not satisfy this assertion.
- Registration writes no app binding; approval records the app binding. The secret is answered only to the approving administrator and no persisted record, receipt, or captured log contains its clear value.
- A signed-in person with an explicit register_app grant can register an app. A person without that grant, or whose required grant or ancestor was revoked or expired, receives a named refusal and creates no app, provider client or binding. An agent, application connector or service account is refused even if presented with register_app or a registrar bearer. No administrator-role bypass substitutes for the ordinary grant check.
- Approval records one stable application-connector identity and its app binding, idempotently across retry and reopen; the binding names no person, agent or service-account identity as the app's authority.
- The newly approved connector has no permissions implicitly, including on its own prefix. An explicit root/derived grant chain from the super administrator permits the requested action; revoking or expiring any required ancestor, or retiring the connector's app, refuses the same action by name.
- Compatibility fixtures written before the connector kind still decode, verify signatures and replay with the same person/agent identities, grants and outcomes; connector event/snapshot fixtures round-trip and reopen with distinct identity tags, and unknown tags/formats are refused by name. Existing fixture bytes are not regenerated to satisfy this test.
- sh scripts/file-length.sh exits 0 after the connector work, including grants.rs and grants_connector.rs.
- The grant refusal tests in crates/lys-identity-server/tests/grants.rs pass unchanged at the card's head.

**Files:**
- create: crates/lys-identity-server/src/apps_api.rs
- create: crates/lys-identity-server/src/apps_state.rs
- create: crates/lys-identity-server/src/apps_store.rs
- create: crates/lys-identity-server/tests/apps.rs
- create: surface/identity/src/features/apps/Apps.tsx
- create: surface/identity/src/features/apps/apps.css
- create: crates/lys-identity-server/src/apps_binding.rs
- create: crates/lys-identity-server/src/apps_views.rs
- create: crates/lys-identity-server/src/apps_error.rs
- create: crates/lys-identity-server/src/apps_binding_tests.rs
- create: crates/lys-identity/tests/connector_compatibility.rs
- create: crates/lys-identity-server/tests/app_connector_grants.rs
- create: crates/lys-identity-server/src/grants_connector.rs
- modify: crates/lys-identity-server/src/routes.rs
- modify: crates/lys-identity-server/src/lib.rs
- modify: crates/lys-identity-server/src/provider.rs
- modify: crates/lys-identity-server/src/grants.rs
- modify: tests/identity_contract/src/harness.rs
- modify: crates/lys-identity-server/src/admission.rs
- modify: crates/lys-identity-server/src/agent_sight.rs
- modify: crates/lys-identity-server/src/agent_signature.rs
- modify: crates/lys-identity-server/src/certificates_issue.rs
- modify: crates/lys-identity-server/src/dev_seed.rs
- modify: crates/lys-identity-server/src/grant_sight.rs
- modify: crates/lys-identity-server/src/launch_api.rs
- modify: crates/lys-identity-server/src/network_api.rs
- modify: crates/lys-identity-server/src/provisioning_api.rs
- modify: crates/lys-identity-server/src/read_api.rs
- modify: crates/lys-identity-server/src/requests_api.rs
- modify: crates/lys-identity-server/src/requests_decide.rs
- modify: crates/lys-identity-server/src/reviews_api.rs
- modify: crates/lys-identity-server/src/runtime_api.rs
- modify: crates/lys-identity-server/src/start.rs
- modify: crates/lys-identity-server/src/stop_api.rs
- modify: crates/lys-identity-server/src/teams_api.rs
- modify: crates/lys-identity/src/directory.rs
- modify: crates/lys-identity/src/encoding.rs
- modify: crates/lys-identity/src/event.rs
- modify: crates/lys-identity/src/grants/admission.rs
- modify: crates/lys-identity/src/grants/authority.rs
- modify: crates/lys-identity/src/grants/codec.rs
- modify: crates/lys-identity/src/grants/lineage.rs
- modify: crates/lys-identity/src/grants/permission.rs
- modify: crates/lys-identity/src/grants/revocation.rs
- modify: crates/lys-identity/src/grants/types.rs
- modify: crates/lys-identity/src/link_audit.rs
- modify: crates/lys-identity/src/projection.rs
- modify: crates/lys-identity/src/projection_state.rs
- modify: crates/lys-identity/src/state_value.rs
- modify: crates/lys-identity/src/id.rs
- modify: crates/lys-identity/src/lib.rs
- modify: crates/lys-identity/src/directory_state.rs
- modify: crates/lys-identity/src/encoding_tests.rs
- modify: crates/lys-identity/src/grants/events.rs
- modify: crates/lys-identity/src/grants/book_state.rs
- modify: crates/lys-identity/src/grants/state.rs
- modify: crates/lys-identity/src/grants/projection.rs
- modify: crates/lys-identity-server/src/grant_contract/requests.rs
- modify: crates/lys-identity-server/src/grant_contract/views.rs

**Checklist:**
- C361 — Only a person holding an explicit register_app grant registers an app; approval creates its distinct application-connector identity and binding, and every app permission follows an explicit grant chain back to the super administrator (ADR-126, DIRECTORY-048 R1).

**Stories:**
- S151 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As a developer of an app, I want to register my app and its permission schema through a documented API, so that my app's resources and actions are checked by Lys without anyone changing Lys.
- S152 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As an administrator, I want to approve each app and see its schema on a Lys screen before it takes effect, so that no app gives itself power I have not seen.

### R2: An app's schema: its own kinds, actions, relations and parents, checked on entry

Behavioural. A schema is JSON: kinds, each with actions, relations (each carrying a set of the kind's actions) and optional parents (a relation on a parent kind flows to the child, as a workspace's member reaches its channels). Every kind is written '{app}.{kind}'; a kind outside the app's prefix, an action not declared on its kind, a relation carrying an unknown action, a parent cycle or a parent in another app is refused schema_invalid naming the JSON pointer of the fault. Grant creation, delegation and every check refuse kind_not_registered for a kind no approved app declares, and not_your_app when a caller acting for one app names another's kind. The schema is translated to the permission store's definitions under the app's prefix only; no write touches another prefix. The app 'lys' is the one exception to the app.kind form: Lys's own kinds keep their present names, and every other app's kinds carry its prefix. An application connector is bound to its app by an app binding record (connector identity id, app id, bound by, when), written on approval and read by not_your_app. The binding constrains which app the caller represents; it grants no authority. Own-kind access must also pass an explicit ordinary grant check rooted in the super administrator.

**Acceptance:**
- Each fault above is refused with its pointer, with one test per fault.
- A grant naming an unregistered kind is refused kind_not_registered.
- An app's authenticated application connector naming another app's kind is refused not_your_app.
- A parent relation grants the child: a workspace member may read a channel in it.
- An authenticated connector with no grant is refused even on its own kinds; an explicit grant with a standing chain to the super administrator permits the act, and revocation refuses it; another app's kinds remain not_your_app regardless of that grant.

**Files:**
- create: crates/lys-identity/src/grants/schema.rs
- create: crates/lys-identity/src/grants/schema_tests.rs
- create: crates/lys-identity-server/src/apps_binding.rs
- modify: crates/lys-identity/src/grants/model.rs
- modify: crates/lys-identity/src/grants/permission.rs
- modify: crates/lys-identity-server/src/spicedb.rs
- modify: crates/lys-identity-server/src/grants.rs

**Checklist:**
- C362 — An app's schema declares kinds under its own prefix, their actions, relations carrying actions and parent kinds; an invalid schema is refused naming the line of the fault (DIRECTORY-048 R2).
- C363 — An app cannot define, change or grant on another app's kinds; the refusal names the owning prefix (DIRECTORY-048 R2).

**Stories:**
- S151 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As a developer of an app, I want to register my app and its permission schema through a documented API, so that my app's resources and actions are checked by Lys without anyone changing Lys.

### R3: Schema changes are versioned, dry-run first, and never strand a grant

Behavioural. PUT /apps/{app}/schema carries the version it replaces and is refused schema_version_moved when that is not current. POST /apps/{app}/schema/check answers what the change would do without writing: kinds, relations and actions added and removed, and the standing grants each removal would strand. A change that removes a kind or relation with standing grants, or an action a standing grant carries, is refused schema_change_strands_grants naming each relation and its count; it is applied only after those grants are revoked through the ordinary revoke route. A change made by the app's application connector waits for administrator approval on the Apps screen, as registration does. Every version stays readable at GET /apps/{app}/schema?version=N. The apps store is born with its checkpoint beside its log, so a start reads the checkpoint and the tail only, never the whole history.

**Acceptance:**
- A stale version is refused schema_version_moved.
- The check route and the refused write name the same relations and counts.
- After revoking the stranded grants the same change applies and the version moves by one.
- Version N stays readable after N+1.

**Files:**
- create: crates/lys-identity-server/tests/apps_schema.rs
- modify: crates/lys-identity-server/src/apps_api.rs
- modify: crates/lys-identity-server/src/apps_state.rs

**Checklist:**
- C364 — A schema change that would strand standing grants is refused naming the relation and the count; a dry run answers the same without writing (DIRECTORY-048 R3).

**Stories:**
- S151 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As a developer of an app, I want to register my app and its permission schema through a documented API, so that my app's resources and actions are checked by Lys without anyone changing Lys.

### R4: Lys's own model is the schema of the app 'lys'

Behavioural. At start, when no 'lys' app exists in the log, the model in grant_model_file is recorded once as the schema of the app 'lys', approved, at the model's version; afterwards the log is the only source and the file is not read again. Lys's own kinds keep their present names, recorded as the 'lys' prefix's kinds, so every existing grant and check answers the same. The 'lys' app cannot be retired and its schema changes only by an administrator. This is the exception R2 names for the app 'lys'.

**Acceptance:**
- Every existing grant and check test passes unchanged after the move.
- A second start with a changed model file changes nothing and says so by name in the start log.
- Retiring 'lys' is refused app_is_lys.

**Files:**
- modify: crates/lys-identity-server/src/config.rs
- modify: crates/lys-identity-server/src/start.rs
- modify: crates/lys-identity-server/src/grants.rs

**Checklist:**
- C365 — Lys's own model is the schema of the app 'lys'; existing grants and checks answer the same after the move (DIRECTORY-048 R4).

**Stories:**
- S151 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As a developer of an app, I want to register my app and its permission schema through a documented API, so that my app's resources and actions are checked by Lys without anyone changing Lys.

### R5: Apps check many permissions at once and list what a subject may act on

Behavioural. POST /grants/check/batch takes up to 500 checks (subject, kind, id, action) and answers each allowed or refused with its reason, in order, at one permission store revision named in the answer. POST /grants/which takes a subject, a kind and an action and answers the ids that subject may act on, paged by cursor. Both require a signed-in administrator or the app's authenticated application connector with an explicit ordinary grant for the act, limited to that app's own kinds. The connector identity, never a service-account or agent identity, is the caller judged by the grant engine; owning the prefix confers no permission. A caller may pass the revision it last wrote (at_least) to read its own write.

**Acceptance:**
- A batch of mixed allowed and refused checks answers each in order at one named revision.
- 501 checks are refused batch_too_large.
- which lists exactly the ids the batch would allow, across pages.
- A check with at_least of a just-made grant sees it.
- An authenticated connector with no grant is refused even on its own kinds; an explicit grant with a standing chain to the super administrator permits the act, and revocation refuses it; another app's kinds remain not_your_app regardless of that grant.

**Files:**
- create: crates/lys-identity-server/tests/grants_batch.rs
- modify: crates/lys-identity-server/src/grants.rs
- modify: crates/lys-identity-server/src/spicedb.rs

**Checklist:**
- C366 — Apps check many permissions and list permitted resources as their own application connector, under explicit grants and their own-kind boundary; neither binding nor prefix ownership grants authority (ADR-126, DIRECTORY-048 R5).

**Stories:**
- S151 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As a developer of an app, I want to register my app and its permission schema through a documented API, so that my app's resources and actions are checked by Lys without anyone changing Lys.

### R6: One OpenAPI document, generated, describes every route

Behavioural. GET /openapi.json answers an OpenAPI 3.1 document generated from the route table and the request, response and refusal types, with each route's authentication (session cookie, Lys bearer token for apps, agent signature) and each refusal name. It is never written by hand. A test walks every route the router serves and fails naming any route without an entry, and any entry with no route. Refusals share one shape: name, words, and the fields at fault. The document is generated by a new crate, crates/lys-openapi (utoipa), declared in the workspace; before the card round, the crate's dependencies are fetched into the build machine's cache so the round's offline check resolves.

**Acceptance:**
- The document validates as OpenAPI 3.1.
- Adding a route without a description fails the test naming the route.
- Every refusal in the document has a test that produces it.

**Files:**
- create: crates/lys-identity-server/src/openapi.rs
- create: crates/lys-identity-server/tests/openapi.rs
- create: crates/lys-openapi/Cargo.toml
- create: crates/lys-openapi/src/lib.rs
- modify: crates/lys-identity-server/src/routes.rs
- modify: crates/lys-identity-server/src/error_status.rs
- modify: crates/lys-identity-server/Cargo.toml

**Checklist:**
- C367 — One OpenAPI document, generated from the routes and types, describes every route; a route without an entry fails the build (DIRECTORY-048 R6).

**Stories:**
- S151 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As a developer of an app, I want to register my app and its permission schema through a documented API, so that my app's resources and actions are checked by Lys without anyone changing Lys.

### R7: Lys depends on no app

Structural. Lys depends on no app. It holds no app's name, kind, schema or code; it never calls an app, waits on one or reads one's store. Every app depends on Lys through this API alone, and Lys runs the same with no apps registered as with twenty. An ast-grep rule in sgconfig.yml refuses any product name (cambium, argus, aion, manifold, meridian, haematite) in crates/ and surface/ outside fixtures; no outbound call is made to any registered redirect or app address. Lys's gate runs with no app registered and passes. The rule lives in rules/ast-grep/, the folder sgconfig.yml runs. Thirteen files outside tests name a product today (the eight listed here and five under deploy/identity); each is changed so no product name remains, or the name moves into a registration an app makes through R1. The rule is seen red on today's tree before the change and green after.

**Acceptance:**
- ast-grep scan exits 0 on the tree and fails on a planted product name.
- The full gate passes with no app registered.
- A test registers two apps with the same kind name under their own prefixes and checks each separately.

**Files:**
- create: rules/ast-grep/no-app-names.yml
- modify: crates/lys-core/src/attestation/mod.rs
- modify: crates/lys-identity-server/src/door_handles.rs
- modify: crates/lys/src/identity/config.rs
- modify: crates/lys/src/identity/themes.rs
- modify: crates/lys/src/identity/configure.rs
- modify: crates/lys/src/identity/cli.rs
- modify: crates/lys/src/identity/install/deployment.template.toml
- modify: surface/styles/tokens.css

**Checklist:**
- C368 — Lys holds no app's name or schema in code or configuration and makes no call to any app (DIRECTORY-048 R7).

**Stories:**
- S151 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As a developer of an app, I want to register my app and its permission schema through a documented API, so that my app's resources and actions are checked by Lys without anyone changing Lys.

### R8: Build an app's permission template on a Lys screen

Behavioural. Building the template is the requirement; a description of it is not. The Apps screen gains a builder in which a person makes an app's whole permission schema without writing any schema language: add kinds under the app's prefix; give each kind its actions; make relations and tick which actions each carries; set a kind's parents by choosing them; start from a template (owner/editor/viewer on one kind; workspace with members over child kinds; team-scoped resources) and change it. The kinds are drawn as a tree with their relations, editable in place. A test bench beside it lets the person place example people and agents on example resources and ask 'may X do Y to Z', answered by the real permission check against the draft, with the path that allowed or refused it. Saving runs the dry run of R3, lists what would be added and removed and any grants a change would strand, and cannot save a change that strands grants. It saves through the same routes as an upload, so a schema built here and one uploaded through the API are the same record, each editable by the other. Controls are styled from the design tokens, never default ones. The draft being built is held in the builder's own session store, separate from the permission store; the bench's check runs against a scratch copy of the draft loaded into a throwaway namespace of the permission service, so no standing check can see a draft, and the namespace is removed when the bench closes.

**Acceptance:**
- A person builds a two-kind schema with a parent from a template on the screen, with no schema text typed, and saving it yields the same record as uploading the same schema through the API.
- An uploaded schema opens in the builder and edits there.
- The test bench answers 'may X do Y to Z' against the draft with the path, matching the real check after saving.
- A change that would strand grants shows the stranded count before saving and cannot be saved.

**Files:**
- create: surface/identity/src/features/apps/SchemaBuilder.tsx
- create: surface/identity/src/features/apps/schema-builder.css
- create: surface/identity/tests/schema-builder.test.tsx
- create: surface/identity/src/features/apps/SchemaBench.tsx
- modify: surface/identity/src/features/apps/Apps.tsx

**Checklist:**
- C386 — A person builds an app's whole permission template on the Apps screen from templates, tests it on example people and resources against the real check, and saves it; built and uploaded schemas are the same record (DIRECTORY-048 R8).

**Stories:**
- S151 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As a developer of an app, I want to register my app and its permission schema through a documented API, so that my app's resources and actions are checked by Lys without anyone changing Lys.
- S152 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As an administrator, I want to approve each app and see its schema on a Lys screen before it takes effect, so that no app gives itself power I have not seen.

## Boundaries

- SHALL NOT put any app's name, kind, schema or special case in Lys code, configuration or tests beyond fixtures named for the test.
- SHALL NOT make Lys call, poll, wait on or read the store of any app.
- SHALL NOT add a timeout, deadline, sleep, poll interval, #[allow], #[ignore] or any bypass.
- SHALL NOT print or log a secret, token, password or key value on any path.
- SHALL NOT add a silent fallback: every failure is a named refusal.
- SHALL NOT let a registration or schema change take effect before an administrator approves it on a Lys screen.
- SHALL NOT revoke or rewrite a grant as a side effect of a schema change.
- SHALL NOT create anything at the provider, or grant anything, before an administrator approves.
- SHALL NOT start this continuation before DIRECTORY-047 R3's provider is complete and green and is present in the card's integrated tree.
- SHALL NOT replace the main branch's DIRECTORY-049 structure descriptions in design.json with older card wording; preserve main's descriptions and regenerate DESIGN.md during integration.
- SHALL NOT claim complete or green for this card while any required provider integration, ordinary register_app authority, file-wall correction or measured acceptance remains unresolved.
- SHALL NOT land without DIRECTORY-045 and DIRECTORY-047 present in dependency order in the integrated tree. Builds may stack on a complete, green prerequisite head. Tom's 29 September 2026 06:53 Melbourne ruling permits one src_land of a fully reviewed green stack or green prefix; that landing integrates current main and gets fresh approval.
- SHALL NOT increase the code-line count of crates/lys-identity-server/src/grants.rs above 498 (ADR-111). Move connector holder and authorization helpers into grants_connector.rs within R1's named file wall. Code may move out of grants.rs.

## Verification

- The full Lys gate, ast-grep scan and the surface checks exit 0 at the card's head, measured by the card round.
- R1 and R3 run end to end on a scratch install: register, approve on the screen, sign in, check, change schema.
- sh scripts/file-length.sh exits 0 at the card's head.
- check_file_length.py counts no more than 498 code lines in crates/lys-identity-server/src/grants.rs at the card's head.
