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

Tom, 28 September 2026 about 19:10: Lys is the only sign-in for everything, and "how is an average person supposed to do it?" Today the install is a terminal command that needs a container engine already running, and nothing packages Lys as something a person downloads.

## Task

Add Lys.app for macOS: a packaging command that builds a signed, notarised app and disk image; an app that runs the install in its own process with progress on a Lys page and hands over to first-run setup; guidance for the container engine; start at login, reopen, upgrade and uninstall; and a proof on a fresh macOS account that no terminal is ever needed.

## Requirements

### R1: `lys package app` builds the app and disk image

Behavioural. `lys package app --out DIR` builds Lys.app (bundle id, Info.plist, icon) holding lys, lys-identity-server, lys-secrets and lys-home built for the host architecture, the screens package, and lys-app as the bundle's executable, and a Lys.dmg with the app and an Applications link. Each binary's --version must name the same commit, or the packaging is refused naming the one that differs. It signs with the Developer ID named by a secret handle (DIRECTORY-052 R6, SECRETS), with the hardened runtime, and submits for notarisation, stapling the ticket; with no signing identity or a refused notarisation it fails naming which, and never produces an unsigned app under the release name.

**Acceptance:**
- The built app passes codesign --verify --deep --strict and spctl --assess.
- Binaries built from two commits are refused naming the binary.
- With no signing identity it is refused signing_identity_missing and no Lys.dmg is written.

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

### R2: Opening the app installs and shows every step on a Lys page

Behavioural. lys-app, when no install exists under the platform data root, serves a Lys-styled progress page on a loopback port it chooses, opens the default browser to it, and runs the install service as a library call (the same code as `lys identity install`, never a spawned shell or terminal). Each step appears as it happens in plain words ("Preparing your directory", "Starting sign-in"), with a failure shown in the same words and the one next thing to do. When the install is ready it hands the browser to the first-run setup page of DIRECTORY-047. Opening the app while an install is running shows that install's page, never a second install.

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
- modify: Cargo.toml
- modify: crates/lys/src/identity/install.rs

**Checklist:**
- C396 — Opening Lys.app runs the install service in the process, shows each step on a Lys page in the browser in plain words, and hands over to first-run setup (DIRECTORY-054 R2).

**Stories:**
- S160 (Person setting up Lys for the first time, Installs Lys on their own machine with no terminal knowledge and signs in) — As someone who has never used a terminal, I want to download Lys, open it and be guided in my browser until I am signed in, so that I can set it up for my team myself.

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

### R4: Start at login, reopen, upgrade and uninstall

Behavioural. After a successful install the app registers Lys to start at login for this user (a LaunchAgent under the user's Library, removed on uninstall). Opening the app when Lys is installed and running opens Lys's screens; when installed and stopped it starts it and then opens them. An app whose bundled build is newer than the install's runs the upgrade of DIRECTORY-045 with its progress on the page and its way back (DIRECTORY-053). An Uninstall control on the account screen of an administrator stops Lys, removes the login registration and the binaries, and keeps the data folder unless the person ticks to remove it after a confirmation that names what is lost.

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

### R5: Proof on a fresh macOS account

Behavioural. On a fresh macOS user account on a machine that has never held Lys, a person downloads Lys.dmg, drags Lys to Applications, opens it, completes first-run setup and signs in. The run is recorded (screen recording and the process tree), and the record shows no Terminal or shell process started by the person or by Lys on their behalf and no page naming the issuer.

**Acceptance:**
- The recorded run reaches signed in from the disk image.
- The process record holds no terminal started for the person, and a text scan of every page shown finds no issuer name.

**Files:**
- create: docs/design/directory/reports/DIRECTORY-054-fresh-account.md

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

## Verification

- The full Lys gate, ast-grep scan and surface checks exit 0 at the card's head, measured by the card round.
- R5's recorded run on a fresh macOS account.
