---
type: brief
id: DIRECTORY-118
cluster: directory
title: Every line Lys says carries the instant it was said, and a sign-in refusal names who was refused
---

# DIRECTORY-118: Every line Lys says carries the instant it was said, and a sign-in refusal names who was refused

> **Cluster:** directory
> **Checklist:**
> - C841 — Every line Lys says and every WARN event it routes begins with the RFC 3339 UTC instant it was said, from the one sink; a sign-in refusal that names an email or handle says it on its line, never a password. DIRECTORY-118/R1.
> **Stories:**
> - S476 (Directory administrator, Resolves a refused act) — As a Directory administrator reading identity.log for why a person could not sign in, I want every line to say when it was said and whom it refused, so that I can find their attempt among everyone's and tell them what happened.

## Purpose

Found 23 Oct 2026 reading the installed Lys's identity.log for Dean's sign-in that went 'round and round': the log held POST /api/sign-in refusals and five GET /auth/v1/providers/callback refusals NotAdmittedYet with handle=none, and not one line could be placed in time or attributed to a person, so which attempt was Dean's and when it happened could not be read off the log at all. Every line the service says goes through one sink, main.rs line 35: `println!("lys-identity-server {line}")`, with no instant; WARN and above events reach the same sink (warn_lines::install, DIRECTORY-095) and carry a level and a target but no instant either. The same log's last 400 lines held 224 copies of one refusals-feed line (DIRECTORY-119) that also cannot be placed in time. The provider-callback refusal (sign_in_refusals.rs lines 24 to 40) knows the email Rauthy named ('User with email ... already exists but is not linked to this provider') and the line says handle=none. A log a person cannot read for when and for whom is not a log.

## Task

Stamp every line Lys says with the instant it was said, at the one sink, and make a sign-in refusal line name the handle or email it was refused for whenever the refusal names one.

## Requirements

### R1: The instant at the sink; the subject on a sign-in refusal

THE SYSTEM SHALL write every line it says, and every WARN or above event routed to the same sink, as the instant it was said in RFC 3339 UTC with milliseconds, one space, then the line as today, at the single sink in main.rs so no caller can say a line without it; the test capture Say keeps receiving the unstamped line. WHEN a provider-callback or password sign-in is refused and the refusal names an email or a handle (Rauthy's 'User with email ... is not linked', a well-formed email tried) THE SYSTEM SHALL say that email or handle on the refusal line in place of handle=none; a refusal naming nobody says handle=none as today, and a password is never said.

**Acceptance:**
- crates/lys-identity-server/tests/sign_in_refusal_lines.rs: a provider-callback refusal whose Rauthy message names dean@ablative.com.au is said with that email on the line, and the password sign-in refusal line for a well-formed email names it; neither line contains the password.
- crates/lys-identity-server/tests/said_lines_stamped.rs: the service started with the real sink (not the capture) writes its listening line and one WARN event to a file, and every line of that file begins with an RFC 3339 UTC instant with milliseconds followed by one space and `lys-identity-server `.

**Files:**
- create: crates/lys-identity-server/tests/said_lines_stamped.rs
- modify: crates/lys-identity-server/src/main.rs
- modify: crates/lys-identity-server/src/sign_in_refusals.rs
- modify: crates/lys-identity-server/tests/sign_in_refusal_lines.rs

**Checklist:**
- C841 — Every line Lys says and every WARN event it routes begins with the RFC 3339 UTC instant it was said, from the one sink; a sign-in refusal that names an email or handle says it on its line, never a password. DIRECTORY-118/R1.

**Stories:**
- S476 (Directory administrator, Resolves a refused act) — As a Directory administrator reading identity.log for why a person could not sign in, I want every line to say when it was said and whom it refused, so that I can find their attempt among everyone's and tell them what happened.

## Boundaries

- No new logging framework and no second sink: the instant is added where the line is printed.
- Nothing else about a line changes: its words, its order and its route stay.
- No password, token or key ever appears on a line, as today.

## Verification

- cargo nextest run -p lys-identity-server -E 'test(/sign_in_refusal/) | test(/said_lines/)'
- The installed identity.log after the next upgrade: every line begins with an instant.
