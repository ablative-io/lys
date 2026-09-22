# Chippy to Archie — identity platform handoff

Written 22 September 2026, Melbourne, following Tom's 15:25:59 request to make the work transferable before Chippy's usage runs out. Read this with the live Git state; Waffles is concurrently committing the statement. No application build or runtime change is in progress.

## Current update — 15:36:25 ruling

Further ruling at **15:40:29**: the release rebase is future **IDENTITY-002** (4h estimate), opened when its upstream release exists; it blocks rows06/07 but is outside IDENTITY-001. Revision5 estimates44.5h within48h. Row02 must use Rust `lys identity prepare/configure/health`, PostgreSQL init SQL and Rust integration tests, not Python/provisioning shell. The exact module manifest is in revision5 for Waffles' re-check before source starts. Read-only preparation found SpiceDB latest release **v1.56.2** (11 September) and working Docker client/server **27.3.1/28.3.2** on this Mac; no images selected/pulled or deployment started from those observations.

Waffles accepted row01 and authorised isolated **development** deployments/showings in rows02–05 on v0.36.2. Row02 preparation resumes; the blanket installation stop in the earlier supplement is superseded. **The security blocker still binds real sign-in row06 and production installation row07.** Brief revision5 names a separate future IDENTITY-002: rebase onto an upstream release carrying both #1696 and #1728, then prove the account-status, exact-redirect and logged-out-session guards on the venue. If no suitable release is published by the row05 showing, Tom chooses nightly versus waiting. Release/advisory checks recur before each install.

The pin and handoff were pushed in **146796f55a46bd21dd4ded86ac1b398fd54c0eff**; no files were left staged. New row02 setup-helper filenames were sent to Waffles for file-wall approval before writing them. Gate schema timeout remains with Heimdall and no gate is submitted until repaired. Archie retains step04 and has acknowledged all needed pointers.

## Latest supplement — supersedes the staged-state and branch questions below

Waffles settled the branch/base at door time **15:30:50**. Branch **`ablative`** has now been created from `dd61ac3` and pushed; fork `main` stays untouched. `.gitmodules` names `branch = ablative`; the exact gitlink remains `dd61ac3`. This supplement, baseline, brief revision 4 and submodule metadata are being committed and pushed together by the explicitly authorised pathspec. The historical staged-state description below records the earlier handoff, not work to repeat after this commit.

Upgrades are rebases onto **upstream release tags only**, each with its own row and gate. No cherry-picks. Read-only review of `989f9ff9` found fixes on our intended deployed paths, so **installing v0.36.2 is blocked pending an acceptable upstream release and Waffles' ruling**. Latest release API still reports v0.36.2. See the baseline supplement for source-level evidence; no exploit or runtime proof has run.

Archie acknowledged the full handoff at **15:30:58** and all three additional pointers at **15:31:45**; his post `687aeb89af243283f3bacb794b10a704491e6c20ec88b100d40e622e6d27bdc1` confirms nothing more is needed. **He retains step 04 only. Chippy's rows pause if usage ends and resume with Chippy unless Tom changes ownership.**

Shared-envelope intent is now jointly agreed: typed audit/context payloads, coordinate in the returned receipt outside the leaf, service-attested human actions. Exact fields/encoding still require joint review. Add explicit hash-algorithm identity; SHA-256 attestation commitment and BLAKE3 content address are different facts. Row04's existing log-store path means the **file store**, not a proven Haematite backend. Archie's Haematite observations are a handoff finding requiring re-verification at an exact commit before adopting that backend.

The three row03 wall additions are approved and in revision4. Waffles explicitly authorises committing/pushing docs and the pin without a gate; fresh recursive clone is proved at the row02 venue gate. Heimdall owns the Argus schema timeout. **No gate submission from this lane until that defect is resolved; no local substitute.**

## Start here

1. `docs/design/identity/STATEMENT-2026-09-22.md` is the agreed product direction, maintained by Waffles.
2. `docs/design/identity/briefs/IDENTITY-001.json` is the authoritative first brief; `.md` is its render. Revision 2 was approved at 15:22:45; revision 3 records the subsequent colour direction and row-start status without expanding implementation scope.
3. `docs/design/identity/RAUTHY-BASELINE.md` records row-01 findings and what is still unproved.

Tom authorised Chippy on identity/access/broker (steps 1–3) and Cambium integration, Archie on memory/context/home (steps 4–5), with the lifecycle screen shared. This handoff makes either lane resumable; it does not silently transfer runtime control or authorise rows beyond Waffles' review. Tom's latest request is to ensure Archie has what he needs if Chippy loses usage.

## Approval and limits

- Approved brief commit: `ae0f0ad1c19018ef08a7ff8b97cc77a740c83d3d`, already pushed by Waffles.
- Approval message: `a1d511ca4ce5ed95cfd577c5d89341a6f16387c6b38dad037b2419b83b65f519`, door time 15:22:45 Melbourne. **Rows 01 and 02 go now.** Do not interpret this as permission to skip later contract review.
- Order: **01, 02, 04, 03, 05, 06, 07**. The directory/audit receiver must exist before linking needs its acknowledgement.
- Estimated focused hours: 1.5, 3.5, 10, 10, 6, 4, 5 in that order: 40 total, ceiling 48 including 8 contingency. External waits reported separately. These are estimates, not a calendar promise or hours already consumed.
- One implementer, one row and one gate at a time; no builders or sub-agents.
- Edit on main in the main Mac checkout. Release builds/checks/tests only through the laptop gate workflow, which fetches pushed refs. No SSH or copying a tree to the laptop. One light local `cargo check` is allowed; none has run.
- Keep every mutation inside its row's wall. Exact new-module manifests and the remaining upstream consumer files require a brief revision before their implementation.
- No production Cambium restart/auth switch without Gypsy's coordination. No identity service has been installed.

## Exact current implementation state

Repository: `/Users/tom/Developer/ablative/stack/lys`, main. Last observed parent before this handoff commit: `031b0ad` (Waffles' orange statement update).

The fork **exists**: <https://github.com/ablative-io/rauthy>. GitHub confirms its parent is `sebadob/rauthy`. Creation was reported to Waffles in post `e83a373f4260097355123290f14afc2dec8f8571940bb1176e9f6c399de81b78`; he told Tom at 15:24.

The submodule is cloned at `vendor/rauthy`, checked out detached at **`dd61ac3c84d6b238108dc8438b53043b5177a662`**, upstream tag **`v0.36.2`**. `git cat-file -t` confirms this object is a commit. The earlier reading-copy description calling it a tree SHA was inaccurate; use the verified commit.

**Fork branch detail to resolve before row-03 source edits:** GitHub's fork copied upstream main at `989f9ff9a2e18a9a3195084a86541c14f890a9e2`, 24 commits after v0.36.2. Only the submodule is pinned to v0.36.2; fork main has not been reset or force-pushed. Its newest subject is “Security hardening, stability and robustness, AI helpers (#1728)”. Waffles must settle the maintained fork branch/base before edits; do not claim fork main itself is at the tag or overwrite those commits to make that claim true. The row-01 baseline review must inspect relevant post-tag security fixes before deployment.

At handoff creation:

- `.gitmodules` and the `vendor/rauthy` gitlink are **staged, not committed**, owned by Chippy.
- Brief JSON/Markdown revision 3 are Chippy's changes; they are committed with this handoff by exact pathspec.
- The submodule has no source changes and no running command.
- No row-02 deployment files exist yet.
- Waffles owns the statement and the estate colour guide. Do not stage his files wholesale.

Use `git status --short --branch`, `git diff --cached --name-only` and `git submodule status` before resuming. The remaining staged submodule installation is intentional, not another person's abandoned change.

## Colour and screen direction

Tom wants common design but distinct product colours. Aion's browser appearance is the reference for layout, typography, spacing and behaviour; Cambium is not the visual authority. Do not introduce a build dependency on either application's frontend. Extracting a shared design package is an open separate decision.

Approved estate guide: `/Users/tom/Developer/ablative/docs/design-system-v2/palette/estate-colour-tokens.json`, rendered `estate-colour-family.html` beside it. Waffles committed the identity entry at **`385916eb437cae62c4269e6db1db2e542af5749e`**:

- identity accent **`#D4975A`**, deep **`#A86B2E`**, wash **`#3D2A17`**;
- use the guide's shared foundation; purple is banned;
- Cambium's own client stays moss green;
- Rauthy's theme supports seven HSL colours in light/dark, button text and radius. Its fonts/layout stay upstream's.

Aion source read: `apps/aion-ops-console/src/index.css` (last file commit `3b622599a`), self-hosted DM Sans and JetBrains Mono, radius/spacing ladders, light/dark palettes. It still contains a purple status token: **do not copy that inconsistency**. No new theme is installed.

## Shared contract for Archie's lane

`IDENTITY-001.json` → `shared_contract` and row 04 carry the proposed interface. No new durable signed format has been emitted or frozen.

- Enduring identity ID stays stable across provider links, keys and multiple sessions.
- Session/fork execution identity is distinct; a fork names its parent execution and enduring agent, with its own authority and credential.
- Service-signed identity audit names the authenticated human actor and authentication provenance. It must not falsely claim a human personally signed with a key they never held.
- Stable operation IDs allow retry/reconciliation without duplicate logical changes.
- Small versioned receipts hold commitments and log coordinates; encrypted context objects live outside the Merkle log.
- Originals remain immutable. Translation, summarisation and compaction create derived records with provenance and an explicit loss/transform account.
- Coordinate the directory/event contract with Chippy or his successor before row 04 signs bytes. Domain code stays out of `lys-core`; published cryptographic formats remain unchanged.

Archie's source reading: `/Users/tom/Developer/ablative/stack/norn/docs/design/norn-memory/MEMORY-AND-LANTERNS.md` and `/Users/tom/Developer/ablative/tools/lantern/docs/03-DESIGN.md`, with `02-SESSION-FORMATS.md` and `10-CUT-ONE-BRIEF.md`. The converter is designed, not proved; the first Lantern cut explicitly excludes it.

Read-only findings already posted at 15:00 in `c36d5c315c6234ed327c1af84678db70dbf00779dc16d9ffc69289670a65f01a`: Norn `7ec7ee2` has session ChildBranch/ForkComplete/Compaction, anchored fork seeds and inherited confinement/permissions/cancellation. That was not a live portability proof. Norn had unrelated dirty files, untouched.

Lys log findings: `lys-log-store/src/log.rs` writes the leaf before the pin, poisons its handle after an interrupted append and repairs one contiguous interrupted append on reopen. It loads all leaves into memory. Use small receipts and plan scaling; do not pretend this is an unlimited transcript store. `lys-core` supplies Merkle proofs, signed attestations and authenticated sealing; `lys-anchor/src/upward/pin.rs` pins signed child checkpoints. Integrity evidence does not prove semantic accuracy or completeness of capture.

## Cambium integration findings

Explicit authenticated account linking already exists. Preserve participant IDs, memberships and authorship; never merge people by matching email. `crates/cambium-door/src/http/oidc.rs` has state/nonce/cookie checks and one-use transactions, but no PKCE yet. Rauthy PKCE is selected by `Client.challenge`, not implied by confidentiality; S256 is a conditional reviewed configuration choice.

Google Drive/Calendar in `http/google.rs` still reuses sign-in provider/client credentials. Separate that before switching login to Rauthy: Cambium-to-Rauthy, Rauthy-to-upstream-provider, and Cambium-to-Google-API are different client purposes. A refresh token remains bound to its original client; changing client IDs may require reconnect.

Gypsy retains production runtime/install coordination. Last completed Cambium receipt: `.work/install-reconnect-stage-timeout-20260922/REPORT.md` and `.work/install-latest-20260922/receipt.json`; client `ed751e108`, backend `6b6057ba9`, PID 46796 at last verification earlier today. **Recheck live state before using these time-sensitive values.** Nothing in this identity work restarted it.

## Checks actually performed, and not performed

Performed: JSON parse/render validation, seven unique dependency-ordered rows, per-row-hour total 40, `git diff --check` and staged path-specific whitespace check, GitHub fork/parent verification, upstream/fork tag agreement, commit object/type and clean submodule inspection. Read source/CI/toolchain/license and published GitHub advisory metadata.

Not performed: fresh recursive clone acceptance; Rust `cargo check`, fmt, clippy, tests or docs; frontend build/typecheck/tests; dependency audit; formal venue battery; deployment, database migration/restore, provider login, live browser acceptance or installation. **Row 01 is in progress, not green. Row 02 has not started.**

Argus discovery: registered this connection as Chippy, role `IDENTITY-001 rows 01-02; venue gate coordination`. `queue_catalog` answered (very large); no run submitted. `workflow_schema({workflow:"gate"})` returned **Argus read failed: :timeout; retry the read.** No gate is running under my ownership. Resolve the actual laptop workflow and available venue before submitting anything; preserve exact refs and do not substitute local builds.

One erroneous Cambium send with empty text was refused locally (“text is required and must be a non-empty string”); the real fork announcement above was then accepted with its recorded post ID. No uncertain duplicate post is pending.

## Next actions

1. Read the approved brief and this handoff; coordinate whether Archie continues only his step-04 brief or also takes Chippy's in-progress rows while usage is unavailable.
2. Close row 01: finish post-tag security assessment and exact consumer inventory; settle fork main/base, prepare the reviewed row-03 file-wall additions, record source baseline. Do not alter upstream source yet.
3. Commit/push the staged `.gitmodules` and exact gitlink through the agreed venue workflow preparation, with Waffles coordinating the necessary exact-ref gate. Prove a fresh recursive clone at the venue. Do not declare the row complete from a local clone alone.
4. Row 02: Rauthy and SpiceDB use **one PostgreSQL service and database**, isolated roles/schema namespaces; verify migrations/search paths can coexist. Three dependency processes beside the platform door. Rauthy's internal Hiqlite cache still runs; document cache/key/config backup needs honestly.
5. Keep the declared milestones: after row 03 show Tom two providers/one person in Rauthy's own account page; after row 05 show the standalone directory. Each has a separate installed-ref/browser-pass receipt with Melbourne time.

## Coordination identifiers

Cambium stream: `mHyUs0FppSFqSzfT5kjBzZk23asX6tgScUEKNwJawq0`.

- Archie registry ID: `WwCyTiki6ydlv610KpU2pIQWIw5Xz2S3CVRenOR-Mgo`.
- Waffles: `ABYieZDByDHveYwp0YgkRlc2ebKoN1p7TdRv-1GVeY8`.
- Gypsy: `hc8rLC5eQGmjtrcoHwp7vJHMkz60CFZCvfEU0Ig0SI4`.

Use Cambium for teammate handoffs and Dot for Tom's spoken conversation. Never use question tools with Tom. Wait for the final voice chunk; a held Dot say is not yet heard and must not be resubmitted. No request to wake Buckley.
