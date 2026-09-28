# directory — what was asked, what it means, and what was written

## The words, as they were typed

Conformance row 6.4 says issuance is entered in a transparency log, and anyone verifies the certificate and its inclusion offline with standard tools. Today lys ca issue writes a certificate and enters it nowhere, and the log that lys-anchor keeps, with its lys/log-inclusion-proof/v1 artifact and scripts/verify_inclusion.py, holds no certificate. No brief joins the two.

This card enters every certificate lys ca issue produces into the lys-anchor log at issuance, as one leaf whose bytes are the certificate's DER encoding and nothing else, so no new wire format is made. The issue command then writes, beside the certificate, the leaf file and the inclusion-proof artifact the anchor already produces. It reports the log, the leaf index and the checkpoint root. If the log refuses the entry, the command refuses by name, and it writes no certificate, so no certificate exists outside the log. A person with the issuer's certificate, the issued certificate, the leaf file and the artifact verifies all of it offline with standard tools and nothing from lys. The openssl verify command checks the certificate against the issuer, and scripts/verify_inclusion.py checks the inclusion against the checkpoint root. The proof document records both commands and their output, using hashes, counts and paths only.

Acceptance: issuing with a test key and a test log writes the certificate, the leaf and the artifact; the leaf's bytes equal the certificate's DER; openssl verify accepts the certificate against the test issuer; verify_inclusion.py exits 0 on the artifact and the leaf; a leaf changed by one byte makes verify_inclusion.py exit 2; a log that refuses the entry makes issue refuse by name and write no certificate; and the whole check runs with no network and no lys binary, as a stranger would run it. This brief names row 6.4 as the row it passes. Withdrawal in the log is row 6.3's, and it is carried by DIRECTORY-012. The typed claims are row 6.2's, and card hhAN8h77 carries them. This card does neither. Production keys and the production log stay Tom's acts under row 6.6, so tests and the proof use test keys and a test log only.

Keep to the method (scripts/design/validate.py, check-coverage.py, render-cluster.py, run by scripts/design/gate.sh). The brief id, roadmap row and any decision take the next free id past main and every open brief, draft and hand branch, checked with git ls-remote immediately before writing. If a sentence here is open or contradicted by the repository as it stands, the survey quotes it whole as a question for the lead. Filed by Archie, lead for the identity line, on 27 September 2026, against lys main 7b536253.

## What the survey found, and its angles

Conformance row 6.4 asks that certificate issuance is entered in a transparency log and that a stranger can check both the certificate and its inclusion offline. The words ask that every certificate `lys ca issue` produces goes into the lys-anchor log as one raw leaf holding its DER, before any certificate file is written. The command then writes the certificate, the leaf file and the existing `lys/log-inclusion-proof/v1` artifact, and reports the log, the leaf index and the checkpoint root. If the log refuses the entry, the command refuses and writes nothing. A proof document records `openssl verify` and `scripts/verify_inclusion.py` passing, and failing on a changed leaf, using test keys and a test log only.

### What the tree holds

- `crates/lys/src/commands/ca.rs` — `issue` (lines 131-243, 341 lines in the file) builds the certificate and then writes PEM straight to `--out` at `write_file(out, …)`. Nothing is appended anywhere. The append has to go in before that write, and the report fields (Emitter) gain the log origin, leaf index and root.
- `crates/lys/src/cli.rs` — The `Issue` variant (line 236) holds the issue flags. It gains the log directory, the log key or admission policy, and the leaf and artifact output paths. Its doc text is quoted in `ca issue` help (line 5).
- `crates/lys/Cargo.toml` — Its dependencies are lys-core and lys-log-store only. The crate is published (0.2.0), so a dependency on the publish=false lys-anchor, which is a path dependency with no version, would make `lys` unpublishable.
- `crates/lys-anchor/Cargo.toml` — Has `publish = false`, with a comment saying the crate stays unpublishable until the operator's ratify-then-publish sequence completes. `unstable-anchor` is a forwarding feature, so default builds keep the receipt path compiled out.
- `crates/lys-anchor/src/anchor/append.rs` — `Anchor::append` is the ungated write path. It consults the AdmissionPolicy before appending, stores the statement verbatim, and collapses every refusal to `AnchorError::NotAdmitted`, which carries nothing. That fixes what 'refuses by name' can name.
- `crates/lys-anchor/src/anchor/artifact.rs` — `Anchor::inclusion_artifact` is ungated and builds `lys/log-inclusion-proof/v1` with a freshly signed checkpoint, so it needs the anchor's signing key. Its docs warn that an artifact taken after another append describes a later tree, so the caller must check `tree_size` against the append outcome.
- `crates/lys-anchor/src/anchor/submit.rs` — `submit` is gated behind unstable-anchor and returns the draft COSE receipt. Issuance should use `append` plus `inclusion_artifact`, not this, or the default build loses the feature and a draft format gets produced.
- `crates/lys-anchor/src/admission/certificate.rs` — The RecognisedCertificate policy already admits a submission that presents a certificate from a given issuer key. It is a natural policy for an issuance log and a ready source of a named refusal in tests.
- `crates/lys-anchor-cli/src/cli.rs` — The anchor binary has init, status, checkpoint, prove and submit. `submit` is cfg(unstable-anchor), and there is no ungated append subcommand. `--admit` is required and has no default (DP23). `prove` needs `--key`.
- `crates/lys-core/src/ca/authority.rs` — `issuer_certificate` (lines 219-235) builds the self-signed CA certificate (CN = issuer public key in hex, CA:TRUE) in memory only and never returns or writes it. `leaf_params` sets ExplicitNoCa. So nothing in the tree yields the issuer certificate `openssl verify` needs.
- `crates/lys/src/commands/log/prove.rs` — `lys log prove inclusion` already builds the same `lys/log-inclusion-proof/v1` artifact over a lys-log-store log inside the published `lys` crate. This is the alternative route for row 6.4's 'on lys-log-store'.
- `scripts/verify_inclusion.py` — The stranger's inclusion checker: stdlib only, exit 0, 1 or 2, with an optional expected-root third argument. It does not verify the checkpoint's Ed25519 signature, and says so.
- `crates/lys-anchor/tests/stranger_verification.rs` — An existing gate that runs verify_inclusion.py under python3 (LYS_REQUIRE_PYTHON). It is the pattern a Rust-side acceptance test for issuance leaves would follow.
- `crates/lys/tests/cli_tests.rs, crates/lys/tests/json_output_tests.rs, crates/lys/tests/certified_attestation_tests.rs` — The existing `ca issue` invocations that break if a log becomes mandatory for every issuance.
- `docs/design/identity/CONFORMANCE.md` — Rows 6.1 to 6.6 (lines 69-74). Row 6.4's brief column says 'same, on lys-log-store'. Row 6.6 keeps production keys and receipts as Tom's acts.
- `docs/design/directory/design.json / DESIGN.md` — The cluster to continue. Its non-goals currently exclude 'Road step 2 onward: capability certificates, … and anchoring in production'. 'Which anchor the first agents pin to' is OPEN for Tom there. Its structure table gains this card's paths.
- `docs/design/roadmap.json, docs/design/decisions.json` — The roadmap has no row for certificates or issuance transparency (RM-001 to RM-010 and RM-016). The new row and any new ADR go here, re-rendered by the method scripts.
- `scripts/design/gate.sh (validate.py, check-coverage.py, render-cluster.py, render-brief.py)` — The method's gate for the brief, roadmap row and cluster documents.

### What was already decided

- CONFORMANCE 6.4 — Issuance is entered in a transparency log; anyone verifies the certificate and its inclusion offline with standard tools. Kind: test. Brief: same, on lys-log-store. Owner: Archie.
- CONFORMANCE 6.2 / 6.3 / 6.6 — Claims as they held at issuance (6.2), and withdrawal in the log that is never edited (6.3), are separate rows. Production keys and receipts stay Tom's acts (6.6).
- lys-anchor DP3 — The certificate log holds the whole certificate. A hashed-record hatch exists only for sensitive claims, and its use is visible in the log (docs/design/lys-anchor/DECISIONS.md:73).
- lys-anchor STRAWMAN §3 — The leaf is the issued certificate's DER verbatim on the raw-leaf path, SHA-256(0x00 ‖ DER), so no new format. At issuance the CA hands the agent its inclusion proof, as in the SCT analogy.
- lys-anchor BUILD-PLAN §2.5 — The anchor CLI must be its own binary, not subcommands on the published `lys`, because publishing `lys` with an anchor dependency would freeze the draft formats.
- lys-anchor DP17 / DP19 / DP23 — Publication waits (DP17). Standalone operation is a hard requirement (DP19). Admission is a policy object and no default ships (DP23).
- lys-anchor DP26 — Revocation is an append and the live set is a fold. This is row 6.3's ground, not this card's.
- ADR-008 — An agent's file shows its lys certificate, with who signed it, when, and the signed receipts. An inclusion proof at issuance is the record that screen will later show.
- ADR-004 — Every project stands alone. The anchor log and the verification run offline with no service of another project.
- directory cluster non-goal — 'Road step 2 onward: capability certificates, … anchoring in production' is a non-goal of the directory design as it stands. 'Which anchor the first agents pin to' is OPEN for Tom.
- CLAUDE.md wire formats / publication — lys-core, lys and lys-log-store 0.2.0 are published. lys-anchor and lys-anchor-cli are publish=false. Frozen formats are never mutated.

### What was measured

- Lines in crates/lys/src/commands/ca.rs: 341 (limit 500)
- Lines in crates/lys/src/cli.rs: approx. 510 (the Issue variant at line 236)
- Log or anchor calls in `lys ca issue` today: 0
- Places the issuer certificate is written or exported by any lys command: 0 (built in memory at crates/lys-core/src/ca/authority.rs:219-235 and dropped)
- `lys` dependencies on lys-anchor: 0. lys-anchor and lys-anchor-cli are both publish = false; lys is published at 0.2.0
- Ungated append subcommands in the lys-anchor binary: 0 (only `submit`, behind unstable-anchor)
- Admission policies lys-anchor ships: 3 (accept-all, max-size, recognised-certificate)
- Distinct refusal values from Anchor::append: 1 (AnchorError::NotAdmitted, carries nothing)
- `ca issue` invocations in crates/lys/tests: 9 across 3 files (cli_tests.rs 4, json_output_tests.rs 3, certified_attestation_tests.rs 2)
- `ca issue` mentions in README.md: 1
- #[test] in crates/lys-anchor/tests/stranger_verification.rs: 8
- scripts/verify_inclusion.py exit codes: 0 verified, 1 usage/I/O, 2 verification failed; the checkpoint signature is not checked
- openssl on this Mac: /opt/homebrew/bin/openssl OpenSSL 3.6.4 (first on PATH); /usr/bin/openssl LibreSSL 3.3.6
- rcgen version: 0.13.2
- Workspace crates: 6 (lys, lys-anchor, lys-anchor-cli, lys-core, lys-home, lys-log-store)
- Directory briefs on main: 7 (DIRECTORY-001 to 006 and 008)
- Highest DIRECTORY id seen in remote branch names: DIRECTORY-013 (hand/DIRECTORY-013); hand/DIRECTORY-011 also exists; DIRECTORY-012 is named by the words
- Roadmap rows on main: 11 (RM-001 to RM-010, RM-016)
- Decisions on main: 18 (ADR-001 to ADR-018)
- Remote heads by prefix at survey time: brief 45, draft 95, hand 31, card 27, card-gate 32
- Checklist items in docs/design/directory/checklist.json: 30 id fields

### What it means for the other projects

- aion — The card runs through the chain (brief_card, sign-off, card_build_v3, src_pr, src_land), and the workflow inputs name the lys repository, main 7b53625, the card and this brief. No aion code changes.
- cambium — The card sits on its board and moves through the chain. No Cambium code or data changes, and rows 06 and 07 stay a non-goal.
- method — The brief, roadmap row and any decision are validated by the method scripts copied into scripts/design at method commit 3c3bac7. Nothing in the method changes.

### The decisions it stands on

- ADR-008 (honour) — The certificate an agent's file will show gains a public inclusion record at issuance. Nothing on the agent file changes in this card.
- ADR-004 (honour) — Issuance into a local test anchor and the stranger check both run standalone and offline, with no service from another project.
- ADR-003 (honour) — The issued certificate still hangs from the issuer the person controls. Logging adds a record and changes no grant.
-  (new) — Record where issuance-with-entry lives (`lys` depending on lys-anchor and giving up publishability, lys-log-store under `lys log`, or the lys-anchor binary), and that the entry precedes any certificate write. This settles BUILD-PLAN §2.5's rule for the issue path, and a project-ledger decision is what makes the rule binding.

### What it requires

- Issuing with a test CA key into a test log writes three files: the certificate (PEM), the leaf file, and a `lys/log-inclusion-proof/v1` artifact.
- The leaf file's bytes equal the certificate's DER byte for byte (`openssl x509 -outform DER` compared with cmp), and the log holds exactly those bytes at the reported index.
- The command reports the log's origin, the leaf index (at least 1, since leaf 0 is genesis) and the checkpoint root in base64, in both human and --json output.
- `openssl verify -CAfile <issuer.pem> <cert.pem>` prints OK and exits 0 against the test issuer's certificate.
- `python3 scripts/verify_inclusion.py <artifact> <leaf>` exits 0, and exits 0 again with the reported root as the third argument.
- The same leaf with any one byte changed makes verify_inclusion.py exit 2.
- Under a log whose admission policy refuses the entry, issue exits non-zero with a named refusal, the certificate path does not exist afterwards, and the log's tree size is unchanged.
- Both issuance paths (--request, and the discarded generated key) enter the log.
- The stranger run uses only openssl, python3 and the four files, with no network and no lys or lys-anchor binary on PATH, and the proof records that.
- A proof document records both verification commands, their exit codes and output, using hashes, counts and paths only, and the openssl and python versions used.
- A brief with the next free DIRECTORY id, a roadmap row with the next free RM id, and any ADR are written, re-rendered, and pass sh scripts/design/gate.sh.
- All six Rust gate legs pass in both feature shapes, including clippy pedantic and both cargo doc runs, run on Dean's laptop.

### What must not change

- No new wire format: the leaf is the DER alone, the artifact is the existing `lys/log-inclusion-proof/v1`, and no leaf wrapper, tag or record is added.
- The frozen artifact fields, scripts/verify_inclusion.py's logic and the checkpoint note format do not change.
- Draft formats are not reached from a default build: no gated `submit` or COSE receipt in the issue path, and the unstable-anchor gate stays as it is.
- lys-anchor stays publish = false, and nothing publishes a crate as part of this card.
- AnchorError::NotAdmitted stays a single value that carries nothing (no admission oracle).
- No production key, production log or production receipt is used or produced (row 6.6).
- No claim vocabulary is added (row 6.2, card hhAN8h77) and no withdrawal is added (row 6.3, DIRECTORY-012).
- `ca verify`, `ca request`, the certificate's contents and lys-core's issuance semantics do not change.
- The IDENTITY-001 historical files and the directory cluster's existing rows, goals and decisions stay as they are, except for a non-goal the lead chooses to amend.

### What we must put in place first

- The lead rules on where issuance-with-entry lives (`lys` depending on lys-anchor, lys-log-store, or the lys-anchor binary) before the brief's file walls can be written.
- The lead rules on whether this card exports the issuer's CA certificate, since `openssl verify` cannot run without it.
- The brief, RM and ADR ids are taken with git ls-remote immediately before writing (the brief id is at least DIRECTORY-014).
- OpenSSL 3.x with Ed25519 X.509 support and python3 are available on the machine that runs the stranger check.
- Heavy gates run on Dean's laptop.

### The risks

- Giving `lys` a lys-anchor dependency silently makes the published crate unpublishable, and reverses BUILD-PLAN §2.5.
- A mandatory log breaks the published lys 0.2.0 CLI surface for anyone scripting `ca issue`, including 9 test invocations and the README.
- The issuer certificate rcgen builds carries a fresh serial each time. An exported copy verifies only if openssl chains by DN and key alone; an AKI/SKI mismatch would make `openssl verify` fail. This needs measuring, not assuming.
- LibreSSL 3.3.6 at /usr/bin/openssl may not verify Ed25519 chains, so a stranger on stock macOS could see a false failure.
- verify_inclusion.py does not check the checkpoint signature. Without an independently obtained root, the inclusion check only proves the artifact agrees with itself, which falls short of row 6.4's 'anyone verifies'.
- Another append between this append and inclusion_artifact produces an artifact for a later tree size than the one reported unless tree_size is checked.
- The append succeeds and a later file write fails: the certificate is in the log with no file, and an append-only log cannot remove it.
- A whole-DER leaf publishes the capability-claims extension verbatim in a public log. DP3's salted-hash hatch does not exist yet.
- A raw DER leaf sits in the same log as other raw statements with no domain separation, so leaf kinds are told apart only by parsing.
- A Rust test that runs openssl and python passes on the developer machine but skips elsewhere unless a require flag, like LYS_REQUIRE_PYTHON, makes absence a failure.

### Still open

- The published `lys` crate cannot depend on the unpublished lys-anchor. Which route should issuance take: (a) `lys ca issue` gains a lys-anchor dependency and `lys` stops being publishable; (b) issuance enters the lys-log-store log that `lys log` already keeps, which produces the same artifact; or (c) the issue-and-enter command lives in the lys-anchor binary instead of `lys ca issue`? The sentence of the words it stands on: "This card enters every certificate lys ca issue produces into the lys-anchor log at issuance, as one leaf whose bytes are the certificate's DER encoding and nothing else, so no new wire format is made.". Why only the lead can settle it: crates/lys-anchor/Cargo.toml and crates/lys-anchor-cli/Cargo.toml are publish = false. crates/lys/Cargo.toml is published at 0.2.0 and cargo publish refuses a path-only dependency. docs/design/lys-anchor/BUILD-PLAN.md §2.5 forbids anchor surface in `lys`. CONFORMANCE row 6.4 says 'on lys-log-store'. The answer decides which command a person runs and whether `lys` can still be published.
- Nothing in the tree writes the issuer's certificate. Should this card add a command that exports the issuer's self-signed CA certificate for `openssl verify`, or does the stranger get it some other way? The sentence of the words it stands on: "A person with the issuer's certificate, the issued certificate, the leaf file and the artifact verifies all of it offline with standard tools and nothing from lys.". Why only the lead can settle it: crates/lys-core/src/ca/authority.rs:219-235 builds the issuer certificate in memory for signing and drops it. No lys command writes it, so the stranger in the acceptance has no issuer certificate to pass to openssl. Adding an export gives a person a new file and command.
- Must the person running `lys ca issue` also hold the log's signing key, since the inclusion artifact embeds a freshly signed checkpoint? The sentence of the words it stands on: "The issue command then writes, beside the certificate, the leaf file and the inclusion-proof artifact the anchor already produces.". Why only the lead can settle it: `Anchor::inclusion_artifact` (crates/lys-anchor/src/anchor/artifact.rs) signs a new checkpoint with the anchor key, and `lys-anchor prove` requires `--key`. Writing the artifact at issuance therefore means the issuer supplies both the CA key and the log key, or the log is operated elsewhere. That changes what the issuer must hold and pass.
- The directory cluster lists capability certificates and anchoring as a non-goal. Should this brief amend that non-goal in docs/design/directory, or should row 6.4 go in a cluster of its own? The sentence of the words it stands on: "This brief names row 6.4 as the row it passes.". Why only the lead can settle it: docs/design/directory/DESIGN.md lists 'Road step 2 onward: capability certificates, … anchoring in production' as a non-goal of this cluster. Landing a certificate-issuance brief there changes that cluster's stated scope.

### The units beyond the first

- Verify the checkpoint signature with standard tools in the stranger check — verify_inclusion.py deliberately leaves the Ed25519 note signature unchecked. Checking it with openssl pkeyutl against the log's verifier key closes the root-trust gap, but it is a separate instrument and a separate proof.
- The salted-hash issuance record for sensitive claims (DP3's hatch) — DP3 rules whole certificates by default and a visible hashed record as the exception. That record would be a new versioned leaf format, which this card is told not to make.
- An inclusion artifact at a stated tree size (inclusion_artifact_at) — artifact.rs names this as the fix for the append-to-proof window and leaves it unbuilt as an unrequested surface.
- Show the issuance inclusion on the agent's file — ADR-008's certificate view would display the leaf index and proof. That is screen work in the directory surface, not issuance.
- Production issuance against the production anchor — Row 6.6 keeps production keys and the production log as Tom's acts, and which anchor the first agents pin to is OPEN for Tom.

### The smallest complete shape

One directory-cluster brief (the next free DIRECTORY id, at least 014) plus a new roadmap row. The brief changes the issue path the lead chooses so that it appends the certificate's DER to a named test anchor through the ungated `Anchor::append`, under an explicit admission policy, before writing anything. It then writes the PEM certificate, the DER leaf file and the `lys/log-inclusion-proof/v1` artifact beside it, and reports origin, leaf index and root. If the lead puts it in scope, it adds the issuer-certificate export that `openssl verify` needs. Rust tests cover leaf equals DER, the refusal (no file written, tree size unchanged) and both issuance paths. A stranger script runs `openssl verify` and verify_inclusion.py (exit 0, then exit 2 on a one-byte change) with no network and no lys binary, and the proof document records its hashes, counts and paths. The unit lands whole through the chain with all gates.

## The roadmap row

- **RM-053** — Enter every issued certificate in the lys-log-store log and prove a stranger verifies it offline (CONFORMANCE row 6.4) (feature, idea)
- Summary: Every certificate `lys ca issue` produces is entered, before anything is written, in the lys-log-store log that `lys log` keeps, as one leaf whose bytes are the certificate's DER, with the issuer holding only the CA key; the log's operator makes the lys/log-inclusion-proof/v1 artifact with `lys log prove inclusion`; `lys ca issuer-cert` writes the issuer's self-signed certificate; and a recorded proof shows a stranger checking the certificate with `openssl verify` and its entry with scripts/verify_inclusion.py, with no lys binary. No new wire format.
- Asked by: tom on 2026-09-27T15:31:00+10:00
- Context: The row 6.4 card on the Lys board, filed by the identity line's lead against lys main 7b536253. The survey's four questions were answered by the lead: issuance enters the lys-log-store log (not lys-anchor's), a command exports the issuer certificate, the issuer holds only the CA key and the log's operator makes the artifact, and the directory non-goal is amended to bring capability certificates into scope. Main has since moved to fa3dd5311e98d1a751c770319e1cfe2570718c76, whose commits 101115b and fa3dd53 add `lys ca issue --log` in another shape; The lead kept those commits as main and DIRECTORY-031 is written as the change against that main, adding the issuer-only entry, the issuer-cert command and the proof document; --log-key and --artifact-out stay as an optional path for one operator holding both keys, and --issuer-out stays beside `lys ca issuer-cert`; both write the same bytes, from one issuer certificate the CLI stores beside the CA key the first time it is built, with lys-core untouched.
- Quote: Conformance row 6.4 says issuance is entered in a transparency log, and anyone verifies the certificate and its inclusion offline with standard tools. Today lys ca issue writes a certificate and enters it nowhere, and the log that lys-anchor keeps, with its lys/log-inclusion-proof/v1 artifact and scripts/verify_inclusion.py, holds no certificate. No brief joins the two.

This card enters every certificate lys ca issue produces into the lys-anchor log at issuance, as one leaf whose bytes are the certificate's DER encoding and nothing else, so no new wire format is made. The issue command then writes, beside the certificate, the leaf file and the inclusion-proof artifact the anchor already produces. It reports the log, the leaf index and the checkpoint root. If the log refuses the entry, the command refuses by name, and it writes no certificate, so no certificate exists outside the log. A person with the issuer's certificate, the issued certificate, the leaf file and the artifact verifies all of it offline with standard tools and nothing from lys. The openssl verify command checks the certificate against the issuer, and scripts/verify_inclusion.py checks the inclusion against the checkpoint root. The proof document records both commands and their output, using hashes, counts and paths only.

Acceptance: issuing with a test key and a test log writes the certificate, the leaf and the artifact; the leaf's bytes equal the certificate's DER; openssl verify accepts the certificate against the test issuer; verify_inclusion.py exits 0 on the artifact and the leaf; a leaf changed by one byte makes verify_inclusion.py exit 2; a log that refuses the entry makes issue refuse by name and write no certificate; and the whole check runs with no network and no lys binary, as a stranger would run it. This brief names row 6.4 as the row it passes. Withdrawal in the log is row 6.3's, and it is carried by DIRECTORY-012. The typed claims are row 6.2's, and card hhAN8h77 carries them. This card does neither. Production keys and the production log stay Tom's acts under row 6.6, so tests and the proof use test keys and a test log only.

Keep to the method (scripts/design/validate.py, check-coverage.py, render-cluster.py, run by scripts/design/gate.sh). The brief id, roadmap row and any decision take the next free id past main and every open brief, draft and hand branch, checked with git ls-remote immediately before writing. If a sentence here is open or contradicted by the repository as it stands, the survey quotes it whole as a question for the lead. Filed by Archie, lead for the identity line, on 27 September 2026, against lys main 7b536253.
- Cluster: directory; briefs: DIRECTORY-031
- Notes: Further units, not written: Verify the checkpoint signature with standard tools in the stranger check; The salted-hash issuance record for sensitive claims (DP3's hatch); An inclusion artifact at a stated tree size (inclusion_artifact_at); Show the issuance inclusion on the agent's file; Production issuance against the production anchor. Ids taken immediately before writing by git ls-remote and a read of main and every brief, draft and hand branch, including each branch's docs/design/directory/briefs: DIRECTORY-029 is carried by draft/directory/8122eb55 (the start-command draft) and brief/directory/8a60bd7a (the capability-claim brief), and DIRECTORY-030 by draft/directory/647a3946, which committed it on origin first, and no branch carries DIRECTORY-031 or above, so this brief is DIRECTORY-031; RM-054 is carried by draft/directory/8122eb55 and ADR-085 to ADR-091 by other open branches, while RM-053 and ADR-081 are held by no branch other than this draft (brief/directory/8122eb55, draft/directory/8122eb55 and draft/secrets/2d8c28b9 name RM-053 only as this draft's), and stay as the lead ruled.

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

Carry IDENTITY-001's open rows (02, 04, 03, 05) into design-system briefs in this cluster, DIRECTORY-002 to DIRECTORY-005, revised for the grant ruling (ADR-003), the PostgreSQL ruling (ADR-005) and the working lifecycle states (ADR-011, proposed), with the fork (ADR-009) and the product accents (ADR-010) in the project ledger and every decision still open for Tom marked open. The IDENTITY-001 files stay as they are, as the record of revision 5. Row 6.4 of docs/design/identity/CONFORMANCE.md, issuance entered in a transparency log and verified offline by anyone, is carried by DIRECTORY-031 on the lys-log-store log that `lys log` keeps (ADR-081): `lys ca issue` enters every certificate it produces as one leaf holding its DER before anything is written, the issuer holds only the CA key, the log's operator makes the inclusion artifact with `lys log prove inclusion`, and `lys ca issuer-cert` gives the stranger the issuer certificate `openssl verify` needs.

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
- ADR-005 — The identity database is PostgreSQL, possibly on a network device — PostgreSQL is used for the identity product's database. It may be set up on one of the network devices rather than on Tom's Mac.
- ADR-007 — The product starts an agent by giving its start command, never by running it — The product is not an execution engine. For the first release an agent's file gives the command that starts it on a chosen machine: the command carries the agent's identity and its handles, never a credential's value, and it is rendered from the agent's kept launch record. A started agent reports back, so the sessions screen shows what is running. A terminal inside the product, sandboxes, virtual machines and containers are later runtimes that plug in, and none is built into this product.
- ADR-008 — An agent's file shows its lys certificate — An agent's file shows its lys certificate once one is issued: what it claims, who signed it, when it was issued and when it expires, with the signed receipts of the changes made to it. An agent registered before any key or proof of possession was supplied shows its certificate as not issued, never a placeholder.
- ADR-009 — People sign in through a maintained Rauthy fork of our own — Rauthy authenticates people, and its one-provider-per-user limit is changed in a fork we maintain, ablative-io/rauthy, not contributed upstream as a prerequisite. The maintained branch is ablative, created from upstream v0.36.2 commit dd61ac3c84d6b238108dc8438b53043b5177a662; the fork's main stays an untouched upstream mirror; lys pins an exact commit of ablative as the submodule vendor/rauthy. Upgrades rebase ablative onto upstream release tags only, each in its own gated row; no cherry-picks and no reset of main.
- ADR-010 — Every product shares one design and keeps its own accent; the identity product's is orange — The identity screens follow Aion's structure, typography, spacing and interaction, and Rauthy's client themes take the same colours, with no build dependency on Cambium or Aion. Each product keeps its own accent within the estate colour family: Cambium green, Aion blue and black, Argus light blue, Haematite mustard. The identity product's accent is orange (accent #D4975A, deep #A86B2E, wash #3D2A17 in the estate colour tokens), set apart from Manifold's copper. No product is silently made Aion-blue, and purple is not used.
- ADR-011 — An identity is registered, active, suspended or retired — An identity is in one of four states: registered (exists in the directory, no grants, no credential handle, may not act), active (may act within its grants), suspended (kept whole, grants kept but not effective) and retired (permanent, history kept, never reactivated; a new identity is made instead). Register, activate, suspend, reinstate and retire are the only transitions, each one signed audit record naming the authenticated actor and their provenance, the identity, from, to, when and reason. Having a grant or a credential is a fact beside the state, not a state. A person is registered by first sign-in; an agent is registered by a signed-in person, who carries it as its responsible person for life and may cause every transition of their own agents. Source: docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:17-44 and docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:71-95.
- ADR-081 — Issuance enters the lys-log-store log `lys log` keeps before any certificate is written, and the issuer holds only the CA key — `lys ca issue` enters every certificate it produces, on both issuance paths, in the lys-log-store log that `lys log` keeps, as one leaf whose bytes are the certificate's DER and nothing else, before any certificate file is written; if the log refuses the entry no certificate is written. The issuer holds only the CA key: issuance appends the leaf, writes the certificate and the leaf file, and reports the log and the leaf index, and the log's operator makes the lys/log-inclusion-proof/v1 artifact with `lys log prove inclusion`, which holds the log's key. Rejected: giving `lys` a dependency on lys-anchor, which would stop `lys` being publishable and reverse BUILD-PLAN section 2.5; putting issue-and-enter in the lys-anchor binary instead of `lys ca issue`; and requiring the issuer to hold the log's key to write the artifact at issuance. The issuer-only entry is the required path, and a production issuer never holds the log's key. `--log-key` and `--artifact-out` stay on `lys ca issue` only as an optional path for one operator who holds both keys, as tests and proofs with test keys do; given one without the other, issuance refuses.

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
- Road step 2 onward: capability certificates, arbitrary grants and their enforcement, session launch and stop, credential handles, memory, context assembly, lanterns and anchoring in production — Revision 5 keeps them out of step 1 (docs/design/identity/briefs/IDENTITY-001.json:29-30); ADR-007 (the start command) and ADR-008 (the certificate on an agent's file) govern what the step-1 screens do not present as working.
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
| `docs/design/directory/briefs/DIRECTORY-031.json` | row 6.4: issuance entered in the lys-log-store log, the issuer certificate command and the stranger's proof | DIRECTORY-031 |
| `docs/design/directory/briefs/DIRECTORY-031.md` | rendered markdown | DIRECTORY-031 |
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
- `crates/lys/src/commands/ca_log.rs` — on main at fa3dd5311e98d1a751c770319e1cfe2570718c76: `lys ca issue --log`, which today takes --log, --log-key, --leaf-out and --artifact-out together or not at all, stages every output before the append, and reports log_dir, leaf_index, tree_size, root_hash, root_base64, leaf_path and artifact_path; a failure after the append is LoggedButUnwritten with the recovering `lys log prove inclusion` command. DIRECTORY-031 changes it (structure row)
- `crates/lys/tests/ca_log_tests.rs` — on main at fa3dd5311e98d1a751c770319e1cfe2570718c76: the `lys ca issue --log` integration tests and their modules under crates/lys/tests/ca_log/ (support.rs, outputs.rs, tamper.rs), which resolve openssl and python3 as hard requirements, never skips. DIRECTORY-031 adds two modules beside them (structure rows)
- `scripts/verify_inclusion.py` — the stranger's inclusion checker for lys/log-inclusion-proof/v1: Python standard library only, exit 0 verified, 1 usage or I/O, 2 verification failed, an optional expected-root-base64 third argument; it does not check the checkpoint's signature and says so. Read, never changed

## Constraints

- **CN1** — Documents only: nothing outside docs/design/directory/ and docs/design/decisions.json is created or modified; the IDENTITY-001 files are not changed.
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
> - C13 — Only the configured administrator mutates the directory in step 1; unauthenticated, same-email and wrong issuer-subject callers are refused without effect (ID001_ADMIN).
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
- C13 — Only the configured administrator mutates the directory in step 1; unauthenticated, same-email and wrong issuer-subject callers are refused without effect (ID001_ADMIN).

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
id: DIRECTORY-031
cluster: directory
title: Enter every certificate `lys ca issue` produces in its lys-log-store log with only the CA key, export the issuer certificate by command, and record the stranger's offline check
---

# DIRECTORY-031: Enter every certificate `lys ca issue` produces in its lys-log-store log with only the CA key, export the issuer certificate by command, and record the stranger's offline check

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

