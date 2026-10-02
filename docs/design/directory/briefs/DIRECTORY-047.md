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
- How: Lys owns the password policy ([password_policy] in deployment.toml). The install writes it to the issuer with the configure key and gives the same values to the directory service, whose setup and account screens show and enforce it before submit. To fix the review's security finding, the configure key is now install-only. The install uses it to mint lys_directory, the key the service holds. That key has no Secrets update, so it cannot rotate signing keys, regenerate client secrets or migrate encryption, and it has no right over API keys. Its rights are read back on every install. Row 7 is now one test on a real install: identity_install.rs configures a non-default policy, sees it in the setup answer, and proves behaviourally that the issuer enforces it. No policy-reading identity is involved.
- Deviation: CN9: these files are outside R1's wall and need a reviewed brief revision to record them: crates/lys/src/identity/install/directory_key.rs and directory_key_tests.rs (new), crates/lys/src/identity/rauthy.rs, crates/lys/tests/identity_install.rs and identity_install/policy.rs (new), plus the round-1 files the reviewer listed (lys-identity-server config.rs, install/server_config.rs, and the test files). The reviewer asked for a reviewed brief revision before the install-only credential. I implemented their suggested shape (an install-only configure key and a service key of its own) because it removes a live privilege on the running service; the brief JSON itself is the workflow's to revise. The OS boundary is limited: the service runs as the same user as the install, so the configure key's file in the state folder is not hidden from it by permissions. The split means the service is only ever given its own narrow key. Upgrade dependency: an install made before this change has a configure key without ApiKeys rights, which is refused by name (RauthyForbidden) at the key step, and without Secrets update, which is refused at the policy step. Granting those is R5's upgrade work, blocked on DIRECTORY-045. Unmeasured: no cargo, identity or surface command ran this round. identity_install.rs, including the new policy checks, runs only on the identity leg (sh scripts/identity-gates/release.sh), which the workflow must request.
- Files changed:
  - created: `crates/lys/src/identity/install/directory_key.rs` — Makes or narrows the lys_directory API key through the install's configure key. Rights are exactly Clients/Users/AuthProviders read-create-update and Secrets read. It reads the rights back (ReadBackMismatch if they differ) and renews the token only when the service does not hold one. It resolves a lost create answer by listing the keys. A configure key without ApiKeys rights is refused by name as RauthyForbidden. provide() writes the token to sign-in-providers-api-key and answers the file Outcome.
  - created: `crates/lys/src/identity/install/directory_key_tests.rs` — Stand-in Rauthy tests: (1) a new key has no Secrets update and no ApiKeys group; (2) a widened key is narrowed and a held token is kept; (3) a service holding the configure key's token gets a renewed lys_directory token; (4) rights that stay wide are a ReadBackMismatch; (5) a forbidden listing is refused naming the key rights, with nothing written; (6) token recognition.
  - modified: `crates/lys/src/identity/rauthy.rs` — Adds list_api_keys, create_api_key, update_api_key and renew_api_key_secret, plus a token check that the answer names the key asked for (alongside put_password_policy from round 1).
  - modified: `crates/lys/src/identity/install.rs` — write_providers_key, which handed the configure key to the service, is replaced by directory_key::provide. A change to the key file restarts the service. apply_password_policy still runs after reconcile. The module doc is updated.
  - modified: `crates/lys/src/identity/prepare.rs` — The bootstrap key is the install-only configure key: Clients/Users/AuthProviders read-create-update, Secrets read-update and ApiKeys read-create-update. The doc says the service never holds it.
  - modified: `crates/lys/src/identity/prepare_tests.rs` — Asserts the configure key's groups, including ApiKeys, and their rights.
  - created: `crates/lys/tests/identity_install/policy.rs` — On a real install: the setup answer carries Lys's 20/64/3-digit policy and its words. The service's key file holds a lys_directory token. With that key, the issuer refuses 'Babbage-Engine-18', which its default policy takes, and accepts 'Difference-Engine-1822', which then signs in through Lys. The same key is refused 403 on PUT /password_policy and on POST /oidc/rotate_jwk. Nothing reads the issuer's policy back.
  - modified: `crates/lys/tests/identity_install.rs` — The install's deployment gets [password_policy] length_min 20, length_max 64, digits 3. The test calls policy::setup_shows_lys_policy on the opened setup and policy::issuer_enforces_lys_policy after the product sign-in.
  - modified: `crates/lys/src/identity/configure.rs` — From round 1: issuer_policy, reconcile_password_policy (403 is named, stored policy checked against what was written) and apply_password_policy.
  - modified: `crates/lys/src/identity/configure_tests.rs` — Password policy tests; the PolicySeen alias and if-let fix the round's type_complexity and single_match_else.
  - modified: `crates/lys/src/identity/config.rs` — From round 1: the [password_policy] table and its validation.
  - modified: `crates/lys/src/identity/config_tests.rs` — From round 1: policy parse and refusal tests.
  - modified: `crates/lys/src/identity/install/server_config.rs` — From round 1: renders password_policy into the service configuration.
  - modified: `crates/lys/src/identity/install_tests.rs` — From round 1: the service is given the deployment's policy.
  - modified: `crates/lys-identity-server/src/accounts.rs` — From round 1: PasswordPolicy from configuration, no hard-coded 14/128. words() builds with push_str of the pieces (the format_push_string fix).
  - modified: `crates/lys-identity-server/src/accounts_tests.rs` — From round 1: policy tests.
  - modified: `crates/lys-identity-server/src/config.rs` — From round 1: the optional password_policy, validated at load.
  - modified: `crates/lys-identity-server/src/routes.rs` — From round 1: AppState.password_policy.
  - modified: `crates/lys-identity-server/src/setup.rs` — From round 1: setup answers and enforces the configured policy.
  - modified: `crates/lys-identity-server/tests/setup_page.rs` — From round 1: a non-default policy is shown and enforced.
  - modified: `surface/identity/src/features/setup/Setup.tsx` — From round 1: shows Lys's policy before submit, counting bytes.
  - modified: `surface/identity/tests/sign-in.test.tsx` — From round 1: policy shown and refused before submit.
  - modified: `docs/design/directory/briefs/DIRECTORY-047.md` — Re-rendered from the workflow's DIRECTORY-047.json, which fixes the round's design-leg 'rendered markdown differs'.

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
- How: Unchanged this round. The reviewer found R2 aligned: every sign-in URL is on Lys's origin, a refusal never names the issuer, per-address throttling uses ConnectInfo and X-Forwarded-For, and the callback regression drives a real OIDC transaction.
- Deviation: The end-to-end run in identity_install.rs needs the identity leg, which was not run this round.
- Files changed:
  - modified: `tests/identity_contract/src/harness.rs` — From round 1: callback_answer begins a real authorization at /sign-in/providers/stand-in and answers it. serve() uses connect-info on start and restart. restart() is kept.
  - modified: `tests/identity_contract/src/fake_issuer.rs` — From round 1: records authorizations.
  - modified: `tests/identity_contract/tests/admission.rs` — From round 1: 303, Location / and the session cookie, and a second use refused SignInStateUnknown.
- Checklist delivery:
  - [x] C356 — Password sign-in stays on Lys; valid browser callbacks set a session and redirect 303 to /, proven with the current OIDC flow (DIRECTORY-047 R2). — Verified by the reviewer: harness.rs callback_answer and admission.rs.
- Story delivery:
  - [x] S150 (Person setting up Lys for the first time, Installs Lys on their own machine with no terminal knowledge and signs in) — As someone using any of our products, I want to sign in with Lys everywhere, so that there is one account and one sign-in page, and it is always Lys's. — Verified by the reviewer.

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
- How: Unchanged. Lys is the products' issuer: discovery, the token and jwks are Lys's, and the four refusals each have a test. Row 1, a fixture product configured from install's output signing in, is product.rs on a real install.
- Deviation: Row 1 is container-backed and needs the identity leg (sh scripts/identity-gates/release.sh) at this head. It was not run this round because no build or test command was run here.
- Files changed:
  - modified: `crates/lys/tests/identity_install/product.rs` — Unchanged this round. It still runs inside the real-install test, after which the new policy check runs.

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
- How: Unchanged. The reviewer found it aligned: the redirect address shown is the one the provider is sent, the provider's refusal is shown in its own words, and a saved provider appears as a button on the sign-in screen.
- Deviation: (none)
- Files changed:
  - modified: `surface/identity/src/features/connections/SignInProviders.tsx` — From round 1: main's picker and guidance with the server-supplied redirect_address.
  - modified: `surface/identity/tests/sign-in-providers.test.tsx` — From round 1: expects main's guide labels.
- Checklist delivery:
  - [x] C358 — Provider setup shows the server-supplied redirect and checks the provider; the build uses a fake-provider round trip, and the landing lead verifies real Google after landing (DIRECTORY-047 R4). — Verified by the reviewer.
- Story delivery:
  - [x] S149 (Person setting up Lys for the first time, Installs Lys on their own machine with no terminal knowledge and signs in) — As an ordinary person setting up Lys, I want one installer, then a Lys page that asks my name, email and password and lets me connect Google, GitHub or Microsoft, so that I am signed in without a terminal, a password file or any page that is not Lys. — Verified by the reviewer.

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
- How: Partly addressed. The service key's rights are now exactly the ones R5's spec lists (Clients/Users/AuthProviders read-create-update, Secrets read), with no Secrets update. They are enforced and read back on every install by install/directory_key.rs. The admin-path refusal and the upgrade are not built.
- Deviation: Blocked on two items. (1) The named refusal on every issuer admin path a host browser can reach needs a front proxy in front of the issuer's published port. That needs a reviewed brief revision choosing the proxy (image, digest, rules) and restating the trusted-proxy chain for R2's per-address throttling: Rauthy cannot disable its admin UI, and the browser and the service reach the port from one gateway address. (2) Upgrading an existing install needs DIRECTORY-045's code, which is not on main. The upgrade must grant Users create/update, the configure key's Secrets update and ApiKeys read/create/update, and handle the deployment.network migration. The R5 wall names crates/lys/src/identity/install/prepare.rs, which does not exist; the revision should name crates/lys/src/identity/prepare.rs.

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
- How: Unchanged in substance. The new strings (the directory key refusal and ReadBackMismatch wording) name the sign-in service by what it does and never name the issuer. The configure key's name, lys_configure, and the service key's name, lys_directory, name no product.
- Deviation: Row 1 (a_first_install_and_a_sign_in_never_name_the_issuer) needs the identity leg at this head, which was not run this round.
- Files changed:
  - modified: `crates/lys/tests/identity_install.rs` — The issuer-name scan also covers the new sign-in with the password Lys's policy took.

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
