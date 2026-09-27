---
type: brief
id: HOME-008
cluster: home
title: Sign what a session was given: a lys/attestation/v2 given statement at render, checked offline with lys verify
---

# HOME-008: Sign what a session was given: a lys/attestation/v2 given statement at render, checked offline with lys verify

> **Cluster:** home
> **Depends on:** HOME-003
> **Design anchor:**
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-007 — The product starts an agent by giving its start command, never by running it — The product is not an execution engine. For the first release an agent's file gives the command that starts it on a chosen machine: the command carries the agent's identity and its handles, never a credential's value, and it is rendered from the agent's kept launch record. A started agent reports back, so the sessions screen shows what is running. A terminal inside the product, sandboxes, virtual machines and containers are later runtimes that plug in, and none is built into this product.
> - ADR-012 — A harness launch template is kept in the home by hash, and each render is recorded on the session beside its context path — A launch template per harness is a JSON object with named slots (transcript, mcp, env, secrets, instructions) plus flags, stored in the home under templates/ by its SHA-256; lys-home renders a template and a session into files and runtime variables with command mappings in text, prints the launch line and never runs it, and records each render as a sixth lys.harness_event kind, template_render, hung as a side leaf beside the context path with the written paths in a manifest block named by hash. Rejected: a transcript converter or adapter protocol per harness, a template kept outside the home (a seat document of another tool), and a render event that advances the head, which would change the session head hash between two renders of the same session.
> - ADR-013 — The context record is a lys.given custom entry of document hashes, never copies — The context record is one lys.given custom entry, appended after the render event, whose data is the harness name, the Claude Code version the load order was measured on, the kinds as two lists, resolved (claude_md_chain, user_claude_md, memory_index, appended_instructions, mcp_config, environment_names) and unlisted (claude_md_imports and claude_rules, which this entry does not list and a later entry at the first request records), the config directory as its path and its source (template or home), the documents in the measured order each as kind, path, byte length and SHA-256, and the names of the environment variables the template set. It is not a copy of each document into the block store, and not a content-bearing record, because the entry must hold no content under home P7 and CN3. It is unsigned and unencrypted now, and because it names hashes only, signing and encryption at rest can be added later without changing what is recorded.
> - ADR-047 — The given statement is a lys/attestation/v2 over the RFC 8785 bytes of lys.given's data, written only when render-launch is given a key — The given statement is lys-core's existing lys/attestation/v2 over the RFC 8785 (JSON Canonicalization Scheme) bytes of the lys.given entry's data, so its signed payload hash is the given hash render-launch reports as given_sha256. It is written only when render-launch is given a key: kept as a block named by a lys.given_statement entry hung under the lys.given entry, and as given-statement.cose and given-data.json under --out for lys verify. Rejected: serialising the record in struct field order, which a stranger cannot rebuild without the Rust type; signing the 64-hex hash string, whose attested hash would be SHA-256 of the hex and not the given hash; adding the given hash to the template_render event, which HOME-003 keeps unaltered; and changing lys verify's one failure message to name the file, which would change the published lys for every user.
> **Checklist:**
> - C45 — render-launch with no key records the lys.given entry with the same data as before, writes no statement, and its report's signing is unsigned.
> - C46 — render-launch reports given_sha256, the SHA-256 of the RFC 8785 bytes of the lys.given entry's data, and the template_render event does not carry it.
> - C47 — render-launch with a key signs those bytes as a lys/attestation/v2 statement kept as a block named by a lys.given_statement entry under the lys.given entry and as given-statement.cose and given-data.json under --out, and its report's signing is signed.
> - C48 — lys verify --attestation accepts the given statement with given-data.json, and refuses a copy with one byte altered with its one message and exit status 1, under a stated command that names the refused file.
> - C49 — A stated command compares the verified statement's signer public key with the test key's and exits 0, and exits 1 for a statement signed by a second test key.
> - C50 — A byte search of the fixture's statement, payload file, statement block and statement entry finds no fixture secret value and no fixture transcript text.
> - C51 — The statement's signed payload hash equals the report's given_sha256 and the SHA-256 recomputed from the lys.given entry in the session file for the same render.
> **Stories:**
> - S24 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want to check what a session was given with lys verify offline, so that I can trust the record without trusting the home that wrote it.
> - S25 (Tom, Owns the platform and reads what a session was given) — As the owner of the platform, I want a render's report to say whether what the session was given was signed, so that an unsigned render is never taken for a signed one.
> - S26 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want to see which key signed a given statement, so that a statement signed by any other key is not taken for the home's.

## Purpose

Row 6.5 of the identity conformance table (docs/design/identity/CONFORMANCE.md) asks that three things be signed statements checked with `lys verify --attestation`: the start of an agent, what the agent was given, and a move of its role. HOME-003 records what a session was given as a lys.given entry at render, unsigned, and lys-core already signs and verifies `lys/attestation/v2` attestations. This brief lands the first of the three, the given statement: at render, `lys-home render-launch` computes the SHA-256 of the lys.given entry's canonical custom.data, reports it beside the template, session head and manifest hashes, and, when it is given a signing key, signs over it in lys-core's existing format and keeps the statement beside the given entry and as two files `lys verify` reads offline. Row 6.5 is partial when this brief lands: the start statement and the role-move statement are further units, each written when the record it signs over lands (ADR-047).

## Task

The card's sentence 'signed at render over the hash it already records' is wrong about the tree: the lys.given entry records the hashes of the documents a session was given and no hash of itself. This brief corrects it: the signed hash is the SHA-256 of the lys.given entry's canonical custom.data, which render_launch computes at render and records in its report as given_sha256, beside the template, session_head and manifest hashes the report already carries. The report is the render record the hash is recorded in. The kept template_render event is not altered and does not gain the hash, so HOME-003's boundary that the render event is not altered holds; a later reader recomputes the given hash from the lys.given entry and compares it with the signed statement, and R6 does exactly that.

The canonical custom.data is RFC 8785 (JSON Canonicalization Scheme) applied to the lys.given entry's data value (ADR-047). It is frozen from the first signature, since a signature over it is a wire contract; R1 pins it with a literal vector. The attestation's claim is SHA-256 of the payload, so the payload `lys verify` reads is the canonical bytes themselves, never the hex hash, and the signed hash then equals given_sha256 by construction.

A render with no key is unchanged: the lys.given entry is recorded as HOME-003 records it, no statement is written, and the report says unsigned. With a key, the render also signs: one `lys/attestation/v2` COSE_Sign1 over the canonical bytes, stored in the home's block store and named by one lys.given_statement custom entry hung under the lys.given entry, off the context path as that entry is (ADR-012), plus given-statement.cose and given-data.json under --out for `lys verify --attestation given-statement.cose --payload given-data.json`. The report says signed. The key is `--key <path>`, a raw 32-byte Ed25519 seed file, the format `lys attest --key` and `lys key generate` use; in tests and in the proof it is a test key generated at the time, and no seed is committed. No production key is created, used or stored: production keys are the owner's acts (CONFORMANCE row 6.6). The attestation carries its signing time, as the existing format does, so a keyed render reads the clock once, to sign; the files the render writes are unchanged and CN9 is unaffected.

`lys verify` is not changed. Without --cert it proves only that some key signed, so the acceptance also checks the signer: after `lys verify` accepts the statement, a stated command compares the attestation's signer public key with the fixture's test public key and fails on a difference, and a statement signed by a second test key fails that line. Checking the signer against a trusted certificate comes with row 6.1's `--cert`, not here. The refusal of an altered statement is by name in this sense: the stated command names the statement file it was checking and `lys verify` exits non-zero with its one message, never which field or byte was altered; the published `lys` message is unchanged for everyone.

Out of scope, as further units not written here: the start statement, signed when a launch is recorded over the launch id, the agent's directory id and the given hash, attached to the START card 98qkhhCb and written when its launch record lands; the role-move statement, signed when a role move is recorded over the agent, the role, the version moved from and the version moved to, attached to the ROLES card Ink1H1Os and written when its version record lands (ADR-006); and the trusted-certificate check of row 6.1. No signer is built over fields no brief names yet. Row 6.5 is passed partially by this brief and stays partial until those two units land.

The home design's non-goal on signing is lifted for the given statement only, and anchoring and receipts into a lys log stay non-goals. The reason is that CONFORMANCE row 6.5, marked test, is part of the conformance test the platform's owner set as the definition of done, and the owner asked for all of it to be done; the design's non_goals entry is amended to say so with this brief. HOME-003's boundary against signing the given record is historical and stays as written, and HOME-003 is not edited.

lys-home takes lys-core as a dependency first (R2), for Ed25519Identity and sign_attestation, with default features. launch.rs has 190 code lines, so the signing and the entry live in a new record/given_statement.rs and launch.rs keeps only the argument, the ordering and the report.

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
- C46 — render-launch reports given_sha256, the SHA-256 of the RFC 8785 bytes of the lys.given entry's data, and the template_render event does not carry it.

**Stories:**
- S24 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want to check what a session was given with lys verify offline, so that I can trust the record without trusting the home that wrote it.

### R2: Sign a given record and keep the statement beside the lys.given entry

Add lys-core to lys-home's dependencies by workspace path with its default features. Add the custom type constant lys.given_statement beside the other lys custom types, and the module record/given_statement.rs declared from record/mod.rs. WHEN a given record appended as a lys.given entry is signed with an Ed25519Identity, THE SYSTEM SHALL call lys-core's sign_attestation over the record's canonical bytes (R1), store the attestation's COSE_Sign1 bytes in the home's block store, and append one custom entry of type lys.given_statement as the child of that lys.given entry, whose data is exactly {given: the lys.given entry's id, statement: the block hash of the COSE bytes}. It SHALL read lys.given_statement entries back with customs_everywhere in file order. THE SYSTEM SHALL NOT change lys-core's attestation format or API, SHALL NOT sign any bytes other than the canonical bytes, SHALL NOT sign the render event or any other entry, SHALL NOT put the canonical bytes, any byte of the key or any document content in the entry, and SHALL NOT move the session head.

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
- C47 — render-launch with a key signs those bytes as a lys/attestation/v2 statement kept as a block named by a lys.given_statement entry under the lys.given entry and as given-statement.cose and given-data.json under --out, and its report's signing is signed.

**Stories:**
- S24 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want to check what a session was given with lys verify offline, so that I can trust the record without trusting the home that wrote it.

### R3: Take an optional signing key in render-launch, report the given hash and say signed or unsigned

Add an optional --key <path> to render-launch's arguments, naming a raw 32-byte Ed25519 seed file. WHEN render-launch appends its lys.given entry, THE SYSTEM SHALL put the record's given hash (R1) in the report as given_sha256, and the report's signing member SHALL be the word signed when a key was given and the statement written, and the word unsigned otherwise. WHEN a key is given, THE SYSTEM SHALL load it with Ed25519Identity::load before any file is written; SHALL refuse by path, before any file is written, when given-statement.cose or given-data.json already exists under --out; and after the lys.given entry is appended SHALL sign it (R2), write the canonical bytes to <out>/given-data.json and the statement's COSE bytes to <out>/given-statement.cose, and report statement (the lys.given_statement entry id), statement_file and payload_file, the two files' absolute paths. WHEN no key is given, THE SYSTEM SHALL write no statement block, no lys.given_statement entry, no given-statement.cose and no given-data.json, and the report SHALL carry no statement, statement_file or payload_file member. IF the key file cannot be loaded, THEN THE SYSTEM SHALL fail with an error naming the key file's path, write no file under --out and append no entry. THE SYSTEM SHALL NOT alter the template_render event or add the given hash to it, SHALL NOT change the lys.given entry's data, SHALL NOT put any byte of the seed in a report, an error, a log line or a Debug form, SHALL NOT generate a key, SHALL NOT read a key from the environment, and SHALL NOT run the launch line.

**Acceptance:**
- A render of the fixture template with no --key exits 0; its report's signing is "unsigned", its given_sha256 is 64 lowercase hex characters, and it has no statement, statement_file or payload_file member; --out holds exactly the five launch files; the session holds no lys.given_statement entry.
- A render of the same fixture with --key naming a test key exits 0; its report's signing is "signed"; --out holds the five launch files plus given-statement.cose and given-data.json; the session holds exactly one lys.given_statement entry, whose id equals the report's statement.
- In the keyed render, the SHA-256 of given-data.json equals the report's given_sha256, and statement_file and payload_file are absolute paths ending in given-statement.cose and given-data.json.
- The lys.given entries' data values of a keyed and an unkeyed render of the same fixture are equal, and their template_render events' data have the same keys, neither holding the given_sha256 value.
- A keyed render's session head hash equals the session head hash taken before it.
- --key naming a 31-byte file exits non-zero with an error naming that file's path; --out holds no file afterwards and the session file is byte-identical to before.
- --key with <out>/given-statement.cose already present exits non-zero with an error naming that path, writes no other file under --out and appends no entry.
- A keyed render with a key file whose 32 bytes are 0x00 to 0x1F in order has a report and a stderr that contain neither the seed's 64-character lowercase hex form nor its standard base64 form, and the Debug form of the error from the 31-byte case contains neither the hex nor the base64 form of that file's bytes.

**Files:**
- modify: crates/lys-home/src/harness/claude_code/launch.rs
- modify: crates/lys-home/src/error.rs

**Checklist:**
- C45 — render-launch with no key records the lys.given entry with the same data as before, writes no statement, and its report's signing is unsigned.
- C46 — render-launch reports given_sha256, the SHA-256 of the RFC 8785 bytes of the lys.given entry's data, and the template_render event does not carry it.
- C47 — render-launch with a key signs those bytes as a lys/attestation/v2 statement kept as a block named by a lys.given_statement entry under the lys.given entry and as given-statement.cose and given-data.json under --out, and its report's signing is signed.

**Stories:**
- S25 (Tom, Owns the platform and reads what a session was given) — As the owner of the platform, I want a render's report to say whether what the session was given was signed, so that an unsigned render is never taken for a signed one.

### R4: Prove the given statement end to end with two test keys

Add an integration test that renders the fixture template with two keys generated at test time in temporary directories by Ed25519Identity::load_or_generate (key A and key B), with the template's environment holding a fixture secret value and the fixture session holding a fixture user message. THE SYSTEM SHALL NOT commit a seed file, SHALL NOT use a fixture secret or message as, or inside, a test name, and every loop over offsets or files SHALL assert how many cases it ran.

**Acceptance:**
- verify_attestation_bytes(given-statement.cose bytes, given-data.json bytes) returns Ok for the render signed with key A.
- For each byte offset of given-statement.cose, flipping the lowest bit of that one byte makes verify_attestation_bytes return Err(TrustError::InvalidSignature); the number of offsets tried equals the file's length, and that length is at least 191 and at most 199.
- verify_attestation_bytes_by_signer with key A's public key returns Ok for the statement signed with key A and Err for the statement signed with key B, and with key B's public key returns Err for the statement signed with key A.
- For one keyed render, the attestation's payload_hash in lowercase hex equals the report's given_sha256, and both equal the SHA-256 of serde_json::to_vec of the data value parsed by the test from the lys.given entry's line in the session file.
- A byte search of given-statement.cose, given-data.json, the statement block's bytes read from the home's block store, and the lys.given_statement entry's line, for the fixture secret value and for the fixture user message's text, searches 4 byte strings and finds 0 matches.

**Files:**
- create: crates/lys-home/tests/given_statement.rs

**Checklist:**
- C48 — lys verify --attestation accepts the given statement with given-data.json, and refuses a copy with one byte altered with its one message and exit status 1, under a stated command that names the refused file.
- C49 — A stated command compares the verified statement's signer public key with the test key's and exits 0, and exits 1 for a statement signed by a second test key.
- C50 — A byte search of the fixture's statement, payload file, statement block and statement entry finds no fixture secret value and no fixture transcript text.
- C51 — The statement's signed payload hash equals the report's given_sha256 and the SHA-256 recomputed from the lys.given entry in the session file for the same render.

**Stories:**
- S24 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want to check what a session was given with lys verify offline, so that I can trust the record without trusting the home that wrote it.
- S26 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want to see which key signed a given statement, so that a statement signed by any other key is not taken for the home's.

### R5: Write lys.given_statement into RECORD.md and the crate README

Add lys.given_statement to RECORD.md's lys custom entries: its data keys, that it hangs under the lys.given entry it signs, that the statement is a lys/attestation/v2 COSE_Sign1 kept as a block, that its payload is the RFC 8785 bytes of the lys.given entry's data, the two files under --out, and that a render with no key writes none of it. Add the row to the crate README's custom-type table and add --key to its render-launch description. Neither document SHALL carry a key's bytes, a secret value or any document content.

**Acceptance:**
- RECORD.md's 'The lys custom entries' section has a lys.given_statement item naming the keys given and statement, lys/attestation/v2, RFC 8785, given-statement.cose and given-data.json.
- The README's custom-type table has a lys.given_statement row, and the README names render-launch's --key argument.

**Files:**
- modify: docs/design/home/RECORD.md
- modify: crates/lys-home/README.md

**Checklist:**
- C47 — render-launch with a key signs those bytes as a lys/attestation/v2 statement kept as a block named by a lys.given_statement entry under the lys.given entry and as given-statement.cose and given-data.json under --out, and its report's signing is signed.

**Stories:**
- S24 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want to check what a session was given with lys verify offline, so that I can trust the record without trusting the home that wrote it.

### R6: Record a stranger's check of the given statement in PROOF-GIVEN.md

Add a section to PROOF-GIVEN.md that renders the fixture with two test keys generated by `lys key generate` in a scratch directory, and records each command a stranger runs with the lys and lys-home binaries built from the tree, with its exit status and output: `lys verify` on the statement; the named refusal of a copy with one byte altered; the signer comparison for both keys; and the given hash recomputed from the session file with python3. The section SHALL name CONFORMANCE row 6.5 as passed partially, the START card 98qkhhCb and the ROLES card Ink1H1Os as carrying the other two statements, and row 6.1's --cert as the trusted-certificate check to come. THE SYSTEM SHALL NOT put a seed, a secret value or any document content in the proof, and SHALL NOT use a production key.

**Acceptance:**
- The proof records `lys verify --attestation <out>/given-statement.cose --payload <out>/given-data.json` exiting 0.
- The proof records `if lys verify --attestation <copy> --payload <out>/given-data.json; then echo "accepted: <copy>"; else echo "refused: <copy>"; fi`, run on a copy of given-statement.cose with the byte at offset 100 altered, printing "refused: <copy>" on stdout, with lys verify exiting 1 and its stderr exactly "error: attestation verification failed: malformed or non-canonical artifact, payload mismatch, or invalid signature".
- The proof records `test "$(lys --json verify --attestation <statement> --payload <payload> | python3 -c 'import json,sys; print(json.load(sys.stdin)["signer_public_key"])')" = "$(lys --json key inspect --key <key A> | python3 -c 'import json,sys; print(json.load(sys.stdin)["public_key_ed25519"])')"` exiting 0 for the statement signed with key A and exiting 1 for the statement signed with key B.
- The proof records a python3 command that reads the lys.given entry's data from the session file, serialises it with json.dumps(data, sort_keys=True, separators=(',', ':'), ensure_ascii=False), encodes it as UTF-8 and prints its SHA-256, and that value equals the render report's given_sha256 and the payload_hash `lys --json verify` prints for the statement.
- The proof names row 6.5 as partial, names 98qkhhCb and Ink1H1Os, and names row 6.1's --cert.
- A search of PROOF-GIVEN.md for the fixture secret value, for key A's seed in lowercase hex and for key B's seed in lowercase hex finds 0 occurrences of each.

**Files:**
- modify: docs/design/home/PROOF-GIVEN.md

**Checklist:**
- C48 — lys verify --attestation accepts the given statement with given-data.json, and refuses a copy with one byte altered with its one message and exit status 1, under a stated command that names the refused file.
- C49 — A stated command compares the verified statement's signer public key with the test key's and exits 0, and exits 1 for a statement signed by a second test key.
- C51 — The statement's signed payload hash equals the report's given_sha256 and the SHA-256 recomputed from the lys.given entry in the session file for the same render.

**Stories:**
- S24 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want to check what a session was given with lys verify offline, so that I can trust the record without trusting the home that wrote it.
- S26 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want to see which key signed a given statement, so that a statement signed by any other key is not taken for the home's.

## Boundaries

- No start statement and no role-move statement: no signer is built over a launch record or a role version record; those are the further units on the START card 98qkhhCb and the ROLES card Ink1H1Os.
- `lys verify`, its one failure message and every other part of the lys crate are not changed; lys-core's attestation format and API are used as they are.
- The template_render event is not altered and does not gain the given hash.
- The lys.given entry's data is the same for every render, keyed or not.
- No production key is created, used or stored; no seed file is committed; test keys are generated when a test or the proof runs.
- No anchoring, no receipt and nothing appended to a lys log.
- No check of the signer against a certificate; row 6.1's --cert does that later.
- HOME-003's brief files and CONFORMANCE.md are not edited.
- No transcript content, document content, secret value, credential or key byte in a statement, a payload file, an entry, a report, a log line, an error or a proof.
- The design's structure array is the whole file list; a path outside it is not created.

## Verification

- python3 scripts/design/validate.py docs/design/home exits 0.
- python3 scripts/design/check-coverage.py docs/design/home exits 0.
- cargo fmt --all -- --check, cargo clippy --all-targets --all-features -- -D warnings, cargo clippy --all-targets -- -D warnings, cargo test --workspace --all-features, cargo doc --no-deps --all-features and cargo doc --no-deps exit 0.
- sh scripts/design/gate.sh exits 0.
- git diff --stat 7b536253 -- crates/lys crates/lys-core docs/design/home/briefs/HOME-003.json docs/design/home/briefs/HOME-003.md docs/design/identity/CONFORMANCE.md prints nothing.
- git ls-files crates/lys-home | grep -E '\.(key|seed)$' prints nothing.
- The commands the R6 section of PROOF-GIVEN.md records, run again from a fresh clone at the landed commit with newly generated test keys, give the same exit statuses it records.
