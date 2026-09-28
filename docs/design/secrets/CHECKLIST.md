# Secrets — Checklist

## The implementation brief

- [ ] **C1** — SECRETS-002 is a design-system brief with one numbered requirement, carrying acceptance criteria and file paths, for each part of the temporary key model: the handle and the proxy swap with its audit line; rotation under one handle; the token revolver as the first consumer; OAuth refresh at the proxy; the seat's own login put into its environment at spawn; sealed knowledge tagged in SpiceDB; the three revocation cases, including the cancellation rule for calls in flight; and leases counted by uses, time window and spend.
- [ ] **C2** — Every checklist item and user story the broker's implementation needs is recorded in this cluster and covered by a requirement of a SECRETS brief.
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

## Who may revoke a lease, and the scoped secrets list: CONFORMANCE rows 7.6 and 7.8

- [ ] **C30** — The person acted for under a lease may revoke it at any time: the revoke stops issuing at once, and GET /leases/{lease_id} reads upstream pending until the system behind confirms, with no timer confirming it (CONFORMANCE 7.6).
- [ ] **C31** — A revoke by anyone but the person acted for is refused: with the not-found refusal naming nothing when the caller cannot discover the lease, which only its person acted for and its holder can; for the lease's holder with a refusal naming the lease and relinquish; and with a refusal naming the lease for any other caller the seam lets discover it.
- [ ] **C32** — A lease's holder may relinquish it, recorded as a relinquish and not as a revoke, which stops issuing at once and reads upstream pending until the system behind confirms.
- [ ] **C33** — Whether a secret's owner may revoke every credential derived from it is recorded as a proposed ADR cited from this cluster's decisions, and no code and no test grants it.
- [ ] **C34** — The secrets list takes one scope from organisation, team and mine, applied at the server; with no scope it answers every secret the caller may see; any other scope is refused by name (CONFORMANCE 7.8).
- [ ] **C35** — The signed-in person's team ids are read from the group claims on their token, one group per team, through one function.
- [ ] **C36** — Every secret visibility and lease revoke check is a call on one seam, answering from the record's scope, team and owner fields until the step-2 SpiceDB evaluator exists, and asking SpiceDB through it once it does.
- [ ] **C37** — A revoke by the person acted for, or a relinquish by the holder, of a lease that has already ended is refused by name as already ended, carrying how, when and by whom it first ended, and records nothing; the first end record stays the only one (CONFORMANCE 7.6).
- [ ] **C38** — The secrets list is a person's view: an agent that asks for it is refused by name, the refusal naming the agent and saying that agents reach secrets only through their virtual credentials, with no list, not even an empty one, and nothing recorded (CONFORMANCE 7.8).
