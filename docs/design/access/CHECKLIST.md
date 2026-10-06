# Access — Checklist

## Grant mode and drafts

- [ ] **C601** — A grant carries one mode: outright, by_draft or by_two; the codec, events and snapshots version for it and old bytes stay readable (ACCESS-001 R1).
- [ ] **C602** — An app schema marks each action hot or deliberate; Lys refuses a by_draft or by_two grant on a hot action by name (ACCESS-001 R2).
- [ ] **C603** — A product records a held act as a draft naming the grant, the target and the request digest; one approval executes by_draft, two distinct approvers execute by_two; the product reads approved drafts of its kinds and records done or refused-on-execution (ACCESS-001 R3).
- [ ] **C604** — The Access screens show a grant's mode, the drafts held for a product, and who approved each (ACCESS-001 R4).

## The pass

- [ ] **C605** — The access token for an app carries a rights claim: holder, kind, responsible person, and for that audience only, each right's resource, actions, mode and grant id, roles expanded (ACCESS-002 R1).
- [ ] **C606** — The pass lifetime is a setting with a stated default; refresh issues a new pass from live grants; a revoked grant is absent from the next pass (ACCESS-002 R2).
- [ ] **C607** — The key that signs passes is published with its id and rotates without invalidating passes already issued within their lifetime (ACCESS-002 R3).

## lys-pass

- [ ] **C608** — lys-pass verifies a pass offline against the published key, audience and time, and evaluates a right by placement reach and restricted children (ACCESS-003 R1).
- [ ] **C609** — lys-pass asks Lys for a deliberate route and refuses "Lys could not be asked" when it cannot, never falling back to the pass; hands a by_draft act to Lys and runs approved drafts (ACCESS-003 R2).
- [ ] **C610** — The one refusal shape names resource, action, the grant needed and the Lys request URL; the conformance fixtures ship in the crate and every product gate runs them (ACCESS-003 R3).

## Roles and reach

- [ ] **C611** — An app schema names roles as action bundles; a grant may name a role and the pass expands it; a change that widens a role by an app waits for the administrator (ACCESS-004 R1).
- [ ] **C612** — A placement may be restricted: the parent's relations do not flow to it and only an explicit grant reaches it (ACCESS-004 R2).

## Machines

- [ ] **C613** — A machine is the fifth identity and grant holder, made at join, answering to the administrator who issued its code; old bytes stay readable (ACCESS-005 R1).
- [ ] **C614** — The Network screen shows machines as holders and their grants; a machine never holds a kept responsibility (ACCESS-005 R2).
