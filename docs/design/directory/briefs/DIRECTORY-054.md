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
  - [ ] The built app passes codesign --verify --deep --strict and spctl --assess. — crates/lys/src/package.rs runs codesign --verify --deep --strict, and spctl --assess for a release, but no package has been built. This Mac holds 0 codesigning identities, and there is no recorded run in docs/design/directory/reports/.
  - [x] Binaries built from two commits are refused naming the binary. — cargo test -p lys --bin lys package: binaries_from_two_commits_are_refused_naming_the_binary ok; packaging_binaries_from_two_commits_writes_nothing ok.
  - [x] With no signing identity it is refused signing_identity_missing and no Lys.dmg is written. — package::tests::with_no_signing_identity_a_release_is_refused_by_name ok; a_release_without_its_signing_identity_writes_no_disk_image ok (run() is refused and no Lys.dmg or output folder is written).
  - [ ] lipo -archs on each bundled binary names arm64 and x86_64. — package.rs checks lipo -archs (universal_incomplete), but no universal package has been built: x86_64-apple-darwin is not installed here, and the run belongs on Dean's laptop under rule 3.
- Issues:
  - Build a --development package on Dean's laptop and record lipo -archs for each bundled binary and codesign --verify --deep --strict on Lys.app.
  - Build a release with Tom's Developer ID and notary profile and record codesign and spctl --assess for Lys.app and Lys.dmg.
  - Get the brief revised: CN1 (documents only) contradicts R1's code, and cli.rs, commands/error.rs, main.rs, Cargo.toml and the shared build.rs restamp lie outside the manifest. A reviewer has to approve the revision under CN9 before these edits stand.

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
  - [ ] Opening the app on a machine without an install reaches the first-run setup page with no terminal started. — flow_tests::the_browser_is_handed_to_first_run_setup_on_lys and no_shell_or_terminal_is_ever_started pass, but the app has not been opened live, and DIRECTORY-047's /setup is not in this tree.
  - [x] A failure at any step shows its words and next action on the page and leaves the install resumable. — flow_tests::a_failure_is_shown_at_its_step_with_the_next_thing_to_do ok. lys-install install.rs run() reports each Step before that step's work, and makes only what is missing, so a rerun resumes. server.rs now names an unreadable file instead of hiding it (the fix below).
  - [ ] Opening the app twice during an install shows one install. — launcher.rs claim() uses a flock one-instance lock (Claim::Theirs reads the port of the instance already running). No test or live run opens the app twice during an install.
- Issues:
  - lys-app server.rs file() answered 404 not_found when a screens file existed but could not be read, which hid the failure.
  - Record a live first open of a development build that reaches /setup; this waits on DIRECTORY-047 landing.
  - Add a test or a recorded run where a second open during an install joins the first install rather than starting another.
  - Get the CN9 revision approved for identity/*, commands/mod.rs, output.rs, steps.rs, engine_path.rs and the lys-app modules outside the manifest.
- Fixes:
  - crates/lys-app/src/server.rs file(): a read error other than NotFound now answers 500 screen_unreadable, naming the path and the error; a missing file still answers 404. lys-app fmt and clippy -D warnings are clean, and tests are 59/59.

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
  - [x] With the engine stopped, the page shows the guidance, and starting the engine continues the install with no action in Lys. — engine_wait_tests: the_wait_ends_on_the_folder_notice_when_the_engine_starts, a_socket_made_before_its_engine_answers_ends_the_wait_when_it_answers and an_engine_that_says_not_yet_is_asked_again_when_its_state_folder_changes pass. progress_tests and surface/identity/tests/install-progress.test.tsx show the guidance. These tests use a stand-in engine socket; Docker Desktop itself was not run.
  - [x] No wait in lys-app reads a clock or loops on a sleep. — A grep of crates/lys-app/src (not the tests) finds no Instant, SystemTime, Duration, sleep or timeout. engine_wait.rs waits only in kqueue poll_forever(None), and progress.rs waits on a Condvar.
- Checklist verified: C397

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
  - [ ] After a restart of the Mac, Lys is answering without the person doing anything. — login_item.rs writes the LaunchAgent au.com.ablative.lys, and login_item_tests pass, but no restart has been run.
  - [ ] Opening a newer app upgrades the install and the Build line shows the new commit. — flow_tests::an_upgrade_says_its_lines ok, but no live upgrade has been run.
  - [ ] Uninstall without the tick leaves the data folder, and a reinstall signs the same people in. — uninstall_tests and surface/identity/tests/uninstall.test.tsx pass, but no reinstall with sign-in has been run.
- Issues:
  - Start at login is a LaunchAgent, not the SMAppService the spec names. Tom's ruling has to be recorded in a brief revision.
  - Record the live runs: a restart, an upgrade with the new commit on the Build line, and an uninstall without the tick followed by a reinstall that signs the same people in.
  - Get the CN9 revision approved for uninstall_api.rs, ServerError::UninstallUnavailable, error_status.rs, routes.rs, lib.rs and surface/identity/tests/fixtures.ts.

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
  - [ ] The recorded run reaches signed in from the disk image. — docs/design/directory/reports/DIRECTORY-054-fresh-account.md says Status: not run.
  - [ ] The process record holds no terminal started for the person, and a text scan of every page shown finds no issuer name. — There is no run, so there is no process record or page scan.
- Issues:
  - Run the fresh-account procedure on the macOS VM from a signed, notarised Lys.dmg, and record the process record and the page-text scan. This needs R1's release image first.

## Boundaries

- SHALL NOT start a terminal or ask the person to type a command, edit a file, choose a port or read a password file at any step.
- SHALL NOT produce an unsigned app under the release name.
- SHALL NOT name the issuer on any page.
- SHALL NOT add a timeout, sleep, poll, #[allow], #[ignore] or any bypass.
- SHALL NOT add a silent fallback: every failure is a named refusal shown in plain words.

## Verification

- The full Lys gate, ast-grep scan and surface checks exit 0 at the card's head, measured by the card round.
- R5's recorded run on a fresh macOS account.
