# lys-core — what was asked, what it means, and what was written

## The words, as they were typed

The lys-core design on main is three hand-written documents, docs/design/lys-core/DESIGN.md, CHECKLIST.md and USER-STORIES.md, holding checklist items C1 to C65 and user stories S1 to S23. The design method renders those documents from design.json, checklist.json and stories.json, and the cluster has none of the three. The first lys-core brief to add a design.json makes the gate render the cluster, and the render would replace the hand-written documents with the JSON's much smaller scope, so the older design would be lost from the record. This card carries the whole of the hand-written design into the three JSON documents first, so that the render reproduces it.

Every checklist item keeps its C id and its text, every user story keeps its S id and its text, and every section of DESIGN.md is carried into design.json in the fields the design schema gives it. Nothing is reworded, merged, dropped or renumbered. Where a sentence of the hand-written documents has no field in the schema, the brief names it and places it in the nearest field that holds free text, and says so in its dev record. The rendered DESIGN.md, CHECKLIST.md and USER-STORIES.md are committed from the render.

Acceptance is that scripts/design/validate.py accepts the three JSON documents; that scripts/design/gate.sh passes on the card's final tree; that checklist.json holds C1 to C65 and stories.json holds S1 to S23, each id once; that a script in the brief compares every C and S text in the rendered documents with the text on main at the commit this card starts from, and prints zero differences; and that every difference between the rendered DESIGN.md and main's DESIGN.md is listed in the dev record with its reason, and each is a difference of layout only.

Not in scope are any new checklist item, user story or design decision, and any change to code. The self-signed card qVVhhP2b's brief, run 5f185fd4, waits for this card to land, and it adds its own rows after the carried ones.

Filed by Archie, lead for the identity line, on 27 September 2026, in answer to round 2 of run 5f185fd4, against lys main 7b536253.

## What the survey found, and its angles

The lys-core design cluster on main is three hand-written markdown files with no JSON sources. The first brief that adds a design.json turns on the design gate's render-and-compare for the cluster, and the hand-written text would then be overwritten. This card writes design.json, checklist.json and stories.json so that the render reproduces the existing design with every C and S id and text unchanged, commits the rendered markdown, and records every place where the render's layout differs from main. Nothing new is designed and no code changes.

### What the tree holds

- `docs/design/lys-core/DESIGN.md` — 255 lines. Sections: Intention, Problem (4 bullets), Solution (### D1 to D8, 1 code fence, 2 blockquote notes), Goals (numbered 1 to 6), Non-Goals (7 bullets), Structure (an 84-line fenced tree with 73 entries under crates/lys-core/ and crates/lys/, then 1 trailing paragraph) and Constraints (9 bullets with no ids). The frontmatter title is quoted and there is no '> **Cluster:**' line.
- `docs/design/lys-core/CHECKLIST.md` — 101 lines, 8 sections, C1 to C65 each once, 61 ticked and C61 to C64 open. It has a 3-paragraph intro blockquote and two italic annotations (above C38 and above C58) that no checklist.json field can hold. C61 to C64 end in italic tails that stay inside the item line.
- `docs/design/lys-core/USER-STORIES.md` — 55 lines, 4 persona headings, S1 to S23 each once. Only 1 of the 4 headings ('Norn Agent Runtime — Signing Session History (primary consumer)') has the ' — ' that render_stories always writes between name and role.
- `scripts/design/render-cluster.py` — Fixes the rendered shape. Checklist: '# Lys-Core — Checklist', then per section '## name' and '- [x] **Cn** — text', with no free-text slot. Stories: '## {name} — {role}'. Design: unquoted title, an added '> **Cluster:**' line, goals rendered as '- ' bullets, constraints as '**CNn** —', structure as a Path|Note|Brief table. Empty principles, decisions and inventory are skipped.
- `scripts/design/schemas/design.schema.json` — All 12 fields are required. problem and solution are strings. principles need P ids, decisions only ADR-nnn ids, constraints CN ids, and structure {path, note, brief} rows. gate is optional. There is no field for a free paragraph after a structure table.
- `scripts/design/schemas/checklist.schema.json and stories.schema.json` — additionalProperties false. A checklist item is {id, text, done} and a section is {name, items}. A persona is {name, role, stories}. Neither schema has a note or preamble field.
- `scripts/design/gate.sh` — Only clusters that have a design.json are measured. For each one it runs validate.py, then check-coverage.py, then re-renders into a temporary copy and compares every *.md under the cluster, including briefs/*.md, byte for byte with the committed files.
- `scripts/design/check-coverage.py` — Fails any checklist item or story that no brief in the cluster claims, and any R# file path missing from design.json structure. So a lys-core design.json with no brief fails 88 times (65 + 23).
- `scripts/design/schemas/brief.schema.json` — The brief id pattern is ^[A-Z]+-\d{3}$, which refuses 'LYS-CORE-001'. requirements[].files needs create, modify and delete arrays, and those paths are checked against structure.
- `docs/design/roadmap.json` — No item links cluster 'lys-core'. The card's row needs links.cluster = lys-core and the brief id.
- `docs/design/directory/briefs/DIRECTORY-008.json` — The nearest precedent, a docs-only card brief. It lists its docs/design/directory/*.json and *.md files in R# files, and the directory design.json structure lists those paths.

### What was already decided

- docs/design/lys-core (DESIGN.md D1–D8) — The extraction plus CLI design. D1 to D8 are solution subsections, not ledger ADRs, so they go into the solution string and not into design.json decisions.
- docs/design/WIRE-FORMATS.md D1–D4 — The checklist and DESIGN.md notes say these ratified decisions are authoritative wherever the lys-core files disagree. The carried text keeps those pointers unchanged.
- scripts/design/SOURCE.md — The renderer, validator and schemas are copied from the design-system method at 3c3bac7. Changing them is a change to code, which the words exclude.
- CLAUDE.md 'A test needs a second party' — The text-comparison script is the second party for 'nothing reworded'. It has to key on main's bytes at 7b536253, not on the JSON the author wrote, and it has to count the items it compared (65 + 23).
- ADR-001..ADR-018 — None of the ledger decisions is about lys-core. They cover secrets, identity, home and project, so the lys-core design.json decisions array stays empty.

### What was measured

- Checklist items on main: 65 (C1–C65, each once), in 8 sections; 61 ticked, 4 open (C61–C64)
- User stories on main: 23 (S1–S23, each once), under 4 persona headings
- Persona headings that already have the ' — ' the renderer writes: 1 of 4
- Lines that differ when main's checklist items are rendered in memory with render_checklist: 11 removed lines, 0 added: the 3-paragraph intro blockquote plus its blank lines, and the two italic annotations. All 65 item lines and 8 headings match byte for byte.
- Hand-written file sizes: DESIGN.md 255 lines, CHECKLIST.md 101 lines, USER-STORIES.md 55 lines
- DESIGN.md characters per section: Solution 18307, Structure 6054, Problem 1291, Goals 1182, Constraints 1102, Intention 1097, Non-Goals 926
- Structure tree: 84 fenced lines, 73 entries, 2 roots, 1 trailing paragraph with no schema field
- Solution subsections, code fences, blockquote lines: 8, 1, 2
- Constraints with ids on main: 0 of 9; the schema requires a CN id on each
- JSON documents and briefs in docs/design/lys-core: 0 and 0; there is no briefs/ directory
- Coverage failures if design.json lands with no lys-core brief: 88 (65 checklist + 23 stories)
- Roadmap items linking cluster lys-core: 0 of 11
- gate.sh on main 7b53625: exit 0; clusters measured: directory, home, secrets
- Relative links in the lys-core documents: Targets docs/ROADMAP.md, docs/PEN-REGISTRATION.md, docs/REVIEW-23-07.md and docs/design/WIRE-FORMATS.md all exist
- Tracked source files under crates/lys-core/src and crates/lys/src: 112, of which 22 are in paths the carried structure tree does not name (delegation/, receipt/, bundle/, ca/request.rs, keys/ssh.rs)

### What it means for the other projects

- aion — Run 5f185fd4 (card qVVhhP2b) is held until this card lands. Its brief then takes the next lys-core brief number and C66+/S24+ after the carried ids, and the card goes through the brief_card → card_build_v3 → src_pr → src_land chain.
- method — The method's renderer and schemas (tools/design-system, copied at 3c3bac7) have no preamble or note field for checklists and always write 'name — role' for personas. This card works inside that and changes nothing there. A method change would be the proper home for checklist notes.
- cambium — The card sits on the Cambium board and runs through its aion chain. No code or document in cambium changes.

### What it requires

- docs/design/lys-core/design.json, checklist.json and stories.json exist and python3 scripts/design/validate.py docs/design/lys-core reports every document OK
- checklist.json holds exactly C1..C65 and stories.json exactly S1..S23, each id once
- Each item's done flag equals main's tick: C1–C60 and C65 true, C61–C64 false
- A comparison script keyed on main's bytes at 7b536253 compares all 65 C texts and 23 S texts in the rendered markdown, prints the count it compared (88), and prints zero differences
- The committed DESIGN.md, CHECKLIST.md and USER-STORIES.md (and the brief's .md) are byte-identical to what render-cluster.py produces
- python3 scripts/design/check-coverage.py docs/design/lys-core reports coverage clean
- sh scripts/design/gate.sh exits 0 on the card's final tree
- The dev record lists every difference between the rendered DESIGN.md and main's DESIGN.md with its reason, and names every sentence moved to a different field, with where it went
- docs/design/roadmap.json has a row linking cluster lys-core and this card's brief

### What must not change

- No C or S id is renumbered, merged or dropped, and no C or S text changes by a byte
- No new checklist item, user story, principle or ADR
- No change under crates/ or scripts/design/ (renderer, validator, schemas)
- The directory, home and secrets clusters, decisions.json and WIRE-FORMATS.md are untouched
- Relative links in the carried text (../../ROADMAP.md, ../WIRE-FORMATS.md, ../../PEN-REGISTRATION.md, ../../REVIEW-23-07.md) stay as written

### What we must put in place first

- The lead's answer on where the checklist notes go, and on whether structure may gain the cluster's own document rows, both before the brief is signed off

### The risks

- Checklist notes placed in design.json make the rendered DESIGN.md differ from main in content, which fails the 'layout only' acceptance as written
- Coverage makes this carry-only brief claim C1–C65. A later reader may take it for the brief that built them.
- Line breaks in the 73-entry tree turned into table rows may merge or split a note unnoticed. The text-comparison script covers C and S only, so structure notes need their own count check.
- Markdown inside the solution string (a code fence, nested bullets, blockquotes) has to survive JSON escaping byte for byte. An escaped backtick or an en-dash turned into a hyphen is a rewording.
- If qVVhhP2b's brief is re-rendered before this card lands, the two cards collide on the lys-core brief number and on design.json

### Still open

- CHECKLIST.md's 3-paragraph intro note and its two italic annotations (C38–C43 and C58–C59) have no field in checklist.json. Should they move into design.json, where they add prose to the rendered DESIGN.md, or into checklist section names, which changes those headings? The sentence of the words it stands on: "Where a sentence of the hand-written documents has no field in the schema, the brief names it and places it in the nearest field that holds free text, and says so in its dev record.". Why only the lead can settle it: render-checklist in scripts/design/render-cluster.py has no preamble or note slot, so the rendered CHECKLIST.md loses these lines either way. Placing them in design.json makes the rendered DESIGN.md differ from main in content, not just layout, against the acceptance sentence. Placing them in section names changes the eight checklist headings a reader sees.
- May design.json structure gain rows that main's DESIGN.md does not have (docs/design/lys-core/*.json, *.md and the brief's own files), recorded as a non-layout difference? The alternative is a brief whose R# files list nothing. The sentence of the words it stands on: "Acceptance is that scripts/design/validate.py accepts the three JSON documents; that scripts/design/gate.sh passes on the card's final tree; that checklist.json holds C1 to C65 and stories.json holds S1 to S23, each id once; that a script in the brief compares every C and S text in the rendered documents with the text on main at the commit this card starts from, and prints zero differences; and that every difference between the rendered DESIGN.md and main's DESIGN.md is listed in the dev record with its reason, and each is a difference of layout only.". Why only the lead can settle it: scripts/design/check-coverage.py fails any brief file path that structure does not list. DIRECTORY-008 set the precedent of listing the cluster's own documents. The structure section of the rendered DESIGN.md would then show rows that main's DESIGN.md does not, which is not a layout-only difference.

### The units beyond the first

- Reconcile the lys-core design with the tree as built — The carried structure omits 22 tracked files (delegation/, receipt/, bundle/, ca/request.rs, keys/ssh.rs), and S12 and S14 describe superseded surfaces. Correcting them rewords carried text, which this card forbids.
- Give the method a note field for checklists and personas — Checklist preambles, section annotations and persona headings with no role have nowhere to live in the method's schemas. That is a change in the method project (tools/design-system), then re-copied here, so it is code and out of this card.

### The smallest complete shape

One docs-only commit under docs/design/lys-core containing four things. First, design.json, checklist.json and stories.json carrying main's text exactly. Second, a lys-core brief (for example CORE-001) that claims C1–C65 and S1–S23 and lists its own files, with those paths in structure. Third, the rendered DESIGN.md, CHECKLIST.md, USER-STORIES.md and brief .md, plus a roadmap row linking cluster lys-core. Fourth, a dev record with the text-comparison output (88 compared, 0 differences) and a line-by-line list of layout differences. gate.sh passes on that tree.

## The roadmap row

- **RM-030** — Carry the hand-written lys-core design into design.json, checklist.json and stories.json (design, idea)
- Summary: The lys-core design on main is three hand-written documents with no JSON sources, and the first lys-core brief to add a design.json turns on the gate's render-and-compare for the cluster, which would replace them with that JSON's smaller scope. This item carries main's DESIGN.md, CHECKLIST.md (C1 to C65) and USER-STORIES.md (S1 to S23) at 7b536253 into design.json, checklist.json and stories.json with every C and S id and text unchanged, commits the rendered markdown, and records every place the render differs from main: layout only, apart from the checklist notes section and the rows for the cluster's own documents that the card's lead allows. Nothing new is designed and no code changes.
- Asked by: tom on 2026-09-27T12:52:00+10:00
- Context: The lys-core carry card on the Cambium board, filed by the identity line's lead in answer to round 2 of run 5f185fd4 (the self-signed card qVVhhP2b, whose brief waits for this card to land), against lys main 7b536253. Brief LYSCORE-001 was written by the brief author in run b1ff26fe from the card's words, the survey, and the lead's two answers: the checklist's intro note and two annotations go into design.json as one checklist-notes section, and the structure lists the cluster's own documents and the brief's own files; each is one of the two content differences the lead allows against the layout-only acceptance.
- Quote: The lys-core design on main is three hand-written documents, docs/design/lys-core/DESIGN.md, CHECKLIST.md and USER-STORIES.md, holding checklist items C1 to C65 and user stories S1 to S23. The design method renders those documents from design.json, checklist.json and stories.json, and the cluster has none of the three. The first lys-core brief to add a design.json makes the gate render the cluster, and the render would replace the hand-written documents with the JSON's much smaller scope, so the older design would be lost from the record. This card carries the whole of the hand-written design into the three JSON documents first, so that the render reproduces it.

Every checklist item keeps its C id and its text, every user story keeps its S id and its text, and every section of DESIGN.md is carried into design.json in the fields the design schema gives it. Nothing is reworded, merged, dropped or renumbered. Where a sentence of the hand-written documents has no field in the schema, the brief names it and places it in the nearest field that holds free text, and says so in its dev record. The rendered DESIGN.md, CHECKLIST.md and USER-STORIES.md are committed from the render.

Acceptance is that scripts/design/validate.py accepts the three JSON documents; that scripts/design/gate.sh passes on the card's final tree; that checklist.json holds C1 to C65 and stories.json holds S1 to S23, each id once; that a script in the brief compares every C and S text in the rendered documents with the text on main at the commit this card starts from, and prints zero differences; and that every difference between the rendered DESIGN.md and main's DESIGN.md is listed in the dev record with its reason, and each is a difference of layout only.

Not in scope are any new checklist item, user story or design decision, and any change to code. The self-signed card qVVhhP2b's brief, run 5f185fd4, waits for this card to land, and it adds its own rows after the carried ones.

Filed by Archie, lead for the identity line, on 27 September 2026, in answer to round 2 of run 5f185fd4, against lys main 7b536253.
- Cluster: lys-core; briefs: LYSCORE-001
- Notes: Further units, not written: 'Reconcile the lys-core design with the tree as built' (the carried structure omits 22 tracked files under delegation/, receipt/, bundle/, ca/request.rs and keys/ssh.rs, and S12 and S14 describe superseded surfaces; correcting them rewords carried text, which this card forbids); 'Give the method a note field for checklists and personas' (a note slot in render-cluster.py and the checklist and stories schemas, a change to the shared design tooling in the method project, re-copied here). The qVVhhP2b card's brief takes the next lys-core brief number and C66 and S24 onward after this lands. RM-030 is the next id after main's highest (RM-016) and every open brief branch's (RM-029 highest, checked against origin's brief/* branches); LYSCORE-001 is the first brief in the cluster. No new ADR.

## The design

---
type: design
cluster: lys-core
title: Lys Core — Trust Primitives and CLI Surface
---

# Lys Core — Trust Primitives and CLI Surface

> **Cluster:** lys-core

## Intention

When this cluster is done, the hardened trust primitives live in this repository as `lys-core` — a standalone, domain-agnostic library with zero Meridian lineage — and the `lys` binary gives operators and auditors a command-line face over every primitive. An agent runtime signs session events with it. An auditor verifies a challenged log with it, offline, from nothing but a signed proof artifact, the leaf in question, and the log's verifier key. A future anchoring service consumes its roots and attestations. Nothing in the crate knows what an agent, session, or workspace is; domain meaning is applied by consumers.

The crate arrives only in its hardened form. Phase 0 (the adversarial review and fix pass on `meridian-trust`) is done; this cluster is the extraction (phase 1) plus the CLI surface (phase 2). Behaviour is ported unchanged except for three deliberate breaks made at the last free moment: the wire-format tags are renamed to lys-owned strings, the legacy pre-domain-separation attestation fallback is stripped, and the identity env var and OID constant take lys names.

## Problem

The primitives exist today as `crates/meridian-trust` inside the Meridian workspace — hardened, adversarially reviewed, ~137 tests, with a live consumer. But they are unusable as the foundation of an open trust project in that shape:

- The wire-format tags (`meridian-trust/attestation/v1`, `meridian-trust-sealed-envelope/v1`) are baked into every signature produced. Once anything durable is signed under a tag, that tag is frozen forever. The extraction is the last moment these strings can change.
- The attestation verifier carries a dual-verify legacy fallback that exists only so Meridian's already-persisted attestations keep verifying. Lys must not inherit that caveat: v1 is domain-separated only.
- The custom-extension OID constant, the identity env var, and the crate naming all carry Meridian identity into what must be a vendor-neutral library.
- There is no operator or auditor surface. The library's defining promise — third parties verify without the operator's cooperation — has no tool a third party can actually run.

Consumers are waiting on the extracted form: the Norn agent runtime (the primary consumer — signing persistence sink, cert-at-spawn, MCP-boundary verification), the future `lys-anchor` transparency service, and haematite commit attestation.

## Solution

### D1: Domain-agnostic boundary

The founding rule carries over unchanged: `lys-core` knows no domain concepts. No agents, sessions, workspaces, peers, contracts, or members — and no Meridian references of any kind. It provides:

- Ed25519 key management with X25519 derivation
- A Certificate Authority that issues X.509 certificates for any subject
- An RFC 6962 Merkle transparency log over any serializable leaf, or over raw leaf bytes
- Signed tree heads as C2SP checkpoints in the signed-note envelope, and self-contained JSON proof artifacts a third party can verify unaided
- Domain-separated signed attestations over any byte payload
- Sealed envelopes for any byte payload, standalone or sender-authenticated

Consumers compose meaning on top: Norn defines what an "agent certificate" or "session event leaf" is; the trust crate doesn't know or care. If a type references a domain concept, it does not belong in this crate.

### D2: Key management (`lys_core::keys`)

`Ed25519Identity` is the single long-term key type. Ported hardened behaviour:

- `load_or_generate(path)` — loads a 32-byte seed file or generates one. Generation is race-free: the seed is written to a unique temp file (pid + per-process counter in the name) and published with a no-clobber `hard_link` — the first generator to publish wins permanently; a loser detects `AlreadyExists`, discards its candidate seed, and loads the persisted key. The key file on disk never changes once created.
- Unix key files are created mode `0o600`; loading a file with loose permissions warns but does not fail.
- `from_env()` — loads a base64-encoded 32-byte seed from **`LYS_IDENTITY_KEY`** (renamed from the Meridian variable). Missing or malformed values are `KeyManagement` errors, never panics.
- All seed material — generated, file-read, or base64-decoded — lives in `Zeroizing` buffers.
- `sign(message)` → `[u8; 64]`; `verify(public_key, message, signature)` uses `verify_strict` (malleability/torsion-safe) everywhere. No non-strict verification exists anywhere in the crate.
- `Debug` output redacts the signing key; redaction is tested, not assumed.
- `x25519_static_secret()` / `x25519_public_key()` derive the Montgomery-form X25519 keys from the Ed25519 identity via the standard clamped-scalar conversion, so one long-term key serves both signing and credential unsealing.

### D3: Certificate Authority (`lys_core::ca`)

Ed25519-rooted X.509 issuance and verification:

- `CertificateAuthority` wraps an `Ed25519Identity`; `issue_certificate(subject, ttl, extensions)` produces an `IssuedCertificate` (DER bytes, subject keypair, SHA-256 fingerprint, expiry, issuer public key; Debug-redacted).
- **Proof of possession.** `issue_certificate` generates the subject keypair itself, so its certificate binds a key the *authority* minted — it is evidence about the authority, not about any holder, and anything layered on it (transparent issuance logging, a cert-gated write path) inherits that emptiness. `issue_certificate_for_request(request_der, subject, ttl, extensions)` is the path with a real binding: the holder presents a PKCS#10 request self-signed by a key they already control, and the certificate is signed over that key. It returns a `CertifiedKey`, which deliberately carries no private material — a separate type rather than an optional signing key, so the two outcomes cannot be confused. `create_certificate_request(identity, subject)` builds the holder's side.
  - PKCS#10 rather than a lys-native tag: RFC 2986 already specifies this exchange and its self-signature *is* the canonical proof of possession, so `openssl req` interoperates in both directions and no permanent wire contract is invented.
  - A request influences **exactly one** certificate field: the subject public key. Both issuance paths build their `CertificateParams` from the authority's inputs through a shared `leaf_params`, and requested extensions are refused rather than stripped. `subject` must equal the request's common name, so a holder cannot name themselves and an authority cannot certify them under a name they never asked for.
  - `verify_certificate_request` parses with `x509-parser` but verifies with `ed25519-dalek::verify_strict`. x509-parser's own `verify_signature` routes Ed25519 to ring's non-strict verification, which accepts small-order and torsion keys — for which signatures verify with no private key known, reducing proof of possession to a formality anyone could satisfy for a key nobody controls.
  - Unlike artifact verification elsewhere in the crate, request verification is **not** non-oracle: no authority secret participates and the requester already knows their own key, so precise diagnostics leak nothing and the operator needs them.
  - **What proof of possession prevents is misattribution by key binding, and it protects the authority, not the verifier.** The tempting misreading — "a verifier who checks a signature against the certified key has already observed key control, so the issuance-time check is ceremony" — is how an authority talks itself into dropping it. The real attack: Mallory presents *Noor's* public key under the name `agent-mallory`; without the check the certificate issues, and every statement Noor legitimately signs then verifies against a certificate naming Mallory. No forgery, a genuinely valid certificate, and the `lys verify --cert` join reports success on a false statement. Requiring a signature over the request — which covers the subject name — means a presented key can only be certified by its holder, under the name they asked for.
  - **A certificate does not record how it was issued.** X.509 has no marker for "issued over a proven key" and this crate adds none, so a relying party cannot tell from the artifact whether the certified key is holder-controlled; that rests on issuer policy. Narrower than it looks — a certificate over a discarded generated key binds a key nobody can sign with, so it fails closed rather than dangerously — but it is the argument for making issuance policy auditable by logging issuance transparently (DP3), not for a certificate field.
  - **Requests are replayable, deliberately.** The signature covers the subject key and requested name and nothing tying it to one issuance, authority, or moment. That is sound: proof of possession is a claim about key control, not an authorisation of a particular issuance, and a replay yields a certificate over a key its holder already proved they hold. An authority that needs issuance *authorised* needs an access-control decision about the requester, which must not be built on the request's signature.
- rcgen signing goes through a `RemoteKeyPair` adapter so the CA's private seed is never serialised into rcgen's key-pair representation, and a presented key reaches rcgen through a `PublicKeyData` adapter that cannot sign. `PKCS_ED25519` throughout.
- `verify_certificate_chain(cert_der, issuer_public_key)` extracts the TBS bytes with `x509-parser` and verifies the signature with `ed25519-dalek::verify_strict` (x509-parser cannot verify Ed25519). The validity window is enforced in-crate: expired and not-yet-valid certificates are rejected, and `verify_certificate_chain_at(cert_der, issuer_public_key, instant)` verifies at an explicit instant for auditing historical records. Self-signed certificates are rejected.
- `certificate_subject_public_key(cert_der)` recovers the 32-byte Ed25519 key a certificate vouches for. Parsing only — a key read from an *unverified* certificate is an attacker-chosen value, and the rustdoc says so. It exists to close the join: verifying a certificate proves an authority issued it and verifying an attestation proves a key signed a payload, but neither says the two concern one identity. Comparing this against an attestation's signer key is the check that connects them, and `lys verify --cert` is where that composition currently lives — deliberately in the CLI rather than the library, because the library's composed verifier is the one the anchor's verification bundle will need and designing it before that format settles would freeze a guess.
- Capability claims travel as opaque DER in custom extensions under **`LYS_OID_ARC`** (`1.3.6.1.4.1.66364`). The payload is opaque to the crate; the consumer defines claim semantics — the cert *is* the permission object. The final component is IANA Private Enterprise Number 66364, assigned to lys; the arc is permanent and sub-arcs beneath it are ours to allocate (see [PEN-REGISTRATION.md](../../PEN-REGISTRATION.md)).

Revocation tracking is deliberately absent (never built in the source crate; a first-class revocation story is an open product question — see repo DESIGN.md §Open questions).

### D4: Merkle transparency log (`lys_core::merkle`, `lys_core::checkpoint`, `lys_core::tlog`)

`AppendOnlyTree<L: Serialize>` provides RFC 6962 semantics over SHA-256, backed by `ct-merkle` behind a deliberately backing-agnostic API:

- `append(leaf)` is the only mutation — the API exposes no delete or modify. Every argument is pre-checked so the underlying library cannot panic; out-of-range indices and invalid size pairs return `MerkleTree` errors.
- Inclusion proofs (`prove_inclusion` / `verify_inclusion`) and consistency proofs (`prove_consistency` / `verify_consistency`), with byte round-tripping (`as_bytes` / `try_from_bytes`) on both proof types.
- `RootHash::from_parts(root_hash, num_leaves)` / `to_parts()` — the external-verifier constructor. A third party holding only a published root and proof bytes can verify inclusion and consistency with no access to the tree. The external-verifier round trip is the defining test of the layer.
- `reconstruct_from_leaves(leaves)` rebuilds an identical tree from a persisted leaf sequence — the crash-recovery path for consumers persisting leaves externally.
- Leaf serialization is a **frozen wire contract**: leaves are canonical bytes; schema evolution means a new versioned leaf type, never a mutated one. This rule is documented at the module level. Two encodings are frozen and never mix within one tree — the typed postcard path, and the raw path (`RawLeaf`: leaf file bytes verbatim, no framing) that every `lys log` artifact uses.

Two layers ratified after this cluster was first written — decisions **D1** and **D2** in [WIRE-FORMATS.md](../WIRE-FORMATS.md) §2–§3 — ship on top of `merkle` and are part of the crate as built:

- **`lys_core::checkpoint`** — the signed tree head: a C2SP tlog-checkpoint body wrapped in the C2SP signed-note envelope, Ed25519-signed, byte-compatible with the Go `sumdb/note` reference. `verify_checkpoint` **enforces** `checkpoint origin == verifier-key name`, so a key that signs two logs can never have one log's checkpoint accepted by a verifier configured for the other. Every failure mode — size, UTF-8, structure, unknown key, bad signature — collapses to the single `TrustError::NoteVerification`.
- **`lys_core::tlog`** — self-contained JSON proof artifacts: an RFC 6962 proof plus the relevant signed checkpoint(s) embedded verbatim, identified by frozen `format` strings, with unknown fields rejected. Redundancy is checked, not trusted: every size an artifact declares is compared against the size inside its signature-verified checkpoint, and roots are recomputed rather than believed. Builders self-verify before returning, tree sizes at or beyond 2^53 are refused on both emit and verify, and every failure collapses to the single `TrustError::LogArtifactVerification`.

### D5: Signed attestations (`lys_core::attestation`) — COSE_Sign1 v2, canonical-strict

> **Superseded in part by decision D4 in [WIRE-FORMATS.md](../WIRE-FORMATS.md) §4.2**: the v1 JSON/preimage form this section originally specified was deleted unshipped; the byte-exact contract now lives there. As-built summary:

Signed statements binding a key to a payload. The artifact is a tagged COSE_Sign1 (`lys/attestation/v2`); the signed preimage is the RFC 9052 §4.4 `Sig_structure`:

```
Sig_structure = ["Signature1", protected, h'', claims]
protected     = {1: -8 (EdDSA), 3: "application/vnd.lys.attestation.v2+cbor", 4: signer key}
claims        = {1: SHA-256(payload), 2: unix-ms timestamp}
```

- The timestamp, payload hash, and signer key are all authenticated — inside the signature, not alongside it. Tampering with any of them fails verification.
- The `Sig_structure` framing plus the signature-covered content type make attestation signatures structurally non-interchangeable with any other lys signing context (sealed-envelope binding, raw CA certificate signing, signed notes) — byte-0 disjoint, per WIRE-FORMATS §4.2.
- **No fallback paths.** `verify_attestation` accepts the v2 `Sig_structure` and nothing else: the deleted v1 preimage (`b"lys/attestation/v1" || timestamp_le || hash`) and pre-domain-separation bare-hash signatures both fail (tests exist).
- `Attestation { payload_hash: [u8; 32], signature: [u8; 64], signer_public_key: [u8; 32], timestamp: i64 }` carries **no serde**; the only durable form is `to_cose_bytes()`, and `from_cose_bytes` is canonical-encoding-strict.

### D6: Sealed envelopes (`lys_core::seal`)

X25519 ephemeral key agreement + HKDF-SHA256 + AES-256-GCM, the standard sealed-box construction with the keys bound into the KDF:

- `seal(payload, recipient_public_key)` → `SealedEnvelope { ephemeral_public_key, ciphertext, nonce }`. Fresh ephemeral keypair per seal — forward secrecy per envelope.
- HKDF info binds the domain tag and both public keys: `b"lys-sealed-envelope/v1" || ephemeral_public_key || recipient_public_key` (tag renamed from the Meridian string). **The HKDF info tag is the hyphen form** `lys-sealed-envelope/v1`; the format name and the authenticated composition's attestation context tag (`SEALED_ENVELOPE_CONTEXT_V1`) are the slash form `lys/sealed-envelope/v1`. They are deliberately different strings — separate domains, separately versioned — and they are not interchangeable: an implementer who feeds the slash form to HKDF derives a different key and cannot decrypt. WIRE-FORMATS.md §1 carries both.
- Contributory-behaviour enforcement on **both** seal and open: a low-order public key producing a non-contributory shared secret is rejected before any key material is derived.
- Every unseal failure — wrong key, tampered ciphertext, tampered nonce — collapses to the single undifferentiated `TrustError::UnsealFailed` through one failure arbiter (AES-GCM tag verification). No oracle, no timing split, no early return.
- `sign_and_seal(payload, sender_identity, recipient_x25519_public_key)` / `open_and_verify(...)` compose attestation over the sealed bytes for sender-identity binding. The attestation covers every wire byte of the envelope (`attestation_bytes()`), and verification gates **before** the cipher is ever touched — a forged sender is rejected without decrypting anything.

### D7: Wire formats are forever

The domain tags (`lys/attestation/v2`, `lys/sealed-envelope/v1`, and the distinct HKDF info tag `lys-sealed-envelope/v1`), the attestation `Sig_structure` layout, the HKDF info layout, both leaf encodings, the checkpoint and signed-note encodings, and the proof-artifact `format` strings are versioned wire contracts, frozen the moment anything durable is signed under them. [WIRE-FORMATS.md](../WIRE-FORMATS.md) §1 is the authoritative table. Evolving one means a new versioned constant and code path, never a mutation of the shipped one. The extraction renames the Meridian tags precisely because it is the last moment nothing has been signed under the lys names.

### D8: CLI surface (`lys` binary)

> **Superseded in part by decisions D1 and D2 in [WIRE-FORMATS.md](../WIRE-FORMATS.md) §2–§3**: the log subcommands emit C2SP signed-note checkpoints and self-contained JSON proof artifacts, and third-party verification takes the artifact, the leaf, and the verifier key rather than raw root parts plus proof bytes. As-built summary:

The auditor's and operator's tool — a thin clap surface over `lys-core`. Logic lives in the library; the binary parses arguments, dispatches, and maps results to exit codes (`0` success, `1` operational or verification failure, `2` clap argument errors). As built it carries its own `thiserror` error type with deliberately non-oracle failure messages, and no `anyhow`. Subcommands:

- `lys key generate` / `lys key inspect` — generate and inspect identities (Ed25519 and derived X25519 public keys, and with `--note-name` the signed-note verifier-key string). **Never prints private key material** under any flag or format.
- `lys ca issue` — issue a certificate with a capability-claim extension payload, signed by an issuer identity.
- `lys ca verify` — verify a certificate chain against an issuer public key, with an optional explicit verification instant (the `verify_certificate_chain_at` path).
- `lys attest` / `lys verify` — sign and verify `lys/attestation/v2` COSE_Sign1 artifacts over a payload file. (File paths only as built; there is no stdin path.)
- `lys seal` / `lys open` — sealed-envelope transport of a payload file, authenticated composition only: `seal` writes the JSON envelope and the sender's COSE attestation, and `open` requires both, verifying before it decrypts.
- `lys inspect attestation` / `lys inspect cert` — read-only viewers that print what a file says **without verifying any of it**, every output opening with an UNVERIFIED banner naming the command that does verify. Local files only.
- `lys log init` / `append` / `checkpoint` / `prove` / `verify` — transparency-log operations over a persisted leaf sequence: `init` pins the log's origin exactly once and refuses re-initialization, `append` hashes a leaf file's raw bytes per RFC 6962, `checkpoint` signs a C2SP signed-note checkpoint over the current root, `prove` emits a self-contained JSON proof artifact with the relevant signed checkpoint(s) embedded, and `verify` checks an inclusion or consistency claim from **only** the artifact, the leaf, and the verifier key — no access to the store or the tree.

The phase proof: a log produced by one process is verified end-to-end by the CLI in another process that never sees the original tree.

Tests live in sibling `*_tests.rs` files throughout `merkle`, `ca`, `keys`, `attestation`, `checkpoint`, and `tlog`; `merkle/leaf.rs`, both `seal/` files, and several CLI modules currently carry inline `mod tests` instead (see [REVIEW-23-07.md](../../REVIEW-23-07.md) F12).

### Checklist notes

> **Reconciled against the shipped code, 2026-07-30.** [ROADMAP.md](../../ROADMAP.md) marks Phase 1 (extract to `lys-core`) and Phase 2 (the `lys` CLI) **DONE**, and the boxes below now record that: an item is ticked only where it was verified against this repository's own bytes.
>
> Four are deliberately left open. **C61–C63** are the build gates — verified by CI on every change, not by reading source. **C64** is not met: tests live inline rather than in sibling `*_tests.rs` files in `merkle/leaf.rs`, both `seal/` files, `error.rs`, and six CLI modules (see [REVIEW-23-07.md](../../REVIEW-23-07.md) F12).
>
> Items amended by a ratified wire-format decision carry an annotation above them and are restated as-built; the decisions themselves live in [WIRE-FORMATS.md](../WIRE-FORMATS.md), which is authoritative wherever this file disagrees. This is the phase-1/2 acceptance list only — the `checkpoint` and `tlog` layers ratified afterwards (WIRE-FORMATS D1 and D2) are specified in [DESIGN.md](DESIGN.md) D4 and pinned by their own test suites, not by items here.

*(C38–C43 amended by WIRE-FORMATS.md decision D4: the attestation artifact is the `lys/attestation/v2` tagged COSE_Sign1; the v1 JSON/preimage form was deleted unshipped.)*

*(C58–C59 superseded by WIRE-FORMATS.md decisions D1 and D2: `lys log` emits C2SP signed-note checkpoints and self-contained JSON proof artifacts, and third-party verification takes the artifact, the leaf, and the verifier key rather than raw root parts plus proof bytes. Restated as-built.)*

## Goals

- `lys-core` compiles standalone in this repository with zero Meridian dependencies and zero Meridian references, behaviour-identical to the hardened source crate except the deliberate breaks (D5 legacy strip, D7 tag renames, `LYS_IDENTITY_KEY`, `LYS_OID_ARC`).
- All hardening commitments hold in the ported code: `verify_strict` everywhere, validity-window enforcement with an `_at` variant, `RootHash::from_parts` external verification, authenticated timestamps with domain separation, contributory-DH rejection, seed zeroization, single-arbiter unsealing, race-free key generation.
- Attestation verification is v2-only (WIRE-FORMATS.md D4): no legacy code path exists in the crate — neither the Meridian preimage nor the deleted-unshipped `lys/attestation/v1` form verifies.
- An external verifier round-trips: inclusion and consistency proofs verify from published root parts and proof bytes alone.
- The `lys` CLI covers every primitive, and a log produced in one process verifies end-to-end via the CLI in another with no access to the original tree.
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --workspace` all pass clean.

## Non-Goals

- **Domain-specific semantics.** No agent, session, or claim vocabulary in the crate. Canonical agent claim schemas are phase 5.
- **Storage traits.** In-memory operations only; persistence is the consumer's concern. The CLI persists leaf sequences as files, using `reconstruct_from_leaves` — that is CLI policy, not a library trait.
- **Network operations.** `lys-core` is a pure library; the CLI is local-only. Transport belongs to `lys-anchor` (phase 4).
- **Anchoring, receipts, SCITT/COSE.** The notary layer is `lys-anchor`; nothing in this cluster emits or verifies COSE receipts.
- **Revocation infrastructure.** No CRLs, no OCSP, no revocation flag. Consumer-side today; a first-class answer is an open product question.
- **MCP surface.** `lys-mcp` is a later phase.
- **Zero-knowledge proofs.** Selective disclosure via salted-hash leaves + inclusion proofs is the v1 privacy story; ZK is a research direction.

## Structure

| Path | Note | Brief |
|------|------|-------|
| `crates/lys-core/` |  |  |
| `crates/lys-core/Cargo.toml` |  |  |
| `crates/lys-core/tests/` | cross-implementation conformance suites |  |
| `crates/lys-core/tests/cose_conformance.rs` | round-trip against veraison/go-cose |  |
| `crates/lys-core/tests/go_conformance.rs` | round-trip against Go sumdb/note |  |
| `crates/lys-core/tests/signed_note_crosscheck.rs` | crosscheck against Cloudflare signed_note |  |
| `crates/lys-core/src/` |  |  |
| `crates/lys-core/src/lib.rs` | pub mod + re-exports, hex_lower helper (D1) |  |
| `crates/lys-core/src/error.rs` | TrustError enum, TrustResult<T> (D1) |  |
| `crates/lys-core/src/keys/` |  |  |
| `crates/lys-core/src/keys/mod.rs` | pub mod / pub use only |  |
| `crates/lys-core/src/keys/identity.rs` | Ed25519Identity: load_or_generate, from_env, sign, verify_strict, X25519 derivation, redaction (D2) |  |
| `crates/lys-core/src/keys/identity_tests.rs` |  |  |
| `crates/lys-core/src/ca/` |  |  |
| `crates/lys-core/src/ca/mod.rs` | pub mod / pub use only |  |
| `crates/lys-core/src/ca/authority.rs` | CertificateAuthority: issue, verify chain, _at variant (D3) |  |
| `crates/lys-core/src/ca/certificate.rs` | IssuedCertificate, Debug redaction (D3) |  |
| `crates/lys-core/src/ca/extensions.rs` | LYS_OID_ARC, encode/decode extension (D3) |  |
| `crates/lys-core/src/ca/*_tests.rs` |  |  |
| `crates/lys-core/src/merkle/` |  |  |
| `crates/lys-core/src/merkle/mod.rs` | pub mod / pub use only |  |
| `crates/lys-core/src/merkle/tree.rs` | AppendOnlyTree<L>: append, root, proofs, reconstruct (D4) |  |
| `crates/lys-core/src/merkle/proof.rs` | RootHash from_parts/to_parts, Inclusion/ConsistencyProof, verify_inclusion, verify_consistency, raw-leaf path (D4) |  |
| `crates/lys-core/src/merkle/leaf.rs` | leaf hashing, RawLeaf, frozen-wire-contract docs (D4) |  |
| `crates/lys-core/src/merkle/*_tests.rs` |  |  |
| `crates/lys-core/src/checkpoint/` | WIRE-FORMATS D1: signed tree heads |  |
| `crates/lys-core/src/checkpoint/mod.rs` | pub mod / pub use only |  |
| `crates/lys-core/src/checkpoint/body.rs` | CheckpointBody encode/parse (C2SP tlog-checkpoint) |  |
| `crates/lys-core/src/checkpoint/note.rs` | sign_note / verify_note / verify_checkpoint (C2SP signed-note; origin == key-name enforced) |  |
| `crates/lys-core/src/checkpoint/verifier_key.rs` | verifier-key strings and RFC 6962-style key IDs |  |
| `crates/lys-core/src/checkpoint/*_tests.rs` |  |  |
| `crates/lys-core/src/tlog/` | WIRE-FORMATS D2: self-contained proof artifacts |  |
| `crates/lys-core/src/tlog/mod.rs` | pub mod / pub use only |  |
| `crates/lys-core/src/tlog/artifact.rs` | frozen JSON artifact shapes and format strings |  |
| `crates/lys-core/src/tlog/build.rs` | self-verifying inclusion/consistency builders |  |
| `crates/lys-core/src/tlog/verify.rs` | third-party verification, single non-oracle error |  |
| `crates/lys-core/src/tlog/*_tests.rs` |  |  |
| `crates/lys-core/src/attestation/` |  |  |
| `crates/lys-core/src/attestation/mod.rs` | pub mod / pub use, invariant docs |  |
| `crates/lys-core/src/attestation/artifact.rs` | Attestation type; to_cose_bytes / canonical-strict from_cose_bytes (D5) |  |
| `crates/lys-core/src/attestation/encoding.rs` | private byte-exact COSE_Sign1 encode + shape-pinned decode, Sig_structure assembly (D5) |  |
| `crates/lys-core/src/attestation/sign.rs` | sign_attestation, verify_attestation, verify_attestation_bytes over the v2 Sig_structure (D5) |  |
| `crates/lys-core/src/attestation/*_tests.rs` | sibling tests incl. golden vectors and mutants A–F |  |
| `crates/lys-core/src/seal/` |  |  |
| `crates/lys-core/src/seal/mod.rs` | pub mod / pub use only |  |
| `crates/lys-core/src/seal/sealed_envelope.rs` | seal/open, HKDF binding, contributory checks, single failure arbiter (D6) |  |
| `crates/lys-core/src/seal/authenticated.rs` | sign_and_seal / open_and_verify (D6) |  |
| `crates/lys/` |  |  |
| `crates/lys/Cargo.toml` |  |  |
| `crates/lys/tests/` |  |  |
| `crates/lys/tests/cli_tests.rs` | key / attest / verify / ca / seal / open / inspect |  |
| `crates/lys/tests/log_tests.rs` | log lifecycle incl. the cross-process third-party path |  |
| `crates/lys/src/` |  |  |
| `crates/lys/src/main.rs` | thin entry: parse args, dispatch, exit codes (D8) |  |
| `crates/lys/src/cli.rs` | clap definitions and help text only (D8) |  |
| `crates/lys/src/commands/` |  |  |
| `crates/lys/src/commands/mod.rs` | pub mod only |  |
| `crates/lys/src/commands/key.rs` | lys key generate / inspect (D8) |  |
| `crates/lys/src/commands/ca.rs` | lys ca issue / verify (D8) |  |
| `crates/lys/src/commands/attest.rs` | lys attest (D8) |  |
| `crates/lys/src/commands/verify.rs` | lys verify (D8) |  |
| `crates/lys/src/commands/inspect.rs` | lys inspect attestation / cert (D8) |  |
| `crates/lys/src/commands/seal.rs` | lys seal / open (D8) |  |
| `crates/lys/src/commands/error.rs` | CLI error type, non-oracle failure messages (D8) |  |
| `crates/lys/src/commands/files.rs` | file I/O incl. owner-only plaintext writes (D8) |  |
| `crates/lys/src/commands/hex.rs` | hex parsing/formatting helpers (D8) |  |
| `crates/lys/src/commands/pem.rs` | PEM encode/decode helpers (D8) |  |
| `crates/lys/src/commands/log/` |  |  |
| `crates/lys/src/commands/log/mod.rs` | pub mod only |  |
| `crates/lys/src/commands/log/init.rs` | lys log init (origin pinned once) (D8) |  |
| `crates/lys/src/commands/log/append.rs` | lys log append (D8) |  |
| `crates/lys/src/commands/log/checkpoint.rs` | lys log checkpoint (D8) |  |
| `crates/lys/src/commands/log/prove.rs` | lys log prove inclusion / consistency (D8) |  |
| `crates/lys/src/commands/log/verify.rs` | lys log verify inclusion / consistency (D8) |  |
| `crates/lys/src/commands/log/store.rs` | leaf-sequence store: O_EXCL leaf writes, atomic tmp+rename state, rebuild on open (D8) |  |
| `docs/design/lys-core/design.json` | the lys-core design, carried from main's DESIGN.md | LYSCORE-001 |
| `docs/design/lys-core/DESIGN.md` | rendered markdown |  |
| `docs/design/lys-core/checklist.json` | the lys-core checklist, carried from main's CHECKLIST.md | LYSCORE-001 |
| `docs/design/lys-core/CHECKLIST.md` | rendered markdown |  |
| `docs/design/lys-core/stories.json` | the lys-core user stories, carried from main's USER-STORIES.md | LYSCORE-001 |
| `docs/design/lys-core/USER-STORIES.md` | rendered markdown |  |
| `docs/design/lys-core/briefs/LYSCORE-001.json` | the brief that carries the hand-written lys-core design into the three JSON documents | LYSCORE-001 |
| `docs/design/lys-core/briefs/LYSCORE-001.md` | rendered markdown | LYSCORE-001 |

## Constraints

- **CN1** — **No domain types and no Meridian references.** If it names an agent, session, workspace, peer, contract, or anything Meridian, it doesn't belong here.
- **CN2** — **No storage traits, no network.** Pure library crate; CLI is local file I/O only.
- **CN3** — **`unsafe_code` forbidden.** All dependencies pure Rust.
- **CN4** — **No `unwrap` / `expect` / `panic` / `todo` in library code.** Tests opt out per-module.
- **CN5** — **Private key material never in `Debug`, logs, error messages, or CLI output.** Redaction tested, not assumed. Seed buffers are `Zeroizing`.
- **CN6** — **Wire formats are frozen.** Tags, preimage layouts, and leaf encodings version forward (`v2`), never mutate.
- **CN7** — **No file over 500 lines of code.** `mod.rs` carries only `pub mod` / `pub use` / module docs; tests live in sibling `*_tests.rs` files.
- **CN8** — **Every public item documented**; module-level `//!` docs state invariants.
- **CN9** — **Cryptographic changes require an adversarial review before landing.** This cluster ports hardened behaviour unchanged; any deviation beyond the four deliberate breaks (tag renames, legacy strip, env var, OID constant) is out of bounds.


---
type: brief
id: LYSCORE-001
cluster: lys-core
title: Carry the hand-written lys-core design into its three JSON documents
---

# LYSCORE-001: Carry the hand-written lys-core design into its three JSON documents

> **Cluster:** lys-core
> **Checklist:**
> - C1 — Root Cargo.toml declares workspace members `crates/lys-core` and `crates/lys`
> - C2 — lys-core Cargo.toml declares ed25519-dalek, x25519-dalek, aes-gcm, rcgen, ct-merkle, x509-parser, sha2, hkdf, rand, serde, thiserror, zeroize, base64, chrono, tracing dependencies — plus ciborium (COSE), postcard (typed leaf encoding), and time
> - C3 — lys-core lib.rs declares public modules: keys, ca, merkle, checkpoint, tlog, attestation, seal, error — and the crate-internal `hex_lower` helper
> - C4 — lib.rs carries `#![cfg_attr(not(test), forbid(unsafe_code))]`, over the workspace-wide `unsafe_code = "deny"`; `forbid` relaxes to `deny` in test builds only so the env-backed tests can call `std::env::set_var` (unsafe in edition 2024) under an explicit `#[allow]`
> - C5 — TrustError enum defined with thiserror: CertificateGeneration, CertificateParsing, CertificateVerification, CertificateRevocation, MerkleTree, Seal, UnsealFailed, AttestationFailed, KeyManagement, Signing, InvalidSignature — plus CheckpointEncoding, CheckpointParsing, VerifierKey, NoteVerification, LogArtifactEncoding, LogArtifactVerification — with `TrustResult<T>` alias
> - C6 — No Meridian reference anywhere in lys-core or lys sources: `grep -ri meridian crates/` returns nothing
> - C7 — Ed25519Identity struct holds SigningKey + VerifyingKey; Debug output contains '[REDACTED]' for the signing key (test exists)
> - C8 — Ed25519Identity::load_or_generate(path) loads a 32-byte seed file or generates and persists one; malformed-length files return KeyManagement errors
> - C9 — Key generation is race-free: seed written to a unique temp file (pid + per-process counter in the name), published via no-clobber `hard_link`; on `AlreadyExists` the loser discards its candidate seed and loads the persisted key (first-writer-wins; concurrent-generation test exists)
> - C10 — On Unix the key file is created mode 0o600; loading a key file with loose permissions emits a warning but still loads
> - C11 — Ed25519Identity::from_env() reads a base64-encoded 32-byte seed from the `LYS_IDENTITY_KEY` environment variable; missing variable, invalid base64, and wrong decoded length each return KeyManagement errors
> - C12 — All seed material in loading, generation, and decode paths is held in `Zeroizing` buffers
> - C13 — Ed25519Identity::sign(message) returns [u8; 64]
> - C14 — Ed25519Identity::verify(public_key, message, signature) uses ed25519-dalek `verify_strict`; no non-strict verification exists in library code anywhere in either crate — the only `verify` calls are inside tests that construct a small-order forgery, prove dalek's non-strict path accepts it, and prove lys rejects it
> - C15 — Ed25519Identity::x25519_public_key() returns the Montgomery-form [u8; 32] and x25519_static_secret() returns the clamped-scalar StaticSecret; Diffie-Hellman between two identities' derived keys agrees from both sides (test exists)
> - C16 — CertificateAuthority::new(identity) wraps Ed25519Identity and exposes public_key_bytes()
> - C17 — issue_certificate(subject, ttl, extensions) returns IssuedCertificate holding DER bytes, subject keypair, SHA-256 fingerprint ([u8; 32] of DER), expiry, and issuer public key
> - C18 — rcgen signing goes through a RemoteKeyPair adapter with PKCS_ED25519 so the CA private seed is never serialised into rcgen's keypair representation
> - C19 — IssuedCertificate Debug output redacts private key material (test exists)
> - C20 — verify_certificate_chain(cert_der, issuer_public_key) extracts TBS bytes with x509-parser and verifies the signature with ed25519-dalek `verify_strict`
> - C21 — Chain verification enforces the validity window: expired certificates and not-yet-valid certificates are both rejected (tests exist for each)
> - C22 — verify_certificate_chain_at(cert_der, issuer_public_key, instant) verifies at an explicit instant; a cert expired now but valid at the given instant passes
> - C23 — Self-signed certificates are rejected by verify_certificate_chain
> - C24 — `LYS_OID_ARC` constant equals [1, 3, 6, 1, 4, 1, 66364] with a doc comment stating that 66364 is the IANA Private Enterprise Number assigned to lys and that the arc is permanent
> - C25 — encode_extension / decode_extension round-trip an arbitrary DER payload under LYS_OID_ARC; decode of a cert without the extension returns Ok(None)
> - C26 — Round-trip test: rcgen-generated Ed25519 keypair is loadable as ed25519-dalek SigningKey/VerifyingKey
> - C27 — AppendOnlyTree<L> generic over leaf type L: Serialize; append(leaf) returns the new tree size
> - C28 — No delete or modify operation exists on the tree — append-only enforced by API
> - C29 — root() returns the current RootHash; the empty tree produces a deterministic empty root hash
> - C30 — prove_inclusion(leaf_index) pre-checks bounds and returns TrustError::MerkleTree on out-of-range index — no panic path into the backing library
> - C31 — prove_consistency(old_size, new_size) pre-checks the size pair (old ≤ new, new ≤ len, old ≥ 1) and returns TrustError::MerkleTree on violation
> - C32 — verify_inclusion(root_hash, leaf, index, proof) and verify_consistency(old_root, new_root, proof) return Result; tampered proofs and mismatched roots fail
> - C33 — RootHash::from_parts(root_hash, num_leaves) and to_parts() round-trip; from_parts requires no tree access
> - C34 — InclusionProof and ConsistencyProof round-trip through as_bytes() / try_from_bytes()
> - C35 — External-verifier round-trip test exists: a verifier holding only published root parts and proof bytes (never the tree) verifies inclusion and consistency
> - C36 — reconstruct_from_leaves(leaves) rebuilds a tree with a root hash identical to the original (test exists)
> - C37 — merkle module docs state the frozen-wire-contract rule: leaf encodings are canonical bytes, evolved only by introducing a new versioned leaf type
> - C38 — sign_attestation(payload, signing_key) signs the COSE `Sig_structure` `["Signature1", protected, h'', claims]` (RFC 9052 §4.4) with protected `{1: -8, 3: "application/vnd.lys.attestation.v2+cbor", 4: signer key}` — no meridian string, no v1 preimage constant remains anywhere
> - C39 — Attestation { payload_hash: [u8; 32], signature: [u8; 64], signer_public_key: [u8; 32], timestamp: i64 } carries no serde; the only durable form is `to_cose_bytes()` / `from_cose_bytes()` (canonical-encoding-strict)
> - C40 — verify_attestation(attestation, payload) rebuilds the `Sig_structure` from the attestation's own fields and verifies with `verify_strict`
> - C41 — No legacy fallback exists: a signature over the bare payload hash and a signature over the deleted v1 preimage both fail verify_attestation (tests exist)
> - C42 — Tampered payload fails verify_attestation
> - C43 — Tampered timestamp fails verify_attestation — the timestamp is a signed claim inside the `Sig_structure` (test exists)
> - C44 — seal(payload, recipient_public_key) returns SealedEnvelope { ephemeral_public_key, ciphertext, nonce } using a fresh ephemeral X25519 keypair per call (two seals of the same payload to the same recipient differ)
> - C45 — HKDF-SHA256 info input is `b"lys-sealed-envelope/v1" || ephemeral_public_key || recipient_public_key` — the hyphen-form HKDF domain tag, deliberately distinct from the slash-form attestation context tag `lys/sealed-envelope/v1`
> - C46 — Both seal and open reject non-contributory Diffie-Hellman: a low-order public key fails via `was_contributory` before any key derivation (test exists)
> - C47 — Seal/open roundtrip succeeds: sealed with the recipient's X25519 public key, opened with the recipient's static secret
> - C48 — Wrong private key, tampered ciphertext, and tampered nonce all return exactly TrustError::UnsealFailed — a single undifferentiated failure through the AES-GCM arbiter, with no early return distinguishing causes
> - C49 — SealedEnvelope::attestation_bytes() covers every wire byte of the envelope (ephemeral key, nonce, ciphertext)
> - C50 — sign_and_seal(payload, sender_identity, recipient_x25519_public_key) returns (SealedEnvelope, Attestation) where the attestation signs attestation_bytes()
> - C51 — open_and_verify verifies the attestation before any decryption: an invalid sender signature is rejected without the cipher being touched, and a valid signature over a tampered envelope also fails (tests exist)
> - C52 — `lys` binary crate exists; main.rs is a thin entry (parse, dispatch, exit codes) with clap definitions isolated in cli.rs; no `anyhow` anywhere — the CLI carries its own thiserror type
> - C53 — `lys key` generates an identity at a path and inspects one (public key, fingerprint); no subcommand, flag, or output format prints private key material (test asserts output contains no seed bytes in any encoding)
> - C54 — `lys ca issue` issues a certificate signed by an issuer identity file, embedding a caller-supplied capability-claim payload as a LYS_OID_ARC extension, and writes the PEM out
> - C55 — `lys ca verify` verifies a certificate against an issuer public key, and accepts an explicit verification instant flag routing to verify_certificate_chain_at
> - C56 — `lys attest` signs a payload file and emits the COSE_Sign1 artifact; `lys verify` checks an artifact against a payload and reports success/failure via exit code. (File paths only as built — the stdin path this item originally anticipated was not implemented, and ROADMAP Phase 2 records the file-only surface.)
> - C57 — `lys seal` seals a payload file for a recipient public key and writes the sender attestation alongside it; `lys open` opens it with the recipient identity, verifying the attestation first; the pair round-trips
> - C58 — `lys log init` pins the log's origin exactly once and refuses to re-initialize; `lys log append` appends a leaf file's raw bytes and prints the new root; `lys log checkpoint` signs a C2SP tlog-checkpoint in the signed-note envelope over the current root; `lys log prove` emits a self-contained JSON proof artifact with the relevant signed checkpoint(s) embedded verbatim
> - C59 — `lys log verify` verifies an inclusion or consistency claim from **only** the artifact, the leaf, and the verifier key — no access to the leaf sequence, the store, or the tree; declared sizes are checked against the signature-verified checkpoint and roots are recomputed, never trusted; every tamper class collapses to one identical message
> - C60 — Cross-process CLI test exists: a log produced by one process is verified end-to-end by the CLI in another process with no access to the original tree
> - C61 — cargo fmt --check passes clean *(gate — verified by CI, not from source)*
> - C62 — cargo clippy --all-targets -- -D warnings passes clean *(gate — verified by CI, not from source)*
> - C63 — cargo test --workspace passes green *(gate — verified by CI, not from source)*
> - C64 — No file exceeds 500 lines of code; every mod.rs carries only pub mod / pub use / module docs; tests live in sibling *_tests.rs files *(not met: `merkle/leaf.rs`, both `seal/` files, `error.rs`, and six CLI modules carry inline `mod tests` — REVIEW-23-07.md F12)*
> - C65 — lys-core builds standalone with zero meridian-* dependencies in its Cargo.toml and Cargo.lock
> **Stories:**
> - S1 (Norn Agent Runtime, Signing Session History (primary consumer)) — As the Norn runtime, I want a signing `PersistenceSink` decorator to attest each session event with the agent's Ed25519 identity so that every persisted event carries a verifiable, timestamp-authenticated signature without any core-loop change.
> - S2 (Norn Agent Runtime, Signing Session History (primary consumer)) — As the Norn runtime, I want to append each event's hash to a per-session `AppendOnlyTree` so that the session history becomes tamper-evident from the first event.
> - S3 (Norn Agent Runtime, Signing Session History (primary consumer)) — As the Norn runtime, I want to export the session root via `RootHash::to_parts()` at every `checkpoint()` so that the 32-byte root can later be anchored externally without revealing session contents.
> - S4 (Norn Agent Runtime, Signing Session History (primary consumer)) — As the Norn runtime, I want to issue an X.509 certificate at agent spawn with capability claims embedded as a `LYS_OID_ARC` extension so that the agent's identity and permissions are one presentable object.
> - S5 (Norn Agent Runtime, Signing Session History (primary consumer)) — As the Norn runtime, I want to verify a counterparty's certificate chain at the MCP boundary — at the dispatch instant, via `verify_certificate_chain_at` — so that cross-agent tool calls are gated on legitimate, unexpired, capability-scoped identity.
> - S6 (Norn Agent Runtime, Signing Session History (primary consumer)) — As the Norn runtime, I want spawned agents to load their identity from `LYS_IDENTITY_KEY` so that key material reaches an agent process through its environment without touching shared disk.
> - S7 (Norn Agent Runtime, Signing Session History (primary consumer)) — As the Norn runtime, I want `reconstruct_from_leaves` to rebuild a session tree from the persisted event sequence after a crash so that the recovered tree's root matches the last checkpointed root exactly.
> - S8 (Norn Agent Runtime, Signing Session History (primary consumer)) — As the Norn runtime, I want concurrent agent processes calling `load_or_generate` on the same key path to converge on one persisted key so that a spawn race never leaves an agent holding an identity that differs from the key on disk.
> - S9 (Lys CLI, Operator and Auditor) — As an operator, I want `lys key` to generate and inspect identities without ever printing private material so that key handling over a shoulder-surfable terminal is safe by construction.
> - S10 (Lys CLI, Operator and Auditor) — As an operator, I want `lys ca issue` to mint certificates with capability-claim extensions from my instance CA key so that I can provision agent identities from the command line.
> - S11 (Lys CLI, Operator and Auditor) — As an auditor, I want `lys ca verify` with an explicit instant so that I can check whether a certificate was valid at the time a disputed action occurred, not just at the time of my audit.
> - S12 (Lys CLI, Operator and Auditor) — As an operator, I want `lys attest` to sign a file or stdin so that I can hand a third party a detached, self-contained attestation over any artifact.
> - S13 (Lys CLI, Operator and Auditor) — As an auditor, I want `lys verify` to check an attestation with only the payload and the signer's public key so that verification requires nothing from the party who produced the record.
> - S14 (Lys CLI, Operator and Auditor) — As an auditor, I want `lys log verify` to prove inclusion from only a published root and proof bytes so that I can confirm a challenged entry was logged without the operator's cooperation and without seeing any other entry.
> - S15 (Lys CLI, Operator and Auditor) — As an auditor, I want `lys log verify` to check consistency between two published roots so that I can detect any rewrite of history between two points in time.
> - S16 (Lys CLI, Operator and Auditor) — As an operator, I want `lys seal` and `lys open` so that I can move a credential file to a specific recipient with per-envelope forward secrecy instead of pasting secrets into a chat.
> - S17 (Lys-Anchor Service, (future notary)) — As the anchor service, I want to reconstruct submitted roots via `RootHash::from_parts` so that I can verify consistency proofs between an instance's successive submissions while seeing only roots, never contents.
> - S18 (Lys-Anchor Service, (future notary)) — As the anchor service, I want to verify the submitter's v1 domain-separated attestation over each submitted root so that only the holder of the registered instance key can extend that instance's anchored history.
> - S19 (Lys-Anchor Service, (future notary)) — As the anchor service, I want to append anchored roots to my own `AppendOnlyTree` and serve inclusion proofs so that my receipt for an anchoring event is itself independently verifiable.
> - S20 (Lys-Anchor Service, (future notary)) — As the anchor service, I want the wire tags and preimage layouts frozen at `v1` so that a receipt issued today still verifies against signatures produced years from now.
> - S21 (Haematite, Commit Attestation) — As haematite, I want to attest a BLAKE3 commit root — 32 opaque bytes signed with the instance identity — so that a whole database state becomes attestable without lys knowing anything about haematite's hash world.
> - S22 (Haematite, Commit Attestation) — As haematite, I want to append commit attestations to an append-only log so that my flat timestamped commit list gains the hash-chained lineage it structurally lacks.
> - S23 (Haematite, Commit Attestation) — As haematite, I want inclusion proofs over the commit log so that any party can verify a historical commit belongs to the canonical lineage without replaying the database.

## Purpose

The lys-core design on main is three hand-written documents, DESIGN.md, CHECKLIST.md and USER-STORIES.md, with no JSON sources. The design gate renders every cluster that has a design.json and compares the render with the committed markdown, so the first lys-core brief to add a design.json would replace the hand-written documents with whatever that JSON holds and the older design would leave the record. This brief carries the whole hand-written design into design.json, checklist.json and stories.json first, so that the render reproduces it: every C id and text, every S id and text, and every section of DESIGN.md in the field the design schema gives it. Nothing new is designed and no code changes.

## Task

Carry main's docs/design/lys-core/DESIGN.md, CHECKLIST.md and USER-STORIES.md at 7b536253 into docs/design/lys-core/design.json, checklist.json and stories.json (R1 to R3), render the cluster with python3 scripts/design/render-cluster.py docs/design/lys-core, commit the rendered DESIGN.md, CHECKLIST.md, USER-STORIES.md and briefs/LYSCORE-001.md from that render (R4), and pass sh scripts/design/gate.sh. Every path is relative to the repository root.

The three JSON documents are written on the brief branch with this brief, as the method writes a cluster's design beside its first brief. The build starts from them: it runs the measurements in R1 to R3 against main's bytes at 7b536253, corrects any carried text the measurements show differs from main so that it matches main exactly, and renders. Write each JSON document with json.dumps(doc, indent=2, ensure_ascii=False) and one trailing newline, so an en dash stays an en dash and a backtick stays a backtick.

What is carried, and where. Nothing is reworded, merged, dropped, renumbered or reordered, and stale text is carried as it stands (S12's stdin, S14's root parts, and the structure tree's missing delegation/, receipt/, bundle/, ca/request.rs and keys/ssh.rs are corrected by a later unit, not here). The constraints take ids CN1 to CN9 in main's order because the schema requires an id; the goals render as bullets in main's order because the renderer writes goals that way; the fenced structure tree becomes one structure row per entry, 75 rows (73 entries under the two roots crates/lys-core/ and crates/lys/, and the two roots), with each path spelled as the tree spells it (directory entries keep their trailing slash, *_tests.rs globs stay globs), continuation lines joined into the note with one space, and every brief field empty because every one of those paths exists on main; the D1 to D8 subsections stay inside the solution text, not in decisions, because they are not ledger ADRs.

Sentences with no field of their own, named: (a) the paragraph under main's structure tree that begins 'Tests live in sibling' has no field after a structure table, so it goes into the solution text, as its own paragraph after the last D8 paragraph; (b) CHECKLIST.md's three-paragraph intro blockquote and its two italic annotations, the one above C38 that covers C38 to C43 and the one above C58 that covers C58 to C59, have no field in checklist.json, so they go into design.json as one section at the end of the solution text headed '### Checklist notes', carried verbatim, each annotation opening with the ids it covers as main's does; they do not go into any checklist section name, and the eight checklist headings stay exactly as main has them; (c) three persona headings have no ' — ' between a name and a role, and the renderer always writes one, so each is split into name and role at the point given in R2. The dev record says each of these, where it went, and why.

Differences the render makes against main, which the R3 dev record lists by name with its reason. Layout: the frontmatter title loses its quotes; the renderer adds the '> **Cluster:** lys-core' line under the heading; the 'Tests live in sibling' paragraph moves from under the structure tree to the end of the D8 text; goals change from '1.' to '6.' numbering to bullets; the structure tree becomes a Path, Note, Brief table; each constraint gains its '**CNn** —' prefix. Content, the two the card's lead allows: the '### Checklist notes' section in the solution (carrying (b) above, so the rendered DESIGN.md holds it and the rendered CHECKLIST.md does not), and the eight structure rows for the cluster's own documents and this brief's own files, listed as the directory cluster lists its own, so that check-coverage.py passes and R1 to R4 name real files. Every other difference between the rendered DESIGN.md and main's is layout only. The dev record also lists the rendered CHECKLIST.md's eleven missing lines and the rendered USER-STORIES.md's three changed headings.

Coverage. This brief claims C1 to C65 and S1 to S23 because check-coverage.py fails every checklist item and story that no brief in the cluster claims. It carries them; it did not build them. The done flags record main's ticks unchanged: C1 to C60 and C65 true, C61 to C64 false. The later lys-core card that waits on this one adds its own brief, items and stories after the carried ones; none of them is added here.

Out of scope: any new checklist item, user story, principle, ADR, goal, non-goal or constraint; any change to code, to crates/, to scripts/design/ (renderer, validator, schemas) or to any other cluster; a note field for checklists and personas in the method's renderer and schemas.

## Requirements

### R1: Carry CHECKLIST.md into checklist.json and render it

docs/design/lys-core/checklist.json holds cluster 'lys-core' and main's eight CHECKLIST.md sections at 7b536253, each named exactly as main names it and in main's order: Crate Setup, Key Management, Certificate Authority, Merkle Transparency Log, Signed Attestations, Sealed Envelope, CLI Surface, Integration Verification. Each of main's item lines '- [x] **Cn** — text' and '- [ ] **Cn** — text' becomes one item {id, text, done} in its section and in main's order, where text is every character after the first ' — ' byte for byte (C61 to C64 keep their italic tails inside the text) and done is true for '[x]' and false for '[ ]'. The document SHALL NOT reword, merge, drop, renumber, reorder or add any item, SHALL NOT change any section name, and SHALL NOT hold the intro blockquote and the two italic annotations in any section name or item text. WHEN python3 scripts/design/render-cluster.py docs/design/lys-core runs, THE SYSTEM SHALL write docs/design/lys-core/CHECKLIST.md whose eight headings and 65 item lines equal main's byte for byte, and SHALL NOT write any line main's CHECKLIST.md does not hold.

**Acceptance:**
- From the repository root: python3 -c "import json; d=json.load(open('docs/design/lys-core/checklist.json')); ids=[i['id'] for s in d['sections'] for i in s['items']]; print(len(ids), len(set(ids)), ids==['C%d' % n for n in range(1, 66)])" prints 65 65 True.
- From the repository root: python3 -c "import json; d=json.load(open('docs/design/lys-core/checklist.json')); print([i['id'] for s in d['sections'] for i in s['items'] if not i['done']])" prints ['C61', 'C62', 'C63', 'C64'].
- From the repository root: python3 -c "import json; print([s['name'] for s in json.load(open('docs/design/lys-core/checklist.json'))['sections']])" prints ['Crate Setup', 'Key Management', 'Certificate Authority', 'Merkle Transparency Log', 'Signed Attestations', 'Sealed Envelope', 'CLI Surface', 'Integration Verification'].
- From the repository root: the C and S text comparison in this brief's verification prints 'compared 88 differences 0' as its last line.
- From the repository root: git show 7b536253:docs/design/lys-core/CHECKLIST.md | diff - docs/design/lys-core/CHECKLIST.md | grep -c '^<' prints 11 (the five intro blockquote lines, their following blank line, and the two annotations with their three blank lines), and the same pipeline with grep -c '^>' prints 0.

**Files:**
- modify: docs/design/lys-core/checklist.json
- modify: docs/design/lys-core/CHECKLIST.md

**Checklist:**
- C1 — Root Cargo.toml declares workspace members `crates/lys-core` and `crates/lys`
- C2 — lys-core Cargo.toml declares ed25519-dalek, x25519-dalek, aes-gcm, rcgen, ct-merkle, x509-parser, sha2, hkdf, rand, serde, thiserror, zeroize, base64, chrono, tracing dependencies — plus ciborium (COSE), postcard (typed leaf encoding), and time
- C3 — lys-core lib.rs declares public modules: keys, ca, merkle, checkpoint, tlog, attestation, seal, error — and the crate-internal `hex_lower` helper
- C4 — lib.rs carries `#![cfg_attr(not(test), forbid(unsafe_code))]`, over the workspace-wide `unsafe_code = "deny"`; `forbid` relaxes to `deny` in test builds only so the env-backed tests can call `std::env::set_var` (unsafe in edition 2024) under an explicit `#[allow]`
- C5 — TrustError enum defined with thiserror: CertificateGeneration, CertificateParsing, CertificateVerification, CertificateRevocation, MerkleTree, Seal, UnsealFailed, AttestationFailed, KeyManagement, Signing, InvalidSignature — plus CheckpointEncoding, CheckpointParsing, VerifierKey, NoteVerification, LogArtifactEncoding, LogArtifactVerification — with `TrustResult<T>` alias
- C6 — No Meridian reference anywhere in lys-core or lys sources: `grep -ri meridian crates/` returns nothing
- C7 — Ed25519Identity struct holds SigningKey + VerifyingKey; Debug output contains '[REDACTED]' for the signing key (test exists)
- C8 — Ed25519Identity::load_or_generate(path) loads a 32-byte seed file or generates and persists one; malformed-length files return KeyManagement errors
- C9 — Key generation is race-free: seed written to a unique temp file (pid + per-process counter in the name), published via no-clobber `hard_link`; on `AlreadyExists` the loser discards its candidate seed and loads the persisted key (first-writer-wins; concurrent-generation test exists)
- C10 — On Unix the key file is created mode 0o600; loading a key file with loose permissions emits a warning but still loads
- C11 — Ed25519Identity::from_env() reads a base64-encoded 32-byte seed from the `LYS_IDENTITY_KEY` environment variable; missing variable, invalid base64, and wrong decoded length each return KeyManagement errors
- C12 — All seed material in loading, generation, and decode paths is held in `Zeroizing` buffers
- C13 — Ed25519Identity::sign(message) returns [u8; 64]
- C14 — Ed25519Identity::verify(public_key, message, signature) uses ed25519-dalek `verify_strict`; no non-strict verification exists in library code anywhere in either crate — the only `verify` calls are inside tests that construct a small-order forgery, prove dalek's non-strict path accepts it, and prove lys rejects it
- C15 — Ed25519Identity::x25519_public_key() returns the Montgomery-form [u8; 32] and x25519_static_secret() returns the clamped-scalar StaticSecret; Diffie-Hellman between two identities' derived keys agrees from both sides (test exists)
- C16 — CertificateAuthority::new(identity) wraps Ed25519Identity and exposes public_key_bytes()
- C17 — issue_certificate(subject, ttl, extensions) returns IssuedCertificate holding DER bytes, subject keypair, SHA-256 fingerprint ([u8; 32] of DER), expiry, and issuer public key
- C18 — rcgen signing goes through a RemoteKeyPair adapter with PKCS_ED25519 so the CA private seed is never serialised into rcgen's keypair representation
- C19 — IssuedCertificate Debug output redacts private key material (test exists)
- C20 — verify_certificate_chain(cert_der, issuer_public_key) extracts TBS bytes with x509-parser and verifies the signature with ed25519-dalek `verify_strict`
- C21 — Chain verification enforces the validity window: expired certificates and not-yet-valid certificates are both rejected (tests exist for each)
- C22 — verify_certificate_chain_at(cert_der, issuer_public_key, instant) verifies at an explicit instant; a cert expired now but valid at the given instant passes
- C23 — Self-signed certificates are rejected by verify_certificate_chain
- C24 — `LYS_OID_ARC` constant equals [1, 3, 6, 1, 4, 1, 66364] with a doc comment stating that 66364 is the IANA Private Enterprise Number assigned to lys and that the arc is permanent
- C25 — encode_extension / decode_extension round-trip an arbitrary DER payload under LYS_OID_ARC; decode of a cert without the extension returns Ok(None)
- C26 — Round-trip test: rcgen-generated Ed25519 keypair is loadable as ed25519-dalek SigningKey/VerifyingKey
- C27 — AppendOnlyTree<L> generic over leaf type L: Serialize; append(leaf) returns the new tree size
- C28 — No delete or modify operation exists on the tree — append-only enforced by API
- C29 — root() returns the current RootHash; the empty tree produces a deterministic empty root hash
- C30 — prove_inclusion(leaf_index) pre-checks bounds and returns TrustError::MerkleTree on out-of-range index — no panic path into the backing library
- C31 — prove_consistency(old_size, new_size) pre-checks the size pair (old ≤ new, new ≤ len, old ≥ 1) and returns TrustError::MerkleTree on violation
- C32 — verify_inclusion(root_hash, leaf, index, proof) and verify_consistency(old_root, new_root, proof) return Result; tampered proofs and mismatched roots fail
- C33 — RootHash::from_parts(root_hash, num_leaves) and to_parts() round-trip; from_parts requires no tree access
- C34 — InclusionProof and ConsistencyProof round-trip through as_bytes() / try_from_bytes()
- C35 — External-verifier round-trip test exists: a verifier holding only published root parts and proof bytes (never the tree) verifies inclusion and consistency
- C36 — reconstruct_from_leaves(leaves) rebuilds a tree with a root hash identical to the original (test exists)
- C37 — merkle module docs state the frozen-wire-contract rule: leaf encodings are canonical bytes, evolved only by introducing a new versioned leaf type
- C38 — sign_attestation(payload, signing_key) signs the COSE `Sig_structure` `["Signature1", protected, h'', claims]` (RFC 9052 §4.4) with protected `{1: -8, 3: "application/vnd.lys.attestation.v2+cbor", 4: signer key}` — no meridian string, no v1 preimage constant remains anywhere
- C39 — Attestation { payload_hash: [u8; 32], signature: [u8; 64], signer_public_key: [u8; 32], timestamp: i64 } carries no serde; the only durable form is `to_cose_bytes()` / `from_cose_bytes()` (canonical-encoding-strict)
- C40 — verify_attestation(attestation, payload) rebuilds the `Sig_structure` from the attestation's own fields and verifies with `verify_strict`
- C41 — No legacy fallback exists: a signature over the bare payload hash and a signature over the deleted v1 preimage both fail verify_attestation (tests exist)
- C42 — Tampered payload fails verify_attestation
- C43 — Tampered timestamp fails verify_attestation — the timestamp is a signed claim inside the `Sig_structure` (test exists)
- C44 — seal(payload, recipient_public_key) returns SealedEnvelope { ephemeral_public_key, ciphertext, nonce } using a fresh ephemeral X25519 keypair per call (two seals of the same payload to the same recipient differ)
- C45 — HKDF-SHA256 info input is `b"lys-sealed-envelope/v1" || ephemeral_public_key || recipient_public_key` — the hyphen-form HKDF domain tag, deliberately distinct from the slash-form attestation context tag `lys/sealed-envelope/v1`
- C46 — Both seal and open reject non-contributory Diffie-Hellman: a low-order public key fails via `was_contributory` before any key derivation (test exists)
- C47 — Seal/open roundtrip succeeds: sealed with the recipient's X25519 public key, opened with the recipient's static secret
- C48 — Wrong private key, tampered ciphertext, and tampered nonce all return exactly TrustError::UnsealFailed — a single undifferentiated failure through the AES-GCM arbiter, with no early return distinguishing causes
- C49 — SealedEnvelope::attestation_bytes() covers every wire byte of the envelope (ephemeral key, nonce, ciphertext)
- C50 — sign_and_seal(payload, sender_identity, recipient_x25519_public_key) returns (SealedEnvelope, Attestation) where the attestation signs attestation_bytes()
- C51 — open_and_verify verifies the attestation before any decryption: an invalid sender signature is rejected without the cipher being touched, and a valid signature over a tampered envelope also fails (tests exist)
- C52 — `lys` binary crate exists; main.rs is a thin entry (parse, dispatch, exit codes) with clap definitions isolated in cli.rs; no `anyhow` anywhere — the CLI carries its own thiserror type
- C53 — `lys key` generates an identity at a path and inspects one (public key, fingerprint); no subcommand, flag, or output format prints private key material (test asserts output contains no seed bytes in any encoding)
- C54 — `lys ca issue` issues a certificate signed by an issuer identity file, embedding a caller-supplied capability-claim payload as a LYS_OID_ARC extension, and writes the PEM out
- C55 — `lys ca verify` verifies a certificate against an issuer public key, and accepts an explicit verification instant flag routing to verify_certificate_chain_at
- C56 — `lys attest` signs a payload file and emits the COSE_Sign1 artifact; `lys verify` checks an artifact against a payload and reports success/failure via exit code. (File paths only as built — the stdin path this item originally anticipated was not implemented, and ROADMAP Phase 2 records the file-only surface.)
- C57 — `lys seal` seals a payload file for a recipient public key and writes the sender attestation alongside it; `lys open` opens it with the recipient identity, verifying the attestation first; the pair round-trips
- C58 — `lys log init` pins the log's origin exactly once and refuses to re-initialize; `lys log append` appends a leaf file's raw bytes and prints the new root; `lys log checkpoint` signs a C2SP tlog-checkpoint in the signed-note envelope over the current root; `lys log prove` emits a self-contained JSON proof artifact with the relevant signed checkpoint(s) embedded verbatim
- C59 — `lys log verify` verifies an inclusion or consistency claim from **only** the artifact, the leaf, and the verifier key — no access to the leaf sequence, the store, or the tree; declared sizes are checked against the signature-verified checkpoint and roots are recomputed, never trusted; every tamper class collapses to one identical message
- C60 — Cross-process CLI test exists: a log produced by one process is verified end-to-end by the CLI in another process with no access to the original tree
- C61 — cargo fmt --check passes clean *(gate — verified by CI, not from source)*
- C62 — cargo clippy --all-targets -- -D warnings passes clean *(gate — verified by CI, not from source)*
- C63 — cargo test --workspace passes green *(gate — verified by CI, not from source)*
- C64 — No file exceeds 500 lines of code; every mod.rs carries only pub mod / pub use / module docs; tests live in sibling *_tests.rs files *(not met: `merkle/leaf.rs`, both `seal/` files, `error.rs`, and six CLI modules carry inline `mod tests` — REVIEW-23-07.md F12)*
- C65 — lys-core builds standalone with zero meridian-* dependencies in its Cargo.toml and Cargo.lock

### R2: Carry USER-STORIES.md into stories.json and render it

docs/design/lys-core/stories.json holds cluster 'lys-core' and main's four USER-STORIES.md persona headings at 7b536253 as four personas in main's order, each split into name and role: 'Norn Agent Runtime — Signing Session History (primary consumer)' at its ' — ' into name 'Norn Agent Runtime' and role 'Signing Session History (primary consumer)'; 'Lys CLI Operator and Auditor' into name 'Lys CLI' and role 'Operator and Auditor'; 'Lys-Anchor Service (future notary)' into name 'Lys-Anchor Service' and role '(future notary)'; 'Haematite Commit Attestation' into name 'Haematite' and role 'Commit Attestation'. Each of main's story lines '**Sn.** text' becomes one story {id, text} under its persona and in main's order, where text is every character after '**Sn.** ' byte for byte. The document SHALL NOT reword, merge, drop, renumber, reorder or add any story, and SHALL NOT drop or add any word of a persona heading. WHEN python3 scripts/design/render-cluster.py docs/design/lys-core runs, THE SYSTEM SHALL write docs/design/lys-core/USER-STORIES.md whose 23 story lines equal main's byte for byte and whose only changed lines are the three headings that gain ' — '.

**Acceptance:**
- From the repository root: python3 -c "import json; d=json.load(open('docs/design/lys-core/stories.json')); ids=[s['id'] for p in d['personas'] for s in p['stories']]; print(len(ids), len(set(ids)), ids==['S%d' % n for n in range(1, 24)])" prints 23 23 True.
- From the repository root: python3 -c "import json; print([(p['name'], p['role']) for p in json.load(open('docs/design/lys-core/stories.json'))['personas']])" prints [('Norn Agent Runtime', 'Signing Session History (primary consumer)'), ('Lys CLI', 'Operator and Auditor'), ('Lys-Anchor Service', '(future notary)'), ('Haematite', 'Commit Attestation')].
- From the repository root: the C and S text comparison in this brief's verification prints 'compared 88 differences 0' as its last line.
- From the repository root: git show 7b536253:docs/design/lys-core/USER-STORIES.md | diff - docs/design/lys-core/USER-STORIES.md | grep '^>' prints exactly the three lines '> ## Lys CLI — Operator and Auditor', '> ## Lys-Anchor Service — (future notary)' and '> ## Haematite — Commit Attestation', and the same pipeline with grep -c '^<' prints 3.

**Files:**
- modify: docs/design/lys-core/stories.json
- modify: docs/design/lys-core/USER-STORIES.md

**Stories:**
- S1 (Norn Agent Runtime, Signing Session History (primary consumer)) — As the Norn runtime, I want a signing `PersistenceSink` decorator to attest each session event with the agent's Ed25519 identity so that every persisted event carries a verifiable, timestamp-authenticated signature without any core-loop change.
- S2 (Norn Agent Runtime, Signing Session History (primary consumer)) — As the Norn runtime, I want to append each event's hash to a per-session `AppendOnlyTree` so that the session history becomes tamper-evident from the first event.
- S3 (Norn Agent Runtime, Signing Session History (primary consumer)) — As the Norn runtime, I want to export the session root via `RootHash::to_parts()` at every `checkpoint()` so that the 32-byte root can later be anchored externally without revealing session contents.
- S4 (Norn Agent Runtime, Signing Session History (primary consumer)) — As the Norn runtime, I want to issue an X.509 certificate at agent spawn with capability claims embedded as a `LYS_OID_ARC` extension so that the agent's identity and permissions are one presentable object.
- S5 (Norn Agent Runtime, Signing Session History (primary consumer)) — As the Norn runtime, I want to verify a counterparty's certificate chain at the MCP boundary — at the dispatch instant, via `verify_certificate_chain_at` — so that cross-agent tool calls are gated on legitimate, unexpired, capability-scoped identity.
- S6 (Norn Agent Runtime, Signing Session History (primary consumer)) — As the Norn runtime, I want spawned agents to load their identity from `LYS_IDENTITY_KEY` so that key material reaches an agent process through its environment without touching shared disk.
- S7 (Norn Agent Runtime, Signing Session History (primary consumer)) — As the Norn runtime, I want `reconstruct_from_leaves` to rebuild a session tree from the persisted event sequence after a crash so that the recovered tree's root matches the last checkpointed root exactly.
- S8 (Norn Agent Runtime, Signing Session History (primary consumer)) — As the Norn runtime, I want concurrent agent processes calling `load_or_generate` on the same key path to converge on one persisted key so that a spawn race never leaves an agent holding an identity that differs from the key on disk.
- S9 (Lys CLI, Operator and Auditor) — As an operator, I want `lys key` to generate and inspect identities without ever printing private material so that key handling over a shoulder-surfable terminal is safe by construction.
- S10 (Lys CLI, Operator and Auditor) — As an operator, I want `lys ca issue` to mint certificates with capability-claim extensions from my instance CA key so that I can provision agent identities from the command line.
- S11 (Lys CLI, Operator and Auditor) — As an auditor, I want `lys ca verify` with an explicit instant so that I can check whether a certificate was valid at the time a disputed action occurred, not just at the time of my audit.
- S12 (Lys CLI, Operator and Auditor) — As an operator, I want `lys attest` to sign a file or stdin so that I can hand a third party a detached, self-contained attestation over any artifact.
- S13 (Lys CLI, Operator and Auditor) — As an auditor, I want `lys verify` to check an attestation with only the payload and the signer's public key so that verification requires nothing from the party who produced the record.
- S14 (Lys CLI, Operator and Auditor) — As an auditor, I want `lys log verify` to prove inclusion from only a published root and proof bytes so that I can confirm a challenged entry was logged without the operator's cooperation and without seeing any other entry.
- S15 (Lys CLI, Operator and Auditor) — As an auditor, I want `lys log verify` to check consistency between two published roots so that I can detect any rewrite of history between two points in time.
- S16 (Lys CLI, Operator and Auditor) — As an operator, I want `lys seal` and `lys open` so that I can move a credential file to a specific recipient with per-envelope forward secrecy instead of pasting secrets into a chat.
- S17 (Lys-Anchor Service, (future notary)) — As the anchor service, I want to reconstruct submitted roots via `RootHash::from_parts` so that I can verify consistency proofs between an instance's successive submissions while seeing only roots, never contents.
- S18 (Lys-Anchor Service, (future notary)) — As the anchor service, I want to verify the submitter's v1 domain-separated attestation over each submitted root so that only the holder of the registered instance key can extend that instance's anchored history.
- S19 (Lys-Anchor Service, (future notary)) — As the anchor service, I want to append anchored roots to my own `AppendOnlyTree` and serve inclusion proofs so that my receipt for an anchoring event is itself independently verifiable.
- S20 (Lys-Anchor Service, (future notary)) — As the anchor service, I want the wire tags and preimage layouts frozen at `v1` so that a receipt issued today still verifies against signatures produced years from now.
- S21 (Haematite, Commit Attestation) — As haematite, I want to attest a BLAKE3 commit root — 32 opaque bytes signed with the instance identity — so that a whole database state becomes attestable without lys knowing anything about haematite's hash world.
- S22 (Haematite, Commit Attestation) — As haematite, I want to append commit attestations to an append-only log so that my flat timestamped commit list gains the hash-chained lineage it structurally lacks.
- S23 (Haematite, Commit Attestation) — As haematite, I want inclusion proofs over the commit log so that any party can verify a historical commit belongs to the canonical lineage without replaying the database.

### R3: Carry DESIGN.md into design.json and render it

docs/design/lys-core/design.json holds cluster 'lys-core' and main's DESIGN.md at 7b536253 in the fields the design schema gives it: title 'Lys Core — Trust Primitives and CLI Surface' (the frontmatter title without its quotes); intention, the two Intention paragraphs; problem, the whole Problem section (opening paragraph, four bullets, closing paragraph); solution, the whole Solution section from '### D1: Domain-agnostic boundary' to the paragraph beginning 'The phase proof', then the paragraph under main's structure tree beginning 'Tests live in sibling', then a section headed '### Checklist notes' holding main's CHECKLIST.md intro blockquote (its five lines as main writes them) and its two italic annotations, the C38 to C43 one first, each carried verbatim and separated by one blank line; goals, the six goals in main's order without their '1. ' to '6. ' numbering; non_goals, the seven bullets in main's order, each whole bullet after '- ' as text and reason empty; structure, 75 rows for the fenced tree's two roots and 73 entries in the tree's order, each path the entry's full repository-relative path as the tree spells it, each note the entry's comment after '— ' with continuation lines joined by one space (empty when the entry has none), each brief empty, followed by eight rows, in this order and exactly as written here (path 'docs/design/lys-core/design.json' with note 'the lys-core design, carried from main's DESIGN.md' and brief 'LYSCORE-001'; path 'docs/design/lys-core/DESIGN.md' with note 'rendered markdown' and brief empty; path 'docs/design/lys-core/checklist.json' with note 'the lys-core checklist, carried from main's CHECKLIST.md' and brief 'LYSCORE-001'; path 'docs/design/lys-core/CHECKLIST.md' with note 'rendered markdown' and brief empty; path 'docs/design/lys-core/stories.json' with note 'the lys-core user stories, carried from main's USER-STORIES.md' and brief 'LYSCORE-001'; path 'docs/design/lys-core/USER-STORIES.md' with note 'rendered markdown' and brief empty; path 'docs/design/lys-core/briefs/LYSCORE-001.json' with note 'the brief that carries the hand-written lys-core design into the three JSON documents' and brief 'LYSCORE-001'; path 'docs/design/lys-core/briefs/LYSCORE-001.md' with note 'rendered markdown' and brief 'LYSCORE-001'); constraints, the nine bullets in main's order as CN1 to CN9, each whole bullet after '- ' as text; principles, decisions and inventory empty; and gate, the project's one tree entry for '.' with its shorthand and its seven legs fmt, clippy-all-features, clippy, tests, doc-all-features, doc and design. It SHALL NOT reword, merge, drop, reorder or add any sentence of main's DESIGN.md, SHALL NOT correct stale carried text, SHALL NOT change any relative link (../../ROADMAP.md, ../WIRE-FORMATS.md, ../../PEN-REGISTRATION.md, ../../REVIEW-23-07.md, DESIGN.md), and SHALL NOT add any principle, ADR, goal, non-goal, constraint, inventory row, structure row beyond the eight named, and SHALL NOT add any word beyond the '### Checklist notes' heading and the eight rows' paths, notes and brief values as stated here. WHEN python3 scripts/design/render-cluster.py docs/design/lys-core runs, THE SYSTEM SHALL write docs/design/lys-core/DESIGN.md, and the build's dev record SHALL list every difference between it and main's DESIGN.md with its reason: six of layout (unquoted title, the added '> **Cluster:** lys-core' line, the moved 'Tests live in sibling' paragraph, goals as bullets, the structure tree as a table, the CN prefixes) and two of content that the card's lead allows (the '### Checklist notes' section, the eight rows for the cluster's own documents and this brief's own files).

**Acceptance:**
- From the repository root: the field comparison in this brief's verification prints 'carried fields 7 differences []'.
- From the repository root: the structure tree comparison in this brief's verification prints '75 75 True True'.
- From the repository root: python3 -c "import json; d=json.load(open('docs/design/lys-core/design.json')); s=d['structure']; print(len(s), [r['brief'] for r in s[:75]] == [''] * 75)" prints 83 True.
- From the repository root: python3 -c "import json; print([(r['path'], r['note'], r['brief']) for r in json.load(open('docs/design/lys-core/design.json'))['structure'][75:]] == [('docs/design/lys-core/design.json', \"the lys-core design, carried from main's DESIGN.md\", 'LYSCORE-001'), ('docs/design/lys-core/DESIGN.md', 'rendered markdown', ''), ('docs/design/lys-core/checklist.json', \"the lys-core checklist, carried from main's CHECKLIST.md\", 'LYSCORE-001'), ('docs/design/lys-core/CHECKLIST.md', 'rendered markdown', ''), ('docs/design/lys-core/stories.json', \"the lys-core user stories, carried from main's USER-STORIES.md\", 'LYSCORE-001'), ('docs/design/lys-core/USER-STORIES.md', 'rendered markdown', ''), ('docs/design/lys-core/briefs/LYSCORE-001.json', 'the brief that carries the hand-written lys-core design into the three JSON documents', 'LYSCORE-001'), ('docs/design/lys-core/briefs/LYSCORE-001.md', 'rendered markdown', 'LYSCORE-001')])" prints True, the eight rows' paths, notes and brief values being exactly those stated in the spec, in its order.
- From the repository root: python3 -c "import json; d=json.load(open('docs/design/lys-core/design.json')); print(d['principles'], d['decisions'], d['inventory'], [l['name'] for g in d['gate'] for l in g['legs']])" prints [] [] [] ['fmt', 'clippy-all-features', 'clippy', 'tests', 'doc-all-features', 'doc', 'design'].
- From the repository root: rg -c '^### Checklist notes$' docs/design/lys-core/DESIGN.md prints 1, and rg -c '^\*\(C38–C43 amended|^\*\(C58–C59 superseded' docs/design/lys-core/DESIGN.md prints 2.
- From the repository root: git show 7b536253:docs/design/lys-core/DESIGN.md | diff - docs/design/lys-core/DESIGN.md | grep -c '^<' prints 104, and the same pipeline with grep -c '^>' prints 117.
- The R3 dev record names each of the eight differences listed in the spec, one entry each, with its reason, and names no other difference.

**Files:**
- modify: docs/design/lys-core/design.json
- modify: docs/design/lys-core/DESIGN.md

### R4: Render the brief's own markdown and pass the design gate

WHEN python3 scripts/design/render-cluster.py docs/design/lys-core runs after R1 to R3, THE SYSTEM SHALL write docs/design/lys-core/briefs/LYSCORE-001.md from docs/design/lys-core/briefs/LYSCORE-001.json, and the committed DESIGN.md, CHECKLIST.md, USER-STORIES.md and briefs/LYSCORE-001.md SHALL each be byte-identical to that render. WHEN sh scripts/design/gate.sh runs on the card's final tree, THE SYSTEM SHALL exit 0 with lys-core among the measured clusters. The build SHALL NOT change any file outside docs/design/lys-core, SHALL NOT hand-edit any rendered markdown, and SHALL NOT change the authored fields of briefs/LYSCORE-001.json.

**Acceptance:**
- From the repository root: sh scripts/design/gate.sh exits 0 and prints no line containing 'differs'.
- From the repository root: python3 scripts/design/render-cluster.py docs/design/lys-core prints 'Done. 4 file(s) rendered.', and git status --porcelain docs/design/lys-core prints nothing after it.
- From the repository root: python3 scripts/design/check-coverage.py docs/design/lys-core exits 0 and reports 65 checklist items, 23 user stories and 1 brief.
- From the repository root, on the build branch: git diff --no-ext-diff --name-only <base>..HEAD -- . ':!docs/design/lys-core' prints nothing, where <base> is the commit the build started from.

**Files:**
- create: docs/design/lys-core/briefs/LYSCORE-001.md

## Boundaries

- No change to code: no file under crates/ and no file under scripts/ changes, including scripts/design/render-cluster.py, scripts/design/validate.py, scripts/design/check-coverage.py and scripts/design/schemas/.
- No new checklist item, user story, principle, ADR, goal, non-goal or constraint: the checklist holds C1 to C65 and the stories S1 to S23 and nothing else; the CN ids name the nine constraints main already has.
- No carried text is reworded, merged, dropped, renumbered, reordered or corrected, even where it is stale; reconciling the design with the tree as built is a later unit.
- No done flag changes: C1 to C60 and C65 stay true and C61 to C64 stay false, exactly as main ticks them.
- The eight checklist section names stay exactly as main has them; no checklist note is placed in a section name.
- The directory, home, secrets, identity, lys-anchor and lys-log-store clusters, docs/design/decisions.json, docs/design/project.json and docs/design/WIRE-FORMATS.md are not touched.
- No note slot is added to the renderer and the schemas; that is a change to the shared design tooling and a later unit.
- The later lys-core card's brief, items and stories are not added here; they come after the carried ones once this brief lands.
- Claiming C1 to C65 and S1 to S23 records that this brief carries them into JSON; it does not claim to have built them.

## Verification

- From the repository root: sh scripts/design/gate.sh exits 0.
- From the repository root: python3 scripts/design/validate.py docs/design/lys-core and python3 scripts/design/check-coverage.py docs/design/lys-core both exit 0.
- C and S text comparison, keyed on main's bytes at 7b536253 and counting every id either side holds; from the repository root run the following, which prints 'compared 88 differences 0' as its last line:
python3 - <<'EOF'
import re, subprocess
base = '7b536253'
pats = {'CHECKLIST.md': r'^- \[[ x]\] \*\*(C\d+)\*\* — (.*)$', 'USER-STORIES.md': r'^\*\*(S\d+)\.\*\* (.*)$'}
compared = differences = 0
for name, pat in pats.items():
    old = re.findall(pat, subprocess.run(['git', 'show', base + ':docs/design/lys-core/' + name], capture_output=True, text=True, check=True).stdout, re.M)
    new = re.findall(pat, open('docs/design/lys-core/' + name).read(), re.M)
    old_d, new_d = dict(old), dict(new)
    if len(old_d) != len(old) or len(new_d) != len(new):
        differences += 1
        print('duplicate id in', name)
    for i in sorted(set(old_d) | set(new_d), key=lambda s: int(s[1:])):
        compared += 1
        if old_d.get(i) != new_d.get(i):
            differences += 1
            print('differs:', i)
print('compared', compared, 'differences', differences)
EOF
- Field comparison of design.json against main's DESIGN.md and CHECKLIST.md at 7b536253; from the repository root run the following, which prints 'carried fields 7 differences []':
python3 - <<'EOF'
import json, subprocess
def main_lines(name):
    return subprocess.run(['git', 'show', '7b536253:docs/design/lys-core/' + name], capture_output=True, text=True, check=True).stdout.split('\n')
D, C = main_lines('DESIGN.md'), main_lines('CHECKLIST.md')
d = json.load(open('docs/design/lys-core/design.json'))
notes = '\n'.join(C[2:7]) + '\n\n' + C[59] + '\n\n' + C[88]
checks = {
    'title': d['title'] == D[3][len('title: "'):-1],
    'intention': d['intention'] == '\n'.join(D[10:13]),
    'problem': d['problem'] == '\n'.join(D[16:24]),
    'solution': d['solution'] == '\n'.join(D[27:133]) + '\n\n' + D[242] + '\n\n### Checklist notes\n\n' + notes,
    'goals': d['goals'] == [l.split('. ', 1)[1] for l in D[136:142]],
    'non_goals': d['non_goals'] == [{'text': l[2:], 'reason': ''} for l in D[145:152]],
    'constraints': d['constraints'] == [{'id': 'CN%d' % i, 'text': l[2:]} for i, l in enumerate(D[246:255], 1)],
}
print('carried fields', len(checks), 'differences', [k for k, v in checks.items() if not v])
EOF
- Structure tree comparison, the entry names and every word of the tree's notes against the first 75 structure rows; from the repository root run the following, which prints '75 75 True True':
python3 - <<'EOF'
import json, re, subprocess
t = subprocess.run(['git', 'show', '7b536253:docs/design/lys-core/DESIGN.md'], capture_output=True, text=True, check=True).stdout
fence = t.split('## Structure\n\n```\n')[1].split('\n```\n')[0]
names, words = [], []
for line in fence.split('\n'):
    m = re.match(r'^(crates/\S+)|^[│ ]*[├└]── (\S+)', line)
    if m:
        names.append(m.group(1) or m.group(2))
        words += line.split('— ', 1)[1].split() if '— ' in line else []
    else:
        words += line.replace('│', ' ').split()
rows = json.load(open('docs/design/lys-core/design.json'))['structure'][:75]
print(len(names), len(rows), all(r['path'].endswith(n) for r, n in zip(rows, names)), ' '.join(words) == ' '.join(w for r in rows for w in r['note'].split()))
EOF
- From the repository root: git diff --no-ext-diff --name-only <base>..HEAD -- crates scripts prints nothing, where <base> is the commit the build started from.
- From the repository root: python3 scripts/design/render-cluster.py docs/design/lys-core run a second time changes no file (git status --porcelain docs/design/lys-core prints nothing after it).

