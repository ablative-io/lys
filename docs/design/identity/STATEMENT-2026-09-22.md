# Identity, access and secrets for people and agents

Tom and Waffles, 22 September 2026, 13:13 to 13:28 Melbourne, in the Dot room. This is what we agreed, in the order we agreed it, and what is built first. It replaces the permission engine part of the 21 September recommendation (Rauthy and OpenFGA); the authentication part of that recommendation is not decided and is not decided here.

## What it is

One identity service for people and agents. It holds five things about every identity: who it is, what it may do, what it holds, what it has done, and where its context lives. Argus keeps context, Cambium keeps talk, aion keeps work, manifold keeps machines. Nothing keeps the agent. This is the missing piece. Tom's words: "as much as all of this is designed to give the agents a home, the agents themselves sort of wind up homeless", and "nothing more central to identity than your own context".

The functions that hang off an identity are the ones a company hangs off a person: commissioning an agent from text, its environment, its learning, its records. Tom's name for it: cyborg relations.

## Permissions

SpiceDB is the permission engine. Tom, 13:13: "I'm happy to go with SpiceDB." It is a Go service and cannot be compiled into our Rust binary, so it runs as one process beside the door. It is the only outside process this design adds. Its revision token is what lets a read be at least as fresh as a named change, which the admission proof needs.

Every check the door makes asks SpiceDB. The browser shows what is permitted; it is never the authority.

## Secrets: the temporary key model

Tom's model, 13:17: "use something with the service, like a temporary key, and that is then mapped so the actual credential can be used." We build this ourselves, in Rust, inside the door. No OpenBao unless we later need credentials minted on demand for a database or a cloud account.

A seat holds a handle: a short-lived value bound to its identity. The real credential sits in the door's encrypted store and never leaves the server. The seat's outbound calls go through the door's proxy. The proxy checks SpiceDB, swaps the handle for the real credential, forwards the call, and writes one audit line: which seat, which handle, which real account, when.

Revoking is dropping the handle. That is instant, and it is instant for a static key too, because the seat never had the key. The real key is rotated upstream only when it is suspected leaked.

### Rotation under one handle

The handle is the stable name. Behind it the store keeps a set of real accounts. The proxy takes the next one in turn for each call and logs which one served it. So usage is attributed in one place, the spread across accounts is enforced at the proxy instead of trusted to each worker, and resting an account is a change in the store with no file copied to any laptop. This replaces the account pool file.

### OAuth

The same shape, and better than a static key. The refresh token sits in the store. The proxy swaps the handle for a live access token and refreshes it itself when it expires. The seat never sees the refresh token. Revoking drops the handle and can also revoke the grant upstream.

### The seat's own login

The Claude Code account a seat runs on is the one place the credential has to reach the process itself. Tom, 13:32: the seat uses the long-lived OAuth token Claude Code generates, "which is what we do for that", and it works like that. The door puts that token into the seat's environment at spawn, read from the store. This is how manifold reads the pool file today, moved into the store. It rotates at spawn, not per call.

A second way stays open to prove later: the seat holds a handle in the token variable and its base URL points at the proxy, which swaps in the real account per request. That is not tested with a subscription login and nothing depends on it.

### Sealed knowledge

Keys that cannot be rotated, and memories an identity wants kept secret, are sealed records in the same store. The door keeps them encrypted and tags each in SpiceDB with which identities may read it. A seat asks for one by name; the door checks the relation, returns the text, and writes one audit line. A sealed record never sits in plain text in a memory file. Each identity's sealed records are its own.

## Where it lives: lys

Tom, 13:32: "the natural home for this should be in lys", and the ledger and the shared chains in it are important. Lys is the trust library and anchor service in the stack (the crates lys-core, lys-anchor, lys-anchor-cli and lys-log-store). Read against this statement, the parts fit together like this:

- **Who the agent is** is a lys certificate. An instance certificate authority issues it when the agent is born, and the certificate carries the agent's capability claims. Rotation is an append to the log, never a replacement, so a rotated key keeps the agent's history (lys decision DP16). The identity proof row's third case is exactly this.
- **What it may do right now** is SpiceDB's answer. Lys does not decide; its own design says so: it certifies claims and proves conduct, and policy belongs to the runtime. The certificate says what an agent was granted; SpiceDB says whether that grant stands this second, and revocation in lys is itself an append with the live set folded from the log (DP26). The two agree because the same grant is the unit in both.
- **What it has done** is the flight recorder. Every session event, every proxy call and every read of a sealed record is signed with the agent's key and appended to a Merkle log. The audit line in the secrets section is one of these entries, not a row in a database.
- **The ledger and the shared chains** are lys-anchor. At each checkpoint the log root, 32 bytes, is anchored to a shared transparency ledger, so from that moment history is fixed in a record the operator does not control. The anchor is itself a lys instance, anchors pin to anchors, and the service is hosted and self-hostable with no hardcoded trust root (DP14, DP19). That is the shared chain agents anchor to, and a counterparty can check an agent's anchored history without our cooperation. Receipts are SCITT standard so verification outlives us.
- **Credential handover and sealed knowledge** are lys sealed envelopes and the delegation format (lys/delegation/v1, a seat as a typed subject). The secrets broker's handle and the sealed record are these, applied.
- **The home** is the agent's context: transcripts, memory and configuration, stored as objects under the identity, so a seat on any device is the same agent and moving devices is a spawn that pulls the context down. The lys roadmap already names this: signed session logs persisted in haematite make sessions portable across nodes. A transcript is full of exactly the things that must stay private, so transcripts are sealed records.

The lys roadmap's own next step is the first consumer integration, and it names the capability gap on the critical path: manifold's compose door carries an unverified claimed seat and lys has no capability type yet. The identity proof row is where that capability type gets built and used.

The home and the anchoring are direction. They are designed with Tom before they are built.

## What exists and what is missing

Tom, 13:41: "I don't think you've looked properly at what's there. Most of it's already there." Read on 22 September, 13:43, from the code in this repository, the cambium door, manifold and haematite.

| Part of the design | State | Where |
| --- | --- | --- |
| The agent's key: Ed25519 identity with X25519 derivation | built and tested | lys-core keys |
| Certificate authority, certificate requests, validity checks, claims carried in extensions under our own OID arc | built and tested; the claim bytes are opaque | lys-core ca |
| Delegation of a seat as a typed subject, version 1, with conformance against the Go implementation | built and tested | lys-core delegation |
| Sealed envelopes and authenticated seal | built and tested | lys-core seal |
| Signed attestations, the Merkle log with inclusion and consistency proofs, signed checkpoints, proof artifacts, receipts, verification bundles | built and tested | lys-core |
| The anchor: genesis, append, submit, checkpoint, open, status, admission policy on certificates, upward pinning of anchors to anchors, a witness | built and tested, not deployed | lys-anchor |
| A log store | built | lys-log-store |
| The verifier's command line | built | lys |
| Every manifold envelope signed with the lys identity | in use | manifold-core |
| Sign-in with Google, account linking, seats, agent seats, sessions, project membership | in use | cambium door |
| An append-only event store | in use | haematite |
| A typed capability claim in the certificate | missing; the claim bytes are opaque and the lys roadmap names this as its own next gap | lys-core ca |
| Revocation as a store, with the live set folded from the log | ruled (DP26), not built | lys |
| The live permission decision (SpiceDB beside the door) | not started | cambium door |
| The secrets broker: the store, the handle, the proxy, rotation under one handle | not started | cambium door |
| A runtime writing its session events into a lys log, and haematite as the store behind it | not started; lys phase 3 | door, manifold, haematite |
| Anchoring in production | not started | lys-anchor |

So the first step joins existing pieces rather than building new ones: the capability claim type in the certificate, and the runtime writing into the log. No step starts before Tom says so. The admission procedure (who may join a place and how) is settled with Tom from Chippy's draft of 22 September. The audit of the existing apps against this design comes after.

## Not decided

- Which service authenticates people (Google and GitHub sign-in to one identity). The identity proof row states the requirement; the engine is chosen after it.
- The name of the identity service.
- Which anchor the first agents pin to: our hosted anchor, a self-hosted one, or both. Lys leaves this as a product decision and so does this statement.
