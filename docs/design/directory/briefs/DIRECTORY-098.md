---
type: brief
id: DIRECTORY-098
cluster: directory
title: An app declares its own sign-in return address with its own credential, so a console at a new name signs in without an administrator's hand
---

# DIRECTORY-098: An app declares its own sign-in return address with its own credential, so a console at a new name signs in without an administrator's hand

> **Cluster:** directory
> **Depends on:** DIRECTORY-097
> **Design anchor:**
> - ADR-115 — Lys is the only sign-in a person or a product ever sees; the issuer inside it is never shown — Lys is the single sign-on for every product: every product is a client of Lys at Lys's own origin, and no product configuration names the issuer. A person meets only Lys screens: first-run setup, sign-in, provider setup and their own account are Lys pages, and the issuer's pages, admin site, name and password files are never part of any path a person follows. First run asks the person for the administrator's name, email and password; nothing is filled from the machine.
> - ADR-116 — Every app registers with Lys through one published API; Lys depends on no app — An app is a record in Lys: an id, a name, its sign-in client, and a permission schema it owns (resource kinds under the app's own prefix, each kind's actions, relations carrying actions, and parent kinds whose relations flow down). An app registers and changes its schema only through the API, is approved by an administrator on a Lys screen before it has any effect, and cannot touch another app's kinds. Lys's own model is the schema of the app 'lys'. The API is described by one OpenAPI document generated from the routes and their types, never written by hand. The MCP server is a face over that same API with three tools, the caller's own identity on every call, and no credential or authority of its own. Lys depends on no app. It holds no app's name, kind, schema or code; it never calls an app, waits on one or reads one's store. Every app depends on Lys through this API alone, and Lys runs the same with no apps registered as with twenty.
> **Checklist:**
> - C743 — POST /apps/{app}/sign_in/returns adds one return address for the administrator or the app itself acting through its own credential, judged by DIRECTORY-097's rule, written nowhere when already listed, refused NotAdmitted to anyone else.
> - C744 — The Sign-in settings show a declared address with who declared it and when, and the administrator's save of the whole list still removes it.
> - C745 — The CHANGELOG and the apps skill's route notes record the route, its callers and its refusals.
> **Stories:**
> - S422 (Person or agent using Lys through an AI assistant, Reads and changes Lys through an MCP client, with only their own rights) — As the lead of a product whose console moves to a new name, I want the console to declare its own sign-in return address with the credential it already holds, so that people sign in there the day it moves and nobody edits Lys by hand.

## Purpose

On 17 and 18 October 2026 every sign-in to liminal's console from any device was refused redirect_invalid for a day: the console's canonical address changed (CON-014, then CON-015 to https://liminal.ablative.com.au) and the liminal app's Sign-in settings still listed only http://127.0.0.1:5178/signin/return. The list is the administrator's alone (apps_sign_in::set, apps_binding::Acting::administrator: "only the administrator does this, on a Lys screen"), Tom could not reach a screen, and nobody signs in as him; Waffles, 18 October 08:4x: "a product fix, not a paste". Lys already lets an app act as itself through its own credential (apps_binding::acting with the lys-app bearer, whose digest is the approved client's secret_sha256, Acting::App); liminal's exchange holds that line today (app_credential_path), and its console server, which holds the issuer's OAuth client secret (a different secret), is given the same bearer line by CON-016. What is missing is one act: the app declaring, with its own credential, the address its sign-in returns to. DIRECTORY-097's rules on what a return address may be stand unchanged.

## Task

Add one route by which an app, acting through its own sign-in client credential, or the administrator, declares one sign-in return address for that app: judged by DIRECTORY-097's rule, added to the list if new, written nowhere if already listed, recorded in the app's log as the caller's act (R1); show a declared address on the Sign-in settings with who declared it and when, the administrator's full-list save keeping the power to drop it (R2); record the route in the CHANGELOG and the apps skill's route notes (R3). Out of scope: an app removing an address, any credential but the app's own, post-logout redirects, DIRECTORY-097's rules.

## Requirements

### R1: An app declares one return address with its own credential

Behavioural. POST /apps/{app}/sign_in/returns takes {"address": "<absolute address>"} under an operation id as every app act does. It is admitted to the signed-in administrator and to the app itself: a caller whose apps_binding::acting is Acting::App with app equal to {app}. A person, the registrar, and another app's credential are refused NotAdmitted with the reason "only the administrator or the app itself declares the app's return addresses"; a pending, declined or retired app's credential is refused as apps_binding::app_acting refuses it today. The address is judged by apps_redirect::admitted, so every DIRECTORY-097 refusal and reason applies (redirect_invalid, http off this machine, a fragment, a malformed wildcard). An address already listed byte for byte answers 200 with the settings unchanged and writes nothing. Otherwise the app's log gains one line, the declaration, by the caller (By::ServiceAccount naming the app for the app's own act, the administrator's login for theirs), and the sign-in settings read back with the address appended and every other setting (the person's name, the other addresses) as it was. The administrator's POST /apps/{app}/sign_in keeps setting the whole list and may drop a declared address. The handler and its route registration live in their own module, apps_sign_in_declare.rs (`routes()`), merged in routes_table.rs beside apps_api::routes(), so apps_api.rs gains no line.

**Acceptance:**
- Red first in apps_sign_in_declare_tests.rs: the app's own lys-app credential declares https://console.example/signin/return and the settings read back with it appended, the log's last line by the app's service account; the same declaration again answers 200 and the log's length is unchanged; another approved app's credential, a signed-in person and the registrar's credential each answer NotAdmitted with the reason; http://console.example/signin/return answers redirect_invalid with DIRECTORY-097's reason; a retired app's credential answers app_retired
- Red first in tests/provider.rs: after the app declares an address, /oauth/authorize with that redirect_uri answers 303 to the issuer and the token exchange completes, as DIRECTORY-097 R2 pins for a listed entry
- The route is in openapi_typed/apps.rs with its request and response schemas and the three refusal names, and GET /openapi.json carries it
- apps_api.rs measures 424 lines of code by scripts/check_file_length.py at c95be049 (limit 500) and gains no line in this brief; apps_sign_in.rs (75) and the new module each stay under the gate's 500

**Files:**
- create: crates/lys-identity-server/src/apps_sign_in_declare.rs
- create: crates/lys-identity-server/src/apps_sign_in_declare_tests.rs
- modify: crates/lys-identity-server/src/lib.rs
- modify: crates/lys-identity-server/src/openapi_typed/apps.rs
- modify: crates/lys-identity-server/tests/provider.rs
- modify: crates/lys-identity-server/src/routes_table.rs

**Checklist:**
- C743 — POST /apps/{app}/sign_in/returns adds one return address for the administrator or the app itself acting through its own credential, judged by DIRECTORY-097's rule, written nowhere when already listed, refused NotAdmitted to anyone else.

**Stories:**
- S422 (Person or agent using Lys through an AI assistant, Reads and changes Lys through an MCP client, with only their own rights) — As the lead of a product whose console moves to a new name, I want the console to declare its own sign-in return address with the credential it already holds, so that people sign in there the day it moves and nobody edits Lys by hand.

### R2: The Sign-in settings show who declared an address and when

Behavioural. On the app's Sign-in settings, an address added by a declaration is shown with the line "declared by the app, <date and time>" or "declared by <administrator's login>, <date and time>" under it, read from the app's log; an address the administrator saved in the list carries no such line. Saving the list without a declared address removes it, as today, and the help page for the Sign-in settings says that an app may declare its own return address with its credential and that the list saved here is the whole list.

**Acceptance:**
- Red first in surface/identity/tests/apps-sign-in.test.tsx: a settings answer carrying a declared address renders the line with the app's name and time; an administrator-saved address renders none; saving without the declared address posts the list without it
- The help page's text is pinned in the surface test with the two sentences

**Files:**
- modify: surface/identity/src/features/apps/Apps.tsx
- modify: surface/identity/src/shell/help/pages.ts
- modify: surface/identity/tests/apps-sign-in.test.tsx

**Checklist:**
- C744 — The Sign-in settings show a declared address with who declared it and when, and the administrator's save of the whole list still removes it.

**Stories:**
- S422 (Person or agent using Lys through an AI assistant, Reads and changes Lys through an MCP client, with only their own rights) — As the lead of a product whose console moves to a new name, I want the console to declare its own sign-in return address with the credential it already holds, so that people sign in there the day it moves and nobody edits Lys by hand.

### R3: The route is recorded

Documentary. CHANGELOG names the route, who may call it and that an already-listed address writes nothing; plugin/skills/apps/references/routes.md and plugins/lys/skills/apps/references/routes.md say at /apps/{app}/sign_in/returns how an app declares its return address with its lys-app credential and that the administrator's /apps/{app}/sign_in still sets the whole list. The liminal console server's declaration at boot is liminal's brief (CON-016), told to its lead the hour this one lands.

**Acceptance:**
- The CHANGELOG entry and both routes.md files name the route, the two admitted callers and the three refusals
- cargo nextest run -p lys-identity-server green; cargo clippy --workspace --all-targets -- -D warnings; cargo fmt --all

**Files:**
- modify: CHANGELOG.md
- modify: plugin/skills/apps/references/routes.md
- modify: plugins/lys/skills/apps/references/routes.md

**Checklist:**
- C745 — The CHANGELOG and the apps skill's route notes record the route, its callers and its refusals.

**Stories:**
- S422 (Person or agent using Lys through an AI assistant, Reads and changes Lys through an MCP client, with only their own rights) — As the lead of a product whose console moves to a new name, I want the console to declare its own sign-in return address with the credential it already holds, so that people sign in there the day it moves and nobody edits Lys by hand.

## Boundaries

- SHALL NOT admit any credential but the app's own sign-in client credential (the lys-app scheme) and the administrator's session: no registrar, no person, no other app.
- SHALL NOT let an app remove an address or set the whole list; removal stays the administrator's save.
- SHALL NOT change what a return address may be: apps_redirect::admitted judges a declaration exactly as it judges a saved line.
- SHALL NOT keep or answer a secret: the app log keeps the credential's SHA-256 as today, and the declaration line carries the address and the caller only.
- SHALL NOT fetch anything from the app: a declared address is kept to be sent back to, never visited.

## Verification

- Every test red first; cargo nextest run only; clippy -D warnings, cargo fmt --all, the surface's bun test; the design gate (validate.py and check-coverage.py on docs/design/directory) parsed for FAIL lines.
- Done means: the six refusal and admission pins of R1 green, the provider end-to-end pin green, the settings rendering pinned, the CHANGELOG and routes notes written, and liminal's lead told the route is live.
