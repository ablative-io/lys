---
type: brief
id: DIRECTORY-054
cluster: directory
title: Install Lys by downloading and opening an app, with no terminal
---

# DIRECTORY-054: Install Lys by downloading and opening an app, with no terminal

> **Cluster:** directory
> **Depends on:** DIRECTORY-045, DIRECTORY-047
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

Tom, 28 September 2026 about 19:10: Lys is the only sign-in for everything, and "how is an average person supposed to do it?" Today the install is a terminal command that needs a container engine already running, and nothing packages Lys as something a person downloads. Amended 28 September 2026 21:05 by Waffles after Archie's read against the tree.

## Task

Add Lys.app for macOS: a packaging command that builds a signed, notarised app and disk image; an app that runs the install in its own process with progress on a Lys page and hands over to first-run setup; guidance for the container engine; start at login, reopen, upgrade and uninstall; and a proof on a fresh macOS account that no terminal is ever needed.

## Requirements

### R1: `lys package app` builds the app and disk image

Behavioural. `lys package app --out DIR` builds Lys.app (bundle id, Info.plist, icon) holding lys, lys-identity-server, lys-secrets and lys-home built for the host architecture, the screens package, and lys-app as the bundle's executable, and a Lys.dmg with the app and an Applications link. Each binary's --version must name the same commit, or the packaging is refused naming the one that differs. It signs with the Developer ID named by a secret handle (DIRECTORY-052 R6, SECRETS), with the hardened runtime, and submits for notarisation, stapling the ticket; with no signing identity or a refused notarisation it fails naming which, and never produces an unsigned app under the release name. The app is one universal build holding arm64 and x86_64 binaries (lipo), so one download serves Apple and Intel processors. Without the Developer ID the command builds a locally signed app under the name 'Lys (development)', never the release name; R2 to R4 are built and tested against that. Only R1's notarisation acceptance and R5 wait for Tom's Developer ID.

**Acceptance:**
- The built app passes codesign --verify --deep --strict and spctl --assess.
- Binaries built from two commits are refused naming the binary.
- With no signing identity it is refused signing_identity_missing and no Lys.dmg is written.
- lipo -archs on each bundled binary names arm64 and x86_64.

**Files:**
- create: crates/lys/src/package.rs
- create: crates/lys/src/package_tests.rs
- create: packaging/macos/Info.plist
- create: packaging/macos/Lys.icns
- modify: crates/lys/src/main.rs

**Checklist:**
- C395 — `lys package app` builds a signed, notarised Lys.app and disk image holding every Lys binary and the screens package, each stamped with its build; with no signing identity it is refused by name (DIRECTORY-054 R1).

**Stories:**
- S160 (Person setting up Lys for the first time, Installs Lys on their own machine with no terminal knowledge and signs in) — As someone who has never used a terminal, I want to download Lys, open it and be guided in my browser until I am signed in, so that I can set it up for my team myself.

#### R1 — Execution record

**Dev (recorded):**

- Status: blocked
- How: lys package app puts together Lys.app from arm64 and x86_64 builds of one commit, refusing binaries from two commits by name. It lipo-merges each binary and checks lipo -archs, then signs with the hardened runtime and runs codesign --verify --deep --strict. A release also goes through notarytool --wait and stapler, then spctl --assess on the app and on the dmg. Staging is used throughout, and nothing is placed until every step has passed. A --development build never carries the release name.
- Deviation: cli.rs and commands/error.rs lie outside the manifest, and so does the shared build.rs restamp. The open-items report asks for a CN9 revision naming each of them. CN1 ('documents only') contradicts the requirement's code files; the build followed the requirement, and a revision is asked for.
- Files changed:
  - modified: `crates/lys/src/package.rs` — lys package app. Makes the binaries universal with lipo and refuses universal_incomplete unless both architectures are held. Codesigns with the hardened runtime and verifies --deep --strict. A release is notarised and stapled, and assessed with spctl, first the app and then the codesigned disk image. The image is placed only after the app. A release without its identity or notary profile is refused by name.
  - modified: `crates/lys/src/package_tests.rs` — 8 tests, including a_release_without_its_signing_identity_writes_no_disk_image: run() with a release Signing is refused signing_identity_missing, and no Lys.dmg or output folder is written
  - modified: `crates/lys/src/cli.rs` — the package app subcommand (outside the manifest; a CN9 revision is asked for in the open-items report)
  - modified: `crates/lys/src/commands/error.rs` — the package refusals (outside the manifest; CN9)
  - created: `packaging/macos/Info.plist` — the bundle property list, with the build stamp
  - created: `packaging/macos/Lys.icns` — the app icon
  - created: `docs/design/directory/reports/DIRECTORY-054-open-items.md` — records the development and release package runs still owed and the CN9 files
- Checklist delivery:
  - [ ] C395 — `lys package app` builds a signed, notarised Lys.app and disk image holding every Lys binary and the screens package, each stamped with its build; with no signing identity it is refused by name (DIRECTORY-054 R1). — The code, and its tests (8/8), are in place. Not yet recorded: lipo -archs and codesign --verify on a real --development package. That run needs rustup target add x86_64-apple-darwin and a heavy two-architecture build, which goes to Dean's laptop under rule 3. A release build needs Tom's Developer ID and a notary profile; this Mac's security find-identity reports 0 valid identities.
- Story delivery:
  - [ ] S160 (Person setting up Lys for the first time, Installs Lys on their own machine with no terminal knowledge and signs in) — As someone who has never used a terminal, I want to download Lys, open it and be guided in my browser until I am signed in, so that I can set it up for my team myself. — Not satisfied until a signed, notarised Lys.dmg exists

**Review (recorded):**

- Alignment: drifted
- Acceptance verdicts:
  - [ ] The built app passes codesign --verify --deep --strict and spctl --assess. — crates/lys/src/package.rs sign() runs codesign --verify --deep --strict (the 'verify' array before notarise_as) and notarise() runs spctl --assess --type execute; notarise_image() assesses the dmg with --type open. No built app exists: security find-identity -v -p codesigning reports '0 valid identities found', and no development package has been run either.
  - [x] Binaries built from two commits are refused naming the binary. — package_tests.rs binaries_from_two_commits_are_refused_naming_the_binary and packaging_binaries_from_two_commits_writes_nothing (detail starts 'x86_64/lys-app is built from', no output folder). Re-run here: ok.
  - [x] With no signing identity it is refused signing_identity_missing and no Lys.dmg is written. — package_tests.rs with_no_signing_identity_a_release_is_refused_by_name covers from_flags. The new a_release_without_its_signing_identity_writes_no_disk_image drives run() with a Developer ID absent from the keychain and asserts signing_identity_missing, !out/Lys.dmg and !out. cargo test -p lys --bin lys package: 8 passed.
  - [ ] lipo -archs on each bundled binary names arm64 and x86_64. — package.rs universal() runs lipo -create, then lipo -archs, and refuses universal_incomplete unless both ARCHS are held. Never run: rustup target list --installed shows only aarch64-apple-darwin and wasm32-unknown-unknown.
- Issues:
  - The signing_identity_missing test never asserted that no Lys.dmg is written: it only called Signing::from_flags.
  - Run `lys package app --development` from arm64 and x86_64 builds of one commit (rustup target add x86_64-apple-darwin; a heavy build, so on Dean's laptop per rule 3) and record lipo -archs on each bundled binary and codesign --verify --deep --strict on the app. This is not blocked on Tom.
  - Build a release with Tom's Developer ID and notary profile, then record codesign --verify --deep --strict and spctl --assess on Lys.app and on Lys.dmg.
  - cli.rs and commands/error.rs lie outside the manifest; they need a reviewed brief revision under CN9.
- Fixes:
  - Added crates/lys/src/package_tests.rs a_release_without_its_signing_identity_writes_no_disk_image: a release refused signing_identity_missing through run() leaves no Lys.dmg and no output folder.

### R2: Opening the app installs and shows every step on a Lys page

Behavioural. lys-app, when no install exists under the platform data root, serves a Lys-styled progress page on a loopback port it chooses, opens the default browser to it, and runs the install service as a library call (the same code as `lys identity install`, never a spawned shell or terminal). Each step appears as it happens in plain words ("Preparing your directory", "Starting sign-in"), with a failure shown in the same words and the one next thing to do. When the install is ready it hands the browser to the first-run setup page of DIRECTORY-047. Opening the app while an install is running shows that install's page, never a second install. The install moves out of the lys binary crate into a library crate, crates/lys-install, that both lys and lys-app depend on; lys identity install becomes a thin call into it. An app opened from the disk image (a read-only, translocated path) refuses to install and says in plain words to move Lys to Applications first; the install's bin/ (DIRECTORY-045) keeps the running install independent of where the app sits.

**Acceptance:**
- Opening the app on a machine without an install reaches the first-run setup page with no terminal started.
- A failure at any step shows its words and next action on the page and leaves the install resumable.
- Opening the app twice during an install shows one install.

**Files:**
- create: crates/lys-app/Cargo.toml
- create: crates/lys-app/src/main.rs
- create: crates/lys-app/src/progress.rs
- create: crates/lys-app/src/progress_tests.rs
- create: surface/identity/src/features/install/InstallProgress.tsx
- create: crates/lys-install/Cargo.toml
- create: crates/lys-install/src/lib.rs
- modify: Cargo.toml
- modify: crates/lys/Cargo.toml
- modify: crates/lys/src/identity/install.rs
- modify: crates/lys/src/main.rs

**Checklist:**
- C396 — Opening Lys.app runs the install service in the process, shows each step on a Lys page in the browser in plain words, and hands over to first-run setup (DIRECTORY-054 R2).

**Stories:**
- S160 (Person setting up Lys for the first time, Installs Lys on their own machine with no terminal knowledge and signs in) — As someone who has never used a terminal, I want to download Lys, open it and be guided in my browser until I am signed in, so that I can set it up for my team myself.

#### R2 — Execution record

**Dev (recorded):**

- Status: blocked
- How: Opening Lys.app runs lys-app. It installs through the lys-install library with no shell: it waits for the container engine on kernel notices, shows progress in plain words, and hands the browser to /setup. The test no_shell_or_terminal_is_ever_started checks this, and a grep of the non-test sources finds no Terminal, osascript or sh.
- Deviation: The files outside the manifest are named in the open-items report for a CN9 revision: identity/*, commands/output.rs and mod.rs, steps.rs and engine_path.rs, and lys-app's launcher, flow, server, bundle, refusal, uninstall, engine and engine_wait. Publishing decision, recorded there: lys-install stays publish = false until a release card moves its embedded deploy/identity files inside the crate and publishes it beside lys; no lys release is cut before then.
- Files changed:
  - created: `crates/lys-install/src/lib.rs` — the install moved into a library that lys and lys-app share
  - created: `crates/lys-install/src/steps.rs` — the install's steps in plain words, for the progress page
  - created: `crates/lys-install/src/install/engine_path.rs` — finds the container engine without a shell
  - created: `crates/lys-app/src/flow.rs` — the app's first-open flow: install or upgrade, then hand the browser to /setup
  - created: `crates/lys-app/src/engine_wait.rs` — waits for the engine only on kqueue notices and the engine's own /_ping answer, never a clock
  - created: `crates/lys-app/src/launcher.rs` — opens the browser, starting no shell or terminal
  - created: `docs/design/directory/reports/DIRECTORY-054-open-items.md` — records that lys-install is published beside lys by a release card, and the CN1/CN9 revisions this row asks for
- Checklist delivery:
  - [ ] C396 — Opening Lys.app runs the install service in the process, shows each step on a Lys page in the browser in plain words, and hands over to first-run setup (DIRECTORY-054 R2). — The code and its tests pass. The live run that reaches DIRECTORY-047's /setup page waits on DIRECTORY-047 landing in this tree.
- Story delivery:
  - [ ] S160 (Person setting up Lys for the first time, Installs Lys on their own machine with no terminal knowledge and signs in) — As someone who has never used a terminal, I want to download Lys, open it and be guided in my browser until I am signed in, so that I can set it up for my team myself. — Waits on the live run

**Review (recorded):**

- Alignment: drifted
- Acceptance verdicts:
  - [ ] Opening the app on a machine without an install reaches the first-run setup page with no terminal started. — flow.rs hands the browser to /setup, but DIRECTORY-047's setup page is not in this tree and no run has been made. flow_tests.rs no_shell_or_terminal_is_ever_started passes, and grep for Terminal, osascript and sh in the non-test lys-app sources finds none.
  - [x] A failure at any step shows its words and next action on the page and leaves the install resumable. — flow_tests.rs a_failure_is_shown_at_its_step_with_the_next_thing_to_do; progress_tests.rs the_page_hears_each_phase_as_it_is_shown_and_retries_only_a_failure; lys-install install.rs run() reports each Step and remakes only what is missing; surface/identity/tests/install-progress.test.tsx (surface leg passed at the venue). cargo test -p lys-app: 59 passed.
  - [x] Opening the app twice during an install shows one install. — launcher_tests.rs a_second_opening_is_shown_the_running_works_page (the single-instance claim through flock in launcher.rs); passed in the 59/59 run.
- Issues:
  - Record a live run: open a development build on a machine without an install and reach DIRECTORY-047's /setup page with no terminal. This waits on DIRECTORY-047 landing in this tree.
  - Files outside the manifest need a reviewed brief revision before they stand (CN9): crates/lys/src/identity/*, commands/output.rs, commands/mod.rs, lys-install steps.rs and engine_path.rs, and lys-app's launcher, flow, server, bundle, refusal, uninstall and engine_wait.
  - lys depends on lys-install (publish = false), so lys cannot be published until a release card publishes lys-install with its embedded deploy/identity files moved inside the crate. Record that decision.
  - CN1 ('documents only') contradicts every code file this brief's requirements name; revise the brief so its constraints and requirements agree.

### R3: The container engine, guided in plain words

Behavioural. Before the install starts its services, a container engine that is missing or not running is shown on the progress page in plain words: what it is for, the official download for this Mac, and that the install continues by itself. The app waits on the engine's socket appearing and answering (a filesystem event on its known socket paths), never on a clock or a poll, and continues the install when it does. No page names the issuer.

**Acceptance:**
- With the engine stopped, the page shows the guidance, and starting the engine continues the install with no action in Lys.
- No wait in lys-app reads a clock or loops on a sleep.

**Files:**
- create: crates/lys-app/src/engine.rs
- create: crates/lys-app/src/engine_tests.rs

**Checklist:**
- C397 — A missing or stopped container engine is a Lys page with what to do, and the install continues by itself when the engine appears, on its socket's event (DIRECTORY-054 R3).

**Stories:**
- S160 (Person setting up Lys for the first time, Installs Lys on their own machine with no terminal knowledge and signs in) — As someone who has never used a terminal, I want to download Lys, open it and be guided in my browser until I am signed in, so that I can set it up for my team myself.

#### R3 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Each install step and each refusal reaches the person as plain words on a Lys page. No port, password file or command is ever shown, and the issuer is never named.
- Deviation: (none)
- Files changed:
  - created: `crates/lys-app/src/progress.rs` — sends each step in plain words only
  - created: `crates/lys-app/src/refusal.rs` — every failure is a named refusal, in plain words
  - created: `surface/identity/src/features/install/InstallProgress.tsx` — the progress page; it never names the issuer
  - created: `surface/identity/tests/install-progress.test.tsx` — asserts the plain words, and that the issuer is never named
- Checklist delivery:
  - [x] C397 — A missing or stopped container engine is a Lys page with what to do, and the install continues by itself when the engine appears, on its socket's event (DIRECTORY-054 R3). — progress_tests and install-progress.test.tsx pass
- Story delivery:
  - [x] S160 (Person setting up Lys for the first time, Installs Lys on their own machine with no terminal knowledge and signs in) — As someone who has never used a terminal, I want to download Lys, open it and be guided in my browser until I am signed in, so that I can set it up for my team myself. — The progress and refusals are shown in plain words

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] With the engine stopped, the page shows the guidance, and starting the engine continues the install with no action in Lys. — flow.rs:164-166 shows Guidance::for_mac and then blocks in engine_wait::wait(&sockets, &engine::state_folders(..)). engine_wait_tests.rs covers the folder notice when the engine starts, a socket made before its engine answers (ended by the held /_ping's 200 with no folder change), and an engine saying 503 asked exactly twice. I checked the wait against kqueue 1.2.1: poll_forever(None) blocks in kevent, and remove_fd issues EV_DELETE before the stream drops. 59/59 passed here. Not recorded against a real Docker Desktop start.
  - [x] No wait in lys-app reads a clock or loops on a sleep. — engine_tests.rs no_wait_in_the_app_reads_a_clock_or_sleeps scans every non-test lys-app source, including engine_wait.rs. My grep for Duration, Instant, SystemTime, sleep and timeout over crates/lys-app/src non-test files found only doc prose.
- Checklist verified: C397
- Issues:
  - The Docker Desktop state folders (Data and Data/vms/0 in engine.rs state_folders) are assumed, not observed. Record one real Docker Desktop start through the wait on a machine where restarting the engine is agreed, as the developer states.

### R4: Start at login, reopen, upgrade and uninstall

Behavioural. After a successful install the app registers Lys to start at login for this user (a LaunchAgent under the user's Library, removed on uninstall). Opening the app when Lys is installed and running opens Lys's screens; when installed and stopped it starts it and then opens them. An app whose bundled build is newer than the install's runs the upgrade of DIRECTORY-045 with its progress on the page and its way back (DIRECTORY-053). An Uninstall control on the account screen of an administrator stops Lys, removes the login registration and the binaries, and keeps the data folder unless the person ticks to remove it after a confirmation that names what is lost. Start at login uses the system's login item service (SMAppService), so its definition stays inside the app bundle and nothing is written into the person's home outside Lys's own data path; the item runs `lys-app --at-login`, which starts the install's units and waits for the container engine as R3 does. The proof of this on Tom's own account waits for his word on this choice. Uninstall is carried out by a helper, `lys-app --uninstall`, started detached by the service before it answers, which outlives the service it stops and removes the binaries, the login item and, if ticked, the data folder, and records the result where the app shows it on next open.

**Acceptance:**
- After a restart of the Mac, Lys is answering without the person doing anything.
- Opening a newer app upgrades the install and the Build line shows the new commit.
- Uninstall without the tick leaves the data folder, and a reinstall signs the same people in.

**Files:**
- create: crates/lys-app/src/login_item.rs
- create: crates/lys-app/src/login_item_tests.rs
- create: surface/identity/src/features/account/Uninstall.tsx
- modify: crates/lys-app/src/main.rs

**Checklist:**
- C398 — Lys starts at login, opening the app again opens Lys, a newer app upgrades through DIRECTORY-045, and uninstalling is a Lys screen that keeps data unless the person chooses otherwise (DIRECTORY-054 R4).

**Stories:**
- S160 (Person setting up Lys for the first time, Installs Lys on their own machine with no terminal knowledge and signs in) — As someone who has never used a terminal, I want to download Lys, open it and be guided in my browser until I am signed in, so that I can set it up for my team myself.

#### R4 — Execution record

**Dev (recorded):**

- Status: blocked
- How: Start at login is a LaunchAgent. Opening a newer app compares the bundle's build with build.json and upgrades. Uninstall keeps the data folder unless the box is ticked. The restamp build.rs is byte-identical across the crates, and the version tests check it.
- Deviation: Start at login is a LaunchAgent, not SMAppService, which needs the Developer ID. Tom's ruling is asked for in the open-items report. uninstall_api, ServerError::UninstallUnavailable, error_status, routes, lib and the surface fixture are named there for a CN9 revision.
- Files changed:
  - created: `crates/lys-app/src/login_item.rs` — writes the LaunchAgent au.com.ablative.lys, which runs bin/lys-app --at-login
  - created: `crates/lys-app/src/uninstall.rs` — removes bin/; runs compose down --volumes only when the box is ticked
  - created: `crates/lys-identity-server/src/uninstall_api.rs` — the uninstall request from the account page (outside the manifest; CN9)
  - created: `surface/identity/src/features/account/Uninstall.tsx` — the uninstall page, with the tick box for the data folder
  - created: `docs/design/directory/reports/DIRECTORY-054-open-items.md` — records the SMAppService ruling asked of Tom and the live runs still owed
- Checklist delivery:
  - [ ] C398 — Lys starts at login, opening the app again opens Lys, a newer app upgrades through DIRECTORY-045, and uninstalling is a Lys screen that keeps data unless the person chooses otherwise (DIRECTORY-054 R4). — The unit tests and the surface tests pass. Still owed: live runs of a restart, an upgrade (with the Build line showing the new commit) and an uninstall without the tick followed by a reinstall. Also Tom's SMAppService ruling.
- Story delivery:
  - [ ] S160 (Person setting up Lys for the first time, Installs Lys on their own machine with no terminal knowledge and signs in) — As someone who has never used a terminal, I want to download Lys, open it and be guided in my browser until I am signed in, so that I can set it up for my team myself. — Waits on the live runs

**Review (recorded):**

- Alignment: drifted
- Acceptance verdicts:
  - [ ] After a restart of the Mac, Lys is answering without the person doing anything. — login_item.rs writes the LaunchAgent au.com.ablative.lys that runs bin/lys-app --at-login; login_item_tests.rs passes. No restart has been run.
  - [ ] Opening a newer app upgrades the install and the Build line shows the new commit. — flow.rs attempt() compares the bundle's build with build.json and calls upgrade_to; the shared build.rs restamp is byte-identical across crates (tests/version.rs). No live upgrade has been run.
  - [ ] Uninstall without the tick leaves the data folder, and a reinstall signs the same people in. — uninstall.rs runs compose down with --volumes only when ticked and otherwise removes only bin/; uninstall_tests.rs and surface uninstall.test.tsx pass. No live uninstall and reinstall has been run.
- Issues:
  - Start at login is a LaunchAgent plist, not the SMAppService the spec names. Get Tom's ruling recorded in a brief revision, or implement SMAppService.
  - Record live runs of a restart, an upgrade (the Build line showing the new commit) and an uninstall-without-tick followed by a reinstall that signs the same people in.
  - uninstall_api.rs, ServerError::UninstallUnavailable, error_status.rs, routes.rs and the surface fixture lie outside the manifest; they need a brief revision under CN9.

### R5: Proof on a fresh macOS account

Behavioural. On a fresh macOS user account on a machine that has never held Lys, a person downloads Lys.dmg, drags Lys to Applications, opens it, completes first-run setup and signs in. The run is recorded (screen recording and the process tree), and the record shows no Terminal or shell process started by the person or by Lys on their behalf and no page naming the issuer. The fresh account is a macOS virtual machine on Tom's Mac made for the proof, never a new account on Dean's laptop (it belongs to Tom's father); making it waits for Tom's word.

**Acceptance:**
- The recorded run reaches signed in from the disk image.
- The process record holds no terminal started for the person, and a text scan of every page shown finds no issuer name.

**Files:**
- create: docs/design/directory/reports/DIRECTORY-054-fresh-account.md

**Checklist:**
- C399 — On a fresh macOS account, a person goes from the downloaded disk image to signed in with no terminal process started and nothing naming the issuer (DIRECTORY-054 R5).

**Stories:**
- S160 (Person setting up Lys for the first time, Installs Lys on their own machine with no terminal knowledge and signs in) — As someone who has never used a terminal, I want to download Lys, open it and be guided in my browser until I am signed in, so that I can set it up for my team myself.

#### R5 — Execution record

**Dev (recorded):**

- Status: blocked
- How: The procedure is written: the process record and a scan of every page's text for the issuer, from a signed, notarised Lys.dmg on a fresh macOS VM.
- Deviation: (none)
- Files changed:
  - created: `docs/design/directory/reports/DIRECTORY-054-fresh-account.md` — the fresh-account procedure; Status: not run
- Checklist delivery:
  - [ ] C399 — On a fresh macOS account, a person goes from the downloaded disk image to signed in with no terminal process started and nothing naming the issuer (DIRECTORY-054 R5). — Not run. It needs R1's release image, signed with Tom's Developer ID, and the macOS VM on Tom's Mac.
- Story delivery:
  - [ ] S160 (Person setting up Lys for the first time, Installs Lys on their own machine with no terminal knowledge and signs in) — As someone who has never used a terminal, I want to download Lys, open it and be guided in my browser until I am signed in, so that I can set it up for my team myself. — Not run

**Review (recorded):**

- Alignment: drifted
- Acceptance verdicts:
  - [ ] The recorded run reaches signed in from the disk image. — docs/design/directory/reports/DIRECTORY-054-fresh-account.md says 'Status: not run.' and has an empty record section.
  - [ ] The process record holds no terminal started for the person, and a text scan of every page shown finds no issuer name. — Not run. Only code-level backing exists: flow_tests.rs no_shell_or_terminal_is_ever_started, progress_tests.rs the_page_is_sent_plain_words_only, and install-progress.test.tsx 'never names the issuer'.
- Issues:
  - Make the fresh-account run from a signed, notarised Lys.dmg on the macOS VM, and record in DIRECTORY-054-fresh-account.md the process record and the page-text scan. This needs R1's release build and the VM on Tom's Mac.

## Boundaries

- SHALL NOT start a terminal or ask the person to type a command, edit a file, choose a port or read a password file at any step.
- SHALL NOT produce an unsigned app under the release name.
- SHALL NOT name the issuer on any page.
- SHALL NOT add a timeout, sleep, poll, #[allow], #[ignore] or any bypass.
- SHALL NOT add a silent fallback: every failure is a named refusal shown in plain words.

## Verification

- The full Lys gate, ast-grep scan and surface checks exit 0 at the card's head, measured by the card round.
- R5's recorded run on a fresh macOS account.
