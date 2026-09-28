---
type: brief
id: SECRETS-007
cluster: secrets
title: The broker's log command reads a window from the tail, never the whole audit log
---

# SECRETS-007: The broker's log command reads a window from the tail, never the whole audit log

> **Cluster:** secrets
> **Checklist:**
> - C414 — lys-secrets log prints a window from the tail and reads only what it prints (SECRETS-007 R1).
> - C415 — The whole audit log is read only by lys-secrets audit, by name, and never on a start (SECRETS-007 R2).
> **Stories:**
> - S166 (Operator, Keeps the accounts and revokes access) — As an operator, I want the broker's log command to show me the latest lines at once however long the log has grown, and a separate command when I want every line checked.

## Purpose

Tom's rule is that nothing reads a whole history when a checkpoint or a window will do. The broker's audit log grows fastest of all, one line per secret use. The broker already reads a window of it from the tail (AuditLog::window, crates/lys-secrets/src/audit.rs, after replay at line 364), but the lys-secrets log command reads and prints every line through replay (crates/lys-secrets/src/bin/lys-secrets/main.rs line 369).

## Task

Make the log command print a window of lines ending at the tail, or before a named line, through AuditLog::window, and remove the whole-log replay from every path except a named audit of the whole log.

## Requirements

### R1: The log command pages from the tail

Behavioural. lys-secrets log prints the last lines of the audit log, oldest first, through AuditLog::window. --most N names how many (a required count the command states in its help, with no hidden default), and --before I names the line to end before. It prints, after the lines, the index to pass as --before for the next older page, or that the first line was reached. Each line's signature is verified as today. It reads only the lines it prints.

**Acceptance:**
- Over an audit log of 10,000 lines, lys-secrets log --most 20 prints the last 20 and a counting store shows it read 20 lines.
- The command --before with the printed index prints the 20 before them, and the page that reaches line 0 says so.
- Without --most the command is refused by its argument parser, naming the flag.

**Files:**
- create: crates/lys-secrets/tests/log_window.rs
- modify: crates/lys-secrets/src/bin/lys-secrets/main.rs
- modify: crates/lys-secrets/src/bin/lys-secrets/args.rs

**Checklist:**
- C414 — lys-secrets log prints a window from the tail and reads only what it prints (SECRETS-007 R1).

**Stories:**
- S166 (Operator, Keeps the accounts and revokes access) — As an operator, I want the broker's log command to show me the latest lines at once however long the log has grown, and a separate command when I want every line checked.

#### R1 — Execution record

**Dev (recorded):**

- Status: implemented
- How: The log command is now a window read from the tail. AuditLog::window(before, most) reads only the leaves it returns, and each leaf's signature is verified as it is read. The CLI makes --most a required argument with no default, so clap refuses a missing one by name and exits 2. The footer gives the next older page's --before index, or says the first line was reached when the page starts at index 0. The read count is measured with a lines_read counter on AuditLog, taken on the same window call the binary makes. This round's fix is to the design leg, which failed again because the workflow wrote the round 2 dev blocks into SECRETS-007.json after the markdown was rendered. This round's dev blocks are written into the JSON first and the markdown is rendered from that, so sh scripts/design/gate.sh exits 0 on the tree the gate measures. The full gate was re-measured at the card's head on this machine.
- Deviation: cli.rs was modified although the brief lists only main.rs and args.rs, because the subcommand enum lives there. The counting store is an in-process lines_read counter on AuditLog, measured on the same window call the binary makes, because a counter cannot cross the process boundary into the binary. SECRETS-007.json and SECRETS-007.md are changed so the design leg passes. The round 2 test-leg failure was in lys-identity-server's requests test (a_kept_request_is_answered_again_after_the_end_it_asked_for), which reads the wall clock and sleeps. This diff does not touch that crate, and changing it here would be out of scope, so the review's call for a card of its own stands and no change is made to it in this card.
- Files changed:
  - created: `crates/lys-secrets/tests/log_window.rs` — Builds a 10,000-line log once per run and copies it for each test. Tests check that --most 20 prints lines 9980..9999, that the lines_read delta is 20, and that the footer reads older: --before 9980. The --before chain pages back until first line reached. Leaving out --most exits 2, stderr names --most, and the help says it is required. The broker open over 10,000 lines counts no call to audit_every_line. The audit command names an altered line.
  - modified: `crates/lys-secrets/src/bin/lys-secrets/args.rs` — New Page args. --most is a required NonZeroU64 whose help says there is no default, and --before is an optional u64.
  - modified: `crates/lys-secrets/src/bin/lys-secrets/cli.rs` — Log now flattens Where and Page, and the new Audit(Where) subcommand is added.
  - modified: `crates/lys-secrets/src/bin/lys-secrets/main.rs` — print_page calls AuditLog::window(before, most), prints the rows oldest first, then prints either older: --before N or first line reached. The Audit arm calls audit_every_line.
  - modified: `crates/lys-secrets/src/audit.rs` — lines_read counter, incremented in leaf(), so a test can count the lines a read touches.
  - modified: `docs/design/secrets/briefs/SECRETS-007.json` — Carries this round's dev blocks, written before the render so the committed markdown matches the brief the gate renders.
  - modified: `docs/design/secrets/briefs/SECRETS-007.md` — Re-rendered from SECRETS-007.json with scripts/design/render-cluster.py, so the design gate's render comparison passes.
- Checklist delivery:
  - [x] C414 — lys-secrets log prints a window from the tail and reads only what it prints (SECRETS-007 R1). — log --most N [--before I] reads only the printed window, verifies each line, and gives the footer. The tests in log_window.rs cover the count, the paging and the parser refusal, and the card round's log shows log_window with 5 passed.
- Story delivery:
  - [x] S166 (Operator, Keeps the accounts and revokes access) — As an operator, I want the broker's log command to show me the latest lines at once however long the log has grown, and a separate command when I want every line checked. — An operator pages the audit log from its tail without reading it whole.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] Over an audit log of 10,000 lines, lys-secrets log --most 20 prints the last 20 and a counting store shows it read 20 lines. — The test the_log_command_prints_the_last_lines_and_reads_only_those in crates/lys-secrets/tests/log_window.rs checks five things. (1) Called on the 10,000-line log, window(None, 20) raises lines_read by exactly 20; the counter is incremented in AuditLog::leaf in crates/lys-secrets/src/audit.rs. (2) The binary's --most 20 prints indexes 9980..9999, and each row's outcome names its own index. (3) The footer reads 'older: --before 9980'. (4) With line 9979 altered, --most 20 still succeeds. (5) --most 21 fails with 'AuditSignatureInvalid: audit line 9979'. print_page in main.rs calls only window(page.before, page.most). Card-round log: log_window has 5 passed.
  - [x] The command --before with the printed index prints the 20 before them, and the page that reaches line 0 says so. — The test the_printed_before_pages_back_to_the_first_line takes the --before value from the tail page's footer. That page prints 9960..9979 with footer 'older: --before 9960'. --before 20 prints 0..19, --before 7 prints 0..6, and --before 0 prints no rows; all three end with 'first line reached'. The footer comes from print_page in crates/lys-secrets/src/bin/lys-secrets/main.rs.
  - [x] Without --most the command is refused by its argument parser, naming the flag. — In crates/lys-secrets/src/bin/lys-secrets/args.rs, Page.most is a non-Option NonZeroU64 with no default_value. The test the_log_command_without_most_is_refused_by_its_parser checks that the command exits 2, that stderr contains --most, that no broker folder was created, and that `log --help` shows '--most <MOST>' and 'required'.
- Checklist verified: C414
- Stories verified: S166
- Issues:
  - cli.rs was changed although the brief does not list it. It holds the Command enum, so the change is necessary and is not scope creep.
  - The 'counting store' is an in-process lines_read counter on AuditLog, read on the same window call the binary makes. The binary run itself is checked by the altered line 9979: --most 20 passes and --most 21 fails naming it.
  - A start reads the log's last line for its anchor check, and the lines after the snapshot as its tail. That is start behaviour required by CLAUDE.md and existed before this diff; the log command itself reads only its page.

### R2: The whole-log read is an audit, by name

Behavioural. AuditLog::replay is renamed audit_every_line and its documentation says it reads the whole log. Its one command-line caller becomes lys-secrets audit, which checks every line's signature and names the first that fails. The demo binary (crates/lys-secrets/src/bin/lys-secrets-demo.rs line 178) uses window. No start path calls it, and a gate test fails if one does.

**Acceptance:**
- The command lys-secrets audit over a log with one altered line names that line and exits non-zero, and over a sound log it exits 0.
- A test that opens the broker over a log of 10,000 lines counts no call to audit_every_line.

**Files:**
- modify: crates/lys-secrets/src/audit.rs
- modify: crates/lys-secrets/src/broker/rotation.rs
- modify: crates/lys-secrets/src/bin/lys-secrets-demo.rs

**Checklist:**
- C415 — The whole audit log is read only by lys-secrets audit, by name, and never on a start (SECRETS-007 R2).

**Stories:**
- S166 (Operator, Keeps the accounts and revokes access) — As an operator, I want the broker's log command to show me the latest lines at once however long the log has grown, and a separate command when I want every line checked.

#### R2 — Execution record

**Dev (recorded):**

- Status: implemented
- How: The whole-log read now carries a name that says what it does. Its one command-line caller is lys-secrets audit, which exits non-zero and names the first line whose signature fails. No start path calls it. The gate test opens the broker over 10,000 lines and asserts every_line_audits() == 0 and at most one line read at start. It then calls audit_every_line once as a control, which shows the counter fires. The altered-line test first snapshots at every line so no start reads the flipped leaf. That way the audit command, not the start, is what catches it.
- Deviation: Test call sites in the tests/ files were renamed too, which is required by the rename though the brief does not list them. The every_line_audits and lines_read counters were added to AuditLog so the gate test counts calls rather than inferring them.
- Files changed:
  - modified: `crates/lys-secrets/src/audit.rs` — replay is renamed audit_every_line. Its docs say it reads the whole log, and an every_line_audits counter counts its calls.
  - modified: `crates/lys-secrets/src/bin/lys-secrets-demo.rs` — Step 11 reads the last 40 lines through window instead of the whole log.
  - modified: `crates/lys-secrets/src/broker/rotation.rs` — The unit-test helper last_rotation calls audit_every_line (renamed from replay). It is test-only code, not a start path.
  - modified: `crates/lys-secrets/src/bin/lys-secrets/main.rs` — The audit subcommand checks every signature and prints the line count. It fails with AuditSignatureInvalid naming the first bad line.
  - modified: `crates/lys-secrets/tests/accounts.rs` — Call sites renamed replay to audit_every_line. The same rename applies to audit_window, broker, ending, next_account, oauth, proxy, records, spawn_login, spend, store_leases and support/leases.
- Checklist delivery:
  - [x] C415 — The whole audit log is read only by lys-secrets audit, by name, and never on a start (SECRETS-007 R2). — replay is renamed audit_every_line with whole-log docs. The audit command, the demo on window, and the gate test counting zero calls at start are in place.
- Story delivery:
  - [x] S166 (Operator, Keeps the accounts and revokes access) — As an operator, I want the broker's log command to show me the latest lines at once however long the log has grown, and a separate command when I want every line checked. — Opening the broker never reads the whole audit log. Reading it whole is an explicit, named audit.

**Review (recorded):**

- Alignment: fixed
- Acceptance verdicts:
  - [x] The command lys-secrets audit over a log with one altered line names that line and exits non-zero, and over a sound log it exits 0. — The test the_audit_command_names_an_altered_line_and_passes_a_sound_log first writes a snapshot at every line, so no start reads the lines. `audit` on the sound log succeeds (exit 0) and prints 'audit sound: every one of N lines'. After line 1 is altered, `log --most 1` still succeeds, and `audit` fails with stderr 'AuditSignatureInvalid: audit line 1'. The Command::Audit arm in main.rs calls audit_every_line. Card-round log: this test is ok.
  - [x] A test that opens the broker over a log of 10,000 lines counts no call to audit_every_line. — The test opening_the_broker_over_ten_thousand_lines_audits_no_line checks that the start is Start::Resumed{size:10000, replayed:0}, that every_line_audits() is 0, and that lines_read is at most 1 (the anchor check). As a control it then calls audit_every_line once and checks every_line_audits is 1 and lines_read rose by 10,000. `grep -rn '.replay()' crates/lys-secrets` finds nothing.
- Checklist verified: C415
- Stories verified: S166
- Issues:
  - crates/lys-secrets/src/bin/lys-secrets-demo.rs added `const LAST_LINES: u64 = 40`, a bound neither the brief nor the design asks for. The demo writes 17 lines, so window(None, 40) read the whole log: the demo still read the whole history, just through window.
- Fixes:
  - Removed LAST_LINES from lys-secrets-demo.rs. Step 10 now records `from_rotation = broker.audit().len()` before rotate_store_key. Step 11 reads `window(None, len - from_rotation)`, which is only the rotation and post-rotation lines, and the step text and module doc now say so. The demo was run and prints lines 12 to 16. fmt check, clippy on lys-secrets in both feature shapes, ast-grep, file-length and the design gate all exit 0 afterwards.

## Boundaries

- SHALL NOT read an audit line the command does not print, apart from lys-secrets audit.
- SHALL NOT change what an audit line holds or how it is signed.
- SHALL NOT add a timeout, deadline, sleep, poll interval, #[allow], #[ignore], unsafe code or any bypass.
- SHALL NOT add a silent default or fallback. Every failure is a named refusal.

## Verification

- The full Lys gate and the ast-grep scan exit 0 at the card's head, measured by the card round.
