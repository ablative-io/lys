---
type: brief
id: DIRECTORY-047
cluster: directory
title: Lys is the only sign-in anyone sees: first-run setup, sign-in, providers and accounts on Lys pages, every product a client of Lys
---

# DIRECTORY-047: Lys is the only sign-in anyone sees: first-run setup, sign-in, providers and accounts on Lys pages, every product a client of Lys

> **Cluster:** directory
> **Depends on:** DIRECTORY-045
> **Design anchor:**
> - ADR-115 — Lys is the only sign-in a person or a product ever sees; the issuer inside it is never shown — Lys is the single sign-on for every product: every product is a client of Lys at Lys's own origin, and no product configuration names the issuer. A person meets only Lys screens: first-run setup, sign-in, provider setup and their own account are Lys pages, and the issuer's pages, admin site, name and password files are never part of any path a person follows. First run asks the person for the administrator's name, email and password; nothing is filled from the machine.
> **Checklist:**
> - C355 — First run asks for the administrator on Lys's setup page without machine defaults; Lys owns the password policy, writes it during preparation, and shows that same value (DIRECTORY-047 R1).
> - C356 — Password sign-in stays on Lys; valid browser callbacks set a session and redirect 303 to /, proven with the current OIDC flow (DIRECTORY-047 R2).
> - C357 — Every product, Cambium first, is a client of Lys at Lys's origin; no product configuration names the issuer (DIRECTORY-047 R3).
> - C358 — Provider setup shows the server-supplied redirect and checks the provider; the build uses a fake-provider round trip, and the landing lead verifies real Google after landing (DIRECTORY-047 R4).
> - C359 — Account changes use Lys screens; reachable issuer admin paths explicitly refuse host browsers while required loopback calls work, and upgrade handles rights and recorded network choices before this card lands after 045 (DIRECTORY-047 R5).
> - C360 — Nothing a person reads (screens, install output, refusals, page titles) names the issuer (DIRECTORY-047 R6).
> **Stories:**
> - S149 (Person setting up Lys for the first time, Installs Lys on their own machine with no terminal knowledge and signs in) — As an ordinary person setting up Lys, I want one installer, then a Lys page that asks my name, email and password and lets me connect Google, GitHub or Microsoft, so that I am signed in without a terminal, a password file or any page that is not Lys.
> - S150 (Person setting up Lys for the first time, Installs Lys on their own machine with no terminal knowledge and signs in) — As someone using any of our products, I want to sign in with Lys everywhere, so that there is one account and one sign-in page, and it is always Lys's.

## Purpose

Tom's word on 28 September 2026: 'Lys is meant to be the sign-in for everything. It's meant to be our only own single sign-on for absolutely everything. Cambium uses Lys, not Rauthy.' and 'How is an average person supposed to do it?' That night the live install showed the issuer's own sign-in page, kept the administrator in the issuer's admin site, took the administrator's email from the machine, and needed a password file read in Terminal. This brief makes Lys the one sign-in a person or product meets, and makes first-run setup something an ordinary person can do. Tom, 20:00: 'The issuer's styling on the page is all still there. I really want that done properly, to match the rest of Lys's styling; it shouldn't feel out of it at all.' Amended 28 September 2026 20:55 by Waffles after Archie's read of the brief against the tree.

## Task

Move every path a person follows (first run, sign-in, provider setup, own account, administration of accounts) onto Lys pages served by lys-identity-server, with the service speaking to the issuer server-side; make Lys the OpenID provider every product is registered with; remove every default and every mention of the issuer from what a person sees. In this continuation, resolve the main/card integration and every prior review gap under the revised R1, R2, R4 and R5 below. Historical dev/review records from an earlier head are not acceptance of these revised requirements. Remeasure the final committed tree, including rendered design documents; retain main's provider picker and guidance while using the server-supplied callback address.

## Requirements

### R1: First run is a Lys setup page that asks for the administrator; install fills nothing from the machine

Behavioural. Today first-run setup (crates/lys-identity-server/src/setup.rs, surface/identity/src/features/setup/Setup.tsx) runs after the configured administrator has already signed in at the issuer, whose account install bootstraps from --admin-email (crates/lys/src/identity/cli.rs, default admin@identity.test) and a generated password kept in the state folder. Change it so the person creates the administrator on the Lys setup page: --admin-email loses its default, and a first install with none creates no administrator in the issuer; it writes a one-time setup code to the state folder (owner-only) and prints and opens http://localhost:8490/setup. The setup screen, served only while no administrator exists and the code matches, asks name, email and password (entered twice, Lys's configured password policy shown before submit); the service creates the account in the issuer through its API key, runs the existing setup act for it, deletes the code, and signs the person in. --admin-email stays for unattended installs and is validated as an email. No field is ever filled from git config, the environment or any other machine default, and no person is ever asked to read a password file. The issuer always makes its own bootstrap account on first start; that account is a machine account with a fixed address naming no person and a generated password nobody reads, and 'exactly one administrator' counts people. The first person is made by the setup page. The setup code is never printed and never written where a person reads it: it rides only in the fragment of the address handed to the browser opener. A headless install (no browser) writes the code to a 0600 file in the install root and says so in one line naming the path; with --admin-email the password still comes only from the setup page with the code, never from a generated file. A forgotten password has a path on the machine: `lys identity setup-code` writes a fresh one-time setup code by the same fragment rule, and the setup page with it sets a new password for the named administrator. Lys owns the password policy in its own configuration. Install preparation writes that value to the issuer through the existing bootstrap settings path; the setup and sign-in pages read Lys's value. The issuer is not the source of truth shown to a person. Do not copy limits 14/128 from an issuer default or introduce a machine identity to read the issuer's policy.

**Acceptance:**
- A first install with no --admin-email creates no administrator, prints the setup address, and the setup route answers the setup screen.
- The setup route refuses by name once an administrator exists, and with a wrong or used code.
- Completing setup creates exactly one administrator with the entered email and leaves the person signed in to Lys.
- A test sets git user.email and every EMAIL-like environment variable to a marker and proves the marker appears nowhere in state, deployment.toml or the issuer.
- The bootstrap account names no person, and a people count after setup is one.
- No output line or log holds the setup code; `lys identity setup-code` lets a sole administrator set a new password.
- A test configures a non-default password policy in Lys, prepares the install, and proves the issuer is configured with that same policy and the setup/sign-in page shows Lys's value before submit; no hard-coded copy of the issuer's defaults or policy-reading machine identity is used.

**Files:**
- modify: crates/lys/src/identity/cli.rs
- modify: crates/lys/src/identity/install.rs
- modify: crates/lys/src/identity/install/layout.rs
- modify: crates/lys-identity-server/src/setup.rs
- modify: crates/lys-identity-server/src/routes.rs
- modify: surface/identity/src/features/setup/Setup.tsx
- modify: crates/lys/src/identity/config.rs
- modify: crates/lys/src/identity/prepare.rs
- modify: crates/lys/src/identity/configure.rs
- modify: crates/lys-identity-server/src/accounts.rs

**Checklist:**
- C355 — First run asks for the administrator on Lys's setup page without machine defaults; Lys owns the password policy, writes it during preparation, and shows that same value (DIRECTORY-047 R1).

**Stories:**
- S149 (Person setting up Lys for the first time, Installs Lys on their own machine with no terminal knowledge and signs in) — As an ordinary person setting up Lys, I want one installer, then a Lys page that asks my name, email and password and lets me connect Google, GitHub or Microsoft, so that I am signed in without a terminal, a password file or any page that is not Lys.

#### R1 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Row 1 (no --admin-email makes no administrator, prints the setup address, setup route answers): met by the card's existing code, which I did not change: install.rs has no administrator path, and setup_page.rs the_setup_route_answers_the_setup_screen_for_the_pending_code_only still asserts status 200 and purpose first-run. Row 2 (refused by name once an administrator exists, and with a wrong or used code): unchanged, setup.rs admit() gives SetupClosed and SetupCodeRefused, tested in setup_page.rs. Row 3 (exactly one administrator, signed in): unchanged, setup_page.rs completing_setup_makes_one_administrator_and_signs_them_in. Row 4 (machine markers): unchanged, crates/lys/tests/cli_tests/identity_setup.rs. Row 5 (bootstrap account names no person, people count one): unchanged, prepare.rs BOOTSTRAP_ACCOUNT_EMAIL plus the setup_page count. Row 6 (no setup code in output; setup-code recovery): unchanged, setup_code.rs hand_over and setup_page.rs a_password_code_sets_the_administrator_a_new_password. Row 7 (Lys-owned policy), newly met across a chain of tests: config.rs:179/206 is Lys's own policy; configure.rs:289 writes it through the bootstrap-key path, now with Secrets update (prepare.rs:167); configure_tests.rs:199 proves the issuer receives and stores exactly the non-default policy; install_tests.rs:435 proves the service is given the same values; setup_page.rs:155 proves the setup route shows exactly Lys's configured view; sign-in.test.tsx:119 proves the page shows the words and refuses before submit. The 14/128 constants are removed from accounts.rs, and no machine identity reads the issuer's policy.
- Deviation: Row 7 is proved by a chain of tests, one per boundary (configuration, install to issuer, install to service configuration, service route, screen), not by one real-install test; identity_install.rs, which runs only on the identity leg, was not extended. Two source files outside R1's wall were edited (CN9): crates/lys-identity-server/src/config.rs, to add the service's password_policy field, and crates/lys/src/identity/rauthy.rs, because RauthyApi's request method is private and the PUT had to live there. The bootstrap key now asks for Secrets update, a right the brief did not list; it is the right the issuer requires to write its policy (Rauthy v0.36.2 generic.rs put_password_policy). On an install made before this change, the key lacks that right, and install then refuses rauthy_forbidden naming Secrets update; granting it is R5's upgrade work (DIRECTORY-045). Lys's default (15 to 64, no composition rule) is a choice I made from NIST SP 800-63B-4, to avoid copying the issuer's 14/128; it needs a ruling if a different value is wanted.
- Files changed:
  - modified: `crates/lys/src/identity/config.rs` — New [password_policy] table (PasswordPolicy, line 179). When the table is absent, Lys's own default applies (line 206): NIST SP 800-63B-4 3.1.1.2, 15 to 64 characters, no composition rule. Validated by field against the ranges the issuer can hold (line 350).
  - modified: `crates/lys/src/identity/configure.rs` — issuer_policy (263) maps Lys's policy to the issuer's request, with valid_days absent. reconcile_password_policy (289) writes it whole, checks that the stored answer is what was written, and refuses a key without Secrets update by name. apply_password_policy (325) runs it with the install's configure key.
  - modified: `crates/lys/src/identity/rauthy.rs` — put_password_policy (288): PUT /auth/v1/password_policy, returning the stored policy.
  - modified: `crates/lys/src/identity/prepare.rs` — The bootstrap API key asks for Secrets read and update (167), because the issuer keeps its password policy under Secrets.
  - modified: `crates/lys/src/identity/prepare_tests.rs` — Asserts Secrets [read, update]. Main's https cookie test now trusts the example network's gateway (212), the only proxy the card's rule accepts.
  - modified: `crates/lys/src/identity/install.rs` — Install writes Lys's password policy to the sign-in service after registering clients (243) and says so in Lys's words.
  - modified: `crates/lys/src/identity/install/server_config.rs` — identity.json carries password_policy (93) with the same values install writes to the issuer.
  - modified: `crates/lys-identity-server/src/accounts.rs` — The hard-coded PASSWORD_MIN/MAX and password_policy() are gone. PasswordPolicy (48) is read from configuration. validate, words, view (148) and check (162) count length and character kinds the way the issuer does. check_password (192) and set_password take the configured policy.
  - modified: `crates/lys-identity-server/src/accounts_tests.rs` — Tests a non-default policy: it takes what it asks for, refuses each shortfall (4 counted), shows only configured values, refuses out-of-range fields by name (5 counted), and with no policy refuses only an empty password.
  - modified: `crates/lys-identity-server/src/config.rs` — Optional password_policy field (150), validated at load (197).
  - modified: `crates/lys-identity-server/src/routes.rs` — AppState carries password_policy (49, 140). Also the integration fix: StatusCode and IntoResponse imports restored (10-11) for the browser callback.
  - modified: `crates/lys-identity-server/src/setup.rs` — The setup page answers Lys's configured policy view (275), never one read from the issuer. Setup and password recovery check against it (309, 376).
  - modified: `crates/lys-identity-server/tests/setup_page.rs` — The setup service is configured with a non-default Lys policy (12-48, two digits, one special). The setup route must answer exactly that view and its words (155), and a one-digit password is refused before anything is sent (304).
  - modified: `crates/lys/src/identity/configure_tests.rs` — A fake issuer policy endpoint. Tests: Lys's non-default policy is sent whole and stored (199); a policy stored otherwise is a read_back_mismatch (227); a key that cannot write it is refused naming Secrets update (238).
  - modified: `crates/lys/src/identity/config_tests.rs` — Tests: the absent table gives Lys's default, a configured table is taken whole (194), and out-of-range fields are refused by name (223, 5 counted).
  - modified: `crates/lys/src/identity/install_tests.rs` — The service configuration carries the deployment's non-default policy, and the issuer request carries the same values (435).
  - modified: `surface/identity/src/features/setup/Setup.tsx` — Handles policy being null. Length is measured as the service measures it (UTF-8 bytes). The policy words show before submit, and a password outside the policy is refused with those words before posting.
  - modified: `surface/identity/tests/sign-in.test.tsx` — POLICY is a non-default Lys value. New tests: a too-short or too-long password is refused with the policy words and nothing is posted (119); no policy note appears when the service names none (136).
- Checklist delivery:
  - [x] C355 — First run asks for the administrator on Lys's setup page without machine defaults; Lys owns the password policy, writes it during preparation, and shows that same value (DIRECTORY-047 R1). — Lys owns the policy in deployment.toml, writes it to the issuer during install, gives the same values to the service, and the setup page shows and checks them. No 14/128 copy and no policy-reading identity.
- Story delivery:
  - [x] S149 (Person setting up Lys for the first time, Installs Lys on their own machine with no terminal knowledge and signs in) — As an ordinary person setting up Lys, I want one installer, then a Lys page that asks my name, email and password and lets me connect Google, GitHub or Microsoft, so that I am signed in without a terminal, a password file or any page that is not Lys. — The first-run setup page asks for name, email and password and shows Lys's own policy before submit.

**Review (recorded):**

- Alignment: drifted
- Acceptance verdicts:
  - [x] A first install with no --admin-email creates no administrator, prints the setup address, and the setup route answers the setup screen. — install.rs has no administrator path (unchanged); crates/lys-identity-server/tests/setup_page.rs the_setup_route_answers_the_setup_screen_for_the_pending_code_only passes (round tests leg exit 0)
  - [x] The setup route refuses by name once an administrator exists, and with a wrong or used code. — setup.rs admit() SetupClosed/SetupCodeRefused; setup_page.rs refusal tests pass in round tests leg
  - [x] Completing setup creates exactly one administrator with the entered email and leaves the person signed in to Lys. — setup_page.rs completing_setup_makes_one_administrator_and_signs_them_in, round tests leg exit 0
  - [x] A test sets git user.email and every EMAIL-like environment variable to a marker and proves the marker appears nowhere in state, deployment.toml or the issuer. — crates/lys/tests/cli_tests/identity_setup.rs (unchanged this round), round tests leg exit 0
  - [x] The bootstrap account names no person, and a people count after setup is one. — prepare.rs:42 BOOTSTRAP_ACCOUNT_EMAIL bootstrap@machine.lys.test; setup_page.rs people count assertion
  - [x] No output line or log holds the setup code; `lys identity setup-code` lets a sole administrator set a new password. — setup_code.rs hand_over; setup_page.rs a_password_code_sets_the_administrator_a_new_password; the new install line 'Lys's password policy set' carries no secret
  - [ ] A test configures a non-default password policy in Lys, prepares the install, and proves the issuer is configured with that same policy and the setup/sign-in page shows Lys's value before submit; no hard-coded copy of the issuer's defaults or policy-reading machine identity is used. — Proved only as a chain of per-hop tests against fakes. configure_tests.rs lys_password_policy_is_written_whole_and_rauthy_stores_that_policy uses a fake TCP issuer. install_tests.rs the_service_is_given_the_password_policy_the_deployment_states checks rendered config only. setup_page.rs:155 and sign-in.test.tsx cover the page side. No test prepares an install and shows the real issuer holds the policy: identity_install.rs was not extended. The 14/128 copy is removed (accounts.rs) and no policy-reading identity exists.
- Issues:
  - The bootstrap key's rights now include Secrets update (prepare.rs:167). install.rs write_providers_key hands the same key to the long-running directory service as sign-in-providers-api-key. In Rauthy v0.36.2, Secrets update also authorizes POST /oidc/rotate_jwk (oidc.rs:791), PUT /clients/{id}/secret (clients.rs:573) and POST /encryption/migrate (generic.rs:175). The directory service can therefore rotate the issuer's signing keys and regenerate any client's secret. R5's spec lists the key's rights, and Secrets update is not among them. Must be done: a reviewed brief revision that writes the policy without leaving Secrets update on the key the service holds, for example a credential used only at install, then the implementation.
  - CN9: files outside R1's wall were edited without a reviewer-approved revision: crates/lys-identity-server/src/config.rs, crates/lys/src/identity/rauthy.rs, crates/lys/src/identity/install/server_config.rs (R4's wall), and test files configure_tests.rs, config_tests.rs, install_tests.rs, prepare_tests.rs, accounts_tests.rs and setup_page.rs. Must be done: record them in a reviewed brief revision.
  - Row 7 is unmet as a single test. Must be done: extend identity_install.rs so a real install with a non-default [password_policy] proves the issuer enforces it behaviourally, for example a password that meets the issuer default but breaks Lys's policy is refused, without any policy-reading identity. Then run the identity leg.
  - Clippy failed in both shapes: configure_tests.rs:150 type_complexity and :163 single_match_else. With those fixed, a hidden accounts.rs:146 format_push_string also failed.
  - An install made before this change refuses rauthy_forbidden at apply_password_policy until the key gains Secrets update. That depends on R5's upgrade work.
- Fixes:
  - configure_tests.rs: added the PolicySeen type alias, and changed match fixed to if let/else.
  - accounts.rs PasswordPolicy::words: replaced push_str(&format!(..)) with push_str of the pieces. fmt and clippy are clean in both feature shapes.

### R2: Password sign-in is a Lys page; the browser never reaches the issuer's pages

Behavioural. GET /login (crates/lys-identity-server/src/routes.rs:319) today redirects to the issuer's authorize page. It serves Lys's sign-in screen instead: email, password and one button per enabled provider. The password form posts to the service, which runs the issuer's authorization server-side for the lys-platform client (including its proof of work and PKCE), completes the code exchange as /callback does today, and sets the session. A refusal is worded by Lys: wrong email or password is one message that does not say which. Provider buttons start the provider flow through the service and return through /callback; a person passes through the provider's own consent page only. The sign-in, first-run setup and account screens are Lys screens in every visible respect: built in surface/identity from the same design tokens, forms.css inputs, selects and buttons, typography, spacing and layout shell as the other screens, with Lys's name and mark; no issuer template, stylesheet, font, colour or default browser control appears on any page a person sees. Because every password sign-in reaches the issuer from the service's address, the service passes the person's address in the forwarded header and the issuer trusts that header from the service's address only (proxy mode on, trusted source set to the service), so one person's failed sign-ins bar only that person. Accounts Lys makes have a password only; an account that asks for a second factor is refused by name (second_factor_unsupported), because a passkey is bound to the origin the browser sees.

**Acceptance:**
- An end-to-end test signs in with email and password and records every URL the browser is sent to: every one is on Lys's origin.
- A wrong password and an unknown email give the same Lys refusal, and neither names the issuer.
- A provider sign-in's recorded URLs are Lys's origin and the provider's, never the issuer's.
- A surface test renders the sign-in and setup screens and finds only the shared form, button and layout classes and no unstyled default control; an end-to-end check loads the served sign-in page and finds no stylesheet or asset served from the issuer.
- Repeated wrong passwords from one address do not bar a sign-in from another address.
- An account with a passkey is refused second_factor_unsupported.
- A callback regression begins a real OIDC transaction through the current card's flow and follows its valid callback with a browser Accept header: status 303, Location /, and a session cookie are all asserted. Replace the obsolete issuer_answer fixture that expected GET /login to redirect to the issuer; retain the card's first-visit and password tests, unknown-state refusal, and single-use checks. The contract harness retains restart() and supplies SocketAddr connect information through serve() on both initial start and restart.

**Files:**
- create: crates/lys-identity-server/src/sign_in.rs
- create: surface/identity/src/features/sign-in/SignIn.tsx
- modify: crates/lys-identity-server/src/routes.rs
- modify: tests/identity_contract/src/harness.rs
- modify: tests/identity_contract/tests/admission.rs

**Checklist:**
- C356 — Password sign-in stays on Lys; valid browser callbacks set a session and redirect 303 to /, proven with the current OIDC flow (DIRECTORY-047 R2).

**Stories:**
- S149 (Person setting up Lys for the first time, Installs Lys on their own machine with no terminal knowledge and signs in) — As an ordinary person setting up Lys, I want one installer, then a Lys page that asks my name, email and password and lets me connect Google, GitHub or Microsoft, so that I am signed in without a terminal, a password file or any page that is not Lys.
- S150 (Person setting up Lys for the first time, Installs Lys on their own machine with no terminal knowledge and signs in) — As someone using any of our products, I want to sign in with Lys everywhere, so that there is one account and one sign-in page, and it is always Lys's.

#### R2 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Row 1 (every URL on Lys's origin): unchanged, sign_in_page.rs every_address_the_browser_is_sent_to_is_on_lys_origin. Row 2 (one refusal for wrong email or password): unchanged, sign_in_page.rs plus error.rs SignInRefused. Row 3 (provider URLs only Lys's and the provider's): unchanged, sign_in_page.rs a_provider_sign_in_passes_through_the_provider_and_lys_only. Row 4 (shared styling; no issuer asset): unchanged, sign-in.test.tsx 'is built from the shared page, field and button styling' and sign_in_page.rs the_served_sign_in_page_loads_nothing_from_the_issuer. Row 5 (per-address throttling): unchanged, sign_in_page.rs failed_sign_ins_from_one_address_do_not_bar_another. Row 6 (second factor refused by name): unchanged, sign_in_page.rs an_account_asking_for_a_second_factor_is_refused_by_name. Row 7 (callback regression): now met. harness.rs:439 begins a real OIDC transaction through GET /sign-in/providers/{id}, which calls Oidc::begin and opens the authorization at the issuer. The fake issuer's authorize endpoint answers that authorization with its state, nonce and PKCE challenge. admission.rs:88 follows the callback with Accept text/html and asserts 303, Location / and a cookie, then asserts a second use is refused SignInStateUnknown. The obsolete fixture that expected GET /login to redirect to the issuer is removed; the first-visit, password and unknown-state tests are kept (admission.rs:103). restart() is kept, and serve() supplies connect info on start and restart (harness.rs:261).
- Deviation: (none)
- Files changed:
  - modified: `tests/identity_contract/src/harness.rs` — Integration fix: the location() helper is restored (235) for get_page. issuer_answer is replaced by callback_answer (439), which begins a real sign-in through the service's provider button and has the issuer answer that same authorization. restart() is kept (418); both the first start and restart serve with SocketAddr connect info (261). password_policy: None is added to the Config literal.
  - modified: `tests/identity_contract/tests/admission.rs` — a_browser_coming_back_from_sign_in_is_taken_to_the_screens_signed_in (88) now uses callback_answer and asserts 303, Location /, a session cookie, and a 400 SignInStateUnknown when the same answer is used again. The first-visit test (103), with its password sign-in and unknown-state refusal, is kept.
  - modified: `tests/identity_contract/src/fake_issuer.rs` — Records the query of every authorization opened at the issuer's sign-in start (authorizations()), so a test can answer the transaction the service began.
  - modified: `crates/lys-identity-server/src/routes.rs` — StatusCode and IntoResponse imports restored, which the browser branch of /callback needs.
- Checklist delivery:
  - [x] C356 — Password sign-in stays on Lys; valid browser callbacks set a session and redirect 303 to /, proven with the current OIDC flow (DIRECTORY-047 R2). — The browser callback on a real OIDC transaction gives 303, Location /, a session cookie, and single use.
- Story delivery:
  - [x] S149 (Person setting up Lys for the first time, Installs Lys on their own machine with no terminal knowledge and signs in) — As an ordinary person setting up Lys, I want one installer, then a Lys page that asks my name, email and password and lets me connect Google, GitHub or Microsoft, so that I am signed in without a terminal, a password file or any page that is not Lys. — Sign-in stays on Lys pages.
  - [x] S150 (Person setting up Lys for the first time, Installs Lys on their own machine with no terminal knowledge and signs in) — As someone using any of our products, I want to sign in with Lys everywhere, so that there is one account and one sign-in page, and it is always Lys's. — One Lys sign-in page; the callback lands on Lys's screens.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] An end-to-end test signs in with email and password and records every URL the browser is sent to: every one is on Lys's origin. — crates/lys-identity-server/tests/sign_in_page.rs every_address_the_browser_is_sent_to_is_on_lys_origin; round tests leg exit 0
  - [x] A wrong password and an unknown email give the same Lys refusal, and neither names the issuer. — sign_in_page.rs, error.rs SignInRefused; round tests leg exit 0
  - [x] A provider sign-in's recorded URLs are Lys's origin and the provider's, never the issuer's. — sign_in_page.rs a_provider_sign_in_passes_through_the_provider_and_lys_only
  - [x] A surface test renders the sign-in and setup screens and finds only the shared form, button and layout classes and no unstyled default control; an end-to-end check loads the served sign-in page and finds no stylesheet or asset served from the issuer. — surface/identity/tests/sign-in.test.tsx 'is built from the shared page, field and button styling' (surface leg exit 0); sign_in_page.rs the_served_sign_in_page_loads_nothing_from_the_issuer
  - [x] Repeated wrong passwords from one address do not bar a sign-in from another address. — sign_in_page.rs failed_sign_ins_from_one_address_do_not_bar_another
  - [x] An account with a passkey is refused second_factor_unsupported. — sign_in_page.rs an_account_asking_for_a_second_factor_is_refused_by_name
  - [x] A callback regression begins a real OIDC transaction through the current card's flow and follows its valid callback with a browser Accept header: status 303, Location /, and a session cookie are all asserted. Replace the obsolete issuer_answer fixture that expected GET /login to redirect to the issuer; retain the card's first-visit and password tests, unknown-state refusal, and single-use checks. The contract harness retains restart() and supplies SocketAddr connect information through serve() on both initial start and restart. — harness.rs:439 callback_answer begins at GET /sign-in/providers/stand-in and answers the recorded authorization (fake_issuer.rs authorizations()). admission.rs:88 asserts 303, Location /, the cookie, and a second use refused 400 SignInStateUnknown. issuer_answer is removed; restart() is kept (harness.rs:418); serve() is shared by start and restart (harness.rs:246).
- Checklist verified: C356
- Stories verified: S150

### R3: Lys is the OpenID provider every product is registered with

Behavioural. Today crates/lys-identity-server/src/oidc.rs makes Lys a client of the issuer, and install registers Cambium as the issuer's client. Add the provider side: lys-identity-server serves /.well-known/openid-configuration, authorize, token, userinfo and jwks at its own origin under Lys's issuer URL, completing sign-in through R2's screens and signing ID tokens with a key the install keeps in the state folder. No product is registered by install and no product is named in Lys: a product registers itself as a client of Lys through the app registration API (DIRECTORY-048), and the configuration a product receives names only Lys's address. Until DIRECTORY-048 lands, acceptance uses a test fixture client. The issuer's ports are published only on the container network, not on the host. The issuer's port stays on loopback, because the directory service is a host process that reaches it there and a Mac has no route from the host into the container network; the requirement is that Lys never sends a browser to it and no page, refusal or redirect gives its address (moving the service into the compose network is its own card). The provider signs tokens with EdDSA (Ed25519) and its jwks carries that key; refusals are named, each with a test: a redirect address not registered, a code used twice, a wrong PKCE verifier, a code past its instant. Expiry instants are data in tokens, compared on use, not waits, so the no-deadline boundary does not apply to them.

**Acceptance:**
- a fixture product configured from install's output signs in with every URL on Lys's origin or a provider's.
- Discovery at Lys's origin answers Lys's issuer, and a token Lys issues verifies against Lys's jwks.
- No page, refusal or redirect from Lys contains the issuer's loopback address or port.
- Each of the four refusals has its own test.

**Files:**
- create: crates/lys-identity-server/src/provider.rs
- modify: crates/lys-identity-server/src/oidc.rs
- modify: crates/lys/src/identity/configure.rs
- modify: deploy/identity/compose.yaml

**Checklist:**
- C357 — Every product, Cambium first, is a client of Lys at Lys's origin; no product configuration names the issuer (DIRECTORY-047 R3).

**Stories:**
- S150 (Person setting up Lys for the first time, Installs Lys on their own machine with no terminal knowledge and signs in) — As someone using any of our products, I want to sign in with Lys everywhere, so that there is one account and one sign-in page, and it is always Lys's.

#### R3 — Execution record

**Dev (recorded):**

- Status: implemented
- How: No change this round. The card's provider.rs, oidc.rs, configure.rs and compose.yaml were integrated as they were, and their earlier review verdicts still stand on this head. Row 1: crates/lys/tests/identity_install/product.rs configures a fixture product from install's identity.json. Row 2: tests/provider.rs a_product_signs_in_through_lys_and_verifies_the_token_against_lys_keys. Row 3: identity_install.rs scans for the issuer's loopback address and port, and configuration.rs and connections.rs assert it is absent. Row 4: tests/provider.rs has one test per refusal (unregistered redirect, code used twice, wrong PKCE verifier, code past its instant). I checked the integration touched none of these paths except routes.rs imports.
- Deviation: (none)
- Checklist delivery:
  - [x] C357 — Every product, Cambium first, is a client of Lys at Lys's origin; no product configuration names the issuer (DIRECTORY-047 R3). — Products are clients of Lys at Lys's origin; no change was needed this round.
- Story delivery:
  - [x] S150 (Person setting up Lys for the first time, Installs Lys on their own machine with no terminal knowledge and signs in) — As someone using any of our products, I want to sign in with Lys everywhere, so that there is one account and one sign-in page, and it is always Lys's. — Products sign in through Lys only.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [ ] a fixture product configured from install's output signs in with every URL on Lys's origin or a provider's. — Only crates/lys/tests/identity_install/product.rs covers this. It is container-backed and runs only on the identity leg, which this round did not request. Unmeasured at this head.
  - [x] Discovery at Lys's origin answers Lys's issuer, and a token Lys issues verifies against Lys's jwks. — crates/lys-identity-server/tests/provider.rs a_product_signs_in_through_lys_and_verifies_the_token_against_lys_keys; round tests leg exit 0
  - [x] No page, refusal or redirect from Lys contains the issuer's loopback address or port. — tests/configuration.rs and tests/connections.rs assert absence (round tests leg exit 0). The real-install scan in identity_install.rs was not run this round.
  - [x] Each of the four refusals has its own test. — tests/provider.rs: unregistered redirect, code used twice, wrong PKCE verifier, code past its instant
- Issues:
  - Row 1 is unmeasured. Must be done: run the identity leg (sh scripts/identity-gates/release.sh) at this head so product.rs runs on a real install.

### R4: Providers are set up inside Lys with every step shown

Behavioural. surface/identity/src/features/connections/SignInProviders.tsx: for Google, GitHub and Microsoft the screen shows the exact redirect address to paste (with a copy button), a link to the provider's page that creates a client, then the client id, secret and (Microsoft) tenant fields. Saving sets the provider through the service as today and then proves it by fetching the provider's discovery or authorize endpoint with the new client id; a failure names what the provider said. The issuer builds the provider redirect address itself as its public address plus /auth/v1/providers/callback, so the issuer's public address (RAUTHY_PUB_URL) is set to Lys's own origin, Lys serves that exact path, and the service finishes the issuer's provider callback from the server with the cookie and state it held from the start of the flow, so the browser never lands on an issuer page. The issuer name in the tokens changes with it, and oidc.rs checks the new name. The redirect address shown to paste is Lys's origin with that path; a provider already registered with the old address needs the new one added, which the Connections screen says in those words. The card round proves the provider round trip with its fake-provider fixture. A real Google sign-in is a separate post-landing verification performed by the landing lead; it is not a live demonstration required inside the build loop.

**Acceptance:**
- The redirect address shown equals the one the service sends to the provider.
- Saving a provider with a client id the provider rejects is refused with the provider's words.
- A saved provider appears as a button on R2's sign-in screen.
- A fake-provider sign-in goes from Lys's page to the fixture provider and back to a Lys page with no issuer page in the browser's history.

**Files:**
- modify: crates/lys-identity-server/src/oidc.rs
- modify: crates/lys-identity-server/src/sign_in_providers.rs
- modify: crates/lys/src/identity/install/server_config.rs
- modify: deploy/identity/compose.yaml
- modify: surface/identity/src/features/connections/SignInProviders.tsx

**Checklist:**
- C358 — Provider setup shows the server-supplied redirect and checks the provider; the build uses a fake-provider round trip, and the landing lead verifies real Google after landing (DIRECTORY-047 R4).

**Stories:**
- S149 (Person setting up Lys for the first time, Installs Lys on their own machine with no terminal knowledge and signs in) — As an ordinary person setting up Lys, I want one installer, then a Lys page that asks my name, email and password and lets me connect Google, GitHub or Microsoft, so that I am signed in without a terminal, a password file or any page that is not Lys.

#### R4 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Row 1 (redirect shown equals the one sent): sign_in_providers.rs the_redirect_address_shown_is_the_one_the_provider_is_asked_with; the screen shows data.redirect_address from the service (SignInProviders.tsx:102). Row 2 (a rejected client id is refused in the provider's words): sign_in_providers.rs a_client_id_the_provider_rejects_is_refused_in_the_providers_words. Row 3 (a saved provider is a sign-in button): sign_in_providers.rs a_saved_provider_is_offered_on_the_sign_in_page and sign-in.test.tsx 'offers one button per provider, each starting at Lys'. Row 4 (fake-provider round trip, no issuer page in history): sign_in_page.rs a_provider_sign_in_passes_through_the_provider_and_lys_only. Main's picker and guidance are kept together with the server-supplied callback, as the task asks.
- Deviation: (none)
- Files changed:
  - modified: `surface/identity/src/features/connections/SignInProviders.tsx` — The integrated screen keeps main's provider picker (77) and step guidance from provider-guides.ts, and shows the server-supplied redirect_address with a copy button (102) and the old-address note. The module doc no longer says providers are set 'at the issuer'.
  - modified: `surface/identity/tests/sign-in-providers.test.tsx` — Integration fix: the test expected the card's removed CONSOLES labels. It now expects main's guide link labels ('Create a client', 'New registration'), and still asserts the server's redirect address and the old-address note.
- Checklist delivery:
  - [x] C358 — Provider setup shows the server-supplied redirect and checks the provider; the build uses a fake-provider round trip, and the landing lead verifies real Google after landing (DIRECTORY-047 R4). — Server-supplied redirect with main's picker and guidance; round trip proved with the fake provider. The real Google check is the landing lead's post-landing step.
- Story delivery:
  - [x] S149 (Person setting up Lys for the first time, Installs Lys on their own machine with no terminal knowledge and signs in) — As an ordinary person setting up Lys, I want one installer, then a Lys page that asks my name, email and password and lets me connect Google, GitHub or Microsoft, so that I am signed in without a terminal, a password file or any page that is not Lys. — Provider setup happens on Lys's Connections screen with every step shown.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] The redirect address shown equals the one the service sends to the provider. — sign_in_providers.rs the_redirect_address_shown_is_the_one_the_provider_is_asked_with; SignInProviders.tsx shows data.redirect_address; sign-in-providers.test.tsx asserts #sign-in-redirect value
  - [x] Saving a provider with a client id the provider rejects is refused with the provider's words. — sign_in_providers.rs a_client_id_the_provider_rejects_is_refused_in_the_providers_words
  - [x] A saved provider appears as a button on R2's sign-in screen. — sign_in_providers.rs a_saved_provider_is_offered_on_the_sign_in_page; sign-in.test.tsx 'offers one button per provider'
  - [x] A fake-provider sign-in goes from Lys's page to the fixture provider and back to a Lys page with no issuer page in the browser's history. — sign_in_page.rs a_provider_sign_in_passes_through_the_provider_and_lys_only
- Checklist verified: C358
- Stories verified: S149

### R5: A person's own account, and administration of accounts, are Lys screens

Behavioural. A You-screen section changes the signed-in person's email (confirmed by re-entering their password) and password; an administrator's screen changes any person's email, resets a password, and enables or disables a person. Each is carried out by the service against the issuer. Keep the issuer's loopback publication for the Lys service, but every issuer admin path reachable by a host browser must answer an explicit, named refusal. Wider network isolation is a separate card, not this round. The service's key needs Users create and update rights beyond its present Clients, Secrets read, Users read and AuthProviders (prepare.rs). A new install asks for them in its bootstrap key. An existing install gains them through the install's configure key when upgraded (DIRECTORY-045); if that key cannot grant them, the upgrade is refused naming the missing right and the one step that grants it, with nothing half-changed. The upgrade also handles deployment.network missing from an existing deployment.toml: migrate it using the recorded install choices or refuse by name with the required operator action before changing anything; do not leave a partly changed install. This round may build before DIRECTORY-045 lands, but this card lands only after DIRECTORY-045, and the upgrade behavior must then be measured on the integrated tree.

**Acceptance:**
- Changing one's email on the You screen changes it in the issuer, and the person signs in with the new email.
- An administrator's reset lets the person sign in with the new password and refuses the old one.
- A host-browser request to every reachable issuer admin path gets the explicit named refusal while the Lys service's required loopback calls still work; the test does not claim that a published loopback port cannot be connected to.
- An existing install is upgraded with Users create/update and deployment.network handled from its recorded state; an ungrantable right or an unavailable migration choice is refused with the specific missing right or choice and the operator action, before any partial change.

**Files:**
- create: surface/identity/src/features/people/Account.tsx
- create: crates/lys-identity-server/src/accounts.rs
- modify: crates/lys/src/identity/install/prepare.rs
- modify: surface/identity/src/features/me/You.tsx
- modify: crates/lys/src/identity/prepare.rs

**Checklist:**
- C359 — Account changes use Lys screens; reachable issuer admin paths explicitly refuse host browsers while required loopback calls work, and upgrade handles rights and recorded network choices before this card lands after 045 (DIRECTORY-047 R5).

**Stories:**
- S149 (Person setting up Lys for the first time, Installs Lys on their own machine with no terminal knowledge and signs in) — As an ordinary person setting up Lys, I want one installer, then a Lys page that asks my name, email and password and lets me connect Google, GitHub or Microsoft, so that I am signed in without a terminal, a password file or any page that is not Lys.

#### R5 — Execution record

**Dev (recorded):**

- Status: blocked
- How: Row 1 (changing one's email on the You screen): met by the card, crates/lys-identity-server/tests/accounts.rs a_changed_email_signs_in_and_the_old_one_does_not. Row 2 (an administrator's reset): met, tests/accounts.rs the_administrator_resets_a_password_and_disables_a_person. The new-install half of the key rights is met: prepare.rs:166-169 asks for Users read/create/update, and now also Secrets update for R1. Row 3 (a named refusal on every issuer admin path a host browser can reach, while the service's loopback calls still work) is NOT met. Rauthy v0.36.2 always serves /auth/v1/admin; its only related setting, ADMIN_BUTTON_HIDE, hides a button (config.toml:290-298). The host browser and the Lys service both reach the published 127.0.0.1 port from the same gateway address, so the refusal has to come from a proxy in front of that port that tells them apart, for example by path and the service's API-Key header. That means a new compose service and image. It also moves the peer address the issuer's TRUSTED_PROXIES and PEER_IP_HEADER_NAME rely on for R2's per-person throttling. I could not choose or verify that change on a real install in this loop. Row 4 (upgrading an existing install) is NOT met: DIRECTORY-045's upgrade code is not in this tree (it exists only on origin/card/DIRECTORY-045-onmain, unlanded), and the brief's own R5 wall names crates/lys/src/identity/install/prepare.rs, a file that does not exist. The brief says this card lands only after 045 and the upgrade is measured on the integrated tree.
- Deviation: Nothing was built for rows 3 and 4. Row 3 needs a reviewed choice of front proxy (image, digest, rules) and a restated trusted-proxy chain for R2. Row 4 needs DIRECTORY-045 on main, including a way to grant an existing key Users create/update and Secrets update, and the deployment.network migration.
- Checklist delivery:
  - [ ] C359 — Account changes use Lys screens; reachable issuer admin paths explicitly refuse host browsers while required loopback calls work, and upgrade handles rights and recorded network choices before this card lands after 045 (DIRECTORY-047 R5). — Account screens are done. The admin-path refusal and the upgrade clause are blocked as described.
- Story delivery:
  - [ ] S149 (Person setting up Lys for the first time, Installs Lys on their own machine with no terminal knowledge and signs in) — As an ordinary person setting up Lys, I want one installer, then a Lys page that asks my name, email and password and lets me connect Google, GitHub or Microsoft, so that I am signed in without a terminal, a password file or any page that is not Lys. — Account changes happen on Lys screens, but R5's admin-path and upgrade rows are open.

**Review (recorded):**

- Alignment: drifted
- Acceptance verdicts:
  - [x] Changing one's email on the You screen changes it in the issuer, and the person signs in with the new email. — crates/lys-identity-server/tests/accounts.rs a_changed_email_signs_in_and_the_old_one_does_not
  - [x] An administrator's reset lets the person sign in with the new password and refuses the old one. — tests/accounts.rs the_administrator_resets_a_password_and_disables_a_person
  - [ ] A host-browser request to every reachable issuer admin path gets the explicit named refusal while the Lys service's required loopback calls still work; the test does not claim that a published loopback port cannot be connected to. — Nothing built. No front proxy and no test exist in the diff.
  - [ ] An existing install is upgraded with Users create/update and deployment.network handled from its recorded state; an ungrantable right or an unavailable migration choice is refused with the specific missing right or choice and the operator action, before any partial change. — No upgrade code in the tree. DIRECTORY-045 is not on main.
- Issues:
  - Must be built: the named refusal on every issuer admin path a host browser can reach, while the service's loopback calls still work. Per the developer, this needs a reviewed brief revision choosing a front proxy (image, digest, rules) and restating the trusted-proxy chain for R2's per-address throttling.
  - Must be built: the upgrade of an existing install (Users create/update, and now also however R1's policy write is resolved, plus the deployment.network migration) on the integrated tree after DIRECTORY-045 lands. The brief's R5 wall names crates/lys/src/identity/install/prepare.rs, which does not exist; the revision must correct it.
  - The key's rights now include Secrets update, beyond what R5's spec lists (see R1's first issue).

### R6: Nothing a person reads names the issuer

Behavioural. Install output, every refusal returned to a browser, every served screen, page title and email use Lys's name. The issuer is named only in operator log files.

**Acceptance:**
- A test runs a first install and a sign-in and scans stdout, stderr, every served page and every refusal body for the issuer's name: none.
- The built screens contain no occurrence of the issuer's name.

**Files:**
- modify: crates/lys/src/identity/install.rs
- modify: crates/lys-identity-server/src/error.rs
- modify: surface/identity/src

**Checklist:**
- C360 — Nothing a person reads (screens, install output, refusals, page titles) names the issuer (DIRECTORY-047 R6).

**Stories:**
- S149 (Person setting up Lys for the first time, Installs Lys on their own machine with no terminal knowledge and signs in) — As an ordinary person setting up Lys, I want one installer, then a Lys page that asks my name, email and password and lets me connect Google, GitHub or Microsoft, so that I am signed in without a terminal, a password file or any page that is not Lys.
- S150 (Person setting up Lys for the first time, Installs Lys on their own machine with no terminal knowledge and signs in) — As someone using any of our products, I want to sign in with Lys everywhere, so that there is one account and one sign-in page, and it is always Lys's.

#### R6 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Row 1 (scan install and sign-in output for the issuer's name): crates/lys/tests/identity_install.rs a_first_install_and_a_sign_in_never_name_the_issuer. The only new install line this round, "Lys's password policy set" (install.rs), names Lys only; the new refusal for a key without the right, configure.rs reconcile_password_policy, reaches a person through said_in_lys_words. Row 2 (built screens): sign-in.test.tsx 'never names the sign-in service behind Lys in any screen source'. Nothing I added under surface/identity/src names the issuer; the SignInProviders doc comment that said 'set at the issuer' is reworded.
- Deviation: (none)
- Checklist delivery:
  - [x] C360 — Nothing a person reads (screens, install output, refusals, page titles) names the issuer (DIRECTORY-047 R6). — No new text a person reads names the issuer.
- Story delivery:
  - [x] S149 (Person setting up Lys for the first time, Installs Lys on their own machine with no terminal knowledge and signs in) — As an ordinary person setting up Lys, I want one installer, then a Lys page that asks my name, email and password and lets me connect Google, GitHub or Microsoft, so that I am signed in without a terminal, a password file or any page that is not Lys. — Install and setup read as Lys.
  - [x] S150 (Person setting up Lys for the first time, Installs Lys on their own machine with no terminal knowledge and signs in) — As someone using any of our products, I want to sign in with Lys everywhere, so that there is one account and one sign-in page, and it is always Lys's. — Screens and refusals name only Lys.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [ ] A test runs a first install and a sign-in and scans stdout, stderr, every served page and every refusal body for the issuer's name: none. — crates/lys/tests/identity_install.rs a_first_install_and_a_sign_in_never_name_the_issuer runs only on the identity leg, which this round did not request. Unmeasured. By reading, the new strings ('Lys's password policy set' and the configure.rs refusal) do not name the issuer.
  - [x] The built screens contain no occurrence of the issuer's name. — sign-in.test.tsx 'never names the sign-in service behind Lys in any screen source'; surface leg exit 0; the SignInProviders.tsx doc no longer says 'at the issuer'
- Issues:
  - Row 1 is unmeasured. Must be done: run the identity leg at this head.

## Boundaries

- SHALL NOT fill any field about a person from git config, environment variables or any machine default.
- SHALL NOT send a person's browser to the issuer's pages on any path.
- SHALL NOT add a timeout, deadline, sleep, poll interval, #[allow], #[ignore] or any bypass.
- SHALL NOT print or log a password, client secret, API key or setup code.

## Verification

- The full Lys gate and the surface checks exit 0 at the card's head, measured by the card round.
- R2 and R6's end-to-end tests run a real install on a scratch estate and pass.
- After this card and DIRECTORY-045 have landed, the landing lead performs a real Google sign-in from the Lys page to Google and back to a Lys page, recording that no issuer page enters browser history. This is post-landing verification, separate from the fake-provider build-loop acceptance.
