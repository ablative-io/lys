# Rauthy-Rebase — Checklist

## Trigger

- [ ] **C1** — The pull request and the rebase report record every non-pre-release upstream tag above v0.36.2 read, each with its tag commit and its git merge-base --is-ancestor exit statuses for c44f7cac and 989f9ff9, the list of qualifying tags, and the newest qualifying tag chosen with its commit, prerelease flag and changelog entry.
- [ ] **C2** — The three guard tests guard_account_status_rechecked_at_code_exchange, guard_code_bound_to_exact_redirect_uri and guard_logged_out_session_not_revived are one fork commit adding src/bin/tests/zzh_guard_auth_state.rs in the fork's rauthy member crate, pushed on the chosen tag commit to rauthy-rebase/<tag commit>-guards, which keeps it until the pin move has landed, and the fork gate's cargo test log for that commit names each as passed.
- [ ] **C3** — A guard test that fails or is absent at a candidate tag stops the card at trigger-guards when no candidate qualifies, with ablative, the fork's main, every pre-rebase tag and the lys pin unmoved.
- [ ] **C4** — The rebase report lists, for c44f7cac and 989f9ff9, every test name the recorded attribute command prints (each fn following an added #[test] or #[tokio::test]), and states against each name, test_validate_dyn_redirect_uri included, that it is not an account-status, redirect or logout guard test.
- [ ] **C5** — The v0.36.2 exception lines are exactly the output of the recorded command git show <lys start commit>:docs/design/identity/reports/IDENTITY-001-deployment.md | grep -n -F 'accepted v0.36.2 exception', which selects only the accepted-exception lines, copied whole with their line numbers into the rebase report.

## Rebase

- [ ] **C6** — Before the rebased build starts, every postgres and hiqlite migration number of the upstream tag and of the fork commits is listed, and a number held on both sides stops the card naming both files and the upstream commit that added the upstream one.
- [ ] **C7** — git log <tag commit>..<candidate> lists the fork commits present at card start by subject, in order and with unchanged patch-ids, then exactly one guard-test commit on top whose git show --name-only lists test files only; with no fork commits it lists the guard-test commit alone. After rebase, rauthy-rebase/<tag commit>-guards holds the guard-test commit and rauthy-rebase/<tag commit> the candidate, at the shas the rebase report names.
- [ ] **C8** — The fork build and full suite pass on rauthy-rebase/<tag commit> before ablative moves, and while fork commits exist the four ID001_LINK_* tests pass there.
- [ ] **C9** — A candidate that fails its build or suite stops the card naming the failing step, with ablative, every pre-rebase tag and the pin unchanged and the candidate branch kept.

## Ref safety

- [ ] **C10** — Before ablative is force-pushed, its previous head is pushed as pre-rebase-v0.36.2-<12 hex of that head>, and ablative moves with --force-with-lease naming the previous head, leaving the fork's main unchanged.
- [ ] **C11** — A failure at any step after ablative moves and before the lys pull request lands resets ablative to the tagged previous head with --force-with-lease naming the rebased head, verified by git ls-remote, and stops the card naming the step.

## Pin and install

- [ ] **C12** — The vendor/rauthy gitlink on the lys pull request's branch names the new ablative head, and ID001_PIN_CLONE passes on that ref.
- [ ] **C13** — After ablative moves, a recursive clone of the lys start commit, whose gitlink is the previous ablative head, exits 0 and checks out that head, and the pre-rebase tag is a fork ref containing it.
- [ ] **C14** — A disposable database is brought up first by the previously pinned build, which writes a test identity, then by the rebuilt image with no hand migration; that identity signs in under the rebuilt image, and the database is removed afterwards.
- [ ] **C15** — The Rauthy entry of deploy/identity/versions.json holds in the same field, produced by the same command, the value for the rebuilt image that the start-commit entry held for the old one.

## Records

- [ ] **C16** — The copied exception lines are gone from the deployment report, which names the new release, its tag commit and its advisory check; the rebase report records the upstream commit, the rebased diff, the regression results and the installed hash; the card's landing record, written after the lys gate passes, holds the gate result as evidence for IDENTITY-001-UPSTREAM-AUTH-STATE; the rebase report names the separate decision to lift it and the later move of the kept development data as acts this card does not perform.

## The fork gate's verdict (RAUTHYREBASE-002)

- [ ] **C17** — The base wrapper's pass on a detached, still-running builder is shown red against real Docker (RAUTHYREBASE-002 R1).
- [ ] **C18** — A result comes only from a stopped container, and Running true with ExitCode 0 is refused by name (RAUTHYREBASE-002 R2).
- [ ] **C19** — Success needs a completed verdict bound to this invocation and commit, and missing, truncated, stale or wrong-commit verdicts are refused by name (RAUTHYREBASE-002 R3).
- [ ] **C20** — Cancellation reaches everything the gate owns, keeps its result, and cleanup never overwrites the outcome (RAUTHYREBASE-002 R4).
- [ ] **C21** — The venue's generic gate receipt is surveyed for the same defect (RAUTHYREBASE-002 R5).
