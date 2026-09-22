# IDENTITY-001 row 01 — Rauthy source baseline

Observed 22 September 2026, Melbourne. **In progress; no gate or deployment passed.**

## Latest ruling and security blocker

Waffles settled the maintenance base at door time 15:30:50: `ablative` starts at v0.36.2, fork main stays untouched, release-tag rebases only, no cherry-picks. **Branch `ablative` was created and pushed at `dd61ac3`; `.gitmodules` tracks that branch while pinning its exact commit.** Earlier unresolved-base wording below is superseded. The three omitted row03 files below are now approved and included in brief revision4.

Read the actual diff of upstream `989f9ff9a2e18a9a3195084a86541c14f890a9e2`, compared its relevant paths with the pinned baseline, and found:

| Intended deployed path | Later change | Baseline implication |
| --- | --- | --- |
| `src/service/src/oidc/grant_types/authorization_code.rs` | Rechecks `user.check_enabled()` and `check_expired()` immediately before token creation | The pinned exchange reads the user then creates tokens without those rechecks; a status change between authorization and exchange needs protection |
| Same grant path plus `src/data/src/entity/auth_codes.rs` | Stores and validates the exact redirect URI used for that code | The pin checks the client's allowed URI list, not the URI bound to that authorization code; relevance increases with multiple allowed callbacks |
| `src/data/src/entity/sessions.rs` | Refuses `SessionState::LoggedOut` in `set_authenticated` | The pinned setter writes Auth and is called by authorization-code exchange; logout-state revival protection is missing on an intended path |
| `src/service/src/oidc/validation.rs`, refresh-token entities | Reworks atomic refresh-token consumption, expiry and ownership checks | Refresh is an intended path; additional semantic review and regression proof are required before a safety claim |
| `src/data/src/entity/auth_providers.rs` | Adds missing PostgreSQL delete argument and normalizes upstream email | Both provider administration and Google/GitHub federation are intended deployment paths |
| `src/api/src/sessions.rs`, `src/service/src/oidc/logout.rs` | Carries device-token revocation policy and corrects logout response/backchannel behavior | Session/logout paths are deployed; device-specific impact depends on enabled flows |
| Provider login/start and callback ATProto branches | Removes panic/unwrap outcomes | Outside the planned Google/GitHub provider configuration; not used as the primary blocker |

**Named blocker: `IDENTITY-001-UPSTREAM-AUTH-STATE`.** The first and third findings concern the planned sign-in/session authority, not an unused extension. Stop installation of this baseline and take the upstream release decision to Waffles. These are source-level findings, not a completed exploit demonstration or severity assignment. No local patch or cherry-pick was applied. The GitHub latest-release API still reports `v0.36.2`; no newer release was available at this read. Reported to Waffles in post `59ddadc6b1446d4e7f19f803b2eccee0c211f273b28a998eb6c65ddc2b8614cc`.

Additional commands: `git switch -c ablative dd61ac3...` and `git push -u origin ablative` both succeeded; `git show --no-ext-diff --format= 989f9ff9 -- <named paths>` and baseline source reads succeeded; `gh api repos/sebadob/rauthy/releases/latest` returned v0.36.2. No runtime or test invocation was made.

Fork: <https://github.com/ablative-io/rauthy>, parent `sebadob/rauthy`. Selected tag `v0.36.2` resolves to commit `dd61ac3c84d6b238108dc8438b53043b5177a662`, subject `Prepare v0.36.2 (#1695)`. The Lys submodule at `vendor/rauthy` is pinned to that commit; fork main remains upstream's later `989f9ff9a2e18a9a3195084a86541c14f890a9e2` and needs an explicit maintenance-base decision before fork edits. No reset/force-push performed.

Licence: Apache-2.0, verified from `LICENSE` and workspace metadata. Cargo edition 2024, declared Rust minimum **1.95.0**. The upstream code-style workflow uses `ghcr.io/sebadob/rauthy-builder:20260519`, runs `just build-wasm`, installs frontend dependencies and builds UI before backend fmt/clippy and frontend format/check. These are source findings, not proof the laptop venue supplies that toolchain. No new lint exemptions inherit legitimacy from upstream's existing code.

Fork maintenance owner for this lane: Chippy, with Waffles reviewing security/update decisions; handoff to Archie requested by Tom if Chippy's usage expires. Upstream release/security checks must recur before release and whenever a relevant advisory appears. Record the new upstream commit, rebased minimal diff, regression results and installed hash on each security update; never treat a past pin as permanently safe.

## Published security metadata read

The repository security-advisory API returned three entries on 22 September:

| Advisory | Affected versions | Patched version |
| --- | --- | --- |
| [GHSA-wx92-7mmw-5x82](https://github.com/sebadob/rauthy/security/advisories/GHSA-wx92-7mmw-5x82), unauthenticated token-endpoint panic | through 0.36.1 | 0.36.2 |
| [GHSA-7qh2-3hc5-2vqp](https://github.com/sebadob/rauthy/security/advisories/GHSA-7qh2-3hc5-2vqp), refresh-token client mismatch | 0.30.0 through before 0.36.1 | 0.36.1 |
| [GHSA-x8jp-v2j6-6vjf](https://github.com/sebadob/rauthy/security/advisories/GHSA-x8jp-v2j6-6vjf), WebAuthn authentication not bound to user | before 0.36.0 | 0.36.0 |

These ranges do not name v0.36.2 as affected. This is **not** a comprehensive security clearance: dependency scanning is unrun, and upstream main contains 24 later commits including `989f9ff9 Security hardening, stability and robustness, AI helpers (#1728)`. Inspect relevant fixes before closing row 01 or installing the baseline.

## Provider-field inventory

`rg -l 'auth_provider_id|federation_uid|provider_unlink' src frontend migrations` found 14 files. The following eight consume the user/link shape and must be traced for the linking change:

- `src/api/src/auth_providers.rs`: link guard and unlink route.
- `src/api_types/src/users.rs`: API user projection.
- `src/data/src/entity/auth_providers.rs`: provider deletion's linked-user query, callback link/auto-link, existing federation validation and initial user creation.
- `src/data/src/entity/users.rs`: data shape, find-by-federation, inserts/updates, unlink, account-type derivation, API conversion and tests.
- `src/data/src/migration/inserts.rs`: cross-database user transfer as well as unrelated logo transfer.
- `frontend/src/api/types/user.ts`: consumer type.
- `frontend/src/lib/account/AccMain.svelte`: selects the user's one linked provider.
- `frontend/src/lib/admin/users/UserInfo.svelte`: admin view of one linked provider.

Other matches are four historical initial schema/data migrations and two files whose matches concern provider logos (`src/data/src/entity/logos.rs`, `src/data/src/migration/db_migrate.rs`). Do not rewrite historical migrations merely because the grep found them. Cross-database migration orchestration still needs a full semantic trace beyond these field-name matches; this list is not yet the finished every-reader proof.

The first draft row-03 wall omits `src/api_types/src/users.rs`, `src/data/src/migration/inserts.rs` and `frontend/src/lib/admin/users/UserInfo.svelte`. **Name and approve these additions before row-03 implementation.** The actual migration suffixes also remain to be validated against the pinned tree. No out-of-wall file edited.

## Native event-system finding

Read `src/data/src/events/event.rs` (EventType 141–168, Event 403–412), `events/listener.rs` (39–53), `src/api/src/events.rs`, `src/schedulers/src/events.rs`, and link/unlink mutation paths.

There is no provider-link/unlink event type or typed subject/provider pair. Explicit link assigns fields for a later save; unlink clears them and saves without emitting such an event. Event handling spawns asynchronous work, persists only above a configured level, and cleanup deletes old events without a Lys acknowledgement. SSE exposes a bounded latest-event replay rather than an acknowledged durable cursor.

Therefore consuming the stock stream alone cannot satisfy the accepted crash-safe link audit contract. Compare a minimal transactional extension of native events with a dedicated link-only audit record during row 01. Keep the fork addition narrow, retain stable operation IDs and commit provenance in the link transaction. **No final table/transport design chosen and no table added.** The directory receiver is built in row 04 before row 03 starts.

## Clients and storage

`src/service/src/oidc/grant_types/client_credentials.rs` supports confidential machine clients; `src/api_types/src/clients.rs` has custom machine-token claims. This does not make an OAuth client synonymous with an enduring agent or prove the later capability design. Step 1 registers agents independently of runtime credentials.

`Client.challenge` drives PKCE validation (`src/data/src/entity/clients.rs`); confidential authentication is a separate check in authorization-code exchange. Dynamic client creation enables S256 by default for non-confidential clients, not for every confidential client. Cambium's S256 work remains conditional on its reviewed client configuration.

`src/data/src/database.rs` supports PostgreSQL and its migrations, while still starting the internal Hiqlite cache/node. Use the shared PostgreSQL deployment required by review; don't claim that eliminating the embedded identity datastore removes every non-SQL runtime file.

## Row-01 command evidence

All commands were run in Lys or its submodule on the Mac unless stated. No builds/tests below.

| Command / action | Result |
| --- | --- |
| `gh repo view ablative-io/rauthy --json nameWithOwner,url,isFork,defaultBranchRef` before creation | Exit 1, repository absent |
| `git ls-remote https://github.com/sebadob/rauthy.git refs/tags/v0.36.2 'refs/tags/v0.36.2^{}'` | Exit 0, tag resolves dd61ac3 |
| `gh repo fork sebadob/rauthy --org ablative-io --fork-name rauthy --clone=false` | Exit 0, fork URL returned |
| `gh repo view ablative-io/rauthy --json nameWithOwner,url,isFork,parent,defaultBranchRef` | Exit 0, correct fork parent and main |
| `git ls-remote https://github.com/ablative-io/rauthy.git refs/tags/v0.36.2` | Exit 0, same tag commit |
| `git submodule add https://github.com/ablative-io/rauthy.git vendor/rauthy` | Exit 0, clone complete |
| `git cat-file -t dd61ac3c84d6b238108dc8438b53043b5177a662` in submodule | Exit 0, commit |
| `git show -s --format='%H%n%ci%n%s' v0.36.2` | Exit 0, exact release commit/date/subject |
| Initial `rg --files` with unquoted `SECURITY*` | zsh refused unmatched glob; corrected with quoted patterns |
| `git checkout --detach v0.36.2` in submodule | Exit 0, exact pin, no source edits |
| `rg --files -g 'AGENTS.md' -g 'CLAUDE.md' -g 'LICENSE*' -g 'SECURITY*' -g '*toolchain*'` | Exit 0, licence files; no matching agent/toolchain files |
| `sed -n '1,100p' Cargo.toml`; `head -n 10 LICENSE` | Exit 0, toolchain and licence inspected |
| `git add -- vendor/rauthy`; `git submodule status`; `git diff --cached --stat` | Exit 0, staged pin and .gitmodules only |
| `gh api repos/sebadob/rauthy/security-advisories --jq ...` | Exit 0, three published advisories read, ranges recorded above |
| `gh api repos/sebadob/rauthy/releases/tags/v0.36.2 --jq ...` | Exit 0, release metadata/security note read |
| `rg -l 'auth_provider_id\|federation_uid\|provider_unlink' src frontend migrations` and line-numbered source searches | Exit 0, inventory above; semantic trace unfinished |
| Read `.github/workflows/code_style.yaml`; search machine-client source | Exit 0, upstream gates/client flow located |
| `git rev-list --count v0.36.2..origin/main`; `git log -3 --oneline origin/main` | Exit 0, 24 later commits and security-hardening head |
| Argus `workflow_schema` for `gate` | Read timeout; no gate submitted |

Fresh recursive clone, upstream/dependency security completion, gate receipt and installed behavior remain unrun. The full handoff is in `HANDOFF-2026-09-22-CHIPPY.md`.
