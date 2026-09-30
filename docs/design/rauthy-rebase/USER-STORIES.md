# Rauthy-Rebase — User Stories

## Operator — Runs the standalone identity product

**S1.** As the operator, I want the identity product rebuilt on the hardened Rauthy release to start on data the previous release wrote, so that the upgrade needs no migration run by hand.

## Fork maintainer — Keeps the ablative branch of the Rauthy fork

**S2.** As the fork maintainer, I want our commits to sit unaltered on top of the upstream release tag, so that a diff against the tag shows only our work by name.

**S3.** As the fork maintainer, I want ablative to move only after its new head is green, with its previous head kept under a tag, so that ablative never sits on a failing head and every earlier lys pin still resolves.

## Reviewer — Reads the rebase pull request

**S4.** As the reviewer, I want the trigger recorded as facts in the pull request, so that whether the card may start is checked rather than judged.

**S5.** As the reviewer, I want the account-status, redirect and logout guards proved at the release tag, so that a release without the hardening can never be taken as the base.

**S7.** As the reviewer deciding whether a fork change may land, I want a gate that was interrupted or cut short to say so by name and never read as a pass, so that I never land a tree nobody finished testing.

## Person — Signs in with two linked providers

**S6.** As a person who linked two sign-in providers, I want both to still resolve to me after the rebase, so that the upgrade never splits my identity.
