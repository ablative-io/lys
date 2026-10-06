---
type: design
cluster: access
title: Access across the stack: one authority, one pass, one way to ask
---

# Access across the stack: one authority, one pass, one way to ask

> **Cluster:** access

## Intention

Lys decides who may do what in every product (Cambium, aion, haematite, liminal) with one grant model, one signed pass products verify by themselves, one way to hand an act to a draft, and one client library, so a new product or a new action is data in Lys and never a Lys release, and a person can read who can do what on one screen.

## Problem

Today a grant names one resource and carries no mode; Lys answers questions but issues nothing a product can verify offline; no product enforces Lys grants (Cambium: any signed-in caller invites and a new person sees every channel; aion: namespaces; haematite: a Studio; liminal: wire passes cambium-door mints; Argus: "registration grants no authority"). Four products implementing permissions four ways is the nightmare Tom named on 6 October 2026.

## Solution

The design page docs/design/PERMISSIONS-ACROSS-THE-STACK-2026-10-06.md (decisions D1 to D11): one authority; every product route classified hot (decided from the pass offline) or deliberate (asked of Lys live, never falling back); the pass is the access token with a rights claim for its audience; a grant carries its mode (outright, by_draft, by_two); draft execution is pull; resources are hierarchical placements, never patterns; roles are named bundles in the product schema; a machine is the fifth identity; one client library lys-pass with conformance fixtures; availability stated on each product's screen; every deliberate act receipted in Lys.

## Principles

- **P1** — One authority: no product keeps a role table that is not derived from Lys grants; a product permission model is data in its app schema (ADR-116).
- **P2** — A route is classified once, in a committed matrix, as hot or deliberate; a deliberate route never falls back to the pass when Lys cannot be asked.
- **P3** — A grant says its own mode; "whatever your grant allows without a draft first" (Tom to Hermes, 6 October 2026) is a grant whose mode is outright.
- **P4** — Lys calls no product (ADR-116): drafts are executed by the product reading Lys, never by Lys pushing.
- **P5** — No wildcard enters the permission engine: reach is by placement under a parent, and a restricted child is reached only by an explicit grant.
- **P6** — One library carries the hard parts and the conformance fixtures every product gate runs; a refusal has one shape and names the grant and the Lys page that requests it.
- **P7** — Nothing is derived at a product start from local files; a product with no Lys says so on its screen and refuses by name when the pass lifetime has passed.

## Decisions

- ADR-116 — Every app registers with Lys through one published API; Lys depends on no app — An app is a record in Lys: an id, a name, its sign-in client, and a permission schema it owns (resource kinds under the app's own prefix, each kind's actions, relations carrying actions, and parent kinds whose relations flow down). An app registers and changes its schema only through the API, is approved by an administrator on a Lys screen before it has any effect, and cannot touch another app's kinds. Lys's own model is the schema of the app 'lys'. The API is described by one OpenAPI document generated from the routes and their types, never written by hand. The MCP server is a face over that same API with three tools, the caller's own identity on every call, and no credential or authority of its own. Lys depends on no app. It holds no app's name, kind, schema or code; it never calls an app, waits on one or reads one's store. Every app depends on Lys through this API alone, and Lys runs the same with no apps registered as with twenty.
- ADR-126 — Applications have connector identities; people register them with explicit permission — An application connector is its own third identity and grant-holder kind. Apps do not act as agents and do not use service accounts, which are accounts people use to service things. Only a signed-in person with an explicit ordinary register_app grant registers an app. The super administrator may grant that permission to a person; an agent, connector or service account cannot register, even if presented with such a grant. Approval creates a connector identity and binds the app to it. No app authority is implicit in approval, binding or ownership of a kind prefix: every permission is an explicit grant with a chain tracing to the super administrator. Own-kind checks, schema acts, batch and which use that connector identity and the ordinary grant engine. Extend IdentityId, grant admission and the permission-store holder, plus event and snapshot representation with compatibility tests. Preserve historical person/agent bytes and signatures; version a representation when necessary, never rewrite history or coerce identities. This supersedes DIRECTORY-048 wording allowing a service account or connector holding register_app to register, and forbids a separate registrar credential as substitute authority.
- ADR-135 — An agent Lys starts carries a pass for that run and may be granted any action a person can take, except the responsibilities a person keeps — Every non-public action is a grantable permission. Lys issues a pass per run at spin-up, written with the Lys MCP address into the seat's config, ended when the run ends; every route takes it as the agent and judges each call against live grants. A refusal names who can grant, walking up the chain, and the agent asks them; the answer is once, for a while, ongoing or no. Grants gain one-time and transfer-and-return. A small named set of responsibilities is never passed to an agent.
- ADR-136 — Permissions across the stack: Lys is the one authority; products decide hot routes from a signed pass and ask Lys for deliberate ones; a grant carries its mode — Lys holds every identity and grant; a product permission model is data in its app schema (ADR-116). Every product route is classified once as hot (decided offline from the pass) or deliberate (asked of Lys live, never falling back). The pass is the access token with a rights claim for its audience, short-lived, signed with the key Lys publishes. A grant carries one mode: outright, by_draft or by_two; a held act is a Lys draft the product executes after approval by reading Lys. Reach is by placement, never a pattern; a placement may be restricted. Roles are named bundles in the product schema; a widening by an app waits for the administrator. A machine is the fifth identity. One client library, lys-pass, with conformance fixtures every product gate runs. Availability is stated on each product screen; every deliberate act is receipted in Lys.

## Goals

- A grant carries outright, by_draft or by_two, and Lys refuses a by_draft or by_two grant on an action its schema marks hot.
- The access token for an app carries the holder's rights for that app, signed with the key Lys publishes, short-lived and refreshed.
- lys-pass verifies a pass offline, evaluates reach through placements, asks Lys for deliberate routes, hands by_draft acts to Lys drafts, runs approved drafts, and ships the conformance fixtures.
- An app schema names roles as action bundles and marks placements restricted; a widening of a role waits for the administrator.
- A machine is a grant holder answering to the administrator who issued its join code.

## Non-Goals

- The products' own briefs (Cambium, aion, haematite, liminal). — Each lives in its repository and consumes this cluster; the design page names what each must do.
- Agents started outside Lys obtaining a pass. — ADR-135 leaves them out of scope; whether Cambium's key-enrolled seats become Lys agents is Tom's to say (design page section 7).
- A pattern or wildcard resource. — Reach is by placement (P5); liminal subjects are placed under their parents.

## Structure

| Path | Note | Brief |
|------|------|-------|
| `docs/design/access/` | this cluster | ACCESS-001 |
| `docs/design/PERMISSIONS-ACROSS-THE-STACK-2026-10-06.md` | the design page the cluster hangs off | ACCESS-001 |
| `crates/lys-identity/src/grants/` | grant types, codec, admission, events: mode and role | ACCESS-001 |
| `crates/lys-identity/src/draft_event.rs` | draft records: the held act and its approvals | ACCESS-001 |
| `crates/lys-identity/src/` | identity kinds: the machine | ACCESS-005 |
| `crates/lys-identity/tests/` | compatibility and codec pins | ACCESS-001 |
| `crates/lys-identity-server/src/` | grant, draft, schema, provider and machine routes | ACCESS-001 |
| `crates/lys-identity-server/tests/` | route and provider tests | ACCESS-001 |
| `crates/lys-pass/` | the client library every product consumes, with its conformance fixtures | ACCESS-003 |
| `Cargo.toml` | workspace members | ACCESS-003 |
| `crates/lys/src/identity/install/` | install settings: the pass lifetime | ACCESS-002 |
| `crates/lys/tests/identity_install/` | the real-install tests | ACCESS-002 |
| `surface/identity/src/features/access/` | Access screens: mode, roles, drafts, machines | ACCESS-001 |
| `surface/identity/src/features/apps/` | Apps screen: schema roles and restricted placements | ACCESS-004 |
| `surface/identity/src/features/network/` | Network screen: machines as holders | ACCESS-005 |
| `surface/identity/tests/` | screen tests | ACCESS-001 |
| `docs/design/directory/` | the directory cluster whose grant model this extends; read, not changed here | ACCESS-001 |

## Inventory

- `docs/design/directory/briefs/DIRECTORY-081.json` — the virtual client credential; the sign-in this cluster's pass rides on. Read, never changed.
- `docs/design/directory/briefs/DIRECTORY-080.json` — the app connector identity. Read, never changed.
- `apps/cambium:docs/design/PERMISSIONS-PROPOSAL-2026-10-05.md` — Cambium's proposed roles and actions, the first product schema. Read.

## Constraints

- **CN1** — Nothing written is unreadable or rewritten: grant bodies, events and snapshots gain versions; old bytes stay valid (ADR-126).
- **CN2** — Every brief here is executed under the standing rules: written whole before one battery and one install; commits by pathspec on main; the design gate judged by parsed failures; every handback carries the hospital sentence.
- **CN3** — Lys never calls a product, waits on one, or reads its store (ADR-116).
