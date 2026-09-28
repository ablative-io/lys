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
- A test with a screens directory sees GET /people answered 200 with the page.
- A test with a screens directory sees GET /assets/missing.js answered 404.

**Files:**
- modify: crates/lys-identity-server/src/surface.rs
- modify: crates/lys-identity-server/src/surface_tests.rs
- modify: crates/lys-identity-server/src/routes.rs

**Checklist:**
- C431 — Every unknown path under /api answers 404 with a named JSON refusal, never the page (DIRECTORY-063 R1).

**Stories:**
- S172 (Installer and operator, Provision and upgrade the internal audit connection) — As the person running a Lys install, I want a missing API to say it is missing and one route that says the service is serving, so that a web page is never mistaken for an answer.

### R2: GET /api/health says the service is serving, with its name and build

Behavioural. A new module crates/lys-identity-server/src/health_api.rs holds one route, GET /health on the API router, merged into it in routes.rs by one line, so it is reached at /api/health when the screens are served and at /health when they are not. It answers 200 with JSON naming the service as lys-identity-server and its build version as the crate version the binary reports for --version. It is answered only by a process that has opened its logs and bound its listener, because the router exists only after both. It needs no session and reveals no configuration, path, key, grant or person. It asks no other service anything, so it never waits on SpiceDB, Rauthy or the database. Those stay with lys identity health.

**Acceptance:**
- A test serving the screens sends GET /api/health with no session and it answers 200.
- In that test the body's service is lys-identity-server.
- In that test the body's version equals env!("CARGO_PKG_VERSION") of lys-identity-server.
- In that test the body has no member other than service and version.
- A test with the SpiceDB endpoint at a listener that accepts and never answers still gets GET /api/health answered 200.

**Files:**
- create: crates/lys-identity-server/src/health_api.rs
- create: crates/lys-identity-server/src/health_api_tests.rs
- modify: crates/lys-identity-server/src/routes.rs
- modify: crates/lys-identity-server/src/lib.rs

**Checklist:**
- C432 — GET /api/health answers that the service is serving, with its name and build, asking no other service (DIRECTORY-063 R2).

**Stories:**
- S172 (Installer and operator, Provision and upgrade the internal audit connection) — As the person running a Lys install, I want a missing API to say it is missing and one route that says the service is serving, so that a web page is never mistaken for an answer.

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
