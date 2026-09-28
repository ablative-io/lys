# Lys-Core — User Stories

## Card author — Landing work in lys

**S24.** As a card author landing work in lys, I want the gate to refuse an underscore-prefixed binding with a message saying why so that an unused warning is fixed at its cause rather than silenced.

## Test writer — Reading a test's fixtures

**S25.** As a test writer, I want a temporary directory's guard named and dropped by name so that I can see where the directory's life ends.

## Third-party verifier — Verifying lys artifacts

**S26.** As a third party verifying a lys artifact, I want every verification failure to keep returning the one uniform error so that the cleanup reveals nothing about which check failed.

## Consumer of lys-log-store — Implementing LeafStore

**S27.** As a consumer implementing lys-log-store's LeafStore, I want the trait left unchanged so that my implementation still compiles against the next release.

## Lead — Trusting the gate

**S28.** As the lead for lys, I want the rule shown to fire once for each binding position so that zero hits on the tree means the rule held and not that nothing was measured.

**S29.** As the lead for lys, I want the full suite to run the same tests before and after the card so that I know the cleanup changed no behaviour.
