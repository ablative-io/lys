# Grant bindings: compatibility matrix and upgrade path (DIRECTORY-089 R2)

A pass's grant binding is a separate signed JWS (`typ: lys-grant-binding+jwt`)
answered beside the pass. The pass's own bytes, its `JWT` header and its claims
(`lys_pass::Claims`) are unchanged, so every pass issued before bindings
existed, and every pass issued without asking for one, verifies exactly as it
did. No current pass carries a required revision.

## How a binding is obtained

| Producer | Request | Answer |
|---|---|---|
| Lys's own token route | `grant_binding=1` on `POST /oauth/token` (code or refresh) | `grant_binding` beside `access_token` |
| A registry-owned producer | `POST /grants/bindings` with the pass's SHA-256, issuer, holder, audience, lifetime and grants | `{grant_binding, revision}` |

Without `grant_binding` the token answer is byte-for-byte the pre-binding answer.
Any other version is refused `grant_binding_unsupported` before the code is spent.

## Matrix

| Client (lys-pass) | Server (Lys) | Asks for a binding | Outcome |
|---|---|---|---|
| old | old | no | pass accepted, as before |
| old | new | no | pass accepted, as before; no `grant_binding` member is sent, so the old `deny_unknown_fields` token answer still parses |
| new, binding optional (`fetch_token`) | old or new | no | pass accepted, as before |
| new, binding required (`fetch_bound_token`) | new | yes | pass and binding accepted; admission waits for the grant stream to reach `revision` |
| new, binding required | old | yes (form member ignored by the old server) | refused `grant_binding_required` by the client: never admitted at revision zero |
| new, binding required | new, grants moved or reading degraded | yes | refused `grant_binding_revision_moved` / `grant_binding_degraded`; nothing issued |
| any | new, binding of another pass, holder, issuer, audience, lifetime or log | — | refused `grant_binding_mismatch` / `grant_binding_log_mismatch` by `VerifiedBinding` |
| any | new, dependency path empty, cyclic, or not exactly the pass's grants | — | refused `grant_binding_ancestry_refused` |
| new | binding version above 1 | — | refused `grant_binding_unsupported` |

## Upgrade path

1. Upgrade Lys. Nothing changes for any existing client: bindings are produced
   only when asked for.
2. Upgrade each product's lys-pass. `fetch_token` and `refresh_token` keep
   their behaviour; the product chooses the moment it switches.
3. The product opens its grant stream (`POST /grants/changes`), keeps a
   `MembershipIndex` for the served log, and only then switches to
   `fetch_bound_token` / `refresh_bound_token`. From that moment a pass
   without a binding is refused `grant_binding_required`.
4. Enforcement — admitting only while the stream is ready, at or past the
   binding's revision, with no revoked grant on a right's path — is the
   consumer's own admission brief (liminal ACCESS-005). Lys's batch
   `at_least` fence remains the server-side comparison.

Bindings are bounded by the pass's own bounds: one dependency per grant the
pass's rights name (bounded by `provider.rights_bytes` when set) and each
grant's existing checked ancestry. No stored token, session or grant shape
changes; the only new stored item is the grant log's identity file, recorded
once beside the grant log directory on first use.
