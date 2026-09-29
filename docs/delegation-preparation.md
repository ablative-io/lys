# Delegation preparation, without issuance

The server library's `delegation` module contains pure binding checks and consent
plans. No route uses it, no credential is minted, no store is opened, and none
of its record types implements a wire or signed-log encoding. This is a boundary
to review and test before a durable consent flow is enabled.

`authenticate` rejects duplicate Authorization headers, mixed Basic/form client
authentication and duplicate decoded form members before inspecting the supplied
authority. Basic components are form-decoded exactly once after Base64 decoding,
as specified by [RFC 6749 section 2.3.1](https://www.rfc-editor.org/rfc/rfc6749#section-2.3.1).
Malformed escapes refuse before lookup; form inputs are already decoded and must
preserve all pairs. A configured OpenID client
never resolves as an approved-app client, even with identical IDs and credentials.
The supplied app must come from the freshly settled apps log. Its snapshot binds
registration, approval and current-version operations, client identity/credential
revision and the exact redirect set. Retirement or any changed coordinate refuses
the older binding. Existing provider behavior is untouched.

`Current` contains trusted facts for a future service resolver to supply: the
authenticated person and public session ID, subject activity, live session end,
current app binding and server time. It must never be populated from claims in
an app's request. The pure functions do not authenticate those facts themselves.

Decision planning authenticates the same owner/session and compares an existing
operation's exact command before requiring its already-consumed nonce. An exact
retry returns `Replay`, containing the original historical record only. It never
returns another code/token, appends again or proves current consent. `standing`
separately refuses withdrawn/declined consent, stale app binding, expired session
or inactive subject. A changed retry body refuses, including a changed proof.

Revocation requires a distinct nonce bound to the current owner/session, exact
consent and revoke action. A pending-decision proof cannot revoke. The person may
revoke from a later authenticated session, even after the app binding has changed.
The same authenticated revocation operation reads back its original receipt.

These are pure plans, not a transactional nonce/consent store. A future caller
must look up both operation and consent, compare and append atomically with nonce
consumption, and prove durable readback before reporting success. It must never
treat an absent lookup after an uncertain write as permission to recreate consent.
Missing pending state after restart refuses; code/token exchange has no retry or
issuance implementation here. Canonical signed encoding/version, origin/CSRF
enforcement, durable operation uniqueness and live-session effect provenance all
remain prerequisites to enabling any route or minting any credential.
