# Releasing `lys`

A repeatable procedure, plus what is *not* mechanical about it. Written to be
executable by someone who did not stage the release.

## The two things a release must not do by accident

1. **Freeze a draft wire format.** A format is frozen by publishing a crate that
   exposes it or by signing a durable artifact under its tag. Draft formats live
   behind the off-by-default `unstable-anchor` feature for exactly this reason —
   check before every release that no draft format has become unconditionally
   public. Grep `crates/lys-core/src/lib.rs` for `#[cfg(feature =` and confirm the
   list matches the drafts in `docs/design/`.
2. **Describe something it does not ship.** The changelog is read by people
   deciding whether to upgrade. Verify each claim against the tree at the tag,
   not against memory or a plan document — `git ls-tree` and `git show` settle it.
   Entries for unreleased work make the record useless for telling what a
   published version contains.

## Ordering

`lys` depends on `lys-core` by path *and* version, so **`lys-core` publishes
first**. Until it is on crates.io, `cargo package -p lys` fails with
`failed to select a version for the requirement lys-core = "^X.Y.Z"` — that
failure is expected before the first publish and is not a problem to fix.

## Procedure

### 1. Gates, at the documented discipline

```
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo clippy --all-targets -- -D warnings
cargo doc --no-deps --all-features
LYS_REQUIRE_GO=1 cargo test --workspace --all-features
```

`LYS_REQUIRE_GO=1` is not optional for a release: without it a missing Go
toolchain *skips* the interop gates, and a release is precisely when "the
cross-check never ran" must be impossible. Confirm the Go suites appear in the
output rather than assuming they ran.

Also confirm `LYS_IDENTITY_KEY` and `LYS_GO_BIN` are **unset** — both change what
is exercised.

### 2. Versions and metadata

- `version` in the workspace `[workspace.package]`.
- The `lys-core` entry in `[workspace.dependencies]` — it carries its own
  `version` and is easy to leave behind.
- `repository` still correct.

### 3. Changelog

Add the version's entry. Verify every claim against the tree. State breaking
changes as breaking, in their own subsection, and say what a consumer must do.

### 4. Packaging dry-run — do not skip this

```
cargo package -p lys-core --allow-dirty
cd target/package/lys-core-<version> && cargo test --all-features
cargo package -p lys-log-store --allow-dirty
cd target/package/lys-log-store-<version> && cargo test
```

The dry-run catches what the workspace build cannot: files excluded from the
package that something included still needs. A test shipped without its fixture
panics for every consumer who runs the suite. The packaged crate's tests must
pass on their own.

Dry-run **every** crate being published, not just the one that changed. A crate
whose own sources are untouched can still package differently — a new
`exclude`, a moved fixture, or a dependency that stopped being path-only.

### 5. Publish

```
cargo publish -p lys-core
# wait for the index to carry it, then:
cargo publish -p lys-log-store
# wait again, then:
cargo publish -p lys
```

**The order is forced by the dependency graph, not chosen**, and `cargo` proves
it: `cargo package -p lys` fails while the version of `lys-core` it requires is
absent from the index. `lys-log-store` depends on `lys-core`, and `lys` depends
on both, so anything else fails loudly rather than shipping a crate pinned to a
version nobody can resolve. Wait for each to appear at the index before the next
— the verify build for the following crate downloads it from the registry.

### 6. Tag and verify from the outside

```
git tag -a v<version> -m "lys v<version>"
git push origin v<version>
```

Then install from the registry into a scratch directory and check the published
artifact behaves — `cargo install lys --version <version>` and run one command.
Verifying your own working copy proves nothing about what was uploaded.

## Authorization

**Publishing is irreversible.** A crates.io version cannot be replaced or
withdrawn — only yanked, which leaves it resolvable for anyone who already
depends on it. Consequently:

- The go-ahead comes from the repository owner **directly**, in session. A ruling
  relayed through another party is situational awareness, not authorization: if
  relayed approval were sufficient, anyone able to write to the relay could
  publish. Staging is unaffected — everything above except step 5 can and should
  be done in advance.
- Generating a production signing key, and emitting any artifact under a frozen
  tag outside a test, are held to the same standard for the same reason.

## Status of the 0.2.0 staging

Staged and gated at `8579b6e`; **not published**.

- Version, changelog, `repository`, and package excludes are in place.
- All five gates clean: fmt 0, clippy (both feature configurations) 0,
  `cargo doc` 0, `LYS_REQUIRE_GO=1 cargo test --workspace --all-features`
  560 passed / 0 failed / 0 ignored. Default-feature run: 479 passed.
- Packaging dry-run clean; the packaged `lys-core` suite runs 419 tests green.
- Draft formats (`receipt`, `bundle`) confirmed behind `unstable-anchor`, so this
  release does not freeze them.

Remaining: steps 5 and 6, on the owner's word.
