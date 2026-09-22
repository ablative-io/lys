# Identity, access and secrets for people and agents

Tom and Waffles, 22 September 2026, 13:13 to 13:28 Melbourne, in the Dot room. This is what we agreed, in the order we agreed it, and what is built first. It replaces the permission engine part of the 21 September recommendation (Rauthy and OpenFGA); the authentication part of that recommendation is not decided and is not decided here.

## What it is

One identity service for people and agents. It holds five things about every identity: who it is, what it may do, what it holds, what it has done, and where its context lives. Argus keeps context, Cambium keeps talk, aion keeps work, manifold keeps machines. Nothing keeps the agent. This is the missing piece. Tom's words: "as much as all of this is designed to give the agents a home, the agents themselves sort of wind up homeless", and "nothing more central to identity than your own context".

The functions that hang off an identity are the ones a company hangs off a person: commissioning an agent from text, its environment, its learning, its records. Tom's name for it: cyborg relations.

Tom, 14:38 to 14:42: the product is the whole identity, access, permissions and agent lifecycle platform, "beyond just a login page", a full standalone application valuable in its own right. What makes it valuable in its own right is the agent lifecycle, the memory and the secret management together: what tools an agent is provisioned with, what skills and MCP servers, what access to the AI, what goes into its system prompt, what is in its memory, and the context engineering behind all of that. Sign-in is one piece inside it.

## Permissions

SpiceDB is the permission engine. Tom, 13:13: "I'm happy to go with SpiceDB." It is a Go service and cannot be compiled into our Rust binary, so it runs as one process beside the door. Chippy, 14:37: SpiceDB's production storage is an external database and it has no embedded option, so it brings a database with it. With Rauthy (decided 14:35, embedded storage of its own) the installation is 3 processes beside the door: Rauthy, SpiceDB and SpiceDB's database. The earlier line that SpiceDB was the only outside process was wrong. Its revision token is what lets a read be at least as fresh as a named change, which the admission proof needs.

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

The Claude Code account a seat runs on is the one place the credential has to reach the process itself. Tom, 13:32: the seat uses the long-lived OAuth token Claude Code generates, "which is what we do for that", and it works like that. The door puts that token into the seat's environment at spawn, read from the store. This is what the account pool file does today, moved into the store. It rotates at spawn, not per call.

A second way stays open to prove later: the seat holds a handle in the token variable and its base URL points at the proxy, which swaps in the real account per request. That is not tested with a subscription login and nothing depends on it.

### Sealed knowledge

Keys that cannot be rotated, and memories an identity wants kept secret, are sealed records in the same store. The door keeps them encrypted and tags each in SpiceDB with which identities may read it. A seat asks for one by name; the door checks the relation, returns the text, and writes one audit line. A sealed record never sits in plain text in a memory file. Each identity's sealed records are its own.

### What revoking reaches

Chippy, 13:52, in the Dot room: revoking a proxy handle stops future calls, but a login token or sealed knowledge already handed to a process needs a different revocation story. It splits into 3.

1. **A handle.** Dropping it refuses every new call, because the process never had the credential. A call already admitted by the proxy is not stopped by the drop; the proxy has an explicit cancellation rule for calls in flight, and that rule is part of the broker's design.
2. **The login token.** It is in the process. The store records which token went to which seat. The engine that runs the seat ends it on its own. Whether revoking the token at the provider makes the seat's next call fail is proved with the provider before it is promised.
3. **Sealed knowledge.** Once read it is in the process's context. Permission controls the disclosure and the audit line records it; neither takes back what was read. So a key is never read, it is used through the proxy even when it cannot rotate; only memories are read, in the smallest piece asked for.

## Two rules from Tom, 13:55

1. **Manifold is an optional execution engine, never a structural part of this.** "It's in the stack as an optional execution engine, but it's not mandatory... I don't want anybody really planning it as a structural cog." Wherever this statement says a seat is started or ended, the engine that does it is whichever one runs the agent: manifold, aion, or a customer's own.
2. **Every project in the stack works without the others.** Lys, cambium, Argus, haematite, Solon and the rest: "each one of them should be able to function without the other." This identity service standalone is certificates, permissions, secrets and records. Cambium without it signs people in as it does today. An engine without it reads its own pool file as it does today.

## Lifecycle, budgets and leases: the discussion of 14:00

Tom asked at 14:00, not ruling: is this product responsible for agent lifecycle, and where are the lines. Settled 15:01, Tom: "the lifecycle, yes, I agree with everything that's been discussed, so let's move ahead." The three positions below stand together as the design: the record and the decision live here, the runtime carries them out and reports, and Chippy's distinctions hold. Positions as given in the room:

- **Waffles.** Lifecycle belongs here as the record and the decision, never as the process. HR decides who is hired, the role, the access, the budget and the leaving date, and takes the laptop back; IT runs the laptop. So this service owns the agent's record and its states (commissioned, provisioned, active, suspended, ended). The engine asks it "may I start this agent, with what environment and what handles" and reports started and ended as signed events into the log. Ending is the laptop coming back: certificate revoked, every handle dropped, context sealed and archived under the identity. Budgets sit here because the proxy attributes every call to an identity. Everything handed out is a lease: a number of uses, a time window, a spend cap; the time window lives in the signed delegation, uses and spend are counted by the door because a signed object cannot count. Argus manages the live context of a running session; this service keeps the durable context across sessions. People get the same record, engine and leases, and the proxy is where a person's delegated credentials across providers live too. The cure for permission dread is a model that reads as sentences and a screen that answers "why can this identity do this".
- **Buckley.** Identity versus execution: the identity platform answers who this is, what it may hold and for how long; the engine answers where it is running and what it is doing now. Seat spawn and seat end are enforcement points that consult the record, like a door reader checking a badge HR issued. Offboarding is one revoke at the identity side and every runtime enforces it by refusing the next call. Provider secrets are never stored in agents: an agent holds an identity token and exchanges it for a scoped, time-boxed, use-counted lease on the real credential; the same exchange serves people. Onboarding is adding one relation, offboarding is deleting the subject. Caution on forking an identity provider: a fork carries its security patches forever; test first whether an agent can be a first-class principal without one.
- **Chippy.** This product owns the agent's lifecycle: creating its identity, giving it a role and budget, arranging its first session, moving it, suspending it, retiring it while keeping the required history, so creating an agent is a complete product experience. The boundary is deciding what should happen versus carrying it out: this product authorises a session and keeps its record; a runtime starts and stops the process and reports what happened; manifold is one runtime adapter and a local launcher is another. Standalone use means reusable libraries inside each product, never a requirement that another product's server is running; the lifecycle could embed aion's workflow machinery subject to checking that boundary. Memory belongs with the enduring identity, and each session receives the context its assignment permits. Time limits and use counts belong on individual grants; a one-use grant must not be spent twice by simultaneous requests, and a retry has a defined outcome. The screen starts from a person's or agent's job: what they need access to, why, when it ends, what changing it affects, and which changes each service has confirmed. Three distinctions to keep visible (14:31): an identity can have several sessions at once, so active and suspended describe the identity's authority while starting, running and stopped describe each session, and a session is reported stopped only when its runtime confirms it, an unreachable machine staying visibly unconfirmed; the proxy sees only the spending that passes through it, so compute and other direct costs need their own reporting, and a hard cap needs a reservation before work starts and a settlement after, so two concurrent sessions cannot both spend the same remaining allowance; and two sessions writing memories need provenance and a rule for reconciling their changes.

Chippy, 14:42, answering Tom's question of 14:42 on what makes the platform valuable in its own right: an agent becomes something a business can provision, understand and maintain through its life. Creating an accounts assistant means choosing its responsibilities, model access, budget, tools, skills and MCP connections, which instructions it follows, which company knowledge it can read and what memory it carries forward; the product assembles that into a versioned configuration, shows what it means, and arranges a first session through the chosen runtime. All of it is inspectable: open the agent and see its effective instructions, tools, credential handles and context sources; open a session and see the exact versions supplied. A restriction a runtime cannot enforce is visible before provisioning, because a sentence in a system prompt cannot grant or restrict access. Memory has its own interface: where an entry came from, observation or inference, who can see it, how to correct or retire it; session notes never become shared knowledge by default. Context engineering is a product capability: assemble the permitted material for the task, fit it in the window, show what was included, summarised or left out, and evaluate a change against representative tasks before it rolls out. The payoff is continuity with control: changing models, updating a skill or moving machines keeps the agent's identity and history, and configuration changes are reviewable and recoverable. The first complete journey: create one useful agent, run it, inspect its work and context, change its access, retire it cleanly.

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

Chippy, 14:37, having traced the Merkle tree, log, proof and checkpoint code: the primitives are there, and what needs designing is the context record that uses them. When a session starts, the context builder selects the permitted instructions, memories and documents within the model's budget and records their exact versions, order and any summaries; the runtime records the final context it submitted; lys signs those records into the append-only history. That proves what a session was given and where it came from, not that the model understood it or that a summary was accurate. Compaction is a new derived record pointing at its sources, never a rewrite. Private content stays encrypted; an anchor receives a checkpoint, never a transcript. One limit: the current lys log loads every leaf into memory, so it is not an unlimited transcript store until that changes.

Chippy, 14:40: that limit need not block the context record. The encrypted context objects live in durable storage; lys holds small, versioned receipts that refer to them. The whole transcript is never loaded into the Merkle log. Those receipt logs still grow and need a scaling plan.

### The home is portable: lanterns, translation and forks (Tom, 14:45)

Tom, 14:45 to 14:46: the innovative feature to put in here is the work on the Norn memory system and lanterns in the ablative directory; it "changes the perspective on how an agent" exists: "being able to translate between harnesses, to translate sessions, files, so say to move Claude Code to OpenAI ChatGPT. Perhaps more than that, to be able to deploy forks of yourself to different environments... a virtual machine, isolated sandboxes, work in a different node in the network. Really changes what an agent footprint could be."

What is already there, read 14:46. Tom's definition of 8 August 2026, in the Norn memory design: "A memory is a way of storing a note against something, so you can retrieve that note against that thing. A lantern comes with an annotation like the note, but is a pathway back to have a conversation with the previous self through a fork." Lanterns are declared on purpose at moments of completion, success or learning, never automatically; their notes grow over time ("what wound up happening"); vector retrieval is in, so memories aware of time and place make themselves known; time means position in the work's history, never wall-clock age. The lantern tool design (Cally Ray with Tom, 5 September 2026, nothing built) makes one primitive of it: cut a session tree at a coordinate, seed a fork from the cut, run it under a lens on chosen weights with read-only tools, get a structured answer back, write the event on the parent's timeline. Its verbs: ask, sound off and compare, light a lantern, add an epilogue, commune with it, and the one fork with hands, the delegate, which does a piece of work and returns. It measured 3 session formats on 5 September, Claude Code's JSONL (already a tree: every record carries its parent), Norn's session tree and codex, and a codex importer exists; that is dated research, not a proof of cross-harness compatibility today, and the lantern design's own first cut leaves the converter out (Chippy, 15:00). Translating a session between harnesses is designed there and not proved.

How it sits in this platform. The context record above is what makes all of it possible: a session is a signed record of exactly what was given, in what versions and order, held under the identity and not under any harness. So:

- **Translation** is reading the context record out in another harness's shape. The original session stays intact and each translation is a derived version with an explicit account of what it preserved, transformed and could not carry; a common format alone does not guarantee faithful continuation (Chippy, 14:48). What each destination can resume is measured per harness.
- **A fork** is a spawn from a coordinate in the agent's history, presented under the same enduring agent but with its own execution identity and credential linked to the parent (the lantern design already gives the delegate its own key). It carries a delegation from the parent with its own lease (uses, time, spend) and its own handles, so a fork in a sandbox or on another node has exactly the access it was given, and its actions are never indistinguishable from the parent's. Each branch gets its own history; lys records the ancestry and the result references without forking or rewriting an existing log. Ending the fork is the same laptop-back act as ending an agent.
- **A lantern** is a declared coordinate in that record with a note, and communing with it is a read-only fork at that coordinate, briefed on what changed since. Institutional memory becomes something to investigate, with later outcomes attached to the original decision.
- **Recall** is a dedicated read-only fork that searches the agent's permitted history and returns evidence with citations, so the main session never absorbs what the search examined. Replaying an old failure with and without a lesson measures whether the lesson helps.

Norn already has anchored branches, inherited context, permission propagation and cancellation in code; the cross-harness product is work to build. This is direction, designed with Tom before it is built.

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

## The road: settled 15:01

Tom, 14:51: "What does the road map look like and how do we take useful steps along the way without requiring the whole thing to be done first?" Two answers were given in the room. They agree on the order and differ on the cut lines. Each step is installed and shown on a screen before the next, and each stands on its own.

Tom, 15:01: "I agree with everything that's been discussed, so let's move ahead." The 7 steps with Chippy's 4 adjustments are the road. Two lines of work run concurrently: identity, access and the broker (steps 1 to 3) with Chippy, who stays across cambium because the platform integrates with it; memory, context and lanterns (steps 4 and 5) with Archie, whom Tom brought into the room at 15:08 and Waffles and Chippy brought up to speed there. The lifecycle screen (step 7) is shared.

**Waffles, 7 steps.**

1. **Sign-in.** Our Rauthy fork beside the door, cambium signs people in through it, the fork links 2 providers to 1 person. Useful alone: a person with Google and GitHub is 1 identity.
2. **The agent's certificate.** The typed capability claim in the lys certificate, an agent seat gets one at spawn, SpiceDB as the second service, the door asks it on every call. Useful alone: an agent that loses a permission in the middle of a 2 step task is refused on the next call; the screen answers why this identity can do this.
3. **The secrets broker.** Store, handle, proxy, rotation under 1 handle. Useful alone: the pool file is gone, no real key in any seat's environment, every call attributed to an identity.
4. **The flight recorder.** The door and the runtime write session events and audit lines into a lys log stored in haematite, with the context record as receipts. Useful alone: open any session and see what it was given and what it did.
5. **The home.** Context objects under the identity; spawning the agent on a laptop pulls its context down. Then translation, forks and lanterns, each its own step.
6. **Anchoring in production.**
7. **The lifecycle screen**, running through all of them: create an agent, run it, inspect it, change its access, retire it, gaining a piece at each step.

Steps 1 to 3 are the platform without any of the memory work.

**Chippy, 5 releases, each completing a real journey, each with a small brief (screen journey, ownership boundaries, acceptance demonstration).**

1. **A standalone identity and access application.** Sign in, register people and existing agents, assign access, see why it is granted, revoke it, in one connected application: the maintained Rauthy fork, the first product screens, SpiceDB with its durable storage chosen, stable identities connected to lys; linking and migration proved without duplicate people. Value: one place to administer access.
2. **Managed credentials.** One provider, one useful operation: issue a time-limited or use-limited handle, enforce it through the proxy, record the account used, demonstrate revocation and recovery after a crash. Useful to people and existing agents before any agent is launched by us.
3. **Create and operate an agent.** One template, one runtime adapter, one complete path: configure instructions, tools and MCP access, provision, start a session, inspect state and usage, stop or suspend with confirmed outcomes. Manifold optional. Hard spending caps ship only where reservation and enforcement are proved.
4. **Durable memory and context.** Preserve sessions, attach notes to their sources, retrieve permitted material, record what each run received; the existing lys primitives throughout, this release adding the context records and the storage integration. Searchable, correctable memory before automatic recall.
5. **Portability and lanterns.** Prove one harness translation, then a separately authorised working fork into one isolated environment, then a read-only conversation with a past session; widen from working examples.

The two map onto each other: Chippy's release 1 is steps 1 and 2; release 2 is step 3; release 3 is step 7 brought forward; release 4 is steps 4 and 5; release 5 is the tail of step 5. Anchoring (step 6) is in neither release list and sits where Tom puts it.

**Chippy, 14:56, keeping the 7 step sequence with 4 adjustments, all accepted by Waffles:**

1. Step 1 already carries the standalone application's directory of people and agents. Cambium is its first connected customer; someone installs and understands the product without installing cambium.
2. The minimum signed audit record comes forward to the first identity or grant change, in step 1; step 4 grows it into the full session recorder. Who granted access or issued a credential is never reconstructed after the fact.
3. The enduring agent and each execution credential stay distinct from step 2 on: spawning another session never creates another enduring agent, and provisioning an agent is possible before it runs. Step 2's deployment work includes SpiceDB's durable database.
4. Step 3's promise carries the login exception already recorded above: provider credentials used through the proxy stay on the server; the harness login token still reaches its process. The pool file is retired once its consumers have migrated. Waffles' spoken line "no real key in any seat's environment" was wrong against Tom's 13:32 ruling and is corrected here.

Every step leaves Tom with a useful operation he can complete and an honest view of what has happened.

## Not decided

- Decided 14:35, Tom: "knowing that we're gonna use Rauthy". Rauthy authenticates people. Required: Rauthy runs beside the door with SpiceDB as the second service; the one-provider-per-user limit in v0.36.2 ("user is already federated") is changed in a fork of our own that we maintain (Tom, 14:38: "we won't be making a contribution to Rauthy upstream. We can give it a try, but it's very unlikely that it'll be accepted... much more likely that we would need to keep maintaining our own fork"); Rauthy is the login piece inside the platform, and the platform, identity, access, permissions and agent lifecycle, is the standalone product, "beyond just a login page"; the door's Google sign-in becomes sign-in through Rauthy; an agent as a first-class principal in Rauthy is proved before the identity proof leans on it; whether Rauthy's own sign-in pages can carry our styling or our own page drives its API is read from its code before it is promised. Read 14:38 from Rauthy v0.36.2's theme entity: a theme is per client, and it carries 7 colours as HSL for light and for dark (text, text_high, bg, bg_high, action, accent, error), the button text colour, the sun and moon icons and the border radius, built into CSS variables. No font, no layout and no custom CSS. So Rauthy's pages can wear our colours and corner radius; fonts and layout stay Rauthy's. Our own sign-in page in front of Rauthy is not verified as possible and is not promised.
- The name of the identity service.
- Which anchor the first agents pin to: our hosted anchor, a self-hosted one, or both. Lys leaves this as a product decision and so does this statement.
- The design system. Tom, 15:18: "I would like a consistent design across all of them. They don't have that at the moment and it's potentially worth extracting out"; of aion, "probably happiest with the overall appearance... in terms of its form and function in the browser, it's pretty slick", "that's not to say that their code is necessarily the best"; "I don't know if it's worth extracting that into a central design system... I'm not sure"; "I'm not perfectly happy with cambium just yet". So: the standalone identity screens are built to aion's appearance, its colours, type and spacing read from aion's code, with no build dependency on cambium's surface; Rauthy's theme takes the same colours. Whether one design system is extracted for every product is open, a thing to look at, not a row.
