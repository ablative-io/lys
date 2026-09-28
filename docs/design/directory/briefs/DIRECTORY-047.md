---
type: brief
id: DIRECTORY-047
cluster: directory
title: Lys is the only sign-in anyone sees: first-run setup, sign-in, providers and accounts on Lys pages, every product a client of Lys
---

# DIRECTORY-047: Lys is the only sign-in anyone sees: first-run setup, sign-in, providers and accounts on Lys pages, every product a client of Lys

> **Cluster:** directory
> **Design anchor:**
> - ADR-115 — Lys is the only sign-in a person or a product ever sees; the issuer inside it is never shown — Lys is the single sign-on for every product: every product is a client of Lys at Lys's own origin, and no product configuration names the issuer. A person meets only Lys screens: first-run setup, sign-in, provider setup and their own account are Lys pages, and the issuer's pages, admin site, name and password files are never part of any path a person follows. First run asks the person for the administrator's name, email and password; nothing is filled from the machine.
> **Checklist:**
> - C355 — First run asks the person for the administrator's name, email and password on a Lys setup page; install fills nothing from the machine and has no default administrator (DIRECTORY-047 R1).
> - C356 — Password sign-in happens on a Lys page at Lys's origin; the browser never reaches the issuer's pages (DIRECTORY-047 R2).
> - C357 — Every product, Cambium first, is a client of Lys at Lys's origin; no product configuration names the issuer (DIRECTORY-047 R3).
> - C358 — Google, GitHub and Microsoft are set up inside Lys, which shows the exact address to paste and tests the provider on save (DIRECTORY-047 R4).
> - C359 — A person changes their own email and password, and an administrator changes anyone's, on Lys screens; the issuer's admin site is not reachable from the host (DIRECTORY-047 R5).
> - C360 — Nothing a person reads (screens, install output, refusals, page titles) names the issuer (DIRECTORY-047 R6).
> **Stories:**
> - S149 (Person setting up Lys for the first time, Installs Lys on their own machine with no terminal knowledge and signs in) — As an ordinary person setting up Lys, I want one installer, then a Lys page that asks my name, email and password and lets me connect Google, GitHub or Microsoft, so that I am signed in without a terminal, a password file or any page that is not Lys.
> - S150 (Person setting up Lys for the first time, Installs Lys on their own machine with no terminal knowledge and signs in) — As someone using any of our products, I want to sign in with Lys everywhere, so that there is one account and one sign-in page, and it is always Lys's.

## Purpose

Tom's word on 28 September 2026: 'Lys is meant to be the sign-in for everything. It's meant to be our only own single sign-on for absolutely everything. Cambium uses Lys, not Rauthy.' and 'How is an average person supposed to do it?' That night the live install showed the issuer's own sign-in page, kept the administrator in the issuer's admin site, took the administrator's email from the machine, and needed a password file read in Terminal. This brief makes Lys the one sign-in a person or product meets, and makes first-run setup something an ordinary person can do. Tom, 20:00: 'The issuer's styling on the page is all still there. I really want that done properly, to match the rest of Lys's styling; it shouldn't feel out of it at all.'

## Task

Move every path a person follows (first run, sign-in, provider setup, own account, administration of accounts) onto Lys pages served by lys-identity-server, with the service speaking to the issuer server-side; make Lys the OpenID provider every product is registered with; remove every default and every mention of the issuer from what a person sees.

## Requirements

### R1: First run is a Lys setup page that asks for the administrator; install fills nothing from the machine

Behavioural. Today first-run setup (crates/lys-identity-server/src/setup.rs, surface/identity/src/features/setup/Setup.tsx) runs after the configured administrator has already signed in at the issuer, whose account install bootstraps from --admin-email (crates/lys/src/identity/cli.rs, default admin@identity.test) and a generated password kept in the state folder. Change it so the person creates the administrator on the Lys setup page: --admin-email loses its default, and a first install with none creates no administrator in the issuer; it writes a one-time setup code to the state folder (owner-only) and prints and opens http://localhost:8490/setup. The setup screen, served only while no administrator exists and the code matches, asks name, email and password (entered twice, the issuer's password policy shown before submit); the service creates the account in the issuer through its API key, runs the existing setup act for it, deletes the code, and signs the person in. --admin-email stays for unattended installs and is validated as an email. No field is ever filled from git config, the environment or any other machine default, and no person is ever asked to read a password file.

**Acceptance:**
- A first install with no --admin-email creates no administrator, prints the setup address, and the setup route answers the setup screen.
- The setup route refuses by name once an administrator exists, and with a wrong or used code.
- Completing setup creates exactly one administrator with the entered email and leaves the person signed in to Lys.
- A test sets git user.email and every EMAIL-like environment variable to a marker and proves the marker appears nowhere in state, deployment.toml or the issuer.

**Files:**
- modify: crates/lys/src/identity/cli.rs
- modify: crates/lys/src/identity/install.rs
- modify: crates/lys/src/identity/install/layout.rs
- modify: crates/lys-identity-server/src/setup.rs
- modify: crates/lys-identity-server/src/routes.rs
- modify: surface/identity/src/features/setup/Setup.tsx

**Checklist:**
- C355 — First run asks the person for the administrator's name, email and password on a Lys setup page; install fills nothing from the machine and has no default administrator (DIRECTORY-047 R1).

**Stories:**
- S149 (Person setting up Lys for the first time, Installs Lys on their own machine with no terminal knowledge and signs in) — As an ordinary person setting up Lys, I want one installer, then a Lys page that asks my name, email and password and lets me connect Google, GitHub or Microsoft, so that I am signed in without a terminal, a password file or any page that is not Lys.

### R2: Password sign-in is a Lys page; the browser never reaches the issuer's pages

Behavioural. GET /login (crates/lys-identity-server/src/routes.rs:319) today redirects to the issuer's authorize page. It serves Lys's sign-in screen instead: email, password and one button per enabled provider. The password form posts to the service, which runs the issuer's authorization server-side for the lys-platform client (including its proof of work and PKCE), completes the code exchange as /callback does today, and sets the session. A refusal is worded by Lys: wrong email or password is one message that does not say which. Provider buttons start the provider flow through the service and return through /callback; a person passes through the provider's own consent page only. The sign-in, first-run setup and account screens are Lys screens in every visible respect: built in surface/identity from the same design tokens, forms.css inputs, selects and buttons, typography, spacing and layout shell as the other screens, with Lys's name and mark; no issuer template, stylesheet, font, colour or default browser control appears on any page a person sees.

**Acceptance:**
- An end-to-end test signs in with email and password and records every URL the browser is sent to: every one is on Lys's origin.
- A wrong password and an unknown email give the same Lys refusal, and neither names the issuer.
- A provider sign-in's recorded URLs are Lys's origin and the provider's, never the issuer's.
- A surface test renders the sign-in and setup screens and finds only the shared form, button and layout classes and no unstyled default control; an end-to-end check loads the served sign-in page and finds no stylesheet or asset served from the issuer.

**Files:**
- create: crates/lys-identity-server/src/sign_in.rs
- create: surface/identity/src/features/sign-in/SignIn.tsx
- modify: crates/lys-identity-server/src/routes.rs

**Checklist:**
- C356 — Password sign-in happens on a Lys page at Lys's origin; the browser never reaches the issuer's pages (DIRECTORY-047 R2).

**Stories:**
- S149 (Person setting up Lys for the first time, Installs Lys on their own machine with no terminal knowledge and signs in) — As an ordinary person setting up Lys, I want one installer, then a Lys page that asks my name, email and password and lets me connect Google, GitHub or Microsoft, so that I am signed in without a terminal, a password file or any page that is not Lys.
- S150 (Person setting up Lys for the first time, Installs Lys on their own machine with no terminal knowledge and signs in) — As someone using any of our products, I want to sign in with Lys everywhere, so that there is one account and one sign-in page, and it is always Lys's.

### R3: Lys is the OpenID provider every product is registered with

Behavioural. Today crates/lys-identity-server/src/oidc.rs makes Lys a client of the issuer, and install registers Cambium as the issuer's client. Add the provider side: lys-identity-server serves /.well-known/openid-configuration, authorize, token, userinfo and jwks at its own origin under Lys's issuer URL, completing sign-in through R2's screens and signing ID tokens with a key the install keeps in the state folder. No product is registered by install and no product is named in Lys: a product registers itself as a client of Lys through the app registration API (DIRECTORY-048), and the configuration a product receives names only Lys's address. Until DIRECTORY-048 lands, acceptance uses a test fixture client. The issuer's ports are published only on the container network, not on the host.

**Acceptance:**
- a fixture product configured from install's output signs in with every URL on Lys's origin or a provider's.
- Discovery at Lys's origin answers Lys's issuer, and a token Lys issues verifies against Lys's jwks.
- No port of the issuer answers from the host after install.

**Files:**
- create: crates/lys-identity-server/src/provider.rs
- modify: crates/lys-identity-server/src/oidc.rs
- modify: crates/lys/src/identity/configure.rs
- modify: deploy/identity/compose.yaml

**Checklist:**
- C357 — Every product, Cambium first, is a client of Lys at Lys's origin; no product configuration names the issuer (DIRECTORY-047 R3).

**Stories:**
- S150 (Person setting up Lys for the first time, Installs Lys on their own machine with no terminal knowledge and signs in) — As someone using any of our products, I want to sign in with Lys everywhere, so that there is one account and one sign-in page, and it is always Lys's.

### R4: Providers are set up inside Lys with every step shown

Behavioural. surface/identity/src/features/connections/SignInProviders.tsx: for Google, GitHub and Microsoft the screen shows the exact redirect address to paste (with a copy button), a link to the provider's page that creates a client, then the client id, secret and (Microsoft) tenant fields. Saving sets the provider through the service as today and then proves it by fetching the provider's discovery or authorize endpoint with the new client id; a failure names what the provider said.

**Acceptance:**
- The redirect address shown equals the one the service sends to the provider.
- Saving a provider with a client id the provider rejects is refused with the provider's words.
- A saved provider appears as a button on R2's sign-in screen.

**Files:**
- modify: surface/identity/src/features/connections/SignInProviders.tsx
- modify: crates/lys-identity-server/src/sign_in_providers.rs

**Checklist:**
- C358 — Google, GitHub and Microsoft are set up inside Lys, which shows the exact address to paste and tests the provider on save (DIRECTORY-047 R4).

**Stories:**
- S149 (Person setting up Lys for the first time, Installs Lys on their own machine with no terminal knowledge and signs in) — As an ordinary person setting up Lys, I want one installer, then a Lys page that asks my name, email and password and lets me connect Google, GitHub or Microsoft, so that I am signed in without a terminal, a password file or any page that is not Lys.

### R5: A person's own account, and administration of accounts, are Lys screens

Behavioural. A You-screen section changes the signed-in person's email (confirmed by re-entering their password) and password; an administrator's screen changes any person's email, resets a password, and enables or disables a person. Each is carried out by the service against the issuer. The issuer's admin site is not reachable from a browser on the host.

**Acceptance:**
- Changing one's email on the You screen changes it in the issuer, and the person signs in with the new email.
- An administrator's reset lets the person sign in with the new password and refuses the old one.
- A browser request from the host to the issuer's admin path fails to connect.

**Files:**
- create: surface/identity/src/features/people/Account.tsx
- create: crates/lys-identity-server/src/accounts.rs
- modify: surface/identity/src/features/me/You.tsx

**Checklist:**
- C359 — A person changes their own email and password, and an administrator changes anyone's, on Lys screens; the issuer's admin site is not reachable from the host (DIRECTORY-047 R5).

**Stories:**
- S149 (Person setting up Lys for the first time, Installs Lys on their own machine with no terminal knowledge and signs in) — As an ordinary person setting up Lys, I want one installer, then a Lys page that asks my name, email and password and lets me connect Google, GitHub or Microsoft, so that I am signed in without a terminal, a password file or any page that is not Lys.

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

## Boundaries

- SHALL NOT fill any field about a person from git config, environment variables or any machine default.
- SHALL NOT send a person's browser to the issuer's pages on any path.
- SHALL NOT add a timeout, deadline, sleep, poll interval, #[allow], #[ignore] or any bypass.
- SHALL NOT print or log a password, client secret, API key or setup code.

## Verification

- The full Lys gate and the surface checks exit 0 at the card's head, measured by the card round.
- R2 and R6's end-to-end tests run a real install on a scratch estate and pass.
