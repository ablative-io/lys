---
type: brief
id: DIRECTORY-085
cluster: directory
title: MCP tokens bound to registered resources, introspection, and every read recorded
---

# DIRECTORY-085: MCP tokens bound to registered resources, introspection, and every read recorded

> **Cluster:** directory
> **Depends on:** DIRECTORY-049
> **Design anchor:**
> - ADR-116 — Every app registers with Lys through one published API; Lys depends on no app — An app is a record in Lys: an id, a name, its sign-in client, and a permission schema it owns (resource kinds under the app's own prefix, each kind's actions, relations carrying actions, and parent kinds whose relations flow down). An app registers and changes its schema only through the API, is approved by an administrator on a Lys screen before it has any effect, and cannot touch another app's kinds. Lys's own model is the schema of the app 'lys'. The API is described by one OpenAPI document generated from the routes and their types, never written by hand. The MCP server is a face over that same API with three tools, the caller's own identity on every call, and no credential or authority of its own. Lys depends on no app. It holds no app's name, kind, schema or code; it never calls an app, waits on one or reads one's store. Every app depends on Lys through this API alone, and Lys runs the same with no apps registered as with twenty.
> **Checklist:**
> - C515 — A directory administrator registers a resource server by URI and public key, and Lys's own door is one (DIRECTORY-085 R1).
> - C516 — Every MCP token names one resource, and /mcp refuses another's as token_wrong_audience (DIRECTORY-085 R2).
> - C517 — A registered resource introspects a token and learns only about its own (DIRECTORY-085 R3).
> - C518 — Every read through /mcp is kept as a leaf before it is answered (DIRECTORY-085 R4).
> - C519 — A registered app exercises a grant for the subject a token names, and the Use is recorded with its action (DIRECTORY-085 R5).
> **Stories:**
> - S269 (Person or agent using Lys through an AI assistant, Reads and changes Lys through an MCP client, with only their own rights) — As a person whose records an assistant reads through Lys, I want every token to work only where it was meant and every read to be recorded, so that I can see who read what and nothing reaches a server it was not meant for.

## Purpose

Lys is the authorization server for MCP. Its tokens name no resource, so a resource server other than Lys's own door has no way to tell a token meant for it from one meant for another, and DIRECTORY-049 R2's token_wrong_audience was written and never built. Its /mcp relay records only changes, so reads of records leave no trace. Percy's read of the handle path found both (Waffles, 7 October, bcfad2cc).

## Task

Register resource servers (R1); bind every token to one resource and refuse it elsewhere (R2); answer introspection to registered resources (R3); record every read (R4); let a registered app exercise a grant for the subject a token names, recorded (R5). Out of scope: signed (JWT) access tokens; dynamic registration of resources; the screens' look.

## Requirements

### R1: An administrator registers a resource server, and Lys's own door is one

Behavioural. A registered resource is a canonical resource URI as RFC 8707 section 2 defines it (absolute, no fragment) and the Ed25519 public key, in hex, that it signs its introspection requests with. Only a directory administrator registers or removes one, through the published API, and each act is a leaf of the directory log. Lys's own /mcp door is registered as itself at start, with no key, because it reads its own store. A second registration of the same URI is refused naming it. Nothing is kept for a resource but its URI, its key and who registered it; no shared secret exists. The key may be a Key entry sealed in the secrets broker, with every signature made through /_lys/sign/{secret} under the resource's handle, so its private half never leaves the broker. The administrator registers the URI and that key's public half once. A resource whose own process restarts with a fresh in-memory handle keeps signing with the same sealed key, so no administrator act is needed at a restart.

**Acceptance:**
- An administrator registers a resource; a non-administrator is refused; both are on the log.
- Registering a URI with a fragment, or one already registered, is refused by name.
- A resource registered with a broker-sealed key's public half introspects with a signature made through /_lys/sign, and still does after its process restarts with a new handle, with no second registration.

**Files:**
- create: crates/lys-identity-server/src/mcp_resources.rs
- create: crates/lys-identity-server/src/mcp_resources_tests.rs
- modify: crates/lys-identity-server/src/mcp_oauth.rs

**Checklist:**
- C515 — A directory administrator registers a resource server by URI and public key, and Lys's own door is one (DIRECTORY-085 R1).

**Stories:**
- S269 (Person or agent using Lys through an AI assistant, Reads and changes Lys through an MCP client, with only their own rights) — As a person whose records an assistant reads through Lys, I want every token to work only where it was meant and every read to be recorded, so that I can see who read what and nothing reaches a server it was not meant for.

### R2: Every token names its resource, and a token for another resource is refused

Behavioural. The authorization request and the token request take the RFC 8707 resource parameter, and it is required: an absent or unregistered resource is refused invalid_target (RFC 8707 section 2). Today authorize refuses any resource but the door's own (mcp_oauth_flow.rs 112-118), and a token carries no audience (Issued, mcp_oauth_store.rs 33-38), so /mcp's bearer check accepts any live access token (mcp_oauth.rs 187-207). Issued gains the audience, a refresh keeps it and cannot change it, and the store holds tokens as digests as now. The stored form is replaced, not read beside the old one: a token issued before this lands is refused, and its app connects again. /mcp refuses an access token whose audience is not its own resource as token_wrong_audience, answering 401 with WWW-Authenticate error="invalid_token". That is DIRECTORY-049 R2's refusal, written and never built. No token is passed through to any resource.

**Acceptance:**
- Red at main: authorize naming a second registered resource is refused, so no token for it can exist. At the head a token for it is issued and /mcp refuses it as token_wrong_audience.
- A token request with no resource, or an unregistered one, is refused invalid_target.
- A refresh answers a token with the same audience, and a refresh naming another resource is refused.

**Files:**
- modify: crates/lys-identity-server/src/mcp_oauth.rs
- modify: crates/lys-identity-server/src/mcp_oauth_flow.rs
- modify: crates/lys-identity-server/src/mcp_oauth_store.rs
- modify: crates/lys-identity-server/src/mcp_oauth_store_tests.rs
- modify: crates/lys-identity-server/src/mcp_oauth_tests.rs

**Checklist:**
- C516 — Every MCP token names one resource, and /mcp refuses another's as token_wrong_audience (DIRECTORY-085 R2).

**Stories:**
- S269 (Person or agent using Lys through an AI assistant, Reads and changes Lys through an MCP client, with only their own rights) — As a person whose records an assistant reads through Lys, I want every token to work only where it was meant and every read to be recorded, so that I can see who read what and nothing reaches a server it was not meant for.

### R3: A registered resource asks Lys whether a token is live for it

Behavioural. Introspection follows RFC 7662 at POST /oauth/introspect, and the protected-resource and authorization-server documents name it. Lys's tokens are opaque and kept as digests, so introspection is chosen over signed tokens: there is no signing key to hold or rotate, and a revocation takes effect at the next call. The caller is admitted only by the header lys-resource-signature, of four words separated by single spaces: the registered resource URI, the signing time in milliseconds since the Unix epoch, a nonce of at least sixteen bytes in lowercase hex, and a COSE_Sign1 Ed25519 signature in lowercase hex. The signed payload is these eight lines joined by a single newline, with no trailing newline: the domain lys-identity/resource-introspect/v1; Lys's own public origin, as its protected-resource document names it; the resource URI; the method; the path with its query; the SHA-256 of the body in lowercase hex; the signing time; and the nonce. The signature verifies against the key registered for that URI, and against no other key. The signing time is within a minute of Lys's clock and not earlier than the time this process began admitting. The nonce was not seen within that minute, in a nonce set of its own that is kept apart from the agent signature's. The domain is distinct from every lys-identity/agent-request version, so an agent signature can never pass as an introspection signature or the reverse, whatever DIRECTORY-086 does to the agent payload. Each failure is refused resource_signature_refused, naming the check that refused it and never the key tried. A token whose audience is that resource answers active true with sub, client_id, aud, exp and scope. Any other token, live or not, answers active false, as RFC 7662 section 2.2 says, so a resource learns nothing about another's tokens. The token appears in no log, audit line or error's words.

**Acceptance:**
- A resource introspecting its own live token gets active true and its claims; another resource's token, an expired token or a revoked one gets active false.
- An unsigned request, or one signed by an unregistered key, is refused.
- An agent-request signature placed in lys-resource-signature is refused, and so is a resource signature placed in lys-agent-signature.

**Files:**
- create: crates/lys-identity-server/src/mcp_introspect.rs
- create: crates/lys-identity-server/src/mcp_introspect_tests.rs
- modify: crates/lys-identity-server/src/mcp_oauth.rs

**Checklist:**
- C517 — A registered resource introspects a token and learns only about its own (DIRECTORY-085 R3).

**Stories:**
- S269 (Person or agent using Lys through an AI assistant, Reads and changes Lys through an MCP client, with only their own rights) — As a person whose records an assistant reads through Lys, I want every token to work only where it was meant and every read to be recorded, so that I can see who read what and nothing reaches a server it was not meant for.

### R4: A read through /mcp is recorded before it is answered

Behavioural. Today the /mcp relay keeps a leaf only for a changing method (mcp_receipts.rs 322-325, used at mcp_endpoint.rs 455), so a read leaves no record. For anything that holds records, who read what is the record that matters. Every relayed call is now kept as a leaf of the directory log. A change is kept before it is made, as now. A read is kept before its answer is released, and a read whose leaf cannot be kept answers nothing but the refusal. A read's leaf carries the agent, the method, the recorded path (its query only as a SHA-256, as recorded_path does) and the kind read. It never carries the answer's body. The leaf is one durable write in the existing Kept path, with no second store and no per-leaf flush beyond the one that path already makes.

**Acceptance:**
- Red at main: a GET relayed through /mcp leaves no leaf. At the head it leaves one naming the agent and the path.
- With the leaf write failing (a test-only seam on the real write path), the read answers the refusal and no body.
- No read leaf contains any byte of the answer's body.

**Files:**
- modify: crates/lys-identity-server/src/mcp_receipts.rs
- modify: crates/lys-identity-server/src/mcp_receipts_tests.rs
- modify: crates/lys-identity-server/src/mcp_endpoint.rs
- modify: crates/lys-identity-server/src/mcp_endpoint_tests.rs

**Checklist:**
- C518 — Every read through /mcp is kept as a leaf before it is answered (DIRECTORY-085 R4).

**Stories:**
- S269 (Person or agent using Lys through an AI assistant, Reads and changes Lys through an MCP client, with only their own rights) — As a person whose records an assistant reads through Lys, I want every token to work only where it was meant and every read to be recorded, so that I can see who read what and nothing reaches a server it was not meant for.

### R5: A registered app exercises a grant for the subject a token names, and the use is recorded

Behavioural. POST /grants/exercise is open to a registered app through its credential, for its own kinds only. Another app's kind is refused not_your_app, as grants_batch.rs refuses it. It takes the subject, the resource (kind and id) and the action. The subject is the one R3's introspection answered for a token whose audience is that app's registered resource. A subject with no live token for that resource is refused subject_not_presented, so an app cannot act for anyone it was not handed a token by. The grant is checked as an Exercise, the same decision route_actions.rs makes, and the Use is recorded with the action named. The request body is JSON with exactly these members, and any other is refused, as CheckWire refuses one (grants_batch.rs 37-47): {"subject": "<identity id>", "resource": {"kind": "<kind>", "id": "<id>"}, "action": "<action>", "at_least": <revision, optional>}. at_least means what it means for the batch. The Use is recorded as a new grant event kind, GrantChange::Exercise {grant, route, action, app}, with the subject as its holder, route Api and app the calling app's id. The existing Use kind and every stored signed event are unchanged, because they are records. A new kind is added for this, and no field is added to Use. The answer is 200 with exactly {"subject": "<identity id>", "grant": "<grant id>", "use_index": <u64>}, where use_index is the ledger index that record_use returns for that event (lys-identity/src/grants/commit.rs 221-227). A refusal answers the identity server's one refusal body, {"refusal": "<name>", "reason": "<words>", "fields": [...]} (error_status.rs 112-115), with the status that refusal already has. That is the broker's {refusal, reason} with fields added, and is not a second shape. The names are not_your_app, subject_not_presented, NotHeld, Revoked and StaleDecision. A refusal records nothing and answers its reason by name. Today /grants/check judges only the caller, and /grants/check/batch takes a subject but records nothing (grants_batch.rs 12 and 190), so no app can get a recorded Use for a subject.

**Acceptance:**
- Red at main: there is no route by which an app gets a recorded Use for a subject. At the head an exercise answers the subject and a use index, and the Use names the action.
- Another app's kind is refused not_your_app; a subject without a live token for the app's resource is refused subject_not_presented; neither records a Use.
- A subject whose grant is revoked is refused Revoked, and nothing is recorded.
- The answer's members are exactly subject, grant and use_index, and use_index reads back the Exercise event naming the action and the app.

**Files:**
- create: crates/lys-identity-server/src/grants_exercise.rs
- create: crates/lys-identity-server/src/grants_exercise_tests.rs
- modify: crates/lys-identity-server/src/grants.rs
- modify: crates/lys-identity-server/src/openapi_table.rs
- modify: crates/lys-identity/src/grants/commit.rs
- modify: crates/lys-identity/src/grants/events.rs

**Checklist:**
- C519 — A registered app exercises a grant for the subject a token names, and the Use is recorded with its action (DIRECTORY-085 R5).

**Stories:**
- S269 (Person or agent using Lys through an AI assistant, Reads and changes Lys through an MCP client, with only their own rights) — As a person whose records an assistant reads through Lys, I want every token to work only where it was meant and every read to be recorded, so that I can see who read what and nothing reaches a server it was not meant for.

## Boundaries

- SHALL NOT accept a token at a resource other than its audience, or pass a token through to any resource.
- SHALL NOT keep a shared secret for a resource; its key's public half is all Lys holds.
- SHALL NOT answer a read whose leaf did not land.
- SHALL NOT read a token in the old stored form beside the new one.
- SHALL NOT add a timeout, deadline, sleep, poll interval, #[allow], #[ignore], unsafe code or any bypass.

## Verification

- The design gate, parsed: every cluster clean, no FAIL, every file valid.
- Each requirement's red at main quoted in the handback.
- The piece's one Lys battery on the Mac, judged by parsed output: whole-crate nextest, cargo test --doc, ast-grep, file length, fmt and clippy pedantic in both configurations, 0 failed.
