---
type: brief
id: DIRECTORY-054
cluster: directory
title: Install Lys by downloading and opening an app, with no terminal
---

# DIRECTORY-054: Install Lys by downloading and opening an app, with no terminal

> **Cluster:** directory
> **Depends on:** DIRECTORY-045, DIRECTORY-047, DIRECTORY-050, DIRECTORY-052, DIRECTORY-053, DIRECTORY-057
> **Design anchor:**
> - ADR-120 — Lys is installed by opening an app: a person downloads Lys, opens it, and finishes in the browser, never in a terminal — Lys ships for macOS as a signed, notarised Lys.app in a disk image. Opening it runs the same install service as `lys identity install`, in the process, showing each step on a Lys page in the browser, and hands over to the first-run setup page (DIRECTORY-047). Lys then starts at login, opening the app again opens Lys, a newer app upgrades through DIRECTORY-045 with its way back, and uninstalling is a Lys screen. Nothing on the path asks for a terminal, a file edit, a port or a password file. Whether the container engine is removed (running the issuer, the permission service and the database as native processes shipped in the app) is put to Tom separately; until he decides, a missing or stopped engine is a Lys page that says what to do in plain words and continues by itself when the engine appears.
> **Checklist:**
> - C395 — `lys package app` builds a signed, notarised Lys.app and disk image holding every Lys binary and the screens package, each stamped with its build; with no signing identity it is refused by name (DIRECTORY-054 R1).
> - C396 — Opening Lys.app runs the install service in the process, shows each step on a Lys page in the browser in plain words, and hands over to first-run setup (DIRECTORY-054 R2).
> - C397 — A missing or stopped container engine is a Lys page with what to do, and the install continues by itself when the engine appears, on its socket's event (DIRECTORY-054 R3).
> - C398 — Lys starts at login, opening the app again opens Lys, a newer app upgrades through DIRECTORY-045, and uninstalling is a Lys screen that keeps data unless the person chooses otherwise (DIRECTORY-054 R4).
> - C399 — On a fresh macOS account, a person goes from the downloaded disk image to signed in with no terminal process started and nothing naming the issuer (DIRECTORY-054 R5).
> **Stories:**
> - S160 (Person setting up Lys for the first time, Installs Lys on their own machine with no terminal knowledge and signs in) — As someone who has never used a terminal, I want to download Lys, open it and be guided in my browser until I am signed in, so that I can set it up for my team myself.

## Purpose

Tom, 28 September 2026 about 19:10: Lys is the only sign-in for everything, and "how is an average person supposed to do it?" Today the install is a terminal command that needs a container engine already running, and nothing packages Lys as something a person downloads. Amended 28 September 2026 21:05 by Waffles after Archie's read against the tree. Amended 29 September 2026 by Chippy after the final build 0972e9c2 ended incomplete: remove contradictory packaging and login-item words, state exact prospective file walls, and preserve the release and live proof obligations. This revision does not approve earlier out-of-wall edits retrospectively.

## Task

Add Lys.app for macOS: a packaging command that builds a signed, notarised app and disk image; an app that runs the install in its own process with progress on a Lys page and hands over to first-run setup; guidance for the container engine; start at login, reopen, upgrade and uninstall; and a proof on a fresh macOS account that no terminal is ever needed.

## Requirements

### R1: Extract the install library without changing its behaviour

Structural. Move the install, configuration, credentials, health, exit watch, surface placement and upgrade implementation from the lys command into crates/lys-install. Both the existing CLI and the app use this library. Move each existing test with its implementation. Only module paths and the imports required by the move change in those tests. Preserve assertions, test names, inputs and operation order. The CLI stays a thin adapter with the same arguments, output and named refusals. The DIRECTORY-053 rollback and DIRECTORY-057 service-owned exit-lock changes are prerequisites and move with their tests. If either landed prerequisite adds a file outside this exact manifest, stop and obtain a reviewed file-wall revision before moving it. This requirement does not fix a timing assertion, add a wait or change exit-lock ownership. A source comparison of moved test bodies and a counted execution of those tests prove the move.

**Acceptance:**
- Every existing install test passes in its new home with only its module paths and imports changed.
- A comparison of each moved test body against the prerequisite tree shows no changed assertion, input or operation order.
- The test inventory records the old path, new path and test name for every moved test, and the executed count equals that inventory.
- The existing CLI install, upgrade and rollback tests pass through the library adapter with their existing expected output and refusals.

**Files:**
- create: crates/lys-install/Cargo.toml
- create: crates/lys-install/src/config.rs
- create: crates/lys-install/src/config/validate.rs
- create: crates/lys-install/src/config_tests.rs
- create: crates/lys-install/src/configure.rs
- create: crates/lys-install/src/configure_tests.rs
- create: crates/lys-install/src/credentials.rs
- create: crates/lys-install/src/credentials_tests.rs
- create: crates/lys-install/src/error.rs
- create: crates/lys-install/src/error_tests.rs
- create: crates/lys-install/src/health.rs
- create: crates/lys-install/src/install.rs
- create: crates/lys-install/src/install/deployment.template.toml
- create: crates/lys-install/src/install/detached.rs
- create: crates/lys-install/src/install/exit_wait.rs
- create: crates/lys-install/src/install/exit_wait_tests.rs
- create: crates/lys-install/src/install/held_start.rs
- create: crates/lys-install/src/install/held_start_tests.rs
- create: crates/lys-install/src/install/layout.rs
- create: crates/lys-install/src/install/server_config.rs
- create: crates/lys-install/src/install/services.rs
- create: crates/lys-install/src/install/start_race_tests.rs
- create: crates/lys-install/src/install/surface.rs
- create: crates/lys-install/src/install_tests.rs
- create: crates/lys-install/src/lib.rs
- create: crates/lys-install/src/loopback_http.rs
- create: crates/lys-install/src/loopback_http_tests.rs
- create: crates/lys-install/src/output.rs
- create: crates/lys-install/src/output_tests.rs
- create: crates/lys-install/src/prepare.rs
- create: crates/lys-install/src/prepare_tests.rs
- create: crates/lys-install/src/private_files.rs
- create: crates/lys-install/src/rauthy.rs
- create: crates/lys-install/src/themes.rs
- create: crates/lys-install/src/themes_tests.rs
- create: crates/lys-install/src/upgrade.rs
- create: crates/lys-install/src/upgrade/back.rs
- create: crates/lys-install/src/upgrade/exchange.rs
- create: crates/lys-install/src/upgrade/exchange_tests.rs
- create: crates/lys-install/src/upgrade_back_tests.rs
- create: crates/lys-install/src/upgrade_tests.rs
- modify: Cargo.lock
- modify: Cargo.toml
- modify: crates/lys/Cargo.toml
- modify: crates/lys/src/commands/mod.rs
- modify: crates/lys/src/identity/install.rs
- modify: crates/lys/src/identity/mod.rs
- modify: crates/lys/src/main.rs
- modify: deploy/identity/theme-map.md
- delete: crates/lys/src/commands/output.rs
- delete: crates/lys/src/commands/output_tests.rs
- delete: crates/lys/src/identity/config.rs
- delete: crates/lys/src/identity/config/validate.rs
- delete: crates/lys/src/identity/config_tests.rs
- delete: crates/lys/src/identity/configure.rs
- delete: crates/lys/src/identity/configure_tests.rs
- delete: crates/lys/src/identity/credentials.rs
- delete: crates/lys/src/identity/credentials_tests.rs
- delete: crates/lys/src/identity/error.rs
- delete: crates/lys/src/identity/error_tests.rs
- delete: crates/lys/src/identity/health.rs
- delete: crates/lys/src/identity/install/deployment.template.toml
- delete: crates/lys/src/identity/install/detached.rs
- delete: crates/lys/src/identity/install/exit_wait.rs
- delete: crates/lys/src/identity/install/exit_wait_tests.rs
- delete: crates/lys/src/identity/install/held_start.rs
- delete: crates/lys/src/identity/install/held_start_tests.rs
- delete: crates/lys/src/identity/install/layout.rs
- delete: crates/lys/src/identity/install/server_config.rs
- delete: crates/lys/src/identity/install/services.rs
- delete: crates/lys/src/identity/install/start_race_tests.rs
- delete: crates/lys/src/identity/install/surface.rs
- delete: crates/lys/src/identity/install_tests.rs
- delete: crates/lys/src/identity/loopback_http.rs
- delete: crates/lys/src/identity/loopback_http_tests.rs
- delete: crates/lys/src/identity/prepare.rs
- delete: crates/lys/src/identity/prepare_tests.rs
- delete: crates/lys/src/identity/private_files.rs
- delete: crates/lys/src/identity/rauthy.rs
- delete: crates/lys/src/identity/themes.rs
- delete: crates/lys/src/identity/themes_tests.rs
- delete: crates/lys/src/identity/upgrade.rs
- delete: crates/lys/src/identity/upgrade/back.rs
- delete: crates/lys/src/identity/upgrade/exchange.rs
- delete: crates/lys/src/identity/upgrade/exchange_tests.rs
- delete: crates/lys/src/identity/upgrade_back_tests.rs
- delete: crates/lys/src/identity/upgrade_tests.rs

**Checklist:**
- C396 — Opening Lys.app runs the install service in the process, shows each step on a Lys page in the browser in plain words, and hands over to first-run setup (DIRECTORY-054 R2).

**Stories:**
- S160 (Person setting up Lys for the first time, Installs Lys on their own machine with no terminal knowledge and signs in) — As someone who has never used a terminal, I want to download Lys, open it and be guided in my browser until I am signed in, so that I can set it up for my team myself.

### R2: Opening the app installs and shows every step on a Lys page

Behavioural. Lys-app, when no install exists under the platform data root, serves a Lys-styled progress page on a loopback port it chooses, opens the default browser to it, and runs the install service as a library call (the same code as `lys identity install`, never a spawned shell or terminal). Each step appears as it happens in plain words ("Preparing your directory", "Starting sign-in"), with a failure shown in the same words and the one next thing to do. When the install is ready it hands the browser to the first-run setup page of DIRECTORY-047. Opening the app while an install is running shows that install's page, never a second install. The install library from R1 is the only install implementation used by lys and lys-app. An app opened from the disk image (a read-only, translocated path) refuses to install and says in plain words to move Lys to Applications first. The install's bin/ (DIRECTORY-045) keeps the running install independent of where the app sits. A launcher integration test holds the install at a named step barrier, opens a second launcher against the same scratch data root, and observes one install invocation and the same progress endpoint. It then releases the barrier. This tests the launcher entry path, not only its lock helper. Read failure for an existing screens file is a named I/O refusal, not not_found. The library extraction preserves every existing install, upgrade and rollback test and configuration value. Moved tests remain executable. No app publication is permitted while DIRECTORY-047 setup is absent from the candidate.

**Acceptance:**
- Opening the app on a scratch install reaches the DIRECTORY-047 first-run setup page.
- The recorded process tree for that opening contains no terminal started by Lys.
- A failure at any install step shows that failure in plain words on the progress page.
- The failed step presents its one next action.
- The install can resume from the preserved state after that action.
- In the scratch launcher integration test, a second open while the first install is held at its step barrier joins the same progress endpoint.
- The second open starts no second install while that barrier holds.
- The first install completes when the test releases the step barrier.
- An existing but unreadable screens file produces its named I/O refusal.
- An absent screens file still produces not_found.

**Files:**
- create: crates/lys-app/Cargo.toml
- create: crates/lys-app/build.rs
- create: crates/lys-app/src/bundle.rs
- create: crates/lys-app/src/bundle_tests.rs
- create: crates/lys-app/src/flow.rs
- create: crates/lys-app/src/flow_tests.rs
- create: crates/lys-app/src/launcher.rs
- create: crates/lys-app/src/launcher_tests.rs
- create: crates/lys-app/src/main.rs
- create: crates/lys-app/src/main_tests.rs
- create: crates/lys-app/src/progress.rs
- create: crates/lys-app/src/progress_tests.rs
- create: crates/lys-app/src/refusal.rs
- create: crates/lys-app/src/server.rs
- create: crates/lys-app/src/server_tests.rs
- create: crates/lys-install/src/steps.rs
- create: crates/lys-install/src/steps_tests.rs
- create: surface/identity/src/features/install/InstallProgress.tsx
- create: surface/identity/tests/install-progress.test.tsx
- modify: Cargo.lock
- modify: Cargo.toml
- modify: crates/lys-install/src/install.rs
- modify: crates/lys-install/src/lib.rs
- modify: crates/lys-install/src/upgrade.rs
- modify: surface/identity/src/App.tsx

**Checklist:**
- C396 — Opening Lys.app runs the install service in the process, shows each step on a Lys page in the browser in plain words, and hands over to first-run setup (DIRECTORY-054 R2).

**Stories:**
- S160 (Person setting up Lys for the first time, Installs Lys on their own machine with no terminal knowledge and signs in) — As someone who has never used a terminal, I want to download Lys, open it and be guided in my browser until I am signed in, so that I can set it up for my team myself.

### R3: The container engine, guided in plain words

Behavioural. Before the install starts its services, a container engine that is missing or not running is shown on the progress page in plain words: what it is for, the official download for this Mac, and that the install continues by itself. The app waits on the engine's socket appearing and answering (a filesystem event on its known socket paths), never on a clock or a poll, and continues the install when it does. No page names the issuer.

**Acceptance:**
- With the engine stopped, the page shows the guidance.
- Starting the engine continues the install with no further action in Lys.
- No wait in lys-app reads a clock or loops on a sleep.

**Files:**
- create: crates/lys-app/src/engine.rs
- create: crates/lys-app/src/engine_tests.rs
- create: crates/lys-app/src/engine_wait.rs
- create: crates/lys-app/src/engine_wait_tests.rs
- create: crates/lys-install/src/install/engine_path.rs
- create: crates/lys-install/src/install/engine_path_tests.rs
- modify: crates/lys-app/src/flow.rs
- modify: crates/lys-app/src/main.rs
- modify: crates/lys-install/src/install.rs
- modify: crates/lys-install/src/install/services.rs

**Checklist:**
- C397 — A missing or stopped container engine is a Lys page with what to do, and the install continues by itself when the engine appears, on its socket's event (DIRECTORY-054 R3).

**Stories:**
- S160 (Person setting up Lys for the first time, Installs Lys on their own machine with no terminal knowledge and signs in) — As someone who has never used a terminal, I want to download Lys, open it and be guided in my browser until I am signed in, so that I can set it up for my team myself.

### R4: Start at login, reopen, upgrade and uninstall

Behavioural. After a successful install the app registers Lys to start at login for this user through SMAppService, removed on uninstall. Opening the app when Lys is installed and running opens Lys's screens. When installed and stopped it starts it and then opens them. An app whose bundled build is newer than the install's runs the upgrade of DIRECTORY-045 with its progress on the page and its way back (DIRECTORY-053). An Uninstall control on the account screen of an administrator stops Lys, removes the login registration and the binaries, and keeps the data folder unless the person ticks to remove it after a confirmation that names what is lost. Start at login uses the system's login item service (SMAppService), so its definition stays inside the app bundle and nothing is written into the person's home outside Lys's own data path. The item runs `lys-app --at-login`, which starts the install's units and waits for the container engine as R3 does. The proof of this on Tom's own account waits for his word on this choice. Uninstall is carried out by a helper, `lys-app --uninstall`, started detached by the service before it answers, which outlives the service it stops and removes the binaries, the login item and, if ticked, the data folder, and records the result where the app shows it on next open. The server and the DIRECTORY-050 runner are started through that same login registration. A second home-directory LaunchAgent is not an allowed substitute in development or release mode. Tests use an explicit login-service double and measure registration, refused registration, unregister and repeated opens. No real login registration or restart on Dean's personal account is part of a card round.

**Acceptance:**
- After a restart of the Mac, Lys is answering without the person doing anything.
- Opening a newer app upgrades the install and the Build line shows the new commit.
- Uninstall without the tick leaves the data folder, and a reinstall signs the same people in.
- The login-service double records one SMAppService registration for repeated successful opens.
- A refused registration is shown by name.
- A refused registration is never recorded as successful.
- The login registration starts the installed server through the app entry point.
- The same login registration starts the DIRECTORY-050 runner through that entry point.
- Uninstall unregisters that entry point.
- No home-directory LaunchAgents file is written by an install or uninstall test.

**Files:**
- create: crates/lys-app/src/login_item.rs
- create: crates/lys-app/src/login_item_tests.rs
- create: crates/lys-app/src/uninstall.rs
- create: crates/lys-app/src/uninstall_tests.rs
- create: crates/lys-identity-server/src/uninstall_api.rs
- create: crates/lys-identity-server/src/uninstall_api_tests.rs
- create: surface/identity/src/features/account/Uninstall.tsx
- create: surface/identity/tests/uninstall.test.tsx
- modify: crates/lys-app/src/flow.rs
- modify: crates/lys-app/src/main.rs
- modify: crates/lys-app/src/refusal.rs
- modify: crates/lys-identity-server/src/error.rs
- modify: crates/lys-identity-server/src/error_status.rs
- modify: crates/lys-identity-server/src/lib.rs
- modify: crates/lys-identity-server/src/routes.rs
- modify: surface/identity/src/features/me/You.tsx
- modify: surface/identity/tests/fixtures.ts

**Checklist:**
- C398 — Lys starts at login, opening the app again opens Lys, a newer app upgrades through DIRECTORY-045, and uninstalling is a Lys screen that keeps data unless the person chooses otherwise (DIRECTORY-054 R4).

**Stories:**
- S160 (Person setting up Lys for the first time, Installs Lys on their own machine with no terminal knowledge and signs in) — As someone who has never used a terminal, I want to download Lys, open it and be guided in my browser until I am signed in, so that I can set it up for my team myself.

### R5: `lys package app` builds the app and disk image

Behavioural. `lys package app --out DIR` builds Lys.app (bundle id, Info.plist, icon) holding lys, lys-identity-server, lys-secrets and lys-home built for both arm64 and x86_64 from the same commit, the screens package, and lys-app as the bundle's executable, and a Lys.dmg with the app and an Applications link. Each binary's --version must name the same commit, or the packaging is refused naming the one that differs. It signs with the Developer ID named by a secret handle (DIRECTORY-052 R6, SECRETS), with the hardened runtime, and submits for notarisation, stapling the ticket. With no signing identity or a refused notarisation it fails naming which, and never produces an unsigned app under the release name. The app is one universal build holding arm64 and x86_64 binaries (lipo), so one download serves Apple and Intel processors. The release mode is the default. Only an explicit --development request builds a locally signed app under the name 'Lys (development)', never Lys.app or Lys.dmg. R2 to R4 are built and tested against that. Development mode is still universal, checks the build stamp on every binary, and verifies the local signature. It does not claim Developer ID signing, notarisation or Gatekeeper approval. Release mode with a missing identity refuses signing_identity_missing before a release artifact is published. Signing authority is resolved by the declared secret handle. A caller-supplied display name is not authority to use a signing credential. Release signing and the fresh-account proof need the declared Developer ID authority. Its absence never turns a development build into release evidence. R5 runs after the library, app, engine guidance and login integration of R1 to R4 exist. Its unit tests use tool doubles. Its real packaging acceptance uses those built executables, not substitute binaries. The build-stamp changes in this row are restricted to making every included executable report the same source commit.

**Acceptance:**
- The release app passes codesign --verify --deep --strict.
- The release app passes spctl --assess.
- The release record names the source commit, artifact hash and notarisation result.
- Binaries built from two commits are refused naming the binary before a release artifact is published.
- Release mode with no signing identity answers signing_identity_missing.
- That refused release leaves neither Lys.app nor Lys.dmg published.
- An explicit --development request with no Developer ID builds only Lys (development).
- The development app local signature passes codesign --verify --deep --strict.
- The development build claims neither notarisation nor Gatekeeper approval.
- lipo -archs on every bundled binary in each packaging mode names arm64 and x86_64.
- A signing-provider double records resolution of the declared secret handle.
- An unknown signing handle is refused by name.
- An inaccessible signing handle is refused by name.
- Credential material appears in neither captured command output nor failure text.

**Files:**
- create: crates/lys/src/package.rs
- create: crates/lys/src/package_tests.rs
- create: packaging/macos/Info.plist
- create: packaging/macos/Lys.icns
- modify: Cargo.lock
- modify: crates/lys-app/build.rs
- modify: crates/lys-home/build.rs
- modify: crates/lys-home/tests/version.rs
- modify: crates/lys-identity-server/build.rs
- modify: crates/lys-identity-server/tests/version.rs
- modify: crates/lys-secrets/build.rs
- modify: crates/lys-secrets/tests/version.rs
- modify: crates/lys/Cargo.toml
- modify: crates/lys/build.rs
- modify: crates/lys/src/cli.rs
- modify: crates/lys/src/commands/error.rs
- modify: crates/lys/src/commands/error_tests.rs
- modify: crates/lys/src/main.rs
- modify: crates/lys/tests/version.rs

**Checklist:**
- C395 — `lys package app` builds a signed, notarised Lys.app and disk image holding every Lys binary and the screens package, each stamped with its build; with no signing identity it is refused by name (DIRECTORY-054 R1).

**Stories:**
- S160 (Person setting up Lys for the first time, Installs Lys on their own machine with no terminal knowledge and signs in) — As someone who has never used a terminal, I want to download Lys, open it and be guided in my browser until I am signed in, so that I can set it up for my team myself.

### R6: Proof on a fresh macOS account

Behavioural. On a fresh macOS user account on a machine that has never held Lys, a person downloads Lys.dmg, drags Lys to Applications, opens it, completes first-run setup and signs in. The run is recorded (screen recording and the process tree), and the record shows no Terminal or shell process started by the person or by Lys on their behalf and no page naming the issuer. The fresh account is a macOS virtual machine on Tom's Mac made for the proof, never a new account on Dean's laptop (it belongs to Tom's father). Making it waits for Tom's word. The implementation round preserves this as an unperformed proof until the real evidence exists: unit tests, mocked signing and a status line saying not run do not satisfy it. The delivery lead records release packaging, first open, restart, upgrade and retained-data reinstall evidence against the landed build in the same report. Code completion, landing, installed and release-ready are reported separately. No release distribution or claim of finished installer precedes these proofs.

**Acceptance:**
- The recorded fresh-account run reaches signed in from the disk image.
- The process record holds no terminal started for the person.
- A text scan of every page shown finds no issuer name.

**Files:**
- create: docs/design/directory/reports/DIRECTORY-054-fresh-account.md
- create: docs/design/directory/reports/DIRECTORY-054-open-items.md

**Checklist:**
- C399 — On a fresh macOS account, a person goes from the downloaded disk image to signed in with no terminal process started and nothing naming the issuer (DIRECTORY-054 R5).

**Stories:**
- S160 (Person setting up Lys for the first time, Installs Lys on their own machine with no terminal knowledge and signs in) — As someone who has never used a terminal, I want to download Lys, open it and be guided in my browser until I am signed in, so that I can set it up for my team myself.

## Boundaries

- SHALL NOT start a terminal or ask the person to type a command, edit a file, choose a port or read a password file at any step.
- SHALL NOT produce an unsigned app under the release name.
- SHALL NOT name the issuer on any page.
- SHALL NOT add a timeout, sleep, poll, #[allow], #[ignore] or any bypass.
- SHALL NOT add a silent fallback: every failure is a named refusal shown in plain words.
- SHALL NOT treat this prospective file-wall revision as evidence that prior out-of-wall edits were approved or that any runtime, signing or installation proof was performed.

## Verification

- The full Lys gate, ast-grep scan and surface checks exit 0 at the card's head, measured by the card round.
- R6's recorded run on a fresh macOS account.
