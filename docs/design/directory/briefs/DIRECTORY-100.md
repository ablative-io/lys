---
type: brief
id: DIRECTORY-100
cluster: directory
title: The development start script's readiness watcher reads a child's lines as they arrive on every machine, so the development scripts test passes under mawk as it does under BSD awk
---

# DIRECTORY-100: The development start script's readiness watcher reads a child's lines as they arrive on every machine, so the development scripts test passes under mawk as it does under BSD awk

> **Cluster:** directory
> **Depends on:** DIRECTORY-099
> **Design anchor:**
> - ADR-115 — Lys is the only sign-in a person or a product ever sees; the issuer inside it is never shown — Lys is the single sign-on for every product: every product is a client of Lys at Lys's own origin, and no product configuration names the issuer. A person meets only Lys screens: first-run setup, sign-in, provider setup and their own account are Lys pages, and the issuer's pages, admin site, name and password files are never part of any path a person follows. First run asks the person for the administrator's name, email and password; nothing is filled from the machine.
> **Checklist:**
> - C750 — start_ready in surface/identity/dev/start.sh watches a child's output with a POSIX sh read loop, never awk, so the ready line is seen the moment it is written under mawk as under BSD awk.
> **Stories:**
> - S424 (Person running Lys's tests on a shared machine, Runs the test suites on a laptop other seats also build on) — As a person running Lys's tests on a Linux machine whose awk is mawk, I want the development start script to see a child's ready line the moment it is written, so that the development scripts test passes there as it does on a Mac instead of timing out with an empty log.

## Purpose

On 18 October 2026 the Lys battery on the 205 (Debian, /usr/bin/awk = mawk 1.3.4 20250131) failed one test: tests/identity_contract/tests/dev_scripts.rs the_real_development_scripts_seed_their_named_grants_on_fresh_state timed out at nextest's 300 s at c5bda0bf, while the same test is green on the Mac (BSD awk 20200816). Read live during the run: the dev seed had finished, dev_issuer was listening and answering /.well-known/openid-configuration, start.sh was waiting in start_ready issuer, the awk watcher was alive with the output FIFO on its stdin and logs/issuer.log on its stdout, and issuer.log was empty; lys-identity-server was never started. The cause is the watcher: start_ready in surface/identity/dev/start.sh reads the child's output FIFO with awk, and mawk block-buffers input from a pipe, so the issuer's "dev issuer ... listening on ..." line stays in mawk's read buffer until the buffer fills or the child ends; the ready line is never written and start.sh's read on the ready pipe waits forever. Reproduced in isolation on the 205 with the script's exact awk program and a writer holding the FIFO open: mawk gives no log line and no readiness in 4 s; mawk -W interactive and a POSIX sh read loop give both at once. dev_issuer prints the line on Linux (checked by hand to a file and to a pipe). start_ready is byte-identical at 840798b8 and c5bda0bf, so the defect is the script's, not DIRECTORY-099's.

## Task

Make start_ready's watcher a POSIX sh read loop that writes each line to the child's log as it arrives and signals ready on the first line containing "listening on ", or ended when the output closes without one, the same on every awk; prove it by the development scripts test on a mawk machine. Out of scope: the test's own fixed waits (the seed script's unbounded curl, the test naming which script stalled), the dev seed, and what the scripts start.

## Requirements

### R1: start_ready reads lines as they arrive without awk

Behavioural. In surface/identity/dev/start.sh, start_ready's watcher subshell no longer runs awk. It reads the output FIFO with `while IFS= read -r line`, writes each line to $LOGS/$name.log with printf as it arrives, writes "ready" to the ready pipe on the first line containing "listening on " and "ended" to it when the output closes without such a line; a last line without a newline is still written to the log. The rest of start_ready is unchanged: the pid file, the wait for the ready pipe, the error line naming the log when the child ended first. The script runs no awk at all.

**Acceptance:**
- Red first on a mawk machine: at c5bda0bf on the 205 (mawk 1.3.4), tests/identity_contract/tests/dev_scripts.rs the_real_development_scripts_seed_their_named_grants_on_fresh_state TIMEOUT at 300 s with logs/issuer.log empty while the issuer answers discovery (the run of 18 October 00:13 UTC); at this brief's commit the same test passes on the 205 and on the Mac
- start.sh invokes no awk: `grep -v '^ *#' surface/identity/dev/start.sh | grep -c awk` answers 0 (the watcher's comment names mawk as the reason, and is the only mention)
- The test is unchanged by this brief

**Files:**
- modify: surface/identity/dev/start.sh

**Checklist:**
- C750 — start_ready in surface/identity/dev/start.sh watches a child's output with a POSIX sh read loop, never awk, so the ready line is seen the moment it is written under mawk as under BSD awk.

**Stories:**
- S424 (Person running Lys's tests on a shared machine, Runs the test suites on a laptop other seats also build on) — As a person running Lys's tests on a Linux machine whose awk is mawk, I want the development start script to see a child's ready line the moment it is written, so that the development scripts test passes there as it does on a Mac instead of timing out with an empty log.

## Boundaries

- SHALL NOT change what start.sh starts, in what order, or the readiness line it looks for.
- SHALL NOT add an awk option (-W interactive is mawk's alone; BSD awk and gawk refuse it) or a dependence on gawk.
- SHALL NOT change the development scripts test or its fixture; its own fixed waits are another brief.

## Verification

- Red first on the 205 at c5bda0bf; cargo nextest run only (`-E 'binary(dev_scripts)'`); the design gate (validate.py and check-coverage.py on docs/design/directory) parsed for FAIL lines.
- Done means: the test green on the 205 and on the Mac at the commit, start.sh free of awk, and the Lys battery on the 205 re-fired at that commit.
