# DIRECTORY-054: decisions taken and items still open

This file records what the build of DIRECTORY-054 decided that its brief does not say, and
what is still open, so a reviewer can see each item by name. Nothing below claims that a
live run has been made.

## Decision: lys-install is published with lys, by a release card

`lys` now depends on `lys-install`. `lys-install` is `publish = false`, so the next `lys`
cannot be published on its own. This was already true before this row: at `a67eefc`, `lys`
used `include_str!` on `deploy/identity/*` files that sit outside its package, so the
published crate could not have been built from its tarball either.

The decision: a release card publishes `lys-install` beside `lys`. Before it does, it moves
the `deploy/identity` files that `lys-install` embeds inside that crate. Until that card
lands, no `lys` release is cut from this tree. This row publishes nothing.

## Brief revisions this row asks for (CN9 and CN1)

- **CN1** says "Documents only: nothing outside docs/design/directory/ and
  docs/design/decisions.json is created or modified". That contradicts every code file R1
  to R4 name. The build followed the requirements. The brief needs CN1 rewritten so that
  its constraints and requirements agree.
- **Files outside the manifest**, which need a reviewed revision under CN9:
  - R1:
    - `crates/lys/src/cli.rs`
    - `crates/lys/src/commands/error.rs` and `error_tests.rs`
    - `crates/lys/src/main.rs`
    - `crates/lys/Cargo.toml`
    - the shared `build.rs` restamp in `crates/lys`, `lys-home`, `lys-identity-server` and
      `lys-secrets`, with their `tests/version.rs`
  - R2:
    - `crates/lys/src/identity/*`
    - `crates/lys/src/commands/mod.rs`
    - `crates/lys-install/src/output.rs`
    - `crates/lys-install/src/steps.rs`
    - `crates/lys-install/src/install/engine_path.rs`
    - in `crates/lys-app/src`: `launcher.rs`, `flow.rs`, `server.rs`, `bundle.rs`,
      `refusal.rs`, `uninstall.rs`, `engine.rs` and `engine_wait.rs`
  - R4:
    - `crates/lys-identity-server/src/uninstall_api.rs`
    - `ServerError::UninstallUnavailable` in `error.rs`
    - `error_status.rs`
    - `routes.rs`
    - `lib.rs`
    - `surface/identity/tests/fixtures.ts`
- **Start at login.** It is written as a LaunchAgent (`au.com.ablative.lys`, running
  `bin/lys-app --at-login`), not the SMAppService the spec names. SMAppService needs the
  app signed with the Developer ID, which is not held here. Tom's ruling is to be recorded
  in the revision: accept the LaunchAgent, or have a later row move it to SMAppService.

## Live runs still to make

| Item | Where it runs | What it needs |
|---|---|---|
| R1 development package: record `lipo -archs` on each bundled binary and `codesign --verify --deep --strict` on the app | Dean's laptop (a heavy build, rule 3) | `rustup target add x86_64-apple-darwin`, then builds for both processors from one commit and `lys package app --development` |
| R1 release: record `codesign --verify --deep --strict` and `spctl --assess` on `Lys.app` and `Lys.dmg` | a machine holding the Developer ID | Tom's Developer ID and a notary keychain profile. This Mac's `security find-identity -v -p codesigning` reports 0 valid identities |
| R2 open a development build with no install and reach `/setup` | Dean's laptop | DIRECTORY-047's setup page landing in this tree |
| R4 a restart, an upgrade (the Build line shows the new commit), and an uninstall without the tick followed by a reinstall that signs the same people in | Dean's laptop | a development build |
| R5 the fresh-account run | the macOS VM on Tom's Mac | R1's release image and the VM, as in `DIRECTORY-054-fresh-account.md` |
