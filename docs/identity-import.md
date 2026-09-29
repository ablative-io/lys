# Importing directory entries

`lys identity import document.json --json` sends one request to the local identity service. Install and upgrade create an independent owner-only credential at `<root>/keys/identity-loader.credential`; an existing file is preserved. The service provisions its person-owned account once the configured administrator has a directory record. The service records ordinary editor grants on `directory:apps` and `directory:agents`, delegated by that administrator to the account, with the administrator responsible. The loader has no special runtime permission. Revoking these grants or retiring the account stops it; restarting does not restore that authority.

The endpoint is `POST /api/identity/import` on an installed surface, or `POST /identity/import` on a headless service, authenticated with the loader's registrar bearer. The CLI accepts only numeric loopback addresses and never redirects or retries. The CLI reads `listen` and `surface_dir` from the installation’s `identity.json` before sending anything. `--root` and `--credential-file` select the installation and credential; `--address` overrides only the listener, retaining that installation’s route prefix. Never pass the bearer as a command argument. Credential refusals never print its bytes.

The JSON object accepts four arrays, processed in order: `apps`, `agents`, `root_grants`, `delegations`. Apps use the existing app registration request without `operation`; agents have `display_name`. Grants use the existing root/delegation request without `operation`, plus a document-local `name`. Operations derive from account, section, name and canonical content. Names must be unique within a section, and grant names across both grant sections. Existing `operation` members and unknown members are refused. Forward or unknown references are refused before mutation.

References in `holder`, `source`, `recipient`, and `responsible` may name `@self`, `@owner`, an earlier `@agent/<display_name>`, or an earlier `@grant/<name>`. Literal existing identifiers also work. A repeated document under the same account uses the original operation IDs. Changed content is a new operation, not an update to the old record.

An import **never approves an app or activates an agent**. Those remain a person's acts. New apps remain pending and new agents remain Registered. Root issuance remains administrator-only: a loader's `root_grants` entry is refused `RootAuthorityRefused`; the account never impersonates its owner. Application delegations need an existing source grant explicitly delegated to this account with sufficient pass-on authority. Initial loader grants are use-only and cannot supply that authority.

`lys identity import docs/examples/estate-seats.json --json` reads six seat display names from that checked-in JSON file and registers them without illustrative apps or grants. It does not read Cambium or import existing credential bindings.

`docs/examples/identity-import.json` registers Cambium, aion and six named agents using illustrative schemas. It does not assert these are either application's production schema. `identity-import-grants.template.json` demonstrates subsequent delegation after approval/activation and an explicit administrator grant; replace the source and identity identifiers. Do not load the template unchanged into a real directory.

A refusal stops the walk and preserves the original refusal name, fields, failed entry and completed prefix. The prefix may already be committed: the operation is not an atomic transaction. `--json` prints the complete received refusal and prefix and exits unsuccessfully. A 5xx, lost connection or incomplete answer is an unknown outcome; reconcile the named operation IDs before an explicit repeat. No automatic retry is made.

Authenticated entry refusals are durably recorded in the existing service-account log, without credential or request payload. The same account/operation/refusal outcome is recorded once. A later changed authority may allow that operation. Owners and administrators read these receipts in `GET /service-accounts` under the existing visibility rules. If recording the refusal fails, the response keeps the original refusal and adds `audit_failure` with a 503 status; it does not pretend the audit was saved.

## Stored compatibility

Existing person/agent events retain their v1 canonical bodies and signed envelopes. Service-account grant callers/holders and pass-on recipients use kind 3 and require grant-event v2. Service-account directory provenance uses authentication method 3, an explicit account identifier, and identity-event v2; the recorded owner login is provenance, not an OIDC sign-in. Verification pairs body and protected-envelope versions and rejects a downgrade. Service-account lifecycle remains in its existing log, separate from the people/agents directory.

On opening the permission engine, upgrade writes the new service-account subject into the grant schema while preserving existing resource and app definitions. The `service_account` definition also remains a grantable resource, with exactly one definition. Ordinary revocation, narrowing, recipient-kind restrictions and owner lifecycle checks still apply.

## Verification status

Source tests cover deterministic IDs, duplicate/ref invalidity, provenance and downgrade rejection, grant log reopen/repeat/revocation, credential refusal, missing grants, refusal audit reopen, older installation bootstrap and old-schema upgrade. The latest combined source is not yet compiled or gated. A private HTTP fixture tests the actual SpiceDb open/write protocol; it does not itself prove acceptance by a live SpiceDB. Heavy checks and live engine verification must run on Dean after the shared build hold is released.

## Connecting the message service

Use `lys identity install --message-service connection.json` for a new install,
or add `--message-service connection.json` to `lys identity upgrade --from ...`
when replacing an existing connection. The file contains the connection itself:
`url`, the explicit session cookie name in `cookie`, and `bindings` entries with
the message registry's `participant` and the corresponding Lys `identity`.
The cookie value and other credentials do not belong in this file.

The URL must use HTTPS or loopback HTTP. Participant and identity bindings must
be unambiguous; names are never used to infer identity. Missing cookies and
invalid bindings are refused before anything is stopped. Omitting the option
keeps the stored connection. Existing legacy `<service>_messages` configuration
is migrated to `message_service`, retaining its historical `<service>_session`
cookie name; new connection files must state that name explicitly. There is no
product-specific CLI alias. Message visibility remains the message service's
decision, with Lys additionally checking the explicit identity binding.
