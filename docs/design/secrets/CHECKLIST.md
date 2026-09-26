# Secrets — Checklist

## The implementation brief

- [ ] **C1** — SECRETS-002 is a design-system brief with one numbered requirement, carrying acceptance criteria and file paths, for each part of the temporary key model: the handle and the proxy swap with its audit line; rotation under one handle; the token revolver as the first consumer; OAuth refresh at the proxy; the seat's own login put into its environment at spawn; sealed knowledge tagged in SpiceDB; the three revocation cases, including the cancellation rule for calls in flight; and leases counted by uses, time window and spend.
- [ ] **C2** — Every checklist item and user story the broker's implementation needs is recorded in this cluster and covered by a SECRETS-002 requirement.
- [ ] **C3** — The rendered markdown of this cluster matches its JSON.

## The broker's implementation

- [ ] **C4** — A seat's call with its handle goes through the door's proxy, which checks SpiceDB, swaps the handle for the real credential, forwards the call and writes one audit line naming the seat, the handle, the real account and the time; the credential never leaves the server.
- [ ] **C5** — The store keeps a set of real accounts behind one handle, the proxy takes the next in turn and logs which one served, and resting an account is a store change with no file copied to any machine.
- [ ] **C6** — The token revolver asks the broker for its next account instead of walking its own list.
- [ ] **C7** — The proxy refreshes an OAuth access token itself from the refresh token in the store, and the seat never sees the refresh token.
- [ ] **C8** — The door puts the seat's own login into its environment at spawn, read from the store, rotating at spawn, and records which token went to which seat.
- [ ] **C9** — Sealed records are kept encrypted in the store, tagged in SpiceDB with which identities may read them, read by name with a relation check and one audit line, and a key is used through the proxy, never read.
- [ ] **C10** — Each of the three revocation cases has its own behaviour: a dropped handle refuses every new call, the login token's seat is ended by its engine, and read sealed knowledge is disclosed and recorded, never claimed back.
- [ ] **C11** — The proxy has an explicit cancellation rule for calls already admitted when their handle is dropped, written down before it is built.
- [ ] **C12** — Everything handed out is a lease counted by uses, a time window and a spend cap: the window in the signed delegation, uses and spend counted by the door, a one-use grant never spent twice.
- [ ] **C13** — Every point the statement leaves open is recorded open in the row it touches and is not decided there.
- [ ] **C14** — No broker row depends on any one engine: an engine without the broker reads its own pool file as it does today.

## Authority and screen conformance, 23 September amendment

- [ ] **C15** — Real secret ownership or a valid human-rooted delegation permitting re-lending establishes affirmative may-lend; mere use and display labels do not. Applicable recipient policy is checked separately, including people-only refusal.
- [ ] **C16** — Personal, team and organisation secret boundaries are enforced in listing, metadata, read, use and lending; knowing another identity's record ID grants nothing.
- [ ] **C17** — Every derived handle stays inside its live ancestry, including shared use/spend budgets and expiry; revoking its source does not revoke an independently authorised sibling.
- [ ] **C18** — Issuance, account selection, retry and revocation preserve exact provenance and current authority; the screen distinguishes local refusal from unconfirmed provider action.

## The broker's build rows, SECRETS-003

- [ ] **C19** — docs/design/secrets/BASELINE.md records, by repository, pinned commit, file and line, the revolver's next-account call site and every consumer of the account pool file.
- [ ] **C20** — docs/design/secrets/BASELINE.md records every credential path into a seat today, by file and line, each classed as exactly one of: the login exception, a proxied credential, or neither.
- [ ] **C21** — docs/design/secrets/BASELINE.md records, by file and line, what lys/delegation/v1 and lys/sealed-envelope/v1 can and cannot carry for a handle, a lease and a sealed record.
- [ ] **C22** — docs/design/secrets/CONTRACT.md states the invariant, the key custody, the handle with how it is stored and how its presenter is authenticated, the lease with its atomic use step and retry outcome, the cancellation rule, the SpiceDB relations, the sealed-record binding and the audit line fields, and is accepted before any code row starts.
- [ ] **C23** — docs/design/secrets/reports/SECRETS-003-adversarial-review.md records every attack tried in the six named classes, by a reviewer who did not write the contract and is a different commit author, and the contract clause defeating each.
- [ ] **C24** — The store, the redacting type, handles and leases in crates/lys-secrets deliver SECRETS-002 R9 and the issuance criteria of R1, with the store key kept out of the store directory, opening without it refused by name, swapped ciphertext refused, and every counted leg of their row passing; for SEC_USE_NOT_LEND and SEC_PEOPLE_ONLY they deliver the library-level decision, and the screen and API legs are recorded as not delivered here.
- [ ] **C25** — The proxy in crates/lys-secrets delivers SECRETS-002 R1's proxy criteria, R2, R8 and R7's handle case, with every counted leg of its row passing; for SEC_REVOKE_STATES it delivers the library-level decision, and the API and screen legs are recorded as not delivered here.
- [ ] **C26** — OAuth refresh at the proxy in crates/lys-secrets delivers SECRETS-002 R4, with every counted leg of its row passing.
- [ ] **C27** — The seat's own login at spawn from crates/lys-secrets delivers SECRETS-002 R5 and R7's login-token case, with every counted leg of its row passing.
- [ ] **C28** — Sealed records in crates/lys-secrets deliver SECRETS-002 R6 and R7's sealed-knowledge case, with every counted leg of their row passing.
- [ ] **C29** — The broker's next-account answer in crates/lys-secrets delivers SECRETS-002 R3's broker side, with every counted leg of its row passing, and names no engine file.
