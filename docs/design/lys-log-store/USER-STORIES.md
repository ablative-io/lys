# Lys-Log-Store — User Stories

## Consumer library — Settling an uncertain append by reading the leaf back

**S1.** As the identity directory committing through the file store, I want a leaf that reads back under its final name to be whole and flushed so that settling an uncertain append by reading the leaf back gives the committed answer.

## Operator — Reopening a log after a crash or a failed write

**S2.** As an operator reopening a log after a crash, I want only whole, flushed leaves to be counted so that a torn leaf is never pinned into the tree.

## Writer — Racing another writer for the same index

**S3.** As a writer whose index another writer has already taken, I want my write refused by name so that neither writer's leaf is replaced.

## Writer — Seeing a failure after its leaf was named

**S4.** As a writer whose append failed after its leaf was named, I want an error that says the leaf is written but its durability is uncertain so that I never mistake my own leaf for another writer's.

## Read-only caller — Opening a store without writing to it

**S5.** As a read-only caller opening a store, I want open to leave every file in the directory as it found it so that a read-only open stays read-only.

## Read-only caller — Reading a log on read-only media

**S6.** As a reader of a log on media that refuses a directory flush, I want a read-only open to count the named leaves without flushing so that a status opens as it did before this change.

## Read-only caller — Reading a log that a crash left one leaf ahead of its pin

**S7.** As a reader of a log one leaf ahead of its pin, I want a read-only open to leave the store as it found it and tell me that a writable open repairs it so that reading never writes and the pending repair is never silent.

## Log operator — Opens a log store, including after a crash

**S4.** As a log operator, I want a store whose pinned leaves no longer rebuild to the pin to refuse to open and tell me the pin and the root the leaves give, so that a damaged leaf is never committed to the tree and I am never told more than the store can prove.

**S1.** As a log operator, I want to be told which leftover temporary files opening my store ignored, so that what an interrupted append left behind is visible rather than skipped in silence.

## Third-party verifier — Checks a leaf without lys

**S2.** As a third-party verifier, I want a leaf from a store written after this change to verify with the standalone Python verifier, so that checking a leaf still needs no lys code.

## Witness operator — Runs a witness anchor that observes other logs' checkpoints

**S3.** As a witness operator, I want each observation to cost only the leaves recorded since the previous one, so that the witness stays usable as its own log grows.

## Auditor — Reading a flight recorder's log without changing it

**S1.** As an auditor, I want a leaf store I open for reading to refuse every write so that reading the log never changes the evidence.

**S2.** As an auditor, I want a store left mid-append to be refused by name when I open it for reading so that I learn a repair is pending instead of performing one.

## Operator — Recovering a log after a crash

**S3.** As an operator, I want a writable open to repair an interrupted append as it does today so that a crash costs no history.

**S4.** As an operator, I want to be told which leftover temporary leaf files an open skipped so that I can clear them myself, knowing the store never will.

## Verifier — Trusting a leaf the log serves

**S5.** As a verifier, I want a leaf damaged inside the pinned prefix to be refused by name at open so that a corrupted leaf is never served as whole.

## Developer — Building a reader on the leaf store

**S6.** As a developer, I want the leaf store's module doc to say what open does and never does so that I know which call proves a leaf whole.

## Log inspector — Opens a log store to read it without changing it

**S5.** As a log inspector, I want a store I open read only to refuse every write, so that reading a store can never change it.

**S6.** As a log inspector, I want a read-only open of a store with an interrupted append to refuse and say that a repair is pending, so that I learn the store is past its pin without my open repairing it.

## Leaf store maintainer — Reads the file store's contract before relying on it or changing it

**S7.** As a leaf store maintainer, I want the file store's module doc to say what opening a store does and never does, so that I can rely on open never deleting a leftover temporary file.
