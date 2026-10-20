---
type: brief
id: DIRECTORY-110
cluster: directory
title: Lys sends its own letters through the install's mail settings, and admits no one it cannot write to
---

# DIRECTORY-110: Lys sends its own letters through the install's mail settings, and admits no one it cannot write to

> **Cluster:** directory
> **Depends on:** DIRECTORY-101
> **Checklist:**
> - C804 — The install reads one [mail] section, keeps its password as a credential, and refuses plain SMTP to any host but loopback. DIRECTORY-110/R1.
> - C805 — The issuer always gets SMTP, and a measured leg decides whether it or Lys sends the added letter, so a person never gets two letters for one act. DIRECTORY-110/R2.
> - C806 — Lys sends added, access given and founded from one sender, each asserted word for word at a capturing sink, once per act. DIRECTORY-110/R3.
> - C807 — With no mail settings, admit refuses as mail_unconfigured before recording anything. DIRECTORY-110/R4.
> - C808 — The copy-invite is gone; the receipt says Lys wrote to the person. DIRECTORY-110/R5.
> **Stories:**
> - S459 (Directory administrator, Resolves a refused act) — As a Directory administrator, I want Lys to write to each person I admit, once, through our own mail settings, and to refuse to admit anyone when it cannot write, so that nobody is added and left to hear about it by chance.

## Purpose

ORGANISATION-ALONE (docs/design/onboarding, lys main 09fdff4c; Tom, 20 Oct 2026 10:5x: an organisation onboards itself, seamless, alone, without him or his devices) names this seam as IDENTITY-004; it is written here because docs/design/identity sits outside the design gate (gate.sh measures clusters with a design.json; Tom, 26 Sep 2026, roadmap.json: an IDENTITY draft there 'does not land'). Step 4 of the page is MISSING: a person Lys admits receives nothing. The invitation is text the administrator copies by hand (surface/identity/src/features/people/AddPerson.tsx:106-138, 'Copy invite'); Lys has no mail path (no mail crate in any Cargo.toml); the issuer could send mail and does not, because the environment Lys writes for it (crates/lys/src/identity/prepare.rs render_env, deploy/identity/compose.yaml:84-117) carries no SMTP key, and deployment.toml (install/deployment.template.toml, config.rs DeploymentConfig) has no mail section. Founding (the page's DIRECTORY-110) cannot exist without a letter that proves an address. Waffles' rules, 20 Oct 11:06: one letter per act reaches the person, never two, and if the issuer's own mail on POST /users duplicates Lys's letter the brief decides by test which goes; every letter is proven in the gate against a capturing SMTP sink with its text asserted, never a live mailbox. Hermes, 20 October 2026.

## Task

Add a [mail] section to deployment.toml and the server configuration; write the issuer's SMTP keys from it; send Lys's three letters (added, access given, founded) from one sender with one template each; refuse admit by name when mail is not configured; remove the copy-invite; give the issuer SMTP, and settle by a measured leg whether the issuer's own POST /users mail is the added letter.

## Requirements

### R1: The install carries one mail section, its password a credential

Behavioural. THE SYSTEM SHALL read an optional [mail] section in deployment.toml with host, port, user, from (an address, optionally with a display name) and tls (implicit, starttls or none), and SHALL keep the SMTP password as a credential file under state, generated or supplied like the issuer's other secrets, never in deployment.toml or the server configuration. tls = none SHALL be accepted only for the hosts 127.0.0.1, localhost and [::1], and refused for any other host as mail_tls_required naming the host. The server configuration (install/server_config/rendering.rs) SHALL carry the same settings with password_file in place of the password. A deployment.toml with no [mail] SHALL install and start as today.

**Acceptance:**
- A deployment.toml with [mail] renders a server configuration whose mail block names host, port, user, from, tls and password_file, and no password text anywhere in either file
- tls = none with host smtp.example.com is refused at install as mail_tls_required naming smtp.example.com; with host 127.0.0.1 it is accepted
- An install with no [mail] renders the configuration it renders today, byte for byte

**Files:**
- modify: crates/lys/src/identity/config.rs
- modify: crates/lys/src/identity/install/deployment.template.toml
- modify: crates/lys/src/identity/install/server_config/rendering.rs
- modify: crates/lys/src/identity/prepare.rs
- modify: crates/lys-identity-server/src/config.rs

**Checklist:**
- C804 — The install reads one [mail] section, keeps its password as a credential, and refuses plain SMTP to any host but loopback. DIRECTORY-110/R1.

**Stories:**
- S459 (Directory administrator, Resolves a refused act) — As a Directory administrator, I want Lys to write to each person I admit, once, through our own mail settings, and to refuse to admit anyone when it cannot write, so that nobody is added and left to hear about it by chance.

### R2: The issuer gets SMTP; a measured leg decides which of the two sends the added letter

Measured, then behavioural. THE SYSTEM SHALL write the [mail] settings to the issuer as RAUTHY_SMTP_* in render_env and carry them in compose.yaml to the keys the pinned issuer reads (quoted from its configuration reference in the END), in every outcome, so a person can recover a password with nobody from us present (Waffles 11:22, inside Tom's bar). A leg SHALL run the pinned issuer image (ghcr.io/sebadob/rauthy:0.36.2 at the digest compose.yaml pins, already on the machine; nothing is downloaded) with its SMTP pointed at a capturing sink, make an account by POST /users exactly as accounts::make does (email, names, no password), and record whether a letter arrives and its subject and body. WHEN no letter arrives, Lys SHALL send all three letters (R3). WHEN a letter arrives and the call has no setting that suppresses it, the issuer's letter SHALL be the added letter and Lys SHALL NOT send its own added letter, sending only access given and founded. Either way one letter per act; the END SHALL record which outcome the leg found.

**Acceptance:**
- The leg's record, quoted in the END: the image digest, the request body, and whether a letter reached the sink (with its subject and body if one did)
- render_env with [mail] writes the RAUTHY_SMTP_* keys and compose.yaml maps each to the issuer's key; with no [mail], neither carries one
- A test pins the outcome the leg found: admitting one person delivers exactly one added letter at the sink, from Lys or from the issuer as the leg decided
- No secret appears in the rendered environment's log or in the leg's output (the rauthy_ready_leg's record search, reused)

**Files:**
- create: crates/lys/tests/rauthy_mail_leg.rs
- modify: crates/lys/src/identity/prepare.rs
- modify: deploy/identity/compose.yaml

**Checklist:**
- C805 — The issuer always gets SMTP, and a measured leg decides whether it or Lys sends the added letter, so a person never gets two letters for one act. DIRECTORY-110/R2.

**Stories:**
- S459 (Directory administrator, Resolves a refused act) — As a Directory administrator, I want Lys to write to each person I admit, once, through our own mail settings, and to refuse to admit anyone when it cannot write, so that nobody is added and left to hear about it by chance.

### R3: One sender, one template per letter, every letter asserted at a sink

Behavioural. THE SYSTEM SHALL send mail from one module (crates/lys-identity-server/src/mail.rs) through the configured settings, with exactly three letters, each one template: added (to a person admitted, naming the address they sign in at and, when Lys offers Google, that they choose it with the email Lys knows them by: today's invitation text, now sent by Lys), access given (to a person when a root grant names them, naming what they may now do in the words the access screen uses) and founded (to a founder, carrying the proving link; written and tested here, called by the founding brief). A letter SHALL be sent after the act it reports is recorded and SHALL never be sent for an act that was refused. A send that fails SHALL leave the act recorded and SHALL be answered as mail_not_sent naming the letter, the recipient's address and the server's reply, so the act can be asked again with the same operation to resend; a repeat of an act whose letter was sent SHALL NOT send it again.

**Acceptance:**
- Against a capturing SMTP sink in the tests (a test-owned listener on 127.0.0.1, tls none): each letter's recipient, sender, subject and body are asserted word for word
- Admitting the same person twice with the same operation delivers one letter; with the letter's first send refused by the sink, the second ask delivers it once
- An admit refused at any step delivers no letter

**Files:**
- create: crates/lys-identity-server/src/mail.rs
- create: crates/lys-identity-server/src/mail_letters.rs
- create: crates/lys-identity-server/tests/mail_letters.rs
- modify: crates/lys-identity-server/src/people_admit.rs
- modify: crates/lys-identity-server/src/grants/handlers.rs
- modify: crates/lys-identity-server/Cargo.toml

**Checklist:**
- C806 — Lys sends added, access given and founded from one sender, each asserted word for word at a capturing sink, once per act. DIRECTORY-110/R3.

**Stories:**
- S459 (Directory administrator, Resolves a refused act) — As a Directory administrator, I want Lys to write to each person I admit, once, through our own mail settings, and to refuse to admit anyone when it cannot write, so that nobody is added and left to hear about it by chance.

### R4: No mail settings, no admit: the refusal is named

Behavioural. WHEN the server has no mail settings, THE SYSTEM SHALL refuse POST /people/admit before its first step as mail_unconfigured, saying that a person admitted now would receive nothing and naming the [mail] section of deployment.toml, and SHALL list mail_unconfigured among the route's refusals in the OpenAPI table. A root grant issued to a person by an install without mail SHALL still be recorded (it is the administrator's act on an existing person), and its answer SHALL say that no letter was sent.

**Acceptance:**
- An admit against a server with no mail block answers 409 mail_unconfigured and records nothing
- The OpenAPI document lists mail_unconfigured for POST /people/admit
- A root grant on a server with no mail answers its grant and letter: not_sent mail_unconfigured

**Files:**
- modify: crates/lys-identity-server/src/people_admit.rs
- modify: crates/lys-identity-server/src/openapi_table.rs
- modify: crates/lys-identity-server/src/openapi_refusals.rs

**Checklist:**
- C807 — With no mail settings, admit refuses as mail_unconfigured before recording anything. DIRECTORY-110/R4.

**Stories:**
- S459 (Directory administrator, Resolves a refused act) — As a Directory administrator, I want Lys to write to each person I admit, once, through our own mail settings, and to refuse to admit anyone when it cannot write, so that nobody is added and left to hear about it by chance.

### R5: The copy-invite goes; the receipt says the letter went

Behavioural. THE SYSTEM SHALL remove the invitation text and the Copy invite act from AddPerson.tsx; the receipt SHALL say that Lys wrote to the person at their email, or SHALL show the mail_not_sent or mail_unconfigured refusal in the words the server gave.

**Acceptance:**
- vitest: after an admit, the receipt reads that Lys wrote to the email, and no Copy invite act is on the page
- vitest: an admit answered mail_unconfigured shows that refusal and no receipt

**Files:**
- modify: surface/identity/src/features/people/AddPerson.tsx
- modify: surface/identity/src/features/people/AddPerson.test.tsx

**Checklist:**
- C808 — The copy-invite is gone; the receipt says Lys wrote to the person. DIRECTORY-110/R5.

**Stories:**
- S459 (Directory administrator, Resolves a refused act) — As a Directory administrator, I want Lys to write to each person I admit, once, through our own mail settings, and to refuse to admit anyone when it cannot write, so that nobody is added and left to hear about it by chance.

## Boundaries

- No live mailbox in any test or gate leg; every letter is asserted at a test-owned sink.
- No third-party mail API; the install's own SMTP server only.
- One letter per act: never Lys's and the issuer's both (R2's leg decides which sends the added letter); the issuer has SMTP in every outcome.
- The SMTP password is never in deployment.toml, the server configuration, a log, an error or a test's output.
- The sender is lettre 0.11 (Waffles 11:22: yes), the smallest feature set giving the message builder, the SMTP transport and tokio with rustls, no DNS resolver features; one cargo fetch, offline after, its fetched size posted.
- Founding itself (route, proving page, states) is the founding brief's; this brief writes and tests only its letter.
- No #[allow]; nextest only; red first; every new file under 500 lines.

## Verification

- The reds red at the brief commit and green at the fix commit through the Go gate on the Mac, named in the END with their seconds.
- R2's leg run once on Tom's Mac against the installed image, its record quoted in the END, and the chosen branch named.
- The closing note says plainly that R1 to R5 are done, or names what is not.
