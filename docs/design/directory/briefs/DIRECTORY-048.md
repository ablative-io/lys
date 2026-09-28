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

- Status: blocked
- How: Unchanged from round 2. An app is a log record, pending until approved. Approval records the client and its secret's SHA-256, shows the secret once and writes it to no log. A pending app's kinds are refused app_not_approved and a retired app's app_retired, and its grants stay readable. sign_in_client is the provider's client lookup, and it refuses a pending, declined or retired app by name.
- Deviation: Blocked on DIRECTORY-047 R3. Lys's OpenID provider (crates/lys-identity-server/src/provider.rs) is not in this tree. It exists, uncommitted, in the DIRECTORY-047b card clone; its client lookup, ProductClient {secret_sha256, redirect_uris}, is the seam sign_in_client fills. So approval cannot enable a client at the provider, and no person can sign in through an app's client. Once 047 R3 lands, the wiring is: the provider's client() calls sign_in_client, and tests/apps.rs:84 signs in through the client instead of GET /apps/me. The registrar credential stands in for 'a service account holding register_app'. In this tree, grants are held only by people and agents, and a service account is a store record with a string id, so it cannot hold a register_app grant. That needs the brief owner's decision. The files outside R1's file list (apps_binding.rs, apps_views.rs, apps_error.rs) need the brief revision CN9 asks for.
- Files changed:
  - created: `crates/lys-identity-server/src/apps_api.rs` — The app routes: register, list, read one, the app's own record (/apps/me), approve, decline, retire and registrars. Approval records the client, shows its secret once and never logs it.
  - modified: `crates/lys-identity-server/src/apps_binding.rs` — sign_in_client, the lookup the provider will make of an app's client.
  - created: `crates/lys-identity-server/src/apps_binding_tests.rs` — Unit tests of sign_in_client: pending, approved, wrong secret, unlisted address, retired.
  - created: `crates/lys-identity-server/tests/apps.rs` — The R1 API tests.
  - created: `surface/identity/src/features/apps/Apps.tsx` — The Apps screen.
- Checklist delivery:
  - [ ] C361 — An app is registered through the API with its id, name, sign-in client and permission schema, and has no effect until an administrator approves it on a Lys screen (DIRECTORY-048 R1). — Registration and approval are done. Enabling the client at the provider, and signing in through it, wait on DIRECTORY-047 R3.
- Story delivery:
  - [x] S151 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As a developer of an app, I want to register my app and its permission schema through a documented API, so that my app's resources and actions are checked by Lys without anyone changing Lys. — An app registers itself and its schema through POST /apps.
  - [x] S152 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As an administrator, I want to approve each app and see its schema on a Lys screen before it takes effect, so that no app gives itself power I have not seen. — The Apps screen shows the whole registration before Approve or Decline.

**Review (recorded):**

- Alignment: drifted
- Acceptance verdicts:
  - [ ] A pending app's client cannot complete a sign-in and checks on its kinds answer app_not_approved. — Checks are covered: tests/apps.rs:42 a_pending_app_has_no_client_and_its_kinds_answer_app_not_approved. The sign-in half is only the unit test apps_binding_tests.rs:59 of sign_in_client (apps_binding.rs:251). That function has no production caller, and no provider exists (no crates/lys-identity-server/src/provider.rs), so no sign-in is attempted.
  - [ ] Approval on the screen enables the client and the schema; a sign-in and a check then pass. — tests/apps.rs:84 approves and then checks. Its 'sign-in' is GET /apps/me with the lys-app bearer credential (line ~130), not a person's OIDC sign-in through the app's client. No client is enabled at a provider.
  - [x] A second registration of the same id is refused app_exists. — tests/apps.rs:149 a_second_registration_of_an_id_and_of_lys_are_refused_app_exists; the card round's tests leg exited 0
  - [x] A retired app's checks answer app_retired and its grants remain readable. — tests/apps.rs:185 a_retired_apps_checks_answer_app_retired_and_its_grants_stay_readable
  - [x] An id with a hyphen, a dot or under three characters is refused app_id_invalid. — tests/apps.rs:168; schema_tests.rs:163 app_ids_take_the_permission_stores_alphabet_only
  - [ ] Registering creates nothing at the provider; approving creates the client and shows its secret once to the approver; no log line holds it. — The secret is shown once and no file holds it: tests/apps.rs:84 checks again[client]==null and held_anywhere(...)==false. But approval creates the client only as an apps-log record (apps_api.rs), not at Lys's provider, which is absent.
- Issues:
  - The end-to-end sign-in through an app's client needs DIRECTORY-047 R3's provider (crates/lys-identity-server/src/provider.rs), which is not in the tree. Land DIRECTORY-047 R3 first, or get a brief revision naming what stands in for it. Then wire sign_in_client into the provider's authorize/token path and replace the /apps/me 'sign-in' in tests/apps.rs:84 with a real sign-in through the client.
  - Approval must enable the client at Lys's provider, not only record it in the apps log.
  - The registrar credential and POST /apps/registrars replace 'a service account holding register_app' without approval. Get the brief owner's decision, or give service accounts the register_app grant path.
  - Scope: every file outside R1's wall (apps_binding.rs gains credentials; apps_views.rs; apps_error.rs) needs the brief revision CN9 asks for.

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
- How: Unchanged from round 2. A schema is parsed and checked when it arrives, and each fault is refused by name.
- Deviation: Unchanged from round 2: files beyond the listed set (admission.rs, cannot_give.rs, schema_diff.rs, spicedb_apps.rs) were needed.
- Files changed:
  - created: `crates/lys-identity/src/grants/schema.rs` — The app schema and its checks.
- Checklist delivery:
  - [x] C362 — An app's schema declares kinds under its own prefix, their actions, relations carrying actions and parent kinds; an invalid schema is refused naming the line of the fault (DIRECTORY-048 R2).
  - [x] C363 — An app cannot define, change or grant on another app's kinds; the refusal names the owning prefix (DIRECTORY-048 R2).
- Story delivery:
  - [x] S151 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As a developer of an app, I want to register my app and its permission schema through a documented API, so that my app's resources and actions are checked by Lys without anyone changing Lys.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] Each fault above is refused with its pointer, with one test per fault. — crates/lys-identity/src/grants/schema_tests.rs:53 (outside prefix), :73 (action not on its kind), :84 (unknown action), :94 and :104 (cycle), :113 (parent in another app); tests/apps.rs:436 at the API
  - [x] A grant naming an unregistered kind is refused kind_not_registered. — tests/apps.rs:268 a_kind_no_approved_app_declares_is_refused_kind_not_registered; grants.rs issue_root/delegate call apps.admit_kind
  - [x] An app's service account naming another app's kind is refused not_your_app. — tests/apps.rs:294 an_apps_credential_naming_another_apps_kind_is_refused_not_your_app
  - [x] A parent relation grants the child: a workspace member may read a channel in it. — tests/apps.rs:338 a_workspace_member_may_read_a_channel_placed_in_it_and_no_more; grants.rs decide() goes through apps.reach
- Checklist verified: C362, C363
- Stories verified: S151
- Issues:
  - Files outside R2's wall were edited without the brief revision CN9 requires: admission.rs, cannot_give.rs, schema_diff.rs, spicedb_apps.rs, apps_schema_api.rs, and the new POST /apps/{app}/placements route. Record them in a brief revision.

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
- How: Unchanged from round 2. Schema versions are kept, a change has a dry run first, and a change that would strand a grant is refused.
- Deviation: (none)
- Files changed:
  - modified: `crates/lys-identity-server/tests/apps_schema.rs` — The bench scenario moved into tests/shared/bench.rs, so the live SpiceDB test runs the same body.
- Checklist delivery:
  - [x] C364 — A schema change that would strand standing grants is refused naming the relation and the count; a dry run answers the same without writing (DIRECTORY-048 R3).
- Story delivery:
  - [x] S151 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As a developer of an app, I want to register my app and its permission schema through a documented API, so that my app's resources and actions are checked by Lys without anyone changing Lys.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] A stale version is refused schema_version_moved. — tests/apps_schema.rs:35 a_change_naming_a_stale_version_is_refused_schema_version_moved
  - [x] The check route and the refused write name the same relations and counts. — tests/apps_schema.rs:48 a_stranding_change_is_named_alike_by_its_dry_run_and_its_write_and_taken_after_revoking
  - [x] After revoking the stranded grants the same change applies and the version moves by one. — tests/apps_schema.rs:48 (the same test, after revoking through the ordinary revoke route)
  - [x] Version N stays readable after N+1. — tests/apps_schema.rs:48 and GET /apps/{app}/schema?version=N in apps_schema_api.rs; tests/apps_schema.rs:325 counts the leaves read after the snapshot
- Checklist verified: C364
- Stories verified: S151

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
- How: Unchanged from round 2.
- Deviation: The service starts in service_saying in routes.rs, so start.rs is unchanged.
- Files changed:
  - modified: `crates/lys-identity-server/src/grants.rs` — Lys's own model is the schema of the app lys.
- Checklist delivery:
  - [x] C365 — Lys's own model is the schema of the app 'lys'; existing grants and checks answer the same after the move (DIRECTORY-048 R4).
- Story delivery:
  - [x] S151 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As a developer of an app, I want to register my app and its permission schema through a documented API, so that my app's resources and actions are checked by Lys without anyone changing Lys.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] Every existing grant and check test passes unchanged after the move. — git diff HEAD shows no change to crates/lys-identity-server/tests/grants.rs or tests/identity_contract/tests; the card round's workspace tests leg exited 0
  - [x] A second start with a changed model file changes nothing and says so by name in the start log. — tests/apps_schema.rs:266 a_second_start_with_a_changed_model_file_changes_nothing_and_says_so_by_name; apps_api.rs:116 says grant_model_file_ignored
  - [x] Retiring 'lys' is refused app_is_lys. — tests/apps_schema.rs:201 lys_is_an_app_whose_schema_is_the_model_and_changes_only_by_the_administrator
- Checklist verified: C365
- Stories verified: S151

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
- How: Unchanged from round 2.
- Deviation: (none)
- Files changed:
  - modified: `crates/lys-identity-server/src/grants_batch.rs` — The batch and which routes.
- Checklist delivery:
  - [x] C366 — Apps check many permissions in one call and list the resources a subject may act on (DIRECTORY-048 R5).
- Story delivery:
  - [x] S151 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As a developer of an app, I want to register my app and its permission schema through a documented API, so that my app's resources and actions are checked by Lys without anyone changing Lys.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] A batch of mixed allowed and refused checks answers each in order at one named revision. — tests/grants_batch.rs:15 a_mixed_batch_is_answered_in_order_at_one_named_revision
  - [x] 501 checks are refused batch_too_large. — tests/grants_batch.rs:68; grants_batch.rs:37 BATCH_MAX=500, the brief's number
  - [x] which lists exactly the ids the batch would allow, across pages. — tests/grants_batch.rs:100 which_lists_exactly_the_ids_the_batch_allows_across_pages
  - [x] A check with at_least of a just-made grant sees it. — tests/grants_batch.rs:190 a_check_naming_the_revision_of_a_just_made_grant_sees_it
- Checklist verified: C366
- Stories verified: S151

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
- How: The document is generated from the request, answer and refusal types. Every route names its types except 21 whose answer is open (the secrets broker's forwarded answers, redirects, the configuration dump, the start owners' own JSON, /openapi.json) and 11 that take no typed body; both lists are exact and tested. Refusals are held complete from both sides: tests/openapi.rs fails on a listed refusal no test produces, and the harness fails any test that meets a refusal its route does not list. 123 missing pairs were found and listed. lys-identity-server and identity-contract then passed 271 tests with 0 failed.
- Deviation: The refusal lists are checked against what the tests produce; they are not generated from the error types. /grants/who cannot answer NotHeld: it lists holders and never calls decide. Drift injections showed each list check failing only the test built for it.
- Files changed:
  - created: `crates/lys-identity-server/src/openapi_types.rs` — The request and answer schema of every table route, from its ToSchema types. The module doc lists each route left untyped, with its reason.
  - created: `crates/lys-identity-server/src/directory_views.rs` — Typed answers replacing hand-built json! for receipts, registrations, identities, sign-in, link audit and the service key; the same JSON on the wire.
  - modified: `crates/lys-identity-server/src/routes.rs` — The directory routes answer typed structs.
  - modified: `crates/lys-identity-server/src/connections_api.rs` — Typed ConnectionsView.
  - modified: `crates/lys-identity-server/src/setup.rs` — Typed answer.
  - modified: `crates/lys-identity-server/src/link_audit_api.rs` — Typed answers.
  - modified: `crates/lys-identity-server/src/receipts_api.rs` — Typed answers.
  - modified: `crates/lys-openapi/src/lib.rs` — Route.refusals is a Vec. Registering two different types under one schema name is now a named fault rather than a silent overwrite; the nine such collisions in the crate were renamed.
  - modified: `crates/lys-identity-server/src/openapi.rs` — Each table route gets its types from openapi_types; its refusals are its sets joined, without repeats.
  - created: `crates/lys-identity-server/src/openapi_refusals.rs` — The refusal sets.
  - modified: `crates/lys-identity-server/src/openapi_table.rs` — One line per route through a macro_rules! table, each with every refusal the suite produces on it.
  - modified: `crates/lys-identity-server/src/openapi_typed.rs` — The app routes list the refusals they answer beyond their sets.
  - created: `tests/identity_contract/src/refusals.rs` — Harness middleware: an answer carrying a refusal its route's document entry does not list becomes 500 RefusalUndocumented, naming the route and the refusal.
  - modified: `tests/identity_contract/src/harness.rs` — Serves the service behind that middleware.
  - modified: `crates/lys-identity-server/tests/openapi.rs` — every_route_names_the_types_it_takes_and_answers, with exact lists of the untyped routes; a listed route that is in fact typed fails the test.
  - modified: `crates/lys-identity-server/tests/grants.rs` — Asserts Revoked by name.
  - modified: `crates/lys-identity-server/tests/grant_explanations.rs` — Asserts GrantUnknown and GrantIdMalformed by name.
  - modified: `crates/lys-identity-server/tests/network.rs` — Asserts IdentifierMalformed by name.
  - modified: `crates/lys-identity-server/tests/requests.rs` — Asserts SourceUnknown by name.
  - modified: `crates/lys-identity-server/src/*_api.rs, *_views.rs, *_state.rs` — utoipa::ToSchema derived on each body and answer type a table route reaches.
- Checklist delivery:
  - [x] C367 — One OpenAPI document, generated from the routes and types, describes every route; a route without an entry fails the build (DIRECTORY-048 R6). — One generated document, typed from the routes' types, with refusal lists enforced by tests.
- Story delivery:
  - [x] S151 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As a developer of an app, I want to register my app and its permission schema through a documented API, so that my app's resources and actions are checked by Lys without anyone changing Lys. — The API is published at /openapi.json.

**Review (recorded):**

- Alignment: drifted
- Acceptance verdicts:
  - [x] The document validates as OpenAPI 3.1. — tests/openapi.rs:102. Independently, the generated document was checked with openapi-spec-validator: VALID 3.1.0, 102 paths
  - [x] Adding a route without a description fails the test naming the route. — tests/openapi.rs:155 a_route_added_without_an_entry_fails_naming_the_route; :132 checks both directions
  - [x] Every refusal in the document has a test that produces it. — tests/openapi.rs:236. Each of the 30 named refusals appears in an assertion or refusal context in the test sources (checked here)
- Issues:
  - Each route's refusal list is written by hand in openapi_table.rs and openapi_typed.rs, not generated from the refusal types, and it is incomplete. POST /grants/check, /grants/why and /grants/who leave out NotHeld, which tests/grants.rs:302 produces. Derive each route's refusals from the error types it returns, or list every refusal each route answers, with a test proving the lists complete.
  - Routes that existed before this card are described by words only, with no request or response types, while the spec asks for a document generated from the request, response and refusal types. Give those routes their types.

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
- How: Lys holds no app's name, kind or schema, and calls no app. Measured here this round: fmt --check clean; clippy with all features clean for lys-openapi, lys-identity-server and identity-contract; their tests pass (271 passed, 0 failed); doc builds in both shapes; ast-grep scan exits 0; file-length finds 0 files over across 576.
- Deviation: The design leg failed in round 2 because the workflow writes the dev report into briefs/DIRECTORY-048.json after the answer, and the design leg runs before the docs tree's render leg. The committed .md is therefore stale when the design leg checks it. The workflow has to render before the design leg, or the round has to re-render. The full workspace gate and the lys identity_* leg run on Dean's laptop and need remeasuring by the card round.
- Files changed:
  - created: `rules/ast-grep/no-app-names.yml` — Enforces no product names in code, configuration or screens.
- Checklist delivery:
  - [x] C368 — Lys holds no app's name or schema in code or configuration and makes no call to any app (DIRECTORY-048 R7).
- Story delivery:
  - [x] S151 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As a developer of an app, I want to register my app and its permission schema through a documented API, so that my app's resources and actions are checked by Lys without anyone changing Lys.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] ast-grep scan exits 0 on the tree and fails on a planted product name. — ast-grep scan --config sgconfig.yml exits 0 here, and ast-grep test gives 2 passed. A planted crates/lys-identity-server/src/zz_plant.rs made the scan report '2 error(s) found'; the plant was removed afterwards
  - [ ] The full gate passes with no app registered. — The card round's design leg exited 1: 'rendered markdown differs from the committed file: docs/design/directory/./briefs/DIRECTORY-048.md' (legs.log 3758-4234). On the current tree sh scripts/design/gate.sh exits 0 here. Every other leg exited 0 in the round. The lys identity_* leg was started here; its binaries had passed 3/3 and 4/4 so far when this report was due, and it had not finished
  - [x] A test registers two apps with the same kind name under their own prefixes and checks each separately. — tests/apps.rs:410 two_apps_with_the_same_kind_name_are_checked_each_under_its_own_prefix
- Checklist verified: C368
- Stories verified: S151
- Issues:
  - The card round must remeasure the whole gate at the card's head, including the design leg (now exit 0 locally) and the lys identity_* leg (not finished here), so 'the full gate passes' is a measurement rather than an inference.

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
- How: When SpiceDB is configured, the bench now goes through the same engine the real check uses, in a throwaway scope removed on answering. Verified against a local SpiceDB 1.56.2: the live test passes, the bench test made 12 schema writes and 33 relationship deletes, and nothing is left behind.
- Deviation: (none)
- Files changed:
  - created: `crates/lys-identity-server/src/spicedb_scope.rs` — A scratch scope of SpiceDB, a child module of spicedb: every definition and caveat named under lys/b{hex}/. The one engine call applies the scope. The service's engine never sees scratch names and keeps them when it writes its schema. Also remove_scratch and clear_scratch.
  - created: `crates/lys-identity-server/src/spicedb_scope_tests.rs` — Unit tests: name length, block parsing, each side's view, each side's write keeping the other's names, and a scoped request.
  - modified: `crates/lys-identity-server/src/spicedb.rs` — A scope field and open_scratch.
  - modified: `crates/lys-identity-server/src/apps_bench_scratch.rs` — With SpiceDB configured, the scratch grants run on a scratch scope, examples are placed in the engine, and the scope is removed before the answer.
  - modified: `crates/lys-identity-server/src/apps_bench.rs` — Under SpiceDB, a question is asked while holding the grants' lock, and scopes an earlier question left are cleared first.
  - modified: `crates/lys-identity-server/tests/identity_spicedb.rs` — Live test: the bench scenario on SpiceDB. A planted leftover scope is removed, and no scratch name remains afterwards.
  - created: `crates/lys-identity-server/tests/shared/bench.rs` — The shared bench scenario.
- Checklist delivery:
  - [x] C386 — A person builds an app's whole permission template on the Apps screen from templates, tests it on example people and resources against the real check, and saves it; built and uploaded schemas are the same record (DIRECTORY-048 R8).
- Story delivery:
  - [x] S151 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As a developer of an app, I want to register my app and its permission schema through a documented API, so that my app's resources and actions are checked by Lys without anyone changing Lys.
  - [x] S152 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As an administrator, I want to approve each app and see its schema on a Lys screen before it takes effect, so that no app gives itself power I have not seen.

**Review (recorded):**

- Alignment: drifted
- Acceptance verdicts:
  - [x] A person builds a two-kind schema with a parent from a template on the screen, with no schema text typed, and saving it yields the same record as uploading the same schema through the API. — surface/identity/tests/schema-builder.test.tsx:75 checks that the PUT body equals the hand-written upload; the surface leg exited 0
  - [x] An uploaded schema opens in the builder and edits there. — schema-builder.test.tsx:93
  - [ ] The test bench answers 'may X do Y to Z' against the draft with the path, matching the real check after saving. — tests/apps_schema.rs:381 matches the real check only on the in-process engine. apps_bench_scratch.rs always uses Relationships::Memory, so on a SpiceDB install it skips the engine_permits SpiceDB decision (spicedb_apps.rs translation) that the real check makes
  - [x] A change that would strand grants shows the stranded count before saving and cannot be saved. — schema-builder.test.tsx:144; the server refuses it with schema_change_strands_grants (tests/apps_schema.rs:48)
- Issues:
  - When SpiceDB is configured, load the bench's draft into a throwaway namespace of the permission service (a scratch SpiceDB prefix) and remove it on close, so the bench goes through the same engine the real check uses. Otherwise get a brief revision accepting the in-process engine.

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
