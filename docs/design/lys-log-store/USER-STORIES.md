# Lys-Log-Store — User Stories

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
