---
type: brief
id: DIRECTORY-063
cluster: directory
title: An unknown API path is refused and the service answers one honest health route
---

# DIRECTORY-063: An unknown API path is refused and the service answers one honest health route

> **Cluster:** directory
> **Design anchor:**
> - ADR-129 — Paths under /api belong to the API and are refused when unknown, and paths outside it stay the page's — The API nested under /api has its own fallback answering 404 with a named JSON refusal. Paths outside /api stay the page's, so /health and /healthz at the root keep answering the page, and the page shows what it shows for a route it does not know. The one health answer is GET /api/health, which names the service and its build and asks no other service. Serving 404 for chosen root names was weighed and not taken, because the server would then guess at the page's routes.
> **Checklist:**
> - C431 — Every unknown path under /api answers 404 with a named JSON refusal, never the page (DIRECTORY-063 R1).
> - C432 — GET /api/health answers that the service is serving, with its name and build, asking no other service (DIRECTORY-063 R2).
> **Stories:**
> - S172 (Installer and operator, Provision and upgrade the internal audit connection) — As the person running a Lys install, I want a missing API to say it is missing and one route that says the service is serving, so that a web page is never mistaken for an answer.

## Purpose

Waffles found on the live install on 29 September that GET /api/health, /health and /healthz each answer 200 with the screens' page. The cause is in crates/lys-identity-server/src/surface.rs. serving() nests the API under /api and merges a screens router whose /{*path} route answers every path without an extension with index.html. An /api path the API router does not know falls through to that route, so a caller of a missing API gets a web page and a 200 and reads it as success. The service also has no route of its own that says it is serving, so a person or a check has nothing honest to ask. Card 8YKXu3wv.

## Task

Refuse every unknown path under /api with a 404 and a named JSON refusal, never the page. Add GET /api/health answering that this service is serving, with its name and build version. Keep paths outside /api as the page's routes.

## Requirements

### R1: An unknown path under /api is refused, never answered with the page

Behavioural. The API router built in routes.rs carries its own fallback, set by one line there, answering 404 with a JSON body whose error names the path as not an API route. When the screens are served, surface::serving nests that router under /api, so a path under /api that no API route matches gets that 404 and never reaches the screens' /{*path} route. When no screens directory is configured, the API router is served alone and answers an unknown path with the same 404. A path outside /api keeps today's answer from surface.rs, the page for a path without an extension and the file or 404 for a path with one. A method the API does not take on a known API path keeps axum's own answer. ADR-129 records why the root paths stay the page's.

**Acceptance:**
- A test serves the API with a screens directory and GET /api/health-unknown answers 404.
- In that test the 404 body is JSON whose error names the path.
- In that test the 404 body does not contain the page's bytes.
- A test serves the API with no screens directory and GET /health-unknown answers 404 with the same JSON error.
- As a guard, a test with a screens directory sees GET /people answered 200 with the page.
- As a guard, a test with a screens directory sees GET /assets/missing.js answered 404.

**Files:**
- modify: crates/lys-identity-server/src/surface.rs
- modify: crates/lys-identity-server/src/surface_tests.rs
- modify: crates/lys-identity-server/src/routes.rs

**Checklist:**
- C431 — Every unknown path under /api answers 404 with a named JSON refusal, never the page (DIRECTORY-063 R1).

**Stories:**
- S172 (Installer and operator, Provision and upgrade the internal audit connection) — As the person running a Lys install, I want a missing API to say it is missing and one route that says the service is serving, so that a web page is never mistaken for an answer.

#### R1 — Execution record

**Dev (recorded):**

- Status: implemented
- How: All rows below are met in code but not yet run: the brief forbids running checks, so each is backed by a test the workflow will run. Row 1 (GET /api/health-unknown answers 404 with screens): surface_tests.rs an_unknown_api_path_is_refused_by_name_never_the_page starts the real service with surface_dir set and asserts status 404. Row 2 (the body is JSON whose error names the path): the same test parses the body as JSON and asserts refusal == "NotAnApiRoute" and error == "/api/health-unknown is not an API route". The handler is surface.rs not_an_api_route, which takes the path from OriginalUri so the /api prefix is kept. Row 3 (the body does not contain the page's bytes): the same test asserts !body.contains(PAGE). Row 4 (no screens directory, GET /health-unknown answers 404 with the same JSON error): without_the_screens_an_unknown_path_is_refused_alike asserts 404, refusal NotAnApiRoute and error "/health-unknown is not an API route". Row 5 (guard: GET /people answers 200 with the page): the first test asserts status 200 and body == PAGE. Row 6 (guard: GET /assets/missing.js answers 404): the first test asserts 404. Extra guards in the same test: /api/authority still answers AUTHORITY, which shows real API routes still reach the router after the switch to nest_service, and root /health still answers the page, as ADR-129 requires. The fallback line is at routes.rs:238. A method the API does not take on a known path still gets axum's own 405 from the API router, because the fallback only fires when no route matches the path.
- Deviation: The spec says surface::serving nests the router under /api. I kept that mount but changed the call from Router::nest to Router::nest_service (surface.rs, serving). The reason is in axum 0.8.9's source (routing/mod.rs, nest, lines 209-231, and call_with_state, lines 417-432): nest moves a nested router's fallback into the outer fallback_router, and that is consulted only after path_router finds no match. The screens' /{*path} route is in path_router, so under nest it would answer /api/health-unknown with the page and the API's fallback would never run. nest_service registers /api, /api/ and /api/{*tail} as path routes that hand the whole request to the API router, whose own fallback then answers. The change stays inside R1's file wall. A second, smaller point: the spec calls the build version 'the crate version the binary reports for --version'. main.rs has no --version handling and is outside the wall, so the health route reports env!("CARGO_PKG_VERSION"), which is what the acceptance row checks, and no --version flag was added.
- Files changed:
  - modified: `crates/lys-identity-server/src/routes.rs` — router() sets the API's fallback with one line, .fallback(crate::surface::not_an_api_route), so every API router, whether nested under /api or served alone, refuses a path it does not know (line 238).
  - modified: `crates/lys-identity-server/src/surface.rs` — serving() mounts the API with nest_service("/api", api), so /api, /api/ and /api/{*tail} all go to the API router and its fallback. The new pub async fn not_an_api_route answers 404 with JSON {refusal: NotAnApiRoute, error: "<original path> is not an API route"}. The module doc states the invariant and cites ADR-129.
  - modified: `crates/lys-identity-server/src/surface_tests.rs` — Adds tests that start the whole service through identity_contract's harness, with and without a screens directory. They cover the unknown-path refusal, a known API route, and guards on /people, /health and /assets/missing.js.
- Checklist delivery:
  - [x] C431 — Every unknown path under /api answers 404 with a named JSON refusal, never the page (DIRECTORY-063 R1). — The API router's fallback answers every unknown /api path with 404 and the NotAnApiRoute JSON refusal, never the page; the surface tests cover this with and without screens.
- Story delivery:
  - [x] S172 (Installer and operator, Provision and upgrade the internal audit connection) — As the person running a Lys install, I want a missing API to say it is missing and one route that says the service is serving, so that a web page is never mistaken for an answer. — The missing-API half of the story: a missing API now says it is missing. The health half is under R2.

### R2: GET /api/health says the service is serving, with its name and build

Behavioural. A new module crates/lys-identity-server/src/health_api.rs holds one route, GET /health on the API router, merged into it in routes.rs by one line, so it is reached at /api/health when the screens are served and at /health when they are not. It answers 200 with JSON naming the service as lys-identity-server and its build version as the crate version the binary reports for --version. It is answered only by a process that has opened its logs and bound its listener, because the router exists only after both. It needs no session and reveals no configuration, path, key, grant or person. It asks no other service anything, so it never waits on SpiceDB, Rauthy or the database. Those stay with lys identity health.

**Acceptance:**
- As a guard, a test serving the screens sends GET /api/health with no session and it answers 200.
- In that test the body's service is lys-identity-server.
- In that test the body's version equals env!("CARGO_PKG_VERSION") of lys-identity-server.
- In that test the body has no member other than service and version.
- As a guard, a test with the SpiceDB endpoint at its own listener, which counts and at once closes each connection it accepts, gets GET /api/health answered 200.
- In that test the listener's count of accepted connections is zero after the answer arrives.

**Files:**
- create: crates/lys-identity-server/src/health_api.rs
- create: crates/lys-identity-server/src/health_api_tests.rs
- modify: crates/lys-identity-server/src/routes.rs
- modify: crates/lys-identity-server/src/lib.rs

**Checklist:**
- C432 — GET /api/health answers that the service is serving, with its name and build, asking no other service (DIRECTORY-063 R2).

**Stories:**
- S172 (Installer and operator, Provision and upgrade the internal audit connection) — As the person running a Lys install, I want a missing API to say it is missing and one route that says the service is serving, so that a web page is never mistaken for an answer.

#### R2 — Execution record

**Dev (recorded):**

- Status: implemented
- How: All rows below are met in code but not yet run, for the same reason as R1. Row 1 (guard: GET /api/health with screens and no session answers 200): health_api_tests.rs the_health_route_names_the_service_and_its_build_and_nothing_else starts the service with surface_dir, calls service.get("/api/health", None) and asserts 200. Row 2 (service is lys-identity-server): the same test asserts body["service"] == "lys-identity-server" and == SERVICE. Row 3 (version equals env!("CARGO_PKG_VERSION")): the same test asserts it; this is a lib unit test, so the value is lys-identity-server's own. Row 4 (no member other than service and version): the same test collects the object's keys and asserts the set is exactly {service, version}. Row 5 (guard: with the SpiceDB endpoint at a counting listener, /api/health answers 200): the_health_route_asks_no_other_service binds a listener that counts each accepted connection, closes it at once and signals on a channel. The SpiceDB settings point at it, with a key file actually written so that a connection attempt could not be stopped short by a missing file. The test first asserts, from the harness's prepare hook, that the service's config really carries that endpoint, then asserts the health route answers 200. Row 6 (the listener's count is zero after the answer arrives): the same test asserts accepted == 0 after the answer. It then connects to the listener itself, waits on the channel and asserts accepted == 1, which proves the counter would have seen a connection from the service. The handler in health_api.rs takes no state and no extractor, so by construction it cannot reach SpiceDB, Rauthy or the directory, and the only way to serve it is from a router built after the logs are opened and the listener bound.
- Deviation: Same --version point as R1: the version is CARGO_PKG_VERSION, and the binary gained no --version flag because main.rs is outside this brief's wall.
- Files changed:
  - created: `crates/lys-identity-server/src/health_api.rs` — Holds one route, GET /health on the API router. It answers {service: "lys-identity-server", version: env!("CARGO_PKG_VERSION")}, needs no session and calls no other service. The module doc states those invariants.
  - created: `crates/lys-identity-server/src/health_api_tests.rs` — Three tests through the whole service: the body's members and values with screens served; no connection to a counting SpiceDB listener, with a control that proves the listener counts; and /health at the root when no screens are served.
  - modified: `crates/lys-identity-server/src/routes.rs` — router() merges crate::health_api::routes() with one line (line 237). That line and the fallback line are the only two code lines added to routes.rs.
  - modified: `crates/lys-identity-server/src/lib.rs` — Declares pub mod health_api.
- Checklist delivery:
  - [x] C432 — GET /api/health answers that the service is serving, with its name and build, asking no other service (DIRECTORY-063 R2). — GET /api/health answers 200 with only service and version, and a counting listener at the SpiceDB endpoint sees no connection from it.
- Story delivery:
  - [x] S172 (Installer and operator, Provision and upgrade the internal audit connection) — As the person running a Lys install, I want a missing API to say it is missing and one route that says the service is serving, so that a web page is never mistaken for an answer. — Together with R1: a missing API says it is missing, and /api/health is the one honest answer that the service is serving.

## Boundaries

- SHALL NOT answer any path under /api with the page.
- SHALL NOT change the answer of any path outside /api.
- SHALL NOT make GET /api/health call SpiceDB, Rauthy, the database or any other service.
- SHALL NOT add a timeout, deadline, sleep, poll or watchdog.
- SHALL NOT add more than two code lines to crates/lys-identity-server/src/routes.rs.
- SHALL NOT add unsafe, #[allow], #[ignore], let _ = or a renamed _name binding.

## Verification

- The full Lys gate and ast-grep scan exit 0 at the card's head, measured by the card round.
- sh scripts/file-length.sh exits 0 at the card's head.
- `cargo test -p lys-identity-server --lib surface` exits 0 on Dean's laptop at the card's head.
- `cargo test -p lys-identity-server --lib health_api` exits 0 on Dean's laptop at the card's head.
