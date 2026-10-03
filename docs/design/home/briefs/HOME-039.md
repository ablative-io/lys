---
type: brief
id: HOME-039
cluster: home
title: The door to a haematite store: Lys lists the stores it is configured with and carries one verb to a store's service for an administrator
---

# HOME-039: The door to a haematite store: Lys lists the stores it is configured with and carries one verb to a store's service for an administrator

> **Cluster:** home
> **Depends on:** HOME-001
> **Design anchor:**
> - ADR-136 — The door to a haematite store is Lys: haem serve stays a local socket, and Lys carries each verb for a caller it has admitted — Lys's identity server lists the haematite stores its configuration names (haem_stores: a name and the absolute path of the service's socket) and carries one verb per request to that store's service over the socket, greeting first. Only an administrator is admitted until a grant per store exists. The service's result is answered as it came; its own refusal is HaemRefused, a definite answer; a service that does not answer is HaemUnreachable, never a refusal, so a caller settles a change by its receipt.
> **Checklist:**
> - C199 — GET /haem answers the stores the configuration names, in name order, to an administrator; POST /haem/{store} carries one verb to that store's service over its Unix socket after greeting it with hello on the same connection, and answers the service's result as it came; a caller who is not signed in is refused NotSignedIn and one who is not an administrator NotAdmitted, before the body is read and before the socket is opened (HOME-039 R1).
> - C200 — The service's own refusal is answered HaemRefused (409) carrying its code and words; a socket that cannot be reached, a connection closed before the answer, an answer to another request or a frame that is not JSON is HaemUnreachable (502) and never a refusal; a store the configuration does not name is HaemStoreUnknown (404); a body that is not a method and optional params and no other member is RequestMalformed (HOME-039 R2).
> - C201 — haem_stores in the configuration maps a store's name to the absolute path of its service's socket; a relative path or a name that is empty or holds a slash is refused at start as ConfigInvalid naming the entry; without the setting the door lists no store (HOME-039 R3).
> **Stories:**
> - S80 (Estate operator, Runs Lys behind every agent and session) — As the operator who runs Lys in front of a haematite store, I want the store's screens and agents to reach it only through Lys, with Lys deciding who is admitted, so that a service which asks nobody who is calling is never the thing a browser or an agent talks to.

## Purpose

The haematite Studio's screens call GET /api/haem and POST /api/haem/<store> on the page's own origin, and haem serve asks nobody who is calling. Lys is the door (ADR-136): it knows who is signed in, and until a grant per store exists it admits an administrator and nobody else.

## Task

Add the two routes to lys-identity-server, the configuration that names each store's socket, the exchange with the service over its framing, and the three refusals, each proved by a test against a real Unix socket.

## Requirements

### R1: An administrator lists the stores and has one verb carried to a store's service

Behavioural. A new file crates/lys-identity-server/src/haem_door.rs gives routes() with GET /haem and POST /haem/{store}, merged in routes_table.rs and listed in openapi_table.rs and openapi_types.rs. Both routes take signed_in then administrator before anything else. GET answers {stores: [{name}]} from the state's haem_stores in name order. POST reads {method, params?} with unknown members refused, connects to the store's socket inside spawn_blocking, writes a frame (four bytes of length, most significant first, then JSON) {id: "hello", operation: {method: "hello"}}, reads its answer, then, unless the verb is hello itself, writes {id: "verb", operation: {method, params}} (params left out when the body gave none) and answers the result member of what it reads.

**Acceptance:**
- A signed-in administrator's GET /haem answers the configured names in order, and POST /haem/<store> with branches.list and params answers exactly the result the socket's service wrote; the service read two frames, hello then the verb with its params.
- With no session, GET /haem and POST /haem/<store> answer 401 NotSignedIn and the socket's service has read nothing.
- POST with method hello answers the greeting's result after one frame.

**Files:**
- create: crates/lys-identity-server/src/haem_door.rs
- create: crates/lys-identity-server/tests/haem_door.rs
- modify: crates/lys-identity-server/src/lib.rs
- modify: crates/lys-identity-server/src/routes_table.rs
- modify: crates/lys-identity-server/src/routes.rs
- modify: crates/lys-identity-server/src/routes_startup.rs
- modify: crates/lys-identity-server/src/openapi_table.rs
- modify: crates/lys-identity-server/src/openapi_types.rs

**Checklist:**
- C199 — GET /haem answers the stores the configuration names, in name order, to an administrator; POST /haem/{store} carries one verb to that store's service over its Unix socket after greeting it with hello on the same connection, and answers the service's result as it came; a caller who is not signed in is refused NotSignedIn and one who is not an administrator NotAdmitted, before the body is read and before the socket is opened (HOME-039 R1).

**Stories:**
- S80 (Estate operator, Runs Lys behind every agent and session) — As the operator who runs Lys in front of a haematite store, I want the store's screens and agents to reach it only through Lys, with Lys deciding who is admitted, so that a service which asks nobody who is calling is never the thing a browser or an agent talks to.

### R2: A refusal is definite and silence is not a refusal

Behavioural. A new file crates/lys-identity-server/src/error_haem.rs defines HaemError with StoreUnknown (HaemStoreUnknown, 404), Refused carrying the service's code and message (HaemRefused, 409) and Unreachable carrying the store and what failed (HaemUnreachable, 502), wrapped by ServerError::Haem with its name in error_names.rs and its status in error_status.rs. An answer carrying an error member is Refused. A failed connect, read or write, an answer whose id is not the request's, a frame longer than 64 MiB, a body that is not JSON, or an answer with neither result nor error is Unreachable.

**Acceptance:**
- A verb the socket's service refuses with code not_available answers 409 HaemRefused with reason "HaemRefused: not_available: <its words>".
- A service that closes the connection instead of answering, and a store whose socket does not exist, each answer 502 HaemUnreachable.
- A store the configuration does not name answers 404 HaemStoreUnknown; a body with an extra member, or without method, answers 400 RequestMalformed.

**Files:**
- create: crates/lys-identity-server/src/error_haem.rs
- modify: crates/lys-identity-server/src/error.rs
- modify: crates/lys-identity-server/src/error_names.rs
- modify: crates/lys-identity-server/src/error_status.rs
- modify: crates/lys-identity-server/src/haem_door.rs
- modify: crates/lys-identity-server/tests/haem_door.rs

**Checklist:**
- C200 — The service's own refusal is answered HaemRefused (409) carrying its code and words; a socket that cannot be reached, a connection closed before the answer, an answer to another request or a frame that is not JSON is HaemUnreachable (502) and never a refusal; a store the configuration does not name is HaemStoreUnknown (404); a body that is not a method and optional params and no other member is RequestMalformed (HOME-039 R2).

**Stories:**
- S80 (Estate operator, Runs Lys behind every agent and session) — As the operator who runs Lys in front of a haematite store, I want the store's screens and agents to reach it only through Lys, with Lys deciding who is admitted, so that a service which asks nobody who is calling is never the thing a browser or an agent talks to.

### R3: The configuration names each store by its socket

Behavioural. crates/lys-identity-server/src/config.rs gains haem_stores, a map from a store's name to the path of its service's socket, empty when the setting is absent. Config::validate refuses, as ConfigInvalid naming the entry, a name that is empty or holds a slash and a socket path that is not absolute. The map is copied into AppState at start.

**Acceptance:**
- A configuration whose haem_stores names a relative socket path does not start, and the refusal says "haem_stores: the socket of `<name>` must be an absolute path".
- A configuration without haem_stores starts and GET /haem answers an empty list to an administrator.

**Files:**
- modify: crates/lys-identity-server/src/config.rs
- modify: crates/lys-identity-server/src/routes.rs
- modify: crates/lys-identity-server/src/routes_startup.rs
- modify: crates/lys-identity-server/tests/haem_door.rs

**Checklist:**
- C201 — haem_stores in the configuration maps a store's name to the absolute path of its service's socket; a relative path or a name that is empty or holds a slash is refused at start as ConfigInvalid naming the entry; without the setting the door lists no store (HOME-039 R3).

**Stories:**
- S80 (Estate operator, Runs Lys behind every agent and session) — As the operator who runs Lys in front of a haematite store, I want the store's screens and agents to reach it only through Lys, with Lys deciding who is admitted, so that a service which asks nobody who is calling is never the thing a browser or an agent talks to.

## Boundaries

- haem serve is not changed and still checks nothing: this brief makes Lys the way a browser or agent reaches a store, and does not stop a process on the same machine opening the socket.
- No grant per store, collection or field: an administrator is admitted and nobody else, and the grant work is its own brief.
- The door adds no caller identity to the verb, and records no event of its own for a carried verb.
- No screen of Lys changes; the Studio's own screens live in the haematite repository.

## Verification

- cargo fmt --all exits 0 and git status --porcelain prints the same lines after it as before it.
- cargo clippy --all-targets --all-features -- -D warnings and cargo clippy --all-targets -- -D warnings exit 0, on Dean.
- cargo nextest run -p lys-identity-server --test haem_door passes on Dean with its failures parsed, not its exit read.
- The whole battery (once.sh) passes at the commit, judged by parsed failures.
