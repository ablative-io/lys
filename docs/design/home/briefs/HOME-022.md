---
type: brief
id: HOME-022
cluster: home
title: Sign what a session was given: a lys/attestation/v2 given statement at render, checked offline with lys verify
---

# HOME-022: Sign what a session was given: a lys/attestation/v2 given statement at render, checked offline with lys verify

> **Cluster:** home
> **Depends on:** HOME-003
> **Design anchor:**
> - ADR-001 — Secrets are held behind a handle the door swaps for the credential — A seat holds a short-lived handle bound to its identity. The real credential sits in the door's encrypted store and never leaves the server. The door's proxy checks SpiceDB, swaps the handle for the credential, forwards the call and writes one audit line. Built in Rust inside the door; no OpenBao unless credentials minted on demand are later needed.
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-007 — The product starts an agent by giving its start command, never by running it — The product is not an execution engine. For the first release an agent's file gives the command that starts it on a chosen machine: the command carries the agent's identity and its handles, never a credential's value, and it is rendered from the agent's kept launch record. A started agent reports back, so the sessions screen shows what is running. A terminal inside the product, sandboxes, virtual machines and containers are later runtimes that plug in, and none is built into this product.
> - ADR-012 — A compaction's loss is a lys.loss custom entry beside it, and a session's block hashes are a lys file beside the session — Each compaction entry is followed in the file by a lys.loss custom entry, a side leaf under the compaction, whose data names the summarised span's first and last entry ids, its counts of entries, messages, tool calls, tool results and blocks, their bytes, a digest of the span's block hashes and the harness's tokensBefore, deterministic so two imports agree apart from ids and timestamps. Block hashes are kept in <id>.blocks.jsonl beside the session, one {entry, part, hash} row per stored part, as the index and head are kept. Rejected: a new field on a Pi message or a custom entry per message for block references, which adds to Pi's grammar or doubles every import's entries; re-hashing parts or following only harness-event record hashes, which cannot find the stored blocks; and placing the loss entry on the chain, which would re-parent the record after the compaction and break the importer's parent equality.
> - ADR-013 — Claude Code's compaction is read and rendered in the shape the measured version writes — The importer reads a compact_boundary record and its isCompactSummary record as one Pi compaction entry: the summary is the isCompactSummary message's text, the first kept entry is the entry of preservedSegment.headUuid (the compaction itself when there is no preservedSegment, so it keeps nothing), tokensBefore is compactMetadata.preTokens, and Pi's details field names both source records by uuid; a named first kept entry not on record is refused by uuid. The summary record path stays for the file that carries one. The render writes a compact_boundary record, then the isCompactSummary record, then the kept records, with the parent chain advancing through all three, measured on the installed Claude Code version. Rejected: attaching the loss entry only to the summary record path, which almost no file uses, and keeping R4's summary line, which a resumed session would not read.
> - ADR-110 — The given statement is a lys/attestation/v2 over the RFC 8785 bytes of lys.given's data, its hash kept in the render manifest, written only when render-launch is given a key — The given statement is lys-core's existing lys/attestation/v2 over the RFC 8785 (JSON Canonicalization Scheme) bytes of the lys.given entry's data. The SHA-256 of those bytes, the given hash, is recorded in every render's manifest block as given_sha256 and printed in the render-launch report; the template_render event keeps the shape ADR-012 decided and names the manifest by hash. The statement is written only when render-launch is given a key: kept as a block named by a lys.given_statement entry hung under the lys.given entry, and as given-statement.cose and given-data.json under --out for lys verify. Rejected: serialising the record in struct field order, which a stranger cannot rebuild without the Rust type; signing the 64-hex hash string, whose attested hash would be SHA-256 of the hex and not the given hash; adding the given hash to the template_render event's detail, which changes the shape ADR-012 decided; recording it only in the report, where a stranger reading the session cannot find it; keeping the COSE bytes inline as hex in the entry rather than as a block; and adding the file's path to lys verify's one failure message, which would change the published lys for every user.
> **Checklist:**
> - C181 — render-launch with no key records the lys.given entry with the same data as before, writes no statement, and its report's signing is unsigned.
> - C182 — render-launch records given_sha256, the SHA-256 of the RFC 8785 bytes of the lys.given entry's data, in the render manifest block and prints it in its report, and the template_render event keeps its shape without it.
> - C183 — render-launch with a key signs those bytes as a lys/attestation/v2 statement kept as a block named by a lys.given_statement entry under the lys.given entry and as given-statement.cose and given-data.json under --out, and its report's signing is signed.
> - C184 — lys verify --attestation accepts the given statement with given-data.json, and refuses a copy with one byte altered with its one message and exit status 1, under a stated command that names the refused file.
> - C185 — A stated command compares the verified statement's signer public key with the test key's and exits 0, and exits 1 for a statement signed by a second test key.
> - C186 — A byte search of the fixture's statement, payload file, statement block, statement entry and render report finds no fixture secret value and no fixture transcript text.
> - C187 — The statement's signed payload hash equals the manifest's and the report's given_sha256 and the SHA-256 recomputed from the lys.given entry in the session file for the same render.
> - C188 — lys verify --cert with a test authority's public key accepts the given statement against a test certificate issued over the signing test key, and exits 1 against one issued over a second test key.
> **Stories:**
> - S76 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want to check what a session was given with lys verify offline, so that I can trust the record without trusting the home that wrote it.
> - S77 (Tom, Owns the platform and reads what a session was given) — As the owner of the platform, I want a render's report to say whether what the session was given was signed, so that an unsigned render is never taken for a signed one.
> - S78 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want to see which key signed a given statement, so that a statement signed by any other key is not taken for the home's.

## Purpose

Row 6.5 of the identity conformance table (docs/design/identity/CONFORMANCE.md) asks that three things be signed statements checked with `lys verify --attestation`: the start of an agent, what the agent was given, and a move of its role. HOME-003 records what a session was given as a lys.given entry at render, unsigned, and lys-core already signs and verifies `lys/attestation/v2` attestations. This brief is the row it passes, row 6.5, and it lands the first of the three, the given statement: at render, `lys-home render-launch` computes the SHA-256 of the lys.given entry's canonical custom.data, records it in the render manifest block beside the template and session head hashes, reports it beside the template, session head and manifest hashes, and, when it is given a signing key, signs over it in lys-core's existing format and keeps the statement beside the given entry and as two files `lys verify` reads offline. Row 6.5 is partial when this brief lands, and stays partial until the start statement and the role-move statement land (ADR-110).

## Task

The card's sentence 'signed at render over the hash it already records' is wrong about the tree: the lys.given entry records the hashes of the documents a session was given and no hash of itself. This brief corrects it: the signed hash is the SHA-256 of the lys.given entry's canonical custom.data. render_launch computes that hash, records it in its render record, the render manifest block, as given_sha256 beside the template and session head hashes, and signs over the value the manifest carries. The render-launch report prints given_sha256 beside the template, session_head and manifest hashes it already prints. The durable template_render event keeps the shape ADR-012 decided and binds the manifest by its manifest hash as today, so tests/launch_template.rs stands unchanged. Because the manifest now carries the given hash, render_launch resolves the given record before it stores the manifest block; the given entry is still appended after the event, under it.

The canonical custom.data is RFC 8785 (JSON Canonicalization Scheme) applied to the lys.given entry's data value (ADR-110). For this data, whose keys are fixed ASCII names and whose numbers are non-negative integers, those bytes are what serde_json writes for the data value without preserve_order, and what python3's json.dumps(data, sort_keys=True, separators=(',', ':'), ensure_ascii=False) writes, encoded as UTF-8. The form is frozen from the first signature, since a signature over it is a wire contract; R1 pins it with a literal vector and R6 recomputes it with python3. The attestation's claim is SHA-256 of the payload, so the payload `lys verify` reads is the canonical bytes themselves, never the hex hash, and the signed hash then equals given_sha256 by construction.

The given statement is signed by the home's key, which in tests and the proof is a test key.

It still records the unsigned lys.given entry, as today, so no existing run changes: a render with no key records the lys.given entry as HOME-003 records it, writes no statement, and its report says unsigned. With a signing key given it also writes the signed statement: one `lys/attestation/v2` COSE_Sign1 over the canonical bytes, stored in the home's block store and named by one lys.given_statement custom entry hung under the lys.given entry, off the context path as that entry is (ADR-012), plus given-statement.cose and given-data.json under --out for `lys verify --attestation given-statement.cose --payload given-data.json`. The report says signed. The key is `--key <path>`, a raw 32-byte Ed25519 seed file, the format `lys attest --key` and `lys key generate` use; in tests it is generated at test time and in the proof by `lys key generate`, and no seed is committed. No production key is created, used or stored: production keys are the owner's acts (CONFORMANCE row 6.6). The attestation carries its signing time, as the existing format does, so a keyed render reads the clock once, to sign; the five rendered files, the seed file and the manifest block do not depend on it, and CN9 is unaffected. given-statement.cose and given-data.json are not listed in the manifest, which is stored before the statement is signed.

The statement signs what a session was given, not which session or which render it was given to: two renders whose lys.given data is equal give equal payloads, so a statement verifies against any lys.given entry with equal data. Its tie to one session is the lys.given_statement entry's place under its lys.given entry, which is not signed. The adversarial review this repository requires for cryptographic changes takes that as its first question.

`lys verify` is not changed. Without --cert it proves only that some key signed, so the acceptance also checks the signer: after `lys verify` accepts the statement, a stated command compares the attestation's signer public key with the fixture's test public key and fails on a difference, and a statement signed by a second test key fails that line. `lys verify --cert` with `--issuer-public-key` exists today; what is missing is a certificate over the home's key, which row 6.1 supplies, and until it does the key-compare line stands in for production. Because `lys ca request`, `lys ca issue --request` and `lys verify --cert` exist, the proof also issues a test certificate over the test key and runs `lys verify --cert` against it, expecting acceptance, and against a certificate over a second test key, expecting refusal. The refusal of an altered statement is by name in this sense: the acceptance command names the statement file it checked, running `lys verify --attestation` on that one file and, on a non-zero exit, printing that file's path beside the exit status, and `lys verify` exits non-zero with its one message, word for word, never which field or byte was altered. `lys verify` is not changed: no path is added to its message, and no user of the published `lys` sees a change.

Out of scope, as further units not written here: the start statement, signed when a launch is recorded over the launch id, the agent's directory id and the given hash, attached to the START card 98qkhhCb and written when its launch record lands; the role-move statement, signed when a role move is recorded over the agent, the role, the version moved from and the version moved to, attached to the ROLES card Ink1H1Os and written when its version record lands (ADR-006); and the check of a home's statements against the certificate row 6.1 issues over the home's key. No signer is built over fields no brief names yet. Row 6.5 is passed partially by this brief and stays partial until the start and role-move units land.

The home design's non-goal on signing is lifted for the given statement only, and anchoring and receipts into a lys log stay non-goals. The reason is that CONFORMANCE row 6.5, marked test, is part of the conformance test Tom set as the definition of done, and Tom's word of 27 September at 12:49, relayed by Waffles, is to get it all done; the design's non_goals entry is amended to say so with this brief. HOME-003's boundary against signing the given record is historical and stays as written, and HOME-003 is not edited. CONFORMANCE.md is not edited either: this brief names row 6.5 as the row it passes.

lys-home takes lys-core as a dependency first (R2), for Ed25519Identity and sign_attestation, with default features. launch.rs has 190 code lines, so the signing and the entry live in a new record/given_statement.rs, and launch.rs keeps only the argument, the ordering and the report.

## Requirements

### R1: Give a lys.given record its canonical bytes and its given hash

Add to GivenRecord its canonical bytes and its given hash. WHEN the canonical bytes of a given record are asked for, THE SYSTEM SHALL return the RFC 8785 serialisation of the record's custom.data value: object members ordered by key, no whitespace between tokens, integers in decimal with no sign, fraction or exponent, strings escaping only the quotation mark, the reverse solidus and U+0000 to U+001F (\b, \t, \n, \f and \r in their two-character forms, every other control character as \u00 and two lowercase hex digits), and every other character as its UTF-8 bytes. The given hash SHALL be the lowercase hex SHA-256 of exactly those bytes. THE SYSTEM SHALL NOT hash the entry's line, id, parentId or timestamp, SHALL NOT serialise in struct field order, SHALL NOT append a newline to the canonical bytes, and SHALL NOT change the bytes a lys.given entry is written with.

**Acceptance:**
- The record with config_dir {path "/c", source template}, one document {kind claude_md_chain, path "/w/é<TAB>x.md" where <TAB> is U+0009, length 3, sha256 of 64 '0' characters}, environment ["A"], harness "claude-code", harness_version "2.1.283" and the measured kinds gives canonical bytes equal, byte for byte, to this 447-byte literal written out in the test: {"config_dir":{"path":"/c","source":"template"},"documents":[{"kind":"claude_md_chain","length":3,"path":"/w/é\tx.md","sha256":"0000000000000000000000000000000000000000000000000000000000000000"}],"environment":["A"],"harness":"claude-code","harness_version":"2.1.283","kinds":{"resolved":["claude_md_chain","user_claude_md","memory_index","appended_instructions","mcp_config","environment_names"],"unlisted":["claude_md_imports","claude_rules"]}}
- The given hash of that record is 052eda2d4a3284437ed550cc26e30d4faa5332f9d9444210d3c5b08e12bcc1c8.
- A record appended as a lys.given entry and read back with from_entry gives canonical bytes equal to the appended record's canonical bytes.
- The tests in record/given_tests.rs and tests/given_record.rs that passed before the change pass unchanged.

**Files:**
- modify: crates/lys-home/src/record/given.rs
- modify: crates/lys-home/src/record/given_tests.rs

**Checklist:**
- C182 — render-launch records given_sha256, the SHA-256 of the RFC 8785 bytes of the lys.given entry's data, in the render manifest block and prints it in its report, and the template_render event keeps its shape without it.

**Stories:**
- S76 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want to check what a session was given with lys verify offline, so that I can trust the record without trusting the home that wrote it.

### R2: Sign a given record and keep the statement beside the lys.given entry

Add lys-core to lys-home's dependencies by workspace path with its default features. Add the custom type constant lys.given_statement beside the other lys custom types, and the module record/given_statement.rs declared from record/mod.rs. WHEN a given record appended as a lys.given entry is signed with an Ed25519Identity, THE SYSTEM SHALL call lys-core's sign_attestation over the record's canonical bytes (R1), store the attestation's COSE_Sign1 bytes in the home's block store, and append one custom entry of type lys.given_statement as the child of that lys.given entry, whose data is exactly {given: the lys.given entry's id, statement: the block hash of the COSE bytes}. It SHALL read lys.given_statement entries back with customs_everywhere in file order. THE SYSTEM SHALL NOT change lys-core's attestation format or API, SHALL NOT sign any bytes other than the canonical bytes, SHALL NOT sign the render event or any other entry, SHALL NOT put the canonical bytes, any byte of the signing seed or any document content in the entry, and SHALL NOT move the session head.

**Acceptance:**
- crates/lys-home/Cargo.toml's [dependencies] has the line `lys-core.workspace = true` and names no lys-core feature.
- Signing a record appended to a session, with a key generated in a temporary directory by Ed25519Identity::load_or_generate, appends one lys.given_statement entry whose parentId equals the lys.given entry's id and whose data has exactly the keys given and statement, with given equal to that id.
- The block named by statement parses with Attestation::from_cose_bytes, its payload_hash in lowercase hex equals the record's given hash, and verify_attestation_bytes(block bytes, canonical bytes) returns Ok.
- The session's head and head hash are equal before and after the statement is appended, and the lys.given entry's line in the session file is byte-identical before and after.
- Reading back a session holding two signed records gives two lys.given_statement entries in file order, each naming its own lys.given entry.

**Files:**
- create: crates/lys-home/src/record/given_statement.rs
- create: crates/lys-home/src/record/given_statement_tests.rs
- modify: Cargo.lock
- modify: crates/lys-home/Cargo.toml
- modify: crates/lys-home/src/record/entries.rs
- modify: crates/lys-home/src/record/mod.rs

**Checklist:**
- C183 — render-launch with a key signs those bytes as a lys/attestation/v2 statement kept as a block named by a lys.given_statement entry under the lys.given entry and as given-statement.cose and given-data.json under --out, and its report's signing is signed.

**Stories:**
- S76 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want to check what a session was given with lys verify offline, so that I can trust the record without trusting the home that wrote it.

### R3: Carry the given hash in the render manifest block

RenderManifest gains the member given_sha256, the given hash (R1) as 64 lowercase hex characters, declared after session_head, so the block serialises its members in the order template, session_head, given_sha256, head, uuid, files. It is a plain string, present in every manifest, keyed render or not. The template_render event is unchanged: its kind, source_uuid, record (the manifest block's hash) and detail keys {template, session_head, files} stay as ADR-012 decided, and the event does not carry the given hash. RECORD.md's template_render item names the manifest block as `{template, session_head, given_sha256, head, uuid, files}`, and the events test that reads RECORD.md checks that string and the six member names. The manifest SHALL NOT carry the canonical bytes, a statement, or any byte of the signing seed.

**Acceptance:**
- A RenderManifest whose given_sha256 is 64 'a' characters, stored with store and read back from the block store, equals the original, and its serialised object's keys in order are template, session_head, given_sha256, head, uuid, files.
- template_render(template, session_head, manifest, 5) gives data whose detail has exactly the keys files, session_head and template, and whose record is the manifest hash.
- RECORD.md's `- `template_render`` item contains `{template, session_head, given_sha256, head, uuid, files}`, and record_md_names_the_sixth_kind_and_the_manifest_fields asserts that string and counts 6 member names found.
- tests/launch_template.rs is unchanged by this brief and passes.

**Files:**
- modify: crates/lys-home/src/harness/claude_code/events.rs
- modify: crates/lys-home/src/harness/claude_code/events_tests.rs
- modify: docs/design/home/RECORD.md

**Checklist:**
- C182 — render-launch records given_sha256, the SHA-256 of the RFC 8785 bytes of the lys.given entry's data, in the render manifest block and prints it in its report, and the template_render event keeps its shape without it.

**Stories:**
- S76 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want to check what a session was given with lys verify offline, so that I can trust the record without trusting the home that wrote it.

### R4: Take an optional signing key in render-launch, record and report the given hash, and say signed or unsigned

Add an optional --key <path> to render-launch's arguments, naming a raw 32-byte Ed25519 seed file. WHEN render-launch renders, THE SYSTEM SHALL resolve the given record before it stores the manifest block, set the manifest's given_sha256 to that record's given hash (R1, R3), put the same value in the report as given_sha256, and set the report's signing member to the word signed when a key was given and the statement written, and to the word unsigned otherwise. WHEN a key is given, THE SYSTEM SHALL load it with Ed25519Identity::load before any file is written; SHALL refuse by path, before any file is written, when given-statement.cose or given-data.json already exists under --out; and after the lys.given entry is appended SHALL sign it (R2), write the canonical bytes to <out>/given-data.json and the statement's COSE bytes to <out>/given-statement.cose, and report statement (the lys.given_statement entry id), statement_file and payload_file, the two files' absolute paths. WHEN no key is given, THE SYSTEM SHALL write no statement block, no lys.given_statement entry, no given-statement.cose and no given-data.json, and the report SHALL carry no statement, statement_file or payload_file member. IF the key file cannot be loaded, THEN THE SYSTEM SHALL fail with an error naming the key file's path, write no file under --out and append no entry. THE SYSTEM SHALL NOT alter the template_render event or add the given hash to it, SHALL NOT change the lys.given entry's data, SHALL NOT list given-statement.cose or given-data.json in the manifest, SHALL NOT put any byte of the seed in a report, an error, a log line or a Debug form, SHALL NOT generate a key, SHALL NOT read a key from the environment, and SHALL NOT run the launch line.

**Acceptance:**
- A render of the fixture template with no --key exits 0; its report's signing is "unsigned", its given_sha256 is 64 lowercase hex characters, and it has no statement member, no statement_file member and no payload_file member; --out holds exactly the five launch files; the session holds no lys.given_statement entry.
- A render of the same fixture with --key naming a test key exits 0; its report's signing is "signed"; --out holds the five launch files plus given-statement.cose and given-data.json; the session holds exactly one lys.given_statement entry, whose id equals the report's statement.
- For the keyed render and for the unkeyed render, the manifest block named by the report's manifest has given_sha256 equal to the report's given_sha256, and its files list has 5 members, no member ending in given-statement.cose and no member ending in given-data.json.
- In the keyed render, the SHA-256 of given-data.json equals the report's given_sha256, and statement_file and payload_file are absolute paths ending in given-statement.cose and given-data.json.
- The lys.given entries' data values of a keyed and an unkeyed render of the same fixture are equal, and their template_render events' detail has exactly the keys files, session_head and template, neither event's data holding the given_sha256 value.
- A keyed render's session head hash equals the session head hash taken before it.
- --key naming a 31-byte file exits 1 with an error naming that file's path; --out holds no file afterwards and the session file is byte-identical to before.
- --key with <out>/given-statement.cose already present exits 1 with an error naming that path, writes no other file under --out and appends no entry.
- A keyed render with a key file whose 32 bytes are 0x00 to 0x1F in order has a report and a stderr that contain neither the seed's 64-character lowercase hex form nor its standard base64 form, and the Debug form of the error from the 31-byte case contains neither the hex nor the base64 form of that file's bytes.
- tests/launch_template.rs and tests/given_record.rs are unchanged by this brief and pass.

**Files:**
- modify: crates/lys-home/src/harness/claude_code/launch.rs
- modify: crates/lys-home/src/error.rs

**Checklist:**
- C181 — render-launch with no key records the lys.given entry with the same data as before, writes no statement, and its report's signing is unsigned.
- C182 — render-launch records given_sha256, the SHA-256 of the RFC 8785 bytes of the lys.given entry's data, in the render manifest block and prints it in its report, and the template_render event keeps its shape without it.
- C183 — render-launch with a key signs those bytes as a lys/attestation/v2 statement kept as a block named by a lys.given_statement entry under the lys.given entry and as given-statement.cose and given-data.json under --out, and its report's signing is signed.

**Stories:**
- S77 (Tom, Owns the platform and reads what a session was given) — As the owner of the platform, I want a render's report to say whether what the session was given was signed, so that an unsigned render is never taken for a signed one.

### R5: Prove the given statement end to end with two test keys

Add an integration test that renders the fixture template through the built lys-home binary with two keys generated at test time in temporary directories by Ed25519Identity::load_or_generate (key A and key B). The rendering process's environment sets LYS_FIXTURE_TOKEN, the variable the fixture template's use-only secret names, to a fixture secret value the test declares, and the fixture session holds a fixture user message. The test checks statements with lys-core's verify_attestation_bytes and verify_attestation_bytes_by_signer, the functions `lys verify` calls, since a lys-home test cannot run the lys binary. THE SYSTEM SHALL NOT commit a seed file, SHALL NOT use the fixture secret value or message as, or inside, a test name, and every loop over offsets, files or search targets SHALL assert how many cases it ran.

**Acceptance:**
- verify_attestation_bytes(given-statement.cose bytes, given-data.json bytes) returns Ok for the render signed with key A.
- For each byte offset of given-statement.cose, flipping the lowest bit of that one byte makes verify_attestation_bytes return Err(TrustError::InvalidSignature); the number of offsets tried equals the file's length, and that length is at least 191 and at most 199.
- verify_attestation_bytes_by_signer with key A's public key returns Ok for the statement signed with key A and Err for the statement signed with key B, and with key B's public key returns Err for the statement signed with key A.
- For one keyed render, the attestation's payload_hash in lowercase hex equals the report's given_sha256 and the manifest block's given_sha256, and all three equal the SHA-256 of serde_json::to_vec of the data value parsed by the test from the lys.given entry's line in the session file.
- A byte search of given-statement.cose, given-data.json, the statement block's bytes read from the home's block store, the lys.given_statement entry's line and the render report, for the fixture secret value and for the fixture user message's text, runs 10 searches over 5 byte strings and finds 0 matches.

**Files:**
- create: crates/lys-home/tests/given_statement.rs

**Checklist:**
- C184 — lys verify --attestation accepts the given statement with given-data.json, and refuses a copy with one byte altered with its one message and exit status 1, under a stated command that names the refused file.
- C185 — A stated command compares the verified statement's signer public key with the test key's and exits 0, and exits 1 for a statement signed by a second test key.
- C186 — A byte search of the fixture's statement, payload file, statement block, statement entry and render report finds no fixture secret value and no fixture transcript text.
- C187 — The statement's signed payload hash equals the manifest's and the report's given_sha256 and the SHA-256 recomputed from the lys.given entry in the session file for the same render.

**Stories:**
- S76 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want to check what a session was given with lys verify offline, so that I can trust the record without trusting the home that wrote it.
- S78 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want to see which key signed a given statement, so that a statement signed by any other key is not taken for the home's.

### R6: Record a stranger's check of the given statement in PROOF-GIVEN.md

Add a section to PROOF-GIVEN.md that renders the fixture with two test keys generated by `lys key generate` in a scratch directory, and records each command a stranger runs with the lys and lys-home binaries built from the tree, with its exit status and output: `lys verify` on the statement; the refusal, by the statement file's name, of a copy with one byte altered; the signer comparison for both keys; the certificate check with a test authority; and the given hash recomputed from the session file with python3. The section SHALL name CONFORMANCE row 6.5 as passed partially, the START card 98qkhhCb and the ROLES card Ink1H1Os as carrying the other two statements, and row 6.1 as supplying the certificate over the home's key that `lys verify --cert` checks in production, with the key-compare line standing in until then. THE SYSTEM SHALL NOT put a seed, a secret value or any document content in the proof, and SHALL NOT use a production key.

**Acceptance:**
- The proof records `lys verify --attestation <out>/given-statement.cose --payload <out>/given-data.json` exiting 0.
- The proof records `lys verify --attestation <copy> --payload <out>/given-data.json; s=$?; if [ "$s" -ne 0 ]; then echo "refused: <copy> exit $s"; fi`, run on a copy of given-statement.cose with the byte at offset 100 altered, printing exactly "refused: <copy> exit 1" on stdout, with lys verify's stderr exactly "error: attestation verification failed: malformed or non-canonical artifact, payload mismatch, or invalid signature" and naming no field and no byte offset.
- The proof records `test "$(lys --json verify --attestation <statement> --payload <payload> | python3 -c 'import json,sys; print(json.load(sys.stdin)["signer_public_key"])')" = "$(lys --json key inspect --key <key A> | python3 -c 'import json,sys; print(json.load(sys.stdin)["public_key_ed25519"])')"` exiting 0 for the statement signed with key A and exiting 1 for the statement signed with key B.
- The proof records `lys ca request --key <key A> --subject home-test --out <a.csr>` and `lys ca issue --key <test authority key> --subject home-test --request <a.csr> --validity 1h --out <a.pem>` each exiting 0, then `lys verify --attestation <statement A> --payload <payload A> --cert <a.pem> --issuer-public-key <test authority public key>` exiting 0; and the same issue for key B giving <b.pem>, with `lys verify --attestation <statement A> --payload <payload A> --cert <b.pem> --issuer-public-key <test authority public key>` exiting 1.
- The proof records the command `python3 -c 'import hashlib,json,sys; [print(hashlib.sha256(json.dumps(e["data"],sort_keys=True,separators=(",",":"),ensure_ascii=False).encode("utf-8")).hexdigest()) for e in map(json.loads,open(sys.argv[1],encoding="utf-8")) if e.get("type")=="custom" and e.get("customType")=="lys.given"]' <session file>` run on the keyed render's session file, printing one line of 64 lowercase hex characters equal to the render report's given_sha256 and to the payload_hash `lys --json verify` prints for the statement; and the same command run on a file whose one line is a custom entry of customType lys.given with data equal to R1's record, printing exactly 052eda2d4a3284437ed550cc26e30d4faa5332f9d9444210d3c5b08e12bcc1c8.
- The proof names row 6.5 as partial, names 98qkhhCb and Ink1H1Os, and names row 6.1 as supplying the certificate over the home's key.
- A search of PROOF-GIVEN.md for the fixture secret value, for key A's seed in lowercase hex and for key B's seed in lowercase hex finds 0 occurrences of each.

**Files:**
- modify: docs/design/home/PROOF-GIVEN.md

**Checklist:**
- C184 — lys verify --attestation accepts the given statement with given-data.json, and refuses a copy with one byte altered with its one message and exit status 1, under a stated command that names the refused file.
- C185 — A stated command compares the verified statement's signer public key with the test key's and exits 0, and exits 1 for a statement signed by a second test key.
- C187 — The statement's signed payload hash equals the manifest's and the report's given_sha256 and the SHA-256 recomputed from the lys.given entry in the session file for the same render.
- C188 — lys verify --cert with a test authority's public key accepts the given statement against a test certificate issued over the signing test key, and exits 1 against one issued over a second test key.

**Stories:**
- S76 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want to check what a session was given with lys verify offline, so that I can trust the record without trusting the home that wrote it.
- S78 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want to see which key signed a given statement, so that a statement signed by any other key is not taken for the home's.

### R7: Write lys.given_statement into RECORD.md and the crate README

Add lys.given_statement to RECORD.md's lys custom entries: its data keys, that it hangs under the lys.given entry it signs, that the statement is a lys/attestation/v2 COSE_Sign1 kept as a block, that its payload is the RFC 8785 bytes of the lys.given entry's data, the two files under --out, and that a render with no key writes none of it. Replace RECORD.md's opening claim that nothing is signed with one naming the given statement as the one signed thing, and say in the lys.given item that the entry itself stays unsigned and a keyed render signs it by a lys.given_statement child. Add the row to the crate README's custom-type table and add --key to its render-launch description. THE SYSTEM SHALL NOT put a key's bytes, a secret value or any document content in either document.

**Acceptance:**
- RECORD.md's 'The lys custom entries' section has a lys.given_statement item naming the keys given and statement, lys/attestation/v2, RFC 8785, given-statement.cose and given-data.json.
- RECORD.md's first paragraph no longer contains the words 'nothing is signed' and names lys.given_statement.
- The README's custom-type table has a lys.given_statement row, and the README names render-launch's --key argument.

**Files:**
- modify: crates/lys-home/README.md
- modify: docs/design/home/RECORD.md

**Checklist:**
- C183 — render-launch with a key signs those bytes as a lys/attestation/v2 statement kept as a block named by a lys.given_statement entry under the lys.given entry and as given-statement.cose and given-data.json under --out, and its report's signing is signed.

**Stories:**
- S76 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want to check what a session was given with lys verify offline, so that I can trust the record without trusting the home that wrote it.

## Boundaries

- No start statement and no role-move statement: no signer is built over a launch record or a role version record; those are the further units on the START card 98qkhhCb and the ROLES card Ink1H1Os.
- `lys verify`, its one failure message and every other part of the lys crate are not changed; no path is added to VerificationFailed; lys-core's attestation format and API are used as they are.
- The template_render event is not altered and does not gain the given hash; the manifest block it names carries it.
- The lys.given entry's data is the same for every render, keyed or not.
- No production key is created, used or stored; no seed file is committed; test keys are generated when a test or the proof runs.
- No certificate over the home's key is issued for production; the test certificates in the proof are issued by a test authority key and are not committed.
- No anchoring, no receipt and nothing appended to a lys log.
- HOME-003's brief files and CONFORMANCE.md are not edited.
- No transcript content, document content, secret value, credential or key byte in a statement, a payload file, an entry, a report, a log line, an error or a proof.
- The design's structure array is the whole file list; a path outside it is not created.

## Verification

- `python3 scripts/design/validate.py docs/design/home` exits 0.
- `python3 scripts/design/check-coverage.py docs/design/home` exits 0.
- `sh scripts/design/gate.sh` exits 0.
- `cargo fmt --all` exits 0, and `git diff --quiet` exits 0 after it.
- `cargo clippy --all-targets --all-features -- -D warnings` exits 0.
- `cargo clippy --all-targets -- -D warnings` exits 0.
- `cargo test --workspace --all-features` exits 0.
- `cargo doc --no-deps --all-features` exits 0.
- `cargo doc --no-deps` exits 0.
- `git diff --exit-code origin/main...HEAD -- crates/lys crates/lys-core crates/lys-home/tests/launch_template.rs crates/lys-home/tests/given_record.rs docs/design/home/briefs/HOME-003.json docs/design/home/briefs/HOME-003.md docs/design/identity/CONFORMANCE.md` exits 0.
- `git ls-files crates/lys-home | grep -E '\.(key|seed|pem|cose)$'` exits 1.
- `cargo build -p lys -p lys-home` exits 0, and the lys and lys-home binaries it builds are the ones the commands below run.
- `lys key generate --out <scratch>/a.key`, `lys key generate --out <scratch>/b.key` and `lys key generate --out <scratch>/ca.key` each exit 0.
- `mkdir -p <scratch>/w <scratch>/homeU/sessions <scratch>/homeA/sessions <scratch>/homeB/sessions <scratch>/outU <scratch>/outA <scratch>/outB` exits 0.
- `cp crates/lys-home/tests/fixtures/launch/session.jsonl <scratch>/homeU/sessions/fixture.jsonl`, `cp crates/lys-home/tests/fixtures/launch/session.jsonl <scratch>/homeA/sessions/fixture.jsonl` and `cp crates/lys-home/tests/fixtures/launch/session.jsonl <scratch>/homeB/sessions/fixture.jsonl` each exit 0, putting the fixture session into each scratch home as the session fixture.
- `lys-home render-launch --home <scratch>/homeU --session fixture --template crates/lys-home/tests/fixtures/launch/template.json --uuid 00000000-0000-4000-8000-000000000003 --cwd <scratch>/w --model claude-fixture --version 2.1.283 --out <scratch>/outU` exits 0, its report's signing is "unsigned", and <scratch>/outU holds no given-statement.cose and no given-data.json.
- `lys-home render-launch --home <scratch>/homeA --session fixture --template crates/lys-home/tests/fixtures/launch/template.json --uuid 00000000-0000-4000-8000-000000000003 --cwd <scratch>/w --model claude-fixture --version 2.1.283 --key <scratch>/a.key --out <scratch>/outA` exits 0 and its report's signing is "signed"; <outA> below is <scratch>/outA.
- `lys-home render-launch --home <scratch>/homeB --session fixture --template crates/lys-home/tests/fixtures/launch/template.json --uuid 00000000-0000-4000-8000-000000000003 --cwd <scratch>/w --model claude-fixture --version 2.1.283 --key <scratch>/b.key --out <scratch>/outB` exits 0 and its report's signing is "signed"; <outB> below is <scratch>/outB.
- `lys verify --attestation <outA>/given-statement.cose --payload <outA>/given-data.json` exits 0.
- `lys verify --attestation <copy> --payload <outA>/given-data.json; s=$?; if [ "$s" -ne 0 ]; then echo "refused: <copy> exit $s"; fi`, on a copy of <outA>/given-statement.cose with the byte at offset 100 altered, exits 0 and prints exactly "refused: <copy> exit 1" on stdout, and lys verify's stderr is exactly "error: attestation verification failed: malformed or non-canonical artifact, payload mismatch, or invalid signature".
- `test "$(lys --json verify --attestation <outA>/given-statement.cose --payload <outA>/given-data.json | python3 -c 'import json,sys; print(json.load(sys.stdin)["signer_public_key"])')" = "$(lys --json key inspect --key <scratch>/a.key | python3 -c 'import json,sys; print(json.load(sys.stdin)["public_key_ed25519"])')"` exits 0.
- `test "$(lys --json verify --attestation <outB>/given-statement.cose --payload <outB>/given-data.json | python3 -c 'import json,sys; print(json.load(sys.stdin)["signer_public_key"])')" = "$(lys --json key inspect --key <scratch>/a.key | python3 -c 'import json,sys; print(json.load(sys.stdin)["public_key_ed25519"])')"` exits 1.
- `lys --json key inspect --key <scratch>/ca.key` exits 0, and the public_key_ed25519 it prints is <ca public key>.
- `lys ca request --key <scratch>/a.key --subject home-test --out <scratch>/a.csr` exits 0.
- `lys ca issue --key <scratch>/ca.key --subject home-test --request <scratch>/a.csr --validity 1h --out <scratch>/a.pem` exits 0.
- `lys ca request --key <scratch>/b.key --subject home-test --out <scratch>/b.csr` exits 0.
- `lys ca issue --key <scratch>/ca.key --subject home-test --request <scratch>/b.csr --validity 1h --out <scratch>/b.pem` exits 0.
- `lys verify --attestation <outA>/given-statement.cose --payload <outA>/given-data.json --cert <scratch>/a.pem --issuer-public-key <ca public key>` exits 0.
- `lys verify --attestation <outA>/given-statement.cose --payload <outA>/given-data.json --cert <scratch>/b.pem --issuer-public-key <ca public key>` exits 1.
- `python3 -c 'import hashlib,json,sys; [print(hashlib.sha256(json.dumps(e["data"],sort_keys=True,separators=(",",":"),ensure_ascii=False).encode("utf-8")).hexdigest()) for e in map(json.loads,open(sys.argv[1],encoding="utf-8")) if e.get("type")=="custom" and e.get("customType")=="lys.given"]' <scratch>/homeA/sessions/fixture.jsonl`, run on the session file of the render into <outA>, exits 0 and prints one line equal to that render report's given_sha256.
