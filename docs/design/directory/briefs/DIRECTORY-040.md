---
type: brief
id: DIRECTORY-040
cluster: directory
title: Enter every certificate `lys ca issue` produces in its lys-log-store log with only the CA key, export the issuer certificate by command, and record the stranger's offline check
---

# DIRECTORY-040: Enter every certificate `lys ca issue` produces in its lys-log-store log with only the CA key, export the issuer certificate by command, and record the stranger's offline check

> **Cluster:** directory
> **Blocked by:** Sign-off of this brief on its card before card_build_v3 runs; a build started without it is a defect of the chain, not a row of this brief. Check: python3 scripts/design/validate.py docs/design/directory and python3 scripts/design/check-coverage.py docs/design/directory both exit 0, and the card records the sign-off before the build is dispatched., The build starts from a main that holds commit fa3dd5311e98d1a751c770319e1cfe2570718c76, which carries crates/lys/src/commands/ca_log.rs, crates/lys/tests/ca_log_tests.rs and crates/lys/tests/ca_log/; every modify path below is stated against it. Check: git merge-base --is-ancestor fa3dd5311e98d1a751c770319e1cfe2570718c76 origin/main exits 0.
> **Design anchor:**
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> - ADR-008 — An agent's file shows its lys certificate — An agent's file shows its lys certificate once one is issued: what it claims, who signed it, when it was issued and when it expires, with the signed receipts of the changes made to it. An agent registered before any key or proof of possession was supplied shows its certificate as not issued, never a placeholder.
> - ADR-081 — Issuance enters the lys-log-store log `lys log` keeps before any certificate is written, and the issuer holds only the CA key — `lys ca issue` enters every certificate it produces, on both issuance paths, in the lys-log-store log that `lys log` keeps, as one leaf whose bytes are the certificate's DER and nothing else, before any certificate file is written; if the log refuses the entry no certificate is written. The issuer holds only the CA key: issuance appends the leaf, writes the certificate and the leaf file, and reports the log and the leaf index, and the log's operator makes the lys/log-inclusion-proof/v1 artifact with `lys log prove inclusion`, which holds the log's key. Rejected: giving `lys` a dependency on lys-anchor, which would stop `lys` being publishable and reverse BUILD-PLAN section 2.5; putting issue-and-enter in the lys-anchor binary instead of `lys ca issue`; and requiring the issuer to hold the log's key to write the artifact at issuance. The issuer-only entry is the required path, and a production issuer never holds the log's key. `--log-key` and `--artifact-out` stay on `lys ca issue` only as an optional path for one operator who holds both keys, as tests and proofs with test keys do; given one without the other, issuance refuses.
> **Checklist:**
> - C209 — `lys ca issue` refuses to run without `--log` and `--leaf-out`, and on both issuance paths enters the certificate in the lys-log-store log as one leaf whose bytes are the certificate's DER and nothing else before any file is written.
> - C210 — With `--log` and `--leaf-out` and no log key, `lys ca issue` writes the certificate and the leaf and reports the log, the leaf index, the tree size and the root in base64, signing nothing and writing no artifact; the log's operator makes the lys/log-inclusion-proof/v1 artifact with `lys log prove inclusion`.
> - C211 — When the log refuses the entry, `lys ca issue` exits 1 with the log's refusal by name, writes no certificate and no leaf, and the log's leaves are unchanged.
> - C212 — `lys ca issuer-cert --key <path> --out <path>` writes the issuer's self-signed CA certificate as one PEM block carrying no key material, and it and `lys ca issue --issuer-out` write the same bytes on every call, from one issuer certificate stored beside the CA key when it is first built.
> - C213 — Holding only the issuer certificate, the issued certificate, the leaf and the operator's artifact, with no lys binary on PATH, `openssl verify -CAfile` accepts the certificate, `scripts/verify_inclusion.py` exits 0 under the reported root, and a leaf changed by one byte makes it exit 2.
> - C214 — docs/design/directory/PROOF-ISSUANCE.md records the stranger's check run with test keys and a test log: each command, its exit code and its output, and otherwise hashes, counts and paths only.
> - C215 — The directory design's non-goal for road step 2 no longer excludes capability certificates, cites CONFORMANCE rows 6.1 to 6.4 as the reason, and still excludes anchoring in production.
> **Stories:**
> - S84 (Stranger, Checks an issued certificate and its log entry offline, holding nothing from lys) — As a stranger holding the issuer's certificate, an issued certificate, its leaf and its inclusion artifact, I want to check the certificate and its entry in the log offline with openssl and a standard-library script, so that I rely on nothing from lys and on no one's word that the certificate was logged.
> - S85 (Issuer, Issues an agent's certificate under the CA key it holds) — As the issuer, I want every certificate I issue entered in the log before it is written, holding only my CA key, so that no certificate of mine exists outside the log.
> - S86 (Log operator, Keeps the log and signs its checkpoints with the log's key) — As the log's operator, I want to be the only holder of the log's key and to make the inclusion artifact for an issued certificate's leaf myself, so that issuing a certificate never needs the log's key.

## Purpose

Pass CONFORMANCE row 6.4 (docs/design/identity/CONFORMANCE.md): issuance is entered in a transparency log and anyone verifies the certificate and its inclusion offline with standard tools. Per ADR-081 the log is the lys-log-store log `lys log` keeps, so `lys` stays publishable and no new wire format is made: the leaf is the certificate's DER and nothing else and the artifact is the existing lys/log-inclusion-proof/v1. Main at fa3dd5311e98d1a751c770319e1cfe2570718c76 already enters a certificate when all four of --log, --log-key, --leaf-out and --artifact-out are given, and exports the issuer certificate through --issuer-out. This brief closes what that leaves: the log becomes mandatory for every certificate on both issuance paths, the issuer needs only the CA key while the log's operator makes the artifact with `lys log prove inclusion` (a production issuer never holds the log key; `--log-key` and `--artifact-out` stay only for one operator holding both keys), `lys ca issuer-cert` writes the issuer certificate, byte-identical to what `--issuer-out` writes because both write one stored copy, the directory design brings capability certificates into scope, and a recorded proof shows a stranger with four files and no lys binary checking both the certificate and its entry.

## Task

Work from the main named in blocked_by. Read crates/lys/src/commands/ca_log.rs, crates/lys/src/commands/ca.rs, crates/lys/src/cli.rs (the Issue variant), crates/lys/src/main.rs (the Ca dispatch), crates/lys/tests/ca_log_tests.rs and crates/lys/tests/ca_log/support.rs first; the entry mechanics (staging before the append, the size check, LoggedButUnwritten) exist and are reused, not rewritten.

In scope, in order: R1 amends one non-goal of docs/design/directory/design.json and re-renders DESIGN.md (documents only). R2 makes `--log` and `--leaf-out` required on `lys ca issue` and adds the issuer-only entry: with no `--log-key` and no `--artifact-out`, the certificate is entered and written and nothing is signed with any log key. R3 moves every existing `ca issue` caller (tests and the README example) onto a test log. R4 adds `lys ca issuer-cert` and keeps one stored issuer certificate beside the CA key, built once through the lys-core `CertificateAuthority::issuer_certificate_der` main already has, so `lys ca issuer-cert` and `lys ca issue --issuer-out` write the same bytes on every call; this is the one new command this brief gives a person, and the issuer certificate it writes is the one new file. Each requirement's acceptance stands on the ones before it and on none after it. R5 adds the end-to-end tests, including the stranger's check with no lys on PATH. R6 records the stranger's check by hand in docs/design/directory/PROOF-ISSUANCE.md.

Out of scope: withdrawal in the log (row 6.3, carried by DIRECTORY-012), typed claims (row 6.2, carried by card hhAN8h77), checking the checkpoint's signature with standard tools, a stated-size inclusion artifact, the agent file's view of the inclusion, and production issuance; production keys and the production log stay out of every test and the proof (row 6.6). `--log-key` and `--artifact-out` stay on `lys ca issue` as an optional path for the case where one operator holds both keys, as the tests and the proof do with test keys; the required path is the issuer-only one, and a production issuer never holds the log key: the log's operator, who holds it, makes the artifact. `--issuer-out` stays beside `lys ca issuer-cert`, which exists so a person who did not run the issuance can still get the issuer certificate. Nothing working on main is removed or renamed, and R2 only changes which flags are required.

Every path is relative to the repository root. Build, test and gate runs happen on the build venue; only small single-crate checks run elsewhere.

## Requirements

### R1: Bring capability certificates into the directory cluster's scope

THE SYSTEM SHALL amend the one entry of non_goals in docs/design/directory/design.json whose text begins 'Road step 2 onward:' so that its text no longer names capability certificates and still names arbitrary grants and their enforcement, session launch and stop, credential handles, memory, context assembly, lanterns and anchoring in production, and so that its reason keeps its present sentence and gains one sentence stating that capability certificates, CONFORMANCE rows 6.1 to 6.4 of docs/design/identity/CONFORMANCE.md, are in this cluster's scope because the project owner asked for the conformance test to be done, and that anchoring in production stays out. THE SYSTEM SHALL re-render docs/design/directory/DESIGN.md with python3 scripts/design/render-cluster.py docs/design/directory. It SHALL NOT change any other non-goal, goal, principle, constraint, decision, inventory row or structure row, SHALL NOT change the number of non-goals, and SHALL NOT change docs/design/identity/CONFORMANCE.md.

**Acceptance:**
- From the repository root: python3 -c "import json; n=[g for g in json.load(open('docs/design/directory/design.json'))['non_goals'] if g['text'].startswith('Road step 2 onward:')]; print(len(n), 'capability certificates' in n[0]['text'], 'anchoring in production' in n[0]['text'], 'CONFORMANCE rows 6.1 to 6.4' in n[0]['reason'])" prints 1 False True True.
- From the repository root: python3 -c "import json; print(len(json.load(open('docs/design/directory/design.json'))['non_goals']))" prints 11, the count the build started from.
- From the repository root: rg -c 'CONFORMANCE rows 6.1 to 6.4' docs/design/directory/DESIGN.md prints 1, and sh scripts/design/gate.sh exits 0.
- From the repository root: git diff --quiet <base> -- docs/design/identity/CONFORMANCE.md exits 0, where <base> is the commit the build started from.

**Files:**
- modify: docs/design/directory/design.json
- modify: docs/design/directory/DESIGN.md

**Checklist:**
- C215 — The directory design's non-goal for road step 2 no longer excludes capability certificates, cites CONFORMANCE rows 6.1 to 6.4 as the reason, and still excludes anchoring in production.

### R2: Require the log on every issuance and enter the certificate with only the CA key

WHEN `lys ca issue` is run, THE SYSTEM SHALL require --log <dir> and --leaf-out <path> on both issuance paths, with --request and without it; a run missing either SHALL be refused by argument parsing with exit code 2 before any key file is read, with the missing flags listed under argument parsing's 'the following required arguments were not provided:'. WHEN --log and --leaf-out are given with neither --log-key nor --artifact-out, THE SYSTEM SHALL build the certificate, open the log and run the checks main runs before the append today, stage the certificate, the leaf and any --issuer-out output, append the certificate's DER as one leaf, place the staged files, and report log_dir, leaf_index, tree_size, root_hash, root_base64 and leaf_path in human and --json output with no artifact_path; in this case THE SYSTEM SHALL NOT read any log key, SHALL NOT sign any checkpoint and SHALL NOT write any inclusion-proof artifact. --log-key and --artifact-out SHALL be accepted only together: given one without the other, THE SYSTEM SHALL exit 1 with the LogFlagsIncomplete error and append nothing, and that error's text SHALL be exactly '--log-key and --artifact-out come together or not at all: missing <flag>', where <flag> is the one of --log-key and --artifact-out that was not given; the text '--log, --log-key, --leaf-out and --artifact-out come together' SHALL NOT remain in the error; given both, the behaviour main has at fa3dd5311e98d1a751c770319e1cfe2570718c76 is unchanged. IF the log refuses the entry, because it is not initialized, fails its integrity check or refuses the append, THEN THE SYSTEM SHALL exit 1 with the log's own refusal (log directory not initialized, log directory invalid, or the store's message) and SHALL NOT leave the certificate, the leaf, the --issuer-out output or any other output on disk. IF a step after the append fails, THEN THE SYSTEM SHALL exit 1 with LoggedButUnwritten naming the log, the leaf index, the tree size and the base64 root, and, when no --log-key was given, its recovery command SHALL read lys log prove inclusion --dir <dir> --key <log-key> --leaf-index <n> --out <artifact> with the literal placeholders <log-key> and <artifact>; it SHALL NOT append again or sign anything. The leaf's bytes SHALL be the certificate's DER and nothing else: no wrapper, tag, prefix or record is added.

**Acceptance:**
- With a test CA key issuer.key and no log flags: lys ca issue --key issuer.key --subject agent-x --validity 1h --out agent-x.pem exits 2, the lines of stderr after 'the following required arguments were not provided:' and before the next blank line name --log and --leaf-out, and agent-x.pem does not exist.
- lys ca issue --key issuer.key --subject agent-x --validity 1h --out agent-x.pem --log log exits 2, the lines of stderr after 'the following required arguments were not provided:' and before the next blank line name --leaf-out and not --log, and agent-x.pem does not exist.
- With a fresh test log from lys log init --dir log --origin example.com/lys/issuance and no log key anywhere in the command: lys --json ca issue --key issuer.key --subject agent-one --validity 1h --out agent-one.pem --log log --leaf-out agent-one.leaf exits 0; the JSON has leaf_index 0, tree_size 1, a root_base64 of 44 characters, leaf_path ending agent-one.leaf and no artifact_path key; the bytes of agent-one.leaf equal the DER of agent-one.pem; and the bytes of log/leaves/00000000000000000000 equal agent-one.leaf.
- On the same log, with agent.csr.pem from lys ca request --key agent.key --subject agent-two --out agent.csr.pem: lys --json ca issue --key issuer.key --subject agent-two --request agent.csr.pem --validity 1h --out agent-two.pem --log log --leaf-out agent-two.leaf exits 0 with leaf_index 1 and tree_size 2, and agent-two.leaf equals the DER of agent-two.pem.
- lys ca issue with --log log --leaf-out agent-three.leaf --log-key operator.key and no --artifact-out exits 1, stderr contains '--log-key and --artifact-out come together or not at all: missing --artifact-out', agent-three.pem does not exist, and log/leaves holds the same number of files as before the run.
- lys ca issue with --log log --leaf-out agent-three.leaf --artifact-out agent-three.inclusion.json and no --log-key exits 1, stderr contains '--log-key and --artifact-out come together or not at all: missing --log-key', neither agent-three.pem nor agent-three.inclusion.json exists, and log/leaves holds the same number of files as before the run.
- rg -c -- '--log, --log-key, --leaf-out and --artifact-out' crates/lys/src prints nothing.
- After lys key generate --out issuer4.key, with no issuer4.key.issuer.pem present, and after replacing root_hash in log/state.json with the base64 of 32 zero bytes: lys ca issue --key issuer4.key --subject agent-four --validity 1h --out agent-four.pem --log log --leaf-out agent-four.leaf --issuer-out issuer4.pem exits 1, stderr begins error: log directory invalid:, none of agent-four.pem, agent-four.leaf, issuer4.pem and issuer4.key.issuer.pem exists afterwards, and log/leaves holds the same number of files as before the run.
- A unit test in crates/lys/src/commands/ca_log_tests.rs builds the failure after the append for an issuer-only entry at leaf index 3 and asserts its message contains lys log prove inclusion --dir <the log dir> --key <log-key> --leaf-index 3 --out <artifact>.
- cargo test -p lys --all-features --bin lys passes, and its output lists the unit test of the failure after the append at leaf index 3, named in the line before, under commands::ca_log::tests (the module compiled from crates/lys/src/commands/ca_log_tests.rs) with the result ok.

**Files:**
- modify: crates/lys/src/commands/ca_log.rs
- modify: crates/lys/src/commands/ca_log_tests.rs
- modify: crates/lys/src/commands/ca.rs
- modify: crates/lys/src/commands/ca_tests.rs
- modify: crates/lys/src/commands/error.rs
- modify: crates/lys/src/cli.rs
- modify: crates/lys/src/main.rs

**Checklist:**
- C209 — `lys ca issue` refuses to run without `--log` and `--leaf-out`, and on both issuance paths enters the certificate in the lys-log-store log as one leaf whose bytes are the certificate's DER and nothing else before any file is written.
- C210 — With `--log` and `--leaf-out` and no log key, `lys ca issue` writes the certificate and the leaf and reports the log, the leaf index, the tree size and the root in base64, signing nothing and writing no artifact; the log's operator makes the lys/log-inclusion-proof/v1 artifact with `lys log prove inclusion`.
- C211 — When the log refuses the entry, `lys ca issue` exits 1 with the log's refusal by name, writes no certificate and no leaf, and the log's leaves are unchanged.

**Stories:**
- S85 (Issuer, Issues an agent's certificate under the CA key it holds) — As the issuer, I want every certificate I issue entered in the log before it is written, holding only my CA key, so that no certificate of mine exists outside the log.
- S86 (Log operator, Keeps the log and signs its checkpoints with the log's key) — As the log's operator, I want to be the only holder of the log's key and to make the inclusion artifact for an issued certificate's leaf myself, so that issuing a certificate never needs the log's key.

### R3: Move every existing `ca issue` caller onto a test log

THE SYSTEM SHALL give every `ca issue` invocation in crates/lys/tests/cli_tests.rs, crates/lys/tests/json_output_tests.rs and crates/lys/tests/certified_attestation_tests.rs a test log made with lys log init in the test's own temporary directory, and --log and --leaf-out naming it, with no --log-key, and each test SHALL keep asserting what it asserted before about the certificate. The cli_tests.rs test that asserts `ca issue` creates no extra files SHALL assert instead that exactly the certificate and the leaf file are created beside the fixtures, and that the log holds one leaf. THE SYSTEM SHALL replace crates/lys/tests/ca_log/outputs.rs's case of an existing certificate refused without a log with the same refusal under --log and --leaf-out, and change crates/lys/tests/ca_log_tests.rs's flags-together case so it asserts R2's rules: --log alone exits 2, and --log-key without --artifact-out exits 1 with nothing appended. THE SYSTEM SHALL update the `lys ca issue` example in README.md so that a lys log init line precedes it and its command names --log and --leaf-out, with the output lines it shows matching what the command prints: one line per label `lys ca issue` prints under --log and --leaf-out without --log-key, in the order it prints them, including entered in log, leaf index, tree size, root hash (sha256), root hash (base64) and leaf written, and no artifact written line. It SHALL NOT delete any test, SHALL NOT weaken any assertion about the certificate, the refusals or the no-overwrite rule, and SHALL NOT use a production key or log.

**Acceptance:**
- From the repository root, on the build branch: cargo test -p lys --all-features --test cli_tests --test json_output_tests --test certified_attestation_tests --test ca_log_tests passes.
- The four-flag tests already in crates/lys/tests/ca_log_tests.rs, crates/lys/tests/ca_log/outputs.rs and crates/lys/tests/ca_log/tamper.rs pass, changed only by the edits this requirement names, and git diff --quiet <base> -- crates/lys/tests/ca_log/tamper.rs exits 0, where <base> is the commit the build started from.
- The number of #[test] functions in each of crates/lys/tests/cli_tests.rs, json_output_tests.rs, certified_attestation_tests.rs, ca_log_tests.rs and ca_log/outputs.rs is at least the number the build started from.
- rg -n -- '--leaf-out' README.md prints at least one line inside the `lys ca issue` example, and rg -n 'lys log init' README.md prints a line above that example.
- Running the README.md example's `lys ca issue` command, with the keys, request and capabilities.json the README's earlier steps create and a test log from its lys log init line, in a temporary directory, prints lines whose labels (the text before the first ': ') are, in order, exactly the labels of the output lines the example shows; those labels include entered in log, leaf index, tree size, root hash (sha256), root hash (base64) and leaf written, and rg -n 'artifact written' README.md prints no line inside the example.

**Files:**
- modify: crates/lys/tests/cli_tests.rs
- modify: crates/lys/tests/json_output_tests.rs
- modify: crates/lys/tests/certified_attestation_tests.rs
- modify: crates/lys/tests/ca_log_tests.rs
- modify: crates/lys/tests/ca_log/outputs.rs
- modify: README.md

**Checklist:**
- C209 — `lys ca issue` refuses to run without `--log` and `--leaf-out`, and on both issuance paths enters the certificate in the lys-log-store log as one leaf whose bytes are the certificate's DER and nothing else before any file is written.

**Stories:**
- S85 (Issuer, Issues an agent's certificate under the CA key it holds) — As the issuer, I want every certificate I issue entered in the log before it is written, holding only my CA key, so that no certificate of mine exists outside the log.

### R4: Add `lys ca issuer-cert`, and keep one stored issuer certificate that it and `--issuer-out` both write

The stored issuer certificate of an issuer key is the file whose path is the issuer key file's path with .issuer.pem appended (for issuer.key, issuer.key.issuer.pem); it holds exactly one PEM CERTIFICATE block and is public. WHEN `lys ca issuer-cert --key <issuer-key> --out <path>` is run with an existing issuer key file, THE SYSTEM SHALL write at <path> the bytes of that key's stored issuer certificate, report the issuer public key as 64 lowercase hex characters under issuer_public_key and the written path under issuer_certificate_path in human and --json output, and exit 0. WHEN `lys ca issue --issuer-out <path>` is run, THE SYSTEM SHALL write at <path> the bytes of the issuer key's stored issuer certificate, staged with the command's other outputs and placed only after the append. IF no stored issuer certificate exists when either command runs, THEN THE SYSTEM SHALL build it once with CertificateAuthority::issuer_certificate_der (crates/lys-core/src/ca/authority.rs, unchanged), encode it as one PEM CERTIFICATE block, write it at the stored path complete or not at all, and then write the same bytes at <path>; `lys ca issue` without --issuer-out SHALL NOT create it. IF the log refuses the entry of a `lys ca issue --issuer-out` run, THEN THE SYSTEM SHALL NOT leave a stored issuer certificate that did not exist before the run. IF the stored file exists but does not hold exactly one PEM CERTIFICATE block whose subject public key equals the issuer key's public key, THEN THE SYSTEM SHALL exit 1 with an error naming the stored file, and SHALL NOT write <path>, append to any log or change the stored file. The output at <path> is written through the output writer `lys ca issue` already uses (crates/lys/src/commands/files.rs), so an existing file at <path> is refused by name as it is for `lys ca issue`; `lys ca issuer-cert` SHALL make that refusal, exit 1, before it builds or writes the stored issuer certificate, so a refused run leaves no stored file that was not there before. IF the key file does not exist, THEN THE SYSTEM SHALL exit 1 with the existing 'identity key file not found' error and write nothing. The help text of both commands SHALL name the stored file's path rule. THE SYSTEM SHALL NOT rebuild, overwrite or delete a stored issuer certificate once it exists, SHALL NOT write any private key material, SHALL NOT create or modify any key file, SHALL NOT open or append to any log from `lys ca issuer-cert`, and SHALL NOT change any file under crates/lys-core.

**Acceptance:**
- In a temporary directory, after lys key generate --out issuer.key: lys --json ca issuer-cert --key issuer.key --out issuer.pem exits 0, issuer.pem holds exactly one line equal to -----BEGIN CERTIFICATE-----, the JSON's issuer_public_key equals the public_key_ed25519 that lys --json key inspect --key issuer.key reports, and cmp issuer.pem issuer.key.issuer.pem exits 0.
- openssl x509 -in issuer.pem -noout -subject -nameopt RFC2253 prints subject=CN=<hex>, where <hex> is the reported issuer_public_key, and openssl x509 -in issuer.pem -noout -ext basicConstraints prints a line containing CA:TRUE.
- The 32 bytes of issuer.key occur nowhere in the DER of issuer.pem, and issuer.pem contains no line with the text PRIVATE.
- After sleep 2 following the first call: lys ca issuer-cert --key issuer.key --out issuer-again.pem exits 0 and cmp issuer.pem issuer-again.pem exits 0.
- With a test log from lys log init --dir log --origin example.com/lys/issuance, after sleep 2: lys ca issue --key issuer.key --subject agent-x --validity 1h --out agent-x.pem --log log --leaf-out agent-x.leaf --issuer-out issuer-at-issue.pem exits 0, cmp issuer.pem issuer-at-issue.pem exits 0, and openssl verify -CAfile issuer.pem agent-x.pem exits 0 with stdout agent-x.pem: OK.
- With a second key from lys key generate --out issuer2.key and no issuer2.key.issuer.pem present: lys ca issue --key issuer2.key --subject agent-y --validity 1h --out agent-y.pem --log log --leaf-out agent-y.leaf --issuer-out first.pem exits 0 and cmp first.pem issuer2.key.issuer.pem exits 0; after sleep 2, lys ca issuer-cert --key issuer2.key --out later.pem exits 0 and cmp first.pem later.pem exits 0.
- After copying issuer2.key.issuer.pem over issuer.key.issuer.pem: lys ca issuer-cert --key issuer.key --out wrong.pem exits 1, stderr names issuer.key.issuer.pem, wrong.pem does not exist, and cmp issuer2.key.issuer.pem issuer.key.issuer.pem exits 0.
- lys ca issuer-cert --key missing.key --out issuer3.pem, with no missing.key present, exits 1 with stderr containing identity key file not found, and none of issuer3.pem, missing.key and missing.key.issuer.pem exists afterwards.
- Run with --out issuer.pem already present, lys ca issuer-cert --key issuer2.key --out issuer.pem exits 1, stderr names issuer.pem, and issuer.pem's bytes are unchanged.
- After lys key generate --out issuer3.key, with no issuer3.key.issuer.pem present and a file taken.pem already present: lys ca issuer-cert --key issuer3.key --out taken.pem exits 1, stderr names taken.pem, taken.pem's bytes are unchanged, and issuer3.key.issuer.pem does not exist afterwards.
- After lys key generate --out issuer5.key, with no issuer5.key.issuer.pem present, and a second test log from lys log init --dir badlog --origin example.com/lys/issuance whose state.json root_hash was replaced by the base64 of 32 zero bytes: lys ca issue --key issuer5.key --subject agent-z --validity 1h --out agent-z.pem --log badlog --leaf-out agent-z.leaf --issuer-out issuer5.pem exits 1, and none of agent-z.pem, agent-z.leaf, issuer5.pem and issuer5.key.issuer.pem exists afterwards.
- These cases are tests in crates/lys/tests/ca_log/issuer_cert.rs, registered in crates/lys/tests/ca_log_tests.rs, and cargo test -p lys --all-features --test ca_log_tests runs them and passes.

**Files:**
- create: crates/lys/tests/ca_log/issuer_cert.rs
- modify: crates/lys/src/cli.rs
- modify: crates/lys/src/main.rs
- modify: crates/lys/src/commands/ca.rs
- modify: crates/lys/src/commands/ca_tests.rs
- modify: crates/lys/tests/ca_log_tests.rs

**Checklist:**
- C212 — `lys ca issuer-cert --key <path> --out <path>` writes the issuer's self-signed CA certificate as one PEM block carrying no key material, and it and `lys ca issue --issuer-out` write the same bytes on every call, from one issuer certificate stored beside the CA key when it is first built.

**Stories:**
- S84 (Stranger, Checks an issued certificate and its log entry offline, holding nothing from lys) — As a stranger holding the issuer's certificate, an issued certificate, its leaf and its inclusion artifact, I want to check the certificate and its entry in the log offline with openssl and a standard-library script, so that I rely on nothing from lys and on no one's word that the certificate was logged.

### R5: Test issuance with the CA key only, the operator's artifact and the stranger's offline check

THE SYSTEM SHALL add crates/lys/tests/ca_log/issuer_only.rs, registered in crates/lys/tests/ca_log_tests.rs, whose tests make, in a temporary directory, a test CA key and a test log operator key with lys key generate and a test log with lys log init --origin example.com/lys/issuance; export the issuer certificate with lys ca issuer-cert; issue with the CA key only, on both issuance paths, with --log and --leaf-out; have the operator make each certificate's artifact with lys log prove inclusion --key <operator key> --leaf-index <reported leaf_index> before the next issuance into that log, so the artifact describes the tree issue reported; and then run the stranger's check in a fresh temporary directory holding only the issuer certificate, the issued certificate, the leaf, the artifact and a copy of scripts/verify_inclusion.py, starting openssl and python3 with a cleared environment whose PATH is one directory holding only entries named openssl and python3, each under a network-denying wrapper started by absolute path: on macOS /usr/bin/sandbox-exec -p '(version 1)(allow default)(deny network*)', on Linux unshare --net --map-root-user; a platform with neither, or a wrapper that fails to start, SHALL fail the test, never skip it. An OpenSSL 3 build that cannot do Ed25519, or a missing python3, SHALL fail the test, never skip it; tool resolution goes through crates/lys/tests/ca_log/support.rs. The stranger step SHALL NOT run lys or lys-anchor, SHALL NOT read the log directory or any key file, and SHALL NOT reach the network; no test SHALL use a production key or log.

**Acceptance:**
- In the stranger's directory, openssl verify -CAfile issuer.pem agent.pem exits 0 and its stdout is agent.pem: OK.
- openssl x509 -in agent.pem -outform DER, run by the stranger, yields bytes equal to agent.leaf.
- python3 verify_inclusion.py agent.inclusion.json agent.leaf <root_base64 reported by issue> exits 0 and the first line of its stdout is INCLUSION VERIFIED.
- The artifact's tree_size equals the tree_size issue reported and its leaf_index equals the leaf_index issue reported.
- The same run on a copy of agent.leaf with its last byte XORed with 0x01 exits 2 and its stderr begins VERIFICATION FAILED.
- The certificate issued with --request passes the same three checks at leaf index 1.
- The directory named by the stranger step's PATH lists exactly openssl and python3.
- In the same test, with a TCP listener the test opens on 127.0.0.1: python3 -c "import socket, sys; socket.create_connection(('127.0.0.1', int(sys.argv[1])))" <port> exits 0 when started without the wrapper and exits non-zero when started under it, and the listener accepts exactly one connection in total; the openssl verify, the verify_inclusion.py exit 0 and the verify_inclusion.py exit 2 above are the runs made under the wrapper.
- A test in the file issues into a log whose state.json root_hash was replaced by the base64 of 32 zero bytes, and asserts exit 1, stderr beginning error: log directory invalid:, no certificate file, no leaf file, and an unchanged number of files in the log's leaves directory.
- cargo test -p lys --all-features --test ca_log_tests passes, and rg -c 'return;' crates/lys/tests/ca_log/issuer_only.rs prints nothing.

**Files:**
- create: crates/lys/tests/ca_log/issuer_only.rs
- modify: crates/lys/tests/ca_log_tests.rs
- modify: crates/lys/tests/ca_log/support.rs

**Checklist:**
- C209 — `lys ca issue` refuses to run without `--log` and `--leaf-out`, and on both issuance paths enters the certificate in the lys-log-store log as one leaf whose bytes are the certificate's DER and nothing else before any file is written.
- C210 — With `--log` and `--leaf-out` and no log key, `lys ca issue` writes the certificate and the leaf and reports the log, the leaf index, the tree size and the root in base64, signing nothing and writing no artifact; the log's operator makes the lys/log-inclusion-proof/v1 artifact with `lys log prove inclusion`.
- C211 — When the log refuses the entry, `lys ca issue` exits 1 with the log's refusal by name, writes no certificate and no leaf, and the log's leaves are unchanged.
- C213 — Holding only the issuer certificate, the issued certificate, the leaf and the operator's artifact, with no lys binary on PATH, `openssl verify -CAfile` accepts the certificate, `scripts/verify_inclusion.py` exits 0 under the reported root, and a leaf changed by one byte makes it exit 2.

**Stories:**
- S84 (Stranger, Checks an issued certificate and its log entry offline, holding nothing from lys) — As a stranger holding the issuer's certificate, an issued certificate, its leaf and its inclusion artifact, I want to check the certificate and its entry in the log offline with openssl and a standard-library script, so that I rely on nothing from lys and on no one's word that the certificate was logged.
- S85 (Issuer, Issues an agent's certificate under the CA key it holds) — As the issuer, I want every certificate I issue entered in the log before it is written, holding only my CA key, so that no certificate of mine exists outside the log.
- S86 (Log operator, Keeps the log and signs its checkpoints with the log's key) — As the log's operator, I want to be the only holder of the log's key and to make the inclusion artifact for an issued certificate's leaf myself, so that issuing a certificate never needs the log's key.

### R6: Record the stranger's check in PROOF-ISSUANCE.md

THE SYSTEM SHALL add docs/design/directory/PROOF-ISSUANCE.md recording one run, made by hand on the build venue with a test CA key, a test log operator key and a test log in a temporary directory, of: lys key generate for each key, lys log init, lys ca issuer-cert, lys ca issue with --log and --leaf-out and no log key, and lys log prove inclusion with the operator key, the four outputs named issuer.pem, agent.pem, agent.leaf and agent.inclusion.json. Each command line is recorded with a leading $, followed by its output verbatim and then one line [exit <n>] giving its exit code. Before the stranger's check the document records the probe python3 -c "import socket; socket.create_connection(('example.com', 443))" run with the network available, and then the one command that starts the shell the stranger's check runs in, under the network-denying wrapper of R5 for the venue's operating system (sandbox-exec -p '(version 1)(allow default)(deny network*)' /bin/sh or unshare --net --map-root-user /bin/sh). The stranger's check is the section headed exactly ## The stranger's check, run in that shell in a fresh directory holding only the four files and a copy of scripts/verify_inclusion.py, with no lys binary on PATH; its commands are, in order: the same probe; command -v lys, which prints nothing and exits non-zero because no lys binary is on PATH; shasum -a 256 and wc -c of each of the four files; openssl verify -CAfile issuer.pem agent.pem; openssl x509 -in agent.pem -outform DER -out agent.der; cmp agent.der agent.leaf; python3 verify_inclusion.py agent.inclusion.json agent.leaf with the root issue reported; and the same on a copy of the leaf changed by one byte. Beyond the command lines and their output, the document records only hashes, counts and paths: the leaf index and tree size, and the absolute paths of the openssl binary (an OpenSSL 3 build with Ed25519) and the python3 binary it ran. Every SHA-256 and byte count the document gives for the four files is the verbatim output of its recorded shasum or wc command. It SHALL NOT contain a PEM block, key material, a key file's bytes, a person's name, or any production key, log or receipt.

**Acceptance:**
- rg -c -- '-----BEGIN' docs/design/directory/PROOF-ISSUANCE.md prints nothing, and rg -c 'PRIVATE' docs/design/directory/PROOF-ISSUANCE.md prints nothing.
- rg -c "^## The stranger's check$" docs/design/directory/PROOF-ISSUANCE.md prints 1; in that section, which runs to the next line beginning ## or the end of the document, every line that begins with $ either is exactly $ command -v lys or begins with one of $ openssl, $ python3, $ cmp, $ shasum, $ wc.
- rg -c '^\$ openssl verify -CAfile issuer\.pem agent\.pem$' docs/design/directory/PROOF-ISSUANCE.md prints 1, and the lines after it are agent.pem: OK and [exit 0].
- rg -c '^\$ python3 .*verify_inclusion\.py ' docs/design/directory/PROOF-ISSUANCE.md prints 2, one run's output beginning INCLUSION VERIFIED and ending [exit 0], the other's beginning VERIFICATION FAILED and ending [exit 2].
- For each name N of issuer.pem, agent.pem, agent.leaf and agent.inclusion.json, inside the stranger's section: exactly one line is $ shasum -a 256 N and the line after it matches ^[0-9a-f]{64}  N$; exactly one line is $ wc -c N and the line after it matches ^ *[0-9]+ N$; and each is followed by [exit 0].
- Every 64-character lowercase hex string in the document that is followed on its line by one of the four names occurs only on the line after that name's $ shasum -a 256 line.
- rg -c '^\$ (sandbox-exec -p .*deny network.* /bin/sh|unshare --net --map-root-user /bin/sh)$' docs/design/directory/PROOF-ISSUANCE.md prints 1, and that line comes before the stranger's section heading.
- The probe line $ python3 -c "import socket; socket.create_connection(('example.com', 443))" occurs exactly twice: once before the wrapper's line, followed by [exit 0], and once as the first $ line of the stranger's section, followed by its error output and a line [exit n] with n not 0.
- rg -c '^\$ command -v lys$' docs/design/directory/PROOF-ISSUANCE.md prints 1; that line is inside the stranger's section, after the section's probe line and before its first $ shasum line, and the line after it is [exit n] with n not 0, so the command printed no path.
- sh scripts/design/gate.sh exits 0 with the document present.

**Files:**
- create: docs/design/directory/PROOF-ISSUANCE.md

**Checklist:**
- C214 — docs/design/directory/PROOF-ISSUANCE.md records the stranger's check run with test keys and a test log: each command, its exit code and its output, and otherwise hashes, counts and paths only.

**Stories:**
- S84 (Stranger, Checks an issued certificate and its log entry offline, holding nothing from lys) — As a stranger holding the issuer's certificate, an issued certificate, its leaf and its inclusion artifact, I want to check the certificate and its entry in the log offline with openssl and a standard-library script, so that I rely on nothing from lys and on no one's word that the certificate was logged.

## Boundaries

- SHALL NOT make a new wire format: the leaf is the certificate's DER alone, the artifact is the existing lys/log-inclusion-proof/v1, and no leaf wrapper, tag, prefix or record is added.
- SHALL NOT change scripts/verify_inclusion.py, the inclusion artifact's fields or the checkpoint note format.
- SHALL NOT change any file under crates/lys-core, crates/lys-log-store, crates/lys-anchor or crates/lys-anchor-cli, and SHALL NOT give `lys` a dependency on lys-anchor.
- SHALL NOT reach a draft format behind unstable-anchor from the issue path, and SHALL NOT change the unstable-anchor gate.
- SHALL NOT publish any crate.
- SHALL NOT use or produce a production key, a production log or a production receipt; tests and the proof use test keys and a test log only.
- SHALL NOT add claim vocabulary (row 6.2) and SHALL NOT add withdrawal (row 6.3).
- SHALL NOT change `lys ca verify`, `lys ca request` or the contents of an issued certificate.
- SHALL NOT remove or rename --log-key, --artifact-out or --issuer-out on `lys ca issue`; R2 changes only which of them are required.
- SHALL NOT change the IDENTITY-001 files, and SHALL NOT change any directory checklist row, story, goal or decision other than R1's one non-goal.

## Verification

- cargo fmt --all -- --check exits 0.
- cargo clippy --all-targets --all-features -- -D warnings and cargo clippy --all-targets -- -D warnings both exit 0.
- cargo test --workspace --all-features exits 0.
- cargo doc --no-deps --all-features and cargo doc --no-deps both exit 0 with no warnings.
- sh scripts/design/gate.sh exits 0.
- git diff --stat <base>..HEAD -- crates/lys-core crates/lys-log-store crates/lys-anchor crates/lys-anchor-cli scripts/verify_inclusion.py prints nothing, where <base> is the commit the build started from.
- cargo tree -p lys --edges normal | rg -c lys-anchor prints nothing.
