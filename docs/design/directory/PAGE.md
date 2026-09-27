# directory — what was asked, what it means, and what was written

## The words, as they were typed

Write the brief for this card in the directory cluster (docs/design/directory/briefs), at the next free id past lys main and every open brief and draft branch, checked with git ls-remote immediately before writing. The card makes a session its own record in the directory, separate from the enduring agent it belongs to. A session is created when an agent's session starts, points at its agent by the agent's directory id, and carries its own session credential. Starting a session never creates an agent. This is road step 2's session half, named by the provision card's brief (6AWx1-JR, run 28d15c0d) as the row its second-session proof waits on, and session start sits in Chippy's release 3 at STATEMENT-2026-09-22.md line 156. Done when a second session for the same agent presents the same enduring identity with a new session credential, and the first session's credential is refused once that session has ended. The form of the session credential is the open question the provision brief records, a per-session lys certificate, a SpeaksFor delegation or an ADR-001 handle. This brief decides it, as a technical choice the lead makes and writes down with its reason, and it is distinct from the agent's one certificate (ADR-008). The started agent reports back to the directory (ADR-007), and the directory checks it. The brief builds that presentation step, which the provision brief names as this card's. Hold to DIRECTORY-003 (each agent under its responsible person, every identity change a signed event) and to the typed capability claim card (hhAN8h77) for what the agent's certificate carries. The build is blocked until DIRECTORY-003 and the provision card have landed, each checked by a command a stranger can run. Keep to the method (scripts/design/validate.py, check-coverage.py, render-cluster.py, run by scripts/design/gate.sh). If a sentence of DESIGN.md or CHECKLIST.md is open or contradicted by the repository as it stands, quote it as a question for the lead rather than rewriting the design around it. The card is built from this brief only after the lead signs it off on the card.

Rulings of the lead, Archie, given on 27 September 2026 to the run d5055cc1-73dc-49b5-ad25-449f195b8796 in answer to its rounds. That run took every answer and then failed before writing, when the account pool refused every session. They are settled here, and the author reopens none of them.

None of the three. The session credential is a random value the directory issues when the session is created. The directory keeps only its hash, on the session record, and the session presents the value to the directory. The reason is that in this card the directory is the only party that checks a session credential. Nobody verifies it offline, so a signed artifact buys nothing here and would freeze a format before anything needs it. A handle waits on the unbuilt secrets broker (SECRETS-002). SpeaksFor is framed for an offline root key and a seat subject, and delegation/mod.rs says not to invent a consumer. Refusing is a change to the record's state, so ending a session refuses its credential at once. The brief records this choice and its reason as an ADR at the next free id, and names a signed session credential as later work, for when a party other than the directory must check a session. Answered by Archie, lead for the identity line.

When the started agent first reports back and proves it is the agent. Rendering a start command creates nothing, so a command that is rendered but never run leaves no session. That fits ADR-007, because the product runs nothing and a start command never carries a credential's value. The report-back is authenticated by the agent's enrolled key. The directory then creates the session record pointing at the agent's directory id, issues the session credential and returns its value once, in that answer only. Answered by Archie, lead for the identity line.

Two acts end a session in this card. One is the agent reporting its end with its session credential, and the other is an operator stopping the session on the directory. Each is recorded with its actor, and after either one the credential is refused, naming the ended session. Expiry is not in this card. A crashed agent's session stays open until an operator stops it. The brief says that plainly, and names lapsing a stale session as its own card. Acceptance lines cover a credential refused after the agent's end, a credential refused after an operator's stop, and a second session for the same agent admitted while the first is open. Answered by Archie, lead for the identity line.

Yes to both. A session starts only for an agent whose certificate has been issued, and the report-back is authenticated by the key enrolled for that certificate (ADR-032). An agent whose certificate is not issued is refused a session, naming the act that answers it, which is issuing that agent's certificate. So the build is also blocked on card hhAN8h77 landing, as a third blocker beside DIRECTORY-003 and the provision card. Each is checked by a command a stranger can run, reading that card's own execution record for landed. Answered by Archie, lead for the identity line.

It stands beside them as a step-2 increment, and neither is rewritten. One line is appended beneath the non-goal at design.json:92, and one beneath P6. Each names this card's session record and credential as step-2 work brought forward under the lead's ruling of 27 September 2026. This is the same form I ruled for CN11 in the revocation brief. DESIGN.md does not gain the road, as ruled in run 28d15c0d. Answered by Archie, lead for the identity line.

The proof is server-side, plus a command a stranger can run that lists an agent's sessions with their states. Neither the sessions screen nor the agent's file is in this card. The brief names the sessions screen as its own card, which reads the records this card makes. Answered by Archie, lead for the identity line.

Sign a challenge the directory issues, and the directory's identity. Before reporting back, the agent asks the directory for a challenge. That is a random value the directory issues once, for that agent, with a short expiry, and keeps until it is used or expires. The lys/session-start/v1 statement signs the agent's directory id, the directory's own identifier and that challenge. The directory admits the proof only when the challenge is one it issued to that agent, unused and unexpired, and the directory named is itself. It marks the challenge used in the same act that creates the session. A reused, expired or foreign challenge, or a proof naming another directory, is refused by name and creates nothing. Acceptance lines cover a replay of an accepted proof, a proof after the challenge expired, a proof for another directory, and a proof whose challenge was issued to a different agent, each refused by name. The reason is that the tag freezes once signed, so freshness and audience have to be in it from the first version. Answered by Archie, lead for the identity line.

Yes, 60 seconds, accepted at 60 and refused as challenge_expired at 61, as the brief has it. A separate ruling, from the start card's round 1, also belongs in this brief before it is written. The signed lys/session-start/v1 message carries the launch record id as well, so it signs the tag, the agent id, the launch record id and the nonce. No agent signs under the tag yet, so the change is free now. The start card ties running to the command given through that id. R1, R4 and the acceptance lines carry the new field, and a report whose launch record id does not match the challenge's launch record is refused by name. Answered by Archie, lead for the identity line.

Admin-only in this card, as DIRECTORY-003 R3 and P9 admit only the configured administrator in step 1. 'A command a stranger can run' means the command is written out in full and reproducible from the brief, not that anyone may call it. The agent's responsible person gains the listing when step 1's admission admits them, and the brief names that under further units not written. Everyone else stays refused by name. Answered by Archie.

Rulings of the lead, Archie, given on 27 September 2026 to the run 8c1bee6c-7421-4bf2-9753-922ad3546590 in answer to its rounds. That run took every answer and then failed before writing, when the account pool was at its usage limit. They are settled here, they stand over any earlier ruling above that they contradict, and the author reopens none of them.

Five fields. The signed lys/session-start/v1 message carries the tag, the agent id, the directory's own identifier, the launch record id and the nonce. The later sentence adds the launch record id and removes nothing, and the acceptance line for a proof naming another directory stands. Answered by Archie, lead for the identity line.

Carry it as supplied, and leave the existence check to the start card, so the two cards do not depend on each other. The challenge names the launch record id its requester gives, and this card refuses by name a report whose launch record id does not match the challenge's. The start card, which makes launch records, adds the check that the record exists and belongs to the agent, with its own acceptance line. The brief names that as the start card's and records that until it lands, an invented id yields only a session whose report names a record nobody made. Answered by Archie.

Superseded and closed. The d5055cc1 draft and its DIRECTORY-015 and ADR-046 are closed by this brief. This brief takes its ids by the next-free rule, re-checked immediately before writing. The start card names this brief by its run, d5055cc1, whatever id it lands under, as I told Buckley, so its reference follows without a renumbering by hand. Answered by Archie.

A new session is refused by name to a retired agent and to an agent whose certificate has been revoked. Suspension stays open for Tom, so this card does not refuse on it, and the brief says so. This card ends no open session. The next report-back authenticated by a revoked certificate's key is refused by name. Ending open sessions is the emergency-stop card's, and the brief names it. Acceptance lines cover a retired agent refused, a revoked certificate refused, and a revoked certificate's report-back refused. Answered by Archie.

Confirmed. The certificate revocation card, DIRECTORY-012 on draft/directory/82386e20, is R9's blocker, checked by a command a stranger can run against lys main that finds its fold. R1 to R8 go ahead without it. No other arrangement: a revoked certificate is refused only through that fold, never through a second list kept here. Answered by Archie, lead for the identity line.

CN1 binds only the documents-only planning briefs. Record that by appending a line after CN1, as P6 was appended, and leave CN1's text as it is: from DIRECTORY-003 on, the directory's code briefs create and change code under crates/lys-identity and crates/lys-identity-server, and CN1 does not bind them. Answered by Archie.

In-process only, as the brief has it. DIRECTORY-003 defines no scriptable sign-in, so this brief does not invent one. It names the deployed-directory run under further units not written, attached to the card that brings a scriptable sign-in, and the acceptance runs the command against the in-process directory the test starts. Answered by Archie, lead for the identity line.

This draft keeps DIRECTORY-021 and RM-041, because it wrote them first. The access-graph brief 526867e5 renumbers, and its round 2 ruling already moves it to the next free ids past every head (DIRECTORY-022 and RM-042 today). Nothing moves here. Answered by Archie, lead for the identity line.

Verify first. The report-back verifies the proof's signature under the agent's enrolled key before it answers anything else. Anything unsigned, badly signed, or naming an agent the directory does not hold or one with no enrolled key is refused as proof_invalid with one body, so an unauthenticated caller learns nothing about which agents exist or their state. certificate_not_issued is not excepted. A key is enrolled with the agent, apart from the certificate, so the signature can be checked first, and that refusal goes only to a correctly signed proof. Only a correctly signed proof then gets the state-revealing refusals, in R4's order: agent_retired, certificate_not_issued, wrong_directory, challenge_reused, challenge_unknown, challenge_foreign_agent and challenge_expired. An unsigned or invalid request never changes, consumes or drops a challenge. An acceptance line shows that the unsigned forms of each of those cases all return the identical proof_invalid body and leave the challenge store byte-identical. One more ruling rides with this round, on ids. 7494cb8b wrote DIRECTORY-021 at 16:54, before this brief took it at 16:55, and the first writer keeps an id. This brief moves off DIRECTORY-021 and RM-041 to the next free ids past main and every brief, draft and hand head, re-checked with git ls-remote immediately before writing. That withdraws my earlier ruling that it keeps them, which rested on a wrong reading of the times. Answered by Archie, lead for the identity line.

## What the survey found, and its angles

The words ask for a design-system brief in the directory cluster, at the next free id, for road step 2's session half. A session becomes its own directory record, apart from its enduring agent. The directory creates it only when the started agent asks for a single-use challenge and reports back with a signed lys/session-start/v1 proof. That proof has five fields (the tag, the agent id, the directory's identifier, the launch record id and the nonce) and is checked under the agent's enrolled key. The session carries a random credential the directory issues once and keeps only as a hash. Two acts end a session, the agent's reported end or an operator's stop, and after either one the credential is refused by name. The lead's rulings of 27 September settle almost every design choice. The author writes them into a brief, a new ADR, a roadmap row, checklist items and stories, plus three appended lines in design.json (non-goal, P6, CN1). The build waits on DIRECTORY-003, the provision card and the typed capability claim card, and R9 also waits on the revocation fold.

### What the tree holds

- `docs/design/directory/briefs/` — Where the new brief's .json and rendered .md land. Main holds DIRECTORY-001 to 006 and 008. The highest id on any head is DIRECTORY-024 (draft/directory/2da3cf8c), so the next free id today is DIRECTORY-025.
- `docs/design/directory/design.json:92` — The non-goal 'Road step 2 onward: ... session launch and stop, credential handles ...'. The ruling appends one line beneath it naming this card's session record and credential as step-2 work brought forward, and leaves the non-goal's text as it is.
- `docs/design/directory/design.json (principles P6, constraints CN1, P9, CN6, CN7)` — P6 reserves sessions and implements neither session history nor launch. CN1 reads 'Documents only'. Lines are appended beneath P6 and after CN1. P9 and C13 refuse every non-administrator mutation caller in step 1, but the agent's report-back and reported end are mutations by the agent. CN6 is the Tom hold point, and CN7 is the 48-hour ceiling.
- `docs/design/directory/DESIGN.md, CHECKLIST.md, USER-STORIES.md` — Rendered from design.json, checklist.json and stories.json by render-cluster.py. gate.sh fails if a committed .md differs from what its JSON renders to.
- `docs/design/directory/checklist.json and stories.json` — New checklist items and stories for the brief. Main ends at C30 and has no S ids yet. The highest ids on any head are C185 and S76 (draft/directory/2da3cf8c).
- `docs/design/decisions.json` — The ADR recording the session credential choice and its reason goes here. Main ends at ADR-018. ADR-074 is on draft/home/aefa8236. ADR-075 is held only by this line's own superseded draft, 8c1bee6c.
- `docs/design/roadmap.json` — The row that carries the card. Main ends at RM-016 and the highest on any head is RM-046 (2da3cf8c). The provision brief (brief/directory/28d15c0d) already made RM-029, 'A session is its own record, issued at session start and pointing at its agent', and names it as the row its second-session proof waits on.
- `scripts/design/gate.sh, validate.py, check-coverage.py, render-cluster.py` — The method's gate. gate.sh validates decisions.json and project.json and each cluster's schema, coverage and render, but does not validate roadmap.json. The earlier draft therefore added an explicit validate.py run on docs/design/roadmap.json.
- `brief/directory/28d15c0d: docs/design/directory/briefs/DIRECTORY-011.json` — The provision card. It writes road adjustment 3 into DIRECTORY-CONTRACT.md as rule 5 ('a second session ... presents the same enduring agent id with a new session credential'). It records the three readings of the session credential as open, and names the presentation step as RM-029's card. It is being built on hand/DIRECTORY-011 (latest commit 2026-09-27T06:56Z) and has not landed.
- `docs/design/directory/briefs/DIRECTORY-003.json` — The directory contract, the signed event envelope, R3 admission (the configured administrator only, P9) and R5 lifecycle. It has no execution record on main. Its build sits on hand/DIRECTORY-003, 11 ahead of main and 2 behind.
- `hand/DIRECTORY-003: crates/lys-identity-server/src/session.rs` — A module named session already exists: the operator's signed-in cookie session (32 random bytes, in memory). The agent's session record needs a name that cannot be confused with it.
- `hand/DIRECTORY-003: crates/lys-identity/src/directory.rs, event.rs, projection.rs` — Where session start, end and stop events and the session projection would land. directory.rs is 11,712 bytes, with register_agent, transition and record already there. The 500-line limit applies.
- `draft/directory/0f34978d: DIRECTORY-012.json (typed capability claim card)` — R5 enrols one Ed25519 key per agent beside its record (agent_key_enrolled), R6 issues the certificate, and R7 replaces a key by an audited act. The report-back verifies under this enrolled key. Its decisions are ADR-062 and ADR-063 on this draft. The ADR-032 the words cite exists only on the older draft 858c0e71.
- `draft/directory/82386e20 DIRECTORY-012 and brief/directory/1e30a4cb DIRECTORY-013` — Both carry the certificate revocation brief under the same title, and hand/DIRECTORY-013 is building it ('DIRECTORY-013 R1: certificate revocation contract'). R9's blocker check should find the fold on lys main whichever id it lands under.
- `crates/lys-core/src/delegation/mod.rs:45-46` — 'no consumer ... do not invent a consumer for it' is part of the reason SpeaksFor is not the session credential. The boundary is that crates gain no SpeaksFor consumer.
- `docs/design/WIRE-FORMATS.md` — The register of frozen and proposed wire formats, 208 lines, with a decision log of Tom's ratifications. lys/session-start/v1 is a new signed tag and appears nowhere in docs or crates today.
- `docs/design/identity/STATEMENT-2026-09-22.md:156` — Chippy's release 3, 'Create and operate an agent ... start a session'. Confirmed at line 156.
- `draft/directory/8c1bee6c: DIRECTORY-021.json` — This line's previous draft, 292 rendered lines and 66 KB, with nine rows R1 to R9, 13 checklist items, 4 stories and ADR-075. It is the closest template, but has to move off DIRECTORY-021 and RM-041.

### What was already decided

- ADR-003 — Everything is pegged to a human authority. An agent is provisioned under a person, and a session must never widen the agent's grants.
- ADR-001 — A handle bound to identity is swapped at the door. The ruling rejects it for the session credential because the broker (SECRETS-002) is unbuilt.
- ADR-004 — The engine that starts a seat can be any engine (manifold, aion, a customer's). The report-back must not assume one engine.
- ADR-007 — The product gives a start command and never runs it. The command never carries a credential's value, and a started agent reports back. This is why session creation happens at report-back, not at render.
- ADR-008 — The agent's one lys certificate is shown on its file. The session credential is distinct from it.
- ADR-011 — Proposed lifecycle states. Retired is permanent, so a retired agent is refused a session. Suspension semantics are open for Tom.
- ADR-062 / ADR-063 (draft 0f34978d) — The certificate half of road step 2 sits in the directory cluster, and certificate validity is 30 days by default and 90 at most. These are the claim card's current decisions.
- ADR-032 (draft 858c0e71) — 'An agent's capability certificate is per enduring agent and issued by the directory'. This is the id the words cite, and it exists only on the older claim draft.
- P6 — A future execution id refers to its enduring identity, and step 1 implements neither session history nor launch. The ruling appends a step-2 line beneath it.
- P9 — Only the configured administrator is bootstrapped, and every other mutation caller is refused in step 1. This bears on the agent-authenticated report-back and end.
- CN1 — Documents only. The ruling appends a line saying CN1 binds only the documents-only planning briefs.
- CN6 — The brief that follows the live demonstration after row 05 waits for Tom's receipt.
- CN7 — The 48-hour ceiling of revision 5. New increments are stated beside it.
- CN9 — A wall for a new module needs an exact file manifest reviewed before its row starts.
- CN11 — Live capability policy and its enforcement belong to step 2.
- CN12 — Nothing is dispatched from a brief whose dependencies block it.
- non-goal design.json:92 — 'Road step 2 onward ... session launch and stop, credential handles'. A line is appended beneath it and its text is not rewritten.
- DIRECTORY-003 R3 — Admits only the configured administrator. The listing command is admin-only under it.
- DIRECTORY-011 (run 28d15c0d) — Contract rule 5 is the second session. The credential form is left open with three readings, and the presentation step belongs to RM-029's card.
- RM-029 (brief/directory/28d15c0d) — 'A session is its own record, issued at session start and pointing at its agent', an idea row the provision brief points at.
- CLAUDE.md: Wire formats are forever / adversarial review — A new signed tag freezes once signed, and a cryptographic change needs a constructed-attack review before it lands.

### What was measured

- Branch heads read by git ls-remote (main, brief/, draft/, hand/): 167 of 235 heads on origin
- Highest DIRECTORY id on any head: DIRECTORY-024 (draft/directory/2da3cf8c), so next free is DIRECTORY-025
- Highest RM id on any head: RM-046 (draft/directory/2da3cf8c), so next free is RM-047. Main ends at RM-016.
- Highest ADR id on any head: ADR-075, held only by draft/directory/8c1bee6c (this line's superseded draft). The next highest is ADR-074 on draft/home/aefa8236. Main ends at ADR-018.
- Highest checklist and story ids in the directory cluster on any head: C185 and S76 (draft/directory/2da3cf8c). Main has C1 to C30 and no S ids.
- Heads holding DIRECTORY-021: 2: 7494cb8b (roles, 16:54, the first writer) and 8c1bee6c (this brief's previous draft)
- Heads holding a DIRECTORY-012 brief: 4. Three are the typed capability claim (0f34978d, b38766d2, 858c0e71) and one is revocation (82386e20). Revocation is also DIRECTORY-013 on 1e30a4cb and on hand/DIRECTORY-013.
- DIRECTORY-003 execution record on main: none ('no execution record')
- crates/lys-identity and crates/lys-identity-server on main: absent. They exist only on hand/DIRECTORY-003: 17 files in lys-identity and 11 in lys-identity-server.
- Size of crates/lys-identity/src/directory.rs on hand/DIRECTORY-003: 11,712 bytes
- Existing 'session' module on hand/DIRECTORY-003: 1: crates/lys-identity-server/src/session.rs, 2,904 bytes, the operator's cookie session
- Occurrences of 'session-start' in docs/ and crates/ of this tree: 0
- STATEMENT-2026-09-22.md line 156: '3. **Create and operate an agent.** ... provision, start a session ...' (confirmed)
- Previous draft DIRECTORY-021 (8c1bee6c): 9 requirements, 44 acceptance lines, 7 blocked_by entries, 8 boundaries, 7 verification lines, 13 checklist items, 4 stories
- Directory cluster documents on main: DESIGN.md 196 lines, design.json 770, checklist.json 185, CHECKLIST.md 49, stories.json 83, USER-STORIES.md 35
- Difference in docs/design between local HEAD 7b53625 and origin main fa3dd531: none. HEAD is an ancestor of main.

### What it means for the other projects

- aion — The card runs through aion's chain (brief_card, sign-off, card_build_v3, src_pr, src_land). The start card, which makes launch records, depends on this brief by run d5055cc1 and adds the launch-record existence check itself. Aion, as one engine that starts agents (ADR-004), would carry an agent that asks for a challenge and reports back. Nothing in aion changes in this card.
- cambium — The card lives on Cambium's board and is signed off by the lead there. The card changes nothing in Cambium's product.
- method — The brief is measured by the method's validate.py, check-coverage.py and render-cluster.py through scripts/design/gate.sh. It must fit brief.schema.json and render byte-identically. The method itself does not change.

### The decisions it stands on

- ADR-003 (honour) — A session never widens the agent's grants and never creates an agent. Each session is reached through an agent kept under its responsible person.
- ADR-007 (honour) — The session is created at report-back, never at render. A start command never carries the credential's value.
- ADR-008 (honour) — The session credential is distinct from the agent's one certificate, which stays the claim card's.
- ADR-011 (honour) — A retired agent is refused a session. Suspension is left open for Tom, and a session never changes the agent's lifecycle state.
- ADR-004 (honour) — The challenge and report-back must not assume a particular engine started the agent.
- ADR-001 (honour) — The handle is not used for the session credential while the broker (SECRETS-002) is unbuilt. The decision itself stands.
-  (new) — An ADR at the next free id records that a session credential is a random value the directory issues when a signed, challenge-bound report-back creates the session. The directory keeps only its hash, and ending the session refuses it at once. The reason is that the directory is the only party that checks it. The ADR names a signed session credential as later work.
-  (new) — lys/session-start/v1 is a new signed domain-separation tag. It covers the tag, the agent id, the directory identifier, the launch record id and the nonce, and freezes once an agent signs under it.

### What it requires

- docs/design/directory/briefs/DIRECTORY-0NN.json and .md exist at the id that is next free past main and every brief, draft and hand head, as checked by git ls-remote immediately before writing. The id is DIRECTORY-025 today, and neither DIRECTORY-021 nor RM-041 is used.
- A new ADR at the next free id records the random, hashed, directory-issued session credential and its reason. It also records why a per-session certificate, SpeaksFor and an ADR-001 handle were not chosen, and names a signed session credential as later work.
- A roadmap row carries the card, and roadmap.json and decisions.json each pass validate.py.
- design.json gains exactly three appended lines: one beneath the design.json:92 non-goal and one beneath P6, each naming this card's session record and credential as step-2 work brought forward under the lead's ruling of 27 September 2026, and one after CN1 saying CN1 binds only the documents-only planning briefs. No existing text is changed.
- sh scripts/design/gate.sh exits 0 with the new brief, checklist items and stories, and DESIGN.md, CHECKLIST.md and USER-STORIES.md re-rendered.
- The brief lists DIRECTORY-003, the provision card and the typed capability claim card (hhAN8h77) as blockers. For each it gives a command a stranger can run that reads that card's own execution record on lys main for 'landed'.
- The brief makes R9 alone wait on the certificate revocation card's fold being on lys main, checked by a command that finds the fold. The revocation state is read only through that fold.
- The brief specifies the challenge: random, issued once per agent with the launch record id its requester supplies, living 60 seconds (accepted at 60, refused as challenge_expired at 61), and marked used in the same act that creates the session.
- The brief specifies refusal order. First, anything unsigned or badly signed, naming an unknown agent, or naming an agent with no enrolled key is refused as proof_invalid with one identical body and leaves the challenge store byte-identical. Then, in order: agent_retired, certificate_not_issued, wrong_directory, challenge_reused, challenge_unknown, challenge_foreign_agent, challenge_expired. A launch record id that does not match the challenge's is refused by name.
- The brief has acceptance lines for: a second session for the same agent admitted while the first is open, presenting the same enduring agent id with a new credential; the first credential refused after the agent's reported end, and refused after an operator's stop, each naming the ended session; a replayed proof, an expired challenge, a proof naming another directory and a challenge issued to a different agent, each refused by name; a retired agent refused; an agent with a revoked certificate refused; and a report-back under a revoked certificate's key refused.
- The brief writes out in full an admin-only listing command, run against the in-process directory the test starts, that prints an agent's sessions with their states and refuses every other caller by name.
- The credential's value appears only in the one answer that issues it. It never appears in an event, log, error, fixture, start command, launch record or document.
- The brief records that suspension, expiry and stale-session lapse, ending open sessions on revocation or emergency stop, the launch-record existence check and the sessions screen are out of this card, and names each as its own card.

### What must not change

- The text of the design.json:92 non-goal, P6 and CN1 is not rewritten; lines are only appended.
- DESIGN.md does not gain the road or its adjustments.
- STATEMENT-2026-09-22.md, the IDENTITY-001 files and DIRECTORY-003's brief do not change.
- lys-core, lys-log-store, lys/attestation/v2, lys/delegation/v1 and every other shipped or drafted tag and content type stay unchanged. SpeaksFor gains no consumer.
- The agent's one certificate and what it carries stay the typed capability claim card's (ADR-008).
- Starting a session never creates, registers or changes an agent, and never widens its grants or changes its lifecycle state.
- The ids the first writer took (7494cb8b's DIRECTORY-021, 526867e5's DIRECTORY-022) are not taken.
- Only disposable test identities and keys are used (CN2). A live demonstration is never an acceptance criterion (CN5).
- No second revocation list is kept here. Revocation is read only through the revocation card's fold.
- The author reopens none of the lead's rulings of 27 September 2026.

### What we must put in place first

- Re-run git ls-remote and read every main, brief, draft and hand head's briefs, decisions.json, roadmap.json, checklist.json and stories.json immediately before writing, to fix DIRECTORY, ADR, RM, C and S ids. Today they are DIRECTORY-025, ADR-075/076, RM-047, C186 and S77.
- Have the lead answer the open product decisions (P9 for agent-authenticated mutations, CN6's hold point, the revocation card's id, and ratification of the lys/session-start/v1 wire format) before the brief is written.

### The risks

- The id race is real and fast: 8c1bee6c lost DIRECTORY-021 to 7494cb8b by one minute, and heads move within the hour, so an id checked earlier than immediately before writing can collide again.
- Blocker ids are unstable. DIRECTORY-012 names the claim card on three drafts and the revocation card on another, and revocation is also DIRECTORY-013. A blocker check keyed on an id rather than a title or fold can point at the wrong brief or report 'landed' for the wrong card.
- DIRECTORY-003 and DIRECTORY-011 are being built on hand/ branches (hand/DIRECTORY-003, hand/DIRECTORY-011), outside the chain. An execution record reading 'landed' may never be written, which would leave the card blocked indefinitely.
- ADR-032, cited by the words, is not the claim card's decision on its latest draft (0f34978d uses ADR-062 and ADR-063), so a citation by number can point at nothing once the claim card lands.
- Once any agent signs under lys/session-start/v1 the tag is frozen, so a field order or encoding mistake cannot be fixed without a v2. The adversarial review has to happen before R4's code.
- The name 'session' already means the operator's cookie session in crates/lys-identity-server/src/session.rs, so a careless module name mixes up two credentials with very different security properties.
- An agent-authenticated mutation may be read as a breach of P9 and C13 at review if the brief does not settle it.
- The provision brief points at RM-029, so a new roadmap row for the same work leaves two rows for one card.
- Refusing proof_invalid with one identical body is easy to break with a timing or length side channel. The acceptance line checks body identity and challenge-store bytes, not timing.

### Still open

- Does P9 ('every other mutation caller is refused in step 1', also C13) give way for this card? The agent, authenticated by its enrolled key, creates and ends its own session record without the configured administrator. Should a line be appended beneath P9 as for P6? The sentence of the words it stands on: "The started agent reports back to the directory (ADR-007), and the directory checks it.". Why only the lead can settle it: docs/design/directory/design.json P9 and checklist.json C13 refuse every non-administrator mutation caller in step 1. The report-back and the agent's reported end are mutations by the agent. Without a ruling, the brief either contradicts P9 or the agent cannot start or end a session. Only the non-goal and P6 are being amended.
- Does CN6's hold point (Tom's ID001_DIRECTORY_LIVE demonstration receipt after row 05) block this brief's build, or does it bind only the brief that follows row 05 in IDENTITY-001's order? The sentence of the words it stands on: "The build is blocked until DIRECTORY-003 and the provision card have landed, each checked by a command a stranger can run.". Why only the lead can settle it: docs/design/directory/design.json CN6 says the brief that follows row 05 waits for Tom's demonstration receipt. The words name only the landed blockers. If CN6 binds, the build waits on Tom and a fourth blocker is added.
- The revocation card is now also DIRECTORY-013 on brief/directory/1e30a4cb and is being built on hand/DIRECTORY-013. Should R9's blocker name it by title and fold rather than as DIRECTORY-012 on draft/directory/82386e20? The sentence of the words it stands on: "The certificate revocation card, DIRECTORY-012 on draft/directory/82386e20, is R9's blocker, checked by a command a stranger can run against lys main that finds its fold.". Why only the lead can settle it: The tree contradicts the id. 82386e20's DIRECTORY-012 was renumbered (its last commit renumbered to RM-022/ADR-037). The same title now sits as DIRECTORY-013 on 1e30a4cb, and DIRECTORY-012 is also the claim card's id on three drafts. A blocker that names a stale id points a stranger at the wrong brief.
- Before any agent signs under lys/session-start/v1, does it get a proposal row in docs/design/WIRE-FORMATS.md and a recorded ratification (and whose), as the claim card does for lys/agent-capability/v1? Or is the ADR enough? The sentence of the words it stands on: "The reason is that the tag freezes once signed, so freshness and audience have to be in it from the first version.". Why only the lead can settle it: docs/design/WIRE-FORMATS.md is the register of frozen and proposed formats, with a decision log of Tom's ratifications, and draft 0f34978d's R3 records a format ratification there. Whether this tag needs the same step changes the build's gates and who must act before R4.

### The units beyond the first

- Sign a session credential for parties other than the directory — It is ruled out of this card because only the directory checks a session today. A signed artifact would freeze a wire format before anything needs it.
- Lapse a stale session whose agent never reported its end — Expiry is not in this card, and a crashed agent's session stays open until an operator stops it. The lead named lapse as its own card.
- The sessions screen, reading the records this card makes — The proof here is server-side plus a command. The screen is a separate journey with its own acceptance.
- Let an agent's responsible person list its sessions — The listing is admin-only while step 1's admission admits only the configured administrator. It widens when that admission widens.
- Run the session listing against a deployed directory — It needs a scriptable sign-in that DIRECTORY-003 does not define, so it attaches to the card that brings one.
- End an agent's open sessions on emergency stop, revocation or retirement — This card ends no open session on those events. Ending them belongs to the emergency-stop card (draft DIRECTORY-018, 07794b8c).
- Refuse a session to a suspended agent — Suspension semantics are open for Tom (ADR-011), so this card does not refuse on suspension.
- Check that a report's launch record exists and belongs to the agent — This belongs to the start card, which makes launch records, so that the two cards do not depend on each other. Until then, an invented id yields only a session whose report names a record nobody made.

### The smallest complete shape

One documents-only landing in docs/design, validated by gate.sh, under a single chain card. It contains the new directory brief (JSON plus rendered markdown) at the next free id; the ADR for the session credential in decisions.json; the roadmap row; the brief's checklist items and stories with the cluster's markdown re-rendered; and the three appended lines in design.json (non-goal, P6, CN1). The brief itself carries the whole card's build as ordered rows: the contract section and event entries, an adversarial review of the credential and lys/session-start/v1 before any code, challenge issue and report-back, credential presentation, end and stop with the second-session proof, the routes, the admin-only listing command, and R9's revocation refusal. It names its three landed blockers, plus R9's fold blocker, each with a command a stranger can run. The build waits on the lead's sign-off on the card.

## The roadmap row

- **RM-049** — Make each session its own directory record with its own credential (feature, idea)
- Summary: Road step 2's session half, as DIRECTORY-026. A session becomes its own record in the directory, apart from the enduring agent. It is created when the started agent, whose certificate is issued, first reports back with a proof under the new, proposed tag lys/session-start/v1 (ADR-080), signed by its enrolled key over its directory id, the directory's identifier, a launch record id and a single-use challenge the directory issued it with a 60-second lifetime; the signature is checked before anything else, so an unauthenticated caller learns nothing about which agents exist. The session points at the agent by its directory id and carries a random credential the directory keeps only as a hash and returns once (ADR-079). The credential is refused as soon as the agent reports its end or the directory's administrator stops the session, and the administrator can list an agent's sessions with their states. It is done when a second session of the same agent presents the same enduring identity with a new credential and the first session's credential is refused once that session has ended.
- Asked by: tom on 2026-09-27T14:29:00+10:00
- Context: The session card on the Lys board (road step 2's session half), surveyed against lys main 7b53625; the quote's later parts are the identity line's lead's rulings as the card's words carry them. Four further answers of the lead settle what the survey left open: P9 and C13 give way narrowly so an agent creates and ends its own session record and nothing else; CN6 binds only the brief after row 05, not this one; R10's blocker is the certificate revocation card named by its title and fold; and lys/session-start/v1 is proposed in docs/design/WIRE-FORMATS.md, ratified there, with no production agent signing under it before.
- Quote: Write the brief for this card in the directory cluster (docs/design/directory/briefs), at the next free id past lys main and every open brief and draft branch, checked with git ls-remote immediately before writing. The card makes a session its own record in the directory, separate from the enduring agent it belongs to. A session is created when an agent's session starts, points at its agent by the agent's directory id, and carries its own session credential. Starting a session never creates an agent. This is road step 2's session half, named by the provision card's brief (6AWx1-JR, run 28d15c0d) as the row its second-session proof waits on, and session start sits in Chippy's release 3 at STATEMENT-2026-09-22.md line 156. Done when a second session for the same agent presents the same enduring identity with a new session credential, and the first session's credential is refused once that session has ended. The form of the session credential is the open question the provision brief records, a per-session lys certificate, a SpeaksFor delegation or an ADR-001 handle. This brief decides it, as a technical choice the lead makes and writes down with its reason, and it is distinct from the agent's one certificate (ADR-008). The started agent reports back to the directory (ADR-007), and the directory checks it. The brief builds that presentation step, which the provision brief names as this card's. Hold to DIRECTORY-003 (each agent under its responsible person, every identity change a signed event) and to the typed capability claim card (hhAN8h77) for what the agent's certificate carries. The build is blocked until DIRECTORY-003 and the provision card have landed, each checked by a command a stranger can run. Keep to the method (scripts/design/validate.py, check-coverage.py, render-cluster.py, run by scripts/design/gate.sh). If a sentence of DESIGN.md or CHECKLIST.md is open or contradicted by the repository as it stands, quote it as a question for the lead rather than rewriting the design around it. The card is built from this brief only after the lead signs it off on the card.

Rulings of the lead, Archie, given on 27 September 2026 to the run d5055cc1-73dc-49b5-ad25-449f195b8796 in answer to its rounds. That run took every answer and then failed before writing, when the account pool refused every session. They are settled here, and the author reopens none of them.

None of the three. The session credential is a random value the directory issues when the session is created. The directory keeps only its hash, on the session record, and the session presents the value to the directory. The reason is that in this card the directory is the only party that checks a session credential. Nobody verifies it offline, so a signed artifact buys nothing here and would freeze a format before anything needs it. A handle waits on the unbuilt secrets broker (SECRETS-002). SpeaksFor is framed for an offline root key and a seat subject, and delegation/mod.rs says not to invent a consumer. Refusing is a change to the record's state, so ending a session refuses its credential at once. The brief records this choice and its reason as an ADR at the next free id, and names a signed session credential as later work, for when a party other than the directory must check a session. Answered by Archie, lead for the identity line.

When the started agent first reports back and proves it is the agent. Rendering a start command creates nothing, so a command that is rendered but never run leaves no session. That fits ADR-007, because the product runs nothing and a start command never carries a credential's value. The report-back is authenticated by the agent's enrolled key. The directory then creates the session record pointing at the agent's directory id, issues the session credential and returns its value once, in that answer only. Answered by Archie, lead for the identity line.

Two acts end a session in this card. One is the agent reporting its end with its session credential, and the other is an operator stopping the session on the directory. Each is recorded with its actor, and after either one the credential is refused, naming the ended session. Expiry is not in this card. A crashed agent's session stays open until an operator stops it. The brief says that plainly, and names lapsing a stale session as its own card. Acceptance lines cover a credential refused after the agent's end, a credential refused after an operator's stop, and a second session for the same agent admitted while the first is open. Answered by Archie, lead for the identity line.

Yes to both. A session starts only for an agent whose certificate has been issued, and the report-back is authenticated by the key enrolled for that certificate (ADR-032). An agent whose certificate is not issued is refused a session, naming the act that answers it, which is issuing that agent's certificate. So the build is also blocked on card hhAN8h77 landing, as a third blocker beside DIRECTORY-003 and the provision card. Each is checked by a command a stranger can run, reading that card's own execution record for landed. Answered by Archie, lead for the identity line.

It stands beside them as a step-2 increment, and neither is rewritten. One line is appended beneath the non-goal at design.json:92, and one beneath P6. Each names this card's session record and credential as step-2 work brought forward under the lead's ruling of 27 September 2026. This is the same form I ruled for CN11 in the revocation brief. DESIGN.md does not gain the road, as ruled in run 28d15c0d. Answered by Archie, lead for the identity line.

The proof is server-side, plus a command a stranger can run that lists an agent's sessions with their states. Neither the sessions screen nor the agent's file is in this card. The brief names the sessions screen as its own card, which reads the records this card makes. Answered by Archie, lead for the identity line.

Sign a challenge the directory issues, and the directory's identity. Before reporting back, the agent asks the directory for a challenge. That is a random value the directory issues once, for that agent, with a short expiry, and keeps until it is used or expires. The lys/session-start/v1 statement signs the agent's directory id, the directory's own identifier and that challenge. The directory admits the proof only when the challenge is one it issued to that agent, unused and unexpired, and the directory named is itself. It marks the challenge used in the same act that creates the session. A reused, expired or foreign challenge, or a proof naming another directory, is refused by name and creates nothing. Acceptance lines cover a replay of an accepted proof, a proof after the challenge expired, a proof for another directory, and a proof whose challenge was issued to a different agent, each refused by name. The reason is that the tag freezes once signed, so freshness and audience have to be in it from the first version. Answered by Archie, lead for the identity line.

Yes, 60 seconds, accepted at 60 and refused as challenge_expired at 61, as the brief has it. A separate ruling, from the start card's round 1, also belongs in this brief before it is written. The signed lys/session-start/v1 message carries the launch record id as well, so it signs the tag, the agent id, the launch record id and the nonce. No agent signs under the tag yet, so the change is free now. The start card ties running to the command given through that id. R1, R4 and the acceptance lines carry the new field, and a report whose launch record id does not match the challenge's launch record is refused by name. Answered by Archie, lead for the identity line.

Admin-only in this card, as DIRECTORY-003 R3 and P9 admit only the configured administrator in step 1. 'A command a stranger can run' means the command is written out in full and reproducible from the brief, not that anyone may call it. The agent's responsible person gains the listing when step 1's admission admits them, and the brief names that under further units not written. Everyone else stays refused by name. Answered by Archie.

Rulings of the lead, Archie, given on 27 September 2026 to the run 8c1bee6c-7421-4bf2-9753-922ad3546590 in answer to its rounds. That run took every answer and then failed before writing, when the account pool was at its usage limit. They are settled here, they stand over any earlier ruling above that they contradict, and the author reopens none of them.

Five fields. The signed lys/session-start/v1 message carries the tag, the agent id, the directory's own identifier, the launch record id and the nonce. The later sentence adds the launch record id and removes nothing, and the acceptance line for a proof naming another directory stands. Answered by Archie, lead for the identity line.

Carry it as supplied, and leave the existence check to the start card, so the two cards do not depend on each other. The challenge names the launch record id its requester gives, and this card refuses by name a report whose launch record id does not match the challenge's. The start card, which makes launch records, adds the check that the record exists and belongs to the agent, with its own acceptance line. The brief names that as the start card's and records that until it lands, an invented id yields only a session whose report names a record nobody made. Answered by Archie.

Superseded and closed. The d5055cc1 draft and its DIRECTORY-015 and ADR-046 are closed by this brief. This brief takes its ids by the next-free rule, re-checked immediately before writing. The start card names this brief by its run, d5055cc1, whatever id it lands under, as I told Buckley, so its reference follows without a renumbering by hand. Answered by Archie.

A new session is refused by name to a retired agent and to an agent whose certificate has been revoked. Suspension stays open for Tom, so this card does not refuse on it, and the brief says so. This card ends no open session. The next report-back authenticated by a revoked certificate's key is refused by name. Ending open sessions is the emergency-stop card's, and the brief names it. Acceptance lines cover a retired agent refused, a revoked certificate refused, and a revoked certificate's report-back refused. Answered by Archie.

Confirmed. The certificate revocation card, DIRECTORY-012 on draft/directory/82386e20, is R9's blocker, checked by a command a stranger can run against lys main that finds its fold. R1 to R8 go ahead without it. No other arrangement: a revoked certificate is refused only through that fold, never through a second list kept here. Answered by Archie, lead for the identity line.

CN1 binds only the documents-only planning briefs. Record that by appending a line after CN1, as P6 was appended, and leave CN1's text as it is: from DIRECTORY-003 on, the directory's code briefs create and change code under crates/lys-identity and crates/lys-identity-server, and CN1 does not bind them. Answered by Archie.

In-process only, as the brief has it. DIRECTORY-003 defines no scriptable sign-in, so this brief does not invent one. It names the deployed-directory run under further units not written, attached to the card that brings a scriptable sign-in, and the acceptance runs the command against the in-process directory the test starts. Answered by Archie, lead for the identity line.

This draft keeps DIRECTORY-021 and RM-041, because it wrote them first. The access-graph brief 526867e5 renumbers, and its round 2 ruling already moves it to the next free ids past every head (DIRECTORY-022 and RM-042 today). Nothing moves here. Answered by Archie, lead for the identity line.

Verify first. The report-back verifies the proof's signature under the agent's enrolled key before it answers anything else. Anything unsigned, badly signed, or naming an agent the directory does not hold or one with no enrolled key is refused as proof_invalid with one body, so an unauthenticated caller learns nothing about which agents exist or their state. certificate_not_issued is not excepted. A key is enrolled with the agent, apart from the certificate, so the signature can be checked first, and that refusal goes only to a correctly signed proof. Only a correctly signed proof then gets the state-revealing refusals, in R4's order: agent_retired, certificate_not_issued, wrong_directory, challenge_reused, challenge_unknown, challenge_foreign_agent and challenge_expired. An unsigned or invalid request never changes, consumes or drops a challenge. An acceptance line shows that the unsigned forms of each of those cases all return the identical proof_invalid body and leave the challenge store byte-identical. One more ruling rides with this round, on ids. 7494cb8b wrote DIRECTORY-021 at 16:54, before this brief took it at 16:55, and the first writer keeps an id. This brief moves off DIRECTORY-021 and RM-041 to the next free ids past main and every brief, draft and hand head, re-checked with git ls-remote immediately before writing. That withdraws my earlier ruling that it keeps them, which rested on a wrong reading of the times. Answered by Archie, lead for the identity line.
- Cluster: directory; briefs: DIRECTORY-026
- Notes: Ids measured with git ls-remote of ablative-io/lys and a fetch of main and every brief/, draft/ and hand/ head, first before writing (169 heads) and again once written (171 heads). Before writing, the highest in use were DIRECTORY-024 (draft/directory/2da3cf8c), ADR-077 (draft/directory/fb954264), RM-047 (draft/secrets/7a0935db), C185 and S76 (draft/directory/2da3cf8c), and this draft was written as DIRECTORY-025, ADR-078 and ADR-079, RM-048, C186 to C201 and S77 to S80. The re-measure once written found draft/directory/b093f439 (the SpiceDB permission-check card, pushed first) holding DIRECTORY-025, ADR-078, RM-048, C186 to C192 and S77 to S79; the first writer keeps an id, so this brief is DIRECTORY-026, its decisions ADR-079 and ADR-080, this row RM-049, and its items C193 to C208 and S80 to S83. This card's previous draft (draft/directory/8c1bee6c) held DIRECTORY-021, RM-041, ADR-075, C131 to C143 and S55 to S58; under the lead's ruling it moves off them, the first writer (draft/directory/7494cb8b) keeping DIRECTORY-021. RM-029, which the provision brief names for this card ('A session is its own record, issued at session start and pointing at its agent'), exists only on the provision brief's branches and is held on draft/home/4433334f and draft/lys-core/6747ce61 for other work, so this row carries the same work at the next free id; RM-029 and RM-049 are one card. Every row main holds is unchanged. The provision card, the typed capability claim card and, for R10 only, the certificate revocation card's fold are not on main, so they are named as blockers in the brief rather than as roadmap dependencies. CN6's hold point binds only the brief that follows row 05 in IDENTITY-001's order and is not a blocker here. This brief supersedes and closes the earlier draft of the card (draft/directory/d5055cc1, its DIRECTORY-015 and ADR-046); the start card names this brief by that run, whatever id it lands under. design.json gains lines appended beneath the 'Road step 2 onward' non-goal, P6 and P9, and after CN1, and C13 gains the same exception as P9's line; no text is rewritten. lys/session-start/v1 is proposed in docs/design/WIRE-FORMATS.md by R3 and ratified there by the register's ratifier, not by this card. Further units, not written: Issue a signed session credential for parties other than the directory; Lapse a stale session whose agent never reported its end; Accept an agent's report that its session is running; The sessions screen, reading the records this card makes; Let an agent's responsible person list its sessions; Run the session listing against a deployed directory, with the card that brings a scriptable sign-in; End an agent's open sessions on emergency stop, revocation or retirement; Refuse a session to a suspended agent; Check that a report's launch record exists and belongs to the agent (the start card's).

## The design

---
type: design
cluster: directory
title: The standalone identity directory, with every grant rooted in a person
---

# The standalone identity directory, with every grant rooted in a person

> **Cluster:** directory

## Intention

An operator installs the identity product without Cambium or Manifold, signs in, links Google and GitHub to one person, registers an agent under a responsible person, and inspects the signed history of every identity change. Cambium later uses this issuer, keeping its participant ids (rows 06 and 07); that direction is recorded here while rows 06 and 07 stay the non-goal recorded below, and this sentence promises neither row.

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
Brought forward as step-2 work under the lead's ruling of 27 September 2026: DIRECTORY-026's session record and session credential.
- **P7** — An audit receipt carries a version, a stable operation ID, the actor, the affected identity, the operation, a payload commitment and the resulting log coordinate or checkpoint; secrets and whole context objects are excluded, and its exact signed encoding is reviewed before use.
- **P8** — The service attests the authenticated human actor and their authentication provenance; it never claims a person signed bytes with a key they do not hold, and a registration records the person who made it, never an invented agent signature.
- **P9** — The initial directory administrator is bootstrapped by an explicitly configured issuer and subject, never by email or first visitor; every other mutation caller is refused in step 1.
Under the lead's ruling of 27 September 2026, P9 gives way narrowly for DIRECTORY-026: an agent authenticated by its enrolled key may create and end its own session record, and nothing else. Every other mutation by a caller who is not an administrator, including an agent acting on another agent's session, is still refused in step 1 by name.

## Decisions

- ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
- ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
- ADR-005 — The identity database is PostgreSQL, possibly on a network device — PostgreSQL is used for the identity product's database. It may be set up on one of the network devices rather than on Tom's Mac.
- ADR-007 — The product starts an agent by giving its start command, never by running it — The product is not an execution engine. For the first release an agent's file gives the command that starts it on a chosen machine: the command carries the agent's identity and its handles, never a credential's value, and it is rendered from the agent's kept launch record. A started agent reports back, so the sessions screen shows what is running. A terminal inside the product, sandboxes, virtual machines and containers are later runtimes that plug in, and none is built into this product.
- ADR-008 — An agent's file shows its lys certificate — An agent's file shows its lys certificate once one is issued: what it claims, who signed it, when it was issued and when it expires, with the signed receipts of the changes made to it. An agent registered before any key or proof of possession was supplied shows its certificate as not issued, never a placeholder.
- ADR-009 — People sign in through a maintained Rauthy fork of our own — Rauthy authenticates people, and its one-provider-per-user limit is changed in a fork we maintain, ablative-io/rauthy, not contributed upstream as a prerequisite. The maintained branch is ablative, created from upstream v0.36.2 commit dd61ac3c84d6b238108dc8438b53043b5177a662; the fork's main stays an untouched upstream mirror; lys pins an exact commit of ablative as the submodule vendor/rauthy. Upgrades rebase ablative onto upstream release tags only, each in its own gated row; no cherry-picks and no reset of main.
- ADR-010 — Every product shares one design and keeps its own accent; the identity product's is orange — The identity screens follow Aion's structure, typography, spacing and interaction, and Rauthy's client themes take the same colours, with no build dependency on Cambium or Aion. Each product keeps its own accent within the estate colour family: Cambium green, Aion blue and black, Argus light blue, Haematite mustard. The identity product's accent is orange (accent #D4975A, deep #A86B2E, wash #3D2A17 in the estate colour tokens), set apart from Manifold's copper. No product is silently made Aion-blue, and purple is not used.
- ADR-011 — An identity is registered, active, suspended or retired — An identity is in one of four states: registered (exists in the directory, no grants, no credential handle, may not act), active (may act within its grants), suspended (kept whole, grants kept but not effective) and retired (permanent, history kept, never reactivated; a new identity is made instead). Register, activate, suspend, reinstate and retire are the only transitions, each one signed audit record naming the authenticated actor and their provenance, the identity, from, to, when and reason. Having a grant or a credential is a fact beside the state, not a state. A person is registered by first sign-in; an agent is registered by a signed-in person, who carries it as its responsible person for life and may cause every transition of their own agents. Source: docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:17-44 and docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:71-95.
- ADR-079 — A session credential is a random value the directory issues when the agent's signed report-back over a directory-issued challenge creates the session, and keeps only as a hash — The session credential is 32 random bytes the directory issues when the session is created. The directory keeps only their SHA-256, on the session record, and returns the value once, in the answer to the report-back that created the session; the session presents the value to the directory, which is the only party that checks it. A session is created when the started agent first reports back and proves it is the agent with its enrolled key, and only for an agent whose certificate has been issued. Before reporting back, the agent asks the directory for a challenge: 32 random bytes the directory issues once, for that agent and a launch record id the agent supplies, kept until used or until it expires more than 60 seconds after issue. The proof is an Ed25519 signature under the tag lys/session-start/v1 (ADR-080). The directory verifies the signature before anything else: an unsigned or badly signed proof, or one naming an agent it does not hold or one with no enrolled key, is refused proof_invalid with one body and touches no challenge, and only a correctly signed proof receives the state-revealing refusals. It admits the proof only when the challenge is one it issued to that agent, unused and unexpired, the directory named is itself and the launch record id matches the challenge's, and it marks the challenge used in the same act that creates the session. Two acts end a session: the agent reporting its end with its credential, and the directory's administrator stopping it. Ending changes the record's state, so the credential is refused at once, naming the ended session. Rejected: a per-session lys certificate, because nobody verifies a session credential offline in this card, so a signed artifact buys nothing and would freeze a format before anything needs it; a lys/delegation/v1 SpeaksFor delegation, framed for an offline root key and a seat subject, whose module says not to invent a consumer; an ADR-001 handle, which waits on the unbuilt secrets broker (SECRETS-002); creating the session when a start command is rendered; and expiry of a session.
- ADR-080 — lys/session-start/v1 signs five fields, the tag, the agent id, the directory's identifier, the launch record id and a directory-issued nonce, and is proposed in the wire-format register until ratified — The report-back is an Ed25519 signature by the agent's enrolled key over the 20 ASCII bytes of lys/session-start/v1, one 0x00 byte, then the agent directory id, the directory's own identifier and the launch record id, each as its UTF-8 length in 4 bytes big-endian followed by its bytes, then the 32-byte nonce, a challenge the directory issued once to that agent for that launch record id and admits for 60 seconds after issue. Freshness (the nonce), audience (the directory's identifier) and the launch record id are in the first version because the tag cannot gain them after it freezes. The format is proposed in docs/design/WIRE-FORMATS.md's decision log; the tests sign under it with disposable test keys only, and no production agent signs under it until the register shows it ratified. Rejected: a four-field message without the directory's identifier, which could not say which directory a proof was made for; a message without the launch record id, which could not tie a session to the command it was started by; a signature over the fields without a directory-issued nonce, which a replay would satisfy; and treating this ledger entry as the ratification, which the register reserves to its own decision log.

## Goals

- Every open row of IDENTITY-001 (02, 04, 03, 05) exists as a valid design-system brief in this cluster, DIRECTORY-002 to DIRECTORY-005, in its dependency order, every ID001 acceptance identifier kept.
- The grant path (create an agent under a person, grant it a project, the action is allowed, revoke or suspend, the same action is refused) is a requirement of the row that owns it, with acceptance criteria; where the sources do not settle the row, it is recorded open for Tom in DIRECTORY-003 with its criteria drafted.
- Every decision still open for Tom is recorded as open and decided nowhere in this cluster.
- The two live demonstrations to Tom, ID001_LINK_LIVE and ID001_DIRECTORY_LIVE, stay hold points a loop completion never replaces (CN6).
- DIRECTORY-006 makes the grant/refusal journey enforceable and binds its acceptance to the reviewed mock-up, without rewriting the historical IDENTITY-001 record.

## Non-Goals

- Rows 06 (connect Cambium) and 07 (gate, install and demonstrate the release) — They change the Cambium repository and depend on IDENTITY-002, the upstream release rebase; they need a Cambium cluster or an agreed cross-repository arrangement first.
- Row 03's changes inside the Rauthy fork — The fork is its own repository under vendor/rauthy; a row whose files live there needs its own brief in the fork, which does not yet exist and blocks DIRECTORY-004. DIRECTORY-004 names the fork files as work with their owner, not as files of this repository.
- The grant representation: how one grant records who may exercise it, whether it may be passed on (person, agent or nobody), what it derives from, and whether it can be bounded. OPEN for Tom. — The statement leaves the exact delegation schema unsettled (docs/design/identity/STATEMENT-2026-09-22.md:21; ADR-003); AGENT-PARITY-2026-09-23's questions (docs/design/identity/AGENT-PARITY-2026-09-23.md:17-23) are inputs to it, not answers.
- Suspension semantics: whether suspending a person also ends their sign-in session at Rauthy or only makes our checks refuse, and what else stops with a suspended identity. OPEN for Tom. — The lifecycle document asks it of the room (docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:99-100, docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:104-105) and its states are only proposed (ADR-011); two systems, one decision.
- The name of the identity service. OPEN for Tom. — The statement lists it as not decided (docs/design/identity/STATEMENT-2026-09-22.md:192); no crate or directory name selects it.
- Which anchor the first agents pin to: our hosted anchor, a self-hosted one, or both. OPEN for Tom. — Lys leaves it as a product decision and so does the statement (docs/design/identity/STATEMENT-2026-09-22.md:193).
- A nightly Rauthy base versus waiting for an upstream release carrying #1696 and #1728. OPEN for Tom. — Waffles' ruling of 15:36:25: if upstream has published no such release by the row 05 showing, Tom decides (docs/design/identity/briefs/IDENTITY-001.json:18; docs/design/identity/STATEMENT-2026-09-22.md:177); rows 02 to 05 run on v0.36.2 meanwhile.
- Road step 2 onward: capability certificates, arbitrary grants and their enforcement, session launch and stop, credential handles, memory, context assembly, lanterns and anchoring in production
Brought forward as step-2 work under the lead's ruling of 27 September 2026: DIRECTORY-026's session record and session credential. — Revision 5 keeps them out of step 1 (docs/design/identity/briefs/IDENTITY-001.json:29-30); ADR-007 (the start command) and ADR-008 (the certificate on an agent's file) govern what the step-1 screens do not present as working.
- The examples in AGENT-PARITY-2026-09-23 (abilities with an assignment or project, seat provisioning within a budget, private and shared notes) — Tom gave them as not yet decided (docs/design/identity/AGENT-PARITY-2026-09-23.md:11-15); they are never turned into requirements.
- A production Cambium auth cutover, and any upstream Rauthy contribution as a prerequisite — Revision 5 forbids both before scratch acceptance, review and Gypsy's coordinated install (docs/design/identity/briefs/IDENTITY-001.json:31).
- A shared design-system package extracted for every product — Tom left it as a thing to look at, not a row (ADR-010).

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
| `docs/design/directory/briefs/DIRECTORY-008.json` | the grant brief residue after PR 6: DIRECTORY-005's verification line, three inventory rows and the intention sentence, as requirements on the documents | DIRECTORY-008 |
| `docs/design/directory/briefs/DIRECTORY-008.md` | rendered markdown | DIRECTORY-008 |
| `docs/design/WIRE-FORMATS.md` | the wire-format register; gains the proposed, unratified decision-log row for lys/session-start/v1 (R3) | DIRECTORY-026 |
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
| `crates/lys-identity/Cargo.toml` | the session module's dependencies (DIRECTORY-026 R5) | DIRECTORY-026 |
| `crates/lys-identity-server/src/lib.rs` | declares the session routes module (DIRECTORY-026 R8) | DIRECTORY-026 |
| `crates/lys-identity-server/src/config.rs` | reads, validates and documents directory_id beside the service's other configured values (DIRECTORY-026 R8) | DIRECTORY-026 |
| `tests/identity_contract/src/harness.rs` | starts the in-process service with a configured directory_id for the session route tests (DIRECTORY-026 R8) | DIRECTORY-026 |

## Inventory

- `docs/design/identity/briefs/IDENTITY-001.json` — IDENTITY-001 revision 5, the reviewed plan these briefs carry forward: rows, walls, acceptance identifiers (ID001_*), estimates, authority and review decisions. Read, never changed.
- `docs/design/identity/briefs/IDENTITY-001.md` — its rendered twin
- `docs/design/identity/STATEMENT-2026-09-22.md` — the authority, including 'Everything is pegged to a human authority (Tom, 17:15 to 17:16)'
- `docs/design/identity/LIFECYCLE-STATES-2026-09-22.md` — the working lifecycle states: registered, active, suspended, retired
- `docs/design/identity/PROVISIONING-2026-09-22.md` — provisioning an agent under a human, the seven-step path
- `docs/design/identity/AGENT-PARITY-2026-09-23.md` — Tom's 23 September ruling: 'If a human can do it through the UI, then I want an agent to be able to do it'; whether an agent may is a permission question. Input to the directory and permission rows. Its examples are recorded as undecided and are never turned into requirements.
- `docs/design/identity/RAUTHY-BASELINE.md` — the fork baseline: pin, maintenance owner, the IDENTITY-001-UPSTREAM-AUTH-STATE blocker; read, never changed
- `docs/design/identity/CONFORMANCE.md` — Accepted mock-up behaviour map; DIRECTORY-006 maps You/delegation/access requirements and does not claim mock-up sample policy is authoritative.
- `vendor/rauthy` — the maintained Rauthy fork (ADR-009), the git submodule pinned at dd61ac3c84d6b238108dc8438b53043b5177a662, the upstream v0.36.2 commit the ablative branch was created from; DIRECTORY-004 moves the pin (structure row); read here, never changed by a document row
- `crates/lys` — the lys CLI crate: Cargo.toml, src/main.rs, src/cli.rs and src/commands/ (attest, ca, key, log, inspect, files); DIRECTORY-002 adds src/identity/ and the identity subcommand to it (structure rows)
- `docs/design/decisions.json` — the project decision ledger, ADR-001 to ADR-018 at main, holding the decisions this cluster cites (ADR-003, ADR-004, ADR-005, ADR-007 to ADR-011); DIRECTORY-001 recorded that it gained the identity decisions (structure row); read here, never changed by a document row

## Constraints

- **CN1** — Documents only: nothing outside docs/design/directory/ and docs/design/decisions.json is created or modified; the IDENTITY-001 files are not changed.
Under the lead's ruling of 27 September 2026, CN1 binds only the documents-only planning briefs: from DIRECTORY-003 on, the directory's code briefs create and change code under crates/lys-identity and crates/lys-identity-server, and CN1 does not bind them.
- **CN2** — Development isolation: rows 02 to 05 use only disposable test identities and test provider registrations; no production tokens, real business sign-in or live Cambium participant migration.
- **CN3** — Every path written in a document of this cluster is relative to the repository root, whatever directory a session starts in; a command runs from its own tree and spells its paths from there.
- **CN4** — No structure row or files entry carries a root token; a file in another repository is named in a requirement's spec with its owner.
- **CN5** — A live demonstration to Tom is never an acceptance criterion of a loop requirement; it is a verification step a person performs after the row lands.
- **CN6** — The live demonstrations ID001_LINK_LIVE (after row 03) and ID001_DIRECTORY_LIVE (after row 05) are mandatory operator hold points: the brief that follows each is blocked by it until Tom's demonstration receipt is recorded; a loop completion never stands in for one.
- **CN7** — Revision 5's ceiling stands: 48 focused implementer hours for IDENTITY-001 (row 01 1.5, closed; 02 8; 04 10; 03 10; 05 6; 06 4; 07 5; total 44.5, contingency 3.5), with IDENTITY-002's 4 hours outside it. An overrun is reported as soon as it is known, and Waffles takes any ceiling change to Tom (docs/design/identity/briefs/IDENTITY-001.json:24).
- **CN8** — One implementer, one row in implementation and one gate invocation at a time in this lane; release builds, checks and tests run through the gate workflow at the venue, and a development exception never bypasses it (docs/design/identity/briefs/IDENTITY-001.json:25-27, docs/design/identity/briefs/IDENTITY-001.json:135).
- **CN9** — A row that needs a file outside its wall stops and names it, and the reviewer approves a brief revision before that file is edited; a directory wall for a wholly new module allows only its named responsibility and needs an exact file manifest reviewed before its row starts (docs/design/identity/briefs/IDENTITY-001.json:28).
- **CN10** — Rows 02 to 05 run on Rauthy v0.36.2 under Waffles' ruling of 15:36:25; each development install checks current releases and advisories and records the accepted exception; IDENTITY-001-UPSTREAM-AUTH-STATE binds real sign-in and install, rows 06 and 07 (docs/design/identity/briefs/IDENTITY-001.json:18, docs/design/identity/briefs/IDENTITY-001.json:46).
- **CN11** — Step 1 is directory records, sign-in and the minimum signed identity audit. SpiceDB is installed and checked in row 02 and enforces nothing in step 1; live capability policy and its enforcement are step 2's, and running SpiceDB is not permission enforcement (docs/design/identity/briefs/IDENTITY-001.json:29-30).
- **CN12** — DIRECTORY-006 is implementation work after the frozen planning task: its source paths are only executable after the DIRECTORY-002/003 foundations are implemented and their integration manifests are reconciled. R6 also waits for the standalone surface foundation. Do not dispatch from a schema-valid but dependency-blocked brief.


---
type: brief
id: DIRECTORY-001
cluster: directory
title: Revise the identity directory plan for human-rooted grants, as design-system briefs
---

# DIRECTORY-001: Revise the identity directory plan for human-rooted grants, as design-system briefs

> **Cluster:** directory
> **Design anchor:**
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> - ADR-005 — The identity database is PostgreSQL, possibly on a network device — PostgreSQL is used for the identity product's database. It may be set up on one of the network devices rather than on Tom's Mac.
> **Checklist:**
> - C1 — The directory design records the outcome, the shared contract as principles, the constraints and the non-goals of IDENTITY-001, revised for the grant ruling.
> - C2 — The project decision ledger holds the identity decisions with their authority and quote: the maintained Rauthy fork, one PostgreSQL service and database (ADR-005, already recorded), the product accents, and the lifecycle states as working team decisions marked proposed.
> - C3 — Each open row (02, 04, 03, 05) is a design-system brief, DIRECTORY-002 to DIRECTORY-005, in dependency order, with its wall as files, its ID001 acceptance identifiers kept, and its estimate in its task.
> - C4 — The grant path (create an agent under a person, grant it a project, the action is allowed, revoke or suspend, the same action is refused) is a requirement with acceptance criteria in the row that owns it, and row 02 states what SpiceDB enforces in step 1 and what it does not.
> - C5 — Every decision still open for Tom is recorded as open: the grant representation, suspension semantics, the service name, the anchor, and nightly versus waiting for the upstream release.
> - C6 — The rendered markdown of this cluster matches its JSON and coverage is clean.
> **Stories:**
> - S1 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want every grant my agent holds to trace back to me, so that withdrawing my authority stops everything derived from it.
> - S2 (Reviewer, Reviews a brief before any of its rows is dispatched) — As the reviewer, I want each open identity row as a design-system brief with numbered requirements and criteria, so that rows can be dispatched to the loop one at a time and reviewed against their criteria.

## Purpose

Rewrite the identity plan so every grant traces back to a person (Tom, 22 September 17:15), with the PostgreSQL decision and the four lifecycle states in, as design-system briefs the loop can build. Documents only; comes before any code.

## Task

Read IDENTITY-001.json revision 5 in full, the statement's section 'Everything is pegged to a human authority', LIFECYCLE-STATES, PROVISIONING, AGENT-PARITY-2026-09-23 (its examples are undecided and are not requirements), and ADR-005 in docs/design/decisions.json, which already records Tom's PostgreSQL ruling. Add the identity decisions to docs/design/decisions.json, complete this cluster's design, checklist and stories, and write DIRECTORY-002 (row 02, 8 hours), DIRECTORY-003 (row 04, 10 hours), DIRECTORY-004 (row 03, 10 hours) and DIRECTORY-005 (row 05, 6 hours), each depending on the one before, revision 5's 48-hour ceiling kept and any re-estimate stated. Carry every ID001 acceptance identifier forward unchanged. Every path in a document of this cluster is relative to the repository root, even though the session starts in docs/.

## Requirements

### R1: Record the identity decisions and complete the directory design

THE SYSTEM SHALL add to docs/design/decisions.json one ADR per identity decision named in C2 that the ledger does not already hold (ADR-005, PostgreSQL, is already there and is cited, not rewritten), each with its authority, date, decider and, where the source holds Tom's own words, the quote; a working team decision SHALL be status proposed, never decided. The directory design SHALL gain those ADR ids and SHALL keep revision 5's outcome, shared contract, constraints and non-goals, revised for the grant ruling. Each ADR's source SHALL be cited from IDENTITY-001's authority or review_decisions, the statement, or the lifecycle document; nothing is invented.

**Acceptance:**
- validate.py reports decisions.json and design.json valid.
- Every ADR added cites its source, and the lifecycle states ADR is status proposed.
- ADR-005 is unchanged and the directory design anchors to it.

**Files:**
- modify: docs/design/decisions.json
- modify: docs/design/directory/design.json
- modify: docs/design/directory/DESIGN.md

**Checklist:**
- C1 — The directory design records the outcome, the shared contract as principles, the constraints and the non-goals of IDENTITY-001, revised for the grant ruling.
- C2 — The project decision ledger holds the identity decisions with their authority and quote: the maintained Rauthy fork, one PostgreSQL service and database (ADR-005, already recorded), the product accents, and the lifecycle states as working team decisions marked proposed.

**Stories:**
- S1 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want every grant my agent holds to trace back to me, so that withdrawing my authority stops everything derived from it.

#### R1 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Row 1 (validate.py reports decisions.json and design.json valid): both files were written to the decisions and design schemas. I checked keys and types against docs/design/decisions.json:146-199 and docs/design/directory/design.json with my own code, not validate.py, which the workflow runs. Row 2 (every added ADR cites its source; the lifecycle ADR is proposed): ADR-009 at docs/design/decisions.json:146 cites the statement at :191 and IDENTITY-001 at :10, :17 and :35. ADR-010 at :164 cites the statement at :194 and IDENTITY-001 at :14-15 and :43-44. ADR-011 at :182 cites LIFECYCLE-STATES at :17-44 and :71-95 and has "status": "proposed" (:184). Row 3 (ADR-005 unchanged, design anchors to it): a comparison against HEAD shows the ADR-001 to ADR-008 prefix is identical. ADR-005 stays in docs/design/directory/design.json 'decisions' (:45). Revision 5's content is kept, revised for the grant ruling: the outcome is the intention, the shared contract is P1 to P9 (:7-44), the constraints are CN1 to CN11 (:444) and the non-goals start at :61.
- Deviation: (none)
- Files changed:
  - modified: `docs/design/decisions.json` — 'updated' is now 2026-09-24. Three ADRs added: ADR-009 (the maintained Rauthy fork, decided, Tom with Waffles' branch rule, with Tom's quote), ADR-010 (shared design with each product keeping its own accent, identity orange, decided, Tom's 15:18 quote) and ADR-011 (registered/active/suspended/retired, status proposed, Archie for the working team). Each cites its sources; ADR-001 to ADR-008 are byte-identical.
  - modified: `docs/design/directory/design.json` — Adds principles P6 to P9 from revision 5's shared contract and review decisions, with P3 revised. Decisions now ADR-003, 004, 005, 007, 008, 009, 010, 011. Goals, non-goals (five marked OPEN for Tom), constraints CN7 to CN11 from revision 5's ceiling, a structure row for every listed path, and the RAUTHY-BASELINE inventory entry.
  - modified: `docs/design/directory/DESIGN.md` — Markdown rendered from design.json with the method's render_design.
- Checklist delivery:
  - [x] C1 — The directory design records the outcome, the shared contract as principles, the constraints and the non-goals of IDENTITY-001, revised for the grant ruling. — Outcome, principles P1 to P9, CN7 to CN11 and the revised non-goals are in docs/design/directory/design.json.
  - [x] C2 — The project decision ledger holds the identity decisions with their authority and quote: the maintained Rauthy fork, one PostgreSQL service and database (ADR-005, already recorded), the product accents, and the lifecycle states as working team decisions marked proposed. — Fork (ADR-009), PostgreSQL (ADR-005, cited), accents (ADR-010) and lifecycle states (ADR-011, proposed).
- Story delivery:
  - [x] S1 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want every grant my agent holds to trace back to me, so that withdrawing my authority stops everything derived from it. — P2 and ADR-003 anchor the design. Every agent is registered under its responsible person (DIRECTORY-003 R1).

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] validate.py reports decisions.json and design.json valid. — From docs/: validate.py docs/design/decisions.json -> 'design/decisions.json: OK [decisions.schema.json] All 1 document(s) valid.' (exit 0); validate.py docs/design/directory -> 'design/directory/design.json: OK [design.schema.json]', all 8 valid (exit 0).
  - [x] Every ADR added cites its source, and the lifecycle states ADR is status proposed. — docs/design/decisions.json ADR-009 context cites STATEMENT-2026-09-22.md:191 and IDENTITY-001.json:10, :17, :35, all verified; the quote matches STATEMENT:191 verbatim. ADR-010 cites STATEMENT:194 and IDENTITY-001.json:14-15, :43-44; its hex values match IDENTITY-001.json:44; its quote matches STATEMENT:194 verbatim. ADR-011 cites LIFECYCLE-STATES:1-8, :17-44, :71-95, :101-103, with status 'proposed' and decided_by 'Archie ... not ruled by Tom'.
  - [x] ADR-005 is unchanged and the directory design anchors to it. — A Python comparison of HEAD:docs/design/decisions.json decisions[] against the working tree's first 8 entries -> 'prefix same True'. design.json decisions list contains ADR-005 (DESIGN.md Decisions section).
- Checklist verified: C1, C2
- Stories verified: S1

### R2: Carry rows 02, 04, 03 and 05 into design-system briefs

THE SYSTEM SHALL write DIRECTORY-002 to DIRECTORY-005 from revision 5's rows 02, 04, 03 and 05, each valid against the brief schema: its work as requirements, its wall as files (a file in the Rauthy fork, which is its own repository under vendor/rauthy, is named in the spec with its owner and is not listed in files), its acceptance as criteria keeping every ID001 identifier, its dependencies as depends_on, its estimate in its task. A live demonstration to Tom SHALL be a verification step, never an acceptance criterion. Every path a row brief lists SHALL gain a structure row in design.json. DIRECTORY-004 (row 03) SHALL carry in blocked_by the fork-owned brief that does not yet exist, named as such, so it is never an executable brief with an empty wall. DIRECTORY-005 SHALL carry in blocked_by the ID001_LINK_LIVE demonstration to Tom, and the design's hold point for ID001_DIRECTORY_LIVE SHALL be kept, per CN6.

**Acceptance:**
- DIRECTORY-002 to DIRECTORY-005 exist and validate.py reports each valid.
- Each ID001 acceptance identifier of rows 02, 04, 03 and 05 appears in its brief.
- depends_on runs DIRECTORY-002, then 003, then 004, then 005, matching revision 5's order 02, 04, 03, 05.
- No acceptance criterion requires a live demonstration; each row's demonstration appears under verification.
- check-coverage.py reports every listed path present in the structure.
- DIRECTORY-004's blocked_by names the missing fork-owned brief.
- DIRECTORY-005's blocked_by names the ID001_LINK_LIVE demonstration.

**Files:**
- create: docs/design/directory/briefs/DIRECTORY-002.json
- create: docs/design/directory/briefs/DIRECTORY-002.md
- create: docs/design/directory/briefs/DIRECTORY-003.json
- create: docs/design/directory/briefs/DIRECTORY-003.md
- create: docs/design/directory/briefs/DIRECTORY-004.json
- create: docs/design/directory/briefs/DIRECTORY-004.md
- create: docs/design/directory/briefs/DIRECTORY-005.json
- create: docs/design/directory/briefs/DIRECTORY-005.md
- modify: docs/design/directory/design.json
- modify: docs/design/directory/DESIGN.md

**Checklist:**
- C3 — Each open row (02, 04, 03, 05) is a design-system brief, DIRECTORY-002 to DIRECTORY-005, in dependency order, with its wall as files, its ID001 acceptance identifiers kept, and its estimate in its task.

**Stories:**
- S2 (Reviewer, Reviews a brief before any of its rows is dispatched) — As the reviewer, I want each open identity row as a design-system brief with numbered requirements and criteria, so that rows can be dispatched to the loop one at a time and reviewed against their criteria.

#### R2 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Row 1 (the four briefs exist and are valid): all four follow brief.schema.json's required keys, and my own key check passed. Row 2 (every ID001 identifier kept): a script compared revision 5's acceptance for each row with the briefs and found nothing missing. Row 02 has DEPLOY, DEPLOY_REFUSAL, THEME, SHARED_DB and PIN_CLONE. Row 04 has DIRECTORY, AUDIT_FAULTS, ADMIN, RECEIPT and RECEIVER. Row 03 has LINK_PAIR, LINK_REFUSAL, LINK_MIGRATION, LINK_AUDIT and LINK_LIVE. Row 05 has STANDALONE, SCREEN_REFUSAL and DIRECTORY_LIVE. Row 3 (dependency order): depends_on at line 5 of each brief is DIRECTORY-001 for 002, then 002, 003 and 004 in turn. Row 4 (no live demonstration in acceptance): no acceptance string names a demonstration. ID001_LINK_LIVE and ID001_DIRECTORY_LIVE are the last verification entries of DIRECTORY-004 and DIRECTORY-005. Row 5 (every listed path in the structure): I checked each path, directory prefixes included, against docs/design/directory/design.json:107 onward, with none missing. Row 6: DIRECTORY-004.json:8 blocked_by names 'The fork-owned brief for row 03's Rauthy changes, which does not yet exist'. Row 7: DIRECTORY-005.json:8 blocked_by starts with 'ID001_LINK_LIVE: Tom's live demonstration of row 03'. The ID001_DIRECTORY_LIVE hold point is kept in CN6 (design.json:466), a design goal, and DIRECTORY-005's verification. Estimates are 8, 10, 10 and 6 hours with no re-estimate, stated in each task, and CN7 keeps the 48-hour ceiling.
- Deviation: Four judgement calls. (1) DIRECTORY-002 depends_on DIRECTORY-001, because row 01 is IDENTITY-001's and is already accepted; that acceptance is stated in the task. (2) Revision 5's report names under docs/design/identity/reports/IDENTITY-001-*.md are kept as each row's wall, so they carry forward unchanged. (3) DIRECTORY-003 and DIRECTORY-005 carry in blocked_by the exact file manifest that revision 5 (IDENTITY-001.json:28) requires before a wholly new module's row starts. (4) Revision 5's 'edit main in the main Mac checkout, no worktree' venue rule is not carried as a constraint, because this loop now runs in clones. CN8 keeps the part that still binds: one implementer, one row and one gate at a time, with the gate workflow never bypassed.
- Files changed:
  - created: `docs/design/directory/briefs/DIRECTORY-002.json` — Row 02. R1: packaging and the development install on one PostgreSQL database. R2: the lys identity CLI per the module manifest. R3: the two themes. R4: SpiceDB's step-1 role. 8 hours.
  - created: `docs/design/directory/briefs/DIRECTORY-002.md` — Rendered markdown.
  - created: `docs/design/directory/briefs/DIRECTORY-003.json` — Row 04. R1: registration under a responsible person. R2: signed events and receipts. R3: the administrator. R4: the link-audit receiver. R5: the lifecycle state, with the grant path's row recorded open. 10 hours. blocked_by: the new-module manifest.
  - created: `docs/design/directory/briefs/DIRECTORY-003.md` — Rendered markdown.
  - created: `docs/design/directory/briefs/DIRECTORY-004.json` — Row 03, the lys side. R1: the pin, with fork files named in the spec alongside their owner. R2: the provider-link contract and the report. blocked_by: the fork-owned brief, which does not exist yet. ID001_LINK_LIVE is under verification. 10 hours.
  - created: `docs/design/directory/briefs/DIRECTORY-004.md` — Rendered markdown.
  - created: `docs/design/directory/briefs/DIRECTORY-005.json` — Row 05: the screen journey, visible refusals and appearance. blocked_by: ID001_LINK_LIVE. ID001_DIRECTORY_LIVE is under verification as the hold point. 6 hours.
  - created: `docs/design/directory/briefs/DIRECTORY-005.md` — Rendered markdown.
  - modified: `docs/design/directory/design.json` — A structure row for every path the four briefs list (61 rows, no duplicates).
  - modified: `docs/design/directory/DESIGN.md` — Rendered.
- Checklist delivery:
  - [x] C3 — Each open row (02, 04, 03, 05) is a design-system brief, DIRECTORY-002 to DIRECTORY-005, in dependency order, with its wall as files, its ID001 acceptance identifiers kept, and its estimate in its task. — DIRECTORY-002 to 005 in order 02, 04, 03, 05, with walls as files, identifiers kept and estimates in the task.
- Story delivery:
  - [x] S2 (Reviewer, Reviews a brief before any of its rows is dispatched) — As the reviewer, I want each open identity row as a design-system brief with numbered requirements and criteria, so that rows can be dispatched to the loop one at a time and reviewed against their criteria. — Each row is a brief with numbered requirements and criteria, blocked until Waffles reviews it.

**Review (recorded):**

- Alignment: fixed
- Acceptance verdicts:
  - [x] DIRECTORY-002 to DIRECTORY-005 exist and validate.py reports each valid. — validate.py docs/design/directory: DIRECTORY-002.json to DIRECTORY-005.json each 'OK [brief.schema.json]', exit 0 (re-run after the harden edit).
  - [x] Each ID001 acceptance identifier of rows 02, 04, 03 and 05 appears in its brief. — Running grep -o 'ID001_[A-Z_]*' on each brief gives 002: DEPLOY, DEPLOY_REFUSAL, PIN_CLONE, SHARED_DB, THEME; 003: ADMIN, AUDIT_FAULTS, DIRECTORY, RECEIPT, RECEIVER; 004: LINK_PAIR, LINK_REFUSAL, LINK_MIGRATION, LINK_AUDIT, LINK_LIVE; 005: STANDALONE, SCREEN_REFUSAL, DIRECTORY_LIVE. These equal IDENTITY-001 rows 02/04/03/05 acceptance, and the acceptance texts are verbatim. CAMBIUM_LINK, GOOGLE_SEPARATE and PKCE belong to rows 06 and 07, which are out of scope.
  - [x] depends_on runs DIRECTORY-002, then 003, then 004, then 005, matching revision 5's order 02, 04, 03, 05. — check-coverage.py 'Brief dependencies': 002 depends on 001, 003 on 002, 004 on 003, 005 on 004. 003 is row 04, 004 is row 03 and 005 is row 05, matching revision 5's order 02, 04, 03, 05.
  - [x] No acceptance criterion requires a live demonstration; each row's demonstration appears under verification. — The ID001_LINK_LIVE text appears only in DIRECTORY-004 verification[3] and DIRECTORY-005 blocked_by[0]. ID001_DIRECTORY_LIVE appears only in DIRECTORY-005 verification[3]. No acceptance array names a demonstration.
  - [x] check-coverage.py reports every listed path present in the structure. — check-coverage.py compares every R# files path with the design.json structure (lines 322-336 of the script) and reports 'Coverage clean: all items covered, briefs consistent.', exit 0.
  - [x] DIRECTORY-004's blocked_by names the missing fork-owned brief. — DIRECTORY-004.json blocked_by[0]: 'The fork-owned brief for row 03's Rauthy changes, which does not yet exist: a brief in the ablative-io/rauthy fork's own repository ...'
  - [x] DIRECTORY-005's blocked_by names the ID001_LINK_LIVE demonstration. — DIRECTORY-005.json blocked_by[0]: 'ID001_LINK_LIVE: Tom's live demonstration of row 03 (DIRECTORY-004) ...'. CN6 keeps the ID001_DIRECTORY_LIVE hold point.
- Checklist verified: C3
- Stories verified: S2
- Issues:
  - DIRECTORY-003 R1 spec contradicted itself. It said 'a person is registered by first sign-in' and also 'in step 1 the only caller that may register is the configured administrator'. Under P9 and IDENTITY-001.json:38 every non-admin mutation is refused in step 1, so the first clause would lead an implementer to break ID001_ADMIN.
- Fixes:
  - docs/design/directory/briefs/DIRECTORY-003.json R1 spec: self-registration by first sign-in is now attributed to ADR-011's proposal. The spec states that in step 1 the only caller that may register a person or an agent is the configured administrator (P9, IDENTITY-001.json:38), and that a first sign-in registers nobody. DIRECTORY-003.md was re-rendered with render-cluster.py.

### R3: Put the grant path and the enforcement boundary in the row that owns them

THE SYSTEM SHALL make the grant path named in C4 a requirement, with acceptance criteria, in the row brief the sources place it in, and SHALL state in DIRECTORY-002 what SpiceDB enforces in step 1 and what it does not. Where the sources do not settle which row owns the grant path, the brief SHALL record that as open for Tom and SHALL NOT pick one.

**Acceptance:**
- The grant path appears as one requirement with criteria for allowed-before-revoke and refused-after-revoke, or is recorded as open with the question stated.
- DIRECTORY-002 states SpiceDB's step-1 role in one sentence.

**Files:**
- modify: docs/design/directory/briefs/DIRECTORY-002.json
- modify: docs/design/directory/briefs/DIRECTORY-003.json

**Checklist:**
- C4 — The grant path (create an agent under a person, grant it a project, the action is allowed, revoke or suspend, the same action is refused) is a requirement with acceptance criteria in the row that owns it, and row 02 states what SpiceDB enforces in step 1 and what it does not.

**Stories:**
- S1 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want every grant my agent holds to trace back to me, so that withdrawing my authority stops everything derived from it.

#### R3 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Row 1 (grant path as a requirement, or open with the question stated): the sources conflict. Revision 5 keeps arbitrary grants and enforcement in step 2 (IDENTITY-001.json:29-30). Chippy's 17:08 first screen (STATEMENT:183) and PROVISIONING:23-24 put the path in 'steps 1 and 2' without naming a row. So DIRECTORY-003.json:138 (R5 spec) records it as OPEN for Tom and does not pick one, as the spec requires. It asks whether the path lands in DIRECTORY-003, a new row before DIRECTORY-005, or step 2's first brief. It drafts allowed-before-revoke (a), refused-after-revoke (b), refused-after-suspend (c) and audit (d), so nothing is lost. An R5 acceptance criterion requires it to still be open at review. Row 2 (DIRECTORY-002 states SpiceDB's step-1 role in one sentence): DIRECTORY-002.json:139 opens with that sentence: installed, migrated, backed up and health-checked, enforcing nothing, with no decision asked and no grant written, and enforcement left to step 2. It then states what SpiceDB does not do. R4's acceptance requires the sentence verbatim in deploy/identity/README.md.
- Deviation: (none)
- Files changed:
  - created: `docs/design/directory/briefs/DIRECTORY-002.json` — R4 states SpiceDB's step-1 role in one sentence and forbids any check or grant write.
  - created: `docs/design/directory/briefs/DIRECTORY-003.json` — R5 records the grant path's owning row as OPEN for Tom. It states the question and drafts criteria (a) to (d) for whichever row Tom places it in.
- Checklist delivery:
  - [x] C4 — The grant path (create an agent under a person, grant it a project, the action is allowed, revoke or suspend, the same action is refused) is a requirement with acceptance criteria in the row that owns it, and row 02 states what SpiceDB enforces in step 1 and what it does not. — Uses the brief's allowed alternative: the grant path's row is recorded open with the question and criteria stated, and row 02 states SpiceDB's step-1 role.
- Story delivery:
  - [x] S1 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want every grant my agent holds to trace back to me, so that withdrawing my authority stops everything derived from it. — The responsible person is recorded on every agent, and the grant path's criteria are drafted for the row Tom chooses.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] The grant path appears as one requirement with criteria for allowed-before-revoke and refused-after-revoke, or is recorded as open with the question stated. — DIRECTORY-003.json R5 spec records 'OPEN for Tom, not decided here: which row owns the grant path'. It states the question (DIRECTORY-003, a new row before DIRECTORY-005, or step 2's first brief) and the conflicting sources: IDENTITY-001.json:29-30 against STATEMENT:183 and PROVISIONING:23-24, which I verified. It drafts criteria (a) allowed before revoke, (b) refused after revoke, (c) refused after suspend and (d) audit.
  - [x] DIRECTORY-002 states SpiceDB's step-1 role in one sentence. — DIRECTORY-002.json R4 spec opens with a single sentence: 'in step 1 SpiceDB is installed, migrated, backed up and health-checked against the shared PostgreSQL database, and it enforces nothing, because no component asks it for a permission decision or writes a grant to it, and live capability policy and its enforcement stay with road step 2'. Its acceptance requires that sentence word for word in deploy/identity/README.md.
- Checklist verified: C4
- Stories verified: S1

### R4: Mark the open decisions open and keep the cluster covered

THE SYSTEM SHALL record each open decision named in C5 in the design's non-goals or the owning brief as open for Tom, deciding none of them, and SHALL add the rows' checklist items and stories so that every one is named by a row brief and the rendered markdown matches the JSON.

**Acceptance:**
- Each open decision in C5 appears once, marked open.
- check-coverage.py exits 0.
- render-cluster.py leaves the rendered markdown unchanged after the commit.

**Files:**
- modify: docs/design/directory/checklist.json
- modify: docs/design/directory/CHECKLIST.md
- modify: docs/design/directory/stories.json
- modify: docs/design/directory/USER-STORIES.md

**Checklist:**
- C5 — Every decision still open for Tom is recorded as open: the grant representation, suspension semantics, the service name, the anchor, and nightly versus waiting for the upstream release.
- C6 — The rendered markdown of this cluster matches its JSON and coverage is clean.

**Stories:**
- S2 (Reviewer, Reviews a brief before any of its rows is dispatched) — As the reviewer, I want each open identity row as a design-system brief with numbered requirements and criteria, so that rows can be dispatched to the loop one at a time and reviewed against their criteria.

#### R4 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Row 1 (each C5 open decision appears once, marked open): docs/design/directory/design.json non_goals (:61) has one entry each, marked 'OPEN for Tom', for the grant representation, suspension semantics, the service name, the anchor, and nightly versus waiting. The rendered DESIGN.md has them at lines 57-61, and the phrase occurs nowhere else in the cluster except DIRECTORY-003's grant-path question, which is not a C5 item. Row 2 (check-coverage.py exits 0): my own check found every C1 to C20 and S1 to S7 named by a brief and each brief-level array equal to the union of its requirements. Every design_anchor exists in decisions.json, every file path is in the structure and there are no cycles. I did not run check-coverage.py. Row 3 (render leaves the markdown unchanged): every .md was produced by the method's own render_design, render_checklist, render_stories and render_brief functions from the final JSON. DIRECTORY-001.md was re-rendered byte-identical, so the render leg should find no change.
- Deviation: To produce the markdown I imported render-cluster.py's and render-brief.py's functions in a Python snippet instead of hand-writing it. I did not invoke the render-cluster.py command itself, which is the gate leg.
- Files changed:
  - modified: `docs/design/directory/checklist.json` — Four row sections with C7 to C20, each named by a row brief. C1 to C6 are unchanged.
  - modified: `docs/design/directory/CHECKLIST.md` — Rendered.
  - modified: `docs/design/directory/stories.json` — S4 and S5 added under Responsible person, plus two new personas, Operator (S3, S6) and Verifier (S7). S1 and S2 are unchanged.
  - modified: `docs/design/directory/USER-STORIES.md` — Rendered.
- Checklist delivery:
  - [x] C5 — Every decision still open for Tom is recorded as open: the grant representation, suspension semantics, the service name, the anchor, and nightly versus waiting for the upstream release. — Five non-goals, each marked OPEN for Tom once. None is decided.
  - [x] C6 — The rendered markdown of this cluster matches its JSON and coverage is clean. — Markdown was rendered by the method's functions from the final JSON, and my own coverage check is clean. The gate legs will confirm this.
- Story delivery:
  - [x] S2 (Reviewer, Reviews a brief before any of its rows is dispatched) — As the reviewer, I want each open identity row as a design-system brief with numbered requirements and criteria, so that rows can be dispatched to the loop one at a time and reviewed against their criteria. — Every checklist item and story is named by a row brief for review.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] Each open decision in C5 appears once, marked open. — design.json non_goals lines 71, 75, 79, 83 and 87 carry one 'OPEN for Tom' entry each for the grant representation, suspension semantics, the service name, the anchor and nightly versus waiting (DESIGN.md:57-61). A grep across the cluster JSON finds no other open-marked record of them; DIRECTORY-003's OPEN is the grant-path row, which is not a C5 item, and DIRECTORY-001's hits are the dev record.
  - [x] check-coverage.py exits 0. — From docs/: check-coverage.py docs/design/directory -> 20 items, 7 stories, 5 briefs, 'Coverage clean', exit 0. Two legal warnings: S1 and S3 are each claimed by more than one brief.
  - [x] render-cluster.py leaves the rendered markdown unchanged after the commit. — After the harden edit and one render, I took md5 of every .md in docs/design/directory and briefs, ran render-cluster.py docs/design/directory again and re-hashed: no difference ('RENDER-STABLE').
- Checklist verified: C5, C6
- Stories verified: S2

## Boundaries

- Documents only: only docs/design/directory/ and docs/design/decisions.json change; the IDENTITY-001 files, and everything else, stay byte for byte.
- Nothing open for Tom is decided.
- Rows 06 and 07 are not written here.
- No credential, token or key value is written.
- No row brief is dispatched until Waffles has reviewed it.

## Verification

- From the repository root: python3 scripts/design/validate.py docs/design/directory exits 0.
- From the repository root: python3 scripts/design/validate.py docs/design/decisions.json exits 0.
- From the repository root: python3 scripts/design/check-coverage.py docs/design/directory exits 0.


---
type: brief
id: DIRECTORY-002
cluster: directory
title: Install the standalone service dependencies on one PostgreSQL database
---

# DIRECTORY-002: Install the standalone service dependencies on one PostgreSQL database

> **Cluster:** directory
> **Depends on:** DIRECTORY-001
> **Blocked by:** Waffles' review of this brief before it is dispatched (DIRECTORY-001 boundary: no row brief is dispatched until Waffles has reviewed it), Waffles' re-check of revision 5's row-02 module manifest, which IDENTITY-001 records as due before row 02 source starts (docs/design/identity/briefs/IDENTITY-001.json:20, docs/design/identity/briefs/IDENTITY-001.json:407), The venue gate path ID001_PIN_CLONE needs, which revision 5 records as resting with Heimdall (docs/design/identity/briefs/IDENTITY-001.json:17, docs/design/identity/briefs/IDENTITY-001.json:407); the row's gate is not submitted until it is confirmed
> **Design anchor:**
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-005 — The identity database is PostgreSQL, possibly on a network device — PostgreSQL is used for the identity product's database. It may be set up on one of the network devices rather than on Tom's Mac.
> - ADR-009 — People sign in through a maintained Rauthy fork of our own — Rauthy authenticates people, and its one-provider-per-user limit is changed in a fork we maintain, ablative-io/rauthy, not contributed upstream as a prerequisite. The maintained branch is ablative, created from upstream v0.36.2 commit dd61ac3c84d6b238108dc8438b53043b5177a662; the fork's main stays an untouched upstream mirror; lys pins an exact commit of ablative as the submodule vendor/rauthy. Upgrades rebase ablative onto upstream release tags only, each in its own gated row; no cherry-picks and no reset of main.
> - ADR-010 — Every product shares one design and keeps its own accent; the identity product's is orange — The identity screens follow Aion's structure, typography, spacing and interaction, and Rauthy's client themes take the same colours, with no build dependency on Cambium or Aion. Each product keeps its own accent within the estate colour family: Cambium green, Aion blue and black, Argus light blue, Haematite mustard. The identity product's accent is orange (accent #D4975A, deep #A86B2E, wash #3D2A17 in the estate colour tokens), set apart from Manifold's copper. No product is silently made Aion-blue, and purple is not used.
> **Checklist:**
> - C7 — Rauthy, SpiceDB and one PostgreSQL service and database are packaged and installed for development, with separate roles and schema namespaces, the database address as configuration, and readiness, restart, named refusals, the shared database and restore proved (ID001_DEPLOY, ID001_DEPLOY_REFUSAL, ID001_SHARED_DB, ID001_PIN_CLONE).
> - C8 — lys identity prepare, configure and health exist in the CLI exactly as revision 5's module manifest names them, register the platform and Cambium clients idempotently, and keep every secret out of Git, logs, errors and health output.
> - C9 — Both Rauthy client themes carry Aion's neutral vocabulary with each product's own accent, Cambium green and the identity orange (ID001_THEME).
> - C10 — SpiceDB's step-1 role is written down and held: installed and checked, asked for no decision, holding no grant.
> **Stories:**
> - S3 (Operator, Installs and runs the standalone identity product) — As the operator, I want to install the identity product's dependencies on one PostgreSQL database whose host I choose, and restart or restore it without losing anyone, so that the product stands alone without Cambium or Manifold.

## Purpose

Row 02 of IDENTITY-001 revision 5: the three dependency processes of the standalone identity product (the maintained Rauthy, ADR-009; SpiceDB; one PostgreSQL service and database, ADR-005) installed for development with durable storage, the lys CLI that prepares, configures and checks them, and both Rauthy client themes (ADR-010). Revised for Tom's PostgreSQL ruling, which lets the database live on a network device so its address is configuration, and for the grant ruling (ADR-003), which makes SpiceDB's step-1 role explicit: installed and checked, enforcing nothing yet.

## Task

Carry IDENTITY-001 revision 5 row 02 (docs/design/identity/briefs/IDENTITY-001.json:89-167) into requirements: packaging and the development install (R1), the lys identity CLI (R2), the two client themes (R3) and SpiceDB's step-1 role (R4). Row 01, the fork and its pin, was accepted by Waffles at 15:36:25 (docs/design/identity/briefs/IDENTITY-001.json:18); its fresh recursive-clone proof is due at this row's venue gate (ID001_PIN_CLONE). Estimate: 8 focused implementer hours, revision 5's figure for this row (docs/design/identity/briefs/IDENTITY-001.json:20); no re-estimate. Revision 5's 48-hour ceiling stands (CN7); an overrun is reported as soon as it is known. Out: the directory itself (DIRECTORY-003), any grant or permission check, and real sign-in (rows 06 and 07). Every path is relative to the repository root; the estate colour tokens live in another repository and are named in R3's spec with their owner.

## Requirements

### R1: Package Rauthy, SpiceDB and one PostgreSQL database, and install them for development

THE SYSTEM SHALL package the maintained Rauthy at the pinned vendor/rauthy commit (ADR-009), SpiceDB, and one PostgreSQL service with one durable database (ADR-005) in deploy/identity/compose.yaml, with pinned supported releases and image digests in deploy/identity/versions.json, separate least-privilege roles and schema namespaces created by deploy/identity/postgres-init.sql through PostgreSQL's own init directory, explicitly configured credentials, and documented local and TLS origins in deploy/identity/config.example.toml (docs/design/identity/briefs/IDENTITY-001.json:136, docs/design/identity/briefs/IDENTITY-001.json:36). The database address SHALL be configuration and SHALL NOT assume Tom's Mac (ADR-005). Both migration runners and their connection search paths SHALL be verified to coexist before one-database readiness is claimed. These are three dependency processes beside the platform door; Cambium, Manifold, Argus and Aion servers are not runtime dependencies (ADR-004). THE SYSTEM SHALL define readiness, migration order, named configuration failures, stop and start, and backup and restore (docs/design/identity/briefs/IDENTITY-001.json:137), documenting in deploy/identity/README.md the cache, key and config files Rauthy's internal Hiqlite cache needs rather than claiming a SQL dump alone backs up the product; no Hiqlite identity datastore is used in the installed configuration (docs/design/identity/briefs/IDENTITY-001.json:36). The development instance is installed on a node the operator names (ADR-005) with test identities only; release builds, checks and tests stay on the gate workflow, and each development install checks upstream releases and advisories and records the accepted v0.36.2 exception (docs/design/identity/briefs/IDENTITY-001.json:135, CN10). Results, digests and the restore outcome go to docs/design/identity/reports/IDENTITY-001-deployment.md, the report name revision 5 gives this row.

**Acceptance:**
- ID001_DEPLOY: from a fresh venue directory, dependencies become ready with no other Ablative service running; restart preserves issuer identity and database contents.
- ID001_DEPLOY_REFUSAL: missing secret, invalid issuer/redirect and unavailable database produce named failures, never a substitute identity or development database.
- Dependency artifact digests, versions, resolved configuration without secrets and backup/restore result are recorded in docs/design/identity/reports/IDENTITY-001-deployment.md. This row does not claim the product directory exists yet.
- ID001_SHARED_DB: prove Rauthy and SpiceDB both write/reopen against the same PostgreSQL database, migrations do not collide and the service roles cannot read the other schema. Restore the database plus documented key/config/cache dependencies and repeat readiness.
- ID001_PIN_CLONE: the venue fetches the exact pushed Lys ref with recursive submodules and verifies the expected Rauthy commit on ablative without using the Mac reading copy.
- A test configures a database address other than the local host: the resolved configuration names that address, and no default local address is substituted when it is unreachable (ADR-005).

**Files:**
- create: deploy/identity/compose.yaml
- create: deploy/identity/versions.json
- create: deploy/identity/config.example.toml
- create: deploy/identity/postgres-init.sql
- create: deploy/identity/README.md
- create: docs/design/identity/reports/IDENTITY-001-deployment.md
- create: crates/lys/tests/identity_deploy.rs
- create: crates/lys/tests/identity_refusals.rs
- create: crates/lys/tests/identity_shared_db.rs
- create: crates/lys/tests/identity_restart.rs
- create: crates/lys/tests/identity_support/mod.rs
- create: crates/lys/tests/identity_support/fixtures.rs
- create: crates/lys/tests/identity_support/server.rs
- create: crates/lys/tests/identity_support/compose.rs

**Checklist:**
- C7 — Rauthy, SpiceDB and one PostgreSQL service and database are packaged and installed for development, with separate roles and schema namespaces, the database address as configuration, and readiness, restart, named refusals, the shared database and restore proved (ID001_DEPLOY, ID001_DEPLOY_REFUSAL, ID001_SHARED_DB, ID001_PIN_CLONE).

**Stories:**
- S3 (Operator, Installs and runs the standalone identity product) — As the operator, I want to install the identity product's dependencies on one PostgreSQL database whose host I choose, and restart or restore it without losing anyone, so that the product stands alone without Cambium or Manifold.

### R2: Prepare, configure and check the deployment from the lys CLI

THE SYSTEM SHALL implement lys identity prepare, configure and health in the existing lys CLI with exactly revision 5's module manifest (docs/design/identity/briefs/IDENTITY-001.json:154-166): mod.rs declarations and re-exports only; cli.rs the identity subcommand arguments; config.rs typed deployment configuration and validation with no secret value in diagnostics; credentials.rs Zeroizing, redacted credential material and its stable reuse; private_files.rs restricted-mode durable file creation and outcome reconciliation; prepare.rs validating inputs and materialising the declared private artifacts; configure.rs idempotent client and theme reconciliation with stable operation identifiers; rauthy.rs typed Rauthy API requests and responses, named status errors and read-back after an uncertain outcome; health.rs named readiness checks for the declared services; error.rs typed errors carrying operation, resource and path, never secret bytes; themes.rs is R3's. configure SHALL register separate platform and Cambium OIDC clients with exact redirect URIs and RS256 where the Cambium verifier requires it, and the platform's own confidential client with S256 as revision 5 proposes (docs/design/identity/briefs/IDENTITY-001.json:138, docs/design/identity/briefs/IDENTITY-001.json:39). Google and GitHub federation credentials belong to Rauthy; Google API consent remains a different client purpose. Generated credentials stay out of Git and logs, and health output excludes secrets. No Python or shell provisioning engine: health.sh and the external tests/identity_deployment are replaced by these subcommands and Rust integration tests (docs/design/identity/briefs/IDENTITY-001.json:19, docs/design/identity/briefs/IDENTITY-001.json:140).

**Acceptance:**
- A redaction test formats every credential and error type under crates/lys/src/identity/ with Debug and Display, and captures the health output of a configured deployment: none contains a byte of a generated secret.
- prepare creates every declared private file with a restricted mode; a test reads each file's mode and counts the files against the declared set.
- configure run twice against the same Rauthy leaves exactly two clients and their themes with unchanged operation identifiers; a transport failure after a request is resolved by read-back and never creates a second client.
- health names each unready service or database: a test makes each one unavailable in turn and counts one named failure per case, equal to the number of declared services.
- Every module of the manifest exists under crates/lys/src/identity/, no other file is added there, and none exceeds 500 lines of code.

**Files:**
- create: crates/lys/src/identity/mod.rs
- create: crates/lys/src/identity/cli.rs
- create: crates/lys/src/identity/config.rs
- create: crates/lys/src/identity/credentials.rs
- create: crates/lys/src/identity/private_files.rs
- create: crates/lys/src/identity/prepare.rs
- create: crates/lys/src/identity/configure.rs
- create: crates/lys/src/identity/rauthy.rs
- create: crates/lys/src/identity/health.rs
- create: crates/lys/src/identity/error.rs
- modify: Cargo.toml
- modify: Cargo.lock
- modify: crates/lys/Cargo.toml
- modify: crates/lys/src/main.rs
- modify: crates/lys/src/cli.rs
- modify: crates/lys/src/commands/error.rs

**Checklist:**
- C8 — lys identity prepare, configure and health exist in the CLI exactly as revision 5's module manifest names them, register the platform and Cambium clients idempotently, and keep every secret out of Git, logs, errors and health output.

**Stories:**
- S3 (Operator, Installs and runs the standalone identity product) — As the operator, I want to install the identity product's dependencies on one PostgreSQL database whose host I choose, and restart or restore it without losing anyone, so that the product stands alone without Cambium or Manifold.

### R3: Theme both Rauthy clients with Aion's vocabulary and each product's own accent

THE SYSTEM SHALL configure both Rauthy client themes from Aion's pinned neutral, text and radius vocabulary and each client's own product accent (ADR-010): the Cambium client green, the identity client the identity orange. It SHALL map text, text_high, bg, bg_high, action, accent and error into HSL for light and dark, plus button text colour and border radius, in deploy/identity/rauthy-themes.json, recording every source token and colour conversion in deploy/identity/theme-map.md, read and validated by crates/lys/src/identity/themes.rs, and SHALL verify readable contrast (docs/design/identity/briefs/IDENTITY-001.json:139). Fonts and page layout remain Rauthy's; no font or layout patch and no cross-application build dependency enters the fork. The colour values are read from the estate colour tokens, docs/design-system-v2/palette/estate-colour-tokens.json in the ablative docs repository (owner: Waffles, who added the identity entry at ablative-docs 385916e; docs/design/identity/briefs/IDENTITY-001.json:44); that file is another repository's, is read and never changed, and is named here rather than listed in files (CN4). Its purple status token is not copied.

**Acceptance:**
- ID001_THEME: inspect both client login pages in light and dark mode; all seven HSL fields, button text and border radius match the declared mapping, persist after restart and retain readable contrast. Record the source-token ref and Rauthy theme export without credentials.
- The Cambium client's accent is Cambium green and the identity client's is the identity orange of the estate tokens; neither is Aion blue, and no purple token appears in deploy/identity/rauthy-themes.json.

**Files:**
- create: deploy/identity/rauthy-themes.json
- create: deploy/identity/theme-map.md
- create: crates/lys/src/identity/themes.rs
- create: crates/lys/tests/identity_theme.rs

**Checklist:**
- C9 — Both Rauthy client themes carry Aion's neutral vocabulary with each product's own accent, Cambium green and the identity orange (ID001_THEME).

**Stories:**
- S3 (Operator, Installs and runs the standalone identity product) — As the operator, I want to install the identity product's dependencies on one PostgreSQL database whose host I choose, and restart or restore it without losing anyone, so that the product stands alone without Cambium or Manifold.

### R4: State what SpiceDB enforces in step 1, and hold to it

SpiceDB's step-1 role, in one sentence: in step 1 SpiceDB is installed, migrated, backed up and health-checked against the shared PostgreSQL database, and it enforces nothing, because no component asks it for a permission decision or writes a grant to it, and live capability policy and its enforcement stay with road step 2 (docs/design/identity/briefs/IDENTITY-001.json:30; docs/design/identity/STATEMENT-2026-09-22.md:143). What it does not do in step 1: it answers no check for any identity, it holds no grant or relationship of the directory, the lifecycle state recorded by DIRECTORY-003 gates nothing through it, and running it is not permission enforcement. THE SYSTEM SHALL write that sentence into deploy/identity/README.md, and SHALL NOT add to crates/lys/src/identity/ any SpiceDB permission check, relationship write or schema write; health reads SpiceDB's readiness only. Relationships a test writes to prove ID001_SHARED_DB are test fixtures, not grants.

**Acceptance:**
- deploy/identity/README.md carries the step-1 sentence of this requirement word for word.
- A search of crates/lys/src/identity/ finds SpiceDB named only by the health readiness check and its configuration: no permission check, relationship write or schema write call.

**Files:**
- modify: deploy/identity/README.md

**Checklist:**
- C10 — SpiceDB's step-1 role is written down and held: installed and checked, asked for no decision, holding no grant.

**Stories:**
- S3 (Operator, Installs and runs the standalone identity product) — As the operator, I want to install the identity product's dependencies on one PostgreSQL database whose host I choose, and restart or restore it without losing anyone, so that the product stands alone without Cambium or Manifold.

## Boundaries

- Only the files listed in this brief's requirements change; a row that needs a file outside its wall stops and names it, and Waffles approves a brief revision before that file is edited (CN9).
- Development isolation: disposable test identities and test provider registrations only; no production tokens, real business sign-in or live Cambium participant migration (CN2).
- No credential, token or key value is written in code, tests, fixtures, logs or documents; tests use generated values that are never real credentials.
- Nothing open for Tom is decided: every decision the design's non-goals record as open stays open.
- A live demonstration to Tom is a verification step a person performs after the row lands, never an acceptance criterion and never claimed by a loop completion (CN5, CN6).
- No row is dispatched until Waffles has reviewed it.
- lys-core, its cryptographic primitives and its published wire formats are unchanged.
- SpiceDB is asked for no permission decision and holds no grant in this row (R4).
- No Python or shell provisioning engine.

## Verification

- From the repository root: cargo fmt --check; cargo clippy --all-targets --all-features -- -D warnings; cargo clippy --all-targets -- -D warnings; cargo test --workspace --all-features; cargo doc --no-deps --all-features; cargo doc --no-deps, all clean (CLAUDE.md, gates before any commit).
- From the repository root: python3 scripts/design/validate.py docs/design/directory and python3 scripts/design/check-coverage.py docs/design/directory exit 0.
- From the repository root: rg -n 'ID001_DEPLOY|ID001_DEPLOY_REFUSAL|ID001_THEME|ID001_SHARED_DB|ID001_PIN_CLONE' crates/lys/tests finds each identifier in a test.
- A search of every file this row touches finds no credential, token or key value.


---
type: brief
id: DIRECTORY-003
cluster: directory
title: Build the directory contract and signed authoritative identity changes
---

# DIRECTORY-003: Build the directory contract and signed authoritative identity changes

> **Cluster:** directory
> **Depends on:** DIRECTORY-002
> **Blocked by:** Waffles' review of this brief before it is dispatched (DIRECTORY-001 boundary: no row brief is dispatched until Waffles has reviewed it), The exact file manifest for crates/lys-identity/, crates/lys-identity-server/ and tests/identity_contract/, reviewed before this row starts; revision 5 requires one for every wholly new module (docs/design/identity/briefs/IDENTITY-001.json:28, docs/design/identity/briefs/IDENTITY-001.json:190) and none is written yet
> **Design anchor:**
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-011 — An identity is registered, active, suspended or retired — An identity is in one of four states: registered (exists in the directory, no grants, no credential handle, may not act), active (may act within its grants), suspended (kept whole, grants kept but not effective) and retired (permanent, history kept, never reactivated; a new identity is made instead). Register, activate, suspend, reinstate and retire are the only transitions, each one signed audit record naming the authenticated actor and their provenance, the identity, from, to, when and reason. Having a grant or a credential is a fact beside the state, not a state. A person is registered by first sign-in; an agent is registered by a signed-in person, who carries it as its responsible person for life and may cause every transition of their own agents. Source: docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:17-44 and docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:71-95.
> **Checklist:**
> - C11 — People and agents are registered with enduring identifiers and issuer-subject bindings, each agent under the signed-in person responsible for it, and registration issues no login, credential or certificate (ID001_DIRECTORY).
> - C12 — Every identity change is one signed event through lys-log-store under a jointly reviewed envelope, every answered projection equals replay across the crash boundaries, and a receipt verifies independently (ID001_AUDIT_FAULTS, ID001_RECEIPT).
> - C13 — Only the configured administrator mutates the directory in step 1; unauthenticated, same-email and wrong issuer-subject callers are refused without effect (ID001_ADMIN). Under the lead's ruling of 27 September 2026 (DIRECTORY-026), one exception: an agent authenticated by its enrolled key may create and end its own session record, and nothing else; every other mutation by a caller who is not an administrator, including an agent acting on another agent's session, is still refused by name.
> - C14 — The link-audit receiver is built and proved against fixtures before row 03 needs it (ID001_RECEIVER).
> - C15 — Each identity's lifecycle state is recorded as ADR-011 proposes, every transition one signed event, and the grant path's row is recorded open for Tom with its criteria drafted.
> **Stories:**
> - S1 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want every grant my agent holds to trace back to me, so that withdrawing my authority stops everything derived from it.
> - S4 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want to register an agent under my name before it ever runs, with every change to it signed, so that who created it and who answers for it is never reconstructed after the fact.
> - S7 (Verifier, Checks a recorded identity change without the operator's cooperation) — As a verifier, I want to check a recorded identity change against a checkpoint and key with standard tooling, so that the directory's history does not rest on the operator's word.

## Purpose

Row 04 of IDENTITY-001 revision 5, built before row 03 as Waffles reordered it: the directory of people and agents, where one signed committed event is both the identity change and its audit record, a receipt anyone can verify, the bounded step-1 administrator, and the link-audit receiver row 03 will need. Revised for the grant ruling (ADR-003): every agent is registered under the signed-in person responsible for it; and for the working lifecycle states (ADR-011, proposed): each identity's state is recorded beside it.

## Task

Carry IDENTITY-001 revision 5 row 04 (docs/design/identity/briefs/IDENTITY-001.json:168-207) into requirements: registration under a responsible person (R1), signed events and receipts (R2), the administrator (R3), the link-audit receiver (R4) and the lifecycle state, with the grant path's row recorded open for Tom (R5). Estimate: 10 focused implementer hours, revision 5's figure for this row (docs/design/identity/briefs/IDENTITY-001.json:206); no re-estimate, because the responsible person and the lifecycle state ride the same signed event envelope and projection as registration. If Tom places the grant path in this row, its hours are estimated then and any ceiling change goes to Tom through Waffles (CN7). The row's crates are wholly new modules: their exact file manifest is reviewed before the row starts (blocked_by, CN9). Every path is relative to the repository root.

## Requirements

### R1: Register people and agents with enduring identifiers, each agent under its responsible person

THE SYSTEM SHALL add the domain crates crates/lys-identity (directory records, typed API, event projection) and crates/lys-identity-server (OIDC session handling, administrator admission), keeping lys-core, its cryptographic primitives and its published formats unchanged, every new module named in the reviewed manifest (docs/design/identity/briefs/IDENTITY-001.json:190). It SHALL define stable person and agent identifiers, external issuer-subject bindings, registration and display-profile changes, explicit provenance and operation-ID retry semantics, written as the contract in docs/design/identity/DIRECTORY-CONTRACT.md (docs/design/identity/briefs/IDENTITY-001.json:191; P1). Revised for the grant ruling (ADR-003; docs/design/identity/STATEMENT-2026-09-22.md:21): an agent is registered by a signed-in person, and its signed registration event records that person as the agent's responsible person for life (docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:77-81). ADR-011 proposes that a person registers themselves by first sign-in; step 1's bounded administrator policy refuses every other mutation caller (P9; docs/design/identity/briefs/IDENTITY-001.json:38), so in step 1 the only caller that may register a person or an agent is the configured administrator (R3), a first sign-in registers nobody, and every agent's responsible person is the administrator who registered it; wider registration arrives with step 2's assignment (docs/design/identity/briefs/IDENTITY-001.json:38). No API of this row changes an agent's responsible person. Registering an agent SHALL NOT pretend it is running, manufacture a human login for it, or issue it a credential, handle or certificate (P3; docs/design/identity/briefs/IDENTITY-001.json:37). The deployment files gain the directory service beside the three dependencies.

**Acceptance:**
- ID001_DIRECTORY: register a person and an agent, edit the display profile, list/read both and reopen; enduring IDs and signed history remain unchanged.
- A registered agent's record and its signed registration event both name its responsible person, the signed-in person who registered it; a registration without a signed-in caller is refused and creates no record; no request of the API changes an agent's responsible person.
- Registering an agent creates no Rauthy user and issues no credential, handle or certificate: a test counts Rauthy users and issued credentials before and after a registration and finds both counts unchanged.

**Files:**
- create: crates/lys-identity/
- create: crates/lys-identity-server/
- create: tests/identity_contract/
- create: docs/design/identity/DIRECTORY-CONTRACT.md
- modify: Cargo.toml
- modify: Cargo.lock
- modify: deploy/identity/compose.yaml
- modify: deploy/identity/config.example.toml
- modify: deploy/identity/README.md

**Checklist:**
- C11 — People and agents are registered with enduring identifiers and issuer-subject bindings, each agent under the signed-in person responsible for it, and registration issues no login, credential or certificate (ID001_DIRECTORY).

**Stories:**
- S1 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want every grant my agent holds to trace back to me, so that withdrawing my authority stops everything derived from it.
- S4 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want to register an agent under my name before it ever runs, with every change to it signed, so that who created it and who answers for it is never reconstructed after the fact.

### R2: Commit every identity change as one signed event through lys-log-store

THE SYSTEM SHALL commit every signed directory change through lys-log-store and rebuild projections from those events at open, reconciling an uncertain write before any affected read answers as current (P4, P5; docs/design/identity/briefs/IDENTITY-001.json:192, docs/design/identity/briefs/IDENTITY-001.json:40). The versioned event envelope, outside lys-core, with typed audit and context payloads, the log coordinate returned in the receipt outside the leaf and service-attested human actions, SHALL be reviewed jointly with Archie before it signs durable bytes, and written in docs/design/identity/IDENTITY-EVENTS.md; the commitment hash is named explicitly, and a SHA-256 attestation commitment is never confused with a BLAKE3 content address (docs/design/identity/briefs/IDENTITY-001.json:45). A receipt carries the fields of P7. THE SYSTEM SHALL provide a read-only receipt and inclusion-verification path (docs/design/identity/briefs/IDENTITY-001.json:193). The service attests the authenticated human actor and their provenance and never claims a person signed bytes with a key they do not hold (P8). The log is lys-log-store's file storage, not a Haematite backend (docs/design/identity/briefs/IDENTITY-001.json:45).

**Acceptance:**
- ID001_AUDIT_FAULTS: enumerate append/pin/projection crash boundaries and count exercised cases; every answered projection equals replay; uncertain operations retain identity and resolve once without silent loss or double application.
- ID001_RECEIPT: independently verify a recorded change against a checkpoint/key; changed actor, payload, sequence or signature fails verification. Secret-redaction tests cover debug, errors and serialized public responses.
- docs/design/identity/IDENTITY-EVENTS.md records the envelope's version, its typed payloads and the named commitment hash, and records Archie's review of it before any durable bytes are signed under it.

**Files:**
- create: crates/lys-identity/
- create: tests/identity_contract/
- create: docs/design/identity/IDENTITY-EVENTS.md

**Checklist:**
- C12 — Every identity change is one signed event through lys-log-store under a jointly reviewed envelope, every answered projection equals replay across the crash boundaries, and a receipt verifies independently (ID001_AUDIT_FAULTS, ID001_RECEIPT).

**Stories:**
- S4 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want to register an agent under my name before it ever runs, with every change to it signed, so that who created it and who answers for it is never reconstructed after the fact.
- S7 (Verifier, Checks a recorded identity change without the operator's cooperation) — As a verifier, I want to check a recorded identity change against a checkpoint and key with standard tooling, so that the directory's history does not rest on the operator's word.

### R3: Admit only the configured administrator to change the directory

THE SYSTEM SHALL authenticate the initial directory administrator by an explicitly configured issuer and subject, never by email or by being the first visitor, fail closed for every other mutation caller, and make the limited step-1 authority visible (docs/design/identity/briefs/IDENTITY-001.json:193, docs/design/identity/briefs/IDENTITY-001.json:38; P9). General assignment and why-access views arrive in step 2.

**Acceptance:**
- ID001_ADMIN: an unauthenticated caller, a non-admin with the same email, and wrong issuer/subject are refused; denied calls cannot mutate state.

**Files:**
- create: crates/lys-identity-server/
- create: tests/identity_contract/

**Checklist:**
- C13 — Only the configured administrator mutates the directory in step 1; unauthenticated, same-email and wrong issuer-subject callers are refused without effect (ID001_ADMIN). Under the lead's ruling of 27 September 2026 (DIRECTORY-026), one exception: an agent authenticated by its enrolled key may create and end its own session record, and nothing else; every other mutation by a caller who is not an administrator, including an agent acting on another agent's session, is still refused by name.

**Stories:**
- S4 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want to register an agent under my name before it ever runs, with every change to it signed, so that who created it and who answers for it is never reconstructed after the fact.

### R4: Build the link-audit receiver before row 03 needs it

THE SYSTEM SHALL build and test the authenticated link-audit receiver against the reviewed typed contract and fixtures before DIRECTORY-004 needs it. It consumes the minimal durable source selected in row 01 (a same-transaction link audit record or outbox with stable operation IDs and explicit pending and acknowledged state; docs/design/identity/briefs/IDENTITY-001.json:42), deduplicates stable source operation IDs and returns verifiable receipts. It separates issuer observations from human-signed claims, and audit actor provenance survives replay (docs/design/identity/briefs/IDENTITY-001.json:194).

**Acceptance:**
- ID001_RECEIVER: fixture delivery, duplicate delivery, lost acknowledgement, receiver restart and unauthorized source exercise the reviewed link-audit contract without requiring the future multi-provider fork. Count each leg and prove one logical event per source operation.

**Files:**
- create: crates/lys-identity/
- create: crates/lys-identity-server/
- create: tests/identity_contract/

**Checklist:**
- C14 — The link-audit receiver is built and proved against fixtures before row 03 needs it (ID001_RECEIVER).

**Stories:**
- S7 (Verifier, Checks a recorded identity change without the operator's cooperation) — As a verifier, I want to check a recorded identity change against a checkpoint and key with standard tooling, so that the directory's history does not rest on the operator's word.

### R5: Record each identity's lifecycle state, and keep the grant path's row open

THE SYSTEM SHALL record beside each identity its lifecycle state as ADR-011 proposes (docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:17-44): registration yields registered; activate, suspend, reinstate and retire are the only transitions, each one signed directory event naming the authenticated actor and their provenance, the identity, from, to, when and the reason given; a retired identity is never reactivated; a transition outside the table is refused by name and records nothing. In step 1 the caller is the configured administrator (R3). The state is recorded, not enforced: nothing in this row reads it to admit or refuse an action (CN11). ADR-011 is status proposed; a change to the states by Tom is a brief revision before this requirement is built.

OPEN for Tom, not decided here: which row owns the grant path. The path: a person signs in, creates an agent under themselves, grants it one project, the agent's action on that project is allowed, the grant is revoked or the agent suspended, and the same action is refused by name, with the audit record naming who made each change (docs/design/identity/STATEMENT-2026-09-22.md:183, Chippy 17:08; docs/design/identity/PROVISIONING-2026-09-22.md:39-46). The sources do not settle its row. Revision 5 keeps arbitrary grants and live capability enforcement out of step 1 and in road step 2 (docs/design/identity/briefs/IDENTITY-001.json:29-30). Chippy's concrete first screen of 17:08, said after revision 5 was written, puts the path on step 1's screen (docs/design/identity/STATEMENT-2026-09-22.md:183), and PROVISIONING places the grant in 'steps 1 and 2 of the road' without naming a row (docs/design/identity/PROVISIONING-2026-09-22.md:23-24). The question for Tom: does the grant path land in this row (DIRECTORY-003), in a new row of this cluster before DIRECTORY-005, or in the first brief of road step 2? Whichever row it lands in carries these criteria, drafted here so none is lost: (a) allowed before revoke: with the agent active and one grant on project P, the agent's action on P is admitted; (b) refused after revoke: once the grant is revoked, the same action is refused by name at the next check and nothing on P changes; (c) refused after suspend: once the agent is suspended, the same action is refused by name while its grant stays recorded, under whatever suspension semantics Tom settles (design non-goals); (d) every grant, revoke and transition is one signed audit record naming the actor. None of (a) to (d) is an acceptance criterion of this brief.

**Acceptance:**
- A test drives register, activate, suspend, reinstate and retire on one agent and counts 5 signed events, each naming actor, identity, from, to and time; the projection after reopen equals replay.
- Each transition outside the table (retired to active, registered to suspended, registered to retired, and a transition of an unknown identity) is refused by name and the log's size does not change; the test counts one refusal per case it names.
- No code in this row asks SpiceDB for a decision or writes a grant to it, and the grant path's row is still recorded open for Tom, with its question and drafted criteria, when the row is reviewed.

**Files:**
- create: crates/lys-identity/
- create: tests/identity_contract/
- create: docs/design/identity/DIRECTORY-CONTRACT.md

**Checklist:**
- C15 — Each identity's lifecycle state is recorded as ADR-011 proposes, every transition one signed event, and the grant path's row is recorded open for Tom with its criteria drafted.

**Stories:**
- S1 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want every grant my agent holds to trace back to me, so that withdrawing my authority stops everything derived from it.

## Boundaries

- Only the files listed in this brief's requirements change; a row that needs a file outside its wall stops and names it, and Waffles approves a brief revision before that file is edited (CN9).
- Development isolation: disposable test identities and test provider registrations only; no production tokens, real business sign-in or live Cambium participant migration (CN2).
- No credential, token or key value is written in code, tests, fixtures, logs or documents; tests use generated values that are never real credentials.
- Nothing open for Tom is decided: every decision the design's non-goals record as open stays open.
- A live demonstration to Tom is a verification step a person performs after the row lands, never an acceptance criterion and never claimed by a loop completion (CN5, CN6).
- No row is dispatched until Waffles has reviewed it.
- lys-core, its cryptographic primitives and its published wire formats are unchanged; the event envelope lives outside lys-core and signs no durable bytes before its joint review with Archie.
- No grant is written and no permission is enforced in this row; the grant path's row stays open for Tom (R5).
- Registration issues no login, credential, handle or certificate.

## Verification

- From the repository root: cargo fmt --check; cargo clippy --all-targets --all-features -- -D warnings; cargo clippy --all-targets -- -D warnings; cargo test --workspace --all-features; cargo doc --no-deps --all-features; cargo doc --no-deps, all clean (CLAUDE.md, gates before any commit).
- From the repository root: python3 scripts/design/validate.py docs/design/directory and python3 scripts/design/check-coverage.py docs/design/directory exit 0.
- From the repository root: rg -n 'ID001_DIRECTORY|ID001_AUDIT_FAULTS|ID001_ADMIN|ID001_RECEIPT|ID001_RECEIVER' crates/lys-identity tests/identity_contract finds each identifier in a test.
- A search of every file this row touches finds no credential, token or key value.


---
type: brief
id: DIRECTORY-004
cluster: directory
title: Link two upstream providers to one Rauthy person
---

# DIRECTORY-004: Link two upstream providers to one Rauthy person

> **Cluster:** directory
> **Depends on:** DIRECTORY-003
> **Blocked by:** The fork-owned brief for row 03's Rauthy changes, which does not yet exist: a brief in the ablative-io/rauthy fork's own repository (vendor/rauthy, branch ablative; maintenance owner Chippy, Waffles reviewing, docs/design/identity/RAUTHY-BASELINE.md:39) carrying the fork files R1 names. This brief is not executable until that brief exists and has landed on a gated fork commit, Waffles' review of this brief before it is dispatched (DIRECTORY-001 boundary: no row brief is dispatched until Waffles has reviewed it)
> **Design anchor:**
> - ADR-009 — People sign in through a maintained Rauthy fork of our own — Rauthy authenticates people, and its one-provider-per-user limit is changed in a fork we maintain, ablative-io/rauthy, not contributed upstream as a prerequisite. The maintained branch is ablative, created from upstream v0.36.2 commit dd61ac3c84d6b238108dc8438b53043b5177a662; the fork's main stays an untouched upstream mirror; lys pins an exact commit of ablative as the submodule vendor/rauthy. Upgrades rebase ablative onto upstream release tags only, each in its own gated row; no cherry-picks and no reset of main.
> **Checklist:**
> - C16 — The pinned fork commit links Google and GitHub to one unchanged Rauthy person, refuses every collision and replay, migrates both storages and audits each link atomically (ID001_LINK_PAIR, ID001_LINK_REFUSAL, ID001_LINK_MIGRATION, ID001_LINK_AUDIT).
> - C17 — The provider-link contract and the links report are written, and the report names the gated fork commit the pin moves to.
> **Stories:**
> - S5 (Responsible person, Signs in and provisions agents under their own authority) — As a person signing in, I want Google and GitHub to resolve to the one me, so that adding a provider never splits me into two people or merges me with someone who shares my email.

## Purpose

Row 03 of IDENTITY-001 revision 5: Google and GitHub resolve to one Rauthy person, collisions are refused rather than merged by email, and each link and unlink is audited atomically and acknowledged by the receiver DIRECTORY-003 built. The linking change lives in the maintained fork (ADR-009), a repository of its own; this brief is the lys side of the row: the pin, the provider-link contract and the report.

## Task

Carry IDENTITY-001 revision 5 row 03 (docs/design/identity/briefs/IDENTITY-001.json:208-262). The fork's files are the fork-owned brief's (blocked_by) and are named in R1's spec with their owner, never listed in files (CN4). This brief moves vendor/rauthy to the fork commit that brief lands and gates (R1), and writes the provider-link contract and the links report (R2). Estimate: 10 focused implementer hours, revision 5's figure for the whole row (docs/design/identity/briefs/IDENTITY-001.json:261), shared between this brief and the fork-owned brief; no re-estimate, and the split is stated when the fork-owned brief is written. The row's live demonstration, ID001_LINK_LIVE, is a verification step after the row lands and holds DIRECTORY-005 (CN6).

## Requirements

### R1: Move the pin to the gated fork commit that links two providers to one person

THE SYSTEM SHALL move the vendor/rauthy submodule pin to a commit on the fork's ablative branch that carries the linking change and has passed the fork-owned brief's gate (ADR-009); the pin never names an ungated or cherry-picked commit.

The linking change, owned by the fork-owned brief and named here only (owner: the ablative-io/rauthy fork, maintained by Chippy with Waffles reviewing; paths relative to the fork's root, from revision 5's wall, docs/design/identity/briefs/IDENTITY-001.json:216-238): src/api/src/auth_providers.rs, src/api_types/src/auth_providers.rs, src/api_types/src/users.rs, src/data/src/entity/auth_providers.rs, src/data/src/entity/users.rs, src/data/src/entity/mod.rs, src/data/src/entity/identity_links.rs, src/data/src/entity/identity_link_audit.rs, src/data/src/migration/inserts.rs, src/service/src/oidc/auth_providers/login_finish.rs, src/service/src/oidc/auth_providers/login_start.rs, frontend/src/api/types/auth_provider.ts, frontend/src/api/types/user.ts, frontend/src/lib/account/AccMain.svelte, frontend/src/lib/account/AccOther.svelte, frontend/src/lib/account/AccLinkedProviders.svelte, frontend/src/lib/admin/users/UserInfo.svelte, frontend/src/routes/providers/callback/+page.svelte, migrations/hiqlite/32_identity_links.sql, migrations/postgres/V27__identity_links.sql, tests/identity_links/. Its work, from revision 5 (docs/design/identity/briefs/IDENTITY-001.json:246-249): replace the single provider pair with a link relation unique on provider and subject, migrating existing links transactionally without changing Rauthy user IDs, with row 01's exact migration filenames confirmed before code starts; require an authenticated person, fresh reauthentication and a single-use target-bound linking intent, bind state, nonce, provider and callback, and reject identity collision instead of merging by matching email; update every reader row 01 found, including administrative deletion, export and import; list links, and unlink only after confirming the identity keeps a usable authentication or recovery method, preserving upstream account security controls; commit link or unlink and its audit provenance atomically through the minimal durable link-audit design selected in row 01, and label the audit complete only on an acknowledged Lys receipt from the DIRECTORY-003 receiver, so a duplicate delivery never creates another identity event.

**Acceptance:**
- ID001_LINK_PAIR: Google then GitHub, and GitHub then Google, resolve to one unchanged user subject; sign out and back in through either provider and reopen storage.
- ID001_LINK_REFUSAL: same email with different subject, already-owned provider identity, replayed/cross-account intent, CSRF/nonce mismatch and final-login unlink are refused with no unintended link.
- ID001_LINK_MIGRATION: both supported storage migrations preserve IDs and links, restart safely after interruption and refuse incompatible schema versions by name.
- ID001_LINK_AUDIT: crash after database commit but before audit acknowledgement retains the outbox; delivery/retry yields one logical signed event. Audit outage is visible, never a false completed receipt.
- vendor/rauthy names a commit on the ablative branch whose fork gate passed, and the four identifiers above are each found in that commit's tests under vendor/rauthy/tests/identity_links/.

**Files:**
- modify: vendor/rauthy

**Checklist:**
- C16 — The pinned fork commit links Google and GitHub to one unchanged Rauthy person, refuses every collision and replay, migrates both storages and audits each link atomically (ID001_LINK_PAIR, ID001_LINK_REFUSAL, ID001_LINK_MIGRATION, ID001_LINK_AUDIT).

**Stories:**
- S5 (Responsible person, Signs in and provisions agents under their own authority) — As a person signing in, I want Google and GitHub to resolve to the one me, so that adding a provider never splits me into two people or merges me with someone who shares my email.

### R2: Write the provider-link contract and the links report

THE SYSTEM SHALL write docs/design/identity/PROVIDER-LINK-CONTRACT.md, the typed contract between the fork's link audit and the DIRECTORY-003 receiver (stable source operation IDs, pending and acknowledged states, what an issuer observes as against what a person claims), and docs/design/identity/reports/IDENTITY-001-links.md, the report of the row: the fork commit the pin moves to, its gate result, the counted legs of each ID001_LINK_* case and the development install's release-and-advisory check (CN10). Both keep the file names revision 5 gives this row (docs/design/identity/briefs/IDENTITY-001.json:241-242).

**Acceptance:**
- docs/design/identity/PROVIDER-LINK-CONTRACT.md names every field the receiver reads and the states a link audit passes through, and the DIRECTORY-003 receiver's fixtures match it.
- docs/design/identity/reports/IDENTITY-001-links.md names the pinned fork commit, its gate result and the count of exercised legs per identifier, and records the release-and-advisory check of the install.

**Files:**
- create: docs/design/identity/PROVIDER-LINK-CONTRACT.md
- create: docs/design/identity/reports/IDENTITY-001-links.md

**Checklist:**
- C17 — The provider-link contract and the links report are written, and the report names the gated fork commit the pin moves to.

**Stories:**
- S5 (Responsible person, Signs in and provisions agents under their own authority) — As a person signing in, I want Google and GitHub to resolve to the one me, so that adding a provider never splits me into two people or merges me with someone who shares my email.

## Boundaries

- Only the files listed in this brief's requirements change; a row that needs a file outside its wall stops and names it, and Waffles approves a brief revision before that file is edited (CN9).
- Development isolation: disposable test identities and test provider registrations only; no production tokens, real business sign-in or live Cambium participant migration (CN2).
- No credential, token or key value is written in code, tests, fixtures, logs or documents; tests use generated values that are never real credentials.
- Nothing open for Tom is decided: every decision the design's non-goals record as open stays open.
- A live demonstration to Tom is a verification step a person performs after the row lands, never an acceptance criterion and never claimed by a loop completion (CN5, CN6).
- No row is dispatched until Waffles has reviewed it.
- No fork file is edited by this brief: the fork's changes land through the fork-owned brief and its gate, and reach this repository only as a pin.
- No cherry-pick and no nightly base: rows 02 to 05 run on v0.36.2 (CN10).

## Verification

- From the repository root: cargo fmt --check; cargo clippy --all-targets --all-features -- -D warnings; cargo clippy --all-targets -- -D warnings; cargo test --workspace --all-features; cargo doc --no-deps --all-features; cargo doc --no-deps, all clean (CLAUDE.md, gates before any commit).
- From the repository root: python3 scripts/design/validate.py docs/design/directory and python3 scripts/design/check-coverage.py docs/design/directory exit 0.
- From the repository root, with submodules checked out: rg -n 'ID001_LINK_PAIR|ID001_LINK_REFUSAL|ID001_LINK_MIGRATION|ID001_LINK_AUDIT' vendor/rauthy/tests/identity_links finds each identifier in a test.
- ID001_LINK_LIVE, after the row lands (CN5, CN6): a person installs the exact gated fork and shows Tom two linked providers resolving to one person in Rauthy's own account page, signing in through both, then posts a separate install and showing receipt to Tom with the Melbourne pass time, fork ref, artifact hash and observed result. A venue test or screenshot alone does not replace the live demonstration, and DIRECTORY-005 stays blocked until the receipt is recorded.


---
type: brief
id: DIRECTORY-005
cluster: directory
title: Complete the standalone screen journey
---

# DIRECTORY-005: Complete the standalone screen journey

> **Cluster:** directory
> **Depends on:** DIRECTORY-004
> **Blocked by:** ID001_LINK_LIVE: Tom's live demonstration of row 03 (DIRECTORY-004), recorded by its posted install and showing receipt with the Melbourne pass time, fork ref, artifact hash and observed result; a loop completion never stands in for it (CN6), Waffles' review of this brief before it is dispatched (DIRECTORY-001 boundary: no row brief is dispatched until Waffles has reviewed it), The exact file manifest for surface/identity/, reviewed before this row starts (docs/design/identity/briefs/IDENTITY-001.json:28)
> **Design anchor:**
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-007 — The product starts an agent by giving its start command, never by running it — The product is not an execution engine. For the first release an agent's file gives the command that starts it on a chosen machine: the command carries the agent's identity and its handles, never a credential's value, and it is rendered from the agent's kept launch record. A started agent reports back, so the sessions screen shows what is running. A terminal inside the product, sandboxes, virtual machines and containers are later runtimes that plug in, and none is built into this product.
> - ADR-008 — An agent's file shows its lys certificate — An agent's file shows its lys certificate once one is issued: what it claims, who signed it, when it was issued and when it expires, with the signed receipts of the changes made to it. An agent registered before any key or proof of possession was supplied shows its certificate as not issued, never a placeholder.
> - ADR-010 — Every product shares one design and keeps its own accent; the identity product's is orange — The identity screens follow Aion's structure, typography, spacing and interaction, and Rauthy's client themes take the same colours, with no build dependency on Cambium or Aion. Each product keeps its own accent within the estate colour family: Cambium green, Aion blue and black, Argus light blue, Haematite mustard. The identity product's accent is orange (accent #D4975A, deep #A86B2E, wash #3D2A17 in the estate colour tokens), set apart from Manifold's copper. No product is silently made Aion-blue, and purple is not used.
> **Checklist:**
> - C18 — The standalone journey (sign in, the directory, a record, registering and editing an agent, linked accounts, signed history) works with Cambium and Manifold absent, each agent showing its responsible person (ID001_STANDALONE).
> - C19 — Expired login, unauthorized edit, provider collision, audit pending and backend outage each have a visible, actionable outcome and no optimistic completed state (ID001_SCREEN_REFUSAL).
> - C20 — The screens follow Aion's appearance with the identity product's own orange accent and no build dependency on Cambium or Aion (ADR-010).
> **Stories:**
> - S1 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want every grant my agent holds to trace back to me, so that withdrawing my authority stops everything derived from it.
> - S3 (Operator, Installs and runs the standalone identity product) — As the operator, I want to install the identity product's dependencies on one PostgreSQL database whose host I choose, and restart or restore it without losing anyone, so that the product stands alone without Cambium or Manifold.
> - S6 (Operator, Installs and runs the standalone identity product) — As the operator, I want the directory's screens to show every refusal, pending audit and outage as it is, so that I never act on a completed state that did not happen.

## Purpose

Row 05 of IDENTITY-001 revision 5: the standalone product's own screens, 'beyond just a login page', so an operator installs, signs in, links two providers, registers an agent and reads its signed history with Cambium and Manifold absent. Revised for the grant ruling (ADR-003): each agent shows the person responsible for it, and the screen does not present access or delegation it cannot yet grant.

## Task

Carry IDENTITY-001 revision 5 row 05 (docs/design/identity/briefs/IDENTITY-001.json:263-294) into requirements: the screen journey (R1), visible refusals (R2) and the appearance (R3). Estimate: 6 focused implementer hours, revision 5's figure for this row (docs/design/identity/briefs/IDENTITY-001.json:293); no re-estimate. surface/identity/ is a wholly new module: its exact file manifest is reviewed before the row starts (CN9). The row's live demonstration, ID001_DIRECTORY_LIVE, is a verification step after the row lands and is the design's hold point before any row 06 (CN6). Every path is relative to the repository root.

## Requirements

### R1: Build the standalone screen journey

THE SYSTEM SHALL build sign-in, the directory of people and agents, record detail, register and edit agent, the linked-account entry point and signed change history in surface/identity/, served by crates/lys-identity-server/src/routes.rs and crates/lys-identity-server/src/assets.rs, with boundary types generated from the server schema, preserving shared lifecycle extension points without implementing Archie's screens (docs/design/identity/briefs/IDENTITY-001.json:280). Registration states what has been created. Revised for the grant ruling (docs/design/identity/STATEMENT-2026-09-22.md:21, Chippy 17:17, agreed): each agent's record shows its responsible person and its recorded lifecycle state (ADR-011, proposed); where the statement's first screen shows the access granted and the delegation rights beside the responsible person, step 1 shows them as not yet available, never as empty working controls. An agent's certificate shows as not issued, never a placeholder (ADR-008). Controls for launch, a start command (ADR-007), live permissions, secrets and memory are not presented as working in step 1 (docs/design/identity/briefs/IDENTITY-001.json:282). deploy/identity/README.md gains the screen's install and use, and docs/design/identity/reports/IDENTITY-001-standalone.md records the row's evidence.

**Acceptance:**
- ID001_STANDALONE: with Cambium and Manifold absent, an operator installs, signs in, links two providers, creates an agent, returns after restart and finds the same records and inspectable change history.
- Use the native browser against the installed venue artifact; record screenshots and observed actions plus artifact hashes in docs/design/identity/reports/IDENTITY-001-standalone.md. A mockup or frontend build alone is not this acceptance.
- Each agent's record shows its responsible person and its recorded lifecycle state; its access and delegation rights show as not available in step 1, never as empty working controls; its certificate shows as not issued; no control for launch, permissions, secrets or memory is presented as working.

**Files:**
- create: surface/identity/
- create: crates/lys-identity-server/src/assets.rs
- create: crates/lys-identity-server/src/routes.rs
- create: docs/design/identity/reports/IDENTITY-001-standalone.md
- modify: deploy/identity/README.md

**Checklist:**
- C18 — The standalone journey (sign in, the directory, a record, registering and editing an agent, linked accounts, signed history) works with Cambium and Manifold absent, each agent showing its responsible person (ID001_STANDALONE).

**Stories:**
- S1 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want every grant my agent holds to trace back to me, so that withdrawing my authority stops everything derived from it.
- S3 (Operator, Installs and runs the standalone identity product) — As the operator, I want to install the identity product's dependencies on one PostgreSQL database whose host I choose, and restart or restore it without losing anyone, so that the product stands alone without Cambium or Manifold.

### R2: Show every refusal, pending audit and outage as it is

THE SYSTEM SHALL show provider-link audit pending, unavailable services and named refusals accurately, with an actionable visible outcome for each and no optimistic completed state after a refusal (docs/design/identity/briefs/IDENTITY-001.json:281, docs/design/identity/briefs/IDENTITY-001.json:286; P5).

**Acceptance:**
- ID001_SCREEN_REFUSAL: expired login, unauthorized edit, provider collision, audit-pending and backend outage have actionable visible outcomes; no optimistic completed state after refusal.

**Files:**
- create: surface/identity/

**Checklist:**
- C19 — Expired login, unauthorized edit, provider collision, audit pending and backend outage each have a visible, actionable outcome and no optimistic completed state (ID001_SCREEN_REFUSAL).

**Stories:**
- S6 (Operator, Installs and runs the standalone identity product) — As the operator, I want the directory's screens to show every refusal, pending audit and outage as it is, so that I never act on a completed state that did not happen.

### R3: Follow Aion's appearance with the identity product's own accent

THE SYSTEM SHALL follow Aion's structure, DM Sans and JetBrains Mono typography, radii, spacing and browser interaction conventions, with the identity product's own orange accent instead of Aion-blue (ADR-010; docs/design/identity/briefs/IDENTITY-001.json:281). Neutral design values keep their pinned provenance and the accent values are separately declared, read from the estate colour tokens (docs/design-system-v2/palette/estate-colour-tokens.json in the ablative docs repository, owner Waffles; named, not listed in files, CN4), with no build dependency on Cambium or Aion. Central design-system extraction is outside this brief.

**Acceptance:**
- The screens' accent values are the identity orange of the estate tokens with their recorded source ref; no Aion-blue accent and no purple appear, and the frontend build names no Cambium or Aion package.

**Files:**
- create: surface/identity/

**Checklist:**
- C20 — The screens follow Aion's appearance with the identity product's own orange accent and no build dependency on Cambium or Aion (ADR-010).

**Stories:**
- S3 (Operator, Installs and runs the standalone identity product) — As the operator, I want to install the identity product's dependencies on one PostgreSQL database whose host I choose, and restart or restore it without losing anyone, so that the product stands alone without Cambium or Manifold.

## Boundaries

- Only the files listed in this brief's requirements change; a row that needs a file outside its wall stops and names it, and Waffles approves a brief revision before that file is edited (CN9).
- Development isolation: disposable test identities and test provider registrations only; no production tokens, real business sign-in or live Cambium participant migration (CN2).
- No credential, token or key value is written in code, tests, fixtures, logs or documents; tests use generated values that are never real credentials.
- Nothing open for Tom is decided: every decision the design's non-goals record as open stays open.
- A live demonstration to Tom is a verification step a person performs after the row lands, never an acceptance criterion and never claimed by a loop completion (CN5, CN6).
- No row is dispatched until Waffles has reviewed it.
- Archie's lifecycle screens are not implemented; their extension points are kept.
- No grant, launch, secret or memory control is presented as working in step 1.

## Verification

- From the repository root: cargo fmt --check; cargo clippy --all-targets --all-features -- -D warnings; cargo clippy --all-targets -- -D warnings; cargo test --workspace --all-features; cargo doc --no-deps --all-features; cargo doc --no-deps, all clean (CLAUDE.md, gates before any commit).
- From the repository root: python3 scripts/design/validate.py docs/design/directory and python3 scripts/design/check-coverage.py docs/design/directory exit 0.
- From the repository root: rg -n 'ID001_STANDALONE|ID001_SCREEN_REFUSAL' surface/identity finds each identifier in a test.
- ID001_DIRECTORY_LIVE, after the row lands (CN5, CN6): a person installs the gated standalone directory and completes its journey with Tom in a live browser, then posts a separate line to Tom with the Melbourne pass time, installed refs and hashes and what was demonstrated, before any row 06 starts; no deferral to row 07. This is the design's hold point: the brief that follows is blocked by it until Tom's receipt is recorded, and the choice of Rauthy base recorded in the design's non-goals falls due at this showing if no suitable upstream release exists (CN10).
- Frontend checks pass with strict types and the generated API schema, with negative and crash-boundary tests.


---
type: brief
id: DIRECTORY-006
cluster: directory
title: Enforce human-rooted grants, explicit delegation and the You-page access journey
---

# DIRECTORY-006: Enforce human-rooted grants, explicit delegation and the You-page access journey

> **Cluster:** directory
> **Depends on:** DIRECTORY-002, DIRECTORY-003
> **Blocked by:** Waffles review of every requirement and the accepted mock-up/conformance snapshot before dispatch., DIRECTORY-002 and DIRECTORY-003 are published foundation briefs, not implemented foundations. DIRECTORY-003 must establish the identity crates, reviewed event envelope and server integration manifest; reconcile every future modify path below against its landed implementation before dispatch. The listed new grant modules do not exist in current source., Surface integration waits for DIRECTORY-005 to establish surface/identity and its route/type-generation paths. R6 cannot dispatch before that dependency; DIRECTORY-005 must not depend on R6, so backend grant work and screen integration have separate readiness., Reviewed grant representation, root-authority bootstrap and SpiceDB consistency contract remain open in DIRECTORY-001C5 and ADR-003. Behavioural tests below constrain that design; they do not authorise freezing a cryptographic format., Lifecycle suspension semantics and role version policy are supplied by their reviewed ADRs. Reinstatement safety and expiry invariants here do not settle their remaining policy choices., Review of the proposed R1 grant schema and R4 freshness mechanism; their additional fields and consistency strategy are proposals, not existing settled policy.
> **Design anchor:**
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> - ADR-005 — The identity database is PostgreSQL, possibly on a network device — PostgreSQL is used for the identity product's database. It may be set up on one of the network devices rather than on Tom's Mac.
> **Checklist:**
> - C21 — The proposed application grant schema separates exercise and pass-on authority; its additional fields require independent review before dispatch and do not change published crypto formats.
> - C22 — Delegation requires an explicit permission to delegate to the requested subject kind; use-only, absent policy and owner labels cannot create it.
> - C23 — Every grant is bounded by a live, acyclic ancestry to a responsible person; scope is evaluated by model actions/resources, never display-name rank.
> - C24 — Grant changes and their audit share one durable authoritative event; retries and uncertain outcomes preserve operation identity and replay-equivalent projections.
> - C25 — Revocation stops derived authority while independent authority remains usable; the proposed revision-freshness mechanism is reviewed before dispatch.
> - C26 — Inherited expiry and provisional end dates are enforced at admission; role changes and reinstatement cannot resurrect expired or revoked authority.
> - C27 — Typed browser/API/agent seams use one authenticated evaluator; forward and reverse explanations name the same authority path and revision without private-record leakage.
> - C28 — Observed grant usage names its source and time; not seen is not reported as never used.
> - C29 — The You and delegation screens show real server authority, separate service accounts from sign-in identities, and preserve personal versus administrator visibility.
> - C30 — The accepted mock-up conformance is tested through actual requests, refusals, pending outcomes, keyboard paths and durable read-back rather than simulated success.
> **Stories:**
> - S8 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a responsible person, I want to give my agent a bounded part of my authority and revoke it, so the same action is allowed before revocation and refused after it.
> - S9 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a person or agent with use-only permission, I want the service to explain why I cannot pass it on; with explicit pass-on rights I want the same permitted operation available through UI and tools.
> - S10 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As an auditor, I want grant changes and retries tied to one durable signed operation, so restart or an uncertain reply cannot invent, lose or duplicate authority.
> - S11 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a person granting temporary access, I want its end date and ancestor restrictions enforced, so role changes or reinstatement cannot silently extend it.
> - S12 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As an authorised reviewer, I want to ask why this identity can act and who can reach a resource using the same current decision, without exposing records I may not inspect.

## Purpose

Make the human-rooted grant path enforceable: sign in, give an agent a bounded permission, prove the action works, revoke the source, and prove the same action is refused. Use and lending are separate positive rights and the screen explains the real server decision. Authority for this amendment: docs/design/identity/CONFORMANCE.md at commit 1353c22, rows 1.3, 1.4, 2.1–2.6 and 7.3. Context only: Tom requested complete briefs tied to the mock-up on 23 September 2026 at 19:04:22 Melbourne, then the right Cambium boards and workflows at 19:10:35; these conversational times are not repository evidence.

## Task

Implement R1–R5 in order after the directory/service foundations, then R6 after the standalone surface foundation. Estimated work: R1 2h, R2 3h (ancestry/refusal matrix), R3 4h (forced durability boundaries), R4 3h (freshness and expiry), R5 2h, R6 3h (two-person browser evidence), total 17 hours as a new increment, not silently inside IDENTITY-001's 48-hour ceiling. Estimates are proposals for review. Exact file walls follow the new crate roots already named by IDENTITY-001 rows 04/05; none is asserted implemented. Every future modify path must be reconciled to those dependencies' actual manifests before dispatch. Preserve all ID001 requirements in their owning predecessor briefs and the frozen IDENTITY-001 history. In scope: domain grants, authority enforcement, durable audit, explanation API and You/delegation UI. Out: provider federation changes, secret-value storage/proxy, actual process execution, assistant actions, graph renderer and published cryptographic format changes.

## Requirements

### R1: Define the reviewed grant contract without changing published cryptography

PROPOSAL FOR REVIEW, not a settled grant schema: define a typed application grant contract that identifies the grant, issuer, holder, responsible person, resource, exercisable actions, explicit pass-on authority and permitted recipient kinds, source grant, time window, policy revision and authorising audit event. Exercising an action and delegating it SHALL be separate decisions. A missing pass-on field SHALL NOT mean permission. Relations SHALL resolve through the reviewed model to action/resource sets; the code SHALL NOT infer a global owner/editor/viewer rank from display strings. Before any durable grant is signed, the event envelope, canonical encoding, root-authority bootstrap and lineage semantics SHALL be independently reviewed. This row defines application-domain types in lys-identity; it SHALL NOT reinterpret seat/speaks-for or mutate lys/delegation/v1, which currently supplies neither this capability schema nor an expiry. Provenance: ADR-003; STATEMENT-2026-09-22.md, Everything is pegged to a human authority; PROVISIONING-2026-09-22.md, What one grant says. Conformance 1.3, 1.4, 2.2, 2.3. The extra holder, responsible-person, recipient-kind, time-window, policy-revision and authorising-event fields are design proposals in this row; PROVISIONING-2026-09-22.md supplies five source facts and leaves representation open. This row and GRANT_CONTRACT cannot be dispatched until independent contract review ratifies the schema. Checkable conformance source: docs/design/identity/CONFORMANCE.md at commit 1353c22.

**Acceptance:**
- GRANT_CONTRACT: serialise/parse a grant with each declared member and independently supplied schema fixture; unknown members, absent required authority, malformed lineage and unknown recipient kinds refuse by name. Count each case; no permission is supplied by a deserialisation default.
- GRANT_MODEL: define two model relations whose names do not imply their action sets. A requested subset is checked against the model, not lexical order or a hard-coded rank; model version is retained in the decision.
- GRANT_WIRE_BOUNDARY: an explicit reviewed application-envelope identifier precedes the first durable signed grant. Existing published lys wire-vector tests remain byte-identical; an unratified delegation format is not used as a substitute capability token.

**Files:**
- create: crates/lys-identity/src/grants/mod.rs
- create: crates/lys-identity/src/grants/types.rs
- create: crates/lys-identity/src/grants/error.rs
- create: crates/lys-identity/tests/grant_contract.rs
- create: docs/design/identity/GRANT-CONTRACT.md
- modify: crates/lys-identity/src/lib.rs

**Checklist:**
- C21 — The proposed application grant schema separates exercise and pass-on authority; its additional fields require independent review before dispatch and do not change published crypto formats.

**Stories:**
- S8 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a responsible person, I want to give my agent a bounded part of my authority and revoke it, so the same action is allowed before revocation and refused after it.

### R2: Enforce affirmative delegation and bounded ancestry

WHEN a person or agent asks to delegate authority, THE SYSTEM SHALL check the actor's current exercise and delegation rights, requested resource/actions and recipient kind against every effective ancestor, ending at an authorised person. The requested authority SHALL be an explicit subset, never inherited wholesale. A use-only grant, an owner label, missing prohibition or access to the UI SHALL NOT establish pass-on authority. People-only authority SHALL refuse agents even when the person owns that agent. Unknown parents, cycles, wrong responsible persons and attempts to launder authority through a new role, secret handle or sibling grant SHALL refuse before any mutation. An agent that holds explicit pass-on permission may invoke the same operation as a person; being an agent alone is not either a permit or a categorical prohibition. The policy determines recipients rather than the mock-up's sample restriction to the caller's own agents. Provenance: ADR-003; AGENT-PARITY-2026-09-23.md. Conformance 2.1–2.4, 7.3. Secret lending also recognises server-verified real ownership as the affirmative route in docs/design/identity/CONFORMANCE.md at commit 1353c22 row 7.3; a display label is not that ownership record.

**Acceptance:**
- GRANT_USE_VS_LEND: Tom may read a project through Dana's use-only grant. His read succeeds; browser, direct API and agent-tool attempts to grant that access to his agent all refuse and create zero grant events. Add a distinct affirmative pass-on grant: the bounded control now succeeds.
- GRANT_RECIPIENT: exercise human-only, agent-only and non-delegable grants with both recipient kinds. Assert each named permit/refusal and exact zero mutations on denied requests, including the owner's own agent case.
- GRANT_ANCESTRY: attempt a broader resource, broader action, a forged parent, a cycle, a changed responsible person and a revoked ancestor. Each refuses with the blocking boundary. A two-hop permitted narrower chain succeeds and explains its actual ancestors.
- GRANT_AGENT_PARITY: the same authenticated authority and request produce the same effective decision through browser/API/tool routes. An agent granted pass-on can delegate inside that grant; an agent without it cannot.

**Files:**
- create: crates/lys-identity/src/grants/admission.rs
- create: crates/lys-identity/src/grants/lineage.rs
- create: crates/lys-identity/src/grants/authority.rs
- create: crates/lys-identity/tests/grant_delegation.rs
- modify: crates/lys-identity/src/grants/mod.rs

**Checklist:**
- C22 — Delegation requires an explicit permission to delegate to the requested subject kind; use-only, absent policy and owner labels cannot create it.
- C23 — Every grant is bounded by a live, acyclic ancestry to a responsible person; scope is evaluated by model actions/resources, never display-name rank.

**Stories:**
- S8 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a responsible person, I want to give my agent a bounded part of my authority and revoke it, so the same action is allowed before revocation and refused after it.
- S9 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a person or agent with use-only permission, I want the service to explain why I cannot pass it on; with explicit pass-on rights I want the same permitted operation available through UI and tools.

### R3: Commit grants and their audit as one replayable operation

THE SYSTEM SHALL record a grant mutation as one signed authoritative event using DIRECTORY-003's reviewed event owner, and derive both the directory projection and the permission projection from that event. A database write followed by best-effort audit is forbidden. Each mutation SHALL carry a stable operation ID; the same ID and payload returns the same logical outcome, while changed payload under that ID refuses. Unknown append or projection outcomes SHALL remain named and retained for reconciliation; the API SHALL NOT manufacture success or issue a fresh operation when acknowledgement is lost. Any read or admission that depends on unresolved authority SHALL refuse naming the affected grant/operation; unrelated authority stays usable. Projection progress SHALL record the minimum revision required for a fresh decision, including across restart. Provenance: directory principlesP4/P5 and ID001_AUDIT_FAULTS; conformance1.4,8.1.

**Acceptance:**
- GRANT_DURABILITY: enumerate every append, sync, authoritative-event and projection boundary in the implementation and inject a failure at each. Count the legs. Every answered grant/permission read equals replay, while unresolved reads name the grant/operation. Unrelated grant controls still succeed.
- GRANT_IDEMPOTENCE: lose acknowledgement after commit, retry identical ID/payload, reopen and retry again. One logical grant and one authorising event remain. A changed payload under the same ID refuses and changes neither projection.
- GRANT_AUDIT: independently verify a grant mutation receipt; tampering with actor, source grant, recipient, resource/actions, sequence or signature is rejected. A refusal never appears as a successful grant event.

**Files:**
- create: crates/lys-identity/src/grants/events.rs
- create: crates/lys-identity/src/grants/projection.rs
- create: crates/lys-identity/src/grants/recovery.rs
- create: crates/lys-identity/tests/grant_faults.rs
- create: crates/lys-identity/tests/grant_receipts.rs
- modify: crates/lys-identity/src/grants/mod.rs

**Checklist:**
- C24 — Grant changes and their audit share one durable authoritative event; retries and uncertain outcomes preserve operation identity and replay-equivalent projections.

**Stories:**
- S10 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As an auditor, I want grant changes and retries tied to one durable signed operation, so restart or an uncertain reply cannot invent, lose or duplicate authority.

### R4: Enforce revocation, inherited expiry and current permission decisions

WHEN an ancestor is revoked or expires, THE SYSTEM SHALL refuse fresh exercise and further delegation through every grant derived from it. Independent grants to the same person or resource SHALL remain independent. Proposal under review: require a permission decision at least as fresh as the authoritative change, and refuse or await freshness rather than permit from a stale replica, unavailable engine or unresolved projection. The mechanism and choice must be settled before dispatch. Expiry SHALL be enforced at admission using the named clock, including inherited limits; a role edit or version move SHALL NOT renew a provisional grant. Reinstating a suspended identity SHALL recheck current ancestry and leases, never resurrect separately revoked or expired grants. Shared ancestor use/spend budgets are counted once by the broker in SECRETS-002 R9, never cloned into each descendant. The exact broader suspension policy remains the lifecycle ADR's, not a decision smuggled into this row. Conformance 2.5, 2.6, 3.3, 4.5.

**Acceptance:**
- GRANT_REVOKE: create a two-hop chain and an independently authorised sibling. A permitted action succeeds, then revoke the chain root and repeat exactly the same action for each descendant: every derived call refuses; the independent control succeeds.
- GRANT_EXPIRY: under a controlled clock, assert permit immediately before each relevant end boundary and refusal at/after it, including an earlier ancestor end. Neither editing a role nor selecting a newer role version extends the holding.
- GRANT_FRESHNESS: pause permission projection before applying a committed revoke; a caller presenting the required revision cannot obtain a permit from the old state. Resume projection and verify the named revoked decision after reopen.
- GRANT_REINSTATE: while an identity is suspended, revoke one grant and expire a second. After reinstatement both remain refused; only a third still-authorised control is usable. No renewal or issuance event is invented.

**Files:**
- create: crates/lys-identity/src/grants/revocation.rs
- create: crates/lys-identity/src/grants/expiry.rs
- create: crates/lys-identity/src/grants/permission.rs
- create: crates/lys-identity/tests/grant_revocation.rs
- create: crates/lys-identity/tests/grant_expiry.rs
- modify: crates/lys-identity/src/grants/mod.rs

**Checklist:**
- C25 — Revocation stops derived authority while independent authority remains usable; the proposed revision-freshness mechanism is reviewed before dispatch.
- C26 — Inherited expiry and provisional end dates are enforced at admission; role changes and reinstatement cannot resurrect expired or revoked authority.

**Stories:**
- S8 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a responsible person, I want to give my agent a bounded part of my authority and revoke it, so the same action is allowed before revocation and refused after it.
- S11 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a person granting temporary access, I want its end date and ancestor restrictions enforced, so role changes or reinstatement cannot silently extend it.

### R5: Expose one authenticated grant and explanation seam

THE SYSTEM SHALL expose typed operations for list/read, delegate, revoke and explain through the standalone identity server, all calling the same authority owner. API/tool access SHALL enforce the same checks as the UI; browser controls are presentation only. A why-permitted response SHALL identify actual authority path, responsible person, effective scope and policy revision; a why-refused response SHALL name the blocking condition without disclosing another identity's protected records. The reverse question, who can exercise this action on this resource, SHALL use the same evaluator and revision, with its own visibility permission. Incomplete pagination SHALL remain explicit. A last-use value reports observed use at named enforcement points; absence is not evidence of never-used. No live Cambium, Aion, Argus or Manifold is required. Conformance 8.1, 8.2, 8.4 and ADR-004.

**Acceptance:**
- GRANT_API_AUTH: unauthenticated, wrong issuer/subject, use-only and unauthorised-revoke requests are named refusals and create zero mutations. Exercise valid controls and count each route, rather than testing only button visibility.
- GRANT_EXPLAIN: the forward decision and reverse enumeration agree for the same resource/action/revision; paging returns every authorised holder exactly once and names continuation. Hidden private grants do not leak IDs, labels or existence to an unauthorised querier.
- GRANT_LAST_USED: an unobserved grant is labelled not seen; a recorded use reports its source/time; a missing reporting source is distinguished from a zero count. Reopen preserves the attribution.
- GRANT_STANDALONE: execute all operations against disposable local dependencies with other Ablative servers absent. Permission-engine outage is a named refusal, never an implicit standalone permit.

**Files:**
- create: crates/lys-identity-server/src/grants.rs
- create: crates/lys-identity-server/src/grant_contract.rs
- create: crates/lys-identity-server/tests/grants.rs
- create: crates/lys-identity-server/tests/grant_explanations.rs
- modify: crates/lys-identity-server/src/routes.rs

**Checklist:**
- C27 — Typed browser/API/agent seams use one authenticated evaluator; forward and reverse explanations name the same authority path and revision without private-record leakage.
- C28 — Observed grant usage names its source and time; not seen is not reported as never used.

**Stories:**
- S9 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a person or agent with use-only permission, I want the service to explain why I cannot pass it on; with explicit pass-on rights I want the same permitted operation available through UI and tools.
- S12 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As an authorised reviewer, I want to ask why this identity can act and who can reach a resource using the same current decision, without exposing records I may not inspect.

### R6: Implement the You and delegation screens from the server contract

THE SYSTEM SHALL render the accepted mock-up's You page and delegation form from the server's generated grant types and decisions. Show sign-in identities separately from service access, source grants, effective operations/resource, affirmative pass-on rights and inherited end boundary. The form SHALL explain refused choices the caller is allowed to discover; it SHALL NOT enumerate other people's private secrets merely to say no. Personal views are scoped to the signed-in person, while an independently authorised directory administrator may inspect the wider directory. A failed or uncertain mutation SHALL remain refused or pending, with the original operation ID, never an optimistic grant or a new retry. The access graph consumes the same explanation response; its rendering belongs to Archie's graph brief. No simulated confirmation timer or sample-data authority calculation ships. Conformance 1.3–1.5, 2.3–2.4, 8.1 and 8.4.

**Acceptance:**
- GRANT_SCREEN: using two test people and their agents, render the source grant, action/resource scope, pass-on decision and inherited expiry from fixture IDs. Switching the signed-in person changes the personal data; permitted administrator inspection remains possible through its separate route.
- GRANT_SCREEN_REFUSAL: a use-only source, excessive requested scope, people-only policy, expired ancestor and service outage are refused visibly with the server reason; the request count and record count prove no hidden mutation happened.
- GRANT_SCREEN_PENDING: withhold the acknowledgement after durable delegation. The page keeps the original operation pending, retries by that ID, and eventually shows one grant; it never changes to success on a timer.
- GRANT_CONFORMANCE: pin the accepted mock-up file hash and numbered conformance rows in the test evidence. Exercise keyboard operation and deep linking as well as API refusal parity; sample data and visual similarity alone are not a passing acceptance.

**Files:**
- create: surface/identity/src/features/grants/YouGrants.tsx
- create: surface/identity/src/features/grants/DelegateGrant.tsx
- create: surface/identity/src/features/grants/GrantExplanation.tsx
- create: surface/identity/tests/grants.test.tsx
- create: surface/identity/tests/acceptance/grants.spec.ts
- modify: surface/identity/src/routes.tsx
- modify: surface/identity/src/generated/index.ts

**Checklist:**
- C29 — The You and delegation screens show real server authority, separate service accounts from sign-in identities, and preserve personal versus administrator visibility.
- C30 — The accepted mock-up conformance is tested through actual requests, refusals, pending outcomes, keyboard paths and durable read-back rather than simulated success.

**Stories:**
- S8 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a responsible person, I want to give my agent a bounded part of my authority and revoke it, so the same action is allowed before revocation and refused after it.
- S9 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a person or agent with use-only permission, I want the service to explain why I cannot pass it on; with explicit pass-on rights I want the same permitted operation available through UI and tools.
- S12 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As an authorised reviewer, I want to ask why this identity can act and who can reach a resource using the same current decision, without exposing records I may not inspect.

## Boundaries

- A missing file or unreviewed integration seam is a named dispatch blocker; do not silently expand a row wall or label an empty integration ready.
- No real credentials, live operator account mutation or production service writes in tests; use disposable local providers and permission fixtures.
- Every mutation seam is typed and generated; UI and agent callers use the same server checks. Do not infer authority from email, display name, mock-up data or absence of a deny flag.
- The same grant cannot acquire more authority through delegation, a secret handle, role migration, retries or restart. Use, delegation and permission to inspect another identity are distinct.
- The source/mock-up is a conformance specification, not implementation evidence. A test or source-only review never substitutes for the required live demonstration.
- No code in lys-core or published wire format changes in this brief. A capability format decision requires its own independent adversarial review before durable signing.
- Waffles owns board placement and workflow connection review. Create a blocked card with this exact brief path; do not move it into a dispatching status until its named blockers and row review are resolved.

## Verification

- From docs/: python3 "$DS2_METHOD/scripts/validate.py" design/directory and python3 "$DS2_METHOD/scripts/check-coverage.py" design/directory. Render with render-cluster.py and verify a second render is byte-identical.
- At implementation time run the repository battery from the exact revision: fmt, strict default/all-feature Clippy, feature-full tests and both rustdoc shapes. Run the granted/refused matrix and report each exercised leg, fault ordinal and unrun check.
- After the relevant screen row lands, demonstrate sign-in, agent registration, permitted action, source revocation and refusal to Tom using installed artifact hashes; record Melbourne time and exact authority/operation/receipt IDs. This human verification is not an automated acceptance criterion.
- Review the Cambium card: Card key resolves this exact JSON brief; project board rule connects to the supported deployed workflow; blocker/status prevents premature dispatch. No live test run or duplicate submission is used merely to inspect the configuration.


---
type: brief
id: DIRECTORY-008
cluster: directory
title: Grant brief residue after PR 6
---

# DIRECTORY-008: Grant brief residue after PR 6

> **Cluster:** directory
> **Depends on:** DIRECTORY-001
> **Blocked by:** Sign-off of this brief before it is dispatched (DIRECTORY-001 boundary: no row brief is dispatched until Waffles has reviewed it)
> **Design anchor:**
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> - ADR-009 — People sign in through a maintained Rauthy fork of our own — Rauthy authenticates people, and its one-provider-per-user limit is changed in a fork we maintain, ablative-io/rauthy, not contributed upstream as a prerequisite. The maintained branch is ablative, created from upstream v0.36.2 commit dd61ac3c84d6b238108dc8438b53043b5177a662; the fork's main stays an untouched upstream mirror; lys pins an exact commit of ablative as the submodule vendor/rauthy. Upgrades rebase ablative onto upstream release tags only, each in its own gated row; no cherry-picks and no reset of main.
> **Checklist:**
> - C1 — The directory design records the outcome, the shared contract as principles, the constraints and the non-goals of IDENTITY-001, revised for the grant ruling.
> - C3 — Each open row (02, 04, 03, 05) is a design-system brief, DIRECTORY-002 to DIRECTORY-005, in dependency order, with its wall as files, its ID001 acceptance identifiers kept, and its estimate in its task.
> - C6 — The rendered markdown of this cluster matches its JSON and coverage is clean.
> **Stories:**
> - S2 (Reviewer, Reviews a brief before any of its rows is dispatched) — As the reviewer, I want each open identity row as a design-system brief with numbered requirements and criteria, so that rows can be dispatched to the loop one at a time and reviewed against their criteria.

## Purpose

Three small items the 25 September grant-brief patch carries that main lacks after PR 6 (5593354) landed DIRECTORY-001 to DIRECTORY-006: the DIRECTORY-005 verification line for the frontend checks (R1), three design.json inventory rows for vendor/rauthy, crates/lys and docs/design/decisions.json beside the eight rows main lists (R2), and one design.json intention sentence recording that Cambium later uses this issuer keeping its participant ids, while the non-goal for rows 06 and 07 stands (R3). Main wins wherever the patch contradicts it on ADR ids and lifecycle; nothing here reopens a settled row. Documents only under docs/design/directory; no code; no new ADR; no change to any row's identifiers, estimates or dependency order.

## Task

Make the three document edits R1 to R3 on the documents as main holds them at 1756688cc08169bef4a9ac37b9b079efb9e38b18 (docs/design/directory/briefs/DIRECTORY-005.json:114-119, docs/design/directory/design.json:4 and docs/design/directory/design.json:580-613), re-render the cluster so each rendered twin matches its JSON, and pass the gate. Estimate: 1 focused implementer hour (R1 0.25, R2 0.5, R3 0.25), a document increment outside CN7's 48-hour ceiling, which stands unchanged; no row's estimate changes, DIRECTORY-005 keeps its 6 hours. Every path is relative to the repository root (CN3).

Ledger. Under the ruling of 26 September at 20:31 that a brief's ids are the next after main's highest and every open brief branch's, this brief is DIRECTORY-008 at docs/design/directory/briefs/DIRECTORY-008.json and its roadmap row is RM-016 in docs/design/roadmap.json, appended after RM-015, which the audit receipt card's brief DIRECTORY-007 takes in the run fired beside this one; main's ledger ends at RM-010 (docs/design/roadmap.json:276) and RM-011 to RM-015 are taken on open brief branches, so RM-016 is appended as the next row with every row main holds unchanged. It records no new decision: docs/design/decisions.json is not touched. Per the settled answer B1 (round 1 of run a68a7767, 22:49): RM-016 is the brief's own ledger entry, which every brief on main has, written by the brief method the gate runs, as the brief file is written under docs/design/directory/briefs; the boundary 'documents only under docs/design/directory' governs the work R1 to R3 do when built, not the brief's own files; RM-016 is appended with every row main holds unchanged, and no other file outside docs/design/directory changes. The two structure rows for this brief's own files in design.json are the method's listing of the brief, as DIRECTORY-006's rows are (docs/design/directory/design.json:414-423), and are written on the brief branch, not by the build.

Coverage. C1, C3, C6 and S2 are claimed by DIRECTORY-001, which wrote these documents, and by this brief, which completes them with the three items; the split is noted here, as check-coverage.py allows, and is reported as a warning, not a failure.

Dev note. The build starts from the main this brief lands on, which may be later than 1756688 if DIRECTORY-007 lands first; the counts in R1 to R3 are stated against 1756688 and hold, because that brief's words touch neither DIRECTORY-005's verification nor design.json's inventory or intention. Read the four files first. Make each JSON edit as an append that keeps every existing member byte for byte: json.load, append, json.dump with indent=2 and ensure_ascii=False plus one trailing newline reproduces main's formatting exactly (checked at 1756688 for both files by a round trip). Then run python3 scripts/design/render-cluster.py docs/design/directory and commit the rendered DESIGN.md and DIRECTORY-005.md beside their JSON; the gate compares a fresh render against the committed markdown byte for byte. Run bash scripts/design/gate.sh last and commit the four files by exact path.

## Requirements

### R1: Add the frontend verification line to DIRECTORY-005

THE SYSTEM SHALL append to the verification array of docs/design/directory/briefs/DIRECTORY-005.json (docs/design/directory/briefs/DIRECTORY-005.json:114-119, four entries at 1756688cc08169bef4a9ac37b9b079efb9e38b18, the fourth being the ID001_DIRECTORY_LIVE hold point) one fifth and last entry, exactly: 'Frontend checks pass with strict types and the generated API schema, with negative and crash-boundary tests.' The four entries main holds SHALL stay byte for byte and in their order. No other member of DIRECTORY-005 changes: not its id, depends_on, blocked_by, checklist, stories, design_anchor, purpose, task with its 6-hour estimate, requirements or boundaries. THE SYSTEM SHALL re-render docs/design/directory/briefs/DIRECTORY-005.md with python3 scripts/design/render-cluster.py docs/design/directory so its Verification section (docs/design/directory/briefs/DIRECTORY-005.md:103-108 at 1756688cc08169bef4a9ac37b9b079efb9e38b18) lists the five entries.

**Acceptance:**
- From the repository root: python3 -c "import json; print(len(json.load(open('docs/design/directory/briefs/DIRECTORY-005.json'))['verification']))" prints 5 (it prints 4 at 1756688cc08169bef4a9ac37b9b079efb9e38b18), and the first four entries equal, string for string, the four that git show 1756688cc08169bef4a9ac37b9b079efb9e38b18:docs/design/directory/briefs/DIRECTORY-005.json holds.
- From the repository root: rg -c 'strict types and the generated API schema' docs/design/directory/briefs/DIRECTORY-005.json prints 1, and the same command over docs/design/directory/briefs/DIRECTORY-005.md prints 1; both print nothing at 1756688cc08169bef4a9ac37b9b079efb9e38b18.
- From the repository root, on the build branch: git diff --no-ext-diff --numstat <base>..HEAD -- docs/design/directory/briefs/DIRECTORY-005.json prints 2 added and 1 deleted (the new entry's line and the fourth entry's line gaining its trailing comma), where <base> is the commit the build started from.
- From the repository root: python3 -c "import json; d=json.load(open('docs/design/directory/briefs/DIRECTORY-005.json')); print(d['task'].count('6 focused implementer hours'), d['depends_on'])" prints 1 ['DIRECTORY-004'], as at 1756688cc08169bef4a9ac37b9b079efb9e38b18.

**Files:**
- modify: docs/design/directory/briefs/DIRECTORY-005.json
- modify: docs/design/directory/briefs/DIRECTORY-005.md

**Checklist:**
- C3 — Each open row (02, 04, 03, 05) is a design-system brief, DIRECTORY-002 to DIRECTORY-005, in dependency order, with its wall as files, its ID001 acceptance identifiers kept, and its estimate in its task.
- C6 — The rendered markdown of this cluster matches its JSON and coverage is clean.

**Stories:**
- S2 (Reviewer, Reviews a brief before any of its rows is dispatched) — As the reviewer, I want each open identity row as a design-system brief with numbered requirements and criteria, so that rows can be dispatched to the loop one at a time and reviewed against their criteria.

#### R1 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Accept 1: met. The verification array has 5 entries, and the first four were left untouched by the append (the dump round trip is byte-identical, checked before editing). Accept 2: met. rg -c prints 1 for docs/design/directory/briefs/DIRECTORY-005.json (line 119) and 1 for DIRECTORY-005.md (line 109). Accept 3: met. The working-tree git diff --numstat for DIRECTORY-005.json is '2 1': the new line plus the fourth entry gaining its comma. Accept 4: met. The check prints 1 ['DIRECTORY-004'].
- Deviation: I ran python3 scripts/design/render-cluster.py docs/design/directory once, because R1 to R3 require the re-render and it is the only way to produce the rendered twins. I ran no other listed check, build, lint or test, and made no commit, as instructed.
- Files changed:
  - modified: `docs/design/directory/briefs/DIRECTORY-005.json` — The verification array now holds five entries. The fifth, at line 119, is 'Frontend checks pass with strict types and the generated API schema, with negative and crash-boundary tests.' Every other member is unchanged.
  - modified: `docs/design/directory/briefs/DIRECTORY-005.md` — Re-rendered. The Verification section lists the five entries, with the new one at line 109.
- Checklist delivery:
  - [x] C3 — Each open row (02, 04, 03, 05) is a design-system brief, DIRECTORY-002 to DIRECTORY-005, in dependency order, with its wall as files, its ID001 acceptance identifiers kept, and its estimate in its task. — DIRECTORY-005 keeps its id, depends_on, estimate and ID001 identifiers. Only the verification line was added.
  - [x] C6 — The rendered markdown of this cluster matches its JSON and coverage is clean. — The rendered twin was regenerated from the JSON.
- Story delivery:
  - [x] S2 (Reviewer, Reviews a brief before any of its rows is dispatched) — As the reviewer, I want each open identity row as a design-system brief with numbered requirements and criteria, so that rows can be dispatched to the loop one at a time and reviewed against their criteria. — The brief now carries the frontend verification line for review.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] From the repository root: python3 -c "import json; print(len(json.load(open('docs/design/directory/briefs/DIRECTORY-005.json'))['verification']))" prints 5 (it prints 4 at 1756688cc08169bef4a9ac37b9b079efb9e38b18), and the first four entries equal, string for string, the four that git show 1756688cc08169bef4a9ac37b9b079efb9e38b18:docs/design/directory/briefs/DIRECTORY-005.json holds. — Measured: 4 at 1756688, 5 now. verification[:4] equals the base list (True). No other member differs from base (empty set).
  - [x] From the repository root: rg -c 'strict types and the generated API schema' docs/design/directory/briefs/DIRECTORY-005.json prints 1, and the same command over docs/design/directory/briefs/DIRECTORY-005.md prints 1; both print nothing at 1756688cc08169bef4a9ac37b9b079efb9e38b18. — rg -c prints 1 for DIRECTORY-005.json (line 119) and 1 for DIRECTORY-005.md (line 109). The phrase is absent at base.
  - [x] From the repository root, on the build branch: git diff --no-ext-diff --numstat <base>..HEAD -- docs/design/directory/briefs/DIRECTORY-005.json prints 2 added and 1 deleted (the new entry's line and the fourth entry's line gaining its trailing comma), where <base> is the commit the build started from. — git diff --no-ext-diff --numstat against base 3c1a3b9 (working tree, not yet committed) prints '2	1	docs/design/directory/briefs/DIRECTORY-005.json'.
  - [x] From the repository root: python3 -c "import json; d=json.load(open('docs/design/directory/briefs/DIRECTORY-005.json')); print(d['task'].count('6 focused implementer hours'), d['depends_on'])" prints 1 ['DIRECTORY-004'], as at 1756688cc08169bef4a9ac37b9b079efb9e38b18. — Prints 1 ['DIRECTORY-004'].
- Checklist verified: C3, C6
- Stories verified: S2

### R2: Add the three inventory rows beside what main lists

THE SYSTEM SHALL append three rows to the inventory array of docs/design/directory/design.json (docs/design/directory/design.json:580-613, eight rows at 1756688cc08169bef4a9ac37b9b079efb9e38b18, ending with docs/design/identity/RAUTHY-BASELINE.md at docs/design/directory/design.json:606 and docs/design/identity/CONFORMANCE.md at docs/design/directory/design.json:610), after the eight and in this order, each with its path and a note of what exists there at 1756688cc08169bef4a9ac37b9b079efb9e38b18: (1) path 'vendor/rauthy', note 'the maintained Rauthy fork (ADR-009), the git submodule pinned at dd61ac3c84d6b238108dc8438b53043b5177a662, the upstream v0.36.2 commit the ablative branch was created from; DIRECTORY-004 moves the pin (structure row); read here, never changed by a document row'; (2) path 'crates/lys', note 'the lys CLI crate: Cargo.toml, src/main.rs, src/cli.rs and src/commands/ (attest, ca, key, log, inspect, files); DIRECTORY-002 adds src/identity/ and the identity subcommand to it (structure rows)'; (3) path 'docs/design/decisions.json', note 'the project decision ledger, ADR-001 to ADR-018 at main, holding the decisions this cluster cites (ADR-003, ADR-004, ADR-005, ADR-007 to ADR-011); DIRECTORY-001 recorded that it gained the identity decisions (structure row); read here, never changed by a document row'. The eight rows main lists SHALL stay byte for byte and in their order. No structure row, principle, goal, non-goal, constraint or gate leg changes. THE SYSTEM SHALL re-render docs/design/directory/DESIGN.md with python3 scripts/design/render-cluster.py docs/design/directory so its Inventory section (docs/design/directory/DESIGN.md:167-176 at 1756688cc08169bef4a9ac37b9b079efb9e38b18) lists the eleven rows.

**Acceptance:**
- From the repository root: python3 -c "import json; print(len(json.load(open('docs/design/directory/design.json'))['inventory']))" prints 11 (it prints 8 at 1756688cc08169bef4a9ac37b9b079efb9e38b18), and the first eight rows equal, member for member, the eight that git show 1756688cc08169bef4a9ac37b9b079efb9e38b18:docs/design/directory/design.json holds.
- From the repository root: rg -c '"path": "vendor/rauthy"' docs/design/directory/design.json prints 2 (1 at 1756688cc08169bef4a9ac37b9b079efb9e38b18, the structure row at docs/design/directory/design.json:380); rg -c '"path": "crates/lys"' docs/design/directory/design.json prints 1 (nothing at 1756688cc08169bef4a9ac37b9b079efb9e38b18); rg -c '"path": "docs/design/decisions.json"' docs/design/directory/design.json prints 2 (1 at 1756688cc08169bef4a9ac37b9b079efb9e38b18, the structure row at docs/design/directory/design.json:140).
- From the repository root: rg -c '^- `vendor/rauthy`' docs/design/directory/DESIGN.md prints 1; rg -c '^- `crates/lys`' docs/design/directory/DESIGN.md prints 1; rg -c '^- `docs/design/decisions.json`' docs/design/directory/DESIGN.md prints 1; each prints nothing at 1756688cc08169bef4a9ac37b9b079efb9e38b18.
- From the repository root: python3 -c "import json; d=json.load(open('docs/design/directory/design.json')); print([r['path'] for r in d['inventory'][8:]])" prints ['vendor/rauthy', 'crates/lys', 'docs/design/decisions.json'], and rg -c 'RAUTHY-BASELINE.md"|CONFORMANCE.md"' docs/design/directory/design.json prints 2, as at 1756688cc08169bef4a9ac37b9b079efb9e38b18.

**Files:**
- modify: docs/design/directory/design.json
- modify: docs/design/directory/DESIGN.md

**Checklist:**
- C1 — The directory design records the outcome, the shared contract as principles, the constraints and the non-goals of IDENTITY-001, revised for the grant ruling.
- C6 — The rendered markdown of this cluster matches its JSON and coverage is clean.

**Stories:**
- S2 (Reviewer, Reviews a brief before any of its rows is dispatched) — As the reviewer, I want each open identity row as a design-system brief with numbered requirements and criteria, so that rows can be dispatched to the loop one at a time and reviewed against their criteria.

#### R2 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Accept 1: met. len(inventory) is 11, and the first eight rows were left unchanged by the append. Accept 2: met. '"path": "vendor/rauthy"' counts 2 (lines 380 and 624), '"path": "crates/lys"' counts 1 (line 628), and '"path": "docs/design/decisions.json"' counts 2 (lines 140 and 632). Accept 3: met. Each of the three '^- `...`' patterns matches once in DESIGN.md, at lines 179, 180 and 181. Accept 4: met. inventory[8:] paths are ['vendor/rauthy', 'crates/lys', 'docs/design/decisions.json'], and the RAUTHY-BASELINE and CONFORMANCE rows were not touched, so that count stays 2.
- Deviation: (none)
- Files changed:
  - modified: `docs/design/directory/design.json` — The inventory now holds eleven rows. Rows 9 to 11 (lines 624, 628 and 632) are vendor/rauthy, crates/lys and docs/design/decisions.json, each with the note exactly as specified. The intention also changed, under R3.
  - modified: `docs/design/directory/DESIGN.md` — Re-rendered. The Inventory section lists the eleven rows, with the new three at lines 179 to 181.
- Checklist delivery:
  - [x] C1 — The directory design records the outcome, the shared contract as principles, the constraints and the non-goals of IDENTITY-001, revised for the grant ruling. — The inventory now also records the fork, the CLI crate and the decision ledger. No other design member changed.
  - [x] C6 — The rendered markdown of this cluster matches its JSON and coverage is clean. — DESIGN.md was re-rendered from design.json.
- Story delivery:
  - [x] S2 (Reviewer, Reviews a brief before any of its rows is dispatched) — As the reviewer, I want each open identity row as a design-system brief with numbered requirements and criteria, so that rows can be dispatched to the loop one at a time and reviewed against their criteria. — Reviewers can see what the rows read and move.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] From the repository root: python3 -c "import json; print(len(json.load(open('docs/design/directory/design.json'))['inventory']))" prints 11 (it prints 8 at 1756688cc08169bef4a9ac37b9b079efb9e38b18), and the first eight rows equal, member for member, the eight that git show 1756688cc08169bef4a9ac37b9b079efb9e38b18:docs/design/directory/design.json holds. — Measured: 8 at base, 11 now. inventory[:8] equals the base inventory (True).
  - [x] From the repository root: rg -c '"path": "vendor/rauthy"' docs/design/directory/design.json prints 2 (1 at 1756688cc08169bef4a9ac37b9b079efb9e38b18, the structure row at docs/design/directory/design.json:380); rg -c '"path": "crates/lys"' docs/design/directory/design.json prints 1 (nothing at 1756688cc08169bef4a9ac37b9b079efb9e38b18); rg -c '"path": "docs/design/decisions.json"' docs/design/directory/design.json prints 2 (1 at 1756688cc08169bef4a9ac37b9b079efb9e38b18, the structure row at docs/design/directory/design.json:140). — The three rg -c commands print 2, 1 and 2.
  - [x] From the repository root: rg -c '^- `vendor/rauthy`' docs/design/directory/DESIGN.md prints 1; rg -c '^- `crates/lys`' docs/design/directory/DESIGN.md prints 1; rg -c '^- `docs/design/decisions.json`' docs/design/directory/DESIGN.md prints 1; each prints nothing at 1756688cc08169bef4a9ac37b9b079efb9e38b18. — Each prints 1, at DESIGN.md:179-181, as the rendered Inventory shows.
  - [x] From the repository root: python3 -c "import json; d=json.load(open('docs/design/directory/design.json')); print([r['path'] for r in d['inventory'][8:]])" prints ['vendor/rauthy', 'crates/lys', 'docs/design/decisions.json'], and rg -c 'RAUTHY-BASELINE.md"|CONFORMANCE.md"' docs/design/directory/design.json prints 2, as at 1756688cc08169bef4a9ac37b9b079efb9e38b18. — Prints ['vendor/rauthy', 'crates/lys', 'docs/design/decisions.json'], and the rg count is 2. Each new row's note equals the spec text exactly, and its keys are exactly {path, note}. The notes' facts hold: git ls-tree shows the submodule at dd61ac3c84d6b238108dc8438b53043b5177a662, crates/lys holds the named files, and the last ADR is ADR-018.
- Checklist verified: C1, C6
- Stories verified: S2

### R3: Record the Cambium direction in the intention while the non-goal stands

THE SYSTEM SHALL append one sentence to the intention of docs/design/directory/design.json (docs/design/directory/design.json:4 at 1756688cc08169bef4a9ac37b9b079efb9e38b18, one sentence ending 'every identity change.'), after one space, exactly: 'Cambium later uses this issuer, keeping its participant ids (rows 06 and 07); that direction is recorded here while rows 06 and 07 stay the non-goal recorded below, and this sentence promises neither row.' The non-goal for rows 06 and 07 (docs/design/directory/design.json:63-66, text 'Rows 06 (connect Cambium) and 07 (gate, install and demonstrate the release)' with its reason) SHALL stay byte for byte; no other member of the intention, and no goal, principle, non-goal, structure row, constraint or gate leg, changes. THE SYSTEM SHALL re-render docs/design/directory/DESIGN.md with python3 scripts/design/render-cluster.py docs/design/directory so its Intention section (docs/design/directory/DESIGN.md:11-13 at 1756688cc08169bef4a9ac37b9b079efb9e38b18) carries the sentence.

**Acceptance:**
- From the repository root: rg -c 'keeping its participant ids' docs/design/directory/design.json prints 1 and the same command over docs/design/directory/DESIGN.md prints 1; both print nothing at 1756688cc08169bef4a9ac37b9b079efb9e38b18.
- From the repository root: python3 -c "import json; i=json.load(open('docs/design/directory/design.json'))['intention']; print(i.startswith('An operator installs the identity product without Cambium or Manifold, signs in,') and i.endswith('and this sentence promises neither row.'))" prints True.
- From the repository root: rg -c 'Rows 06 \(connect Cambium\) and 07 \(gate, install and demonstrate the release\)' docs/design/directory/design.json prints 1, as at 1756688cc08169bef4a9ac37b9b079efb9e38b18, and python3 -c "import json; print(len(json.load(open('docs/design/directory/design.json'))['non_goals']))" prints 11, as at 1756688cc08169bef4a9ac37b9b079efb9e38b18.
- From the repository root: python3 -c "import json; d=json.load(open('docs/design/directory/design.json')); print(len(d['goals']), len(d['principles']), len(d['constraints']))" prints 5 9 12, as at 1756688cc08169bef4a9ac37b9b079efb9e38b18.

**Files:**
- modify: docs/design/directory/design.json
- modify: docs/design/directory/DESIGN.md

**Checklist:**
- C1 — The directory design records the outcome, the shared contract as principles, the constraints and the non-goals of IDENTITY-001, revised for the grant ruling.
- C6 — The rendered markdown of this cluster matches its JSON and coverage is clean.

**Stories:**
- S2 (Reviewer, Reviews a brief before any of its rows is dispatched) — As the reviewer, I want each open identity row as a design-system brief with numbered requirements and criteria, so that rows can be dispatched to the loop one at a time and reviewed against their criteria.

#### R3 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Accept 1: met. 'keeping its participant ids' counts 1 in design.json (line 4) and 1 in DESIGN.md. Accept 2: met. The intention starts with the operator sentence and ends with 'and this sentence promises neither row.' Accept 3: met. The non-goals were not edited, so the Rows 06/07 text is still present and non_goals still has 11 entries (measured). Accept 4: met. goals, principles and constraints measure 5, 9 and 12.
- Deviation: (none)
- Files changed:
  - modified: `docs/design/directory/design.json` — At line 4 the intention now ends with one space and then the exact Cambium sentence. The non-goals were not changed.
  - modified: `docs/design/directory/DESIGN.md` — Re-rendered. The Intention section (line 13) carries the sentence.
- Checklist delivery:
  - [x] C1 — The directory design records the outcome, the shared contract as principles, the constraints and the non-goals of IDENTITY-001, revised for the grant ruling. — The outcome now records the Cambium direction, and the non-goal is kept.
  - [x] C6 — The rendered markdown of this cluster matches its JSON and coverage is clean. — DESIGN.md was re-rendered.
- Story delivery:
  - [x] S2 (Reviewer, Reviews a brief before any of its rows is dispatched) — As the reviewer, I want each open identity row as a design-system brief with numbered requirements and criteria, so that rows can be dispatched to the loop one at a time and reviewed against their criteria. — The direction is recorded without promising any row.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] From the repository root: rg -c 'keeping its participant ids' docs/design/directory/design.json prints 1 and the same command over docs/design/directory/DESIGN.md prints 1; both print nothing at 1756688cc08169bef4a9ac37b9b079efb9e38b18. — Prints 1 for design.json and 1 for DESIGN.md.
  - [x] From the repository root: python3 -c "import json; i=json.load(open('docs/design/directory/design.json'))['intention']; print(i.startswith('An operator installs the identity product without Cambium or Manifold, signs in,') and i.endswith('and this sentence promises neither row.'))" prints True. — Prints True. The intention also equals the base intention + ' ' + the exact spec sentence (True).
  - [x] From the repository root: rg -c 'Rows 06 \(connect Cambium\) and 07 \(gate, install and demonstrate the release\)' docs/design/directory/design.json prints 1, as at 1756688cc08169bef4a9ac37b9b079efb9e38b18, and python3 -c "import json; print(len(json.load(open('docs/design/directory/design.json'))['non_goals']))" prints 11, as at 1756688cc08169bef4a9ac37b9b079efb9e38b18. — rg -c prints 1, and len(non_goals) is 11. non_goals equals the base value (True).
  - [x] From the repository root: python3 -c "import json; d=json.load(open('docs/design/directory/design.json')); print(len(d['goals']), len(d['principles']), len(d['constraints']))" prints 5 9 12, as at 1756688cc08169bef4a9ac37b9b079efb9e38b18. — Prints 5 9 12. Against HEAD, only intention and inventory differ in design.json.
- Checklist verified: C1, C6
- Stories verified: S2

## Boundaries

- Documents only under docs/design/directory: the build changes exactly the four files R1 to R3 name (docs/design/directory/briefs/DIRECTORY-005.json, docs/design/directory/briefs/DIRECTORY-005.md, docs/design/directory/design.json, docs/design/directory/DESIGN.md) and nothing else; no code; no file under crates/, surface/, deploy/, vendor/, scripts/ or tests/; docs/design/decisions.json is not touched either, narrower than CN1.
- No new ADR: docs/design/decisions.json is unchanged; this brief's design_anchor cites decisions that exist at 1756688cc08169bef4a9ac37b9b079efb9e38b18 and it records no decision.
- No change to any row's identifiers, estimates or dependency order: every brief id, roadmap id, ADR id, C, S, R and ID001 identifier, every estimate (DIRECTORY-005's 6 hours, CN7's ceiling) and every depends_on array stays as main holds it.
- Main wins wherever the 25 September patch contradicts it on ADR ids and lifecycle: nothing from the patch beyond the three items is carried, and no settled row reopens.
- The non-goal for rows 06 and 07 stands: the intention sentence records a direction and promises no row, no Cambium cluster and no cross-repository arrangement.
- Nothing open for Tom is decided: every decision the design's non-goals record as open stays open.
- The roadmap row RM-016 in docs/design/roadmap.json is this brief's own ledger entry, written on the brief branch by this brief and never by the build; per the settled answer B1 of 22:49 it stands outside the 'documents only under docs/design/directory' boundary, which governs the build's work (R1 to R3). The build changes no file outside docs/design/directory.
- No credential, token or key value is written in any document.
- No row is dispatched until this brief is signed off.

## Verification

- From the repository root: bash scripts/design/gate.sh exits 0 (scripts/design/gate.sh: validate.py over docs/design/decisions.json, docs/design/project.json and every cluster, check-coverage.py over every cluster, and render-cluster.py into a temporary copy compared byte for byte with the committed markdown).
- From the repository root, on the build branch: git diff --no-ext-diff --name-only <base>..HEAD prints exactly four paths, docs/design/directory/DESIGN.md, docs/design/directory/briefs/DIRECTORY-005.json, docs/design/directory/briefs/DIRECTORY-005.md and docs/design/directory/design.json, where <base> is the commit the build started from; git diff --no-ext-diff --name-only <base>..HEAD -- . ':!docs/design/directory' prints nothing.
- From the repository root: python3 scripts/design/validate.py docs/design/directory and python3 scripts/design/check-coverage.py docs/design/directory exit 0; the coverage report may list C1, C3, C6 and S2 as claimed by DIRECTORY-001 and DIRECTORY-008, a warning whose split this brief's task notes, and lists no failure.
- From the repository root: python3 scripts/design/render-cluster.py docs/design/directory run a second time changes no file (git status --porcelain docs/design/directory prints nothing after it).


---
type: brief
id: DIRECTORY-026
cluster: directory
title: Make each session its own directory record with its own credential, started when the agent reports back with a signed challenge
---

# DIRECTORY-026: Make each session its own directory record with its own credential, started when the agent reports back with a signed challenge

> **Cluster:** directory
> **Depends on:** DIRECTORY-003
> **Blocked by:** The lead's sign-off of this brief on its card, before card_build_v3 runs. No row is dispatched before it, and a build started without it is a defect of the chain. The sign-off is also the review of this brief's file manifest (CN9). Check: the card records the sign-off., Every row waits on DIRECTORY-003 landed on main, read from its own execution record. Check, from the repository root: git fetch origin main && git show origin/main:docs/design/directory/briefs/DIRECTORY-003.json | python3 -c "import json,sys; print(json.load(sys.stdin).get('execution', {}).get('status', 'no execution record'))" prints landed., Every row waits on the provision card (the brief titled 'Keep the enduring agent apart from each session's credential, and show a registered agent with no session credential') landed on main, read from its own execution record; its contract section '## Enduring agents and session credentials' is what R1 builds beside. Check, from the repository root: git fetch origin main && T="Keep the enduring agent apart from each session's credential, and show a registered agent with no session credential" python3 -c "import json,os,subprocess as s; g=lambda *a: s.run(['git',*a],capture_output=True,text=True,check=True).stdout; fs=[f for f in g('ls-tree','--name-only','origin/main','docs/design/directory/briefs/').split() if f.endswith('.json')]; m=[json.loads(g('show','origin/main:'+f)) for f in fs]; m=[b for b in m if b.get('title')==os.environ['T']]; print(len(m), *[b.get('execution',{}).get('status','no execution record') for b in m])" prints 1 landed. The check selects the card's brief by its whole top-level title field, so this brief, which quotes the title only inside its own text, never matches. It prints 1 landed once the card has landed; 0 while no brief on main carries the title; 1 followed by another status while the card is on main but not landed; a first number above 1 if two briefs carry the title, which stops the row as a defect for the card's lead., Every row waits on the typed capability claim card (the brief titled 'Carry a typed capability claim in an agent's certificate and read it with a named verifier') landed on main, read from its own execution record, because a session starts only for an agent whose certificate has been issued, and the report-back is checked against the key that card's enrolment requirement enrols with the agent, apart from the certificate. Check, from the repository root: git fetch origin main && T="Carry a typed capability claim in an agent's certificate and read it with a named verifier" python3 -c "import json,os,subprocess as s; g=lambda *a: s.run(['git',*a],capture_output=True,text=True,check=True).stdout; fs=[f for f in g('ls-tree','--name-only','origin/main','docs/design/directory/briefs/').split() if f.endswith('.json')]; m=[json.loads(g('show','origin/main:'+f)) for f in fs]; m=[b for b in m if b.get('title')==os.environ['T']]; print(len(m), *[b.get('execution',{}).get('status','no execution record') for b in m])" prints 1 landed. The check selects the card's brief by its whole top-level title field, so this brief, which quotes the title only inside its own text, never matches. It prints 1 landed once the card has landed; 0 while no brief on main carries the title; 1 followed by another status while the card is on main but not landed; a first number above 1 if two briefs carry the title, which stops the row as a defect for the card's lead., R10 alone also waits on the certificate revocation card's fold on main, because a revoked certificate is refused only through that fold, never through a second list kept here. The card is the brief titled 'Revoke a certificate by appending a leaf, fold the live set from the log, and refuse a revoked certificate while its history still verifies', DIRECTORY-013 on brief/directory/1e30a4cb, built by hand on hand/DIRECTORY-013 and carried in PR 28; it is named by its title and its fold, whatever id it lands under. R1 to R9 do not wait on it. Check, from the repository root: git fetch origin main && git ls-tree --name-only origin/main -- crates/lys-identity/src/revocation/fold.rs prints exactly that one path., R5 to R10 wait on R4: docs/design/identity/SESSION-CREDENTIAL-REVIEW.md on the card's branch records every attack R4 names as defeated, reviewed by a party other than the author of R1, R2 and R3. Check, from the repository root: grep -c '| defeated |' docs/design/identity/SESSION-CREDENTIAL-REVIEW.md prints 13 or more, and grep -c '| not defeated |' docs/design/identity/SESSION-CREDENTIAL-REVIEW.md prints 0., Before R5 is dispatched, every modify path under crates/lys-identity, crates/lys-identity-server and tests/identity_contract is reconciled against the landed manifests of DIRECTORY-003 and of the typed capability claim card (CN9, CN12). A path that landed under another name stops the row, which names it, and the act that answers is a brief revision through the card's lead. Check, from the repository root: git ls-tree -r --name-only origin/main -- crates/lys-identity/src/lib.rs crates/lys-identity/src/event.rs crates/lys-identity/src/encoding.rs crates/lys-identity/src/provenance.rs crates/lys-identity/Cargo.toml crates/lys-identity-server/src/lib.rs crates/lys-identity-server/src/routes.rs crates/lys-identity-server/src/config.rs tests/identity_contract/src/harness.rs crates/lys-identity/src/agent_key.rs prints all ten paths.
> **Design anchor:**
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> - ADR-007 — The product starts an agent by giving its start command, never by running it — The product is not an execution engine. For the first release an agent's file gives the command that starts it on a chosen machine: the command carries the agent's identity and its handles, never a credential's value, and it is rendered from the agent's kept launch record. A started agent reports back, so the sessions screen shows what is running. A terminal inside the product, sandboxes, virtual machines and containers are later runtimes that plug in, and none is built into this product.
> - ADR-008 — An agent's file shows its lys certificate — An agent's file shows its lys certificate once one is issued: what it claims, who signed it, when it was issued and when it expires, with the signed receipts of the changes made to it. An agent registered before any key or proof of possession was supplied shows its certificate as not issued, never a placeholder.
> - ADR-011 — An identity is registered, active, suspended or retired — An identity is in one of four states: registered (exists in the directory, no grants, no credential handle, may not act), active (may act within its grants), suspended (kept whole, grants kept but not effective) and retired (permanent, history kept, never reactivated; a new identity is made instead). Register, activate, suspend, reinstate and retire are the only transitions, each one signed audit record naming the authenticated actor and their provenance, the identity, from, to, when and reason. Having a grant or a credential is a fact beside the state, not a state. A person is registered by first sign-in; an agent is registered by a signed-in person, who carries it as its responsible person for life and may cause every transition of their own agents. Source: docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:17-44 and docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:71-95.
> - ADR-079 — A session credential is a random value the directory issues when the agent's signed report-back over a directory-issued challenge creates the session, and keeps only as a hash — The session credential is 32 random bytes the directory issues when the session is created. The directory keeps only their SHA-256, on the session record, and returns the value once, in the answer to the report-back that created the session; the session presents the value to the directory, which is the only party that checks it. A session is created when the started agent first reports back and proves it is the agent with its enrolled key, and only for an agent whose certificate has been issued. Before reporting back, the agent asks the directory for a challenge: 32 random bytes the directory issues once, for that agent and a launch record id the agent supplies, kept until used or until it expires more than 60 seconds after issue. The proof is an Ed25519 signature under the tag lys/session-start/v1 (ADR-080). The directory verifies the signature before anything else: an unsigned or badly signed proof, or one naming an agent it does not hold or one with no enrolled key, is refused proof_invalid with one body and touches no challenge, and only a correctly signed proof receives the state-revealing refusals. It admits the proof only when the challenge is one it issued to that agent, unused and unexpired, the directory named is itself and the launch record id matches the challenge's, and it marks the challenge used in the same act that creates the session. Two acts end a session: the agent reporting its end with its credential, and the directory's administrator stopping it. Ending changes the record's state, so the credential is refused at once, naming the ended session. Rejected: a per-session lys certificate, because nobody verifies a session credential offline in this card, so a signed artifact buys nothing and would freeze a format before anything needs it; a lys/delegation/v1 SpeaksFor delegation, framed for an offline root key and a seat subject, whose module says not to invent a consumer; an ADR-001 handle, which waits on the unbuilt secrets broker (SECRETS-002); creating the session when a start command is rendered; and expiry of a session.
> - ADR-080 — lys/session-start/v1 signs five fields, the tag, the agent id, the directory's identifier, the launch record id and a directory-issued nonce, and is proposed in the wire-format register until ratified — The report-back is an Ed25519 signature by the agent's enrolled key over the 20 ASCII bytes of lys/session-start/v1, one 0x00 byte, then the agent directory id, the directory's own identifier and the launch record id, each as its UTF-8 length in 4 bytes big-endian followed by its bytes, then the 32-byte nonce, a challenge the directory issued once to that agent for that launch record id and admits for 60 seconds after issue. Freshness (the nonce), audience (the directory's identifier) and the launch record id are in the first version because the tag cannot gain them after it freezes. The format is proposed in docs/design/WIRE-FORMATS.md's decision log; the tests sign under it with disposable test keys only, and no production agent signs under it until the register shows it ratified. Rejected: a four-field message without the directory's identifier, which could not say which directory a proof was made for; a message without the launch record id, which could not tie a session to the command it was started by; a signature over the fields without a directory-issued nonce, which a replay would satisfy; and treating this ledger entry as the ratification, which the register reserves to its own decision log.
> **Checklist:**
> - C193 — docs/design/identity/DIRECTORY-CONTRACT.md has a section '## Sessions and their credentials' giving the session record's seven members, the credential as 32 random bytes kept only as its SHA-256, the challenge issued for one agent id and one launch record id with its 60-second lifetime, the lys/session-start/v1 message over the agent's directory id, the directory's identifier, the launch record id and the challenge with a 109-byte worked example, the signature checked first, the state-revealing refusals in their order, the two acts that end a session, and the fourteen named refusals.
> - C194 — docs/design/identity/IDENTITY-EVENTS.md extends lys/identity-event/v1 under a fresh joint review without changing its byte layout: session_started and session_ended are event kinds naming the agent and its responsible person, session_started carrying the launch record id, agent_reported is the agent's accepted report itself, and a session ended by the agent is an agent_reported followed by a session_ended naming it; no event carries a credential's value; the actor gains method codes 2, 3 and 4 with their issuer and subject stated, and no event is written under method 4 in this card, the change kinds gain 7 to 9, and every code is only ever appended.
> - C195 — docs/design/WIRE-FORMATS.md carries one decision-log row proposing lys/session-start/v1, PROPOSED and not ratified, naming its fields (the tag, the agent id, the launch record id, the nonce, the audience and the freshness) and stating that no production agent signs under it until the log shows it ratified.
> - C196 — docs/design/identity/SESSION-CREDENTIAL-REVIEW.md records an adversarial review of the session credential and the report-back proof, by a party other than their author, with each of its thirteen attacks defeated, including replay, cross-directory, cross-agent, launch-record swap, cross-protocol use and unauthenticated probing, before any session code starts.
> - C197 — A challenge is issued once, for one agent id and one launch record id, with the same answer whether or not the directory holds the agent: a proof arriving 60 seconds after its issue is admitted, and one arriving 61 seconds after is refused challenge_expired.
> - C198 — A report-back from an agent whose certificate is issued, signed by its enrolled key, marks the challenge used in the same act that creates one session record pointing at the agent's directory id, commits one session_started event and returns the credential's value once, the directory keeping only its SHA-256.
> - C199 — Starting a session never creates an agent: a report-back naming an agent the directory does not hold is refused proof_invalid, and the directory's agent count is unchanged by any number of session starts.
> - C200 — The report-back's signature is verified before anything else: an unsigned or badly signed report-back, or one naming an agent the directory does not hold or one with no enrolled key, is refused proof_invalid with one and the same body and leaves the challenge store byte-identical, and only a correctly signed proof receives the state-revealing refusals, in their order.
> - C201 — A correctly signed report-back is refused by name, committing no event and creating no session, for a reused challenge (challenge_reused), an expired one (challenge_expired), one issued to another agent (challenge_foreign_agent), one never issued (challenge_unknown), a proof naming another directory (wrong_directory), a launch record id that differs from the challenge's (launch_record_mismatch), a retired agent (agent_retired) and an agent whose certificate is not issued (certificate_not_issued, naming the act of issuing it); a suspended agent is not refused.
> - C202 — A presented session credential is answered with the agent directory id and session id from its session record, and a presentation commits no event.
> - C203 — A second session of the same agent, started while the first is open, presents the same agent directory id with a different credential, and after the agent reports the first session's end its credential is refused session_ended, naming that session, while the second is still accepted.
> - C204 — The directory's configured administrator stops a session as one signed event naming the administrator as actor, after which its credential is refused session_ended, naming the session; any other caller's stop is refused by DIRECTORY-003's admission.
> - C205 — The directory service answers the challenge, the report-back, the presentation, the agent's end and the administrator's stop on five routes, answers every proof_invalid case with the same status and body bytes, and no log line, trace or answer other than the report-back's carries a credential's value.
> - C206 — An agent authenticated by its enrolled key creates and ends its own session record and nothing else: its session credential presented on another agent's session stop and on DIRECTORY-003's agent registration is refused by DIRECTORY-003's admission, and a report-back naming another agent signed by its key is refused proof_invalid.
> - C207 — An agent's sessions are listed by its directory id, in the order they started, with each session's state and how it ended, on a route only the directory's configured administrator is answered on, by a command written out in full, and every other caller is refused by DIRECTORY-003's admission.
> - C208 — A report-back from an agent whose certificate has been revoked is refused certificate_revoked, creating no session, and the agent's open sessions stay open.
> **Stories:**
> - S80 (Started agent, Reports back to the directory and presents its session credential) — As a started agent, I want to present my session credential to the directory, so that it confirms I am my enduring agent in this session.
> - S81 (Responsible person, Signs in and provisions agents under their own authority) — As a responsible person, I want each session of my agent kept as its own record under that one agent, so that starting another session never makes another agent.
> - S82 (Operator, Installs and runs the standalone identity product) — As the operator, I want to stop a session on the directory, so that its credential is refused even when the agent crashed and never reported its end.
> - S83 (Operator, Installs and runs the standalone identity product) — As the operator, I want to list an agent's sessions with their states, so that I can see which of its session credentials are still accepted.

## Purpose

Road step 2's session half. A session becomes its own record in the directory, apart from the enduring agent it belongs to: it is created when the started agent first reports back and proves it is the agent, it points at that agent by the agent's directory id, and it carries a session credential of its own, distinct from the agent's one certificate (ADR-008). The directory checks that credential when the session presents it, and refuses it once the session has ended. This is the presentation step the provision card names as this card's, and the second-session proof that card waits on: a second session of the same agent presents the same enduring identity with a new credential, and the first session's credential is refused once that session has ended.

## Task

Ten rows in dependency order. R1 to R3 are documents: R1 writes the session section of the directory contract, R2 writes the two session events into the event envelope's document, and R3 proposes the tag lys/session-start/v1 in the wire-format register. R4 is the adversarial review of R1 to R3, which comes before any code. R5 to R10 are code in lys-identity and lys-identity-server: R5 issues the challenge and starts a session, R6 checks a presented credential, R7 ends a session and proves both done criteria, R8 exposes the routes, R9 lists an agent's sessions with their states for the directory's configured administrator, and R10 refuses a revoked certificate. Estimates, in focused implementer hours: R1 1.5, R2 0.5, R3 0.5, R4 2, R5 5, R6 1, R7 2, R8 3, R9 2.5, R10 1.5, total 19.5. They are stated beside CN7's ceiling, not inside it: CN7's 48 hours are IDENTITY-001's rows, and this card is road step 2 work outside those rows, so it changes nothing in that ceiling.

The session credential form is recorded in ADR-079. It is none of the three readings the provision brief names (a per-session lys certificate, a lys/delegation/v1 SpeaksFor delegation, an ADR-001 handle). It is a random value the directory issues when the session is created. The directory keeps only its hash, on the session record, and the session presents the value to the directory. In this card the directory is the only party that checks a session credential, so a signed artifact would buy nothing and would freeze a format before anything needs it; a handle waits on the unbuilt secrets broker; SpeaksFor is framed for an offline root key and a seat subject, and its module says not to invent a consumer. Ending a session changes the record's state, so the credential is refused at once. A signed session credential, for when a party other than the directory must check a session, is later work.

A session is created when the started agent first reports back and proves it is the agent, never when a start command is rendered. A start command that is rendered and never run leaves no session, and a start command never carries a credential's value (ADR-007). Before reporting back, the agent asks the directory for a challenge, naming its agent directory id and a launch record id: a random value the directory issues once, for that agent and that launch record id, with a lifetime of 60 seconds, and keeps until it is used or expires. The challenge's answer is the same whether or not the directory holds the agent named, so asking for one reveals nothing about which agents exist. The report-back is a proof under the new tag lys/session-start/v1 (ADR-080), signed by the agent's enrolled key over five fields: the tag, the agent's directory id, the directory's own identifier, the launch record id and the challenge. The enrolled key is the one the typed capability claim card's enrolment requirement enrols with the agent, apart from its certificate.

The report-back verifies the proof's signature under the agent's enrolled key before it answers anything else. A report-back that is unsigned or badly signed, or names an agent the directory does not hold or one with no enrolled key, is refused proof_invalid with one body, so an unauthenticated caller learns nothing about which agents exist or their state; certificate_not_issued is not excepted. An unsigned or invalid report-back never changes, consumes or drops a challenge. Only a correctly signed proof then gets the state-revealing refusals, in this order: agent_retired, certificate_not_issued (naming the act that answers it, issuing this agent's certificate), certificate_revoked (R10), wrong_directory, challenge_reused, challenge_unknown, challenge_foreign_agent, challenge_expired and launch_record_mismatch. When every check passes, the directory marks the challenge used in the same act that creates the session. Freshness, audience and the launch record are in the tag from its first version because it freezes once signed. Starting a session never creates, registers or changes an agent. Whichever engine runs the agent can present the report-back (ADR-004).

lys/session-start/v1 is proposed in docs/design/WIRE-FORMATS.md by R3, and its ratification is recorded in that register's decision log by the register's ratifier, never by this card. The build does not wait for ratification: the tests and the proof sign under the proposed tag with disposable test keys only, and no production agent signs under lys/session-start/v1 until the register's decision log shows it ratified.

The launch record id is carried as supplied. This card does not check that a launch record with that id exists or that it belongs to the agent; that check is the start card's, which makes launch records and adds it with its own acceptance line, so the two cards do not depend on each other. Until the start card lands, an invented launch record id yields only a session whose report names a launch record nobody made.

Two acts end a session: the agent reporting its end with its session credential, and the directory's configured administrator stopping it on the directory. Each is recorded with its actor: the administrator's stop is one session_ended event, and the agent's reported end is an agent_reported event, the report itself, followed by a session_ended event naming it. After either one the credential is refused, naming the ended session. Expiry is not in this card. A crashed agent never reports its end, so its session stays open until an operator stops it. Lapsing a stale session is its own card. This card ends no open session on any other act: an agent's retirement or its certificate's revocation refuses its next report-back and leaves its open sessions open, and ending open sessions is the emergency-stop card's. Suspension is not a refusal in this card: whether a suspended agent may start a session stays open for the person who owns that question, and this brief says so rather than deciding it.

P9 gives way narrowly for this card, as the line appended beneath it records: an agent authenticated by its enrolled key creates its own session record, and ends it with the session credential that report-back issued it, and nothing else. Every other mutation by a caller who is not the configured administrator, an agent acting on another agent's session included, is still refused in step 1 by DIRECTORY-003's admission refusal. C13 carries the same exception in its words. CN6's hold point binds only the brief that follows row 05 in IDENTITY-001's order, not this brief: this build is blocked by exactly the blockers listed, and no demonstration receipt is one of them.

The proof is server-side, plus a command written out in full in this brief that lists an agent's sessions with their states. It is admitted only for the directory's configured administrator, as DIRECTORY-003 R3 and P9 admit only that administrator in step 1; every other caller is refused by name. Neither the sessions screen nor the agent's file is in this card. The sessions screen is its own card, which reads the records this card makes. The agent's responsible person gains the listing when step 1's admission admits them, which is later work. The listing command runs against the in-process directory its own test starts, because DIRECTORY-003 defines no scriptable sign-in; running it against a deployed directory is later work, with the card that brings a scriptable sign-in.

This card stands beside the design's 'Road step 2 onward' non-goal and P6 as a step-2 increment. Each carries one appended line naming this card's session record and credential as step-2 work brought forward, and neither is otherwise rewritten; DESIGN.md does not gain the road. CN1 carries one appended line recording that it binds only the documents-only planning briefs, and its text is unchanged. DIRECTORY-003 stands: each agent is under its responsible person, and every identity change is one signed event. Each session event names the agent and its responsible person. What the agent's certificate carries is the typed capability claim card's, and this card reads only whether a certificate has been issued and which key is enrolled for the agent.

This brief supersedes and closes the earlier draft of this card (run d5055cc1), its brief and its decision; the start card names this brief by that run, whatever id it lands under.

C193 to C208 and S80 to S83 are this brief's alone. The agent's session lives in modules named agent_session, so it is never confused with lys-identity-server's operator sign-in session. Every code row is red first: its first commit adds its test file with a failing expectation named, and its second commit makes it pass. Test names are snake_case and carry the criterion ids (d026_r5_ac1 and so on). No wall clock is read inside lys-identity, where time comes from a clock passed in. Throughout, <base> is the commit the row's build started from. Every path is relative to the repository root.

## Requirements

### R1: Write the session record, its credential, the challenge and the report-back proof into the directory contract

The directory contract docs/design/identity/DIRECTORY-CONTRACT.md SHALL gain one section headed '## Sessions and their credentials', added after the section '## Enduring agents and session credentials' and changing no earlier section. The section SHALL state: (1) A session record has exactly these members: session id, agent directory id, launch record id, credential hash, state, start event id and end event id, the last empty while the session is open. The agent record gains no member. (2) A session id is 16 random bytes from the operating system's generator, written as 32 lowercase hexadecimal characters. (3) A session credential is 32 random bytes from the operating system's generator, written as unpadded base64url (43 characters). The directory keeps only the SHA-256 digest of the 32 bytes, as the session record's credential hash. It returns the value once, in the answer to the report-back that created the session, and never again. (4) The state is open or ended, and ended is final. (5) Before reporting back, the agent asks the directory for a challenge naming its agent directory id and a launch record id. A challenge is 32 random bytes from the operating system's generator, written as unpadded base64url, issued once to that one agent directory id for that one launch record id, and held by the directory with its issue instant until it is used or expires. The answer to a challenge request is the same whether or not the directory holds the agent named. A challenge is expired when the directory's clock reads more than 60 seconds after its issue instant: a proof arriving 60 seconds after issue is admitted and one arriving 61 seconds after is refused challenge_expired. (6) The launch record id is carried as supplied, byte for byte; this card checks only that the report-back's launch record id equals the challenge's, and the check that a launch record with that id exists and belongs to the agent is the start card's. (7) The directory's identifier is a UTF-8 string set in the directory service's configuration. The agent signs the identifier of the directory it is configured to report to, never one taken from a challenge's answer. (8) A session is created only when the started agent reports back with a proof. The proof is an Ed25519 signature, by the agent's enrolled key, over this message: the 20 ASCII bytes of the tag lys/session-start/v1, one 0x00 byte, then for each of the agent directory id, the directory identifier and the launch record id in that order its UTF-8 length as 4 bytes big-endian followed by those UTF-8 bytes, then the 32 bytes of the challenge. The report-back carries the agent directory id, the directory identifier, the launch record id, the challenge and the signature. (9) The report-back's signature is verified first, with Ed25519 strict verification under the agent's enrolled key. A report-back with no signature, a signature that does not verify, an agent directory id the directory does not hold, or an agent with no enrolled key is refused proof_invalid with one and the same body, and changes, consumes and drops no challenge. Only a correctly signed proof receives the state-revealing refusals, in this order: agent_retired, certificate_not_issued, certificate_revoked, wrong_directory, challenge_reused, challenge_unknown, challenge_foreign_agent, challenge_expired, launch_record_mismatch. The directory admits the proof only when every one of those checks passes, and it marks the challenge used in the same act that creates the session. (10) An agent authenticated by its enrolled key creates its own session record, and the session credential that report-back issued ends that session and no other; neither admits any other mutation. (11) Two acts end a session: the agent reporting its end with its credential, and the directory's configured administrator stopping it. Expiry of a session is not in this card, and a crashed agent's session stays open until an operator stops it. Retirement and revocation refuse a new session and end no open one. Suspension refuses nothing in this card. (12) The refusals, one row each in a table naming what each refuses: proof_invalid, agent_retired, certificate_not_issued, certificate_revoked, wrong_directory, challenge_reused, challenge_unknown, challenge_foreign_agent, challenge_expired, launch_record_mismatch, credential_unknown, session_ended, session_unknown, unknown_agent, unknown_agent being answered only to the configured administrator's listing. The section SHALL say that the tag lys/session-start/v1 is proposed in docs/design/WIRE-FORMATS.md, that no production agent signs under it until that register's decision log shows it ratified, that it is frozen once any agent signs under it, and that a change is a new tag alongside. The section SHALL NOT add a member to the agent record, choose a signed session credential, give lys/delegation/v1's SpeaksFor a consumer, define a launch record, or edit any other section of the contract or any other file.

**Acceptance:**
- grep -c '^## Sessions and their credentials$' docs/design/identity/DIRECTORY-CONTRACT.md prints 1, and the section's heading line comes after the line '## Enduring agents and session credentials'.
- The section lists the session record's members as exactly seven: session id, agent directory id, launch record id, credential hash, state, start event id, end event id.
- The section's worked example gives, for the agent directory id agent-1, the directory identifier dir-1, the launch record id 0123456789abcdef0123456789abcdef and a challenge of 32 bytes each 0x00, a message of exactly 109 bytes: the 20 bytes of lys/session-start/v1, 0x00, then 0x00 0x00 0x00 0x07 and the 7 bytes agent-1, then 0x00 0x00 0x00 0x05 and the 5 bytes dir-1, then 0x00 0x00 0x00 0x20 and the 32 bytes 0123456789abcdef0123456789abcdef, then 32 bytes each 0x00.
- The section's refusal table has exactly 14 rows, whose names are proof_invalid, agent_retired, certificate_not_issued, certificate_revoked, wrong_directory, challenge_reused, challenge_unknown, challenge_foreign_agent, challenge_expired, launch_record_mismatch, credential_unknown, session_ended, session_unknown, unknown_agent.
- The section names the four report-backs refused proof_invalid with one and the same body (no signature, a signature that does not verify, an agent directory id the directory does not hold, an agent with no enrolled key), and lists the state-revealing refusals in exactly this order: agent_retired, certificate_not_issued, certificate_revoked, wrong_directory, challenge_reused, challenge_unknown, challenge_foreign_agent, challenge_expired, launch_record_mismatch.
- The section states a challenge's lifetime as 60 seconds with the two cases 60 seconds admitted and 61 seconds refused challenge_expired, and says in so many words that expiry of a session is not in this card, that a crashed agent's session stays open until an operator stops it, and that the existence check of a launch record is the start card's.
- From the repository root, on the build branch, where <base> is the commit the row's build started from: git diff -U0 <base> HEAD -- docs/design/identity/DIRECTORY-CONTRACT.md | grep -c '^-[^-]' prints 0, and git diff --name-only <base> HEAD after R1's commit prints exactly one path, docs/design/identity/DIRECTORY-CONTRACT.md.

**Files:**
- modify: docs/design/identity/DIRECTORY-CONTRACT.md

**Checklist:**
- C193 — docs/design/identity/DIRECTORY-CONTRACT.md has a section '## Sessions and their credentials' giving the session record's seven members, the credential as 32 random bytes kept only as its SHA-256, the challenge issued for one agent id and one launch record id with its 60-second lifetime, the lys/session-start/v1 message over the agent's directory id, the directory's identifier, the launch record id and the challenge with a 109-byte worked example, the signature checked first, the state-revealing refusals in their order, the two acts that end a session, and the fourteen named refusals.

**Stories:**
- S81 (Responsible person, Signs in and provisions agents under their own authority) — As a responsible person, I want each session of my agent kept as its own record under that one agent, so that starting another session never makes another agent.

### R2: Extend lys/identity-event/v1 with the session events and the agent's actor methods, under a fresh joint review of the envelope

docs/design/identity/IDENTITY-EVENTS.md SHALL gain one section headed '## Session events', defining three event kinds added alongside the existing kinds. lys/identity-event/v1 is extended, not versioned: its byte layout stays exactly as built, with the same body keys and the same map shapes. The actor's method codes gain 2 (agent, enrolled key), 3 (agent, session credential) and 4 (the directory itself) beside 1 (OIDC), written as one line beneath the body table, and the change-kind table gains, appended after its last row, 7 session_started, 8 session_ended and 9 agent_reported, the three codes the extension adds. Both the method codes and the change-kind table state one rule: a code is only ever appended, and is never reused or renumbered. The extension is reviewed jointly, by DIRECTORY-003's builder and the card's lead, and the review is recorded as one entry appended to the document's '## Review' section. The actor map keeps exactly its four members (1 issuer, 2 subject, 3 method, 4 authenticated-at) and gains none. Under method 2 the issuer and subject are the agent's issuer-subject binding as DIRECTORY-003 R1 records it: the issuer is the issuer that issued the agent's enrolled key and the subject is that binding's subject. Under method 3 the issuer is the directory's own identifier, since the directory issued the session credential, and the subject is the agent's directory id. Under method 4 the issuer is the directory's own issuer name as configured and the subject is the directory's service identifier, so an event the directory itself causes names who caused it and is checked like any other actor, with no sentinel and no empty member. The session a credential belongs to is carried by the event's own session id member, never by the actor map, and the section says so in words. agent_reported is the agent's report itself: one event for each signed report the directory accepts from the agent, whatever the report says, committed at the moment the directory accepts it. It has exactly the members session id, agent directory id, reported (one of the closed list started, running and ended), the time the directory received the report, start proof SHA-256 and actor, the actor being the agent. The only time it carries is the directory's own received time: it carries no time the agent stated, because the signed message of lys/session-start/v1 carries none and a time nobody signed proves nothing. It carries no signature member. What authenticated the report is the actor's method: a start report is the signed lys/session-start/v1 proof, under method 2, and the event names that proof by start proof SHA-256, the SHA-256 digest (32 bytes) of the message R1 defines that the agent signed, present exactly when reported is started; a reported end is presented with the session credential, under method 3, and carries nothing more. running is defined in the closed list so that the wire is settled once; this card accepts no running report and commits no agent_reported event reporting running. session_ended is committed only by the directory, once, when the session ends, and names how it ended. So a session ended by the agent is two events, an agent_reported reporting ended and then a session_ended naming it, and a report of running is an agent_reported alone. session_started has exactly the members session id, agent directory id, responsible person id, launch record id, credential SHA-256 (32 bytes), challenge (32 bytes) and actor, the actor being the agent authenticated by its enrolled key. session_ended has exactly the members session id, agent directory id, responsible person id, actor, how it ended, one of operator_stopped, agent_reported and expired, and report event id, the id of the agent_reported event that reported the end, present exactly when how it ended is agent_reported. For how it ended agent_reported the actor is the agent under method 3, since the agent presenting its session credential caused the end, while the directory is what writes the event. For operator_stopped the actor is the configured administrator's person id with their authentication provenance (P8). For expired the actor is the directory itself, under method 4; expiry is not in this card, and this card commits no session_ended event with how it ended expired. The section states method 4 and its two members as the settled encoding, and names the card that lapses a stale session as the one that writes the first event under method 4 and proves it; this card adds no test for method 4. Issuing the credential and starting the session are the one session_started event. The section SHALL NOT give any of the three events a member holding the credential's value, SHALL NOT give any of the three events a member holding a signature or a time the agent stated, and SHALL NOT put a session id in the actor map. It SHALL NOT change an existing kind, an existing method code, the envelope's version, its body keys, its map shapes or its signed encoding, SHALL NOT reuse or renumber a code, and it SHALL NOT change lys/attestation/v2, lys/delegation/v1 or any other shipped tag.

**Acceptance:**
- grep -c '^## Session events$' docs/design/identity/IDENTITY-EVENTS.md prints 1.
- The section defines session_started with exactly seven members (session id, agent directory id, responsible person id, launch record id, credential SHA-256, challenge, actor); session_ended with exactly six (session id, agent directory id, responsible person id, actor, how it ended, report event id), how it ended having exactly the three values operator_stopped, agent_reported and expired; and agent_reported with exactly six (session id, agent directory id, reported, time the directory received it, start proof SHA-256, actor), reported having exactly the three values started, running and ended, and start proof SHA-256 stated as present exactly when reported is started.
- No member of any of the three events is named credential without the suffix SHA-256; no member of any of the three events is named signature or time the agent stated; and the section's text for methods 2, 3 and 4 names the issuer and subject as: method 2, the issuer that issued the agent's enrolled key and that issuer-subject binding's subject; method 4, the directory's own issuer name as configured and the directory's service identifier; method 3, the directory's own identifier and the agent's directory id.
- From the repository root, on the build branch, where <base> is the commit the row's build started from: git diff -U0 <base> HEAD -- docs/design/identity/IDENTITY-EVENTS.md | grep -c '^-[^-]' prints 0.
- docs/design/identity/IDENTITY-EVENTS.md's change-kind table has exactly nine rows, rows 1 to 6 byte-identical to the base commit's and rows 7, 8 and 9 reading session_started, session_ended and agent_reported; the method codes line names exactly 1 OIDC, 2 agent, enrolled key, 3 agent, session credential and 4 the directory itself; and grep -c 'never reused or renumbered' docs/design/identity/IDENTITY-EVENTS.md prints 2 or more.
- The '## Review' section of docs/design/identity/IDENTITY-EVENTS.md holds one entry more than at the base commit, recording the joint review of this extension by DIRECTORY-003's builder and the card's lead, and every entry present at the base commit is unchanged.

**Files:**
- modify: docs/design/identity/IDENTITY-EVENTS.md

**Checklist:**
- C194 — docs/design/identity/IDENTITY-EVENTS.md extends lys/identity-event/v1 under a fresh joint review without changing its byte layout: session_started and session_ended are event kinds naming the agent and its responsible person, session_started carrying the launch record id, agent_reported is the agent's accepted report itself, and a session ended by the agent is an agent_reported followed by a session_ended naming it; no event carries a credential's value; the actor gains method codes 2, 3 and 4 with their issuer and subject stated, and no event is written under method 4 in this card, the change kinds gain 7 to 9, and every code is only ever appended.

### R3: Propose lys/session-start/v1 in the wire-format register, unratified

docs/design/WIRE-FORMATS.md SHALL gain one row in its decision log (section '## 5. Decision log'), numbered one above the highest D number in that log at <base>, proposing lys/session-start/v1 with status PROPOSED. The row SHALL name the format's fields: the tag, the agent id, the launch record id, the nonce, the audience (the directory's own identifier, the fifth signed field) and the freshness (the nonce is a challenge the directory issues once and admits for 60 seconds after issue). It SHALL give the byte layout by pointing at the section '## Sessions and their credentials' of docs/design/identity/DIRECTORY-CONTRACT.md, and SHALL state that no production agent signs under lys/session-start/v1 until this log shows it ratified, and that tests sign under it with disposable test keys only. Ratification is recorded in this log by the register's ratifier, as for every other format, and is not an act of this card. The row SHALL NOT mark the format RATIFIED, IMPLEMENTED or frozen, and the row's commit SHALL NOT change any other row, section or file.

**Acceptance:**
- From the repository root, on the build branch: grep -c '^| D.*lys/session-start/v1' docs/design/WIRE-FORMATS.md prints 1, and that one line contains PROPOSED and contains neither RATIFIED nor IMPLEMENTED.
- That line's D number is exactly one above the highest D number among the lines beginning '| D' in git show <base>:docs/design/WIRE-FORMATS.md.
- That line names the tag, the agent id, the launch record id, the nonce, the audience and the freshness, and contains the words no production agent signs.
- From the repository root, on the build branch, where <base> is the commit the row's build started from: git diff -U0 <base> HEAD -- docs/design/WIRE-FORMATS.md | grep -c '^-[^-]' prints 0, and git diff --name-only for R3's commit prints exactly one path, docs/design/WIRE-FORMATS.md.

**Files:**
- modify: docs/design/WIRE-FORMATS.md

**Checklist:**
- C195 — docs/design/WIRE-FORMATS.md carries one decision-log row proposing lys/session-start/v1, PROPOSED and not ratified, naming its fields (the tag, the agent id, the launch record id, the nonce, the audience and the freshness) and stating that no production agent signs under it until the log shows it ratified.

### R4: Attack the session credential and the report-back proof before any code

WHEN R1, R2 and R3 are complete, THE SYSTEM SHALL have them reviewed adversarially, before R5 starts, by a party other than their author, recorded in docs/design/identity/SESSION-CREDENTIAL-REVIEW.md with the commit it reviewed. Each attack is built as concrete bytes or a concrete request and gets one table row naming the refusal in R1 or the property that defeats it, with the word defeated, or recording the words not defeated. The attacks are: (a) an accepted proof replayed with the same agent, directory, launch record id and challenge; (b) a proof for one agent presented under another agent's directory id; (c) a proof signed by an Ed25519 key other than the enrolled key; (d) cross-protocol use: an lys/attestation/v2 or lys/delegation/v1 signature by the enrolled key offered as a proof, and a proof offered where an lys/attestation/v2 signature, a delegation or a certificate's signature is checked; (e) a non-canonical signature and a small-order public key, both refused under strict verification; (f) an ended session's credential presented again; (g) one agent's session credential presented as another agent's, the answer's agent id always coming from the session record; (h) a guessed credential, and whether the time taken by the lookup, keyed on the SHA-256 of the presented value, reveals anything about a stored credential; (i) the credential's value reaching Debug output, logs, error messages or events; (j) a valid proof intercepted in transit, or signed ahead of time, and submitted by a third party before its owner submits it; (k) a proof made for one directory replayed at another directory that holds the same agent directory id and the same enrolled key; (l) a launch-record swap: a proof over one launch record id presented against a challenge issued for another, and two fields shifted across the length prefixes so that the same bytes read as different field values; (m) an unauthenticated caller probing which agents exist and their state: challenge requests for a held and an unheld agent id, and unsigned and badly signed report-backs for an unheld agent, an agent with no enrolled key and each state-revealing case of R1, compared byte for byte, with the challenge store read before and after each. The review SHALL NOT be done by the author of R1, R2 and R3. IF an attack is not defeated, THEN R5 SHALL NOT start until R1, R2 or R3 is revised and the attack is re-run.

**Acceptance:**
- docs/design/identity/SESSION-CREDENTIAL-REVIEW.md names the commit of R1 to R3 it reviewed, and its reviewer differs from the author of R1's commit.
- The file's attack table has at least 13 rows, one for each of (a) to (m), and grep -c '| defeated |' on the file prints the number of table rows.
- Rows (a), (b), (c), (f), (g), (k), (l) and (m) each name the one refusal from R1's table that defeats the attack; rows (d), (e), (h), (i) and (j) each name the refusal or the property that defeats it; every row names the concrete input it was built with.

**Files:**
- create: docs/design/identity/SESSION-CREDENTIAL-REVIEW.md

**Checklist:**
- C196 — docs/design/identity/SESSION-CREDENTIAL-REVIEW.md records an adversarial review of the session credential and the report-back proof, by a party other than their author, with each of its thirteen attacks defeated, including replay, cross-directory, cross-agent, launch-record swap, cross-protocol use and unauthenticated probing, before any session code starts.

### R5: Issue a challenge, and start a session when the started agent reports back with its proof, verifying the signature first

WHEN an agent asks for a challenge naming an agent directory id and a launch record id, THE SYSTEM SHALL draw a new 32-byte challenge, hold it with that agent directory id, that launch record id and its issue instant, and answer the agent directory id, the launch record id and the challenge, whether or not the directory holds the agent named. WHEN a report-back names an agent directory id, a directory identifier, a launch record id, a challenge and a signature, THE SYSTEM SHALL first verify the signature with Ed25519 strict verification over R1's message under the agent's enrolled key, and IF the signature is absent, the directory does not hold the agent, the agent has no enrolled key, or the signature does not verify, THEN it SHALL refuse proof_invalid with one and the same refusal value in every one of those cases. WHEN the signature verifies, THE SYSTEM SHALL check in this order: the agent is not retired (else agent_retired, naming the state); its certificate has been issued (else certificate_not_issued, naming the act that answers, which is to issue this agent's certificate); the directory identifier is its own (else wrong_directory); the challenge appears in no session_started event (else challenge_reused); the challenge is held (else challenge_unknown); it was issued to this agent (else challenge_foreign_agent); the clock reads no more than 60 seconds after its issue instant (else challenge_expired, after which it is no longer held); and the launch record id equals the challenge's (else launch_record_mismatch). When every check passes, it SHALL, in one act, mark the challenge used by ceasing to hold it, create one session record pointing at the agent's directory id and carrying the launch record id, draw a new credential, and commit, before answering (P4, P5), one agent_reported event (R2) with reported started, the SHA-256 of R1's message for this report-back as its start proof SHA-256, the directory's clock reading as the time it received the report and the agent under method 2 as actor, and then one session_started event naming the agent and its responsible person, and answer the session id, the agent directory id and the credential's value. The challenge store exposes, for tests, a snapshot of its held challenges as bytes in issue order. Time is read from a clock passed in, never from the wall clock inside lys-identity. THE SYSTEM SHALL NOT create, register or change an agent, SHALL NOT commit any event for a challenge or a refusal, SHALL NOT hold a challenge after it is used, SHALL NOT change, consume or drop a challenge on a proof_invalid refusal, SHALL NOT cease to hold a challenge on any refusal but challenge_expired, SHALL NOT answer any refusal but proof_invalid before the signature has verified, SHALL NOT refuse on the agent being suspended, and SHALL NOT check that a launch record exists. It SHALL NOT keep the credential's value anywhere but in the one answer: the value is held in a Zeroizing buffer, its Debug prints AgentSessionCredential([redacted]), and only its SHA-256 is recorded. The module lives in crates/lys-identity/src/agent_session/, with mod.rs carrying only declarations and re-exports. The row encodes and decodes change kinds 7, 8 and 9 and actor method codes 2 and 3 as R2 writes them, in the event and provenance code DIRECTORY-003 lands, and changes the encoding of no existing kind or method.

**Acceptance:**
- d026_r5_ac1: for a test agent A with an issued certificate and an enrolled test key, in a test directory dir-1 that holds no launch record, a challenge is asked for naming A and the launch record id 0123456789abcdef0123456789abcdef, the test clock is advanced by exactly 60 seconds, and a report-back signed over A, dir-1, 0123456789abcdef0123456789abcdef and that challenge yields a session whose agent directory id equals A's and whose launch record id equals 0123456789abcdef0123456789abcdef. Its credential decodes from base64url to exactly 32 bytes. The log's size is exactly 2 greater: the first new event is agent_reported naming the session and A, reporting started, whose start proof SHA-256 equals the SHA-256 of R1's message over A, dir-1, 0123456789abcdef0123456789abcdef and that challenge, and whose received time equals the test clock's reading of 60 seconds after the challenge's issue; the second is session_started naming A and A's responsible person, its credential SHA-256 equals the SHA-256 of those 32 bytes, its challenge equals the challenge asked for, and the number of challenges held is 0.
- d026_r5_ac2: a challenge asked for naming the agent id agent-unheld, which the directory does not hold, answers agent-unheld, the launch record id and a 32-byte challenge, the same three members as for A; a report-back naming agent-unheld over that challenge, signed by a disposable test key, is refused proof_invalid. The directory list holds exactly 1 agent before them and exactly 1 after them, and the log's size is unchanged. After 5 further accepted report-backs for A, each over its own challenge, the list still holds exactly 1 agent and there are exactly 5 session records for A.
- d026_r5_ac3: exactly 6 refusals fire on dir-1 for correctly signed proofs by A, each creating no session and leaving the log's size unchanged: a report-back accepted in this test and submitted again gives challenge_reused; every other refusal is over a fresh challenge of its own: a proof presented when the test clock reads 61 seconds after its issue gives challenge_expired; a proof signed over dir-2 and naming dir-2 gives wrong_directory; a proof by agent A naming A over a challenge issued to agent B gives challenge_foreign_agent; a proof naming and signed over the launch record id fedcba9876543210fedcba9876543210, over a challenge issued for 0123456789abcdef0123456789abcdef, gives launch_record_mismatch; and a proof over 32 bytes never issued as a challenge gives challenge_unknown.
- d026_r5_ac4: exactly 4 refusals fire, each over a fresh challenge and each leaving the log's size unchanged: a correctly signed proof by a retired agent with an issued certificate gives agent_retired naming retired; a correctly signed proof by an agent with an enrolled key and no issued certificate gives certificate_not_issued, whose message contains issue this agent's certificate; a proof by A signed by an Ed25519 key other than A's enrolled key gives proof_invalid; and a proof naming an agent registered with no enrolled key, signed by a disposable test key, gives proof_invalid.
- d026_r5_ac5: an agent with an issued certificate, suspended through DIRECTORY-003's transition, reports back over a fresh challenge and is admitted: the answer carries a session id and a credential, and the log's size is exactly 2 greater.
- d026_r5_ac6: format!("{:?}", credential) equals AgentSessionCredential([redacted]), and neither the Debug output of the start answer nor the committed agent_reported and session_started events contain the credential's base64url value as a substring.
- d026_r5_ac7: nine report-backs are submitted with the signature field empty: naming agent-unheld; naming an agent with no enrolled key; by the retired agent; by the agent with no issued certificate; naming dir-2; over a challenge already used; over 32 bytes never issued; over a challenge issued to agent B; and over a challenge whose issue was 61 seconds before. Exactly 9 refusals fire, every one proof_invalid, all 9 refusal values are equal and their Display strings are byte-identical, the challenge store's snapshot bytes after each equal its bytes before it, and the log's size is unchanged. The correctly signed forms of the last seven, submitted afterwards in the same order, give agent_retired, certificate_not_issued, wrong_directory, challenge_reused, challenge_unknown, challenge_foreign_agent and challenge_expired. A correctly signed proof by the retired agent naming dir-2 over a used challenge gives agent_retired.
- d026_r5_ac8: one event of each of DIRECTORY-003's six change kinds, encoded by the code at the row's base commit and kept in the test as fixed bytes, decodes under the row's code and re-encodes to bytes identical to those fixed bytes, all 6 compared.
- d026_r5_ac9: an agent_reported event with actor method 2 reporting started, a session_started event with actor method 2, an agent_reported event with actor method 3 reporting ended, a session_ended event with how it ended agent_reported, actor method 3 and a report event id, and a session_ended event with how it ended operator_stopped and actor method 1 each encode with change-kind code 9, 7, 9, 8 and 8 respectively and decode back to an event equal to the one encoded, all 5 compared, and none of the five has a member named session id inside its actor map.
- cargo test -p lys-identity --test agent_session_start prints 'test result: ok. 9 passed; 0 failed'.

**Files:**
- create: crates/lys-identity/src/agent_session/mod.rs
- create: crates/lys-identity/src/agent_session/record.rs
- create: crates/lys-identity/src/agent_session/credential.rs
- create: crates/lys-identity/src/agent_session/challenge.rs
- create: crates/lys-identity/src/agent_session/proof.rs
- create: crates/lys-identity/src/agent_session/events.rs
- create: crates/lys-identity/src/agent_session/error.rs
- create: crates/lys-identity/src/agent_session/start.rs
- create: crates/lys-identity/tests/agent_session_start.rs
- modify: crates/lys-identity/src/lib.rs
- modify: crates/lys-identity/src/event.rs
- modify: crates/lys-identity/src/encoding.rs
- modify: crates/lys-identity/src/provenance.rs
- modify: crates/lys-identity/Cargo.toml
- modify: Cargo.lock

**Checklist:**
- C197 — A challenge is issued once, for one agent id and one launch record id, with the same answer whether or not the directory holds the agent: a proof arriving 60 seconds after its issue is admitted, and one arriving 61 seconds after is refused challenge_expired.
- C198 — A report-back from an agent whose certificate is issued, signed by its enrolled key, marks the challenge used in the same act that creates one session record pointing at the agent's directory id, commits one session_started event and returns the credential's value once, the directory keeping only its SHA-256.
- C199 — Starting a session never creates an agent: a report-back naming an agent the directory does not hold is refused proof_invalid, and the directory's agent count is unchanged by any number of session starts.
- C200 — The report-back's signature is verified before anything else: an unsigned or badly signed report-back, or one naming an agent the directory does not hold or one with no enrolled key, is refused proof_invalid with one and the same body and leaves the challenge store byte-identical, and only a correctly signed proof receives the state-revealing refusals, in their order.
- C201 — A correctly signed report-back is refused by name, committing no event and creating no session, for a reused challenge (challenge_reused), an expired one (challenge_expired), one issued to another agent (challenge_foreign_agent), one never issued (challenge_unknown), a proof naming another directory (wrong_directory), a launch record id that differs from the challenge's (launch_record_mismatch), a retired agent (agent_retired) and an agent whose certificate is not issued (certificate_not_issued, naming the act of issuing it); a suspended agent is not refused.

**Stories:**
- S81 (Responsible person, Signs in and provisions agents under their own authority) — As a responsible person, I want each session of my agent kept as its own record under that one agent, so that starting another session never makes another agent.

### R6: Check a presented session credential and answer its enduring agent

WHEN a session presents a credential, THE SYSTEM SHALL check in this order: the value decodes from base64url to 32 bytes and a session record's credential hash equals the SHA-256 of those bytes (else credential_unknown); and that session is open (else session_ended, naming the session id). When both checks pass, it SHALL answer that record's agent directory id and session id. The first check that fails is the one refusal answered. THE SYSTEM SHALL NOT commit any event for a presentation, accepted or refused, SHALL NOT end the session or change the agent on a refusal, and SHALL NOT refuse an open session's credential on the agent's lifecycle state. The agent directory id it answers comes only from the session record, never from the caller.

**Acceptance:**
- d026_r6_ac1: with two open sessions of agent A holding credentials c1 and c2, presenting c1 answers A's directory id and the first session's id, and presenting c2 answers A's directory id and the second session's id. The log's size after 4 presentations equals its size before them.
- d026_r6_ac2: presenting 32 random bytes never issued gives credential_unknown, and presenting the string not-a-credential gives credential_unknown. Exactly 2 refusals fire, and the log's size is unchanged.
- d026_r6_ac3: after A is retired through DIRECTORY-003's transition, presenting c1 still answers A's directory id and the first session's id, and the log's size after that presentation equals its size after the transition.
- cargo test -p lys-identity --test agent_session_present prints 'test result: ok. 3 passed; 0 failed'.

**Files:**
- create: crates/lys-identity/src/agent_session/present.rs
- create: crates/lys-identity/tests/agent_session_present.rs
- modify: crates/lys-identity/src/agent_session/mod.rs

**Checklist:**
- C202 — A presented session credential is answered with the agent directory id and session id from its session record, and a presentation commits no event.

**Stories:**
- S80 (Started agent, Reports back to the directory and presents its session credential) — As a started agent, I want to present my session credential to the directory, so that it confirms I am my enduring agent in this session.

### R7: End a session by the agent's report or the administrator's stop, and prove the second session

WHEN the agent reports its end with its session credential, THE SYSTEM SHALL check the credential in R6's order and answer R6's refusal for the first check that fails, and otherwise commit one agent_reported event with reported ended, the directory's clock reading as the time it received the report, no start proof SHA-256 and the agent under method 3 as actor, then one session_ended event with how it ended agent_reported, the agent under method 3 as actor and that agent_reported event's id as its report event id, and mark the session the credential names ended. WHEN a caller stops a session by its session id, THE SYSTEM SHALL check in this order: the caller is the directory's configured administrator, admitted as DIRECTORY-003 R3 admits it (else DIRECTORY-003's admission refusal); the session id names a session (else session_unknown); and the session is open (else session_ended, naming the session id). When every check passes, it SHALL commit one session_ended event with how it ended operator_stopped and the administrator's person id and provenance as actor, and mark the session ended. On every refusal THE SYSTEM SHALL NOT commit an event. A session credential SHALL NOT end any session but the one whose record holds its hash. Ending one session SHALL NOT end or change another session of the same agent, and SHALL NOT change the agent. The session's state is rebuilt from its events when the store is opened (P5), so an ended session stays ended across a reopen. Nothing ends a session by time.

**Acceptance:**
- d026_r7_ac1: agent A starts session s1 with credential c1, then starts s2 with credential c2 while s1 is open, and both starts are accepted. c1 differs from c2 and s1 differs from s2. Presenting c1 and presenting c2 each answer A's directory id. A reports its end with c1, the log grows by exactly 2: the first new event is agent_reported naming s1 and A, reporting ended, and the second is session_ended naming s1, A, A's responsible person, actor A, agent_reported and, as its report event id, the first new event's id. Presenting c1 then gives session_ended naming s1, and presenting c2 answers A's directory id and s2. Across the test exactly 1 presentation is refused and exactly 3 are accepted.
- d026_r7_ac2: A starts s3 with credential c3. The configured test administrator stops s3, the log grows by exactly 1, and the new event is session_ended naming s3, actor the administrator's person id, and operator_stopped. Presenting c3 then gives session_ended naming s3, and presenting c2 answers A's directory id and s2.
- d026_r7_ac3: exactly 4 refusals fire, each leaving the log's size unchanged: reporting an end again with c1 gives session_ended naming s1; the administrator stopping s1 gives session_ended naming s1; the administrator stopping the session id 00000000000000000000000000000000 gives session_unknown; and a signed-in caller who is not the configured administrator stopping s2 gets DIRECTORY-003's admission refusal, and presenting c2 afterwards answers A's directory id and s2.
- d026_r7_ac4: after the store is closed and reopened from its log, presenting c1 gives session_ended naming s1, and presenting c2 answers A's directory id and s2.
- cargo test -p lys-identity --test agent_second_session prints 'test result: ok. 4 passed; 0 failed'.

**Files:**
- create: crates/lys-identity/src/agent_session/end.rs
- create: crates/lys-identity/tests/agent_second_session.rs
- modify: crates/lys-identity/src/agent_session/mod.rs

**Checklist:**
- C203 — A second session of the same agent, started while the first is open, presents the same agent directory id with a different credential, and after the agent reports the first session's end its credential is refused session_ended, naming that session, while the second is still accepted.
- C204 — The directory's configured administrator stops a session as one signed event naming the administrator as actor, after which its credential is refused session_ended, naming the session; any other caller's stop is refused by DIRECTORY-003's admission.

**Stories:**
- S81 (Responsible person, Signs in and provisions agents under their own authority) — As a responsible person, I want each session of my agent kept as its own record under that one agent, so that starting another session never makes another agent.
- S82 (Operator, Installs and runs the standalone identity product) — As the operator, I want to stop a session on the directory, so that its credential is refused even when the agent crashed and never reported its end.

### R8: Expose the challenge, report-back, presentation, end and stop as directory routes, admitting an agent only to its own session

THE SYSTEM SHALL add five routes to the directory service, each calling R5, R6 or R7 and answering its refusals by name in the refusal form DIRECTORY-003's routes use, in the order those rows check them. POST /sessions/challenge takes a JSON body with agent_id and launch_record_id and answers 201 with agent_id, launch_record_id and challenge (base64url), for an agent_id the directory holds and for one it does not hold alike. POST /sessions takes a JSON body with agent_id, directory, launch_record_id, challenge and signature (base64url), and answers 201 with session_id, agent_id and credential. Its refusals are 401 proof_invalid, answered with one and the same status and body bytes in every case R5 refuses proof_invalid, then 409 agent_retired, 409 certificate_not_issued, 401 wrong_directory, 401 challenge_reused, 401 challenge_unknown, 401 challenge_foreign_agent, 401 challenge_expired and 401 launch_record_mismatch. POST /sessions/present takes a JSON body with credential and answers 200 with agent_id and session_id. Its refusals are 401 credential_unknown and 401 session_ended with session_id. POST /sessions/end takes a JSON body with credential and answers 200 with session_id and the state ended, with the same refusals as present. POST /sessions/{session_id}/stop is admitted only for the configured administrator, as DIRECTORY-003 R3 admits it, and answers 200 with session_id and the state ended. Its refusals are DIRECTORY-003's admission refusal for any other caller, 404 session_unknown and 409 session_ended. POST /sessions and POST /sessions/end are the one exception to DIRECTORY-003's administrator admission that P9's appended line records: the agent authenticated by its enrolled key creates its own session, and the credential issued to it ends that session only. A session credential admits nothing else: presented as a bearer value in the Authorization header on the stop route or on any of DIRECTORY-003's mutation routes, it gets DIRECTORY-003's admission refusal and changes nothing. The service reads the directory's identifier as the member directory_id of its JSON configuration, which crates/lys-identity-server/src/config.rs reads, validates and documents beside the service's other configured values, and passes R5 that identifier and its clock. IF directory_id is absent or empty, THEN loading the configuration SHALL fail with the service's configuration refusal naming directory_id, and the service SHALL NOT start. THE SYSTEM SHALL NOT write a credential's value to a log line, a trace or any answer other than the 201 of POST /sessions. It SHALL NOT answer a report-back differently by which proof_invalid case it is, and SHALL NOT answer a challenge request differently by whether the agent is held. It SHALL NOT offer a route that renders a start command, creates an agent, or reads back a credential. The route tests run in the directory's contract-test crate (tests/identity_contract), whose harness starts the service in process behind DIRECTORY-003's fake issuer and gains the configured directory_id; crates/lys-identity-server gains no test and no dev-dependency.

**Acceptance:**
- d026_r8_ac1: for one test agent, POST /sessions/challenge answers 201 with a challenge, and POST /sessions over it answers 201; done twice, the two answers carry two different credential values and the same agent_id. POST /sessions/present with the first credential answers 200 with that agent_id.
- d026_r8_ac2: POST /sessions/end with the first credential answers 200 with state ended. POST /sessions/present with the first credential then answers 401 with the refusal session_ended and the first session_id, and with the second credential answers 200.
- d026_r8_ac3: POST /sessions/{session_id}/stop by the configured test administrator on the second session answers 200. Stopping it again answers 409 session_ended. Stopping 00000000000000000000000000000000 answers 404 session_unknown. Stopping by a signed-in caller who is not the configured administrator answers DIRECTORY-003's admission refusal.
- d026_r8_ac4: POST /sessions/challenge for the agent id agent-unheld answers 201 with the members agent_id, launch_record_id and challenge, the same members as for the held agent; POST /sessions for agent-unheld answers 401 proof_invalid; and the directory list's agent count is unchanged. Submitting the body of ac1's first POST /sessions again answers 401 challenge_reused. Loading a configuration file identical to the test service's but with no directory_id member fails with the configuration refusal whose reason names directory_id, and loading one whose directory_id is the empty string fails the same way.
- d026_r8_ac5: with the service's tracing captured at trace level through all of ac1 to ac4, the captured output contains neither credential value as a substring.
- d026_r8_ac6: agents A and B each start a session over the routes, A holding credential cA and B's session being sB. A ends its own session with POST /sessions/end and cA, answered 200. Before that end, exactly 3 refusals fire: POST /sessions/{sB}/stop with cA as the Authorization bearer value answers DIRECTORY-003's admission refusal and sB is listed open afterwards; DIRECTORY-003's agent registration route called with cA as the Authorization bearer value answers DIRECTORY-003's admission refusal and the directory list's agent count is unchanged; and POST /sessions naming B over a challenge issued to B, signed by A's enrolled key, answers 401 proof_invalid and creates no session.
- d026_r8_ac7: POST /sessions with an empty signature for each of the nine cases of d026_r5_ac7 answers 401, and the nine answer bodies are byte-identical to one another and to the body answered for a proof signed by a key other than the agent's enrolled key.
- cargo test -p identity-contract --test agent_session_routes prints 'test result: ok. 7 passed; 0 failed'.

**Files:**
- create: crates/lys-identity-server/src/agent_session_routes.rs
- create: tests/identity_contract/tests/agent_session_routes.rs
- modify: crates/lys-identity-server/src/routes.rs
- modify: crates/lys-identity-server/src/lib.rs
- modify: crates/lys-identity-server/src/config.rs
- modify: tests/identity_contract/src/harness.rs

**Checklist:**
- C205 — The directory service answers the challenge, the report-back, the presentation, the agent's end and the administrator's stop on five routes, answers every proof_invalid case with the same status and body bytes, and no log line, trace or answer other than the report-back's carries a credential's value.
- C206 — An agent authenticated by its enrolled key creates and ends its own session record and nothing else: its session credential presented on another agent's session stop and on DIRECTORY-003's agent registration is refused by DIRECTORY-003's admission, and a report-back naming another agent signed by its key is refused proof_invalid.

**Stories:**
- S80 (Started agent, Reports back to the directory and presents its session credential) — As a started agent, I want to present my session credential to the directory, so that it confirms I am my enduring agent in this session.

### R9: List an agent's sessions with their states for the configured administrator

WHEN an agent's sessions are listed by its directory id, THE SYSTEM SHALL answer every session record of that agent in the order they started, each with exactly four members: session_id, launch_record_id, state (open while the session is open, ended once it has ended) and ended_how (agent_reported, operator_stopped, none while open). GET /agents/{agent_id}/sessions SHALL answer that list as JSON with 200 only to the directory's configured administrator, admitted as DIRECTORY-003 R3 admits it by the configured issuer and subject. It SHALL check in this order: the caller is that administrator (else DIRECTORY-003's admission refusal, naming no agent and no session); the directory holds the agent (else 404 unknown_agent). The list SHALL NOT carry a credential's value or its hash, and listing SHALL NOT commit an event. The route SHALL NOT answer the list to any caller other than the configured administrator, the agent's responsible person included. THE SYSTEM SHALL carry the listing command, run against the in-process directory only, written out in full and run from the repository root: cargo test -p identity-contract --test agent_session_routes d026_r9_ac4_listing_command -- --exact --nocapture. The command runs the directory in process on a loopback port, configured as DIRECTORY-003 R3 configures its administrator with a disposable test issuer and subject. It obtains the administrator's credential the one way DIRECTORY-003 R3 admits it: it completes the directory's OIDC sign-in as that configured issuer and subject against DIRECTORY-003's fake issuer (tests/identity_contract/src/fake_issuer.rs), through the contract-test harness's sign-in (tests/identity_contract/src/harness.rs), and holds the directory's sign-in session that the sign-in answers. It then registers test agent A, starts s1, s2 and s3, has A report the end of s1, has the administrator stop s3, and sends GET /agents/{A}/sessions three times: with the administrator's sign-in session, with the sign-in session of a second disposable subject of the same fake issuer who is not the configured administrator, and with no sign-in. As each of the two refused callers it also sends POST /sessions/{s2}/stop, the administrator's stop route of R8, and keeps the status that route answers that caller, without printing it. For each listing call it writes one line holding the caller (administrator, other_signed_in or no_sign_in) and the HTTP status, followed by one line for each session entry answered, holding its session_id, state and ended_how separated by single spaces, and every line it writes begins with the fixed prefix 'd026-listing: '. The test writes those lines into its own output buffer, asserts its measurements on that buffer, and then prints the buffer, so the lines cargo test adds itself carry no such prefix and are not among them. The command SHALL NOT print a session credential's value, its hash or a sign-in session's value.

**Acceptance:**
- d026_r9_ac1: after agent A starts s1, s2 and s3, reports the end of s1 and has s3 stopped by the administrator, listing A answers exactly 3 entries in the order s1, s2, s3, with states ended, open, ended and how each ended agent_reported, none, operator_stopped. The log's size is unchanged by the listing.
- d026_r9_ac2: GET /agents/{A}/sessions by the configured test administrator answers 200 with the same 3 entries, whose members are exactly session_id, launch_record_id, state and ended_how, and for an agent id the directory does not hold it answers 404 unknown_agent.
- d026_r9_ac3: GET /agents/{A}/sessions by a signed-in person who is not the configured administrator (a second disposable subject; in step 1 A's responsible person is the administrator who registered it) gets DIRECTORY-003's admission refusal and no session entry; by a caller with no sign-in it gets the same refusal; and by a caller with no sign-in for an agent id the directory does not hold it gets the same admission refusal, not 404.
- d026_r9_ac4_listing_command: of the listing command's output, exactly 6 lines begin with 'd026-listing: ', and the test asserts that count on its own output buffer before printing it; piped through grep -c '^d026-listing: ' the command's output prints 6. Those 6 lines, with the prefix removed, are in this order: 'administrator 200'; three session lines, for s1, s2 and s3, ending 'ended agent_reported', 'open none' and 'ended operator_stopped'; 'other_signed_in' followed by a status; and 'no_sign_in' followed by a status. The status printed after 'other_signed_in' equals the status POST /sessions/{s2}/stop answered the same second subject in the same run, and the status printed after 'no_sign_in' equals the status that stop answered the caller with no sign-in; neither printed status is 200, and s2 is listed open after both refused stops. The test counts 3 session lines after the administrator's line and 0 after each other caller's line, and asserts that none of the three session credentials, their hashes and the two sign-in sessions drawn in the run appears in the output.
- cargo test -p lys-identity --test agent_session_list prints 'test result: ok. 1 passed; 0 failed', and cargo test -p identity-contract --test agent_session_routes prints 'test result: ok. 10 passed; 0 failed'.

**Files:**
- create: crates/lys-identity/src/agent_session/list.rs
- create: crates/lys-identity/tests/agent_session_list.rs
- modify: crates/lys-identity/src/agent_session/mod.rs
- modify: crates/lys-identity-server/src/agent_session_routes.rs
- modify: tests/identity_contract/tests/agent_session_routes.rs

**Checklist:**
- C207 — An agent's sessions are listed by its directory id, in the order they started, with each session's state and how it ended, on a route only the directory's configured administrator is answered on, by a command written out in full, and every other caller is refused by DIRECTORY-003's admission.

**Stories:**
- S83 (Operator, Installs and runs the standalone identity product) — As the operator, I want to list an agent's sessions with their states, so that I can see which of its session credentials are still accepted.

### R10: Refuse a new session to an agent whose certificate has been revoked, and end no open session

WHEN a correctly signed report-back passes R5's check that the agent's certificate has been issued, THE SYSTEM SHALL next check that the agent's most recently issued certificate is not in the revoked set of the certificate revocation card's fold (crates/lys-identity/src/revocation/fold.rs) over the directory's certificate log, read at its full extent (else certificate_revoked, naming the agent directory id), before R5's remaining checks. IF a report-back is refused certificate_revoked, THEN THE SYSTEM SHALL commit no event, create no session and hold its challenge as R5 holds a challenge after a refusal. POST /sessions SHALL answer certificate_revoked as 409. THE SYSTEM SHALL NOT answer certificate_revoked to a report-back whose signature has not verified, SHALL NOT end, mark or change any open session of that agent on a revocation, SHALL NOT refuse an open session's credential at presentation on a revocation, SHALL NOT keep a list of revoked certificates of its own, and SHALL NOT append, rewrite or read past the extent of the certificate log other than through that card's fold.

**Acceptance:**
- d026_r10_ac1: an agent whose certificate has been issued and then revoked through the certificate revocation card's append, reporting back over a fresh challenge with a proof signed by its enrolled key, is refused certificate_revoked naming its agent directory id, and the log's size and the number of its session records are unchanged.
- d026_r10_ac2: agent A starts session s1 with credential c1; A's certificate is then revoked; A's next report-back, over a fresh challenge and signed by the key enrolled for the revoked certificate, is refused certificate_revoked; presenting c1 afterwards answers A's directory id and s1, and listing A answers s1 with state open.
- From the repository root, on the build branch: rg -n 'revocation::fold' crates/lys-identity/src/agent_session/revoked.rs prints at least one line, and rg -n -i 'revoked_(set|list)\s*[:=]' crates/lys-identity/src/agent_session prints nothing.
- cargo test -p lys-identity --test agent_session_revoked prints 'test result: ok. 2 passed; 0 failed', and cargo test -p identity-contract --test agent_session_routes prints 'test result: ok. 11 passed; 0 failed', its eleventh test being d026_r10_ac3: POST /sessions for a revoked certificate's agent answers 409 certificate_revoked.

**Files:**
- create: crates/lys-identity/src/agent_session/revoked.rs
- create: crates/lys-identity/tests/agent_session_revoked.rs
- modify: crates/lys-identity/src/agent_session/mod.rs
- modify: crates/lys-identity/src/agent_session/start.rs
- modify: crates/lys-identity/src/agent_session/error.rs
- modify: crates/lys-identity-server/src/agent_session_routes.rs
- modify: tests/identity_contract/tests/agent_session_routes.rs

**Checklist:**
- C208 — A report-back from an agent whose certificate has been revoked is refused certificate_revoked, creating no session, and the agent's open sessions stay open.

**Stories:**
- S81 (Responsible person, Signs in and provisions agents under their own authority) — As a responsible person, I want each session of my agent kept as its own record under that one agent, so that starting another session never makes another agent.

## Boundaries

- Only the files listed in this brief's requirements change. A row that needs a file outside its wall stops and names it, and the card's lead approves a brief revision before that file is edited (CN9).
- Starting a session never creates, registers or changes an agent, and the agent record gains no member. A session never widens the agent's grants (ADR-003) and never changes its lifecycle state (ADR-011).
- The session credential is the random value ADR-079 records, never a per-session lys certificate, a lys/delegation/v1 SpeaksFor delegation or an ADR-001 handle. The agent's one certificate and what it carries stay the typed capability claim card's, unchanged (ADR-008).
- lys-core, lys-log-store, lys/attestation/v2, lys/delegation/v1 and every other shipped or drafted tag and content type are unchanged, and SpeaksFor gains no consumer. The only new tag is lys/session-start/v1 (ADR-080): it is proposed, not ratified, in docs/design/WIRE-FORMATS.md, tests sign under it with disposable test keys only, no production agent signs under it until that register's decision log shows it ratified, and it is frozen once an agent signs under it.
- A credential's value appears only in the one answer that issues it: never in a start command, a launch record, an event, a log, a trace, an error, a fixture, a test assertion message or a document. Test credentials are drawn at run time and are never written into a file.
- No refusal but proof_invalid is answered before the report-back's signature verifies, and no unsigned or invalid report-back changes, consumes or drops a challenge. A session credential admits its own session's presentation and end and no other mutation (P9's appended line).
- Nothing ends a session by time, and no act but the agent's reported end and the administrator's stop ends one. Lapsing a stale session, accepting an agent's report that its session is running, ending an agent's open sessions on retirement, revocation or emergency stop, refusing a suspended agent, checking that a launch record exists, the sessions screen, the agent's file, listing for the responsible person, running the listing command against a deployed directory, a signed session credential, an engine's report-back client, and any change to the Cambium, Rauthy fork or Aion repositories are not in this brief.
- docs/design/directory/DESIGN.md and design.json gain only the lines appended beneath the 'Road step 2 onward' non-goal, P6 and P9, and the line appended after CN1 recording that CN1 binds only the documents-only planning briefs. STATEMENT-2026-09-22.md, the IDENTITY-001 files and every requirement, wall and acceptance line of DIRECTORY-003, the provision card and the typed capability claim card are read, never changed.
- Development isolation: disposable test identities, test keys and a test anchor only; no production token, key or sign-in (CN2). A live demonstration is a verification step a person performs after a row lands, never an acceptance criterion (CN5). No row is dispatched before the sign-off on the card.

## Verification

- From the repository root: sh scripts/design/gate.sh exits 0, and python3 scripts/design/validate.py docs/design/roadmap.json and python3 scripts/design/validate.py docs/design/decisions.json each exit 0.
- From the repository root, at the gate venue after R10: cargo fmt --all; cargo clippy --all-targets --all-features -- -D warnings; cargo clippy --all-targets -- -D warnings; cargo test --workspace --all-features; cargo doc --no-deps --all-features; cargo doc --no-deps, all clean, with git status showing no change from cargo fmt (CN8).
- cargo test -p lys-identity --test agent_session_start, --test agent_session_present, --test agent_second_session, --test agent_session_list and --test agent_session_revoked, and cargo test -p identity-contract --test agent_session_routes, print 9, 3, 4, 1, 2 and 11 passed respectively, with 0 failed.
- Each code row's first commit adds only its test file, and its test fails naming at least one of its d026 criteria. Its second commit passes. The review reads both commits.
- From the repository root, on the build branch, where <base> is the commit the row's build started from: git diff --stat <base> HEAD -- crates/lys-core crates/lys-log-store docs/design/identity/STATEMENT-2026-09-22.md docs/design/identity/briefs prints nothing, and rg -n 'SpeaksFor' crates/lys-identity crates/lys-identity-server prints nothing.
- rg -n 'SystemTime::now|Utc::now|Instant::now' crates/lys-identity/src/agent_session crates/lys-identity/tests/agent_session_start.rs crates/lys-identity/tests/agent_session_present.rs crates/lys-identity/tests/agent_second_session.rs crates/lys-identity/tests/agent_session_list.rs crates/lys-identity/tests/agent_session_revoked.rs prints nothing.
- After the card lands, a person runs the listing command from the repository root, as R9 writes it out: cargo test -p identity-contract --test agent_session_routes d026_r9_ac4_listing_command -- --exact --nocapture, and, among the lines beginning 'd026-listing: ', reads each of the test agent's three sessions with its state under 'administrator 200', and DIRECTORY-003's admission refusal with no session line under 'other_signed_in' and under 'no_sign_in' (CN5).

