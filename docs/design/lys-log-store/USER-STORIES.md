# Lys-Log-Store — User Stories

## Consumer library — Settling an uncertain append by reading the leaf back

## Operator — Recovering a log after a crash

## Writer — Racing another writer for the same index

## Writer — Seeing a failure after its leaf was named

## Read-only caller — Opening a store without writing to it

## Read-only caller — Reading a log on read-only media

## Read-only caller — Reading a log that a crash left one leaf ahead of its pin

## Log operator — Opens a log store, including after a crash

**S4.** As a log operator, I want a store whose pinned leaves no longer rebuild to the pin to refuse to open and tell me the pin and the root the leaves give, so that a damaged leaf is never committed to the tree and I am never told more than the store can prove.

## Third-party verifier — Checks a leaf without lys

## Witness operator — Runs a witness anchor that observes other logs' checkpoints

**S3.** As a witness operator, I want each observation to cost only the leaves recorded since the previous one, so that the witness stays usable as its own log grows.

## Auditor — Reading a flight recorder's log without changing it

**S1.** As an auditor, I want a leaf store I open for reading to refuse every write so that reading the log never changes the evidence.

**S2.** As an auditor, I want a store left mid-append to be refused by name when I open it for reading so that I learn a repair is pending instead of performing one.

## Verifier — Trusting a leaf the log serves

## Developer — Building a reader on the leaf store

## Log inspector — Opens a log store to read it without changing it

**S5.** As a log inspector, I want a store I open read only to refuse every write, so that reading a store can never change it.

**S6.** As a log inspector, I want a read-only open of a store with an interrupted append to refuse and say that a repair is pending, so that I learn the store is past its pin without my open repairing it.

## Leaf store maintainer — Reads the file store's contract before relying on it or changing it

**S7.** As a leaf store maintainer, I want the file store's module doc to say what opening a store does and never does, so that I can rely on open never deleting a leftover temporary file.

## Estate operator — Runs Lys behind every agent and session

**S8.** As the operator of an estate where Lys runs behind every agent, I want each request, append and open to do its work once and scale with what it touches, so that Lys costs nothing it does not need to as history grows.
