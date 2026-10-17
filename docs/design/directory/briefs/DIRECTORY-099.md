---
type: brief
id: DIRECTORY-099
cluster: directory
title: A refresh token whose sign-in ended is refused by that name, and every refused person sign-in is written to the log with its route, its reason and the handle tried
---

# DIRECTORY-099: A refresh token whose sign-in ended is refused by that name, and every refused person sign-in is written to the log with its route, its reason and the handle tried

> **Cluster:** directory
> **Design anchor:**
> - ADR-115 — Lys is the only sign-in a person or a product ever sees; the issuer inside it is never shown — Lys is the single sign-on for every product: every product is a client of Lys at Lys's own origin, and no product configuration names the issuer. A person meets only Lys screens: first-run setup, sign-in, provider setup and their own account are Lys pages, and the issuer's pages, admin site, name and password files are never part of any path a person follows. First run asks the person for the administrator's name, email and password; nothing is filled from the machine.
> **Checklist:**
> - C746 — A refresh token whose Lys sign-in ended is refused SessionEnded with the instant it ended, kept thirty days past its end and renewing nothing, read on the service's clock; an unknown token is still RefreshUnknown.
> - C747 — Every refused person sign-in on POST /api/sign-in and on the provider callback says one WARN line with the route, the refusal and the handle tried when it is a well-formed email, and never the password.
> - C748 — The CHANGELOG records SessionEnded for an ended sign-in and the refused-sign-in line with its fields and handle rule.
> **Stories:**
> - S423 (Administrator, Suspends, reinstates and retires identities, and reads why a check refused) — As an administrator asked why someone could not sign in, I want every refused sign-in written down with its route, its reason and the email tried, and an ended sign-in named as ended, so I can answer from the log instead of guessing.

## Purpose

On 17 and 18 October 2026 Cambium lost its sign-in four times, and each time identity.log said only "lys-identity-server token request refused: client_id cambium error invalid_grant refusal RefreshUnknown reason RefreshUnknown: that refresh token is not one Lys gave this product, or it has ended" (lines 3642, 5297 and 5453 of the install's identity.log; line 3640 is the same line for haematite). That reads as a token lost in a restart, but it was not. Refresh tokens are already kept across a restart (provider.rs:283 opens <key file>.tokens.json; tests/provider_refresh_restart.rs pins a refresh that renews after Service::restart). What happened is that the refresh token lives only as long as the Lys sign-in it stands on (provider/issue.rs:77 and :119, ends_at = the sign-in's end), and this install's sign-in lasts session_seconds 28800, eight hours. The saved Cambium refresh entry in state/provider-signing.tokens.json ended at 1792215810 (17 October 16:43:30 Melbourne). After that, ProviderTokens::get filters it out (provider_tokens.rs:133-138), every open and insert prunes it (:99, :167), and by_refresh turns the TokenUnknown into RefreshUnknown (provider/issue.rs:155). So an ended sign-in and a token Lys never gave get the same words, and SessionEnded, which already exists for a sign-out (error_provider.rs:57), is never said for a sign-in that ran out. Waffles' R1 asked for refresh tokens kept with the session store and revoked by the same acts that end sessions; at 7e3a88a7 that already holds, and the one timer is the sign-in's own eight hours. How long a sign-in lasts is this install's session_seconds setting, not code, and is Waffles' call outside this brief. The second gap: a person's refused sign-in leaves no line at all. The issuer's 401 becomes SignInRefused with no log (sign_in.rs:344-346), an empty email or password returns early with none (:630), and a refused admission after the issuer said yes (session_admission::begin) logs nothing. The provider callback writes its refusal by eprintln outside the WARN sink (sign_in_upstream.rs:313) and never names the address. On 17 October nobody could say whether Callie had tried to sign in to Cambium: the log had nothing to read.

## Task

Keep an ended refresh token's record past its end and refuse it SessionEnded, naming the instant it ended, read on the service's own clock (R1); write one WARN line through the identity server's WARN sink for every refused person sign-in on POST /api/sign-in and on the provider callback, with the route, the refusal's name and the handle tried when it is a well-formed email, and never the password (R2); record both in the CHANGELOG (R3). Out of scope: how long a sign-in lasts (the install's session_seconds), Cambium's handling of the refusal, agent and machine passes, and the access-token refusals on userinfo.

## Requirements

### R1: A refresh token whose sign-in ended is refused SessionEnded with the instant it ended

Behavioural. An entry of kind refresh in the provider's access table is kept past its expires_at for ENDED_REFRESH_KEPT_SECONDS, 2,592,000 (thirty days). It is a constant in provider_tokens.rs, chosen so that a person back after a month away still gets the right words; it costs one small entry per sign-in per app. While kept, the entry answers no token: ProviderTokens::get still answers TokenUnknown for it, and a new ProviderTokens::ended(key, at) answers Some(ended_at) for an ended refresh entry and None for every other key. Access entries are pruned at expires_at as today. Open and insert prune a refresh entry only once expires_at + ENDED_REFRESH_KEPT_SECONDS has passed. A refresh entry revoked by sign-out, by a reused code, or by retiring or suspending its holder is removed as today, never kept. by_refresh asks ended() when get() answers TokenUnknown for a refresh grant of the same client. If the entry ended, the answer is ProviderError::SessionEnded with the reason "SessionEnded: the Lys sign-in this refresh token stands on ended at <RFC 3339 UTC instant>; sign in again", carried as the error's own field. The token-request log line then reads "lys-identity-server token request refused: client_id <client> error invalid_grant refusal SessionEnded reason SessionEnded: the Lys sign-in this refresh token stands on ended at <instant>; sign in again". An unknown token, or another client's, is still RefreshUnknown. by_refresh, ProviderTokens::open and the provider's issue path read the time through the service's clock (the ClockSource that Sessions holds, Sessions::now), not session::now(), so a supplied clock moves both. The wire answer stays HTTP 400 with error invalid_grant.

**Acceptance:**
- Red first in provider_tokens_tests.rs at 7e3a88a7: a refresh entry that ends at T, reopened at T + 1, answers ended == Some(T) and get == TokenUnknown. At T + ENDED_REFRESH_KEPT_SECONDS + 1 it is gone from the file after an insert. An access entry ending at T is gone at T + 1. A revoked refresh entry answers ended == None
- Red first in tests/provider_refresh_restart.rs: started with a ManualClock and session_seconds 60, a refresh token is issued, the clock is set past the sign-in's end, the service restarts, and the refresh answers 400, refusal SessionEnded, with the reason naming the instant. The service's said lines hold exactly one token-request line with refusal SessionEnded and none with RefreshUnknown. A refresh token string Lys never issued still answers RefreshUnknown. The existing renew-after-restart test stays green
- provider_tokens.rs (255 lines of code by scripts/check_file_length.py at 7e3a88a7) and provider/issue.rs (181) stay under the gate's 500

**Files:**
- modify: crates/lys-identity-server/src/provider_tokens.rs
- modify: crates/lys-identity-server/src/provider_tokens_tests.rs
- modify: crates/lys-identity-server/src/provider/issue.rs
- modify: crates/lys-identity-server/src/provider.rs
- modify: crates/lys-identity-server/src/error_provider.rs
- modify: crates/lys-identity-server/tests/provider_refresh_restart.rs

**Checklist:**
- C746 — A refresh token whose Lys sign-in ended is refused SessionEnded with the instant it ended, kept thirty days past its end and renewing nothing, read on the service's clock; an unknown token is still RefreshUnknown.

**Stories:**
- S423 (Administrator, Suspends, reinstates and retires identities, and reads why a check refused) — As an administrator asked why someone could not sign in, I want every refused sign-in written down with its route, its reason and the email tried, and an ended sign-in named as ended, so I can answer from the log instead of guessing.

### R2: Every refused person sign-in is one WARN line naming the route, the refusal and the handle tried

Behavioural. A new module, sign_in_refusal_line.rs, holds one function that writes the line and is called from both routes, so sign_in.rs (481 lines of code at 7e3a88a7) gains at most the call. When POST /api/sign-in answers any refusal, the line is tracing::warn!(refusal = <ServerError::name>, route = "POST /api/sign-in", handle = <handle>, "a person's sign-in was refused"). This covers an empty field, the issuer's 401, throttling, IssuerRefused, IssuerChallengeExpired and an admission refused by session_admission::begin. It goes through the WARN sink (warn_lines.rs) into identity.log as: "lys-identity-server WARN lys_identity_server::sign_in_refusal_line: a person's sign-in was refused refusal=SignInRefused route=POST /api/sign-in handle=grace@example.test". When the provider callback refuses, refused_to_sign_in writes the same line with route "GET /auth/v1/providers/callback" and the ProviderRefusal's email as the handle; this replaces the eprintln at sign_in_upstream.rs:313. The handle is the text tried, trimmed and lowercased, written only when accounts::check_email admits it; otherwise the line carries handle=none when the field was empty and handle=not-an-email otherwise. A password typed into the email box therefore never reaches the log, and no newline or control character can enter a line. The password, the issuer's body and any token are never written. IssuerRefused and IssuerChallengeExpired keep their existing issuer-level WARN lines; this line is added beside them. A sign-in that succeeds writes no line.

**Acceptance:**
- Red first in tests/sign_in_refusal_lines.rs at 7e3a88a7, with the service started saying (Service::start_saying, the lines kept): a held account's email with a wrong password answers 401 SignInRefused and the said lines hold exactly one line containing "refusal=SignInRefused route=POST /api/sign-in handle=<that email, lowercased>"; an unknown email gives the same with its own handle; an empty password gives one line with refusal=SignInRefused; an email field holding "Not-The-Password-1" gives handle=not-an-email; a successful sign-in gives no line; and no said line contains either password sentinel
- Red first in the same file: a provider callback refused by the issuer's admission body (NotAdmittedYet with an email) says one line with route=GET /auth/v1/providers/callback, refusal=NotAdmittedYet and that email as the handle, and nothing is written to the process's stderr by sign_in_upstream.rs
- tests/sign_in_page.rs stays green as it is: the issuer-level capture in issuer_refusal still holds none of its sentinels
- sign_in.rs stays at or under 483 lines of code and sign_in_upstream.rs (405) under 500, by scripts/check_file_length.py

**Files:**
- create: crates/lys-identity-server/src/sign_in_refusal_line.rs
- create: crates/lys-identity-server/tests/sign_in_refusal_lines.rs
- modify: crates/lys-identity-server/src/sign_in.rs
- modify: crates/lys-identity-server/src/sign_in_upstream.rs
- modify: crates/lys-identity-server/src/lib.rs

**Checklist:**
- C747 — Every refused person sign-in on POST /api/sign-in and on the provider callback says one WARN line with the route, the refusal and the handle tried when it is a well-formed email, and never the password.

**Stories:**
- S423 (Administrator, Suspends, reinstates and retires identities, and reads why a check refused) — As an administrator asked why someone could not sign in, I want every refused sign-in written down with its route, its reason and the email tried, and an ended sign-in named as ended, so I can answer from the log instead of guessing.

### R3: The two changes are recorded

Documentary. The CHANGELOG's Unreleased section names the SessionEnded refusal for an ended refresh token with its thirty-day keeping, says that Cambium and every product read it as "sign in again", and gives the refused-sign-in line with its three fields and the handle rule.

**Acceptance:**
- The CHANGELOG entry names SessionEnded, ENDED_REFRESH_KEPT_SECONDS, the line's three fields, both routes and the handle rule
- cargo nextest run -p lys-identity-server green; cargo clippy --workspace --all-targets -- -D warnings; cargo fmt --all

**Files:**
- modify: CHANGELOG.md

**Checklist:**
- C748 — The CHANGELOG records SessionEnded for an ended sign-in and the refused-sign-in line with its fields and handle rule.

**Stories:**
- S423 (Administrator, Suspends, reinstates and retires identities, and reads why a check refused) — As an administrator asked why someone could not sign in, I want every refused sign-in written down with its route, its reason and the email tried, and an ended sign-in named as ended, so I can answer from the log instead of guessing.

## Boundaries

- SHALL NOT change session_seconds, a sign-in's length, or tie a refresh token to anything but the sign-in it was issued in.
- SHALL NOT let a kept ended refresh entry renew anything: it answers SessionEnded and nothing else.
- SHALL NOT write a password, an issuer body, a token, a token digest, or a handle that accounts::check_email does not admit.
- SHALL NOT remove or reword the existing IssuerRefused and IssuerChallengeExpired WARN lines.
- SHALL NOT keep a refresh entry that was revoked: sign-out, a reused code and a retired or suspended holder remove it as today.

## Verification

- Every test red first at 7e3a88a7; cargo nextest run only; clippy -D warnings, cargo fmt --all; the design gate (validate.py and check-coverage.py on docs/design/directory) parsed for FAIL lines.
- Done means: the four token-table pins and the restart pin of R1 green, the six route pins of R2 green, sign_in_page.rs unchanged and green, the CHANGELOG written, and Cambium's lead told that an ended sign-in now says SessionEnded.
