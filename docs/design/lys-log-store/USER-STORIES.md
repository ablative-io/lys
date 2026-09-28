# Lys-Log-Store — User Stories

## Log operator — Opens a log store, including after a crash

**S4.** As a log operator, I want a store whose pinned leaves no longer rebuild to the pin to refuse to open and tell me the pin and the root the leaves give, so that a damaged leaf is never committed to the tree and I am never told more than the store can prove.

**S1.** As a log operator, I want to be told which leftover temporary files opening my store ignored, so that what an interrupted append left behind is visible rather than skipped in silence.

## Third-party verifier — Checks a leaf without lys

**S2.** As a third-party verifier, I want a leaf from a store written after this change to verify with the standalone Python verifier, so that checking a leaf still needs no lys code.

## Witness operator — Runs a witness anchor that observes other logs' checkpoints

**S3.** As a witness operator, I want each observation to cost only the leaves recorded since the previous one, so that the witness stays usable as its own log grows.

## Log inspector — Opens a log store to read it without changing it

**S5.** As a log inspector, I want a store I open read only to refuse every write, so that reading a store can never change it.

**S6.** As a log inspector, I want a read-only open of a store with an interrupted append to refuse and say that a repair is pending, so that I learn the store is past its pin without my open repairing it.

## Leaf store maintainer — Reads the file store's contract before relying on it or changing it

**S7.** As a leaf store maintainer, I want the file store's module doc to say what opening a store does and never does, so that I can rely on open never deleting a leftover temporary file.
