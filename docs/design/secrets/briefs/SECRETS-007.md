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

Tom's rule: nothing reads a whole history when a checkpoint or a window will do. The broker's audit log grows fastest of all, one line per secret use. The broker already reads a window of it from the tail (AuditLog::window, crates/lys-secrets/src/audit.rs, after replay at line 364), but the lys-secrets log command reads and prints every line through replay (crates/lys-secrets/src/bin/lys-secrets/main.rs line 369).

## Task

Make the log command print a window of lines ending at the tail, or before a named line, through AuditLog::window, and remove the whole-log replay from every path except a named audit of the whole log.

## Requirements

### R1: The log command pages from the tail

Behavioural. lys-secrets log prints the last lines of the audit log, oldest first, through AuditLog::window: --most N names how many (a required count the command states in its help, with no hidden default), and --before I names the line to end before. It prints, after the lines, the index to pass as --before for the next older page, or that the first line was reached. Each line's signature is verified as today. It reads only the lines it prints.

**Acceptance:**
- Over an audit log of 10,000 lines, lys-secrets log --most 20 prints the last 20 and a counting store shows it read 20 lines.
- --before with the printed index prints the 20 before them; the page that reaches line 0 says so.
- Without --most the command is refused by its argument parser, naming the flag.

**Files:**
- create: crates/lys-secrets/tests/log_window.rs
- modify: crates/lys-secrets/src/bin/lys-secrets/main.rs
- modify: crates/lys-secrets/src/bin/lys-secrets/args.rs

**Checklist:**
- C414 — lys-secrets log prints a window from the tail and reads only what it prints (SECRETS-007 R1).

**Stories:**
- S166 (Operator, Keeps the accounts and revokes access) — As an operator, I want the broker's log command to show me the latest lines at once however long the log has grown, and a separate command when I want every line checked.

### R2: The whole-log read is an audit, by name

Behavioural. AuditLog::replay is renamed audit_every_line and its documentation says it reads the whole log. Its one command-line caller becomes lys-secrets audit, which checks every line's signature and names the first that fails. The demo binary (crates/lys-secrets/src/bin/lys-secrets-demo.rs line 178) uses window. No start path calls it, and a gate test fails if one does.

**Acceptance:**
- lys-secrets audit over a log with one altered line names that line and exits non-zero; over a sound log it exits 0.
- A test that opens the broker over a log of 10,000 lines counts no call to audit_every_line.

**Files:**
- modify: crates/lys-secrets/src/audit.rs
- modify: crates/lys-secrets/src/broker/rotation.rs
- modify: crates/lys-secrets/src/bin/lys-secrets-demo.rs

**Checklist:**
- C415 — The whole audit log is read only by lys-secrets audit, by name, and never on a start (SECRETS-007 R2).

**Stories:**
- S166 (Operator, Keeps the accounts and revokes access) — As an operator, I want the broker's log command to show me the latest lines at once however long the log has grown, and a separate command when I want every line checked.

## Boundaries

- SHALL NOT read an audit line the command does not print, apart from lys-secrets audit.
- SHALL NOT change what an audit line holds or how it is signed.
- SHALL NOT add a timeout, deadline, sleep, poll interval, #[allow], #[ignore], unsafe code or any bypass.
- SHALL NOT add a silent default or fallback: every failure is a named refusal.

## Verification

- The full Lys gate and the ast-grep scan exit 0 at the card's head, measured by the card round.
