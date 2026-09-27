# Roots — User Stories

## Worker — Runs a survey or a round on a machine other than the one the design was written on

**S1.** As a worker, I want each root resolved through my own checkout of its repository at the pinned commit, so that a survey reads the same estate on any machine.

**S2.** As a worker whose checkout lacks a pinned commit, I want a refusal naming the checkout path, the repository, the commit and the fetch that answers it, so that nothing is cloned or fetched without my knowing.

## Method lead — Writes the design-system card that makes the method's reader accept the new shape

**S3.** As the method's lead, I want the root contract, the resolver and the pin check written down in lys before the method changes, so that the method's card is written from a contract rather than from a guess.

## Reviewer — Decides whether a root's pin should be moved

**S4.** As a reviewer, I want each root's pin printed beside origin's head with the distance between them, and a root with no pin or no origin head named rather than skipped, so that a stale pin is a visible fact and moving it is a decision someone makes.
