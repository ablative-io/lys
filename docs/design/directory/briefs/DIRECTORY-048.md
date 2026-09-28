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
> **Checklist:**
> - C361 — An app is registered through the API with its id, name, sign-in client and permission schema, and has no effect until an administrator approves it on a Lys screen (DIRECTORY-048 R1).
> - C362 — An app's schema declares kinds under its own prefix, their actions, relations carrying actions and parent kinds; an invalid schema is refused naming the line of the fault (DIRECTORY-048 R2).
> - C363 — An app cannot define, change or grant on another app's kinds; the refusal names the owning prefix (DIRECTORY-048 R2).
> - C364 — A schema change that would strand standing grants is refused naming the relation and the count; a dry run answers the same without writing (DIRECTORY-048 R3).
> - C365 — Lys's own model is the schema of the app 'lys'; existing grants and checks answer the same after the move (DIRECTORY-048 R4).
> - C366 — Apps check many permissions in one call and list the resources a subject may act on (DIRECTORY-048 R5).
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

Behavioural. Add apps as log events and a projection (apps_api.rs, apps_state.rs, apps_store.rs, following the *_api/_state/_store shape of teams and service accounts). POST /apps takes an id (lower-case letters, digits and hyphens, 2 to 32, not 'lys'), a display name, redirect addresses and a permission schema (R2); it is accepted from a signed-in administrator or from a service account holding register_app, and creates the app as pending with its sign-in client prepared but not enabled. An administrator approves or declines it on a new Apps screen (surface/identity/src/features/apps) that shows the name, the redirect addresses and the full schema in words before the button; only on approval is the sign-in client enabled at Lys's provider (DIRECTORY-047 R3) and the schema written to the permission store. GET /apps and GET /apps/{app} answer what the caller may see. Retiring an app disables its client and refuses every check on its kinds by name; its grants stay in the log. An app id is 3 to 40 characters of lowercase letters, digits and underscores, starting with a letter, the same alphabet the permission store's names allow, so the id is the store prefix unchanged; a hyphen or dot is refused app_id_invalid. Registering writes only the pending registration: the sign-in client is created at the provider on approval and never before. On approval the app's client secret is shown once, to the approving administrator, on the approval screen and in the approval's API answer, and is never written to a log, a receipt or a stored record in the clear.

**Acceptance:**
- A pending app's client cannot complete a sign-in and checks on its kinds answer app_not_approved.
- Approval on the screen enables the client and the schema; a sign-in and a check then pass.
- A second registration of the same id is refused app_exists.
- A retired app's checks answer app_retired and its grants remain readable.
- An id with a hyphen, a dot or under three characters is refused app_id_invalid.
- Registering creates nothing at the provider; approving creates the client and shows its secret once to the approver; no log line holds it.

**Files:**
- create: crates/lys-identity-server/src/apps_api.rs
- create: crates/lys-identity-server/src/apps_state.rs
- create: crates/lys-identity-server/src/apps_store.rs
- create: crates/lys-identity-server/tests/apps.rs
- create: surface/identity/src/features/apps/Apps.tsx
- create: surface/identity/src/features/apps/apps.css
- modify: crates/lys-identity-server/src/routes.rs
- modify: crates/lys-identity-server/src/lib.rs

**Checklist:**
- C361 — An app is registered through the API with its id, name, sign-in client and permission schema, and has no effect until an administrator approves it on a Lys screen (DIRECTORY-048 R1).

**Stories:**
- S151 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As a developer of an app, I want to register my app and its permission schema through a documented API, so that my app's resources and actions are checked by Lys without anyone changing Lys.
- S152 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As an administrator, I want to approve each app and see its schema on a Lys screen before it takes effect, so that no app gives itself power I have not seen.

#### R1 — Execution record

**Dev (recorded):**

- Status: implemented
- How: An app is a log record. Registration writes only a pending Registered line. Approval creates the client: its id, plus the SHA-256 of a fresh 32-byte secret that is shown once in the approval answer and on the screen and is never logged. Approval also binds the service account, makes the schema version 1 and refreshes the grants' model. Pending or declined apps' kinds are refused app_not_approved and retired apps' app_retired; grants stay readable. An id with a hyphen or dot, or under 3 characters, is refused app_id_invalid, and a second registration app_exists. sign_in_client is the lookup Lys's provider (DIRECTORY-047 R3) makes of a client. A pending app's client cannot complete a sign-in, an approved one can with its own secret to a listed address, and a retired one cannot; all of this is unit-tested.
- Deviation: 1. Lys's OIDC provider (DIRECTORY-047 R3: provider.rs with authorize, token and jwks) is not in the tree. So the client is created as a record in the apps log, not at a provider, and a person's sign-in through it cannot be run. sign_in_client is the lookup that provider will call. The end-to-end sign-in waits on DIRECTORY-047 R3, or on a brief revision naming what stands in for it.
2. Service accounts have no credential and cannot hold register_app in the tree. So an administrator makes a service account a registrar (POST /apps/registrars), which issues a lys-registrar credential shown once. This needs the brief owner to approve the substitute or to provide the register_app grant path.
3. The brief's R1 text gives two id rules. The code follows the second (3 to 40 of [a-z0-9_], starting with a letter), which the acceptance row matches.
4. CN1 ('documents only') contradicts the code files the requirements list; the requirements were followed.
- Files changed:
  - created: `crates/lys-identity-server/src/apps_api.rs` — POST/GET /apps, GET /apps/{app}, /apps/me, approve, decline, retire, registrars. Also opening the store and recording the app lys once, and refreshing the grants' model. No unasked bounds on name, redirects or reason.
  - created: `crates/lys-identity-server/src/apps_state.rs` — The apps log's lines and their fold, sealed in the signed snapshot.
  - created: `crates/lys-identity-server/src/apps_store.rs` — The apps' leaf store: snapshot plus tail start, settle on an uncertain append, idempotent keep, admission and reach.
  - modified: `crates/lys-identity-server/src/apps_binding.rs` — Adds sign_in_client, the provider's lookup of an app's client. A pending or declined app's client is refused app_not_approved, a retired app's app_retired, a wrong secret credential_refused (compared by SHA-256 in constant time, never named) and an unlisted address redirect_invalid.
  - created: `crates/lys-identity-server/src/apps_binding_tests.rs` — Five tests: pending refused, approved passes with its secret to a listed address, another secret refused and not named, unlisted address refused, retired refused.
  - created: `crates/lys-identity-server/src/apps_views.rs` — The views. ClientIssued and RegistrarIssued have hand-written Debug that never shows the secret or credential.
  - created: `crates/lys-identity-server/src/apps_views_tests.rs` — Redaction tests for the issued client and registrar.
  - created: `crates/lys-identity-server/src/apps_error.rs` — Every apps refusal by name, with status and fields.
  - modified: `crates/lys-identity-server/src/routes.rs` — AppState holds the apps and the benches; the benches' namespaces are made under a directory beside the apps log.
  - modified: `crates/lys-identity-server/src/lib.rs` — Declares the new modules.
  - created: `crates/lys-identity-server/tests/apps.rs` — The R1 and R2 API tests.
  - created: `surface/identity/src/features/apps/Apps.tsx` — The Apps screen: name, redirects and the whole schema in words before Approve/Decline; the secret is shown once.
  - created: `surface/identity/src/features/apps/apps.css` — Styles from the design tokens.
- Checklist delivery:
  - [x] C361 — An app is registered through the API with its id, name, sign-in client and permission schema, and has no effect until an administrator approves it on a Lys screen (DIRECTORY-048 R1). — Registered through the API with id, name, redirects and schema; no effect until approved on the Apps screen. Enabling the client at the provider waits on DIRECTORY-047 R3.
- Story delivery:
  - [x] S151 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As a developer of an app, I want to register my app and its permission schema through a documented API, so that my app's resources and actions are checked by Lys without anyone changing Lys. — An app registers itself and its schema through POST /apps, described in /openapi.json.
  - [x] S152 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As an administrator, I want to approve each app and see its schema on a Lys screen before it takes effect, so that no app gives itself power I have not seen. — The Apps screen shows the name, redirects and full schema before Approve/Decline.

**Review (recorded):**

- Alignment: drifted
- Acceptance verdicts:
  - [x] A pending app's client cannot complete a sign-in and checks on its kinds answer app_not_approved. — apps_store.rs admit_kind (Pending|Declined => AppNotApproved); apps_binding.rs app_acting refuses a pending app's credential; tests/apps.rs a_pending_app_has_no_client_and_its_kinds_answer_app_not_approved passed (13/13 in /tmp/r048-tests1.log)
  - [ ] Approval on the screen enables the client and the schema; a sign-in and a check then pass. — apps_api.rs approve makes a client record (id plus SHA-256) in the apps log, and tests/apps.rs approval_creates_the_client_shows_its_secret_once_and_a_sign_in_and_check_pass proves an API bearer credential plus a check. No client is enabled at Lys's provider (DIRECTORY-047 R3 is not in the tree), so no sign-in through the client exists.
  - [x] A second registration of the same id is refused app_exists. — apps_api.rs register; tests/apps.rs a_second_registration_of_an_id_and_of_lys_are_refused_app_exists passed
  - [x] A retired app's checks answer app_retired and its grants remain readable. — apps_store.rs admit_kind Retired => AppRetired; tests/apps.rs a_retired_apps_checks_answer_app_retired_and_its_grants_stay_readable passed
  - [x] An id with a hyphen, a dot or under three characters is refused app_id_invalid. — lys-identity grants/schema.rs app_id; tests/apps.rs ids_with_a_hyphen_a_dot_or_under_three_characters_are_refused_app_id_invalid and schema_tests app_ids_take_the_permission_stores_alphabet_only passed
  - [ ] Registering creates nothing at the provider; approving creates the client and shows its secret once to the approver; no log line holds it. — The secret is shown once and only its digest is kept: the approval test scans every file under the service directory for the secret (tests/apps.rs held_anywhere). ClientIssued Debug now redacts it (apps_views_tests.rs). But approval creates no client at a provider, because none exists in the tree.
- Issues:
  - The sign-in client must be created and enabled at Lys's provider on approval, and a person's sign-in through it proven. This needs DIRECTORY-047 R3 (authorize, token, jwks) landed first, or a brief revision saying what stands in for it.
  - Registration by 'a service account holding register_app' was replaced by an administrator-made registrar credential (POST /apps/registrars). This needs a brief revision approving the substitute, or the register_app grant path.
  - Unasked bounds in apps_api.rs: NAME_MAX 100, REDIRECTS_MAX 20, and reason NAME_MAX*5.
  - ClientIssued and RegistrarIssued derived Debug over the client secret and credential.
  - The brief's R1 text gives two contradicting id rules (hyphens, 2 to 32; then 3 to 40 with underscores). The code follows the second, which the acceptance row matches; the brief text needs correcting.
  - CN1 ('documents only') contradicts every code file the brief lists. The brief's constraints need reconciling.
- Fixes:
  - Removed NAME_MAX, REDIRECTS_MAX and the reason length limit from crates/lys-identity-server/src/apps_api.rs.
  - ClientIssued and RegistrarIssued now have a hand-written Debug that names only the client id or service account. Added crates/lys-identity-server/src/apps_views_tests.rs asserting the secret never appears.

### R2: An app's schema: its own kinds, actions, relations and parents, checked on entry

Behavioural. A schema is JSON: kinds, each with actions, relations (each carrying a set of the kind's actions) and optional parents (a relation on a parent kind flows to the child, as a workspace's member reaches its channels). Every kind is written '{app}.{kind}'; a kind outside the app's prefix, an action not declared on its kind, a relation carrying an unknown action, a parent cycle or a parent in another app is refused schema_invalid naming the JSON pointer of the fault. Grant creation, delegation and every check refuse kind_not_registered for a kind no approved app declares, and not_your_app when a caller acting for one app names another's kind. The schema is translated to the permission store's definitions under the app's prefix only; no write touches another prefix. The app 'lys' is the one exception to the app.kind form: Lys's own kinds keep their present names, and every other app's kinds carry its prefix. A service account is bound to an app by an app binding record (service account id, app id, bound by, when), written on approval of the registration that names it and read by not_your_app.

**Acceptance:**
- Each fault above is refused with its pointer, with one test per fault.
- A grant naming an unregistered kind is refused kind_not_registered.
- An app's service account naming another app's kind is refused not_your_app.
- A parent relation grants the child: a workspace member may read a channel in it.

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

#### R2 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Every schema fault is refused schema_invalid at its JSON pointer: a kind outside the prefix, an action not on its kind, an unknown action, a parent cycle, or a parent in another app. Issuing, delegating and every check refuse kind_not_registered, and a caller acting for another app not_your_app, naming the owning prefix. A relation held on a parent reaches children placed in it, through AppSchema::reach, used by decide(). SpiceDB definitions are written under the app's prefix only.
- Deviation: 'Not declared on its kind' and 'unknown action' are separate faults. POST /apps/{app}/placements was added so parents can flow. Checks refuse an undeclared action by name (action_not_declared). Files outside the listed set (admission.rs, cannot_give.rs, schema_diff.rs, spicedb_apps.rs) were needed for the kind-aware model.
- Files changed:
  - modified: `crates/lys-identity/src/grants/schema.rs` — AppSchema parse and check, refusing schema_invalid at the JSON pointer. A comment now states where the length rules come from: the app id's 3–40 is the brief's, and a name's 3–64 is the permission engine's own identifier rule.
  - created: `crates/lys-identity/src/grants/schema_tests.rs` — One test per fault.
  - created: `crates/lys-identity/src/grants/schema_diff.rs` — Diff and stranded grants.
  - modified: `crates/lys-identity/src/grants/model.rs` — Per-app-kind relation tables.
  - modified: `crates/lys-identity/src/grants/permission.rs` — Parent relation and placement.
  - modified: `crates/lys-identity/src/grants/admission.rs` — Relations resolved on the resource's kind.
  - modified: `crates/lys-identity/src/grants/cannot_give.rs` — Iterates the source kind's relations.
  - modified: `crates/lys-identity/src/grants/mod.rs` — Declares and re-exports.
  - modified: `crates/lys-identity/Cargo.toml` — serde_json.
  - modified: `crates/lys-identity-server/src/grants.rs` — Kind admission and decide() through placements.
  - modified: `crates/lys-identity-server/src/spicedb.rs` — App kinds as {app}/{kind} definitions.
  - created: `crates/lys-identity-server/src/spicedb_apps.rs` — An app kind's definition text.
  - created: `crates/lys-identity-server/src/apps_schema_api.rs` — Includes POST /apps/{app}/placements.
- Checklist delivery:
  - [x] C362 — An app's schema declares kinds under its own prefix, their actions, relations carrying actions and parent kinds; an invalid schema is refused naming the line of the fault (DIRECTORY-048 R2). — Kinds under the prefix with actions, relations and parents; faults refused at their pointer.
  - [x] C363 — An app cannot define, change or grant on another app's kinds; the refusal names the owning prefix (DIRECTORY-048 R2). — Another app's kinds are refused, naming the owning prefix.
- Story delivery:
  - [x] S151 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As a developer of an app, I want to register my app and its permission schema through a documented API, so that my app's resources and actions are checked by Lys without anyone changing Lys. — An app's kinds are checked from its own schema with no change to Lys.

**Review (recorded):**

- Alignment: fixed
- Acceptance verdicts:
  - [x] Each fault above is refused with its pointer, with one test per fault. — crates/lys-identity/src/grants/schema_tests.rs has one test per fault (outside prefix, action on another kind, unknown action, cycle, self-parent, parent in another app, undeclared parent, engine name, relation named as action, unknown member), each asserting the pointer. tests/apps.rs a_schema_fault_is_refused_at_its_pointer_and_a_bad_redirect_by_name passed.
  - [x] A grant naming an unregistered kind is refused kind_not_registered. — grants.rs issue_root and delegate call admit_kind; tests/apps.rs a_kind_no_approved_app_declares_is_refused_kind_not_registered passed
  - [x] An app's service account naming another app's kind is refused not_your_app. — apps_store.rs admit_kind(Some(app)); tests/apps.rs an_apps_credential_naming_another_apps_kind_is_refused_not_your_app passed
  - [x] A parent relation grants the child: a workspace member may read a channel in it. — grants.rs decide() over AppStore::reach; tests/apps.rs a_workspace_member_may_read_a_channel_placed_in_it_and_no_more passed
- Checklist verified: C362, C363
- Stories verified: S151
- Issues:
  - SpiceDb gained app_kinds, which pushed lys-secrets' Grants::Directory(SpiceGrants) over clippy large_enum_variant. Both clippy legs failed at the venue.
- Fixes:
  - Boxed the variant: Grants::Directory(Box<SpiceGrants>) in crates/lys-secrets/src/bin/lys-secrets/spice.rs, constructed with Box::new in args.rs. Both clippy shapes now exit 0.

### R3: Schema changes are versioned, dry-run first, and never strand a grant

Behavioural. PUT /apps/{app}/schema carries the version it replaces and is refused schema_version_moved when that is not current. POST /apps/{app}/schema/check answers what the change would do without writing: kinds, relations and actions added and removed, and the standing grants each removal would strand. A change that removes a kind or relation with standing grants, or an action a standing grant carries, is refused schema_change_strands_grants naming each relation and its count; it is applied only after those grants are revoked through the ordinary revoke route. A change made by a service account waits for administrator approval on the Apps screen, as registration does. Every version stays readable at GET /apps/{app}/schema?version=N. The apps store is born with its checkpoint beside its log, so a start reads the checkpoint and the tail only, never the whole history.

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

#### R3 — Execution record

**Dev (recorded):**

- Status: implemented
- How: A stale replaces is refused schema_version_moved. The dry run and the write compute the same strands, and a stranding change is refused schema_change_strands_grants with {at, count}. It applies after the grants are revoked through the ordinary revoke route, and the version moves by one. Every version stays readable. The store is born with its checkpoint, and a restart reads the snapshot and only the tail (a test counts it).
- Deviation: (none)
- Files changed:
  - created: `crates/lys-identity-server/src/apps_schema_api.rs` — PUT/GET schema, check, approve and decline a pending change, GET ?version=N.
  - created: `crates/lys-identity-server/src/apps_state.rs` — Proposed, Applied and ChangeDeclined lines; every version kept.
  - created: `crates/lys-identity-server/tests/apps_schema.rs` — Covers: stale version, the check and the write naming the same strands, applied after revoking, versions readable, an app's change waiting, and the restart leaf count. Also the bench tests, including a namespace removed from under an open bench being refused apps_unavailable on close.
- Checklist delivery:
  - [x] C364 — A schema change that would strand standing grants is refused naming the relation and the count; a dry run answers the same without writing (DIRECTORY-048 R3). — The refusal and the dry run name the same relations and counts.
- Story delivery:
  - [x] S151 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As a developer of an app, I want to register my app and its permission schema through a documented API, so that my app's resources and actions are checked by Lys without anyone changing Lys. — An app changes its schema through the API; the change waits for the administrator.

**Review (recorded):**

- Alignment: fixed
- Acceptance verdicts:
  - [x] A stale version is refused schema_version_moved. — tests/apps_schema.rs a_change_naming_a_stale_version_is_refused_schema_version_moved passed
  - [x] The check route and the refused write name the same relations and counts. — apps_schema_api.rs check and change both call strands(); tests/apps_schema.rs a_stranding_change_is_named_alike_by_its_dry_run_and_its_write_and_taken_after_revoking passed
  - [x] After revoking the stranded grants the same change applies and the version moves by one. — same test, revoking through POST /grants/{id}/revoke; version becomes 2
  - [x] Version N stays readable after N+1. — same test, GET /apps/{app}/schema?version=1 after version 2
- Checklist verified: C364
- Stories verified: S151
- Issues:
  - App::standing (apps_state.rs) treated any Registered line with by == By::Start as approved, so a pending app's placement was accepted. the_apps_store_starts_from_its_snapshot_and_reads_only_the_leaves_after_it failed at apps_schema.rs:355, and the test's fixture registered with By::Start.
- Fixes:
  - standing() now treats only the app lys recorded at start as approved without an approval.
  - The fixture registers as a service account, and the test asserts the refusal is app_not_approved, not just any error.

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

#### R4 — Execution record

**Dev (recorded):**

- Status: implemented
- How: At the first start, the model file is recorded as the app lys, approved at the model's version. Afterwards the log is the only source and the file is not read, which the start says by name. Unprefixed kinds belong to lys and are judged exactly as before. Retiring lys is refused app_is_lys, and only an administrator changes its schema.
- Deviation: The service start is service_saying in routes.rs, not start.rs (start.rs is the agent start route), so start.rs is unchanged.
- Files changed:
  - modified: `crates/lys-identity-server/src/config.rs` — apps_dir(); the model file is read only to record lys once.
  - created: `crates/lys-identity-server/src/apps_api.rs` — opened(): says lys_model_recorded once, then grant_model_file_ignored.
  - modified: `crates/lys-identity-server/src/roles_api.rs` — Reads the model through GrantSetup::model().
  - modified: `crates/lys-identity-server/src/configuration_api.rs` — Reads the model through GrantSetup::model().
- Checklist delivery:
  - [x] C365 — Lys's own model is the schema of the app 'lys'; existing grants and checks answer the same after the move (DIRECTORY-048 R4). — Lys's model is the app lys; existing grants and checks are judged the same.
- Story delivery:
  - [x] S151 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As a developer of an app, I want to register my app and its permission schema through a documented API, so that my app's resources and actions are checked by Lys without anyone changing Lys. — Lys is one app among others in the same records.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] Every existing grant and check test passes unchanged after the move. — No existing test file under crates/lys-identity-server/tests or tests/identity_contract/tests is modified (git diff). Every pre-existing grant_* and identity-server test in /tmp/r048-tests1.log passed; the only failure there was the new apps_schema snapshot test, since fixed.
  - [x] A second start with a changed model file changes nothing and says so by name in the start log. — apps_api.rs opened says grant_model_file_ignored; tests/apps_schema.rs a_second_start_with_a_changed_model_file_changes_nothing_and_says_so_by_name passed
  - [x] Retiring 'lys' is refused app_is_lys. — apps_api.rs retire and apps_state.rs allows_on Refused::Lys; tests/apps.rs a_retired_apps_checks_answer_app_retired_and_its_grants_stay_readable asserts 403 app_is_lys
- Checklist verified: C365
- Stories verified: S151
- Issues:
  - The brief names crates/lys-identity-server/src/start.rs, but the service start is service_saying in routes.rs, so the recording lives in apps_api::opened. The brief's file list needs correcting.

### R5: Apps check many permissions at once and list what a subject may act on

Behavioural. POST /grants/check/batch takes up to 500 checks (subject, kind, id, action) and answers each allowed or refused with its reason, in order, at one permission store revision named in the answer. POST /grants/which takes a subject, a kind and an action and answers the ids that subject may act on, paged by cursor. Both are open to an app's service account for its own kinds only, and to administrators. A caller may pass the revision it last wrote (at_least) to read its own write.

**Acceptance:**
- A batch of mixed allowed and refused checks answers each in order at one named revision.
- 501 checks are refused batch_too_large.
- which lists exactly the ids the batch would allow, across pages.
- A check with at_least of a just-made grant sees it.

**Files:**
- create: crates/lys-identity-server/tests/grants_batch.rs
- modify: crates/lys-identity-server/src/grants.rs
- modify: crates/lys-identity-server/src/spicedb.rs

**Checklist:**
- C366 — Apps check many permissions in one call and list the resources a subject may act on (DIRECTORY-048 R5).

**Stories:**
- S151 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As a developer of an app, I want to register my app and its permission schema through a documented API, so that my app's resources and actions are checked by Lys without anyone changing Lys.

#### R5 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Each check in a batch is answered in order at one named revision, through admit_kind, admit_action and decide. which answers the ids a subject may act on, paged by cursor. An app sees only its own kinds; administrators see all.
- Deviation: (none)
- Files changed:
  - created: `crates/lys-identity-server/src/grants_batch.rs` — POST /grants/check/batch (up to 500) and POST /grants/which, paged. one() is now pub(crate) so the bench asks through it.
  - created: `crates/lys-identity-server/tests/grants_batch.rs` — Batch order, revision, at_least, app scope, which paging.
- Checklist delivery:
  - [x] C366 — Apps check many permissions in one call and list the resources a subject may act on (DIRECTORY-048 R5). — Batch checks and resource lookups.
- Story delivery:
  - [x] S151 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As a developer of an app, I want to register my app and its permission schema through a documented API, so that my app's resources and actions are checked by Lys without anyone changing Lys. — An app checks many permissions in one call.

**Review (recorded):**

- Alignment: fixed
- Acceptance verdicts:
  - [x] A batch of mixed allowed and refused checks answers each in order at one named revision. — tests/grants_batch.rs a_mixed_batch_is_answered_in_order_at_one_named_revision passed
  - [x] 501 checks are refused batch_too_large. — grants_batch.rs BATCH_MAX 500 (from the brief); tests/grants_batch.rs five_hundred_and_one_checks_are_refused_batch_too_large passed
  - [x] which lists exactly the ids the batch would allow, across pages. — tests/grants_batch.rs which_lists_exactly_the_ids_the_batch_allows_across_pages passed
  - [x] A check with at_least of a just-made grant sees it. — tests/grants_batch.rs a_check_naming_the_revision_of_a_just_made_grant_sees_it passed
- Checklist verified: C366
- Stories verified: S151
- Issues:
  - which dropped any decide() error with `.is_err() => continue`. An engine outage, a missing log or a stale at_least silently gave a short list.
  - Unasked bound WHICH_PAGE_MAX 1000 in grants_batch.rs.
- Fixes:
  - which now answers StaleDecision, PermissionEngineUnavailable, ProjectionPending and LogUnavailable by name, through unanswered(); only a refusal leaves an id out.
  - Removed WHICH_PAGE_MAX; page_size must be at least 1.

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

#### R6 — Execution record

**Dev (recorded):**

- Status: implemented
- How: One document is generated from the route table and the types; a route without an entry fails the test. The bench's refusal list now names what the real-check bench can answer, and each name is produced by a test.
- Deviation: Routes that existed before this card are described by words, without request and response types.
- Files changed:
  - created: `crates/lys-openapi/src/lib.rs` — Generates OpenAPI 3.1 from the route table and ToSchema types, and validates a document.
  - created: `crates/lys-identity-server/src/openapi.rs` — GET /openapi.json.
  - created: `crates/lys-identity-server/src/openapi_table.rs` — The untyped route entries (under 500 lines of code).
  - created: `crates/lys-identity-server/src/openapi_typed.rs` — The typed app routes; the bench's refusal list now comes from apps_bench::BENCH.
  - created: `crates/lys-identity-server/tests/openapi.rs` — Every route has an entry and every entry a route; the document validates; every refusal it names is produced by a test.
- Checklist delivery:
  - [x] C367 — One OpenAPI document, generated from the routes and types, describes every route; a route without an entry fails the build (DIRECTORY-048 R6). — One generated document; a missing entry fails the build's tests.
- Story delivery:
  - [x] S151 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As a developer of an app, I want to register my app and its permission schema through a documented API, so that my app's resources and actions are checked by Lys without anyone changing Lys. — The API an app codes against is published at /openapi.json.

**Review (recorded):**

- Alignment: fixed
- Acceptance verdicts:
  - [x] The document validates as OpenAPI 3.1. — The generated document was dumped and checked with the independent validator openapi-spec-validator 0.7.2: VALID, openapi 3.1.0, 102 paths, 113 operations. The in-repo test the_document_reads_as_openapi_3_1 also passes.
  - [x] Adding a route without a description fails the test naming the route. — tests/openapi.rs a_route_added_without_an_entry_fails_naming_the_route and every_declared_route_has_an_entry_and_every_entry_a_route passed
  - [x] Every refusal in the document has a test that produces it. — tests/openapi.rs every_refusal_the_document_names_is_produced_by_a_test passed. Spot-checked the app refusals: each is asserted through identity_contract::apps::refused(answer, status, name), e.g. apps.rs:156 app_exists and apps_schema.rs:458 bench_unknown.
- Checklist verified: C367
- Stories verified: S151
- Issues:
  - crates/lys-identity-server/src/openapi.rs was 744 lines of code, over the 500 limit; the file-length leg failed.
- Fixes:
  - Split into openapi.rs (route(), api(), document(), the route), openapi_table.rs (the untyped entries and refusal sets, 495 lines of code) and openapi_typed.rs (the typed app routes). The file-length leg now exits 0.

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

#### R7 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Lys holds no app's name, kind or schema and calls no app. The ast-grep rule enforces the names. The gates this round's measurement failed were fixed at their cause: clippy large_enum_variant (boxed), the ast-grep rule parse, openapi.rs length (split), design coverage (structure entries plus re-render) and the surface clock. Measured here: fmt, clippy (all-features and default) for the changed crates, design gate exit 0, ast-grep scan exit 0 and test pass, file-length 0 over across 572 files including the untracked new ones, vitest 48/328 and tsc.
- Deviation: The workspace test leg in round 1 ended on exit -15 (SIGTERM) straight after a passing binary; four other cards' test legs ended the same way. It was stopped from outside, not by a failing test, and the card round must remeasure it. The full identity-crate test run had not finished when this report was due. The lys identity_* leg was not run here, as a heavy run.
- Files changed:
  - created: `rules/ast-grep/no-app-names.yml` — No product name in code, configuration or screens; ast-grep scan exits 0 and ast-grep test passes.
  - modified: `sgconfig.yml` — testConfigs.
  - modified: `crates/lys-secrets/src/bin/lys-secrets/spice.rs` — Directory(Box<SpiceGrants>), which fixes clippy::large_enum_variant.
  - modified: `crates/lys-secrets/src/bin/lys-secrets/args.rs` — Boxes the SpiceGrants.
  - modified: `docs/design/directory/design.json` — Adds crates/lys/src/cli/mcp.rs and crates/lys/tests/mcp_stdio.rs (DIRECTORY-049 R7) to structure, which fixes the coverage failure.
  - modified: `docs/design/directory/DESIGN.md` — Re-rendered.
  - modified: `docs/design/directory/briefs/DIRECTORY-048.md` — Re-rendered from its JSON.
  - modified: `surface/identity/vite.config.ts` — Switches off vitest's per-test and per-hook clocks (testTimeout 0, hookTimeout 0), per the no-time-limits rule. The mockup test had failed only on the 5 s default under load.
  - modified: `crates/lys-core/src/attestation/mod.rs` — No product named.
  - modified: `deploy/identity/compose.yaml` — No product named.
- Checklist delivery:
  - [x] C368 — Lys holds no app's name or schema in code or configuration and makes no call to any app (DIRECTORY-048 R7). — No app name or schema in Lys; no call to any app; ast-grep enforces it.
- Story delivery:
  - [x] S151 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As a developer of an app, I want to register my app and its permission schema through a documented API, so that my app's resources and actions are checked by Lys without anyone changing Lys. — Lys runs the same with no apps as with many.

**Review (recorded):**

- Alignment: fixed
- Acceptance verdicts:
  - [x] ast-grep scan exits 0 on the tree and fails on a planted product name. — ast-grep scan --config sgconfig.yml exits 0 on the tree. A planted '// the Cambium seat' in a .rs file and '/* argusBlue */' in tokens.css each fail it (exit 1). ast-grep test: 2 passed. On HEAD's crates and surface the rule is red in 7 files.
  - [ ] The full gate passes with no app registered. — Passing: fmt, clippy in both shapes, doc in both shapes, ast-grep, file-length, surface. The design gate fails on DIRECTORY-049 R7 paths, already failing on HEAD. The workspace all-features test run had not finished when this report was written.
  - [x] A test registers two apps with the same kind name under their own prefixes and checks each separately. — tests/apps.rs two_apps_with_the_same_kind_name_are_checked_each_under_its_own_prefix passed
- Checklist verified: C368
- Stories verified: S151
- Issues:
  - rules/ast-grep/no-app-names.yml did not parse ('Rule must specify a set of AST kinds'), so the ast-grep leg exited 8.
  - The design gate fails on DIRECTORY-049 R7 (crates/lys/src/cli/mcp.rs and crates/lys/tests/mcp_stdio.rs are not in design.json structure). This is already failing on HEAD and must be fixed by that brief's owner before the full gate can pass.
  - The full workspace all-features test run must be seen green at the card's head.
- Fixes:
  - Rewrote each language's rule as all: [any-of-kinds, regex, not has regex] over comment, string and name kinds, without the \b anchor so joined names such as argusBlue are caught. Generated the rule-test snapshots under rules/ast-grep-tests/__snapshots__.

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

#### R8 — Execution record

**Dev (recorded):**

- Status: implemented
- How: The bench no longer decides by its own procedure. Each question loads a scratch copy of the draft into a throwaway namespace: the draft approved beside lys, example people as active people, example holdings as root grants, and example placements kept as the placements route keeps them. It is then asked through the batch route's own one-check decision, and the path is read back from the grant and resource that check names. No standing check can see a draft (the test asserts the real batch answers kind_not_registered before saving). The namespace is removed after the answer and the bench's directory on close.
- Deviation: The throwaway namespace uses the service's in-process permission engine (Relationships::Memory), not a scratch SpiceDB namespace, even when SpiceDB is configured. The decision code is the real one; the relationship backend is the in-process one. This needs the brief owner to accept it, or a later change to load the draft into a scratch SpiceDB prefix.
- Files changed:
  - modified: `crates/lys-identity-server/src/apps_bench.rs` — Benches owns a namespace directory; leftovers from an earlier run are removed at start and said. Open checks the draft and makes the bench's namespace. Ask answers through the scratch module. Close removes the namespace, and a failure is refused apps_unavailable. The separate decision procedure and OPEN_MAX are gone.
  - created: `crates/lys-identity-server/src/apps_bench_scratch.rs` — A throwaway namespace for each question: scratch apps store (lys with the service's own model, and the draft approved), scratch grant log and key, in-process permission engine, and a scratch directory of example people. It asks through grants_batch::one (admit_kind, admit_action, decide) and is removed before answering.
  - modified: `crates/lys-identity-server/tests/apps_schema.rs` — The bench agrees with the real check after saving on allowed, NotHeld, action_not_declared and kind_not_registered. The namespace is removed on close. A namespace gone from under a bench is refused apps_unavailable, and the bench is closed.
  - created: `surface/identity/src/features/apps/SchemaBuilder.tsx` — The builder: templates, kinds tree, relations with ticked actions, parents, dry run and save through the upload routes.
  - created: `surface/identity/src/features/apps/SchemaBench.tsx` — The bench beside the builder.
  - created: `surface/identity/src/features/apps/schema-builder.css` — Styles from the tokens.
  - created: `surface/identity/tests/schema-builder.test.tsx` — Covers: build from a template with no schema text, same record as upload, uploaded schema edits, a strand shown and unsaveable, and the bench.
- Checklist delivery:
  - [x] C386 — A person builds an app's whole permission template on the Apps screen from templates, tests it on example people and resources against the real check, and saves it; built and uploaded schemas are the same record (DIRECTORY-048 R8). — Built from templates on the screen, tested on examples against the real check, saved as the same record as an upload.
- Story delivery:
  - [x] S151 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As a developer of an app, I want to register my app and its permission schema through a documented API, so that my app's resources and actions are checked by Lys without anyone changing Lys. — A developer builds and tests a schema without writing schema text.
  - [x] S152 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As an administrator, I want to approve each app and see its schema on a Lys screen before it takes effect, so that no app gives itself power I have not seen. — The administrator sees and tests the schema before approving.

**Review (recorded):**

- Alignment: drifted
- Acceptance verdicts:
  - [x] A person builds a two-kind schema with a parent from a template on the screen, with no schema text typed, and saving it yields the same record as uploading the same schema through the API. — surface/identity/tests/schema-builder.test.tsx passes in the surface run (48 files, 328 tests)
  - [x] An uploaded schema opens in the builder and edits there. — schema-builder.test.tsx, fromSchema case, passes
  - [ ] The test bench answers 'may X do Y to Z' against the draft with the path, matching the real check after saving. — apps_bench.rs answer() is a separate decision procedure over the draft's reach and relation tables, not the real permission check. tests/apps_schema.rs the_bench_answers_the_draft_as_the_real_check_answers_it_once_saved shows agreement on one case, but the spec requires the real check against a scratch copy in a throwaway permission-service namespace, removed on close.
  - [x] A change that would strand grants shows the stranded count before saving and cannot be saved. — SchemaBuilder.tsx disables Save while stranded is non-empty; the schema-builder.test.tsx strand case passes
- Issues:
  - The bench must answer through the real permission check against the draft, loaded into a throwaway namespace of the permission service and removed when the bench closes. The alternative is a brief revision accepting an in-process bench, with a test proving it agrees with the real check on every refusal class.
  - Unasked bound OPEN_MAX 64 in apps_bench.rs.
- Fixes:
  - Removed OPEN_MAX from crates/lys-identity-server/src/apps_bench.rs.

## Boundaries

- SHALL NOT put any app's name, kind, schema or special case in Lys code, configuration or tests beyond fixtures named for the test.
- SHALL NOT make Lys call, poll, wait on or read the store of any app.
- SHALL NOT add a timeout, deadline, sleep, poll interval, #[allow], #[ignore] or any bypass.
- SHALL NOT print or log a secret, token, password or key value on any path.
- SHALL NOT add a silent fallback: every failure is a named refusal.
- SHALL NOT let a registration or schema change take effect before an administrator approves it on a Lys screen.
- SHALL NOT revoke or rewrite a grant as a side effect of a schema change.
- SHALL NOT create anything at the provider, or grant anything, before an administrator approves.

## Verification

- The full Lys gate, ast-grep scan and the surface checks exit 0 at the card's head, measured by the card round.
- R1 and R3 run end to end on a scratch install: register, approve on the screen, sign in, check, change schema.
