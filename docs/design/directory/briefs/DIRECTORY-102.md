---
type: brief
id: DIRECTORY-102
cluster: directory
title: Connecting an app again keeps the running app signed in until it collects its new credentials, Lys hands an app its own public address, reads the app's manifest itself, and knows which apps connected by themselves
---

# DIRECTORY-102: Connecting an app again keeps the running app signed in until it collects its new credentials, Lys hands an app its own public address, reads the app's manifest itself, and knows which apps connected by themselves

> **Cluster:** directory
> **Depends on:** DIRECTORY-059, DIRECTORY-101
> **Design anchor:**
> - ADR-115 — Lys is the only sign-in a person or a product ever sees; the issuer inside it is never shown — Lys is the single sign-on for every product: every product is a client of Lys at Lys's own origin, and no product configuration names the issuer. A person meets only Lys screens: first-run setup, sign-in, provider setup and their own account are Lys pages, and the issuer's pages, admin site, name and password files are never part of any path a person follows. First run asks the person for the administrator's name, email and password; nothing is filled from the machine.
> - ADR-116 — Every app registers with Lys through one published API; Lys depends on no app — An app is a record in Lys: an id, a name, its sign-in client, and a permission schema it owns (resource kinds under the app's own prefix, each kind's actions, relations carrying actions, and parent kinds whose relations flow down). An app registers and changes its schema only through the API, is approved by an administrator on a Lys screen before it has any effect, and cannot touch another app's kinds. Lys's own model is the schema of the app 'lys'. The API is described by one OpenAPI document generated from the routes and their types, never written by hand. The MCP server is a face over that same API with three tools, the caller's own identity on every call, and no credential or authority of its own. Lys depends on no app. It holds no app's name, kind, schema or code; it never calls an app, waits on one or reads one's store. Every app depends on Lys through this API alone, and Lys runs the same with no apps registered as with twenty.
> **Checklist:**
> - C764 — Connecting an approved app again leaves its running credentials live until the exchange hands over the new ones; the revocation happens in the exchange with its reason, and the approve page says so.
> - C765 — GET /apps answers public_address from the server's own dialable address, or null with a reason, and the approve page sends and shows it.
> - C766 — GET /apps/connect/manifest reads an app's manifest server-side for an administrator and answers it or one named reason; the approve page says each reason with its next act.
> - C767 — GET /apps answers connect_origin per app; a self-registered app is listed pending under its own name; the surface holds no app's name.
> **Stories:**
> - S435 (Administrator, Suspends, reinstates and retires identities, and reads why a check refused) — As an administrator who connects Cambium again, the people signed in to it keep working until Cambium collects its new credentials, and nothing is cut off at the moment I press Approve.
> - S436 (Administrator, Suspends, reinstates and retires identities, and reads why a check refused) — As an administrator connecting an app from my laptop, Lys hands the app its own reachable address, and tells me when it has none and what to change.
> - S437 (Administrator, Suspends, reinstates and retires identities, and reads why a check refused) — As an administrator whose approve page could not read the app's manifest, I am told whether the app is not answering, answered with an error, or is not an app, and what to do next.
> - S438 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As the developer of an app, my app registers itself with Lys and appears on Apps under its own name, and Lys never has to know my app's name in advance.

## Purpose

The 18 October 2026 surface review's apps domain (builder 04, lys-review/CONSENSUS.md, Server asks) left four things the screens cannot do because the server does not: its own worst case is that connecting an approved app again revokes the running app's credentials at Approve (apps_connect.rs at ca1228b0: connect, line 275, calls replace_credentials at 353 and then revokes every entry of record.prior at 355), before the app has exchanged its new code, so a reconnect the app never finishes cuts every clinician's sign-in to it until someone finishes the connect on the app's side; the surface now warns before Approve, which is a floor. The approve page hands the app the browser's own origin as Lys's address, so an administrator at localhost hands Cambium an address Cambium cannot dial; the server already computes the dialable address for runners (network_join/routes.rs dial_address). The approve page reads the app's manifest from the browser, so a browser rule (CORS, mixed content) and a dead app look the same and the page can only say it cannot tell which. And Lys's surface carries a list of known app names (ConnectApp.tsx KNOWN_APPS) against ADR-116, because the server does not say which apps connected by themselves or from where.

## Task

In lys-identity-server: move the revocation of an app's prior credentials from connect to the exchange, after the new values are handed over; add public_address and connect_origin to GET /apps; add GET /apps/connect/manifest?address= that reads a manifest server-side for an administrator and answers it or a named reason. In the surface: the approve page sends public_address and shows it, reads the manifest through the server, drops KNOWN_APPS and offers Connect again from each app's own card with its connect_origin, and keys "given its credentials by connecting" on connect_origin. apps_api.rs is 631 lines at ca1228b0; the GET /apps answer builder moves to apps_api_read.rs first, as a move with no change, so each file is under 500 (ADR-111). Out of scope: the app-side registration itself (DIRECTORY-059 R5 specifies POST /apps/connect-requests; this brief only orders it first and names its origin).

## Requirements

### R1: Prior credentials are revoked after the exchange, not at Approve

Behavioural. In crates/lys-identity-server/src/apps_connect.rs, connect no longer revokes record.prior after replace_credentials; the entries stay in record.prior, marked with the operation that replaced them. exchange (line 423 at ca1228b0), after it has handed the new values to the app's server and the hand-over is recorded, revokes every prior credential that operation replaced, with the reason "the app was connected again". A reconnect the app never finishes therefore leaves the running app signed in with its old credentials; a second reconnect before the first exchange replaces the pending values and keeps the same prior set. The approve page's words (surface/identity/src/features/apps/ConnectApp.tsx) change from the warning and "Approve and replace its credentials" to "Approve" with the line "The running app keeps its credentials until it collects the new ones."

**Acceptance:**
- Red first: a test in crates/lys-identity-server/src/apps_connect_reconnect_tests.rs connects an approved app again, reads that the prior credential still authenticates after Approve, exchanges, and reads that it is refused after the exchange with the reason named
- A second connect before any exchange, then one exchange, revokes the first prior set once and leaves nothing live but the newest values
- The approve page's vitest case shows the new words and no warning

**Files:**
- create: crates/lys-identity-server/src/apps_connect_reconnect_tests.rs
- create: crates/lys-identity-server/src/apps_connect_exchange.rs
- create: crates/lys-identity-server/src/apps_client_bearer.rs
- modify: crates/lys-identity-server/src/apps_connect.rs
- modify: crates/lys-identity-server/src/apps_connect_checkpoint.rs
- modify: crates/lys-identity-server/src/apps_client_credentials.rs
- modify: crates/lys-identity-server/src/apps_credentials.rs
- modify: crates/lys-identity-server/tests/apps_connect.rs
- modify: crates/lys-identity-server/tests/apps_connect_retry.rs
- modify: surface/identity/src/features/apps/ConnectApp.tsx
- modify: surface/identity/src/features/apps/connect-again.test.tsx
- modify: surface/identity/src/features/apps/connect-signed-out.test.tsx

**Checklist:**
- C764 — Connecting an approved app again leaves its running credentials live until the exchange hands over the new ones; the revocation happens in the exchange with its reason, and the approve page says so.

**Stories:**
- S435 (Administrator, Suspends, reinstates and retires identities, and reads why a check refused) — As an administrator who connects Cambium again, the people signed in to it keep working until Cambium collects its new credentials, and nothing is cut off at the moment I press Approve.

### R2: GET /apps answers Lys's own public address for an app to dial

Behavioural. GET /apps gains public_address: string | null, computed as network_join/routes.rs dial_address(state.oidc.redirect_url()) computes it for runners, and null with a public_address_reason naming why (loopback, plain http) when that refuses. The GET /apps answer builder moves from apps_api.rs to apps_api_read.rs first, unchanged, so both files are under 500 lines. The approve page (ConnectApp.tsx, KnownAppRow and the connect-again act) sends lys=<public_address> to the app instead of the browser's origin and shows it in its "<app> will call Lys back at" line; when it is null the page says the reason and that the app will be handed the browser's address, and names what to change (the registered address) to fix it.

**Acceptance:**
- Red first: a test in crates/lys-identity-server/src/apps_api_read_tests.rs reads public_address equal to dial_address's answer for a registered https address, and null with a reason for a loopback one
- The approve page's vitest cases show the address sent and shown, and the reason line when null
- apps_api.rs and apps_api_read.rs are each under 500 lines

**Files:**
- create: crates/lys-identity-server/src/apps_api_read.rs
- create: crates/lys-identity-server/src/apps_api_read_tests.rs
- modify: crates/lys-identity-server/src/apps_api.rs
- modify: crates/lys-identity-server/src/apps_api_tests.rs
- modify: crates/lys-identity-server/src/apps_read.rs
- modify: crates/lys-identity-server/src/apps_views.rs
- modify: surface/identity/src/features/apps/ConnectApp.tsx
- modify: surface/identity/src/features/apps/Apps.tsx
- modify: surface/identity/src/features/apps/connect-address.test.tsx
- modify: surface/identity/src/features/apps/connect-another.test.tsx

**Checklist:**
- C765 — GET /apps answers public_address from the server's own dialable address, or null with a reason, and the approve page sends and shows it.

**Stories:**
- S436 (Administrator, Suspends, reinstates and retires identities, and reads why a check refused) — As an administrator connecting an app from my laptop, Lys hands the app its own reachable address, and tells me when it has none and what to change.

### R3: Lys reads an app's manifest itself for the approve page

Behavioural. GET /apps/connect/manifest?address=<manifest url>, administrator only, fetches the manifest from Lys's server and answers it, or one named reason: not answering (with the connection error's words), answered with a status (the status), or not a manifest (what was found). No browser rule stands between the approve page and the app. ConnectApp.tsx reads this route instead of fetch(manifestUrl); its two-cause sentence ("cannot tell whether Cambium is not answering there or does not let this Lys read it") and the Open-the-manifest link go, and each named reason is said as its own line with the next act. surface/identity/tests/apps-connect.test.tsx and src/features/apps/connect-manifest.test.tsx pin the three reasons and the manifest case.

**Acceptance:**
- Red first: a test in crates/lys-identity-server/src/apps_connect_manifest_tests.rs serves a manifest, a 503, and an HTML page from a fixture app and reads the manifest and the two reasons; a non-administrator is refused
- The approve page's vitest cases show the manifest's app name on success and one reason line per failure, with no two-cause sentence left (grep of ConnectApp.tsx for "cannot tell" answers 0)

**Files:**
- create: crates/lys-identity-server/src/apps_connect_manifest.rs
- create: crates/lys-identity-server/src/apps_connect_manifest_tests.rs
- modify: crates/lys-identity-server/src/apps_connect.rs
- modify: crates/lys-identity-server/src/openapi_typed.rs
- modify: crates/lys-identity-server/src/openapi_typed/apps.rs
- modify: surface/identity/src/features/apps/ConnectApp.tsx
- modify: surface/identity/src/features/apps/connect-manifest.test.tsx
- modify: surface/identity/tests/apps-connect.test.tsx

**Checklist:**
- C766 — GET /apps/connect/manifest reads an app's manifest server-side for an administrator and answers it or one named reason; the approve page says each reason with its next act.

**Stories:**
- S437 (Administrator, Suspends, reinstates and retires identities, and reads why a check refused) — As an administrator whose approve page could not read the app's manifest, I am told whether the app is not answering, answered with an error, or is not an app, and what to do next.

### R4: Lys knows which apps connected by themselves and from where; the surface holds no app's name

Behavioural. Each app in GET /apps gains connect_origin: string | null, the origin of the manifest it last connected with, null for an app registered by hand. The app-side registration of DIRECTORY-059 R5 (POST /apps/connect-requests) is the first step of a connect: an app that registered itself is listed on Apps as pending under its own name before anyone types its address. ConnectApp.tsx drops KNOWN_APPS: the Connect card offers "Another app" by address only; each connected app's own card offers Connect again with its connect_origin filled in; and the "given its credentials by connecting" marking keys on connect_origin !== null instead of the name list, so no app name lives in Lys's code (ADR-116).

**Acceptance:**
- Red first: a test in apps_api_read_tests.rs reads connect_origin for an app that connected by manifest and null for one registered by hand
- grep of surface/identity/src for KNOWN_APPS answers 0 after the change, and the Apps vitest cases show a self-registered app pending under its own name and Connect again on a connected app's card

**Files:**
- create: surface/identity/src/features/apps/connect-origin.test.tsx
- create: crates/lys-identity-server/tests/fixtures/apps-line-connected-from.json
- modify: crates/lys-identity-server/src/apps_api_read.rs
- modify: crates/lys-identity-server/src/apps_connect.rs
- modify: crates/lys-identity-server/src/apps_client_lines.rs
- modify: crates/lys-identity-server/src/apps_state.rs
- modify: crates/lys-identity-server/src/apps_fold.rs
- modify: crates/lys-identity-server/src/apps_views.rs
- modify: crates/lys-identity-server/tests/apps_lines.rs
- modify: crates/lys-identity-server/tests/apps_snapshot.rs
- modify: surface/identity/src/features/apps/ConnectApp.tsx
- modify: surface/identity/src/features/apps/Apps.tsx
- modify: surface/identity/src/features/apps/ClientCredentials.tsx
- modify: surface/identity/src/features/apps/credentials-where.test.tsx
- modify: surface/identity/tests/config-secrets-review.test.tsx

**Checklist:**
- C767 — GET /apps answers connect_origin per app; a self-registered app is listed pending under its own name; the surface holds no app's name.

**Stories:**
- S438 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As the developer of an app, my app registers itself with Lys and appears on Apps under its own name, and Lys never has to know my app's name in advance.

## Boundaries

- SHALL NOT revoke any credential before the exchange that replaces it has handed the new values over.
- SHALL NOT keep an app's name in the surface's or the server's code (ADR-116).
- SHALL NOT read a manifest for anyone but an administrator, and SHALL NOT follow a manifest address to a loopback or private address of Lys's own host.
- SHALL NOT put a fixed wait, deadline or poll anywhere, in code or test.
- SHALL NOT leave apps_api.rs over 500 lines: the move to apps_api_read.rs comes first, unchanged.

## Verification

- Red first for every requirement; cargo nextest run only (never cargo test), clippy pedantic -D warnings, cargo fmt; vitest and tsc for the surface files; the design gate's validate.py and check-coverage.py on the cluster parsed for FAIL lines, never by exit code; file length 500 on every file touched.
- Done means: every route and field answers as specified with the named tests green on the Mac, the surface reads each one, the openapi contract test is green, and the lane's END names any file it touched beyond the lists here so the brief is amended in one commit.
