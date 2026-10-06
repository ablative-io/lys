---
type: design
cluster: directory
title: The standalone identity directory, with every grant rooted in a person
---

# The standalone identity directory, with every grant rooted in a person

> **Cluster:** directory

## Intention

An operator installs the identity product without Cambium or Manifold, signs in, links Google and GitHub to one person, registers an agent under a responsible person, and inspects the signed history of every identity change. Cambium later uses this issuer, keeping its participant ids (row 06); that direction is recorded here while row 06 stays the non-goal recorded below, and this sentence does not promise it.

## Problem

IDENTITY-001 revision 5 is the reviewed plan for this, in the older row form, and it predates Tom's ruling of 22 September 17:15 that every grant is pegged to a human authority, his PostgreSQL ruling of 23 September 14:14, and the working lifecycle states. Its row 02 installs SpiceDB without saying what it enforces. It cannot be dispatched to the design-system loop as it stands.

## Solution

Carry IDENTITY-001's open rows (02, 04, 03, 05) into design-system briefs in this cluster, DIRECTORY-002 to DIRECTORY-005, revised for the grant ruling (ADR-003), the PostgreSQL ruling (ADR-005) and the working lifecycle states (ADR-011, proposed), with the fork (ADR-009) and the product accents (ADR-010) in the project ledger and every decision still open for Tom marked open. The IDENTITY-001 files stay as they are, as the record of revision 5.

## Principles

- **P1** — An enduring identity ID is stable through provider additions, key rotation and later sessions; issuer plus subject identifies an external login; email and display name never establish identity equivalence.
- **P2** — Everything is pegged to a human authority: the responsible person's permissions are the ceiling and an agent holds an explicit subset; exercising an act and delegating it are separate grants; withdrawing the authority stops every grant derived from it.
- **P3** — A person or agent may be registered before any session exists; registration creates no running state and issues no Rauthy login, runtime credential or capability certificate.
- **P4** — One signed committed directory event is both the identity change and its audit record; never a database change followed by a best-effort log append.
- **P5** — No success before durable evidence; an uncertain append is reconciled before its projection answers as current, and pending or refused outcomes stay visible until resolved.
- **P6** — A future execution or fork ID refers to its enduring identity and, for a fork, its parent execution; step 1 reserves that distinction in the contract and implements neither session history nor launch.
- **P7** — An audit receipt carries a version, a stable operation ID, the actor, the affected identity, the operation, a payload commitment and the resulting log coordinate or checkpoint; secrets and whole context objects are excluded, and its exact signed encoding is reviewed before use.
- **P8** — The service attests the authenticated human actor and their authentication provenance; it never claims a person signed bytes with a key they do not hold, and a registration records the person who made it, never an invented agent signature.
- **P9** — The initial directory administrator is bootstrapped by an explicitly configured issuer and subject, never by email or first visitor; every other mutation caller is refused in step 1.

## Decisions

- ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
- ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
- ADR-005 — The identity database is PostgreSQL, possibly on a network device — PostgreSQL is used for the identity product's database. It may be set up on one of the network devices rather than on the operator's workstation.
- ADR-007 — The product starts an agent by giving its start command, never by running it — The product is not an execution engine. For the first release an agent's file gives the command that starts it on a chosen machine: the command carries the agent's identity and its handles, never a credential's value, and it is rendered from the agent's kept launch record. A started agent reports back, so the sessions screen shows what is running. A terminal inside the product, sandboxes, virtual machines and containers are later runtimes that plug in, and none is built into this product.
- ADR-008 — An agent's file shows its lys certificate — An agent's file shows its lys certificate once one is issued: what it claims, who signed it, when it was issued and when it expires, with the signed receipts of the changes made to it. An agent registered before any key or proof of possession was supplied shows its certificate as not issued, never a placeholder.
- ADR-009 — People sign in through a maintained Rauthy fork of our own — Rauthy authenticates people, and its one-provider-per-user limit is changed in a fork we maintain, ablative-io/rauthy, not contributed upstream as a prerequisite. The maintained branch is ablative, created from upstream v0.36.2 commit dd61ac3c84d6b238108dc8438b53043b5177a662; the fork's main stays an untouched upstream mirror; lys pins an exact commit of ablative as the submodule vendor/rauthy. Upgrades rebase ablative onto upstream release tags only, each in its own gated row; no cherry-picks and no reset of main.
- ADR-010 — Every product shares one design and keeps its own accent; the identity product's is orange — The identity screens follow Aion's structure, typography, spacing and interaction, and Rauthy's client themes take the same colours, with no build dependency on Cambium or Aion. Each product keeps its own accent within the estate colour family: Cambium green, Aion blue and black, Argus light blue, Haematite mustard. The identity product's accent is orange (accent #D4975A, deep #A86B2E, wash #3D2A17 in the estate colour tokens), set apart from Manifold's copper. No product is silently made Aion-blue, and purple is not used.
- ADR-011 — An identity is registered, active, suspended or retired — An identity is in one of four states: registered (exists in the directory, no grants, no credential handle, may not act), active (may act within its grants), suspended (kept whole, grants kept but not effective) and retired (permanent, history kept, never reactivated; a new identity is made instead). Register, activate, suspend, reinstate and retire are the only transitions, each one signed audit record naming the authenticated actor and their provenance, the identity, from, to, when and reason. Having a grant or a credential is a fact beside the state, not a state. A person is registered by first sign-in; an agent is registered by a signed-in person, who carries it as its responsible person for life and may cause every transition of their own agents. Source: docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:17-44 and docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:71-95.
- ADR-024 — Audit receipts are for changes only; answered permission checks go to step 2's separate access record — Receipts are for changes only. A check that is answered and changes nothing is not a receipt. When step 2 brings enforcement, allows and refusals go into a separate access record that step 2 defines: refusals are always recorded there, and whether allows are too is decided with step 2. Rejected: every answered check as a signed receipt in the lys log, and deferring the question to the build.
- ADR-025 — The receipt verifier lives in the lys CLI, offline, needing only the log and the service's public key — The verifier lives in the lys CLI, as lys log verify receipt, offline, needing only the log directory and the service's public key. Changing the published lys binary for this is accepted; the release that carries it is a separate, deliberate act after docs/design/identity/IDENTITY-EVENTS.md's envelope version is ratified and recorded, never before, because publishing a crate that exposes a format freezes it (CLAUDE.md:78-80). Rejected: a verifier inside the directory service, which would be checking the service with the service and defeats the point of an independent audit.
- ADR-026 — A development install emits test receipts under a test key and a test tag, and a real key never verifies one — A development install emits test receipts, signed with a test key and carrying a test tag, inside IDENTITY-001's development isolation, and never performs the reserved act. The verifier refuses a test-tagged receipt by name unless it is given the test key explicitly, and a real key never verifies a test receipt, so a test receipt never passes as real. Harness tests stay as well. Rejected: a development install that emits no receipts, and reading its receipts as the reserved act.
- ADR-021 — The Rauthy client themes are dark only from the estate tokens, and every field the estate names no token for is a named gap — Both client themes are dark only. A Rauthy field takes an estate token only where the estate names the same role: the dark text takes text, bg takes ink, bg_high takes raised and accent takes the client's product accent. Every other field is a gap that keeps Rauthy's own default: light mode, the error colour, the radius, and the dark text_high, action, btn_text, theme_sun and theme_moon, eight in all, listed by name in deploy/identity/theme-map.md. Rejected: mapping text to muted, which would set the body of every page in the secondary colour; and choosing colours for the fields the estate names no token for.
- ADR-027 — An identity's lifecycle state is folded from its signed transition records and never set directly — The state at any moment is the fold of the identity's transition records in log order, from registered. No API, migration or repair sets a state; a transition validates against the folded state and the cause rule, appends its record through the directory's event path, and the state is read again by folding. A read answers the fold or refuses by name at the coordinate of a record the transition table refuses, and never answers a state past it, repairs the log or skips the record. Rejected: a stored state column set at transition and audited after it; a repair path that writes a state; and treating provisioned as a fifth state rather than a view over the facts beside the state.
- ADR-028 — A suspension takes effect at the next check, a session refresh is a check, and no Rauthy session is revoked at once — A suspension or retirement takes effect at the next check and not before. The directory's resolution of a Rauthy-authenticated session at sign-in and at refresh is a check, so a session that outlives a suspension is refused at its next refresh and at every access check before that, by state, whatever grants it holds. Nothing in DIRECTORY-009 calls Rauthy to revoke, end or invalidate a session, keeps a list of sessions to end, or rolls back an admitted action. An at-once revocation of the Rauthy session would be a later revision of the suspend transition, and whether suspending also ends the sign-in session at Rauthy stays open as the design records it. Rejected: revoking the session at Rauthy at once within this brief, and treating a refresh as something other than a check.
- ADR-039 — A certificate carrying one claim is revoked whole, by its issuing authority's key, permanently, by a leaf folded in lys-identity — A certificate carries one capability claim, so the revocable unit and the claim unit are the same; revoking a claim is revoking its certificate and issuing a new one without it, and the fold never treats a certificate carrying several claims as partially revoked. Only the issuing authority's key signs a revocation leaf; a person responsible for the agent under ADR-003, or an administrator, revokes by asking the directory, which asks the authority to append, and the fold refuses a revocation signed by any other key. A revocation is permanent: no leaf reinstates a certificate, the fold refuses a reinstatement leaf by name, a verbatim replay of an earlier issuance leaf included, and the way back is a new certificate. A revocation always names a certificate the log already holds: the fold refuses a revocation leaf with no earlier issuance leaf for its certificate as revocation_before_issuance, and a later issuance of that certificate is valid. A revoked certificate's inclusion and consistency proofs and its issuance record always verify, and an attestation by its key verifies if and only if its own log entry precedes the revocation leaf, the log order being the evidence of time; an attestation with no entry before the revocation leaf fails, naming the revocation leaf; an attestation by a certificate that is not revoked needs no log entry of its own. The fold and the revocation-aware verification live in lys-identity; lys-core keeps revocation a consumer-side non-goal, and its published verify_certificate_chain and the lys ca verify command keep their meaning and keep passing with no log, while a new library form taking the log, the size N and the tolerance as explicit inputs checks revocation; its command line is a later, separate unpublished binary, and the published lys binary does not depend on lys-identity. Certificate revocation and grant revocation are separate folds: this stands beside DIRECTORY-006 R4 and does not discharge it, and C25 stays open with DP26 as the proposal its review reads. Rejected: revoking claims one by one inside a certificate, letting any key but the issuing authority's sign a revocation, a later leaf reinstating a certificate, letting every attestation made before a revocation stand whatever its log position, a revocation standing for a certificate the log does not hold, building the fold in lys-core, and changing what the published no-log verification means.
- ADR-056 — Needs a new person is a view worked out on read from an agent's registration and its person's retire record, never stored or appended — An agent's needs-a-new-person flag is worked out each time its lifecycle record is read, from the responsible person its registration names and that person's retire record: it is true when that person's folded state is retired and false otherwise, a suspended person included. Nothing is appended to any agent's history, no transition is made on the agent, the agent keeps its own state, and no responsible person changes. The flag records a fact, that the agent's person is retired; clearing it needs the reassignment card, which is the one that revises ADR-011's responsible person for life. Rejected: a signed flag record appended on each agent when its person is retired; moving the agent to another state with its person.
- ADR-001 — Secrets are held behind a handle the door swaps for the credential — A seat holds a short-lived handle bound to its identity. The real credential sits in the door's encrypted store and never leaves the server. The door's proxy checks SpiceDB, swaps the handle for the credential, forwards the call and writes one audit line. Built in Rust inside the door; no OpenBao unless credentials minted on demand are later needed.
- ADR-019 — The secrets broker lives in lys, as crates/lys-secrets; Cambium consumes it and is never its home (amends ADR-001) — The broker lives in the lys repository as the crate crates/lys-secrets, part of the standalone identity platform, built in Rust. A person installs Lys to hold secrets; Cambium reaches the broker through the door as a consumer and is never its home. Rejected: the broker inside the cambium door that ADR-001 and SECRETS-002 name, which would make Cambium a runtime a person must install to keep a credential.
- ADR-079 — A session credential is a random value the directory issues when the agent's signed report-back over a directory-issued challenge creates the session, and keeps only as a hash — The session credential is 32 random bytes the directory issues when the session is created. The directory keeps only their SHA-256, on the session record, and returns the value once, in the answer to the report-back that created the session; the session presents the value to the directory, which is the only party that checks it. A session is created when the started agent first reports back and proves it is the agent with its enrolled key, and only for an agent whose certificate has been issued. Before reporting back, the agent asks the directory for a challenge: 32 random bytes the directory issues once, for that agent and a launch record id the agent supplies, kept until used or until it expires more than 60 seconds after issue. The proof is an Ed25519 signature under the tag lys/session-start/v1 (ADR-080). The directory verifies the signature before anything else: an unsigned or badly signed proof, or one naming an agent it does not hold or one with no enrolled key, is refused proof_invalid with one body and touches no challenge, and only a correctly signed proof receives the state-revealing refusals. It admits the proof only when the challenge is one it issued to that agent, unused and unexpired, the directory named is itself and the launch record id matches the challenge's, and it marks the challenge used in the same act that creates the session. Two acts end a session: the agent reporting its end with its credential, and the directory's administrator stopping it. Ending changes the record's state, so the credential is refused at once, naming the ended session. Rejected: a per-session lys certificate, because nobody verifies a session credential offline in this card, so a signed artifact buys nothing and would freeze a format before anything needs it; a lys/delegation/v1 SpeaksFor delegation, framed for an offline root key and a seat subject, whose module says not to invent a consumer; an ADR-001 handle, which waits on the unbuilt secrets broker (SECRETS-002); creating the session when a start command is rendered; and expiry of a session.
- ADR-080 — lys/session-start/v1 signs five fields, the tag, the agent id, the directory's identifier, the launch record id and a directory-issued nonce, and is proposed in the wire-format register until ratified — The report-back is an Ed25519 signature by the agent's enrolled key over the 20 ASCII bytes of lys/session-start/v1, one 0x00 byte, then the agent directory id, the directory's own identifier and the launch record id, each as its UTF-8 length in 4 bytes big-endian followed by its bytes, then the 32-byte nonce, a challenge the directory issued once to that agent for that launch record id and admits for 60 seconds after issue. Freshness (the nonce), audience (the directory's identifier) and the launch record id are in the first version because the tag cannot gain them after it freezes. The format is proposed in docs/design/WIRE-FORMATS.md's decision log; the tests sign under it with disposable test keys only, and no production agent signs under it until the register shows it ratified. Rejected: a four-field message without the directory's identifier, which could not say which directory a proof was made for; a message without the launch record id, which could not tie a session to the command it was started by; a signature over the fields without a directory-issued nonce, which a replay would satisfy; and treating this ledger entry as the ratification, which the register reserves to its own decision log.
- ADR-077 — A holding with an end date moves at its next renewal by default; one without moves only by a deliberate act — A move at the next renewal is the default for a holding with an end date, shown on the holder as the date it will move, which is the holding's end date, with the words that its next renewal moves it to the current version and who can stop the move. A renewal, which the roles brief introduces, is a deliberate recorded act by the person the holder answers to (the responsible person of ADR-011) or by an owner of the role's project: a new grant of the holding with a new end date, recorded naming the actor, the holding it renews and the version it lands on. So the move rides on a deliberate recorded act and no holder ever moves without one; unrenewed, the holding lapses at its end date under ADR-006 and does not move. A holding with no end date has no renewal, so whatever the role's default it moves only by a deliberate act, and the screen says so on that holder. A role carries a default policy, and a newly made role's default is a move at the next renewal; a holding with an end date takes that default at grant time, and the person the holder answers to or an owner of the role's project may change the policy of one holding afterwards, recorded with who did it and in which capacity. The holding's own policy governs it. Under a move only by a deliberate act a renewal is made at the version the holding holds; under a move at the next renewal it is made at the role's current version. Changing a role's default applies to holdings granted after the change and moves nothing that exists. Rejected: holders staying on their version until moved as the default for a holding with an end date, because the renewal is a deliberate recorded act the move can ride on; and a policy set only per role, because one holding could not then be kept apart.
- ADR-089 — A role is a set of grant templates copied at grant time, not a live group — A role is a template copied at grant time, not a live group. A holding names the role and the version it holds, the grants it carries are made from that version's templates when the holding is granted, and an edit to the role changes none of them. Who holds a role remains answerable as a query over holdings. Only the versioned grant templates make a version; a title edit is recorded and makes no version, and conformance 4.1's job and profile stay proposed and outside. Rejected: a live group, whose membership carries the role's current grants, because a live group would change every holder's permissions at the moment of the edit, which ADR-006 forbids.
- ADR-078 — Permission enforcement through SpiceDB on the standalone identity server enters the directory design, on a decided grant representation — Amend the design narrowly: permission enforcement through SpiceDB on the standalone identity server leaves the 'Road step 2 onward' non-goal and becomes a goal of the directory design, and CN11 gains one appended ruling line saying so. Everything else in the non-goal stays, and neither the non-goal's text nor CN11's is reworded; the non-goal's reason gains one appended sentence saying it does not cover that enforcement. The grant representation (C5) is a technical decision of the owning lead with a second reader, not one left open for Tom, and is decided here: every grant, root or delegated, is its own grant object in SpiceDB, keyed by its grant identifier, and is exactly 3 relationships: the resource's relationship to the grant object under the grant's action; the grant's holder relationship to the identity that holds it, carrying an expiry caveat that ends no later than its source's; and the grant's standing relationship, which for a delegated grant is its source relationship to its source grant, the source relation the permission walks, and for a root grant, which has no source, is its root relationship, the grant object's relationship to the directory's root authority, one object per directory and the same object the issue_root permission is checked against when a person issues a root grant through POST /grants/roots; a permission holds through a grant only while the grant's standing holds, a root grant's through its root relationship and a delegated grant's through its source grant's standing, so derivation is a walk of the source relation that ends at the directory's root authority in a bounded number of steps, a grant that stands on neither a source relationship nor a root relationship is malformed and refused, and deleting a grant's standing relationship refuses its holder and every grant derived from it at once; two live grants giving the same holder the same action on the same resource are two grant objects, each with its own relationships, and revoking one leaves the other standing; withdrawing a grant deletes its relationships and those of every grant derived from it through the source relation on the same committed revoke event, the withdrawn grant's standing relationship alone in the first write and the rest in further writes within SpiceDB's configured max_updates_per_write. The expiry bound is refused, never clamped: a delegation asking for an expiry later than its source grant's is refused by name before anything is committed, and a committed delegation event that nonetheless asks for one is refused by the projector by name, which writes no relationship for it and stops there; a silent clamp would make the record say one thing and the grant another. Rejected: sitting beside the non-goal as a named exception, which leaves the design's stated scope contradicting the brief; rewording the non-goal or CN11; and bringing Cambium's door into this cluster.
- ADR-086 — Every command given keeps its own signed launch record, and running is tied to it through lys/session-start/v1 — Every command given keeps one launch record, a signed directory event naming the machine, the executable, the working directory, the profile version and the credential ids. Its id is carried in lys/session-start/v1, whose signed message then holds five things: the tag, the agent's directory id, the directory's identifier, the launch record id and the directory-issued challenge. The agent reads the id from LYS_LAUNCH_RECORD and its report names it, and running is tied to the command given through that id. A start given again copies a kept record into a new record with its own id that names the record it was copied from. Rejected: tying running to the agent id alone, re-issuing one record for every command given, and keeping the launch facts only in the command line.
- ADR-087 — The executable, its arguments and the working directory are fields of the reviewed profile version, never of a start request — The executable, the arguments and the working directory are read from the reviewed profile version's record, as the mock-up draws them, and a start request names only the agent, the profile version and the machine. Rejected: letting a start request set either, and taking either from the machine or a default.
- ADR-088 — Withdrawing a start request records only that the request no longer stands — A start request can be withdrawn by the person who gave it or anyone holding the same right. The withdrawal records that the request no longer stands, with who and when, and never that the agent did not start. If a signed report later arrives for the withdrawn launch record, the session shows as running with the withdrawal beside it. Rejected: recording a withdrawal as not started, and refusing a report that arrives after a withdrawal.
- ADR-091 — One standing start per agent, and the command given as ids-only environment assignments before the recorded executable — While a start for an agent stands unconfirmed, a second start for that agent, on any machine, is refused as start_unconfirmed, naming the standing request and the act that answers it: wait for its report, or withdraw it first. The given command reads env LYS_AGENT_ID=<agent id> LYS_LAUNCH_RECORD=<launch record id> LYS_CREDENTIAL_IDS=<comma-separated credential ids> <executable> <recorded arguments>, with the working directory set to the recorded one. The three values are ids only, each shell-quoted and refused by name if it holds anything outside its id grammar. Rejected: appending the ids to the recorded arguments, and giving a second command while one stands unconfirmed.
- ADR-092 — Row 07 is carried in the directory cluster up to a staged install, with its Cambium parts on Cambium-owned cards and the production acts left to Tom — Row 07 leaves the non-goal and is carried by a directory brief that stops at a staged install on the node the operator names, keyed with a test service key, where standalone acceptance is proved and recorded before any cutover. The Cambium install document lands through its own Cambium card with its own pull request and gate, and row 06's Cambium half stays on its own Cambium card; this brief names both as blockers and lists no Cambium file. The brief registers exactly one release leg, as a demand-cadence leg in docs/design/project.json, and every other row registers the leg it needs in its own brief. The production signing key, production receipts and the Cambium cutover are Tom's three acts. Rejected: leaving row 07 a non-goal beside a brief that carries it, listing the Cambium install document on this brief's wall, adding the release leg to .land/gates.sh where every land would run it, and standing up a staging Cambium inside this row.
- ADR-093 — A Claude Code import builds its session under a staging name and publishes it once — The import command builds its new session under the staging name `<id>.jsonl.importing` beside `sessions/<id>.jsonl`, holding `<id>.lock` throughout: the header and every entry line are written to the staging file with no sync, and the index rows and the head are kept in memory. It then publishes once, in this order: one sync of the staging file; the index written whole to `<id>.index.jsonl` through a temporary file, synced and renamed; the head written once to `<id>.head` through a temporary file, synced and renamed; one sync of the sessions directory; the staging file renamed to `<id>.jsonl`; one more sync of the sessions directory. A crash before the rename leaves no session under the final name, so a re-import is not refused as Exists; the next import of that id and the next open of that session file, each holding the lock, remove the staging file and, when `<id>.jsonl` is absent, the `<id>.index.jsonl` and `<id>.head` a part-way publish left. Rejected: appending in place and leaving a session that refuses by name after a crash; extending reconcile to trim an unsynced tail.
- ADR-094 — An agent certificate's validity window comes from the directory's configuration: 30 days by default, 90 days at most — The directory's configuration holds a default validity window and a maximum, shipped as 30 days (2592000 seconds) and 90 days (7776000 seconds). An issuance call with no window takes the default; the operator may ask for a shorter window, never a longer one, and a request above the maximum is refused by name, lifetime_above_maximum, naming the maximum. The certificate records the window it was given as its own notBefore and notAfter. A verifier takes a set of trusted issuer keys, never exactly one, and the certificate names the issuer key that signed it in X.509's Authority Key Identifier by its issuer-key fingerprint, the first 20 bytes of the SHA-256 of that key's SubjectPublicKeyInfo (RFC 7093 method 1, matching the SubjectKeyIdentifier of lys-core's issuer certificate), so a later rotation of the issuer key needs no change of format. Rejected: an operator-supplied lifetime with no default and no bound, which lets a typo outlive a withdrawn grant; a window fixed in code with no configuration; and a verifier bound to exactly one issuer key, which would make every rotation a format change.
- ADR-081 — Issuance enters the lys-log-store log `lys log` keeps before any certificate is written, and the issuer holds only the CA key — `lys ca issue` enters every certificate it produces, on both issuance paths, in the lys-log-store log that `lys log` keeps, as one leaf whose bytes are the certificate's DER and nothing else, before any certificate file is written; if the log refuses the entry no certificate is written. The issuer holds only the CA key: issuance appends the leaf, writes the certificate and the leaf file, and reports the log and the leaf index, and the log's operator makes the lys/log-inclusion-proof/v1 artifact with `lys log prove inclusion`, which holds the log's key. Rejected: giving `lys` a dependency on lys-anchor, which would stop `lys` being publishable and reverse BUILD-PLAN section 2.5; putting issue-and-enter in the lys-anchor binary instead of `lys ca issue`; and requiring the issuer to hold the log's key to write the artifact at issuance. The issuer-only entry is the required path, and a production issuer never holds the log's key. `--log-key` and `--artifact-out` stay on `lys ca issue` only as an optional path for one operator who holds both keys, as tests and proofs with test keys do; given one without the other, issuance refuses.
- ADR-098 — The access graph draws Access's answers and the directory's records, and holds no permission rule — Grant edges and every reachability the graph draws come only from Access answers returned by the directory, paths exactly as returned and a no as Access's reason. Choosing a person or agent reads the grants it holds that the caller may see and asks the forward question for each resource and action they carry; choosing a resource asks the reverse question for each action the resource declares. A grant is drawn as the dashed does not stand edge only when a forward refusal names it, with Access's reason; a grant Access no longer returns disappears on the next draw; the graph never infers that a grant does not stand. A resource's parent and an agent's responsible person are recorded facts drawn as recorded: parents from the permission model through a read-only route that records nothing, serves each resource's declared actions and the model version it read them under, and shows a resource only where the caller holds a grant on it or an ancestor, or is a directory administrator, who sees every resource; a hidden parent is served as withheld, drawn as a mark with no containment edge and never named; responsible people come from the directory's records. The graph holds no rule of its own and asks Access only on navigation, reload or its refresh control. Rejected: porting the mock-up's raw-grant edges, its standing reckoning and its reach(); amending Access to enumerate reach or list grants that do not stand; a new directory record for resources; serving a resource with a hidden parent as a root; and dropping the containment and answers-to edges because they are not Access answers.
- ADR-104 — The shell and its mock-up change together, and a corrected mock-up is a new version beside the prior — A change to the shell's behaviour is made in the built shell and in the mock-up in the same change, and the corrected mock-up is written as a new file, index.v6.html, beside the prior, the way v4 was kept beside v5. CONFORMANCE.md's reference line moves to the new file in that change and carries its path and sha256; DIRECTORY-006 pins no file and no hash, so DIRECTORY-037 is the first brief to pin one, and DIRECTORY-006 does not change. Rejected: fixing only the build, which lets the definition drift from what is built; and editing index.v5.html in place, which changes the file the conformance table was written against.
- ADR-105 — Sign-in identities belong to people only; the harness login token is the one named exception — A sign-in identity, a provider account linked to a person in the directory, belongs to that person only: lys never links, delegates or issues from it to an agent, and refuses each such act of its own by name. An agent's own machine account, bound to no person, is a service account and stays allowed. An issuer and subject is bound to one holder, whichever came first. The harness login token recorded at docs/design/identity/STATEMENT-2026-09-22.md:43 and :167 and docs/design/identity/PROVISIONING-2026-09-22.md:18 is the one named exception; it never passes through anything lys issues, links or delegates, and it is not redefined. Rejected: letting a person lend a sign-in identity to their own agent, and redefining the login token so the rule could be claimed without an exception.
- ADR-109 — The server judges a grant's standing and effective end over its chain; the screens render them and walk nothing — GrantView carries each grant's effective standing ({stands: true}, or {stands: false, refusal, grant, reason} read through the service's sight rules, so a withheld refusal names no grant or identity the caller may not see) and its effective end, both worked out on the server by the same judgement a check makes, and every screen renders those two members and walks no chain, compares no window with the clock and reads no holder's lifecycle state to decide standing. Rejected: keeping the client chain walk beside the server's judgement, and a screen-side derivation of the effective end.
- ADR-112 — Lys hot paths do each piece of work once: no replay, no whole-state read for a sliver, no clone to read, no blocking on an async worker — Work on a request, append or open path is done once and scales with what the caller touches, not with history: lookups by index instead of scans, a checkpoint or cursor instead of a replay, a filtered read instead of the whole set, borrowed data instead of a clone made to read, one fsync per batch instead of per entry, and blocking I/O and std mutexes kept off async workers. Each fix is proved by counting the work done in a test that fails before it, never by a clock.
- ADR-113 — The identity install and its clients wait on signals, never on a clock — A wait ends on the event it is waiting for or on the failure that makes the event impossible: a service is ready when it says so or when a single connection to it succeeds after a readiness event (its log gaining its listening line, observed through the platform's file-change notification), a stopped process is gone when the platform reports its exit (kqueue EVFILT_PROC on macOS, pidfd on Linux), a peer that refuses or closes is an error at once, and nothing carries a timeout. The screen re-measures on a ResizeObserver or animation-frame signal. Rejected: shorter sleeps, which still wait on a clock; keeping timeouts as a safety net, which the rule forbids and which hides a stuck peer's cause.
- ADR-114 — An installed identity product names its build and upgrades itself in place, keeping the previous build to return to — Every Lys binary answers --version with the commit and dirty state it was built from, stamped at build time; a build with no commit to read says so in those words, never a made-up value. An exported build may be given LYS_BUILD_COMMIT as exactly 40 lowercase hexadecimal characters: its stamp is `<commit>; stated`, distinguishing the builder's statement from a commit read from Git; invalid values or disagreement with a tracked checkout's HEAD refuse the build, and changing or removing the variable refreshes the stamp. `lys identity upgrade` takes a folder of newly built binaries (and optionally a screens package), checks each one's --version, stops the service and the broker by their exit events, swaps the binaries by rename keeping the previous set beside them, starts the new ones and waits for them ready; if any new binary fails to start or become ready, it puts the previous set back, starts it, and fails naming what broke. Data, credentials, configuration and the compose services are never touched by an upgrade. Rejected: making install restart what differs, which would mix a first install's promises with an upgrade's risks; asking people to copy binaries by hand.
- ADR-115 — Lys is the only sign-in a person or a product ever sees; the issuer inside it is never shown — Lys is the single sign-on for every product: every product is a client of Lys at Lys's own origin, and no product configuration names the issuer. A person meets only Lys screens: first-run setup, sign-in, provider setup and their own account are Lys pages, and the issuer's pages, admin site, name and password files are never part of any path a person follows. First run asks the person for the administrator's name, email and password; nothing is filled from the machine.
- ADR-122 — Install provisions the audit agent and separate authenticated transport — Install provisions private stable sender material and a separate TLS-only authority key. Browser setup, after creating the real administrator, completes sender enrolment and login binding through one recoverable intent. The capability authority and event signer never sign TLS certificates. The Rauthy sender verifies chain, hostname and pinned server key before HTTP. DIRECTORY-045 R5 swaps public trust/configuration/mounts with binaries and preserves private credentials. Cross-repository source builds are ordered; activation requires matched artifacts.
- ADR-126 — Applications have connector identities; people register them with explicit permission — An application connector is its own third identity and grant-holder kind. Apps do not act as agents and do not use service accounts, which are accounts people use to service things. Only a signed-in person with an explicit ordinary register_app grant registers an app. The super administrator may grant that permission to a person; an agent, connector or service account cannot register, even if presented with such a grant. Approval creates a connector identity and binds the app to it. No app authority is implicit in approval, binding or ownership of a kind prefix: every permission is an explicit grant with a chain tracing to the super administrator. Own-kind checks, schema acts, batch and which use that connector identity and the ordinary grant engine. Extend IdentityId, grant admission and the permission-store holder, plus event and snapshot representation with compatibility tests. Preserve historical person/agent bytes and signatures; version a representation when necessary, never rewrite history or coerce identities. This supersedes DIRECTORY-048 wording allowing a service account or connector holding register_app to register, and forbids a separate registrar credential as substitute authority.
- ADR-127 — A SpiceDB wait ends on its answer, its close or its caller, and grants sections run off the async workers — Keep RelationshipStore synchronous. Run each grants section under spawn_blocking inside a cancel scope owned by the handler's future. Every SpiceDB exchange uses a non-blocking socket and waits in poll(2) on that socket and the scope's wake pipe, with no timeout, and dropping the handler's future cancels the scope, which wakes the poll so the call returns. A section that gets the grants lock after its request left returns before it opens GrantState or calls SpiceDB. The SpiceDB endpoint must be a socket address, so no name lookup is ever waited on. WAIT and the three socket timeouts go with no clock in their place. Making the SpiceDB store and the grant authority async with a tokio Mutex was weighed and not taken, because it changes the lys-identity core trait and every grant act for the same result.
- ADR-128 — The runner applies OS containment from the same Lys policy and reports native denials — Compile one Lys policy into Seatbelt on macOS and Landlock with a private network namespace on Linux. A policy-bound egress service enforces hostnames while OS rules prevent direct bypass. A runner applies containment before untrusted exec and records its policy digest and session incarnation. Native kernel events feed the same authenticated refusal stream as051, with distinct provenance from tool and proxy denials. Missing enforcement or required audit support refuses launch. gaps are visible and end affected sessions through the existing ownership mechanism. A writable outside-root fixture that succeeds without confinement is the filesystem control. Never infer sandbox enforcement from a failure to write /etc/x. Both native platforms require real tests and receipts.
- ADR-129 — Paths under /api belong to the API and are refused when unknown, and paths outside it stay the page's — The API nested under /api has its own fallback answering 404 with a named JSON refusal. Paths outside /api stay the page's, so /health and /healthz at the root keep answering the page, and the page shows what it shows for a route it does not know. The one health answer is GET /api/health, which names the service and its build and asks no other service. Serving 404 for chosen root names was weighed and not taken, because the server would then guess at the page's routes.
- ADR-131 — Codex policy comes from Lys and refusal provenance follows the real harness contract — Render the same Lys policy into isolated native Codex settings. Use064's one transport owner and051's one refusal store. Distinguish Codex-reported rejection, Lys judge denial and062 OS denial. Required unrepresentable policy refuses launch. Coverage is capability-derived, never a blanket claim.
- ADR-135 — An agent Lys starts carries a pass for that run and may be granted any action a person can take, except the responsibilities a person keeps — Every non-public action is a grantable permission. Lys issues a pass per run at spin-up, written with the Lys MCP address into the seat's config, ended when the run ends; every route takes it as the agent and judges each call against live grants. A refusal names who can grant, walking up the chain, and the agent asks them; the answer is once, for a while, ongoing or no. Grants gain one-time and transfer-and-return. A small named set of responsibilities is never passed to an agent.

## Goals

- Every open row of IDENTITY-001 (02, 04, 03, 05) exists as a valid design-system brief in this cluster, DIRECTORY-002 to DIRECTORY-005, in its dependency order, every ID001 acceptance identifier kept.
- The grant path (create an agent under a person, grant it a project, the action is allowed, revoke or suspend, the same action is refused) is a requirement of the row that owns it, with acceptance criteria; where the sources do not settle the row, it is recorded open for Tom in DIRECTORY-003 with its criteria drafted.
- Every decision still open for Tom is recorded as open and decided nowhere in this cluster.
- The two live demonstrations to Tom, ID001_LINK_LIVE and ID001_DIRECTORY_LIVE, stay hold points a loop completion never replaces (CN6).
- DIRECTORY-006 makes the grant/refusal journey enforceable and binds its acceptance to the reviewed mock-up, without rewriting the historical IDENTITY-001 record.
- DIRECTORY-007 leaves one verifiable receipt for every sign-in and permission change, verified offline from the log and the service's key alone, with receipts for changes only and test receipts that never pass as real.
- DIRECTORY-009 moves an identity between registered, active, suspended and retired only by signed record, answers its state only by folding those records, refuses a suspended or retired identity at every check and at its next session refresh by state alone, and reads a retired identity's history exactly as a live one's.
- A revoked certificate fails verification against the certificate log while its history still verifies, with revocation an appended leaf and the live set folded from the log (DIRECTORY-013, ADR-039).
- CONFORMANCE rows 3.1 and 3.2 are true on the lifecycle record, each tested by at least one acceptance line of DIRECTORY-019, reading the lifecycle-hand brief DIRECTORY-009's record and keeping no copy of it.
- The rauthy-ready leg is declared with cadence demand in docs/design/project.json and in this cluster's gate, both files validate, and the leg exits 0 at the venue only on Rauthy's ready answer (DIRECTORY-009).
- DIRECTORY-009 states and tests conformance row 1.2: linking a provider account linked to a person to an agent, and delegating a sign-in identity, are each refused by name and write nothing; one counted test over the store finds no agent record carrying a sign-in identity; the broker's refusal is recorded as a finding against SECRETS-002, on which row 1.2's full pass waits.
- Every decision still open for Tom is recorded as open and decided nowhere in this cluster. Decided under ADR-078: the grant representation was ruled by the owning lead, Archie, with Apollo as second reader, on Waffles' ruling of 12:13 on Tom's word of 12:12 that technical formats are the owning lead's with a second reader.
- Every grant and permission check the standalone identity server makes is answered by SpiceDB behind DIRECTORY-006's permission decision, the administrator's admission being the one named exception, a call after a committed revoke is never admitted, and the screen answers why an identity can and why it cannot do a thing from the server's answer (DIRECTORY-025, ADR-078).
- DIRECTORY-029 makes CONFORMANCE rows 5.1 to 5.6 true, each with at least one acceptance line that tests it, and no start path spawns a process.
- Road step 2's certificate half (defined in docs/design/identity/STATEMENT-2026-09-22.md, The road; brought into this cluster by ADR-093): an enduring agent's certificate states what it was granted at issuance in a typed, versioned claim, lys/agent-capability/v1, a new format under its own proposed OID 1.3.6.1.4.1.66364.2.1 alongside the unchanged shipped .1 extension transport, and a named verifier, verify_agent_capability, checks the chain against a set of trusted issuer keys, reads the claim and refuses what it does not cover (DIRECTORY-031).
- DIRECTORY-034 draws the access graph (conformance 8.3) with grant edges and every reachability from Access's answers, and responsible people and resource parents as recorded, holding no permission rule of its own (ADR-098).
- DIRECTORY-037 passes conformance rows 9.1 to 9.3: the surface's test command runs in .land/gates.sh and proves the rail, the dock side by all three of its controls, the route table, the palette and go-to keys, the help overlay and a counted Tab walk, and index.v6.html carries the same two fixes as the built shell.
- DIRECTORY-038 states and tests conformance row 1.2: linking a person's provider account to an agent, and delegating from a sign-in identity, are each refused by name and write nothing; one counted test over the store finds no agent record carrying a sign-in identity; the broker's refusal is recorded as a finding against SECRETS-002, on which row 1.2's full pass waits.
- DIRECTORY-061 takes the last production clock out of Lys. A SpiceDB call ends on its answer, on SpiceDB closing, or on its request leaving, and a request that leaves lets the grants lock go.
- DIRECTORY-063 refuses unknown API paths by name and adds GET /api/health, so the live install never answers a missing API with its page.

## Non-Goals

- Rows 06 (connect Cambium) and 07 (gate, install and demonstrate the release) — They change the Cambium repository and depend on IDENTITY-002, the upstream release rebase; they need a Cambium cluster or an agreed cross-repository arrangement first.
- Row 03's changes inside the Rauthy fork — The fork is its own repository under vendor/rauthy; a row whose files live there needs its own brief in the fork, which does not yet exist and blocks DIRECTORY-004. DIRECTORY-004 names the fork files as work with their owner, not as files of this repository.
- The grant representation: how one grant records who may exercise it, whether it may be passed on (person, agent or nobody), what it derives from, and whether it can be bounded. OPEN for Tom. — The statement leaves the exact delegation schema unsettled (docs/design/identity/STATEMENT-2026-09-22.md:21; ADR-003); AGENT-PARITY-2026-09-23's questions (docs/design/identity/AGENT-PARITY-2026-09-23.md:17-23) are inputs to it, not answers.
- Suspension semantics: whether suspending a person also ends their sign-in session at Rauthy or only makes our checks refuse, and what else stops with a suspended identity. OPEN for Tom. — The lifecycle document asks it of the room (docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:99-100, docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:104-105) and its states are only proposed (ADR-011); two systems, one decision.
- The name of the identity service. OPEN for Tom. — The statement lists it as not decided (docs/design/identity/STATEMENT-2026-09-22.md:192); no crate or directory name selects it.
- Which anchor the first agents pin to: our hosted anchor, a self-hosted one, or both. OPEN for Tom. — Lys leaves it as a product decision and so does the statement (docs/design/identity/STATEMENT-2026-09-22.md:193).
- A nightly Rauthy base versus waiting for an upstream release carrying #1696 and #1728. OPEN for Tom. — Waffles' ruling of 15:36:25: if upstream has published no such release by the row 05 showing, Tom decides (docs/design/identity/briefs/IDENTITY-001.json:18; docs/design/identity/STATEMENT-2026-09-22.md:177); rows 02 to 05 run on v0.36.2 meanwhile.
- Road step 2 onward: arbitrary grants and their enforcement, session launch and stop, credential handles, memory, context assembly, lanterns and anchoring in production — Revision 5 keeps them out of step 1 (docs/design/identity/briefs/IDENTITY-001.json:29-30); ADR-007 (the start command) and ADR-008 (the certificate on an agent's file) govern what the step-1 screens do not present as working. Capability certificates, CONFORMANCE rows 6.1 to 6.4 of docs/design/identity/CONFORMANCE.md, are in this cluster's scope because the project owner asked for the conformance test to be done, and anchoring in production stays out.
- The examples in AGENT-PARITY-2026-09-23 (abilities with an assignment or project, seat provisioning within a budget, private and shared notes) — Tom gave them as not yet decided (docs/design/identity/AGENT-PARITY-2026-09-23.md:11-15); they are never turned into requirements.
- A production Cambium auth cutover, and any upstream Rauthy contribution as a prerequisite — Revision 5 forbids both before scratch acceptance, review and Gypsy's coordinated install (docs/design/identity/briefs/IDENTITY-001.json:31).
- A shared design-system package extracted for every product — Tom left it as a thing to look at, not a row (ADR-010).
- Step 2's access record, where answered permission checks are recorded. — W1 (ADR-024): receipts are for changes only; when step 2 brings enforcement, allows and refusals go into a separate access record that step 2 defines, refusals always, allows decided with step 2. Nothing in DIRECTORY-007 records an answered check.
- Receipts for refusals Rauthy makes itself: a bad password, a failed provider callback. — They never reach the directory service, Rauthy's native events are asynchronous and severity-filtered (docs/design/identity/briefs/IDENTITY-001.md:44), and a signing key inside Rauthy is a fork change under ADR-009 that no brief of this repository makes (A4); sign-in receipts cover what the directory service records and refuses.
- A receipt verifier inside the directory service. — W2 (ADR-025): a verifier that needs the running service would be checking the service with the service; the verifier lives in the lys CLI, offline.
- The lys release that carries the receipt verifier. — A separate, deliberate act after IDENTITY-EVENTS.md's envelope version is ratified and recorded, never before, because publishing a crate that exposes a format freezes it (CLAUDE.md:78-80, A10); landing DIRECTORY-007 R3 on main publishes nothing.
- A separate agent-provisioning receipt. — Provisioned is a view over grants and a handle, not a transition (docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:26-28); an agent's provisioning shows as its register transition and then its grants (A5).
- The grant receipt's payload and its test legs. — They cover exactly the grant representation, which is OPEN for Tom above, and stay with DIRECTORY-006 GRANT_AUDIT; DIRECTORY-007 commits only to the contract's seven fields (A6).
- A state-based sign-in refusal, and a receipt for one, in step 1. — Lifecycle state is recorded and not enforced in step 1 (CN11, DIRECTORY-003 R5); the step-2 refusals arrive with enforcement and suspension semantics stay OPEN for Tom (A11).
- An at-once revocation of a Rauthy session on suspension or retirement. — L1 (ADR-028): the suspension takes effect at the next check, a refresh is a check, and nothing in DIRECTORY-009 calls Rauthy to revoke a session; an at-once revocation would be a later revision of the suspend transition, and whether suspending also ends the sign-in session at Rauthy stays OPEN for Tom above.
- A policy that moves an identity between lifecycle states. — The lifecycle document names no policy-caused transition; the only automatic effect of a root's suspension or retirement is the grant-check one DIRECTORY-006 R4 enforces (docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:87-89). DIRECTORY-009 names no policy actor and records that one would be a later revision of its contract.
- The lifecycle screen of step 7. — DIRECTORY-009 R6 builds the typed read and list the screen consumes and R1 says what the screen shows; the screen itself is a later unit on DIRECTORY-005's surface.
- The credential-handle half of the provisioned view. — provisioned is a view over having any grant and having a handle (docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:26-28); no row of this cluster yet brings a credential handle into the directory, so DIRECTORY-009 R6 reads the grant half and leaves the handle half false until the secrets rows bring handles.
- Road step 2 onward: capability certificates, arbitrary grants and their enforcement, session launch and stop, credential handles, memory, context assembly, lanterns and anchoring in production. Brought forward as step-2 work under the lead's ruling of 27 September 2026: DIRECTORY-013's certificate revocation fold and its history check. — Revision 5 keeps them out of step 1 (docs/design/identity/briefs/IDENTITY-001.json:29-30); ADR-007 (the start command) and ADR-008 (the certificate on an agent's file) govern what the step-1 screens do not present as working.
- Giving an agent a new person — It is its own card, the one that revises ADR-011's responsible person for life and clears the flag; DIRECTORY-019 only flags that an agent's person is retired (ADR-056).
- A screen that shows the needs-a-new-person flag on the people list and the agent file — It belongs to a later card; DIRECTORY-019 shows the flag in the lifecycle read and the lifecycle-hand brief's typed read (RM-017), which that screen consumes.
- A demand leg for SpiceDB readiness on the shared database — DIRECTORY-002 also installs SpiceDB, but the readiness leg's words name only Rauthy and its scratch database.
- Building a Rauthy image from the vendor/rauthy pin — The leg accepts only an image whose own build set its revision label to the pin, and the upstream v0.36.2 image carries no such label. Building that image is heavy venue work, and the readiness leg's card does not do it.
- A way for the method to request a demand leg and record its outcome — It lives in the design-system repository and changes the ledger for every project; design-system card 9jJgCzeG carries it, together with the acceptance line that a round records rauthy-ready as Unmeasured on a venue declaring no tool:docker.
- Adding the readiness leg to .land/gates.sh — The land gate script is not changed; a demand leg never enters landing.
- The broker's refusal to issue an agent any credential derived from a person's sign-in session — Issuance is the broker's (ADR-001); DIRECTORY-038 records it as a finding against SECRETS-002, carried by the broker's card.
- The cannot-give list on the delegation screen — The screen is row 2.4's, carried by Cambium card jAfmblAP; DIRECTORY-038 supplies only the server's reason 'sign-in identity'.
- Road step 2 onward: capability certificates, arbitrary grants and their enforcement, session launch and stop, credential handles, memory, context assembly, lanterns and anchoring in production
Brought forward as step-2 work under the lead's ruling of 27 September 2026: DIRECTORY-026's session record and session credential. — Revision 5 keeps them out of step 1 (docs/design/identity/briefs/IDENTITY-001.json:29-30); ADR-007 (the start command) and ADR-008 (the certificate on an agent's file) govern what the step-1 screens do not present as working.
- The grant representation: how one grant records who may exercise it, whether it may be passed on (person, agent or nobody), what it derives from, and whether it can be bounded. OPEN for Tom. Decided under ADR-078: the grant representation was ruled by the owning lead, Archie, with Apollo as second reader, on Waffles' ruling of 12:13 on Tom's word of 12:12 that technical formats are the owning lead's with a second reader. — The statement leaves the exact delegation schema unsettled (docs/design/identity/STATEMENT-2026-09-22.md:21; ADR-003); AGENT-PARITY-2026-09-23's questions (docs/design/identity/AGENT-PARITY-2026-09-23.md:17-23) are inputs to it, not answers. Ruling (ADR-078): the grant representation is a technical decision of the owning lead with a second reader, and is decided for the SpiceDB schema: every grant, root or delegated, is its own grant object in SpiceDB, keyed by its grant identifier, and is exactly 3 relationships: the resource's relationship to the grant object under the grant's action; the grant's holder relationship to the identity that holds it, carrying an expiry caveat that ends no later than its source's; and the grant's standing relationship, which for a delegated grant is its source relationship to its source grant, the source relation the permission walks, and for a root grant, which has no source, is its root relationship, the grant object's relationship to the directory's root authority, one object per directory and the same object the issue_root permission is checked against when a person issues a root grant through POST /grants/roots; a permission holds through a grant only while the grant's standing holds, a root grant's through its root relationship and a delegated grant's through its source grant's standing, so derivation is a walk of the source relation that ends at the directory's root authority in a bounded number of steps, a grant that stands on neither a source relationship nor a root relationship is malformed and refused, and deleting a grant's standing relationship refuses its holder and every grant derived from it at once; two live grants giving the same holder the same action on the same resource are two grant objects, each with its own relationships, and revoking one leaves the other standing; withdrawing a grant deletes its relationships and those of every grant derived from it through the source relation on the same committed revoke event, the withdrawn grant's standing relationship alone in the first write and the rest in further writes within SpiceDB's configured max_updates_per_write.
- Road step 2 onward: capability certificates, arbitrary grants and their enforcement, session launch and stop, credential handles, memory, context assembly, lanterns and anchoring in production
Brought forward as step-2 work under the identity line lead's ruling: DIRECTORY-029's Start on the agent file, which checks, gives a command and keeps its launch record, and never launches, runs or stops a session. — Revision 5 keeps them out of step 1 (docs/design/identity/briefs/IDENTITY-001.json:29-30); ADR-007 (the start command) and ADR-008 (the certificate on an agent's file) govern what the step-1 screens do not present as working.
- CONFORMANCE row 5.7: a terminal view, sandbox, VM or container runner — Proposed as separate products; lys does not run agents (ADR-007).
- How a start command proves itself, such as a one-time code for one machine for a few minutes — It stays proposed until the launch design chooses it (CONFORMANCE 5.3).
- A grant to start an agent held by someone other than its responsible person or a directory administrator — A further unit; it needs DIRECTORY-006's grant contract.
- The records the start checks read: the profile version and its review, the role's machines, the virtual credentials, the egress list, the sessions record and the provision record — Each belongs to its owner's card; DIRECTORY-029 reads them and keeps no copy.
- Row 06 (connect Cambium) — It changes the Cambium repository and depends on IDENTITY-002, the upstream release rebase; it needs a Cambium cluster or an agreed cross-repository arrangement first. Row 07 left this non-goal with DIRECTORY-030 (ADR-092).
- Road step 2 onward, except its certificate half, the typed capability claim and its verifier, which ADR-093 makes a goal: arbitrary grants and their enforcement, session launch and stop, credential handles, memory, context assembly, lanterns and anchoring in production — Revision 5 keeps them out of step 1 (docs/design/identity/briefs/IDENTITY-001.json:29-30); ADR-007 (the start command) and ADR-008 (the certificate on an agent's file) govern what the step-1 screens do not present as working. Step 2 is defined in docs/design/identity/STATEMENT-2026-09-22.md (The road); only its certificate half left this non-goal, under ADR-093.
- The graph's rail entry and g h shortcut — The shell card AL4OyeKG registers every screen's route and key; DIRECTORY-034 adds only the graph's deep links and its Show in graph links.
- Building the screens and sections the route table marks not yet: roles, resources, graph, requests, reviews, secrets, connections, network, sessions, model, and the six Configuration sections other than Layout — Each is screen content, which DIRECTORY-037 leaves unchanged, and each belongs to its own conformance rows and card; its row is marked built when that card lands.
- The assistant's composer in the dock (conformance 9.4) — Row 9.4 is proposed with no brief and its runtime is open; the dock holds a not-yet note in its place.
- A digit key for the identity file's eighth tab — The mock-up's digit keys stop at 7 while v5 has eight tabs; changing that departs from the mock-up and needs its own decision.
- Pointing the Brief cells of CONFORMANCE.md rows 9.1 to 9.3 at DIRECTORY-037 — The words change only line 3 of that file in this card; the Brief cells follow once the brief id is fixed on main.

## Structure

| Path | Note | Brief |
|------|------|-------|
| `docs/design/directory/design.json` | the directory design; gains a structure row for every path a row brief names | DIRECTORY-001 |
| `docs/design/directory/DESIGN.md` | rendered markdown | DIRECTORY-001 |
| `docs/design/directory/checklist.json` | the directory checklist; gains the row items | DIRECTORY-001 |
| `docs/design/directory/CHECKLIST.md` | rendered markdown | DIRECTORY-001 |
| `docs/design/directory/stories.json` | the directory stories; gains the row stories | DIRECTORY-001 |
| `docs/design/directory/USER-STORIES.md` | rendered markdown | DIRECTORY-001 |
| `docs/design/decisions.json` | the project decision ledger; gains the identity decisions | DIRECTORY-001 |
| `docs/design/directory/briefs/DIRECTORY-002.json` | row 02 of IDENTITY-001, converted | DIRECTORY-001 |
| `docs/design/directory/briefs/DIRECTORY-002.md` | rendered markdown | DIRECTORY-001 |
| `docs/design/directory/briefs/DIRECTORY-003.json` | row 04 of IDENTITY-001, converted | DIRECTORY-001 |
| `docs/design/directory/briefs/DIRECTORY-003.md` | rendered markdown | DIRECTORY-001 |
| `docs/design/directory/briefs/DIRECTORY-004.json` | row 03 of IDENTITY-001, converted | DIRECTORY-001 |
| `docs/design/directory/briefs/DIRECTORY-004.md` | rendered markdown | DIRECTORY-001 |
| `docs/design/directory/briefs/DIRECTORY-005.json` | row 05 of IDENTITY-001, converted | DIRECTORY-001 |
| `docs/design/directory/briefs/DIRECTORY-005.md` | rendered markdown | DIRECTORY-001 |
| `Cargo.toml` | workspace manifest; gains the identity dependencies (DIRECTORY-002) and the directory crates (DIRECTORY-003) | DIRECTORY-002 |
| `Cargo.lock` | lock file; follows Cargo.toml (DIRECTORY-002, DIRECTORY-003) | DIRECTORY-002 |
| `crates/lys/Cargo.toml` | the CLI crate's manifest; gains the identity subcommand's dependencies | DIRECTORY-002 |
| `crates/lys/src/main.rs` | the CLI entry; dispatches lys identity | DIRECTORY-002 |
| `crates/lys/src/cli.rs` | the CLI arguments; gains the identity subcommand | DIRECTORY-002 |
| `crates/lys/src/commands/error.rs` | the CLI error type; carries the identity errors | DIRECTORY-002 |
| `crates/lys/src/identity/mod.rs` | declarations and re-exports only | DIRECTORY-002 |
| `crates/lys/src/identity/cli.rs` | identity subcommand argument declarations | DIRECTORY-002 |
| `crates/lys/src/identity/config.rs` | typed deployment configuration and validation; no secret values in diagnostics | DIRECTORY-002 |
| `crates/lys/src/identity/credentials.rs` | Zeroizing, redacted credential material and stable reuse | DIRECTORY-002 |
| `crates/lys/src/identity/private_files.rs` | restricted-mode durable file creation and outcome reconciliation | DIRECTORY-002 |
| `crates/lys/src/identity/prepare.rs` | validates inputs and materialises the declared private deployment artifacts | DIRECTORY-002 |
| `crates/lys/src/identity/configure.rs` | idempotent client and theme reconciliation with stable operation identifiers | DIRECTORY-002 |
| `crates/lys/src/identity/rauthy.rs` | typed Rauthy API requests and responses, named status errors, read-back after an uncertain outcome | DIRECTORY-002 |
| `crates/lys/src/identity/themes.rs` | reads the declared estate palette mapping and validates both client themes | DIRECTORY-002 |
| `crates/lys/src/identity/health.rs` | named readiness checks for the declared services; SpiceDB readiness only | DIRECTORY-002 |
| `crates/lys/src/identity/error.rs` | typed errors carrying operation, resource and path, never secret bytes | DIRECTORY-002 |
| `crates/lys/tests/identity_deploy.rs` | ID001_DEPLOY | DIRECTORY-002 |
| `crates/lys/tests/identity_refusals.rs` | ID001_DEPLOY_REFUSAL | DIRECTORY-002 |
| `crates/lys/tests/identity_theme.rs` | ID001_THEME | DIRECTORY-002 |
| `crates/lys/tests/identity_shared_db.rs` | ID001_SHARED_DB | DIRECTORY-002 |
| `crates/lys/tests/identity_restart.rs` | restart and restore of the dependencies | DIRECTORY-002 |
| `crates/lys/tests/identity_support/mod.rs` | shared test support for the identity tests | DIRECTORY-002 |
| `crates/lys/tests/identity_support/fixtures.rs` | test identities and configuration fixtures, never real credentials | DIRECTORY-002 |
| `crates/lys/tests/identity_support/server.rs` | test server harness | DIRECTORY-002 |
| `crates/lys/tests/identity_support/compose.rs` | compose harness for the three dependencies | DIRECTORY-002 |
| `deploy/identity/compose.yaml` | Rauthy, SpiceDB and one PostgreSQL service (DIRECTORY-002); gains the directory service (DIRECTORY-003) | DIRECTORY-002 |
| `deploy/identity/versions.json` | pinned releases and image digests | DIRECTORY-002 |
| `deploy/identity/config.example.toml` | example configuration without secrets, the database address included (DIRECTORY-002, DIRECTORY-003) | DIRECTORY-002 |
| `deploy/identity/postgres-init.sql` | roles and schema namespaces for Rauthy and SpiceDB in one database | DIRECTORY-002 |
| `deploy/identity/README.md` | install, readiness, backup and restore, and SpiceDB's step-1 sentence (DIRECTORY-002); the directory (DIRECTORY-003) and the screens (DIRECTORY-005) | DIRECTORY-002 |
| `deploy/identity/rauthy-themes.json` | both Rauthy client themes | DIRECTORY-002 |
| `deploy/identity/theme-map.md` | source tokens and colour conversions of the themes | DIRECTORY-002 |
| `docs/design/identity/reports/IDENTITY-001-deployment.md` | row 02's report: digests, versions, resolved configuration without secrets, restore result | DIRECTORY-002 |
| `crates/lys-identity/` | directory records, typed API and event projection; exact manifest reviewed before the row starts | DIRECTORY-003 |
| `crates/lys-identity-server/` | OIDC session handling, administrator admission and the link-audit receiver (DIRECTORY-003); routes and assets of the screens (DIRECTORY-005) | DIRECTORY-003 |
| `tests/identity_contract/` | the directory's contract tests: ID001_DIRECTORY, ID001_AUDIT_FAULTS, ID001_ADMIN, ID001_RECEIPT, ID001_RECEIVER and the lifecycle transitions | DIRECTORY-003 |
| `docs/design/identity/DIRECTORY-CONTRACT.md` | the directory contract: identifiers, bindings, responsible person, lifecycle state | DIRECTORY-003 |
| `docs/design/identity/IDENTITY-EVENTS.md` | the versioned event envelope, reviewed jointly with Archie | DIRECTORY-003 |
| `vendor/rauthy` | the maintained fork's pin (ADR-009); moves to the gated fork commit that links two providers | DIRECTORY-004 |
| `docs/design/identity/PROVIDER-LINK-CONTRACT.md` | the typed contract between the fork's link audit and the receiver | DIRECTORY-004 |
| `docs/design/identity/reports/IDENTITY-001-links.md` | row 03's report: pinned fork commit, gate result, counted legs | DIRECTORY-004 |
| `surface/identity/` | the standalone screens; exact manifest reviewed before the row starts | DIRECTORY-005 |
| `crates/lys-identity-server/src/assets.rs` | serves the screens' assets | DIRECTORY-005 |
| `crates/lys-identity-server/src/routes.rs` | the screens' routes; DIRECTORY-006 later extends the same route seam after reconciling the dependency-owned manifest | DIRECTORY-005 |
| `docs/design/identity/reports/IDENTITY-001-standalone.md` | row 05's report: screenshots, observed actions, artifact hashes | DIRECTORY-005 |
| `docs/design/directory/briefs/DIRECTORY-006.json` | The human-rooted grants and delegation implementation brief, awaiting reviewed foundation manifests and contract decisions | DIRECTORY-006 |
| `docs/design/directory/briefs/DIRECTORY-006.md` | Rendered grants and delegation brief | DIRECTORY-006 |
| `crates/lys-identity/src/grants/mod.rs` | Define the reviewed grant contract without changing published cryptography; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `crates/lys-identity/src/grants/types.rs` | Define the reviewed grant contract without changing published cryptography; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `crates/lys-identity/src/grants/error.rs` | Define the reviewed grant contract without changing published cryptography; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `crates/lys-identity/tests/grant_contract.rs` | Define the reviewed grant contract without changing published cryptography; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `docs/design/identity/GRANT-CONTRACT.md` | Define the reviewed grant contract without changing published cryptography; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `crates/lys-identity/src/lib.rs` | Define the reviewed grant contract without changing published cryptography; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `crates/lys-identity/src/grants/admission.rs` | Enforce affirmative delegation and bounded ancestry; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `crates/lys-identity/src/grants/lineage.rs` | Enforce affirmative delegation and bounded ancestry; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `crates/lys-identity/src/grants/authority.rs` | Enforce affirmative delegation and bounded ancestry; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `crates/lys-identity/tests/grant_delegation.rs` | Enforce affirmative delegation and bounded ancestry; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `crates/lys-identity/src/grants/events.rs` | Commit grants and their audit as one replayable operation; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `crates/lys-identity/src/grants/projection.rs` | Commit grants and their audit as one replayable operation; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `crates/lys-identity/src/grants/recovery.rs` | Commit grants and their audit as one replayable operation; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `crates/lys-identity/tests/grant_faults.rs` | Commit grants and their audit as one replayable operation; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `crates/lys-identity/tests/grant_receipts.rs` | Commit grants and their audit as one replayable operation; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `crates/lys-identity/src/grants/revocation.rs` | Enforce revocation, inherited expiry and current permission decisions; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `crates/lys-identity/src/grants/expiry.rs` | Enforce revocation, inherited expiry and current permission decisions; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `crates/lys-identity/src/grants/permission.rs` | Enforce revocation, inherited expiry and current permission decisions; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `crates/lys-identity/tests/grant_revocation.rs` | Enforce revocation, inherited expiry and current permission decisions; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `crates/lys-identity/tests/grant_expiry.rs` | Enforce revocation, inherited expiry and current permission decisions; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `crates/lys-identity-server/src/grants.rs` | Expose one authenticated grant and explanation seam; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `crates/lys-identity-server/src/grant_contract.rs` | Expose one authenticated grant and explanation seam; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `crates/lys-identity-server/tests/grants.rs` | Expose one authenticated grant and explanation seam; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `crates/lys-identity-server/tests/grant_explanations.rs` | Expose one authenticated grant and explanation seam; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `surface/identity/src/features/grants/YouGrants.tsx` | Implement the You and delegation screens from the server contract; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `surface/identity/src/features/grants/DelegateGrant.tsx` | Implement the You and delegation screens from the server contract; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `surface/identity/src/features/grants/GrantExplanation.tsx` | Implement the You and delegation screens from the server contract; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `surface/identity/tests/grants.test.tsx` | Implement the You and delegation screens from the server contract; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `surface/identity/tests/acceptance/grants.spec.ts` | Implement the You and delegation screens from the server contract; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `surface/identity/src/routes.tsx` | Implement the You and delegation screens from the server contract; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `surface/identity/src/generated/index.ts` | Implement the You and delegation screens from the server contract; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `docs/design/directory/briefs/DIRECTORY-007.json` | the audit receipt brief: the contract document, the receipt crate, the CLI verifier, the sign-in receipts and the development-install proof | DIRECTORY-007 |
| `docs/design/directory/briefs/DIRECTORY-007.md` | rendered markdown | DIRECTORY-007 |
| `docs/design/identity/AUDIT-RECEIPT.md` | the audit receipt contract as it stands, the emitter of every operation, the step-1 sign-in boundary, the changes-only and test-receipt rules, and what IDENTITY-EVENTS.md must carry | DIRECTORY-007 |
| `crates/lys-receipt/Cargo.toml` | the receipt crate's manifest; depends on lys-core and lys-log-store only | DIRECTORY-007 |
| `crates/lys-receipt/src/lib.rs` | declarations and re-exports only | DIRECTORY-007 |
| `crates/lys-receipt/src/receipt.rs` | the typed receipt of the shared shape at IDENTITY-EVENTS.md's version, parsed from JSON, refusing a malformed input by name | DIRECTORY-007 |
| `crates/lys-receipt/src/verify.rs` | verification in order: test tag and key selection, signature, coordinate within extent, commitment against the leaf; one refusal class for every cryptographic or structural failure | DIRECTORY-007 |
| `crates/lys-receipt/src/error.rs` | ReceiptError: MalformedReceipt, NeedsTestKey, VerificationFailed; never carries leaf or signature bytes | DIRECTORY-007 |
| `crates/lys-receipt/tests/receipt_verify.rs` | rcpt_accept, rcpt_tamper, rcpt_leaf, rcpt_testtag, rcpt_shape, rcpt_redact | DIRECTORY-007 |
| `crates/lys-receipt/tests/support/mod.rs` | harness signer over generated test keys, fixture log builder with an injected clock, tamper table; never compiled into the library | DIRECTORY-007 |
| `crates/lys/src/commands/log/receipt.rs` | lys log verify receipt: offline, from the receipt, the log directory and the key strings | DIRECTORY-007 |
| `crates/lys/src/commands/log/mod.rs` | the log command family's module list; pre-existing, gains the receipt module line (DIRECTORY-007) |  |
| `crates/lys/tests/receipt_tests.rs` | rcli_accept, rcli_tamper, rcli_precrypto, rcli_test, rcli_readonly, driving the binary | DIRECTORY-007 |
| `crates/lys-identity-server/src/sign_in_receipt.rs` | the sign-in and sign-in-refusal receipts the directory service emits, through lys-identity's commit path (DIRECTORY-007, after DIRECTORY-003 lands) | DIRECTORY-007 |
| `crates/lys-identity-server/tests/sign_in_receipts.rs` | signin_ok, signin_refused, signin_first, signin_refused_reason | DIRECTORY-007 |
| `docs/design/identity/reports/DIRECTORY-007-receipts.md` | the development-install proof: commands, exit codes, leaf indices, operation IDs, key fingerprints and counts; never a private key | DIRECTORY-007 |
| `.land/gates.sh` | the landing gate; gains the identity leg: container_runtime_missing when no runtime answers, then clippy and the test = false identity targets | DIRECTORY-002 |
| `CLAUDE.md` | the repository's standing instructions; its gate list gains one line naming the identity-release demand leg | DIRECTORY-030 |
| `crates/lys/src/identity/config_tests.rs` | unit tests of config.rs | DIRECTORY-002 |
| `crates/lys/src/identity/credentials_tests.rs` | unit tests of credentials.rs, the Debug and Display redaction included | DIRECTORY-002 |
| `crates/lys/src/identity/error_tests.rs` | unit tests of error.rs, the Debug and Display redaction included | DIRECTORY-002 |
| `crates/lys/src/identity/prepare_tests.rs` | unit tests of prepare.rs, the private files' modes included | DIRECTORY-002 |
| `crates/lys/src/identity/themes_tests.rs` | unit tests of themes.rs | DIRECTORY-002 |
| `docs/design/directory/briefs/DIRECTORY-009.json` | conformance row 1.2: sign-in identities belong to people only, refused by name at every directory act that would give one to an agent | DIRECTORY-009 |
| `docs/design/directory/briefs/DIRECTORY-009.md` | rendered markdown | DIRECTORY-009 |
| `docs/design/identity/LIFECYCLE-CONTRACT.md` | the lifecycle contract, created by the lifecycle-hand brief DIRECTORY-009 (RM-017); DIRECTORY-019 R1 adds its conformance section for rows 3.1 and 3.2 | DIRECTORY-019 |
| `crates/lys-identity/src/lifecycle/mod.rs` | the lifecycle module list, created by the lifecycle-hand brief DIRECTORY-009 (RM-017); DIRECTORY-019 R2 adds the responsible module | DIRECTORY-019 |
| `crates/lys-identity/src/lifecycle/causes.rs` | who may cause each transition, created by the lifecycle-hand brief DIRECTORY-009 (RM-017); DIRECTORY-019 R2 adds one line calling the no-person check | DIRECTORY-019 |
| `crates/lys-identity/src/lifecycle/error.rs` | the lifecycle refusal names, created by the lifecycle-hand brief DIRECTORY-009 (RM-017); DIRECTORY-019 R2 adds lifecycle_agent_without_person | DIRECTORY-019 |
| `crates/lys-identity/src/lifecycle/state.rs` | the state type of exactly four values: registered, active, suspended, retired; provisioned is not among them | DIRECTORY-009 |
| `crates/lys-identity/src/lifecycle/fold.rs` | the fold of an identity's transition records in log order through LeafStore; refuses lifecycle_fold_invalid at the coordinate of a record the table refuses; never opens the log for writing | DIRECTORY-009 |
| `crates/lys-identity/src/lifecycle/gate.rs` | the access check as the conjunction, state first, then DIRECTORY-006 R4's grant existence and freshness through an injected answerer and clock; appends nothing | DIRECTORY-009 |
| `crates/lys-identity/tests/lifecycle_support/mod.rs` | the lifecycle test support, created by the lifecycle-hand brief DIRECTORY-009 (RM-017); DIRECTORY-019 adds persons with agents to its fixtures | DIRECTORY-019 |
| `crates/lys-identity/tests/lifecycle_causes.rs` | d009_r2_ac1 to d009_r2_ac4 | DIRECTORY-009 |
| `crates/lys-identity/tests/lifecycle_fold.rs` | d009_r3_ac1 to d009_r3_ac5 | DIRECTORY-009 |
| `crates/lys-identity/tests/lifecycle_gate.rs` | d009_r4_ac1 to d009_r4_ac5 | DIRECTORY-009 |
| `crates/lys-identity-server/src/lib.rs` | the directory service's crate root; declares the issuer key, key enrolment, key replacement and issuance route modules | DIRECTORY-031 |
| `crates/lys-identity-server/src/lifecycle_gate.rs` | the state refusal at sign-in and at refresh, called by one line at DIRECTORY-003's session resolution point; calls Rauthy to revoke nothing | DIRECTORY-009 |
| `crates/lys-identity-server/src/lifecycle_read.rs` | the typed lifecycle read, created by the lifecycle-hand brief DIRECTORY-009 (RM-017); DIRECTORY-019 R3 adds the needs_a_new_person field | DIRECTORY-019 |
| `crates/lys-identity-server/tests/lifecycle_session.rs` | d009_r5_ac1 to d009_r5_ac5, against a test issuer stub that records every request | DIRECTORY-009 |
| `crates/lys-identity-server/tests/lifecycle_read.rs` | d009_r6_ac1 to d009_r6_ac5 | DIRECTORY-009 |
| `docs/design/directory/briefs/DIRECTORY-008.json` | the grant brief residue after PR 6: DIRECTORY-005's verification line, three inventory rows and the intention sentence, as requirements on the documents | DIRECTORY-008 |
| `docs/design/directory/briefs/DIRECTORY-008.md` | rendered markdown | DIRECTORY-008 |
| `docs/design/directory/briefs/DIRECTORY-013.json` | certificate revocation as an appended leaf, the live set folded from the log, and the revoked certificate's history (DP26, ADR-039) | DIRECTORY-013 |
| `docs/design/directory/briefs/DIRECTORY-013.md` | rendered markdown | DIRECTORY-013 |
| `docs/design/identity/CERTIFICATE-REVOCATION.md` | the certificate revocation contract: leaf layouts, unit, signer, permanence, fold, N and tolerance, history, no-log forms, refusals | DIRECTORY-013 |
| `crates/lys-identity/src/revocation/mod.rs` | declarations, re-exports and module docs only | DIRECTORY-013 |
| `crates/lys-identity/src/revocation/leaf.rs` | the three certificate-log leaves and the revocation's signed bytes | DIRECTORY-013 |
| `crates/lys-identity/src/revocation/error.rs` | the named revocation refusals, never carrying key material | DIRECTORY-013 |
| `crates/lys-identity/src/revocation/fold.rs` | the live set folded from a LeafStore to its extent; reads only | DIRECTORY-013 |
| `crates/lys-identity/src/revocation/append.rs` | issuance, revocation and attestation-entry leaves appended through Log::append | DIRECTORY-013 |
| `crates/lys-identity/src/revocation/verify.rs` | revocation-aware certificate verification with N and the tolerance as inputs | DIRECTORY-013 |
| `crates/lys-identity/src/revocation/history.rs` | an attestation by a certificate's key judged against the revocation leaf's position | DIRECTORY-013 |
| `crates/lys-identity/tests/revocation_leaf.rs` | leaf encoding, malformed refusals and the revocation signer | DIRECTORY-013 |
| `crates/lys-identity/tests/revocation_fold.rs` | the fold's live set, refusals and read-only pass | DIRECTORY-013 |
| `crates/lys-identity/tests/revocation_append.rs` | append at the extent, reopen and write failure | DIRECTORY-013 |
| `crates/lys-identity/tests/revocation_verify.rs` | revoked, stale, unreadable, not-in-log, expired and no-log legs | DIRECTORY-013 |
| `crates/lys-identity/tests/revocation_history.rs` | attestations before and after revocation, proofs and issuance record | DIRECTORY-013 |
| `crates/lys-identity/tests/revocation_support/mod.rs` | test authorities, certificates and file-backed logs; generated keys only | DIRECTORY-013 |
| `crates/lys/src/cli_tests.rs` | the CLI's help tests; DIRECTORY-013 adds the no-log revocation sentence test | DIRECTORY-013 |
| `crates/lys-identity/tests/agent_without_session.rs` | the proof that an agent registered with no session is listed under its responsible person and read with no session credential (R2) | DIRECTORY-011 |
| `docs/design/directory/briefs/DIRECTORY-011.json` | the enduring agent kept apart from each session's credential: road adjustment 3 written into the directory contract, and the no-session agent proved (R1, R2) | DIRECTORY-011 |
| `docs/design/directory/briefs/DIRECTORY-011.md` | rendered markdown | DIRECTORY-011 |
| `crates/lys-identity/src/lifecycle/responsible.rs` | the no-person refusal and the needs-a-new-person flag worked out on read (DIRECTORY-019 R2, R3) | DIRECTORY-019 |
| `crates/lys-identity/tests/lifecycle_no_person.rs` | CONFORMANCE 3.1: an agent recorded with no person is refused by name (DIRECTORY-019 R2) | DIRECTORY-019 |
| `crates/lys-identity/tests/lifecycle_needs_new_person.rs` | CONFORMANCE 3.1: retiring a person flags every one of their agents as needing a new person, and suspending one flags none (DIRECTORY-019 R3) | DIRECTORY-019 |
| `crates/lys-identity/tests/lifecycle_walk.rs` | CONFORMANCE 3.2: the walk of all 20 changes of state (DIRECTORY-019 R4) | DIRECTORY-019 |
| `crates/lys-identity-server/tests/lifecycle_authority.rs` | CONFORMANCE 3.2: the state is authority only and the read says nothing about running (DIRECTORY-019 R5) | DIRECTORY-019 |
| `docs/design/project.json` | the project's gate trees; gains the demand-cadence identity-release leg | DIRECTORY-030 |
| `scripts/identity-gates/rauthy_ready.py` | the rauthy-ready leg's command: start the pinned Rauthy against a scratch PostgreSQL, pass on its ready answer, remove what it made | DIRECTORY-009 |
| `crates/lys/tests/rauthy_ready_leg.rs` | runs the rauthy-ready script against a stub runtime; counts refusals, cleanup and searched secrets | DIRECTORY-009 |
| `crates/lys-identity/src/sign_in_identity.rs` | classifies an issuer-subject pair as a sign-in identity, an agent's own account, unbound or unreconciled, from the signed history | DIRECTORY-038 |
| `crates/lys-identity/src/sign_in_refusal.rs` | the sign-in identity refusal and its owner-and-administrator view and its view for anyone else | DIRECTORY-038 |
| `crates/lys-identity/tests/sign_in_classification.rs` | the four classes of an issuer-subject pair, through reopen | DIRECTORY-038 |
| `crates/lys-identity/tests/sign_in_refusal_views.rs` | the two views of the refusal, and the absence of provider and subject from the view for anyone else | DIRECTORY-038 |
| `crates/lys-identity/tests/sign_in_agent_binding.rs` | a person's provider account refused as an agent's binding, an agent's own machine account accepted | DIRECTORY-038 |
| `crates/lys-identity-server/src/sign_in_link_check.rs` | the directory's side of the link path: a person's link of an account bound to an agent refused by name and recorded in the history, the agent withheld from the person linking | DIRECTORY-038 |
| `crates/lys-identity-server/tests/sign_in_link_check.rs` | the reverse-order refusal, its history entry naming the agent to the responsible person and the administrator and never to the person linking, the unbound answer and the two refused callers | DIRECTORY-038 |
| `crates/lys-identity/tests/grant_sign_in_identity.rs` | delegating from a sign-in identity refused for agent and person recipients; a service-access grant on the same account admitted | DIRECTORY-038 |
| `crates/lys-identity-server/tests/grant_cannot_give.rs` | the reason 'sign-in identity' on the cannot-give list, and the two views at the grant seam | DIRECTORY-038 |
| `crates/lys-identity-server/tests/sign_in_store.rs` | every refusal writes nothing; no agent record carries a sign-in identity, counted over the store | DIRECTORY-009 |
| `docs/design/directory/briefs/DIRECTORY-024.json` | the cannot-give list: everything a person cannot give on the delegation form, with its one reason (conformance row 2.4) | DIRECTORY-024 |
| `docs/design/directory/briefs/DIRECTORY-024.md` | rendered markdown | DIRECTORY-024 |
| `crates/lys-identity/src/grants/cannot_give.rs` | the closed reason set, its precedence, and the per-recipient cannot-give list computed from the authority and lineage decisions | DIRECTORY-024 |
| `crates/lys-identity/tests/grant_cannot_give.rs` | the cannot-give fixture, precedence, in-force, source-mark, service-account and no-rank cases | DIRECTORY-024 |
| `crates/lys-identity-server/src/grant_contract/requests.rs` | the cannot-give request: the source grant and the chosen recipient | DIRECTORY-024 |
| `crates/lys-identity-server/src/grant_contract/views.rs` | the grant view the screens read; DIRECTORY-035 adds each grant's effective standing and effective end | DIRECTORY-035 |
| `surface/identity/src/features/grants/CannotGiveList.tsx` | renders the server's cannot-give answer and nothing else | DIRECTORY-024 |
| `surface/identity/src/features/grants/cannotGiveAnswer.ts` | decodes the cannot-give answer and refuses an unknown reason by name | DIRECTORY-024 |
| `surface/identity/tests/cannot_give.test.tsx` | the cannot-give list on the delegation form against fixture answers | DIRECTORY-024 |
| `docs/design/identity/CONFORMANCE.md` | the mock-up conformance table; DIRECTORY-037 changes its line 3 to name index.v6.html, with its sha256, as the reference and v5 as the prior | DIRECTORY-037 |
| `docs/design/WIRE-FORMATS.md` | the wire-format register; gains one line pointing at the capability-claim proposal and its PROPOSED decision-log row, which reads RATIFIED once the ratification is recorded on the card (R3), and both of its ratification sentences name the owning lead with a second reader; D1 to D6 and every other existing line unchanged | DIRECTORY-031 |
| `docs/design/identity/SESSION-CREDENTIAL-REVIEW.md` | the adversarial review of the session credential, the challenge and the lys/session-start/v1 report-back proof, recorded before any session code (R4) | DIRECTORY-026 |
| `crates/lys-identity/src/agent_session/mod.rs` | declarations and re-exports only; named agent_session so it is never confused with the server's operator sign-in session | DIRECTORY-026 |
| `crates/lys-identity/src/agent_session/record.rs` | the session record: its id, agent directory id, launch record id, credential hash and open or ended state, rebuilt from its events | DIRECTORY-026 |
| `crates/lys-identity/src/agent_session/credential.rs` | the session credential: 32 random bytes, Zeroizing, redacted Debug, kept only as its SHA-256 | DIRECTORY-026 |
| `crates/lys-identity/src/agent_session/challenge.rs` | the directory-issued challenge: 32 random bytes held for one agent id and one launch record id until used, expired more than 60 seconds after issue by a clock passed in, with a byte snapshot for tests | DIRECTORY-026 |
| `crates/lys-identity/src/agent_session/proof.rs` | the lys/session-start/v1 message over the agent's directory id, the directory's identifier, the launch record id and the challenge, and its strict verification under the enrolled key, checked first | DIRECTORY-026 |
| `crates/lys-identity/src/agent_session/events.rs` | the session_started and session_ended payloads | DIRECTORY-026 |
| `crates/lys-identity/src/agent_session/error.rs` | the named session refusals, proof_invalid carrying one body | DIRECTORY-026 |
| `crates/lys-identity/src/agent_session/start.rs` | issuing the challenge and starting a session at the agent's report-back (R5) | DIRECTORY-026 |
| `crates/lys-identity/src/agent_session/present.rs` | checking a presented session credential (R6) | DIRECTORY-026 |
| `crates/lys-identity/src/agent_session/end.rs` | the agent's reported end and the administrator's stop (R7) | DIRECTORY-026 |
| `crates/lys-identity/src/agent_session/list.rs` | an agent's sessions with their states (R9) | DIRECTORY-026 |
| `crates/lys-identity/src/agent_session/revoked.rs` | refusing a new session to an agent whose certificate has been revoked, read from the revocation fold (R10) | DIRECTORY-026 |
| `crates/lys-identity/tests/agent_session_start.rs` | the challenge, the session start, the signature checked first and the named refusals (R5) | DIRECTORY-026 |
| `crates/lys-identity/tests/agent_session_present.rs` | the presentation check (R6) | DIRECTORY-026 |
| `crates/lys-identity/tests/agent_second_session.rs` | the second-session proof and the ended session's refusal (R7) | DIRECTORY-026 |
| `crates/lys-identity/tests/agent_session_list.rs` | the session listing (R9) | DIRECTORY-026 |
| `crates/lys-identity/tests/agent_session_revoked.rs` | the revoked certificate's refusal (R10) | DIRECTORY-026 |
| `crates/lys-identity-server/src/agent_session_routes.rs` | the challenge, report-back, presentation, end, stop and list routes, admitting an agent only to its own session (R8, R9, R10) | DIRECTORY-026 |
| `tests/identity_contract/tests/agent_session_routes.rs` | the session routes, the one proof_invalid body, and the stop and list routes' admission (R8, R9, R10) | DIRECTORY-026 |
| `docs/design/directory/briefs/DIRECTORY-026.json` | each session its own directory record with its own credential, started when the agent reports back with a signed challenge (R1 to R10) | DIRECTORY-026 |
| `docs/design/directory/briefs/DIRECTORY-026.md` | rendered markdown | DIRECTORY-026 |
| `crates/lys-identity/src/event.rs` | gains change kinds 7 session_started, 8 session_ended and 9 agent_reported, appended (DIRECTORY-026 R5) | DIRECTORY-026 |
| `crates/lys-identity/src/encoding.rs` | encodes and decodes the three session change kinds without changing an existing kind's bytes (DIRECTORY-026 R5) | DIRECTORY-026 |
| `crates/lys-identity/src/provenance.rs` | gains actor method codes 2 (agent, enrolled key) and 3 (agent, session credential), appended (DIRECTORY-026 R5) | DIRECTORY-026 |
| `crates/lys-identity/Cargo.toml` | the directory crate's manifest; gains the capability claim's encoding dependency from the workspace lockfile | DIRECTORY-031 |
| `crates/lys-identity-server/src/config.rs` | reads, validates and documents directory_id beside the service's other configured values (DIRECTORY-026 R8) | DIRECTORY-026 |
| `tests/identity_contract/src/harness.rs` | starts the in-process service with a configured directory_id for the session route tests (DIRECTORY-026 R8) | DIRECTORY-026 |
| `docs/design/directory/briefs/DIRECTORY-021.json` | the roles and their versions brief, conformance 4.2 to 4.5 (roles half) | DIRECTORY-021 |
| `docs/design/directory/briefs/DIRECTORY-021.md` | rendered markdown | DIRECTORY-021 |
| `crates/lys-identity/src/roles/mod.rs` | R1: Define the role, version and holding records and their signed events | DIRECTORY-021 |
| `crates/lys-identity/src/roles/types.rs` | R1: Define the role, version and holding records and their signed events | DIRECTORY-021 |
| `crates/lys-identity/src/roles/events.rs` | R1: Define the role, version and holding records and their signed events | DIRECTORY-021 |
| `crates/lys-identity/src/roles/error.rs` | R1: Define the role, version and holding records and their signed events | DIRECTORY-021 |
| `crates/lys-identity/tests/roles_records.rs` | R1: Define the role, version and holding records and their signed events | DIRECTORY-021 |
| `crates/lys-identity/src/roles/check.rs` | R2: Test project ownership by the explicit owner relation, and answer every role-version check through one seam | DIRECTORY-021 |
| `crates/lys-identity/src/roles/ownership.rs` | R2: Test project ownership by the explicit owner relation, and answer every role-version check through one seam | DIRECTORY-021 |
| `crates/lys-identity/tests/roles_check.rs` | R2: Test project ownership by the explicit owner relation, and answer every role-version check through one seam | DIRECTORY-021 |
| `crates/lys-identity/src/roles/holding.rs` | R3: Assign a holding at the role's current version, its grants copied from that version's templates | DIRECTORY-021 |
| `crates/lys-identity/tests/roles_assign.rs` | R3: Assign a holding at the role's current version, its grants copied from that version's templates | DIRECTORY-021 |
| `crates/lys-identity/src/roles/edit.rs` | R4: Make a role and a new version when its templates are edited, record a title edit, and move no holder | DIRECTORY-021 |
| `crates/lys-identity/tests/roles_edit.rs` | R4: Make a role and a new version when its templates are edited, record a title edit, and move no holder | DIRECTORY-021 |
| `crates/lys-identity/src/roles/holders.rs` | R5: Answer who holds a role, and on which version, as a query over holdings | DIRECTORY-021 |
| `crates/lys-identity/tests/roles_holders.rs` | R5: Answer who holds a role, and on which version, as a query over holdings | DIRECTORY-021 |
| `crates/lys-identity/src/roles/move_holder.rs` | R6: Move a holder to a newer version by a deliberate act that shows what changes first | DIRECTORY-021 |
| `crates/lys-identity/tests/roles_move.rs` | R6: Move a holder to a newer version by a deliberate act that shows what changes first | DIRECTORY-021 |
| `crates/lys-identity/src/roles/timing.rs` | R7: Keep a running session on the version it started with, and stop every open session first for a move now | DIRECTORY-021 |
| `crates/lys-identity/tests/roles_move_timing.rs` | R7: Keep a running session on the version it started with, and stop every open session first for a move now | DIRECTORY-021 |
| `crates/lys-identity/src/roles/renewal.rs` | R8: Renew a holding as a new recorded grant, at the version its policy names | DIRECTORY-021 |
| `crates/lys-identity/tests/roles_renewal.rs` | R8: Renew a holding as a new recorded grant, at the version its policy names | DIRECTORY-021 |
| `crates/lys-identity/src/roles/policy.rs` | R9: Show each holding's policy, move date and who can stop the move, and change a holding's policy or a role's default by a recorded act | DIRECTORY-021 |
| `crates/lys-identity/tests/roles_policy.rs` | R9: Show each holding's policy, move date and who can stop the move, and change a holding's policy or a role's default by a recorded act | DIRECTORY-021 |
| `crates/lys-identity-server/src/roles.rs` | R10: Expose the role acts through the identity server, which alone decides them, and carry the owner relation into SpiceDB | DIRECTORY-021 |
| `crates/lys-identity-server/tests/roles.rs` | R10: Expose the role acts through the identity server, which alone decides them, and carry the owner relation into SpiceDB | DIRECTORY-021 |
| `crates/lys-identity/tests/roles_owner_relationship.rs` | R10: Expose the role acts through the identity server, which alone decides them, and carry the owner relation into SpiceDB | DIRECTORY-021 |
| `surface/identity/src/features/roles/RoleVersions.tsx` | R11: Show versions, holders, move dates and the move preview on the role screen from the server's answers | DIRECTORY-021 |
| `surface/identity/src/features/roles/RoleHolders.tsx` | R11: Show versions, holders, move dates and the move preview on the role screen from the server's answers | DIRECTORY-021 |
| `surface/identity/src/features/roles/MoveHolder.tsx` | R11: Show versions, holders, move dates and the move preview on the role screen from the server's answers | DIRECTORY-021 |
| `surface/identity/src/features/roles/HoldingPolicy.tsx` | R11: Show versions, holders, move dates and the move preview on the role screen from the server's answers | DIRECTORY-021 |
| `surface/identity/src/features/roles/RoleBuilder.tsx` | R11: Show versions, holders, move dates and the move preview on the role screen from the server's answers | DIRECTORY-021 |
| `surface/identity/src/features/roles/AssignRole.tsx` | R11: Show versions, holders, move dates and the move preview on the role screen from the server's answers | DIRECTORY-021 |
| `surface/identity/tests/roles.test.tsx` | R11: Show versions, holders, move dates and the move preview on the role screen from the server's answers | DIRECTORY-021 |
| `surface/identity/tests/acceptance/roles.spec.ts` | R11: Show versions, holders, move dates and the move preview on the role screen from the server's answers | DIRECTORY-021 |
| `docs/design/directory/briefs/DIRECTORY-025.json` | road step 2: every permission check the identity server makes asks SpiceDB, and the screen answers why | DIRECTORY-025 |
| `docs/design/directory/briefs/DIRECTORY-025.md` | rendered markdown | DIRECTORY-025 |
| `crates/lys-identity-server/Cargo.toml` | the server crate's manifest inside DIRECTORY-003's wall; DIRECTORY-025 adds the SpiceDB client dependencies (tonic, prost, protox) | DIRECTORY-003 |
| `crates/lys-identity-server/build.rs` | compiles the vendored authzed v1 protocol files with protox; no protoc | DIRECTORY-025 |
| `crates/lys-identity-server/proto/SOURCE.md` | the authzed API release the protocol files are copied from | DIRECTORY-025 |
| `crates/lys-identity-server/proto/authzed/` | the vendored authzed v1 protocol files | DIRECTORY-025 |
| `crates/lys-identity-server/src/spicedb/mod.rs` | declarations and re-exports only | DIRECTORY-025 |
| `crates/lys-identity-server/src/spicedb/client.rs` | the one SpiceDB gRPC client; NOT_FOUND on ReadSchema is the typed no-schema result; the preshared key redacted | DIRECTORY-025 |
| `crates/lys-identity-server/src/spicedb/error.rs` | typed SpiceDB errors naming the operation and address, never the key | DIRECTORY-025 |
| `crates/lys-identity-server/src/spicedb/schema.rs` | writes the schema at start-up when absent and refuses a stored schema that differs | DIRECTORY-025 |
| `crates/lys-identity-server/src/spicedb/schema.zed` | the SpiceDB schema: the grant representation ADR-078 records, with the expiry caveat and the source relation | DIRECTORY-025 |
| `crates/lys-identity-server/src/spicedb/projector.rs` | the one relationship writer: applies committed signed grant events in log order and records position and revision token; deletes a revoked grant's standing relationship first and its remaining and derived relationships in writes within the configured max_updates_per_write, resuming on replay | DIRECTORY-025 |
| `crates/lys-identity-server/src/spicedb/check.rs` | the one evaluator and the only CheckPermission caller, behind DIRECTORY-006 R4's permission decision; offers the plain check and the traced variant, each at least as fresh as the projector's recorded token; sends the current time from the injected named clock as the expiry caveat's context on every call and refuses CONDITIONAL_PERMISSION as permission_conditional | DIRECTORY-025 |
| `crates/lys-identity-server/src/spicedb/freshness.rs` | refuses with StaleDecision (GrantError::StaleDecision), naming the affected grant, only for a check that depends on a grant event the projection has not applied, discarding the verdict check.rs returned; unrelated authority stays usable | DIRECTORY-025 |
| `crates/lys-identity-server/src/spicedb/explain.rs` | why can and why cannot, from check.rs's traced variant, behind DIRECTORY-006 R5's explain seam; never calls CheckPermission itself | DIRECTORY-025 |
| `crates/lys-identity-server/src/spicedb/lookup.rs` | who can act on a resource, the only LookupSubjects caller, read at the forward answer's revision behind DIRECTORY-006 R5's explain seam | DIRECTORY-025 |
| `crates/lys-identity-server/tests/spicedb_support/mod.rs` | declarations only | DIRECTORY-025 |
| `crates/lys-identity-server/tests/spicedb_support/server.rs` | starts a disposable in-memory SpiceDB from the release deploy/identity/versions.json pins | DIRECTORY-025 |
| `crates/lys-identity-server/tests/spicedb_projection.rs` | schema at start-up and relationship projection, against a hand-written expected list | DIRECTORY-025 |
| `crates/lys-identity-server/tests/spicedb_checks.rs` | every route asks CheckPermission once; engine outage refuses | DIRECTORY-025 |
| `crates/lys-identity-server/tests/spicedb_freshness.rs` | no call admitted after a committed revoke; only the affected grant is refused while the projection lags | DIRECTORY-025 |
| `crates/lys-identity-server/tests/agent_mid_task.rs` | an agent's two-step task refused on its next call | DIRECTORY-025 |
| `crates/lys-identity-server/tests/spicedb_explain.rs` | why can and why cannot agree with the check; who can agrees with the forward answers at one revision | DIRECTORY-025 |
| `crates/lys-identity/src/grants/relationships.rs` | pure mapping from a committed grant event to SpiceDB relationship updates | DIRECTORY-025 |
| `crates/lys-identity/tests/grant_relationships.rs` | the mapping over fixture events | DIRECTORY-025 |
| `surface/identity/src/features/grants/PermissionWhy.tsx` | the why view, showing the server's answer and never deciding | DIRECTORY-025 |
| `surface/identity/tests/permission_why.test.tsx` | the why view against fixture answers | DIRECTORY-025 |
| `surface/identity/tests/acceptance/permission_why.spec.ts` | the why view against the standalone server | DIRECTORY-025 |
| `docs/design/directory/briefs/DIRECTORY-029.json` | the start-an-agent brief: CONFORMANCE rows 5.1 to 5.6 | DIRECTORY-029 |
| `docs/design/directory/briefs/DIRECTORY-029.md` | rendered markdown | DIRECTORY-029 |
| `crates/lys-identity/src/start/mod.rs` | declarations and re-exports of the start module only | DIRECTORY-029 |
| `crates/lys-identity/src/start/request.rs` | the start request: agent, profile version and machine only, resolved to the enduring agent record DIRECTORY-011 keeps | DIRECTORY-029 |
| `crates/lys-identity/src/start/error.rs` | the named start refusals and their words, never a credential value | DIRECTORY-029 |
| `crates/lys-identity/tests/start_request.rs` | R1 test: define the start request, resolve the agent to its enduring record, and name every refusal | DIRECTORY-029 |
| `crates/lys-identity/src/start/authority.rs` | who may start, give again and withdraw: the responsible person and a directory administrator | DIRECTORY-029 |
| `crates/lys-identity/tests/start_authority.rs` | R2 test: admit a start, a start again and a withdrawal only from the agent's responsible person or a directory administrator | DIRECTORY-029 |
| `crates/lys-identity/src/start/checks.rs` | the five named checks, run in order, each reading its owner's record | DIRECTORY-029 |
| `crates/lys-identity/src/start/active.rs` | the agent is active, read from DIRECTORY-003's lifecycle state | DIRECTORY-029 |
| `crates/lys-identity/tests/start_checks.rs` | R3 test: run the five named checks before any command, with the agent is active read live from the lifecycle record | DIRECTORY-029 |
| `crates/lys-identity/src/start/profile_review.rs` | its profile version is reviewed, read from Ink1H1Os's review record | DIRECTORY-029 |
| `crates/lys-identity/tests/start_profile_review.rs` | R4 test: check that the profile version is reviewed, from the roles card's review record | DIRECTORY-029 |
| `crates/lys-identity/src/start/machine_role.rs` | the machine is allowed for the role, read from Ink1H1Os's role machines | DIRECTORY-029 |
| `crates/lys-identity/tests/start_machine_role.rs` | R5 test: check that the machine is allowed for the role, from the role's machines in the roles card's record | DIRECTORY-029 |
| `crates/lys-identity/src/start/credentials.rs` | its virtual credentials are valid, read from SECRETS-002's handle record; ids only | DIRECTORY-029 |
| `crates/lys-identity/tests/start_credentials.rs` | R6 test: check that the agent's virtual credentials are valid, from the door's handle record | DIRECTORY-029 |
| `crates/lys-identity-server/src/door_handles.rs` | the HandleRecords client of the door's handle-resolving endpoint (SECRETS-002 R1); ids and validity only, and an answer carrying a credential value is refused by name as credential_value_in_answer; wired into the start route by R12 | DIRECTORY-029 |
| `crates/lys-identity-server/tests/door_handles.rs` | R6 test: the HandleRecords door client against a local stub of the door's handle read shape; ids and validity for an answer without a value, and the credential_value_in_answer refusal for an answer with one | DIRECTORY-029 |
| `crates/lys-identity-server/examples/door_handles.rs` | R6 verification: reads the door's handle records through the same client for a given door address and agent, and prints credential ids and counts only, never a value | DIRECTORY-029 |
| `crates/lys-identity/src/start/egress.rs` | the machine may reach what the profile needs, read from row 8.5's egress list | DIRECTORY-029 |
| `crates/lys-identity/tests/start_egress.rs` | R7 test: check that the machine may reach what the profile needs, from its egress list | DIRECTORY-029 |
| `crates/lys-identity/src/start/profile_command.rs` | the executable, its arguments and the working directory read from the reviewed profile version through the ProfileVersionRecords seam over Ink1H1Os's record | DIRECTORY-029 |
| `crates/lys-identity/tests/start_profile_command.rs` | R9 test: take the executable, its arguments and the working directory from the reviewed profile version | DIRECTORY-029 |
| `crates/lys-identity/src/start/launch_record.rs` | the launch record and the withdrawal as signed directory events, appended and read by id | DIRECTORY-029 |
| `crates/lys-identity/src/start/state.rs` | running, unconfirmed and withdrawn, derived from the sessions record, and whether a start for an agent stands unconfirmed | DIRECTORY-029 |
| `crates/lys-identity/src/start/withdrawal.rs` | the withdrawal event: the request no longer stands, who and when | DIRECTORY-029 |
| `crates/lys-identity/tests/start_state.rs` | R10 test: running only on a signed report, unconfirmed without one, a withdrawal that never claims not started, and whether a start stands | DIRECTORY-029 |
| `crates/lys-identity/src/start/give.rs` | gives a start: runs the authority step and the checks, keeps a launch record, gives the command, gives a start again, and refuses a second start while one stands unconfirmed | DIRECTORY-029 |
| `crates/lys-identity/src/start/command.rs` | builds the command from a launch record and the profile version it names: the executable with its recorded arguments in its working directory, with no credential value | DIRECTORY-029 |
| `crates/lys-identity/tests/start_launch_record.rs` | R11 test: keep a launch record for every command given, render the command from it, give a start again, refuse a second start while one stands, and give a second start of a running agent without writing a session record | DIRECTORY-029 |
| `crates/lys-identity-server/src/start.rs` | the start route: give, give again, withdraw, state | DIRECTORY-029 |
| `crates/lys-identity-server/tests/start.rs` | R12 test: answer a start through the route and the CLI, and prove that no start path spawns a process | DIRECTORY-029 |
| `crates/lys/src/identity/start.rs` | the lys identity start CLI answer | DIRECTORY-029 |
| `crates/lys/tests/identity_start.rs` | R12 test: answer a start through the route and the CLI, and prove that no start path spawns a process | DIRECTORY-029 |
| `crates/lys-identity/tests/start_no_spawn.rs` | R12 test: answer a start through the route and the CLI, and prove that no start path spawns a process | DIRECTORY-029 |
| `surface/identity/src/features/start/StartDrawer.tsx` | the agent file's Start drawer | DIRECTORY-029 |
| `surface/identity/src/features/start/StartNotice.tsx` | the unconfirmed, withdrawn and running notice | DIRECTORY-029 |
| `surface/identity/tests/start.test.tsx` | R13 test: build the Start drawer and the unconfirmed notice on the agent file | DIRECTORY-029 |
| `surface/identity/tests/acceptance/start.spec.ts` | R13 test: build the Start drawer and the unconfirmed notice on the agent file | DIRECTORY-029 |
| `docs/design/directory/briefs/DIRECTORY-030.json` | row 07 of IDENTITY-001: gate, install and demonstrate the exact release, up to a staged, test-keyed install | DIRECTORY-030 |
| `docs/design/directory/briefs/DIRECTORY-030.md` | rendered markdown | DIRECTORY-030 |
| `scripts/identity-gates/release.sh` | the identity-release leg: refuses an unclean tree, a vendor/rauthy pin off ablative and an unpushed HEAD, then runs the six lys legs and prints one JSON line per leg | DIRECTORY-030 |
| `docs/design/identity/reports/IDENTITY-001-release.md` | row 07's release report: refs, venue legs, review, install, receipts, standalone acceptance, not met, reserved acts, status | DIRECTORY-030 |
| `docs/design/identity/reports/IDENTITY-001-commands.jsonl` | row 07's command record: one JSON line per command run for the release | DIRECTORY-030 |
| `docs/design/directory/briefs/DIRECTORY-031.json` | row 6.4: issuance entered in the lys-log-store log, the issuer certificate command and the stranger's proof | DIRECTORY-031 |
| `docs/design/directory/briefs/DIRECTORY-031.md` | rendered markdown | DIRECTORY-031 |
| `docs/design/identity/CAPABILITY-CLAIM.md` | the design round's proposal for lys/agent-capability/v1 under the proposed OID 1.3.6.1.4.1.66364.2.1: transport, assertion, encoding, issuer key identifier, scope, verifier check, rendering consumer, revocation, issuer and anchor, status | DIRECTORY-031 |
| `docs/design/identity/CAPABILITY-CLAIM-REVIEW.md` | the adversarial review of the capability-claim draft, by a party other than its author, before the owning lead's ratification with a second reader | DIRECTORY-031 |
| `docs/PEN-REGISTRATION.md` | the sub-arc register under 1.3.6.1.4.1.66364; its .2 row keeps .2 a family arc for agent-certificate extensions, names .2.1 for the typed capability claim lys/agent-capability/v1 and keeps the rest of its stated purpose; its status cell stays Reserved until the ratification is recorded and then reads In use (.2.1) (R3); the .1 and .3+ rows unchanged | DIRECTORY-031 |
| `crates/lys-identity/src/capability/mod.rs` | declarations and re-exports only | DIRECTORY-031 |
| `crates/lys-identity/src/capability/claim.rs` | the typed lys/agent-capability/v1 claim: holder id and every grant held at issuance, each with its grant id and its window as it stood then | DIRECTORY-031 |
| `crates/lys-identity/src/capability/encoding.rs` | canonical encode and strict decode of the claim: claim_malformed, claim_version_unknown | DIRECTORY-031 |
| `crates/lys-identity/src/capability/error.rs` | named capability errors, never key or seed bytes | DIRECTORY-031 |
| `crates/lys-identity/src/capability/verify.rs` | verify_agent_capability: the signing key from the trusted set first, then the Authority Key Identifier, the claim, holder, listed grant and the certificate's own window, each refusal named | DIRECTORY-031 |
| `crates/lys-identity/src/capability/issue.rs` | the directory's issuance of an enduring agent's one certificate carrying one claim and the Authority Key Identifier of its issuer key, appended to the certificate log as the revocation card's issuance leaf, refusing an agent that holds a current certificate, recorded as one directory event naming the certificate it follows | DIRECTORY-031 |
| `crates/lys-identity/tests/capability_encoding.rs` | the pinned vector and the counted decode refusals | DIRECTORY-031 |
| `crates/lys-identity/tests/capability_verify.rs` | the verifier's acceptance, counted refusal legs and drift injections | DIRECTORY-031 |
| `crates/lys-identity/tests/capability_issue.rs` | issuance, reissue after expiry and its counted refusals | DIRECTORY-031 |
| `crates/lys-identity/src/agent_key.rs` | an agent's one enrolled Ed25519 public key, keyed by its directory id beside the agent record, enrolled once by the operator | DIRECTORY-031 |
| `crates/lys-identity/tests/agent_key.rs` | enrolment and its counted refusals | DIRECTORY-031 |
| `crates/lys-identity-server/src/issuer_key.rs` | custody of the directory's issuer key: restricted private file, Zeroizing seed, redacted Debug | DIRECTORY-031 |
| `crates/lys-identity-server/tests/issuer_key.rs` | issuer key loading, counted refusals and redaction | DIRECTORY-031 |
| `crates/lys-identity-server/src/agent_key_route.rs` | the operator's enrol route POST /identity/agents/{agent_id}/key and its named refusals | DIRECTORY-031 |
| `crates/lys-identity-server/tests/agent_key_route.rs` | the enrol route's acceptance and counted refusals | DIRECTORY-031 |
| `crates/lys-identity-server/src/certificate_route.rs` | the directory's issuance route POST /identity/agents/{agent_id}/certificate: the operator's call with the agent's certificate-signing request, its named refusals | DIRECTORY-031 |
| `crates/lys-identity-server/tests/certificate_route.rs` | the issuance route's acceptance and counted refusals | DIRECTORY-031 |
| `crates/lys-identity/src/agent_key_replacement.rs` | the audited replacement of an agent's enrolled key, one event naming the old and new key, never an overwrite | DIRECTORY-031 |
| `crates/lys-identity/tests/agent_key_replacement.rs` | replacement, reissue under the new key and the counted refusals | DIRECTORY-031 |
| `crates/lys-identity-server/src/agent_key_replacement_route.rs` | the operator's replacement route POST /identity/agents/{agent_id}/key/replacement and its named refusals | DIRECTORY-031 |
| `crates/lys-identity-server/tests/agent_key_replacement_route.rs` | the replacement route's acceptance and counted refusals | DIRECTORY-031 |
| `docs/design/directory/PROOF-ISSUANCE.md` | the recorded stranger's check of an issued certificate and its log entry: commands, exit codes and output, hashes, counts and paths only | DIRECTORY-031 |
| `crates/lys/src/commands/ca.rs` | `lys ca request`, `issue` and `verify`; gains `issuer_cert` and the issuer certificate stored beside the CA key, and `issue` always carries a log entry (DIRECTORY-031) |  |
| `crates/lys/src/commands/ca_tests.rs` | unit tests of ca.rs |  |
| `crates/lys/src/commands/ca_log.rs` | entering an issued certificate in a lys-log-store log before anything is written; the issuer-only entry with no log key (DIRECTORY-031) |  |
| `crates/lys/src/commands/ca_log_tests.rs` | unit tests of ca_log.rs |  |
| `crates/lys/tests/ca_log_tests.rs` | `lys ca issue --log` integration tests; registers the ca_log modules |  |
| `crates/lys/tests/ca_log/support.rs` | shared bench: test keys, a test log, openssl and python3 resolved as hard requirements |  |
| `crates/lys/tests/ca_log/outputs.rs` | output refusals and the failure after the append |  |
| `crates/lys/tests/ca_log/issuer_cert.rs` | `lys ca issuer-cert`: the stored issuer certificate, byte identity with `--issuer-out`, and its refusals | DIRECTORY-031 |
| `crates/lys/tests/ca_log/issuer_only.rs` | issuance with the CA key only, the operator's artifact, the refusal, and the stranger's offline check with no lys on PATH | DIRECTORY-031 |
| `crates/lys/tests/cli_tests.rs` | CLI integration tests; its `ca issue` invocations name a test log |  |
| `crates/lys/tests/json_output_tests.rs` | --json output tests; its `ca issue` invocations name a test log |  |
| `crates/lys/tests/certified_attestation_tests.rs` | certified attestation tests; its `ca issue` invocations name a test log |  |
| `README.md` | the project README; its `lys ca issue` example names a log |  |
| `crates/lys-identity-server/src/grant_resources.rs` | The read-only route serving the permission model's resources with their parents, declared actions and model version, for the access graph (conformance 8.3); created on top of DIRECTORY-006, reconcile against its landed manifest before dispatch | DIRECTORY-034 |
| `crates/lys-identity-server/tests/grant_resources.rs` | The read-only route serving the permission model's resources with their parents, declared actions and model version, for the access graph (conformance 8.3); created on top of DIRECTORY-006, reconcile against its landed manifest before dispatch | DIRECTORY-034 |
| `surface/identity/src/features/graph/GraphScreen.tsx` | The access graph from Access's answers only (conformance 8.3); created on top of DIRECTORY-005 and DIRECTORY-006, reconcile against their landed manifests before dispatch | DIRECTORY-034 |
| `surface/identity/src/features/graph/graphModel.ts` | The access graph from Access's answers only (conformance 8.3); created on top of DIRECTORY-005 and DIRECTORY-006, reconcile against their landed manifests before dispatch | DIRECTORY-034 |
| `surface/identity/src/features/graph/recordEdges.ts` | The access graph from Access's answers only (conformance 8.3); created on top of DIRECTORY-005 and DIRECTORY-006, reconcile against their landed manifests before dispatch | DIRECTORY-034 |
| `surface/identity/src/features/graph/graphQuestions.ts` | The access graph from Access's answers only (conformance 8.3); created on top of DIRECTORY-005 and DIRECTORY-006, reconcile against their landed manifests before dispatch | DIRECTORY-034 |
| `surface/identity/src/features/graph/graphLayout.ts` | The access graph from Access's answers only (conformance 8.3); created on top of DIRECTORY-005 and DIRECTORY-006, reconcile against their landed manifests before dispatch | DIRECTORY-034 |
| `surface/identity/src/features/graph/ShowInGraphLink.tsx` | The access graph from Access's answers only (conformance 8.3); created on top of DIRECTORY-005 and DIRECTORY-006, reconcile against their landed manifests before dispatch | DIRECTORY-034 |
| `surface/identity/tests/graph_boundary.test.ts` | The access graph from Access's answers only (conformance 8.3); created on top of DIRECTORY-005 and DIRECTORY-006, reconcile against their landed manifests before dispatch | DIRECTORY-034 |
| `surface/identity/tests/graph.test.tsx` | The access graph from Access's answers only (conformance 8.3); created on top of DIRECTORY-005 and DIRECTORY-006, reconcile against their landed manifests before dispatch | DIRECTORY-034 |
| `surface/identity/tests/acceptance/graph.spec.ts` | The access graph from Access's answers only (conformance 8.3); created on top of DIRECTORY-005 and DIRECTORY-006, reconcile against their landed manifests before dispatch | DIRECTORY-034 |
| `surface/identity/tests/fixtures/graph-directory.ts` | The access graph from Access's answers only (conformance 8.3); created on top of DIRECTORY-005 and DIRECTORY-006, reconcile against their landed manifests before dispatch | DIRECTORY-034 |
| `surface/identity/src/features/people/Preview.tsx` | The directory's preview drawer (DIRECTORY-005); DIRECTORY-034 adds its Show in graph link. Named at the path the identity surface was built at; reconcile against DIRECTORY-005's landed manifest before dispatch | DIRECTORY-034 |
| `surface/identity/src/features/file/IdentityFile.tsx` | The identity record screen (DIRECTORY-005); DIRECTORY-034 adds its Show in graph link. Named at the path the identity surface was built at; reconcile against DIRECTORY-005's landed manifest before dispatch | DIRECTORY-034 |
| `.gitignore` | DIRECTORY-034 adds two entries, crates/lys-identity-server/tests/grant_resources.ids.json and surface/identity/tests/fixtures/graph-directory.ids.json: the access graph tests' label-to-id maps, uncommitted run outputs each test also removes before it exits | DIRECTORY-034 |
| `docs/design/directory/briefs/DIRECTORY-034.json` | the access graph brief (conformance 8.3) | DIRECTORY-034 |
| `docs/design/directory/briefs/DIRECTORY-034.md` | rendered markdown | DIRECTORY-034 |
| `crates/lys-identity/tests/grant_last_used.rs` | the last-used tests of conformance 8.4, named by their row under DIRECTORY-035 | DIRECTORY-035 |
| `surface/identity/src/generated/grants.ts` | the grant wire types the screens compile against, mirroring views.rs; gains the two effective fields | DIRECTORY-035 |
| `surface/identity/src/features/grants/model.ts` | the grant screens' reading of the service's answers; loses the client standing walk | DIRECTORY-035 |
| `surface/identity/src/features/grants/GrantCard.tsx` | one grant with its chain, window, last use and standing (conformance 8.4) | DIRECTORY-035 |
| `surface/identity/src/features/grants/Delegate.tsx` | the delegation form (conformance 2.3): source grant, actions, may-pass-on and the end bound | DIRECTORY-035 |
| `surface/identity/src/features/me/You.tsx` | the You page: What you hold (conformance 1.4) and the personal scope of conformance 1.5 | DIRECTORY-035 |
| `surface/identity/src/features/access/Access.tsx` | the access screens that list grants with their standing and last use | DIRECTORY-035 |
| `surface/identity/src/features/file/sections.tsx` | an identity file's sections that read which grants stand | DIRECTORY-035 |
| `surface/identity/tests/fixtures.ts` | the vitests' fixture service answers | DIRECTORY-035 |
| `surface/identity/tests/me.test.tsx` | the You vitests, carrying row 1.5's test | DIRECTORY-035 |
| `surface/identity/tests/revoke.test.tsx` | the Revoke vitests, carrying row 2.5's screen test | DIRECTORY-035 |
| `docs/design/directory/briefs/DIRECTORY-037.json` | the identity surface shell brief: the route table, the two keyboard gaps closed in the shell and the mock-up, and the gate leg | DIRECTORY-037 |
| `docs/design/directory/briefs/DIRECTORY-037.md` | rendered markdown | DIRECTORY-037 |
| `surface/identity/package.json` | the surface package, arriving with pull request 35 (DIRECTORY-005); DIRECTORY-037 adds @testing-library/user-event 14.6.7 and its peer @testing-library/dom 10.4.2 as development dependencies | DIRECTORY-005 |
| `surface/identity/package-lock.json` | the surface lock file, arriving with pull request 35 (DIRECTORY-005); follows package.json | DIRECTORY-005 |
| `surface/identity/src/shell/keyable.ts` | makes a non-button click action focusable and answer Enter and Space; arrives with pull request 35, DIRECTORY-037 keeps keyable(activate) and makes Enter and Space dispatch the click that runs it | DIRECTORY-005 |
| `surface/identity/src/shell/Palette.tsx` | the command palette; arrives with pull request 35, DIRECTORY-037 makes its rows keyboard-reachable | DIRECTORY-005 |
| `surface/identity/src/shell/keys.ts` | the shell key registry; arrives with pull request 35, DIRECTORY-037 makes j and k move focus and Enter open the focused row | DIRECTORY-005 |
| `surface/identity/src/shell/routeTable.ts` | the one table of every screen and tab route, with its built column | DIRECTORY-037 |
| `surface/identity/tests/routes.test.tsx` | route table checks: the 44 rows, built rows followed, drawn hashes matched | DIRECTORY-037 |
| `surface/identity/tests/rail.test.tsx` | rail labels and dock side, by control and key, kept across a reload | DIRECTORY-037 |
| `surface/identity/tests/goto.test.tsx` | palette Go to entries and g go-to letters reach every screen | DIRECTORY-037 |
| `surface/identity/tests/keyable.test.tsx` | Enter and Space on click actions and palette rows | DIRECTORY-037 |
| `surface/identity/tests/cursor.test.tsx` | j and k move focus; Enter opens the focused row | DIRECTORY-037 |
| `surface/identity/tests/overlay.test.tsx` | the help overlay's count, dismissal and focus return | DIRECTORY-037 |
| `surface/identity/tests/walk.test.tsx` | the counted Tab walk over every built route | DIRECTORY-037 |
| `surface/identity/tests/mockup.test.tsx` | index.v6.html's two fixes, proved on v6 and shown failing on v5 | DIRECTORY-037 |
| `docs/design/identity/mockup/index.v6.html` | the mock-up with the focus fix and the keyboard fix, beside v5 (ADR-104) | DIRECTORY-037 |
| `docs/design/directory/briefs/DIRECTORY-038.json` | conformance row 1.2: sign-in identities belong to people only, refused by name at every directory act that would give one to an agent | DIRECTORY-038 |
| `docs/design/directory/briefs/DIRECTORY-038.md` | rendered markdown | DIRECTORY-038 |
| `crates/lys-identity/src/bindings.rs` | DIRECTORY-003 R1's binding module, reconciled against its reviewed manifest before dispatch: gains the refusal of a person's provider account as an agent's binding | DIRECTORY-038 |
| `crates/lys-identity/tests/sign_in_store.rs` | after the two directory refusals, no agent record carries a sign-in identity, counted over the store | DIRECTORY-038 |
| `docs/design/directory/briefs/DIRECTORY-035.json` | the grant conformance rows of the directory, each named to the test that passes it | DIRECTORY-035 |
| `docs/design/directory/briefs/DIRECTORY-035.md` | rendered markdown | DIRECTORY-035 |
| `docs/design/directory/briefs/DIRECTORY-039.json` | rendered brief list fields: the sync of the method's render-brief.py, its test, and the re-render of every rendered brief | DIRECTORY-039 |
| `docs/design/directory/briefs/DIRECTORY-039.md` | rendered markdown | DIRECTORY-039 |
| `scripts/design/render-brief.py` | the brief renderer, a copy of the design-system method's scripts/render-brief.py; re-copied at the method commit that holds the prose-list rule (DIRECTORY-039) | DIRECTORY-039 |
| `scripts/design/SOURCE.md` | where the design scripts are copied from; names the method commit render-brief.py is copied at (DIRECTORY-039) | DIRECTORY-039 |
| `scripts/design/gate.sh` | the design leg; runs the renderer test beside validate, coverage and the render cmp (DIRECTORY-039) | DIRECTORY-039 |
| `scripts/design/tests/test_render_brief.py` | stdlib unittest of the renderer's prose-list and bare-id list rendering | DIRECTORY-039 |
| `docs/design/home/briefs/HOME-001.md` | rendered markdown; re-rendered with blocked_by as a list (DIRECTORY-039) | DIRECTORY-039 |
| `docs/design/home/briefs/HOME-003.md` | rendered markdown; re-rendered with blocked_by as a list (DIRECTORY-039) | DIRECTORY-039 |
| `docs/design/home/briefs/HOME-006.md` | rendered markdown; re-rendered with blocked_by as a list (DIRECTORY-039) | DIRECTORY-039 |
| `docs/design/secrets/briefs/SECRETS-002.md` | rendered markdown; re-rendered with blocked_by as a list (DIRECTORY-039) | DIRECTORY-039 |
| `crates/lys-identity/src/grants/commit.rs` | touched by DIRECTORY-042 R4: Use events do not round-trip the mirror one by one | DIRECTORY-042 |
| `crates/lys-identity-server/src/spicedb.rs` | touched by DIRECTORY-042 R7: The SpiceDB schema is read when it can have changed, not before every call | DIRECTORY-042 |
| `crates/lys-identity-server/src/spicedb_http.rs` | touched by DIRECTORY-042 R8: SpiceDB calls use a pooled async client, outside the directory lock, with no timeout | DIRECTORY-042 |
| `crates/lys-identity-server/src/runtime_state.rs` | touched by DIRECTORY-043 R1: Runtime state is indexed | DIRECTORY-043 |
| `crates/lys-identity-server/src/runtime_store.rs` | touched by DIRECTORY-043 R1: Runtime state is indexed | DIRECTORY-043 |
| `crates/lys-identity-server/src/requests_state.rs` | touched by DIRECTORY-043 R2: Snapshots serialise by reference | DIRECTORY-043 |
| `crates/lys-identity-server/src/certificates_store.rs` | touched by DIRECTORY-043 R2: Snapshots serialise by reference | DIRECTORY-043 |
| `crates/lys-identity-server/src/teams_state.rs` | touched by DIRECTORY-043 R2: Snapshots serialise by reference | DIRECTORY-043 |
| `crates/lys-identity-server/src/stops_state.rs` | touched by DIRECTORY-043 R2: Snapshots serialise by reference | DIRECTORY-043 |
| `crates/lys-identity-server/src/service_accounts_state.rs` | touched by DIRECTORY-043 R2: Snapshots serialise by reference | DIRECTORY-043 |
| `crates/lys-identity-server/src/reviews_state.rs` | touched by DIRECTORY-043 R2: Snapshots serialise by reference | DIRECTORY-043 |
| `crates/lys-identity-server/src/runtime_api.rs` | touched by DIRECTORY-043 R4: A session listing locks once and clones nothing it does not return | DIRECTORY-043 |
| `crates/lys-identity-server/src/network_store.rs` | touched by DIRECTORY-043 R4: A session listing locks once and clones nothing it does not return | DIRECTORY-043 |
| `crates/lys-identity-server/src/launch_api.rs` | touched by DIRECTORY-043 R4: A session listing locks once and clones nothing it does not return | DIRECTORY-043 |
| `crates/lys-identity-server/src/memory_api.rs` | touched by DIRECTORY-043 R5: A memory view reads each session once, off the async worker | DIRECTORY-043 |
| `crates/lys-home/src/recall.rs` | touched by DIRECTORY-043 R5: A memory view reads each session once, off the async worker | DIRECTORY-043 |
| `crates/lys-home/src/given.rs` | touched by DIRECTORY-043 R5: A memory view reads each session once, off the async worker | DIRECTORY-043 |
| `crates/lys-identity/src/restart.rs` | touched by DIRECTORY-043 R6: A receipt is rebuilt from the stored coordinate, not by re-reading leaves | DIRECTORY-043 |
| `crates/lys-identity-server/src/read_api.rs` | touched by DIRECTORY-043 R6: A receipt is rebuilt from the stored coordinate, not by re-reading leaves | DIRECTORY-043 |
| `crates/lys-identity/src/directory.rs` | touched by DIRECTORY-043 R6: A receipt is rebuilt from the stored coordinate, not by re-reading leaves | DIRECTORY-043 |
| `crates/lys/src/identity/install/services.rs` | DIRECTORY-044 R1: Waiting for a service to answer waits on its readiness event, not a one-second sleep | DIRECTORY-044 |
| `crates/lys/src/identity/install/exit_wait.rs` | DIRECTORY-044 R2: Stopping a service waits on the platform's exit notification | DIRECTORY-044 |
| `crates/lys/src/identity/loopback_http.rs` | DIRECTORY-044 R3: Loopback, Rauthy and health exchanges carry no timeout | DIRECTORY-044 |
| `surface/identity/src/shell/Explain.tsx` | DIRECTORY-044 R4: The Explain view re-measures on a layout signal | DIRECTORY-044 |
| `crates/lys/build.rs` | DIRECTORY-045 R1: Every Lys binary answers --version with its build commit | DIRECTORY-045 |
| `crates/lys-secrets/build.rs` | DIRECTORY-045 R1: Every Lys binary answers --version with its build commit | DIRECTORY-045 |
| `crates/lys-home/build.rs` | DIRECTORY-045 R1: Every Lys binary answers --version with its build commit | DIRECTORY-045 |
| `crates/lys-identity-server/src/main.rs` | DIRECTORY-045 R1: Every Lys binary answers --version with its build commit | DIRECTORY-045 |
| `crates/lys-secrets/src/bin/lys-secrets/main.rs` | DIRECTORY-045 R1: Every Lys binary answers --version with its build commit | DIRECTORY-045 |
| `crates/lys-home/src/main.rs` | DIRECTORY-045 R1: Every Lys binary answers --version with its build commit | DIRECTORY-045 |
| `crates/lys/src/identity/upgrade.rs` | DIRECTORY-045 R2: `lys identity upgrade` swaps the binaries and screens and returns to the previous build on failure | DIRECTORY-045 |
| `crates/lys/src/identity/upgrade_tests.rs` | DIRECTORY-045 R2: `lys identity upgrade` swaps the binaries and screens and returns to the previous build on failure | DIRECTORY-045 |
| `crates/lys/src/identity/install/layout.rs` | DIRECTORY-045 R2: `lys identity upgrade` swaps the binaries and screens and returns to the previous build on failure | DIRECTORY-045 |
| `crates/lys/src/identity/install.rs` | DIRECTORY-045 R3: The install and the server say which build is running | DIRECTORY-045 |
| `surface/identity/src/shell/About.tsx` | DIRECTORY-045 R3: The install and the server say which build is running | DIRECTORY-045 |
| `docs/design/directory/PROOF-UPGRADE.md` | DIRECTORY-045 R4: Prove it on the live install | DIRECTORY-045 |
| `crates/lys-identity-server/src/reviews_api.rs` | DIRECTORY-046 R1: GET /reviews filters by viewer first and finds each latest decision by index | DIRECTORY-046 |
| `crates/lys-identity-server/src/requests_store.rs` | DIRECTORY-046 R2: A request is found by id, not by scanning every request | DIRECTORY-046 |
| `crates/lys-identity-server/src/certificates_issue.rs` | DIRECTORY-046 R3: Certificate issue uses the service key the server already holds | DIRECTORY-046 |
| `crates/lys-identity-server/src/surface.rs` | DIRECTORY-046 R4: Served screens never block an async worker and index.html is read once | DIRECTORY-046 |
| `surface/identity/src/features/grants/check.ts` | DIRECTORY-046 R6: The grants screens ask for reach in one concurrent batch | DIRECTORY-046 |
| `surface/identity/src/api.ts` | DIRECTORY-046 R7: The screens choose the right route first and fetch independent reads together | DIRECTORY-046 |
| `sgconfig.yml` | DIRECTORY-048 R7: the ast-grep configuration; gains the rule that refuses an app named in Lys code | DIRECTORY-048 |
| `crates/lys/src/cli/mcp.rs` | DIRECTORY-049 R7: the Lys MCP server over standard input and output: three tools that cover the whole published API | DIRECTORY-049 |
| `crates/lys/tests/mcp_stdio.rs` | DIRECTORY-049 R7: the MCP server driven over standard input and output, where the caller is always an agent, so each tool call carries the agent's signature; a person's token is only for the /mcp route over the network | DIRECTORY-049 |
| `crates/lys-runner/Cargo.toml` | DIRECTORY-050 R1: Lys's own runner: each agent in its own background pseudo-terminal | DIRECTORY-050 |
| `crates/lys-runner/src/lib.rs` | DIRECTORY-050 R1: Lys's own runner: each agent in its own background pseudo-terminal | DIRECTORY-050 |
| `crates/lys-runner/src/pty.rs` | DIRECTORY-050 R1: Lys's own runner: each agent in its own background pseudo-terminal | DIRECTORY-050 |
| `crates/lys-runner/src/session.rs` | DIRECTORY-050 R1: Lys's own runner: each agent in its own background pseudo-terminal | DIRECTORY-050 |
| `crates/lys-runner/src/socket.rs` | DIRECTORY-050 R1: Lys's own runner: each agent in its own background pseudo-terminal | DIRECTORY-050 |
| `crates/lys-runner/tests/runner.rs` | DIRECTORY-050 R1: Lys's own runner: each agent in its own background pseudo-terminal | DIRECTORY-050 |
| `crates/lys/src/cli/runner.rs` | DIRECTORY-050 R1: Lys's own runner: each agent in its own background pseudo-terminal | DIRECTORY-050 |
| `crates/lys-runner/src/protocol.rs` | DIRECTORY-050 R2: A published runner protocol; any tool may be the runner | DIRECTORY-050 |
| `crates/lys-runner/tests/conformance.rs` | DIRECTORY-050 R2: A published runner protocol; any tool may be the runner | DIRECTORY-050 |
| `crates/lys-identity-server/src/runner_client.rs` | DIRECTORY-050 R2: A published runner protocol; any tool may be the runner | DIRECTORY-050 |
| `crates/lys-identity-server/tests/runner_start.rs` | DIRECTORY-050 R3: Start runs the agent | DIRECTORY-050 |
| `crates/lys-identity-server/src/runner_api.rs` | DIRECTORY-050 R4: Type, keys, read, wait, resize and compact through Lys | DIRECTORY-050 |
| `crates/lys-identity-server/tests/runner_api.rs` | DIRECTORY-050 R4: Type, keys, read, wait, resize and compact through Lys | DIRECTORY-050 |
| `crates/lys-runner/src/rotation.rs` | DIRECTORY-050 R5: Account rotation on usage limit | DIRECTORY-050 |
| `surface/identity/src/features/runtime/Terminal.tsx` | DIRECTORY-050 R7: Sessions screen: see, type to and stop every agent | DIRECTORY-050 |
| `surface/identity/src/features/runtime/terminal.css` | DIRECTORY-050 R7: Sessions screen: see, type to and stop every agent | DIRECTORY-050 |
| `crates/lys-identity-server/src/usage_api.rs` | DIRECTORY-051 R1: Usage measured from the transcript, per turn | DIRECTORY-051 |
| `crates/lys-identity-server/src/usage_state.rs` | DIRECTORY-051 R1: Usage measured from the transcript, per turn | DIRECTORY-051 |
| `crates/lys-identity-server/src/usage_store.rs` | DIRECTORY-051 R1: Usage measured from the transcript, per turn | DIRECTORY-051 |
| `crates/lys-identity-server/tests/usage.rs` | DIRECTORY-051 R1: Usage measured from the transcript, per turn | DIRECTORY-051 |
| `crates/lys-identity-server/src/budgets_api.rs` | DIRECTORY-051 R2: Budgets on agents, teams and people | DIRECTORY-051 |
| `crates/lys-identity-server/src/budgets_state.rs` | DIRECTORY-051 R2: Budgets on agents, teams and people | DIRECTORY-051 |
| `crates/lys-identity-server/tests/budgets.rs` | DIRECTORY-051 R2: Budgets on agents, teams and people | DIRECTORY-051 |
| `crates/lys-identity-server/src/budgets_act.rs` | DIRECTORY-051 R3: A reached budget acts once | DIRECTORY-051 |
| `crates/lys-identity-server/src/goals_api.rs` | DIRECTORY-051 R4: Goals with deadlines and reminders | DIRECTORY-051 |
| `crates/lys-identity-server/src/goals_state.rs` | DIRECTORY-051 R4: Goals with deadlines and reminders | DIRECTORY-051 |
| `crates/lys-identity-server/tests/goals.rs` | DIRECTORY-051 R4: Goals with deadlines and reminders | DIRECTORY-051 |
| `surface/identity/src/features/usage/Usage.tsx` | DIRECTORY-051 R5: Usage screen | DIRECTORY-051 |
| `surface/identity/src/features/usage/usage.css` | DIRECTORY-051 R5: Usage screen | DIRECTORY-051 |
| `surface/identity/src/features/apps/SchemaBuilder.tsx` | DIRECTORY-048 R8: the permission schema builder | DIRECTORY-048 |
| `surface/identity/src/features/apps/schema-builder.css` | DIRECTORY-048 R8: the permission schema builder | DIRECTORY-048 |
| `surface/identity/tests/schema-builder.test.tsx` | DIRECTORY-048 R8: the permission schema builder | DIRECTORY-048 |
| `crates/lys-identity-server/src/team_plans_api.rs` | R1: A team plan record | DIRECTORY-052 |
| `crates/lys-identity-server/src/team_plans_state.rs` | R1: A team plan record | DIRECTORY-052 |
| `crates/lys-identity-server/tests/team_plans.rs` | R1: A team plan record | DIRECTORY-052 |
| `crates/lys-identity-server/src/team_plans_provision.rs` | R2: Provision in one all-or-nothing act | DIRECTORY-052 |
| `surface/identity/src/features/team-plans/TeamPlans.tsx` | R5: Teams screen | DIRECTORY-052 |
| `surface/identity/src/features/team-plans/team-plans.css` | R5: Teams screen | DIRECTORY-052 |
| `surface/identity/src/features/apps/SchemaBench.tsx` | R8: Build an app's permission template on a Lys screen | DIRECTORY-048 |
| `crates/lys-home/src/record/given.rs` | DIRECTORY-052 R3: the given record lists a member's starting memories and opening conversation | DIRECTORY-052 |
| `crates/lys/src/identity/upgrade_back_tests.rs` | R1: `lys identity upgrade --back` | DIRECTORY-053 |
| `crates/lys-build-stamp/Cargo.toml` | R2: One build stamp | DIRECTORY-053 |
| `crates/lys-build-stamp/src/lib.rs` | R2: One build stamp | DIRECTORY-053 |
| `crates/lys-build-stamp/tests/stamp.rs` | R2: One build stamp | DIRECTORY-053 |
| `crates/lys-secrets/Cargo.toml` | R2: One build stamp | DIRECTORY-053 |
| `crates/lys-home/Cargo.toml` | R2: One build stamp | DIRECTORY-053 |
| `crates/lys/tests/version.rs` | R2: One build stamp | DIRECTORY-053 |
| `crates/lys/src/identity/upgrade/render.rs` | R5: The configuration and compose files move with the binaries | DIRECTORY-045 |
| `crates/lys/src/identity/upgrade/render_tests.rs` | R5: The configuration and compose files move with the binaries | DIRECTORY-045 |
| `crates/lys/src/identity/install/server_config.rs` | R5: The configuration and compose files move with the binaries | DIRECTORY-045 |
| `crates/lys/src/identity/upgrade/adopt.rs` | R6: The first move of an install made before this card | DIRECTORY-045 |
| `crates/lys/src/identity/upgrade/adopt_tests.rs` | R6: The first move of an install made before this card | DIRECTORY-045 |
| `crates/lys/src/identity/upgrade/intent.rs` | R7: An upgrade stopped part-way is finished or put back | DIRECTORY-045 |
| `crates/lys/src/identity/upgrade/intent_tests.rs` | R7: An upgrade stopped part-way is finished or put back | DIRECTORY-045 |
| `crates/lys/src/identity/upgrade/swap.rs` | R7: An upgrade stopped part-way is finished or put back | DIRECTORY-045 |
| `crates/lys-identity-server/src/apps_api.rs` | R1: An app is a record, registered through the API and approved on a Lys screen | DIRECTORY-048 |
| `crates/lys-identity-server/src/apps_state.rs` | R1: An app is a record, registered through the API and approved on a Lys screen | DIRECTORY-048 |
| `crates/lys-identity-server/src/apps_store.rs` | R1: An app is a record, registered through the API and approved on a Lys screen | DIRECTORY-048 |
| `crates/lys-identity-server/tests/apps.rs` | R1: An app is a record, registered through the API and approved on a Lys screen | DIRECTORY-048 |
| `surface/identity/src/features/apps/Apps.tsx` | R1: An app is a record, registered through the API and approved on a Lys screen | DIRECTORY-048 |
| `surface/identity/src/features/apps/apps.css` | R1: An app is a record, registered through the API and approved on a Lys screen | DIRECTORY-048 |
| `crates/lys-identity/src/grants/schema.rs` | R2: An app's schema: its own kinds, actions, relations and parents, checked on entry | DIRECTORY-048 |
| `crates/lys-identity/src/grants/schema_tests.rs` | R2: An app's schema: its own kinds, actions, relations and parents, checked on entry | DIRECTORY-048 |
| `crates/lys-identity-server/src/apps_binding.rs` | R2: An app's schema: its own kinds, actions, relations and parents, checked on entry | DIRECTORY-048 |
| `crates/lys-identity/src/grants/model.rs` | R2: An app's schema: its own kinds, actions, relations and parents, checked on entry | DIRECTORY-048 |
| `crates/lys-identity-server/tests/apps_schema.rs` | R3: Schema changes are versioned, dry-run first, and never strand a grant | DIRECTORY-048 |
| `crates/lys-identity-server/tests/grants_batch.rs` | R5: Apps check many permissions at once and list what a subject may act on | DIRECTORY-048 |
| `crates/lys-identity-server/src/openapi.rs` | R6: One OpenAPI document, generated, describes every route | DIRECTORY-048 |
| `crates/lys-identity-server/tests/openapi.rs` | R6: One OpenAPI document, generated, describes every route | DIRECTORY-048 |
| `crates/lys-openapi/Cargo.toml` | R6: One OpenAPI document, generated, describes every route | DIRECTORY-048 |
| `crates/lys-openapi/src/lib.rs` | R6: One OpenAPI document, generated, describes every route | DIRECTORY-048 |
| `crates/lys-identity-server/src/error_status.rs` | R6: One OpenAPI document, generated, describes every route | DIRECTORY-048 |
| `rules/ast-grep/no-app-names.yml` | R7: Lys depends on no app | DIRECTORY-048 |
| `crates/lys-core/src/attestation/mod.rs` | R7: Lys depends on no app | DIRECTORY-048 |
| `crates/lys/src/identity/install/deployment.template.toml` | R7: Lys depends on no app | DIRECTORY-048 |
| `surface/styles/tokens.css` | R7: Lys depends on no app | DIRECTORY-048 |
| `crates/lys-identity-server/src/receipts_api.rs` | R4: Type, keys, read, wait, resize and compact through Lys | DIRECTORY-050 |
| `crates/lys-identity/src/provisioning.rs` | R5: Account rotation on usage limit | DIRECTORY-050 |
| `crates/lys-identity-server/src/stop_api.rs` | R6: Wake with a message; stop through the runner | DIRECTORY-050 |
| `surface/identity/src/features/runtime/Sessions.tsx` | R7: Sessions screen: see, type to and stop every agent | DIRECTORY-050 |
| `rules/ast-grep/no-poll.yml` | R1: Usage measured from the transcript, per turn | DIRECTORY-051 |
| `crates/lys-identity-server/src/setup.rs` | R1: First run is a Lys setup page that asks for the administrator; install fills nothing from the machine | DIRECTORY-047 |
| `surface/identity/src/features/setup/Setup.tsx` | R1: First run is a Lys setup page that asks for the administrator; install fills nothing from the machine | DIRECTORY-047 |
| `crates/lys-identity-server/src/sign_in.rs` | R2: Password sign-in is a Lys page; the browser never reaches the issuer's pages | DIRECTORY-047 |
| `surface/identity/src/features/sign-in/SignIn.tsx` | R2: Password sign-in is a Lys page; the browser never reaches the issuer's pages | DIRECTORY-047 |
| `crates/lys-identity-server/src/provider.rs` | R3: Lys is the OpenID provider every product is registered with | DIRECTORY-047 |
| `crates/lys-identity-server/src/oidc.rs` | R3: Lys is the OpenID provider every product is registered with | DIRECTORY-047 |
| `crates/lys-identity-server/src/sign_in_providers.rs` | R4: Providers are set up inside Lys with every step shown | DIRECTORY-047 |
| `surface/identity/src/features/connections/SignInProviders.tsx` | R4: Providers are set up inside Lys with every step shown | DIRECTORY-047 |
| `surface/identity/src/features/people/Account.tsx` | R5: A person's own account, and administration of accounts, are Lys screens | DIRECTORY-047 |
| `crates/lys-identity-server/src/accounts.rs` | R5: A person's own account, and administration of accounts, are Lys screens | DIRECTORY-047 |
| `crates/lys/src/identity/install/prepare.rs` | R5: A person's own account, and administration of accounts, are Lys screens | DIRECTORY-047 |
| `crates/lys-identity-server/src/error.rs` | R6: Nothing a person reads names the issuer | DIRECTORY-047 |
| `surface/identity/src` | R6: Nothing a person reads names the issuer | DIRECTORY-047 |
| `crates/lys/src/identity/install/held_start.rs` | R2: The holder opens the lock, takes it and becomes the service | DIRECTORY-057 |
| `crates/lys/src/identity/install/held_start_tests.rs` | R2: The holder opens the lock, takes it and becomes the service | DIRECTORY-057 |
| `crates/lys/src/identity/install/start_race_tests.rs` | R3: The starter never opens the exit lock | DIRECTORY-057 |
| `crates/lys-identity-server/src/checkpoint_api.rs` | R1: The receipts route answers a signed checkpoint | DIRECTORY-058 |
| `crates/lys-identity-server/tests/receipts_signed.rs` | R1: The receipts route answers a signed checkpoint | DIRECTORY-058 |
| `crates/lys-identity/src/receipt_answer.rs` | R2: One function verifies a receipt against a pinned key | DIRECTORY-058 |
| `crates/lys-identity/src/receipt_answer_tests.rs` | R2: One function verifies a receipt against a pinned key | DIRECTORY-058 |
| `crates/lys-identity-server/src/secrets_api.rs` | R6: Accounts and secrets stored and reached by handle | DIRECTORY-052 |
| `crates/lys-identity-server/src/verified_caller.rs` | The one extractor that turns a verified request into the caller every handler takes | DIRECTORY-049 |
| `crates/lys-identity-server/tests/verified_caller.rs` | Proof that only the edges build a verified caller | DIRECTORY-049 |
| `rules/ast-grep/no-handler-verification.yml` | Refuses a handler that reads its caller from headers | DIRECTORY-049 |
| `crates/lys/src/identity/config/validate.rs` | Emit an explicit cross-repository sender configuration contract |  |
| `crates/lys-core/src/ca/mod.rs` | Export the separate installation TLS authority |  |
| `crates/lys-identity/src/link_audit_config.rs` | Emit an explicit cross-repository sender configuration contract | DIRECTORY-055 |
| `crates/lys-identity/tests/link_audit_config.rs` | Emit an explicit cross-repository sender configuration contract | DIRECTORY-055 |
| `crates/lys-core/src/ca/tls_authority.rs` | Serve audit traffic with a separate installation TLS authority | DIRECTORY-055 |
| `crates/lys-core/src/ca/tls_authority_tests.rs` | Serve audit traffic with a separate installation TLS authority | DIRECTORY-055 |
| `crates/lys-identity-server/src/tls.rs` | Serve audit traffic with a separate installation TLS authority | DIRECTORY-055 |
| `crates/lys-identity-server/tests/tls.rs` | Serve audit traffic with a separate installation TLS authority | DIRECTORY-055 |
| `crates/lys/src/identity/install/link_audit_files.rs` | Publish private provisioning files and read-only mounts | DIRECTORY-055 |
| `crates/lys/src/identity/install/link_audit_files_tests.rs` | Publish private provisioning files and read-only mounts | DIRECTORY-055 |
| `crates/lys/src/identity/install/link_audit.rs` | Browser setup completes the sender enrolment under the new person | DIRECTORY-055 |
| `crates/lys/src/identity/install/link_audit_tests.rs` | Browser setup completes the sender enrolment under the new person | DIRECTORY-055 |
| `crates/lys-identity-server/src/link_audit_provisioning.rs` | Browser setup completes the sender enrolment under the new person | DIRECTORY-055 |
| `crates/lys-identity-server/src/link_audit_provisioning_tests.rs` | Browser setup completes the sender enrolment under the new person | DIRECTORY-055 |
| `surface/identity/src/features/setup/SetupAudit.test.tsx` | Browser setup completes the sender enrolment under the new person | DIRECTORY-055 |
| `crates/lys-identity-server/tests/setup.rs` | Browser setup completes the sender enrolment under the new person |  |
| `crates/lys/tests/identity_link_audit_install.rs` | Prove provisioning through a scratch installation and upgrade | DIRECTORY-055 |
| `crates/lys/tests/identity_support/link_audit_install.rs` | Prove provisioning through a scratch installation and upgrade | DIRECTORY-055 |
| `crates/lys/src/identity/upgrade/back.rs` | `lys identity upgrade --back`: the exchange of bin/, configuration, compose and surface with the kept build, atomic where both folders exist, undoing its own finished renames on failure, and finishing an interrupted exchange from the intent record. | DIRECTORY-053 |
| `crates/lys-secrets/src/audit/codec.rs` | The broker's audit log codec: its kinds are one list the decoder matches, entered in the workspace kinds registry so go-back can compare kind sets. | DIRECTORY-053 |
| `crates/lys/src/identity/install/exit_wait_tests.rs` | Exit watch tests, calling start_detached with the holder. | DIRECTORY-057 |
| `crates/lys/src/identity/install_tests.rs` | Install tests, calling start_detached with the holder. | DIRECTORY-057 |
| `crates/lys-identity-server/src/broker_handles.rs` | R1: A start reads its credentials from Lys's own broker, and no route reaches an app for them | DIRECTORY-059 |
| `crates/lys-identity-server/src/broker_handles_tests.rs` | R1: A start reads its credentials from Lys's own broker, and no route reaches an app for them | DIRECTORY-059 |
| `crates/lys-identity-server/tests/start_with_own_broker.rs` | R1: A start reads its credentials from Lys's own broker, and no route reaches an app for them | DIRECTORY-059 |
| `crates/lys-secrets/src/access.rs` | R1: A start reads its credentials from Lys's own broker, and no route reaches an app for them | DIRECTORY-059 |
| `crates/lys-secrets/src/bin/lys-secrets/callers.rs` | R1: A start reads its credentials from Lys's own broker, and no route reaches an app for them | DIRECTORY-059 |
| `crates/lys/src/identity/install/discovery.rs` | R2: Lys writes a discovery record any app can find | DIRECTORY-059 |
| `crates/lys/src/identity/install/discovery_tests.rs` | R2: Lys writes a discovery record any app can find | DIRECTORY-059 |
| `crates/lys-identity-server/src/app_members_api.rs` | R3: Any registered app reads Lys's people, seats and agents | DIRECTORY-059 |
| `crates/lys-identity-server/tests/app_members.rs` | R3: Any registered app reads Lys's people, seats and agents | DIRECTORY-059 |
| `crates/lys-identity-server/src/backchannel_logout.rs` | R4: Signing out of Lys signs the person out of every registered app | DIRECTORY-059 |
| `crates/lys-identity-server/tests/backchannel_logout.rs` | R4: Signing out of Lys signs the person out of every registered app | DIRECTORY-059 |
| `crates/lys-identity-server/src/sessions_api.rs` | R4: Signing out of Lys signs the person out of every registered app | DIRECTORY-059 |
| `crates/lys-secrets/tests/held_states.rs` | A start reads its credentials from Lys's own broker, and no route reaches an app for them | DIRECTORY-059 |
| `crates/lys-secrets/src/bin/lys-secrets/serve.rs` | A start reads its credentials from Lys's own broker, and no route reaches an app for them | DIRECTORY-059 |
| `crates/lys-secrets/src/bin/lys-secrets/view.rs` | A start reads its credentials from Lys's own broker, and no route reaches an app for them | DIRECTORY-059 |
| `crates/lys-identity-server/src/issuer_sign_out.rs` | Signing out of Lys ends the person's sessions at the issuer, and the issuer tells every registered app | DIRECTORY-059 |
| `crates/lys-identity-server/src/apps_connect.rs` | An app asks for its registration, an administrator approves it on one screen, and the app receives its secret by a one-time code | DIRECTORY-059 |
| `crates/lys-identity-server/tests/apps_connect.rs` | An app asks for its registration, an administrator approves it on one screen, and the app receives its secret by a one-time code | DIRECTORY-059 |
| `surface/identity/src/features/apps/ConnectRequest.tsx` | An app asks for its registration, an administrator approves it on one screen, and the app receives its secret by a one-time code | DIRECTORY-059 |
| `crates/lys-identity-server/src/session.rs` | Signing out of Lys signs the person out at the issuer, and the issuer tells every registered app | DIRECTORY-059 |
| `surface/identity/src/features/sessions/Sessions.tsx` | Signing out of Lys signs the person out at the issuer, and the issuer tells every registered app | DIRECTORY-059 |
| `crates/lys-identity/src/log.rs` | The receipts route answers a signed checkpoint | DIRECTORY-058 |
| `crates/lys-identity/src/error.rs` | One function verifies a receipt against a pinned key | DIRECTORY-058 |
| `crates/lys-identity/src/receipt.rs` | One function verifies a receipt against a pinned key | DIRECTORY-058 |
| `crates/lys-core/src/checkpoint/note.rs` | One function verifies a receipt against a pinned key | DIRECTORY-058 |
| `crates/lys-core/src/checkpoint/note_tests.rs` | One function verifies a receipt against a pinned key | DIRECTORY-058 |
| `crates/lys-secrets/src/service.rs` | A start reads its credentials from Lys's own broker, and no route reaches an app for them | DIRECTORY-059 |
| `crates/lys-identity/tests/uncertain_support/mod.rs` | The receipts route answers a signed checkpoint | DIRECTORY-058 |
| `crates/lys-identity/tests/signed_head_settle.rs` | The receipts route answers a signed checkpoint | DIRECTORY-058 |
| `crates/lys-identity/tests/support/mod.rs` | The receipts route answers a signed checkpoint | DIRECTORY-058 |
| `crates/lys/src/identity/install/detached.rs` | a detached start through the holder | DIRECTORY-057 |
| `docs/design/directory/HOLDER-KEY-OPTIONS.md` | options for agent session holder keys, for a ruling before DIRECTORY-049 R8 and a DIRECTORY-050 amendment | DIRECTORY-050 |
| `crates/lys-runner/src/session_key.rs` | the runner's session holder key | DIRECTORY-060 |
| `crates/lys-runner/tests/session_key.rs` | the runner's session holder key | DIRECTORY-060 |
| `crates/lys-core/src/keys/identity.rs` | the runner's session holder key | DIRECTORY-060 |
| `crates/lys-runner/src/peer.rs` | R6: socket peer credentials, ancestry and leader start identity; DIRECTORY-060 reuses this proof | DIRECTORY-051 |
| `crates/lys-runner/tests/present.rs` | the runner's session holder key | DIRECTORY-060 |
| `crates/lys-runner/src/present_client.rs` | the runner's session holder key | DIRECTORY-060 |
| `docs/design/directory/briefs/DIRECTORY-060.json` | the runner's session holder key brief | DIRECTORY-060 |
| `docs/design/directory/briefs/DIRECTORY-060.md` | the runner's session holder key brief | DIRECTORY-060 |
| `crates/lys/src/identity/upgrade/exchange.rs` | The exchange of bin/, the configuration and the screens with what is kept, undoing its own renames on a failure. | DIRECTORY-053 |
| `crates/lys/src/identity/upgrade/exchange_tests.rs` | Tests of the exchange with a rename step that fails on a chosen call. | DIRECTORY-053 |
| `crates/lys-identity-server/src/kinds.rs` | The registry of every signed log kind lys-identity-server folds. | DIRECTORY-053 |
| `crates/lys-secrets/src/bin/lys-secrets/kinds.rs` | The registry of every audit log kind lys-secrets folds. | DIRECTORY-053 |
| `crates/lys-install/Cargo.toml` | R1: Extract the install library without changing its behaviour | DIRECTORY-054 |
| `crates/lys-install/src/config.rs` | R1: Extract the install library without changing its behaviour | DIRECTORY-054 |
| `crates/lys-install/src/config/validate.rs` | R1: Extract the install library without changing its behaviour | DIRECTORY-054 |
| `crates/lys-install/src/config_tests.rs` | R1: Extract the install library without changing its behaviour | DIRECTORY-054 |
| `crates/lys-install/src/configure.rs` | R1: Extract the install library without changing its behaviour | DIRECTORY-054 |
| `crates/lys-install/src/configure_tests.rs` | R1: Extract the install library without changing its behaviour | DIRECTORY-054 |
| `crates/lys-install/src/credentials.rs` | R1: Extract the install library without changing its behaviour | DIRECTORY-054 |
| `crates/lys-install/src/credentials_tests.rs` | R1: Extract the install library without changing its behaviour | DIRECTORY-054 |
| `crates/lys-install/src/error.rs` | R1: Extract the install library without changing its behaviour | DIRECTORY-054 |
| `crates/lys-install/src/error_tests.rs` | R1: Extract the install library without changing its behaviour | DIRECTORY-054 |
| `crates/lys-install/src/health.rs` | R1: Extract the install library without changing its behaviour | DIRECTORY-054 |
| `crates/lys-install/src/install.rs` | R1: Extract the install library without changing its behaviour | DIRECTORY-054 |
| `crates/lys-install/src/install/deployment.template.toml` | R1: Extract the install library without changing its behaviour | DIRECTORY-054 |
| `crates/lys-install/src/install/detached.rs` | R1: Extract the install library without changing its behaviour | DIRECTORY-054 |
| `crates/lys-install/src/install/exit_wait.rs` | R1: Extract the install library without changing its behaviour | DIRECTORY-054 |
| `crates/lys-install/src/install/exit_wait_tests.rs` | R1: Extract the install library without changing its behaviour | DIRECTORY-054 |
| `crates/lys-install/src/install/held_start.rs` | R1: Extract the install library without changing its behaviour | DIRECTORY-054 |
| `crates/lys-install/src/install/held_start_tests.rs` | R1: Extract the install library without changing its behaviour | DIRECTORY-054 |
| `crates/lys-install/src/install/layout.rs` | R1: Extract the install library without changing its behaviour | DIRECTORY-054 |
| `crates/lys-install/src/install/server_config.rs` | R1: Extract the install library without changing its behaviour | DIRECTORY-054 |
| `crates/lys-install/src/install/services.rs` | R1: Extract the install library without changing its behaviour | DIRECTORY-054 |
| `crates/lys-install/src/install/start_race_tests.rs` | R1: Extract the install library without changing its behaviour | DIRECTORY-054 |
| `crates/lys-install/src/install/surface.rs` | R1: Extract the install library without changing its behaviour | DIRECTORY-054 |
| `crates/lys-install/src/install_tests.rs` | R1: Extract the install library without changing its behaviour | DIRECTORY-054 |
| `crates/lys-install/src/lib.rs` | R1: Extract the install library without changing its behaviour | DIRECTORY-054 |
| `crates/lys-install/src/loopback_http.rs` | R1: Extract the install library without changing its behaviour | DIRECTORY-054 |
| `crates/lys-install/src/loopback_http_tests.rs` | R1: Extract the install library without changing its behaviour | DIRECTORY-054 |
| `crates/lys-install/src/output.rs` | R1: Extract the install library without changing its behaviour | DIRECTORY-054 |
| `crates/lys-install/src/output_tests.rs` | R1: Extract the install library without changing its behaviour | DIRECTORY-054 |
| `crates/lys-install/src/prepare.rs` | R1: Extract the install library without changing its behaviour | DIRECTORY-054 |
| `crates/lys-install/src/prepare_tests.rs` | R1: Extract the install library without changing its behaviour | DIRECTORY-054 |
| `crates/lys-install/src/private_files.rs` | R1: Extract the install library without changing its behaviour | DIRECTORY-054 |
| `crates/lys-install/src/rauthy.rs` | R1: Extract the install library without changing its behaviour | DIRECTORY-054 |
| `crates/lys-install/src/themes.rs` | R1: Extract the install library without changing its behaviour | DIRECTORY-054 |
| `crates/lys-install/src/themes_tests.rs` | R1: Extract the install library without changing its behaviour | DIRECTORY-054 |
| `crates/lys-install/src/upgrade.rs` | R1: Extract the install library without changing its behaviour | DIRECTORY-054 |
| `crates/lys-install/src/upgrade/back.rs` | R1: Extract the install library without changing its behaviour | DIRECTORY-054 |
| `crates/lys-install/src/upgrade/exchange.rs` | R1: Extract the install library without changing its behaviour | DIRECTORY-054 |
| `crates/lys-install/src/upgrade/exchange_tests.rs` | R1: Extract the install library without changing its behaviour | DIRECTORY-054 |
| `crates/lys-install/src/upgrade_back_tests.rs` | R1: Extract the install library without changing its behaviour | DIRECTORY-054 |
| `crates/lys-install/src/upgrade_tests.rs` | R1: Extract the install library without changing its behaviour | DIRECTORY-054 |
| `Cargo.lock` | R1: Extract the install library without changing its behaviour | DIRECTORY-054 |
| `Cargo.toml` | R1: Extract the install library without changing its behaviour | DIRECTORY-054 |
| `crates/lys/Cargo.toml` | R1: Extract the install library without changing its behaviour | DIRECTORY-054 |
| `crates/lys/src/commands/mod.rs` | R1: Extract the install library without changing its behaviour | DIRECTORY-054 |
| `crates/lys/src/identity/install.rs` | R1: Extract the install library without changing its behaviour | DIRECTORY-054 |
| `crates/lys/src/identity/mod.rs` | R1: Extract the install library without changing its behaviour | DIRECTORY-054 |
| `crates/lys/src/main.rs` | R1: Extract the install library without changing its behaviour | DIRECTORY-054 |
| `deploy/identity/theme-map.md` | R1: Extract the install library without changing its behaviour | DIRECTORY-054 |
| `crates/lys/src/commands/output.rs` | R1: Extract the install library without changing its behaviour (old path removed after relocation) | DIRECTORY-054 |
| `crates/lys/src/commands/output_tests.rs` | R1: Extract the install library without changing its behaviour (old path removed after relocation) | DIRECTORY-054 |
| `crates/lys/src/identity/config.rs` | R1: Extract the install library without changing its behaviour (old path removed after relocation) | DIRECTORY-054 |
| `crates/lys/src/identity/config/validate.rs` | R1: Extract the install library without changing its behaviour (old path removed after relocation) | DIRECTORY-054 |
| `crates/lys/src/identity/config_tests.rs` | R1: Extract the install library without changing its behaviour (old path removed after relocation) | DIRECTORY-054 |
| `crates/lys/src/identity/configure.rs` | R1: Extract the install library without changing its behaviour (old path removed after relocation) | DIRECTORY-054 |
| `crates/lys/src/identity/configure_tests.rs` | R1: Extract the install library without changing its behaviour (old path removed after relocation) | DIRECTORY-054 |
| `crates/lys/src/identity/credentials.rs` | R1: Extract the install library without changing its behaviour (old path removed after relocation) | DIRECTORY-054 |
| `crates/lys/src/identity/credentials_tests.rs` | R1: Extract the install library without changing its behaviour (old path removed after relocation) | DIRECTORY-054 |
| `crates/lys/src/identity/error.rs` | R1: Extract the install library without changing its behaviour (old path removed after relocation) | DIRECTORY-054 |
| `crates/lys/src/identity/error_tests.rs` | R1: Extract the install library without changing its behaviour (old path removed after relocation) | DIRECTORY-054 |
| `crates/lys/src/identity/health.rs` | R1: Extract the install library without changing its behaviour (old path removed after relocation) | DIRECTORY-054 |
| `crates/lys/src/identity/install/deployment.template.toml` | R1: Extract the install library without changing its behaviour (old path removed after relocation) | DIRECTORY-054 |
| `crates/lys/src/identity/install/detached.rs` | R1: Extract the install library without changing its behaviour (old path removed after relocation) | DIRECTORY-054 |
| `crates/lys/src/identity/install/exit_wait.rs` | R1: Extract the install library without changing its behaviour (old path removed after relocation) | DIRECTORY-054 |
| `crates/lys/src/identity/install/exit_wait_tests.rs` | R1: Extract the install library without changing its behaviour (old path removed after relocation) | DIRECTORY-054 |
| `crates/lys/src/identity/install/held_start.rs` | R1: Extract the install library without changing its behaviour (old path removed after relocation) | DIRECTORY-054 |
| `crates/lys/src/identity/install/held_start_tests.rs` | R1: Extract the install library without changing its behaviour (old path removed after relocation) | DIRECTORY-054 |
| `crates/lys/src/identity/install/layout.rs` | R1: Extract the install library without changing its behaviour (old path removed after relocation) | DIRECTORY-054 |
| `crates/lys/src/identity/install/server_config.rs` | R1: Extract the install library without changing its behaviour (old path removed after relocation) | DIRECTORY-054 |
| `crates/lys/src/identity/install/services.rs` | R1: Extract the install library without changing its behaviour (old path removed after relocation) | DIRECTORY-054 |
| `crates/lys/src/identity/install/start_race_tests.rs` | R1: Extract the install library without changing its behaviour (old path removed after relocation) | DIRECTORY-054 |
| `crates/lys/src/identity/install/surface.rs` | R1: Extract the install library without changing its behaviour (old path removed after relocation) | DIRECTORY-054 |
| `crates/lys/src/identity/install_tests.rs` | R1: Extract the install library without changing its behaviour (old path removed after relocation) | DIRECTORY-054 |
| `crates/lys/src/identity/loopback_http.rs` | R1: Extract the install library without changing its behaviour (old path removed after relocation) | DIRECTORY-054 |
| `crates/lys/src/identity/loopback_http_tests.rs` | R1: Extract the install library without changing its behaviour (old path removed after relocation) | DIRECTORY-054 |
| `crates/lys/src/identity/prepare.rs` | R1: Extract the install library without changing its behaviour (old path removed after relocation) | DIRECTORY-054 |
| `crates/lys/src/identity/prepare_tests.rs` | R1: Extract the install library without changing its behaviour (old path removed after relocation) | DIRECTORY-054 |
| `crates/lys/src/identity/private_files.rs` | R1: Extract the install library without changing its behaviour (old path removed after relocation) | DIRECTORY-054 |
| `crates/lys/src/identity/rauthy.rs` | R1: Extract the install library without changing its behaviour (old path removed after relocation) | DIRECTORY-054 |
| `crates/lys/src/identity/themes.rs` | R1: Extract the install library without changing its behaviour (old path removed after relocation) | DIRECTORY-054 |
| `crates/lys/src/identity/themes_tests.rs` | R1: Extract the install library without changing its behaviour (old path removed after relocation) | DIRECTORY-054 |
| `crates/lys/src/identity/upgrade.rs` | R1: Extract the install library without changing its behaviour (old path removed after relocation) | DIRECTORY-054 |
| `crates/lys/src/identity/upgrade/back.rs` | R1: Extract the install library without changing its behaviour (old path removed after relocation) | DIRECTORY-054 |
| `crates/lys/src/identity/upgrade/exchange.rs` | R1: Extract the install library without changing its behaviour (old path removed after relocation) | DIRECTORY-054 |
| `crates/lys/src/identity/upgrade/exchange_tests.rs` | R1: Extract the install library without changing its behaviour (old path removed after relocation) | DIRECTORY-054 |
| `crates/lys/src/identity/upgrade_back_tests.rs` | R1: Extract the install library without changing its behaviour (old path removed after relocation) | DIRECTORY-054 |
| `crates/lys/src/identity/upgrade_tests.rs` | R1: Extract the install library without changing its behaviour (old path removed after relocation) | DIRECTORY-054 |
| `crates/lys-app/Cargo.toml` | R2: Opening the app installs and shows every step on a Lys page | DIRECTORY-054 |
| `crates/lys-app/build.rs` | R2: Opening the app installs and shows every step on a Lys page | DIRECTORY-054 |
| `crates/lys-app/src/bundle.rs` | R2: Opening the app installs and shows every step on a Lys page | DIRECTORY-054 |
| `crates/lys-app/src/bundle_tests.rs` | R2: Opening the app installs and shows every step on a Lys page | DIRECTORY-054 |
| `crates/lys-app/src/flow.rs` | R2: Opening the app installs and shows every step on a Lys page | DIRECTORY-054 |
| `crates/lys-app/src/flow_tests.rs` | R2: Opening the app installs and shows every step on a Lys page | DIRECTORY-054 |
| `crates/lys-app/src/launcher.rs` | R2: Opening the app installs and shows every step on a Lys page | DIRECTORY-054 |
| `crates/lys-app/src/launcher_tests.rs` | R2: Opening the app installs and shows every step on a Lys page | DIRECTORY-054 |
| `crates/lys-app/src/main.rs` | R2: Opening the app installs and shows every step on a Lys page | DIRECTORY-054 |
| `crates/lys-app/src/main_tests.rs` | R2: Opening the app installs and shows every step on a Lys page | DIRECTORY-054 |
| `crates/lys-app/src/progress.rs` | R2: Opening the app installs and shows every step on a Lys page | DIRECTORY-054 |
| `crates/lys-app/src/progress_tests.rs` | R2: Opening the app installs and shows every step on a Lys page | DIRECTORY-054 |
| `crates/lys-app/src/refusal.rs` | R2: Opening the app installs and shows every step on a Lys page | DIRECTORY-054 |
| `crates/lys-app/src/server.rs` | R2: Opening the app installs and shows every step on a Lys page | DIRECTORY-054 |
| `crates/lys-app/src/server_tests.rs` | R2: Opening the app installs and shows every step on a Lys page | DIRECTORY-054 |
| `crates/lys-install/src/steps.rs` | R2: Opening the app installs and shows every step on a Lys page | DIRECTORY-054 |
| `crates/lys-install/src/steps_tests.rs` | R2: Opening the app installs and shows every step on a Lys page | DIRECTORY-054 |
| `surface/identity/src/features/install/InstallProgress.tsx` | R2: Opening the app installs and shows every step on a Lys page | DIRECTORY-054 |
| `surface/identity/tests/install-progress.test.tsx` | R2: Opening the app installs and shows every step on a Lys page | DIRECTORY-054 |
| `surface/identity/src/App.tsx` | R2: Opening the app installs and shows every step on a Lys page | DIRECTORY-054 |
| `crates/lys-app/src/engine.rs` | R3: The container engine, guided in plain words | DIRECTORY-054 |
| `crates/lys-app/src/engine_tests.rs` | R3: The container engine, guided in plain words | DIRECTORY-054 |
| `crates/lys-app/src/engine_wait.rs` | R3: The container engine, guided in plain words | DIRECTORY-054 |
| `crates/lys-app/src/engine_wait_tests.rs` | R3: The container engine, guided in plain words | DIRECTORY-054 |
| `crates/lys-install/src/install/engine_path.rs` | R3: The container engine, guided in plain words | DIRECTORY-054 |
| `crates/lys-install/src/install/engine_path_tests.rs` | R3: The container engine, guided in plain words | DIRECTORY-054 |
| `crates/lys-app/src/login_item.rs` | R4: Start at login, reopen, upgrade and uninstall | DIRECTORY-054 |
| `crates/lys-app/src/login_item_tests.rs` | R4: Start at login, reopen, upgrade and uninstall | DIRECTORY-054 |
| `crates/lys-app/src/uninstall.rs` | R4: Start at login, reopen, upgrade and uninstall | DIRECTORY-054 |
| `crates/lys-app/src/uninstall_tests.rs` | R4: Start at login, reopen, upgrade and uninstall | DIRECTORY-054 |
| `crates/lys-identity-server/src/uninstall_api.rs` | R4: Start at login, reopen, upgrade and uninstall | DIRECTORY-054 |
| `crates/lys-identity-server/src/uninstall_api_tests.rs` | R4: Start at login, reopen, upgrade and uninstall | DIRECTORY-054 |
| `surface/identity/src/features/account/Uninstall.tsx` | R4: Start at login, reopen, upgrade and uninstall | DIRECTORY-054 |
| `surface/identity/tests/uninstall.test.tsx` | R4: Start at login, reopen, upgrade and uninstall | DIRECTORY-054 |
| `crates/lys-identity-server/src/error.rs` | R4: Start at login, reopen, upgrade and uninstall | DIRECTORY-054 |
| `crates/lys-identity-server/src/error_status.rs` | R4: Start at login, reopen, upgrade and uninstall | DIRECTORY-054 |
| `crates/lys-identity-server/src/lib.rs` | R4: Start at login, reopen, upgrade and uninstall | DIRECTORY-054 |
| `crates/lys-identity-server/src/routes.rs` | R4: Start at login, reopen, upgrade and uninstall | DIRECTORY-054 |
| `surface/identity/src/features/me/You.tsx` | R4: Start at login, reopen, upgrade and uninstall | DIRECTORY-054 |
| `surface/identity/tests/fixtures.ts` | R4: Start at login, reopen, upgrade and uninstall | DIRECTORY-054 |
| `crates/lys/src/package.rs` | R5: `lys package app` builds the app and disk image | DIRECTORY-054 |
| `crates/lys/src/package_tests.rs` | R5: `lys package app` builds the app and disk image | DIRECTORY-054 |
| `packaging/macos/Info.plist` | R5: `lys package app` builds the app and disk image | DIRECTORY-054 |
| `packaging/macos/Lys.icns` | R5: `lys package app` builds the app and disk image | DIRECTORY-054 |
| `crates/lys-home/build.rs` | R5: `lys package app` builds the app and disk image | DIRECTORY-054 |
| `crates/lys-home/tests/version.rs` | R5: `lys package app` builds the app and disk image | DIRECTORY-054 |
| `crates/lys-identity-server/build.rs` | R5: `lys package app` builds the app and disk image | DIRECTORY-054 |
| `crates/lys-identity-server/tests/version.rs` | R5: `lys package app` builds the app and disk image | DIRECTORY-054 |
| `crates/lys-secrets/build.rs` | R5: `lys package app` builds the app and disk image | DIRECTORY-054 |
| `crates/lys-secrets/tests/version.rs` | R5: `lys package app` builds the app and disk image | DIRECTORY-054 |
| `crates/lys/build.rs` | R5: `lys package app` builds the app and disk image | DIRECTORY-054 |
| `crates/lys/src/cli.rs` | R5: `lys package app` builds the app and disk image | DIRECTORY-054 |
| `crates/lys/src/commands/error.rs` | R5: `lys package app` builds the app and disk image | DIRECTORY-054 |
| `crates/lys/src/commands/error_tests.rs` | R5: `lys package app` builds the app and disk image | DIRECTORY-054 |
| `crates/lys/tests/version.rs` | R5: `lys package app` builds the app and disk image | DIRECTORY-054 |
| `docs/design/directory/reports/DIRECTORY-054-fresh-account.md` | R6: Prepare the fresh-account proof and its evidence record | DIRECTORY-054 |
| `docs/design/directory/reports/DIRECTORY-054-open-items.md` | R6: Prepare the fresh-account proof and its evidence record | DIRECTORY-054 |
| `crates/lys-home/src/record/lantern.rs` | DIRECTORY-052 R3: a member's starting memories are written as lantern notes | DIRECTORY-052 |
| `crates/lys-home/src/cli/fewshot.rs` | DIRECTORY-052 R3: a member's opening conversation is written as fewshot turns | DIRECTORY-052 |
| `crates/lys-home/tests/fewshot_write.rs` | DIRECTORY-052 R3: fewshot turns written through lys-home's public library function | DIRECTORY-052 |
| `crates/lys-home/src/lib.rs` | DIRECTORY-052 R3: lys-home's library exposes the fewshot writer | DIRECTORY-052 |
| `crates/lys-home/src/cli.rs` | DIRECTORY-052 R3: the fewshot command calls the library writer | DIRECTORY-052 |
| `crates/lys-identity-server/src/grants_connector.rs` | Application-connector holder conversion and authorization helpers, keeping grants.rs within ADR-111. | DIRECTORY-048 |
| `crates/lys-identity-server/src/provisioning_api.rs` | R1: Authenticated parsed tracking from Argus, durably resumed by cursor | DIRECTORY-051 |
| `crates/lys-identity-server/src/budgets_store.rs` | R2: Budgets on agents, teams and people | DIRECTORY-051 |
| `crates/lys-runner/src/operations.rs` | R3: A reached budget acts once | DIRECTORY-051 |
| `crates/lys-runner/tests/operations.rs` | R3: A reached budget acts once | DIRECTORY-051 |
| `crates/lys-runner/src/state.rs` | R3: A reached budget acts once | DIRECTORY-051 |
| `crates/lys-identity-server/src/runner_acts.rs` | R3: A reached budget acts once | DIRECTORY-051 |
| `crates/lys-identity-server/src/goals_store.rs` | R4: Goals, expectations and deliverables, with deadlines and reminders | DIRECTORY-051 |
| `surface/identity/tests/usage.test.tsx` | R5: Plain budget and goal controls | DIRECTORY-051 |
| `surface/identity/src/shell/Shell.tsx` | R5: Plain budget and goal controls | DIRECTORY-051 |
| `crates/lys-identity-server/src/refusals_api.rs` | R6: Every refused act is visible on the agent page, from authoritative records | DIRECTORY-051 |
| `crates/lys-identity-server/src/refusals_store.rs` | R6: Every refused act is visible on the agent page, from authoritative records | DIRECTORY-051 |
| `crates/lys-identity-server/tests/refusals.rs` | R6: Every refused act is visible on the agent page, from authoritative records | DIRECTORY-051 |
| `crates/lys-runner/src/refusals.rs` | R6: Every refused act is visible on the agent page, from authoritative records | DIRECTORY-051 |
| `crates/lys-runner/tests/refusals.rs` | R6: Every refused act is visible on the agent page, from authoritative records | DIRECTORY-051 |
| `surface/identity/src/features/file/AgentRefusals.tsx` | R6: Every refused act is visible on the agent page, from authoritative records | DIRECTORY-051 |
| `surface/identity/tests/agent-refusals.test.tsx` | R6: Every refused act is visible on the agent page, from authoritative records | DIRECTORY-051 |
| `surface/identity/src/features/file/tabs.ts` | R6: Every refused act is visible on the agent page, from authoritative records | DIRECTORY-051 |
| `crates/lys-runner/src/tracking.rs` | R1: Lys-owned hooks, status line and incremental session-stream tracking | DIRECTORY-051 |
| `crates/lys-runner/src/tracking_store.rs` | R1: Lys-owned hooks, status line and incremental session-stream tracking | DIRECTORY-051 |
| `crates/lys-runner/tests/tracking.rs` | R1: Lys-owned hooks, status line and incremental session-stream tracking | DIRECTORY-051 |
| `crates/lys-home/src/harness/tracking.rs` | R1: Lys-owned hooks, status line and incremental session-stream tracking | DIRECTORY-051 |
| `crates/lys-home/src/harness/tracking_tests.rs` | R1: Lys-owned hooks, status line and incremental session-stream tracking | DIRECTORY-051 |
| `crates/lys-identity-server/src/tracking_export.rs` | R1: Lys-owned hooks, status line and incremental session-stream tracking | DIRECTORY-051 |
| `crates/lys-runner/src/session/lifecycle.rs` | R1: Lys-owned hooks, status line and incremental session-stream tracking | DIRECTORY-051 |
| `crates/lys-home/src/harness/mod.rs` | R1: Lys-owned hooks, status line and incremental session-stream tracking | DIRECTORY-051 |
| `crates/lys-home/src/harness/claude_code/mod.rs` | R1: Lys-owned hooks, status line and incremental session-stream tracking | DIRECTORY-051 |
| `crates/lys-home/src/harness/claude_code/launch.rs` | R1: Lys-owned hooks, status line and incremental session-stream tracking | DIRECTORY-051 |
| `crates/lys-home/src/harness/claude_code/launch_env.rs` | R1: Lys-owned hooks, status line and incremental session-stream tracking | DIRECTORY-051 |
| `crates/lys-home/src/harness/codex/mod.rs` | R1: Lys-owned hooks, status line and incremental session-stream tracking | DIRECTORY-051 |
| `surface/identity/tests/acceptance/refusals.spec.ts` | R6: Every refused act is visible on the agent page, from authoritative records | DIRECTORY-051 |
| `surface/identity/vite.config.ts` | R6: Every refused act is visible on the agent page, from authoritative records | DIRECTORY-051 |
| `crates/lys-identity-server/src/grants_refusals.rs` | R6: refusal capture without growing grants.rs | DIRECTORY-051 |
| `crates/lys-identity-server/src/spicedb_cancel.rs` | a SpiceDB wait ends when its request leaves | DIRECTORY-061 |
| `crates/lys-identity-server/src/spicedb_cancel_tests.rs` | a SpiceDB wait ends when its request leaves | DIRECTORY-061 |
| `crates/lys-identity-server/tests/spicedb_cancel.rs` | a SpiceDB wait ends when its request leaves | DIRECTORY-061 |
| `docs/design/directory/briefs/DIRECTORY-061.json` | the SpiceDB cancel brief | DIRECTORY-061 |
| `docs/design/directory/briefs/DIRECTORY-061.md` | the SpiceDB cancel brief | DIRECTORY-061 |
| `crates/lys-runner/src/containment_policy.rs` | One Lys policy becomes a bound containment plan; DIRECTORY-062 R1. | DIRECTORY-062 |
| `crates/lys-runner/src/containment.rs` | One Lys policy becomes a bound containment plan; DIRECTORY-062 R1. | DIRECTORY-062 |
| `crates/lys-runner/tests/containment_policy.rs` | One Lys policy becomes a bound containment plan; DIRECTORY-062 R1. | DIRECTORY-062 |
| `crates/lys-identity-server/src/containment_policy.rs` | One Lys policy becomes a bound containment plan; DIRECTORY-062 R1. | DIRECTORY-062 |
| `crates/lys-identity-server/tests/containment_policy.rs` | One Lys policy becomes a bound containment plan; DIRECTORY-062 R1. | DIRECTORY-062 |
| `crates/lys-runner/src/containment_macos.rs` | macOS applies Seatbelt before the harness can run; DIRECTORY-062 R2. | DIRECTORY-062 |
| `crates/lys-runner/src/containment_egress.rs` | macOS applies Seatbelt before the harness can run; DIRECTORY-062 R2. | DIRECTORY-062 |
| `crates/lys-runner/tests/containment_macos.rs` | macOS applies Seatbelt before the harness can run; DIRECTORY-062 R2. | DIRECTORY-062 |
| `crates/lys-runner/tests/containment_egress.rs` | macOS applies Seatbelt before the harness can run; DIRECTORY-062 R2. | DIRECTORY-062 |
| `crates/lys-runner/src/containment_linux.rs` | Linux applies Landlock and a network namespace before exec; DIRECTORY-062 R3. | DIRECTORY-062 |
| `crates/lys-runner/src/containment_helper.rs` | Linux applies Landlock and a network namespace before exec; DIRECTORY-062 R3. | DIRECTORY-062 |
| `crates/lys-runner/tests/containment_linux.rs` | Linux applies Landlock and a network namespace before exec; DIRECTORY-062 R3. | DIRECTORY-062 |
| `crates/lys/src/cli/containment.rs` | Linux applies Landlock and a network namespace before exec; DIRECTORY-062 R3. | DIRECTORY-062 |
| `crates/lys-runner/src/containment_audit.rs` | Kernel evidence feeds the existing refusal stream; DIRECTORY-062 R4. | DIRECTORY-062 |
| `crates/lys-runner/src/containment_audit_macos.rs` | Kernel evidence feeds the existing refusal stream; DIRECTORY-062 R4. | DIRECTORY-062 |
| `crates/lys-runner/src/containment_audit_linux.rs` | Kernel evidence feeds the existing refusal stream; DIRECTORY-062 R4. | DIRECTORY-062 |
| `crates/lys-runner/tests/containment_audit.rs` | Kernel evidence feeds the existing refusal stream; DIRECTORY-062 R4. | DIRECTORY-062 |
| `crates/lys-identity-server/src/containment_api.rs` | The agent page states the sandbox and the evidence; DIRECTORY-062 R5. | DIRECTORY-062 |
| `crates/lys-identity-server/tests/containment.rs` | The agent page states the sandbox and the evidence; DIRECTORY-062 R5. | DIRECTORY-062 |
| `surface/identity/src/features/file/AgentContainment.tsx` | The agent page states the sandbox and the evidence; DIRECTORY-062 R5. | DIRECTORY-062 |
| `surface/identity/tests/agent-containment.test.tsx` | The agent page states the sandbox and the evidence; DIRECTORY-062 R5. | DIRECTORY-062 |
| `scripts/identity-gates/containment-macos.sh` | A person watches real native denials and an allowed control; DIRECTORY-062 R6. | DIRECTORY-062 |
| `scripts/identity-gates/containment-linux.sh` | A person watches real native denials and an allowed control; DIRECTORY-062 R6. | DIRECTORY-062 |
| `crates/lys-runner/tests/containment_probe.rs` | A person watches real native denials and an allowed control; DIRECTORY-062 R6. | DIRECTORY-062 |
| `surface/identity/tests/acceptance/containment.spec.ts` | A person watches real native denials and an allowed control; DIRECTORY-062 R6. | DIRECTORY-062 |
| `docs/design/directory/PROOF-CONTAINMENT.md` | A person watches real native denials and an allowed control; DIRECTORY-062 R6. | DIRECTORY-062 |
| `crates/lys-home/src/harness/claude_code/settings.rs` | R1: Lys-owned hooks, status line and incremental session-stream tracking | DIRECTORY-051 |
| `crates/lys-home/src/harness/claude_code/settings_tests.rs` | R1: Lys-owned hooks, status line and incremental session-stream tracking | DIRECTORY-051 |
| `crates/lys-home/src/harness/codex/config.rs` | R1: Lys-owned hooks, status line and incremental session-stream tracking | DIRECTORY-051 |
| `crates/lys-home/src/harness/codex/config_tests.rs` | R1: Lys-owned hooks, status line and incremental session-stream tracking | DIRECTORY-051 |
| `crates/lys/src/commands/runner_hook.rs` | R1: Lys-owned hooks, status line and incremental session-stream tracking | DIRECTORY-051 |
| `crates/lys/src/commands/runner_statusline.rs` | R1: Lys-owned hooks, status line and incremental session-stream tracking | DIRECTORY-051 |
| `crates/lys/src/commands/runner_notify.rs` | R1: Lys-owned hooks, status line and incremental session-stream tracking | DIRECTORY-051 |
| `crates/lys-home/src/harness/claude_code/render.rs` | R1: Lys-owned hooks, status line and incremental session-stream tracking | DIRECTORY-051 |
| `crates/lys-home/src/harness/claude_code/render_write.rs` | R1: Lys-owned hooks, status line and incremental session-stream tracking | DIRECTORY-051 |
| `crates/lys-home/src/harness/claude_code/template.rs` | R1: Lys-owned hooks, status line and incremental session-stream tracking | DIRECTORY-051 |
| `crates/lys-identity-server/src/provisioning_store.rs` | R1: Lys-owned hooks, status line and incremental session-stream tracking | DIRECTORY-051 |
| `crates/lys-runner/src/judge.rs` | R6: Lys creates tool-boundary policy enforcement and shows authoritative refusals | DIRECTORY-051 |
| `crates/lys-runner/src/refusal_log.rs` | R6: Lys creates tool-boundary policy enforcement and shows authoritative refusals | DIRECTORY-051 |
| `crates/lys-runner/tests/judge.rs` | R6: Lys creates tool-boundary policy enforcement and shows authoritative refusals | DIRECTORY-051 |
| `crates/lys/src/commands/runner_judge.rs` | R6: Lys creates tool-boundary policy enforcement and shows authoritative refusals | DIRECTORY-051 |
| `crates/lys-identity-server/src/agent_policy_api.rs` | R6: Lys creates tool-boundary policy enforcement and shows authoritative refusals | DIRECTORY-051 |
| `crates/lys-identity-server/src/agent_policy_store.rs` | R6: Lys creates tool-boundary policy enforcement and shows authoritative refusals | DIRECTORY-051 |
| `crates/lys-identity-server/tests/agent_policy.rs` | R6: Lys creates tool-boundary policy enforcement and shows authoritative refusals | DIRECTORY-051 |
| `surface/identity/src/features/file/AgentPolicy.tsx` | R6: Lys creates tool-boundary policy enforcement and shows authoritative refusals | DIRECTORY-051 |
| `surface/identity/tests/agent-policy.test.tsx` | R6: Lys creates tool-boundary policy enforcement and shows authoritative refusals | DIRECTORY-051 |
| `crates/lys-runner/tests/judge_peer.rs` | R6: Lys creates tool-boundary policy enforcement and shows authoritative refusals | DIRECTORY-051 |
| `crates/lys-identity-server/src/runner_sessions.rs` | R6: Lys creates tool-boundary policy enforcement and shows authoritative refusals | DIRECTORY-051 |
| `crates/lys-identity-server/src/grant_sight.rs` | R6: Lys creates tool-boundary policy enforcement and shows authoritative refusals | DIRECTORY-051 |
| `crates/lys-identity-server/src/health_api.rs` | the service's own health answer | DIRECTORY-063 |
| `crates/lys-identity-server/src/health_api_tests.rs` | the service's own health answer | DIRECTORY-063 |
| `docs/design/directory/briefs/DIRECTORY-063.json` | the unknown API path and health brief | DIRECTORY-063 |
| `docs/design/directory/briefs/DIRECTORY-063.md` | the unknown API path and health brief | DIRECTORY-063 |
| `crates/lys-runner/src/containment_forwarder.rs` | Linux applies Landlock and a network namespace before exec. DIRECTORY-062 R3. | DIRECTORY-062 |
| `crates/lys-runner/src/containment_prepare.rs` | Linux applies Landlock and a network namespace before exec. DIRECTORY-062 R3. | DIRECTORY-062 |
| `crates/lys-runner/src/containment_prepare_macos.rs` | Linux applies Landlock and a network namespace before exec. DIRECTORY-062 R3. | DIRECTORY-062 |
| `crates/lys-runner/src/containment_prepare_linux.rs` | Linux applies Landlock and a network namespace before exec. DIRECTORY-062 R3. | DIRECTORY-062 |
| `crates/lys-runner/tests/containment_prepare.rs` | Linux applies Landlock and a network namespace before exec. DIRECTORY-062 R3. | DIRECTORY-062 |
| `crates/lys-runner/tests/containment_forwarder.rs` | Linux applies Landlock and a network namespace before exec. DIRECTORY-062 R3. | DIRECTORY-062 |
| `docs/CONTAINMENT-PREPARATION.md` | Linux applies Landlock and a network namespace before exec. DIRECTORY-062 R3. | DIRECTORY-062 |
| `scripts/identity-gates/containment-capabilities.sh` | A person watches real native denials and an allowed control. DIRECTORY-062 R6. | DIRECTORY-062 |
| `crates/lys-runner/src/harness_control.rs` | One managed harness channel, with proved turn boundaries (DIRECTORY-064 R1). | DIRECTORY-064 |
| `crates/lys-runner/src/harness_control/events.rs` | One managed harness channel, with proved turn boundaries (DIRECTORY-064 R1). | DIRECTORY-064 |
| `crates/lys-runner/src/harness_control/process.rs` | One managed harness channel, with proved turn boundaries (DIRECTORY-064 R1). | DIRECTORY-064 |
| `crates/lys-runner/tests/harness_control.rs` | One managed harness channel, with proved turn boundaries (DIRECTORY-064 R1). | DIRECTORY-064 |
| `crates/lys-runner/src/harness_control/claude.rs` | Use Claude and Codex control protocols, never terminal typing (DIRECTORY-064 R2). | DIRECTORY-064 |
| `crates/lys-runner/src/harness_control/codex.rs` | Use Claude and Codex control protocols, never terminal typing (DIRECTORY-064 R2). | DIRECTORY-064 |
| `crates/lys-runner/tests/harness_claude.rs` | Use Claude and Codex control protocols, never terminal typing (DIRECTORY-064 R2). | DIRECTORY-064 |
| `crates/lys-runner/tests/harness_codex.rs` | Use Claude and Codex control protocols, never terminal typing (DIRECTORY-064 R2). | DIRECTORY-064 |
| `crates/lys-runner/src/harness_control/context.rs` | Enforce context thresholds at the owned boundary (DIRECTORY-064 R3). | DIRECTORY-064 |
| `crates/lys-runner/tests/context_control.rs` | Enforce context thresholds at the owned boundary (DIRECTORY-064 R3). | DIRECTORY-064 |
| `crates/lys-runner/src/harness_control/reminders.rs` | Deliver current goal and reminder words at turn boundaries (DIRECTORY-064 R4). | DIRECTORY-064 |
| `crates/lys-runner/tests/reminder_delivery.rs` | Deliver current goal and reminder words at turn boundaries (DIRECTORY-064 R4). | DIRECTORY-064 |
| `crates/lys-runner/tests/control_recovery.rs` | Reconcile uncertain delivery with existing operation receipts (DIRECTORY-064 R5). | DIRECTORY-064 |
| `crates/lys-identity-server/tests/control_receipts.rs` | Reconcile uncertain delivery with existing operation receipts (DIRECTORY-064 R5). | DIRECTORY-064 |
| `surface/identity/tests/acceptance/agent-control.spec.ts` | Plain controls and a real managed-session proof (DIRECTORY-064 R6). | DIRECTORY-064 |
| `crates/lys/src/identity/upgrade/runner.rs` | Place the runner binary without restarting a live runner | DIRECTORY-066 |
| `crates/lys/src/identity/upgrade/runner_tests.rs` | Place the runner binary without restarting a live runner | DIRECTORY-066 |
| `crates/lys/src/identity/upgrade/scratch_tests.rs` | Place the runner binary without restarting a live runner | DIRECTORY-066 |
| `crates/lys-runner/tests/build_identity.rs` | Record running and placed builds as different facts | DIRECTORY-066 |
| `crates/lys/src/commands/runner.rs` | Record running and placed builds as different facts | DIRECTORY-066 |
| `crates/lys/src/identity/runner_restart.rs` | Restart only through an explicit session-aware operation | DIRECTORY-066 |
| `crates/lys/src/identity/runner_restart_tests.rs` | Restart only through an explicit session-aware operation | DIRECTORY-066 |
| `crates/lys-runner/tests/restart_fence.rs` | Restart only through an explicit session-aware operation | DIRECTORY-066 |
| `crates/lys/src/identity/status.rs` | Show the same pending restart on the page and CLI | DIRECTORY-066 |
| `crates/lys/src/identity/status_tests.rs` | Show the same pending restart on the page and CLI | DIRECTORY-066 |
| `surface/identity/tests/acceptance/runner-build.spec.ts` | Show the same pending restart on the page and CLI | DIRECTORY-066 |
| `crates/lys-runner/src/codex_policy_contract.rs` | Pin the actual Codex executable and its supported policy contract. DIRECTORY-065 R1. | DIRECTORY-065 |
| `crates/lys-runner/tests/codex_policy_contract.rs` | Pin the actual Codex executable and its supported policy contract. DIRECTORY-065 R1. | DIRECTORY-065 |
| `docs/design/directory/CODEX-POLICY-CONTRACT.md` | Pin the actual Codex executable and its supported policy contract. DIRECTORY-065 R1. | DIRECTORY-065 |
| `crates/lys-home/src/harness/codex/policy.rs` | Render native Codex permissions from the bound Lys policy. DIRECTORY-065 R2. | DIRECTORY-065 |
| `crates/lys-home/src/harness/codex/policy_tests.rs` | Render native Codex permissions from the bound Lys policy. DIRECTORY-065 R2. | DIRECTORY-065 |
| `crates/lys-runner/src/codex_judge.rs` | Bind Codex pre-tool policy checks to the existing Lys judge. DIRECTORY-065 R3. | DIRECTORY-065 |
| `crates/lys-runner/tests/codex_judge.rs` | Bind Codex pre-tool policy checks to the existing Lys judge. DIRECTORY-065 R3. | DIRECTORY-065 |
| `crates/lys-home/src/harness/codex/hooks.rs` | Bind Codex pre-tool policy checks to the existing Lys judge. DIRECTORY-065 R3. | DIRECTORY-065 |
| `crates/lys-home/src/harness/codex/hooks_tests.rs` | Bind Codex pre-tool policy checks to the existing Lys judge. DIRECTORY-065 R3. | DIRECTORY-065 |
| `crates/lys-runner/src/codex_refusals.rs` | Record native Codex rejections with honest provenance. DIRECTORY-065 R4. | DIRECTORY-065 |
| `crates/lys-runner/tests/codex_refusals.rs` | Record native Codex rejections with honest provenance. DIRECTORY-065 R4. | DIRECTORY-065 |
| `surface/identity/tests/codex-policy.test.tsx` | The Codex agent page shows measured policy and refusal coverage. DIRECTORY-065 R5. | DIRECTORY-065 |
| `scripts/identity-gates/codex-policy.sh` | Prove config enforcement and denial delivery through the real harness. DIRECTORY-065 R6. | DIRECTORY-065 |
| `crates/lys-runner/tests/codex_policy_native.rs` | Prove config enforcement and denial delivery through the real harness. DIRECTORY-065 R6. | DIRECTORY-065 |
| `crates/lys-runner/tests/support/codex_policy_fixture.rs` | Prove config enforcement and denial delivery through the real harness. DIRECTORY-065 R6. | DIRECTORY-065 |
| `surface/identity/tests/acceptance/codex-policy.spec.ts` | Prove config enforcement and denial delivery through the real harness. DIRECTORY-065 R6. | DIRECTORY-065 |
| `docs/design/directory/PROOF-CODEX-POLICY.md` | Prove config enforcement and denial delivery through the real harness. DIRECTORY-065 R6. | DIRECTORY-065 |
| `crates/lys/src/identity/install/ports_tests.rs` | Tests that the service and broker ports come from the deployment configuration and a clash is refused | DIRECTORY-070 |
| `scripts/identity-gates/upgrade_live.py` | touched by DIRECTORY-070 R2: the old install's ports come from its deployment configuration, not layout.rs | DIRECTORY-070 |
| `scripts/identity-gates/test_upgrade_fixture.py` | touched by DIRECTORY-070 R2: fixture ports read from the old deployment configuration | DIRECTORY-070 |
| `scripts/identity-gates/UPGRADE.md` | touched by DIRECTORY-070 R2: how the upgrade gate finds the old install's ports | DIRECTORY-070 |
| `crates/lys/tests/identity_install.rs` | The first-install test on ports the operating system hands it, in folders it removes, built into the workspace target | DIRECTORY-070 |
| `crates/lys/tests/cli_tests/identity_setup.rs` | touched by DIRECTORY-070 R3: the setup test takes its service port from the operating system | DIRECTORY-070 |
| `crates/lys-identity-server/tests/teams_nested.rs` | Teams nest under a parent team and name a lead (DIRECTORY-071 R1) | DIRECTORY-071 |
| `crates/lys-identity-server/src/teams_store.rs` | Teams nest under a parent team and name a lead (DIRECTORY-071 R1) | DIRECTORY-071 |
| `crates/lys-identity-server/src/teams_api.rs` | Teams nest under a parent team and name a lead (DIRECTORY-071 R1) | DIRECTORY-071 |
| `crates/lys-identity-server/src/teams_migration.rs` | Teams nest under a parent team and name a lead (DIRECTORY-071 R1) | DIRECTORY-071 |
| `crates/lys-identity-server/src/tree_api.rs` | One tree read answers the caller's teams, agents, live sessions and profile summaries (DIRECTORY-071 R2) | DIRECTORY-071 |
| `crates/lys-identity-server/tests/tree_api.rs` | One tree read answers the caller's teams, agents, live sessions and profile summaries (DIRECTORY-071 R2) | DIRECTORY-071 |
| `crates/lys-identity-server/src/routes_table.rs` | One tree read answers the caller's teams, agents, live sessions and profile summaries (DIRECTORY-071 R2) | DIRECTORY-071 |
| `crates/lys-identity-server/src/openapi_types.rs` | One tree read answers the caller's teams, agents, live sessions and profile summaries (DIRECTORY-071 R2) | DIRECTORY-071 |
| `crates/lys-identity-server/src/mcp_requests_api.rs` | A seat asks for a declared MCP server by name (DIRECTORY-072 R1) | DIRECTORY-072 |
| `crates/lys-identity-server/tests/mcp_requests.rs` | A seat asks for a declared MCP server by name (DIRECTORY-072 R1) | DIRECTORY-072 |
| `crates/lys-identity-server/src/restart_api.rs` | A restart ends the session and starts it from the latest reviewed version (DIRECTORY-073 R1) | DIRECTORY-073 |
| `crates/lys-identity-server/tests/restart_api.rs` | A restart ends the session and starts it from the latest reviewed version (DIRECTORY-073 R1) | DIRECTORY-073 |
| `crates/lys-identity-server/tests/input_no_screen.rs` | Compaction and reminders reach a session no screen is reading (DIRECTORY-073 R3) | DIRECTORY-073 |
| `crates/lys-runner/tests/launch_folders.rs` | The Launch carries the profile's writable folders (DIRECTORY-074 R1) | DIRECTORY-074 |
| `crates/lys-identity-server/src/launch_template.rs` | The Launch carries the profile's writable folders (DIRECTORY-074 R1) | DIRECTORY-074 |
| `crates/lys-runner/src/containment_paths.rs` | The runner confines the session to those folders (DIRECTORY-074 R2) | DIRECTORY-074 |
| `crates/lys-harness-settings/Cargo.toml` | Each harness's settings are Rust generated from its published docs (DIRECTORY-075 R1) | DIRECTORY-075 |
| `crates/lys-harness-settings/src/lib.rs` | Each harness's settings are Rust generated from its published docs (DIRECTORY-075 R1) | DIRECTORY-075 |
| `crates/lys-harness-settings/src/claude_code.rs` | Each harness's settings are Rust generated from its published docs (DIRECTORY-075 R1) | DIRECTORY-075 |
| `crates/lys-harness-settings/src/codex.rs` | Each harness's settings are Rust generated from its published docs (DIRECTORY-075 R1) | DIRECTORY-075 |
| `crates/lys-harness-settings/tests/generated.rs` | Each harness's settings are Rust generated from its published docs (DIRECTORY-075 R1) | DIRECTORY-075 |
| `scripts/harness-settings/generate.py` | Each harness's settings are Rust generated from its published docs (DIRECTORY-075 R1) | DIRECTORY-075 |
| `docs/harness/claude-code.md` | Each harness's settings are Rust generated from its published docs (DIRECTORY-075 R1) | DIRECTORY-075 |
| `docs/harness/codex.md` | Each harness's settings are Rust generated from its published docs (DIRECTORY-075 R1) | DIRECTORY-075 |
| `crates/lys-identity-server/tests/harness_settings.rs` | A profile version is checked against its harness's schema and its settings read in full (DIRECTORY-075 R2) | DIRECTORY-075 |
| `crates/lys-identity-server/src/mcp_registry.rs` | Approved MCP servers are a registry (DIRECTORY-075 R3) | DIRECTORY-075 |
| `crates/lys-identity-server/tests/mcp_registry.rs` | Approved MCP servers are a registry (DIRECTORY-075 R3) | DIRECTORY-075 |
| `crates/lys-runner/src/input.rs` | Per-session ordered writes complete by signal without holding the shared session table (DIRECTORY-073 R3) | DIRECTORY-073 |
| `crates/lys-runner/tests/input_no_screen/shared_lock.rs` | A full input pipe leaves a second session answering without a clock (DIRECTORY-073 R3) | DIRECTORY-073 |
| `crates/lys-runner/src/session/restart.rs` | Prove a peer restart of its own held launch | DIRECTORY-073 |
| `crates/lys-runner/src/operations/restart.rs` | Prove a peer restart of its own held launch | DIRECTORY-073 |
| `crates/lys-runner/tests/peer_restart.rs` | Prove a peer restart of its own held launch | DIRECTORY-073 |
| `crates/lys-runner/tests/peer_restart/cases.rs` | Peer proof, held launch and operation replay through an injected process tree | DIRECTORY-073 |
| `crates/lys-identity-server/src/harness_catalogue.rs` | Lys answers the programs it can start, with their models and modes as named choices (DIRECTORY-076 R1) | DIRECTORY-076 |
| `crates/lys-identity-server/tests/harness_catalogue.rs` | Lys answers the programs it can start, with their models and modes as named choices (DIRECTORY-076 R1) | DIRECTORY-076 |
| `docs/harness/catalogue/claude-code.json` | Lys answers the programs it can start, with their models and modes as named choices (DIRECTORY-076 R1) | DIRECTORY-076 |
| `docs/harness/catalogue/codex.json` | Lys answers the programs it can start, with their models and modes as named choices (DIRECTORY-076 R1) | DIRECTORY-076 |
| `crates/lys-identity-server/src/openapi_table.rs` | Lys answers the programs it can start, with their models and modes as named choices (DIRECTORY-076 R1) | DIRECTORY-076 |
| `crates/lys-runner/src/installed.rs` | Each computer's runner reports where each described program is installed (DIRECTORY-076 R2) | DIRECTORY-076 |
| `crates/lys-runner/tests/installed.rs` | Each computer's runner reports where each described program is installed (DIRECTORY-076 R2) | DIRECTORY-076 |
| `crates/lys-identity-server/src/routes.rs` | Lys answers the programs it can start, with their models and modes as named choices (DIRECTORY-076 R1) | DIRECTORY-076 |
| `crates/lys-identity-server/src/error.rs` | Lys answers the programs it can start, with their models and modes as named choices (DIRECTORY-076 R1) | DIRECTORY-076 |
| `crates/lys-identity-server/src/error_status.rs` | Lys answers the programs it can start, with their models and modes as named choices (DIRECTORY-076 R1) | DIRECTORY-076 |
| `crates/lys-identity-server/src/openapi_types.rs` | Lys answers the programs it can start, with their models and modes as named choices (DIRECTORY-076 R1) | DIRECTORY-076 |
| `crates/lys-home/src/harness/rendering.rs` | The programme catalogue reads registration through the rendering resolver (DIRECTORY-076 R1) | DIRECTORY-076 |
| `crates/lys-identity-server/tests/profile_folders.rs` | The reviewed profile keeps optional machine placement and one writable folder, preserving old stores (DIRECTORY-074 R1) | DIRECTORY-074 |
| `crates/lys-identity-server/src/provisioning_store.rs` | The reviewed profile keeps optional machine placement and one writable folder, preserving old stores (DIRECTORY-074 R1) | DIRECTORY-074 |
| `crates/lys-identity-server/src/provisioning_api.rs` | The reviewed profile keeps optional machine placement and one writable folder, preserving old stores (DIRECTORY-074 R1) | DIRECTORY-074 |
| `crates/lys-identity-server/tests/provisioning.rs` | The reviewed profile keeps optional machine placement and one writable folder, preserving old stores (DIRECTORY-074 R1) | DIRECTORY-074 |
| `crates/lys-identity-server/tests/support/teams_before_nesting_state.rs` | Team nesting, named refusals, route schema and the historical writer proof (DIRECTORY-071). | DIRECTORY-071 |
| `crates/lys-identity-server/tests/support/teams_before_nesting_store.rs` | Team nesting, named refusals, route schema and the historical writer proof (DIRECTORY-071). | DIRECTORY-071 |
| `crates/lys-identity-server/src/teams_state.rs` | Team nesting, named refusals, route schema and the historical writer proof (DIRECTORY-071). | DIRECTORY-071 |
| `crates/lys-identity-server/src/error.rs` | Team nesting, named refusals, route schema and the historical writer proof (DIRECTORY-071). | DIRECTORY-071 |
| `crates/lys-identity-server/src/error_status.rs` | Team nesting, named refusals, route schema and the historical writer proof (DIRECTORY-071). | DIRECTORY-071 |
| `crates/lys-identity-server/src/openapi_table.rs` | Team nesting, named refusals, route schema and the historical writer proof (DIRECTORY-071). | DIRECTORY-071 |
| `crates/lys-identity-server/src/lib.rs` | Team nesting, named refusals, route schema and the historical writer proof (DIRECTORY-071). | DIRECTORY-071 |
| `crates/lys-identity-server/src/teams_nesting.rs` | Versioned nesting events and their admitted mutations (DIRECTORY-071). | DIRECTORY-071 |
| `crates/lys-identity-server/src/tree_views.rs` | Typed summaries computed from held state for the tree read (DIRECTORY-071). | DIRECTORY-071 |
| `crates/lys-identity-server/src/route_actions.rs` | created by DIRECTORY-077 R1 | DIRECTORY-077 |
| `crates/lys-identity-server/tests/route_actions.rs` | created by DIRECTORY-077 R1 | DIRECTORY-077 |
| `crates/lys-identity-server/src/agent_pass.rs` | created by DIRECTORY-077 R2 | DIRECTORY-077 |
| `crates/lys-identity-server/src/agent_pass_store.rs` | created by DIRECTORY-077 R2 | DIRECTORY-077 |
| `crates/lys-identity-server/tests/agent_pass.rs` | created by DIRECTORY-077 R2 | DIRECTORY-077 |
| `crates/lys-home/src/harness/launch_fields.rs` | changed by DIRECTORY-077 R2 | DIRECTORY-077 |
| `crates/lys-home/src/harness/codex/launch.rs` | changed by DIRECTORY-077 R2 | DIRECTORY-077 |
| `crates/lys-identity-server/src/kept_responsibilities.rs` | created by DIRECTORY-077 R3 | DIRECTORY-077 |
| `crates/lys-identity-server/tests/agent_pass_routes.rs` | created by DIRECTORY-077 R3 | DIRECTORY-077 |
| `crates/lys-identity-server/src/signed_first.rs` | changed by DIRECTORY-077 R3 | DIRECTORY-077 |
| `crates/lys-identity-server/src/agent_signature.rs` | changed by DIRECTORY-077 R3 | DIRECTORY-077 |
| `crates/lys-identity-server/src/mcp_endpoint.rs` | changed by DIRECTORY-077 R3 | DIRECTORY-077 |
| `crates/lys-identity-server/src/who_can_grant.rs` | created by DIRECTORY-077 R4 | DIRECTORY-077 |
| `crates/lys-identity-server/tests/who_can_grant.rs` | created by DIRECTORY-077 R4 | DIRECTORY-077 |
| `crates/lys-identity-server/src/grants_reach.rs` | changed by DIRECTORY-077 R4 | DIRECTORY-077 |
| `crates/lys-identity-server/tests/requests_from_agents.rs` | created by DIRECTORY-077 R5 | DIRECTORY-077 |
| `crates/lys-identity-server/src/requests_api.rs` | changed by DIRECTORY-077 R5 | DIRECTORY-077 |
| `crates/lys-identity-server/src/requests_decide.rs` | changed by DIRECTORY-077 R5 | DIRECTORY-077 |
| `crates/lys-identity/tests/grant_kinds.rs` | created by DIRECTORY-077 R6 | DIRECTORY-077 |
| `crates/lys-identity/src/grants/usage.rs` | changed by DIRECTORY-077 R6 | DIRECTORY-077 |
| `crates/lys-identity/src/grants/codec.rs` | changed by DIRECTORY-077 R6 | DIRECTORY-077 |
| `docs/design/directory/DIRECTORY-077-PROOF.md` | created by DIRECTORY-077 R7 | DIRECTORY-077 |
| `docs/design/directory/briefs/DIRECTORY-077.json` | the brief | DIRECTORY-077 |
| `docs/design/directory/briefs/DIRECTORY-077.md` | rendered markdown | DIRECTORY-077 |
| `crates/lys/src/identity/upgrade/scratch_hold_tests.rs` | the upgrade tests' hold: the FIFO a Scratch keeps open for its life, the owner it records, the sweep that names each folder it removes, and the tests that kill a Scratch's process | DIRECTORY-078 |
| `docs/design/directory/briefs/DIRECTORY-078.json` | the brief | DIRECTORY-078 |
| `docs/design/directory/briefs/DIRECTORY-078.md` | rendered markdown | DIRECTORY-078 |
| `crates/lys-secrets/src/bin/lys-secrets/app_client.rs` | the broker route that authenticates an app's virtual client credential, and issues and revokes it | DIRECTORY-081 |
| `crates/lys-secrets/src/bin/lys-secrets/app_client_tests.rs` | the route's tests: issue, authenticate, revoke, end, refusals and audit lines | DIRECTORY-081 |
| `crates/lys-secrets/src/broker/app_client.rs` | the broker's virtual client credentials: digest bound to the app's sealed client entry, issue, check, revoke, end | DIRECTORY-081 |
| `crates/lys-secrets/src/bin/lys-secrets/router.rs` | the broker's routes; gains POST /_lys/apps/client | DIRECTORY-081 |
| `crates/lys/tests/identity_install/product.rs` | the real-install product test; regains its token, userinfo and keys steps through an issued credential | DIRECTORY-081 |
| `crates/lys-secrets/src/error/app_client.rs` | the broker's refusals of an app's virtual client credential, each by its own name | DIRECTORY-081 |
| `crates/lys-secrets/src/bin/lys-secrets/save_app.rs` | the broker route that prepares an app's sealed credentials at approval; its save route is gone | DIRECTORY-081 |
| `crates/lys-secrets/src/bin/lys-secrets/save_app_tests.rs` | the prepare route's tests | DIRECTORY-081 |
| `crates/lys-secrets/src/broker.rs` | the broker; names its app client credentials module | DIRECTORY-081 |
| `crates/lys-secrets/src/broker/seal_once.rs` | sealing a broker-made value once; prepare's app id check moves beside the app client credentials | DIRECTORY-081 |
| `crates/lys-secrets/src/error.rs` | the broker's errors; gains the app client credential refusals | DIRECTORY-081 |
| `crates/lys-secrets/src/error/name.rs` | each broker error's name | DIRECTORY-081 |
| `crates/lys-secrets/src/lib.rs` | the broker crate's exports | DIRECTORY-081 |
| `crates/lys-runner/src/operations/delivery.rs` | The operations log's delivery: what an admitted act does to the harness and how its receipt is recorded. | DIRECTORY-064 |
| `crates/lys-runner/src/collector.rs` | The runner's hook collector: SessionStart, Stop and PreCompact events bound to the session. | DIRECTORY-064 |
| `crates/lys-runner/src/socket/acts.rs` | The signed Act dispatch of the runner socket, exhaustive over every act. | DIRECTORY-064 |
| `crates/lys-runner/src/session/control.rs` | Manual input and resize of a live session. | DIRECTORY-064 |
| `crates/lys-runner/src/harness_control/approval.rs` | The policy response to a harness approval request on a managed channel: never fabricated. | DIRECTORY-064 |
| `crates/lys-runner/src/session/lifecycle/managed.rs` | The managed spawn branch of a session's lifecycle: the child entry started and proved. | DIRECTORY-064 |
| `docs/design/home/launch-template.schema.json` | The launch template schema. | DIRECTORY-064 |

## Inventory

- `docs/design/identity/briefs/IDENTITY-001.json` — IDENTITY-001 revision 5, the reviewed plan these briefs carry forward: rows, walls, acceptance identifiers (ID001_*), estimates, authority and review decisions. Read, never changed.
- `docs/design/identity/briefs/IDENTITY-001.md` — its rendered twin
- `docs/design/identity/STATEMENT-2026-09-22.md` — the authority, including 'Everything is pegged to a human authority (Tom, 17:15 to 17:16)'
- `docs/design/identity/LIFECYCLE-STATES-2026-09-22.md` — the working lifecycle states: registered, active, suspended, retired
- `docs/design/identity/PROVISIONING-2026-09-22.md` — provisioning an agent under a human, the seven-step path
- `docs/design/identity/AGENT-PARITY-2026-09-23.md` — Tom's 23 September ruling: 'If a human can do it through the UI, then I want an agent to be able to do it'; whether an agent may is a permission question. Input to the directory and permission rows. Its examples are recorded as undecided and are never turned into requirements.
- `docs/design/identity/RAUTHY-BASELINE.md` — the fork baseline: pin, maintenance owner, the IDENTITY-001-UPSTREAM-AUTH-STATE blocker; read, never changed
- `docs/design/identity/CONFORMANCE.md` — Accepted mock-up behaviour map; DIRECTORY-006 maps You/delegation/access requirements and does not claim mock-up sample policy is authoritative.
- `crates/lys/src/commands/log/verify.rs` — lys log verify inclusion and consistency: the published discipline the receipt verifier keeps, pre-crypto refusals named, every cryptographic or structural failure one generic message per artifact class (lines 7-14). Unchanged by DIRECTORY-007.
- `crates/lys/src/commands/error.rs` — CliError, with LogInclusionVerificationFailed and LogConsistencyVerificationFailed at lines 189-196 as the pattern; DIRECTORY-007 R3 appends ReceiptNeedsTestKey and ReceiptVerificationFailed and changes no existing variant.
- `crates/lys-log-store/src/store.rs` — the LeafStore trait (lines 120-141): origin, extent and leaf, which the receipt verifier reads through; it never opens a Log, whose open may write a repaired pin (crates/lys-log-store/src/log.rs:99).
- `crates/lys-core/src/checkpoint/verifier_key.rs` — NoteVerifierKey::from_spec (line 92): the ORIGIN+KEYID_HEX+KEY_BASE64 key string the CLI's verify commands take; the receipt verifier parses --verifier-key and --test-key with it.
- `docs/design/lys-anchor/DECISIONS.md` — lines 333-335: the three acts reserved for Tom, publishing lys-anchor, generating a production anchor key, emitting any receipt outside a test. Read, never changed.
- `CLAUDE.md` — lines 78-80: a format is frozen once a signature is produced or a leaf is logged under it; a new version alongside, never a mutation. Governs the lys release that carries the verifier.
- `vendor/rauthy` — the maintained Rauthy fork (ADR-009), the git submodule pinned at dd61ac3c84d6b238108dc8438b53043b5177a662, the upstream v0.36.2 commit the ablative branch was created from; DIRECTORY-004 moves the pin (structure row); read here, never changed by a document row
- `crates/lys` — the lys CLI crate: Cargo.toml, src/main.rs, src/cli.rs and src/commands/ (attest, ca, key, log, inspect, files); DIRECTORY-002 adds src/identity/ and the identity subcommand to it (structure rows)
- `docs/design/decisions.json` — the project decision ledger, ADR-001 to ADR-018 at main, holding the decisions this cluster cites (ADR-003, ADR-004, ADR-005, ADR-007 to ADR-011); DIRECTORY-001 recorded that it gained the identity decisions (structure row); read here, never changed by a document row
- `docs/design/project.json` — the lys design project file at main 7b53625: one tree, the rust-build shorthand, seven legs all cadence round, and the roots map
- `scripts/design/schemas/project.schema.json` — the project schema: a leg is name, command, requires (at least one) and cadence (round or demand), the command run with no shell
- `scripts/design/gate.sh` — the design leg: validates the ledgers, checks coverage, and cmp-checks every cluster's rendered markdown against a fresh render; runs no Python tests
- `.land/gates.sh` — the repository gate: the design gate, fmt, both clippy shapes, feature-full tests and both rustdoc shapes; no leg for the surface
- `docs/design/secrets/briefs/SECRETS-002.json` — the broker brief: R4 keeps a sign-in identity and an OAuth service-access grant as distinct records, R5 puts the seat's login token into its environment at spawn; DIRECTORY-038 records a finding against it and changes nothing in it
- `crates/lys-core/src/ca/extensions.rs` — the shipped extension transport: LYS_OID_ARC, encode_extension and decode_extension carry opaque bytes; DIRECTORY-031 uses it unchanged; read, never changed
- `crates/lys-core/src/ca/authority.rs` — issue_certificate_for_request and verify_certificate_chain_at, which DIRECTORY-031 issues and verifies through; read, never changed
- `crates/lys/src/commands/ca_log.rs` — on main at fa3dd5311e98d1a751c770319e1cfe2570718c76: `lys ca issue --log`, which today takes --log, --log-key, --leaf-out and --artifact-out together or not at all, stages every output before the append, and reports log_dir, leaf_index, tree_size, root_hash, root_base64, leaf_path and artifact_path; a failure after the append is LoggedButUnwritten with the recovering `lys log prove inclusion` command. DIRECTORY-031 changes it (structure row)
- `crates/lys/tests/ca_log_tests.rs` — on main at fa3dd5311e98d1a751c770319e1cfe2570718c76: the `lys ca issue --log` integration tests and their modules under crates/lys/tests/ca_log/ (support.rs, outputs.rs, tamper.rs), which resolve openssl and python3 as hard requirements, never skips. DIRECTORY-031 adds two modules beside them (structure rows)
- `scripts/verify_inclusion.py` — the stranger's inclusion checker for lys/log-inclusion-proof/v1: Python standard library only, exit 0 verified, 1 usage or I/O, 2 verification failed, an optional expected-root-base64 third argument; it does not check the checkpoint's signature and says so. Read, never changed
- `docs/design/identity/mockup/index.v5.html` — the accepted mock-up, 1680 lines, sha256 e67622e19f450e2335029f293f18b4757801f3245758a8b667521afa120ab91f; the shell's definition; kept as the prior, never changed
- `scripts/design/render-brief.py` — the brief renderer copied from the design-system method at 3c3bac7; joins every list field of a brief onto one line with ', ', blocked_by and depends_on included
- `scripts/design/SOURCE.md` — states that the design scripts, render-brief.py among them, are copies of the design-system method at 3c3bac7

## Constraints

- **CN1** — The documents-only boundary applies to the original directory planning task, not to published implementation briefs. A published DIRECTORY implementation brief authorises only its exact requirement file walls, including named source, tests, configuration and documentation files. No implementation author may widen those walls silently. The historical IDENTITY-001 record is not rewritten. Tom’s 29 September 07:51 direction, relayed by Waffles, removes lead brief sign-off as a prerequisite to dispatch; the design gate and automatic card review and full delivery gates remain required.
- **CN2** — Development isolation: rows 02 to 05 use only disposable test identities and test provider registrations; no production tokens, real business sign-in or live Cambium participant migration.
- **CN3** — Every path written in a document of this cluster is relative to the repository root, whatever directory a session starts in; a command runs from its own tree and spells its paths from there.
- **CN4** — No structure row or files entry carries a root token; a file in another repository is named in a requirement's spec with its owner.
- **CN5** — A live demonstration to Tom is never an acceptance criterion of a loop requirement; it is a verification step a person performs after the row lands.
- **CN6** — The live demonstrations ID001_LINK_LIVE (after row 03) and ID001_DIRECTORY_LIVE (after row 05) are mandatory operator hold points: the brief that follows each is blocked by it until Tom's demonstration receipt is recorded; a loop completion never stands in for one.
- **CN7** — Revision 5's ceiling stands: 48 focused implementer hours for IDENTITY-001 (row 01 1.5, closed; 02 8; 04 10; 03 10; 05 6; 06 4; 07 5; total 44.5, contingency 3.5), with IDENTITY-002's 4 hours outside it. An overrun is reported as soon as it is known, and Waffles takes any ceiling change to Tom (docs/design/identity/briefs/IDENTITY-001.json:24).
- **CN8** — There is no one-build-per-lead rule: Tom withdrew that restriction on 29 September at 07:32, relayed by Waffles. Independent card builds may run concurrently. Heavy builds and full gates run at the declared venue. Concurrent dispatch does not supply missing prerequisite code: dependent changes need the actual prerequisite implementation in the integration tree, and the integrated commit passes the full delivery chain before landing.
- **CN9** — A row that needs a file outside its wall stops and names it. The lead amends the handwritten brief on main and passes the design gate before that additional file is edited; a new module needs an exact file manifest within its named responsibility. Under Tom’s 29 September 07:51 direction, relayed by Waffles, no separate lead review or sign-off is required before dispatch. Automatic card review and full delivery gates remain required. An active run keeps its captured contract; changed words must be reconciled with its result before publication or landing.
- **CN10** — Rows 02 to 05 run on Rauthy v0.36.2 under Waffles' ruling of 15:36:25; each development install checks current releases and advisories and records the accepted exception; IDENTITY-001-UPSTREAM-AUTH-STATE binds real sign-in and install, rows 06 and 07 (docs/design/identity/briefs/IDENTITY-001.json:18, docs/design/identity/briefs/IDENTITY-001.json:46).
- **CN11** — Step 1 is directory records, sign-in and the minimum signed identity audit. SpiceDB is installed and checked in row 02 and enforces nothing in step 1; live capability policy and its enforcement are step 2's, and running SpiceDB is not permission enforcement (docs/design/identity/briefs/IDENTITY-001.json:29-30).
- **CN12** — DIRECTORY-006 is implementation work after the frozen planning task: its source paths are only executable after the DIRECTORY-002/003 foundations are implemented and their integration manifests are reconciled. R6 also waits for the standalone surface foundation. Do not dispatch from a schema-valid but dependency-blocked brief.
- **CN13** — An audit receipt is signed by the service that performed the operation, never by the identity it is about; a verifier is given that service's public key and never takes a key the receipt carries as its authority (P8).
- **CN14** — The appended leaf is the signed event, appended in the order the operations happened; the receipt is returned beside it carrying the log coordinate the append yielded and the payload commitment, and is never itself appended (docs/design/identity/briefs/IDENTITY-001.md:49).
- **CN15** — Receipts are for changes only: a permission check that is answered and changes nothing leaves no receipt and never enters the signed receipt log; when step 2 brings enforcement, allows and refusals go to a separate access record step 2 defines, refusals always, allows decided with step 2 (ADR-024).
- **CN16** — A development install emits test receipts, signed with a test key and carrying a test tag, inside development isolation, and never performs the act docs/design/lys-anchor/DECISIONS.md:335 reserves; a verifier refuses a test-tagged receipt by name unless given the test key explicitly, saying to supply it, and a real key never verifies a test receipt, so a test receipt never passes as real; harness tests stay as well (ADR-026).
- **CN17** — The receipt verifier keeps the published discipline of crates/lys/src/commands/log/verify.rs: every cryptographic or structural verification failure collapses to one named refusal class for the receipt artifact, so tamper classes are indistinguishable in the output; the pre-crypto, actionable refusals stay named on their own: malformed JSON, a malformed key string, and a test-tagged receipt presented without the test key.
- **CN18** — The receipt's signed encoding, tag and content type are written in docs/design/identity/IDENTITY-EVENTS.md under its recorded joint review, never in a brief; a code row of the receipt starts only when that file is on main with its version, typed payloads, named commitment hash and recorded review; a lys release carrying the verifier is a separate act after that version is ratified, never before (CLAUDE.md:78-80), and landing on main publishes nothing.
- **CN19** — An identity's lifecycle state is folded from its signed transition records and never set directly: no API, migration or repair writes a state, a read answers the fold or refuses by name at the coordinate of a record the table refuses, and the state type has exactly the four values registered, active, suspended and retired, provisioned being a view (ADR-027).
- **CN20** — A suspension or retirement takes effect at the next check; the directory's resolution of a Rauthy session at sign-in and at refresh is a check; no code of this cluster calls Rauthy to revoke, end or invalidate a session, keeps a list of sessions to end, or rolls back an admitted action; an at-once revocation is a later revision of the suspend transition (ADR-028).
- **CN21** — A retired identity's state, its transitions and the record that put it there are readable exactly as a live identity's; no row of this cluster deletes, hides, redacts or truncates an audit record; a retired identity is never reactivated, a new identity being made instead (ADR-011, ADR-027).
