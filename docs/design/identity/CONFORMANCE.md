# Identity: conformance to the mock-up

Reference: `docs/design/identity/mockup/index.v5.html` (v4 kept beside it as the prior).

This table is the conformance test. Each row below is one behaviour the mock-up shows. The build conforms when every row marked **test** passes as an acceptance check in the brief named against it. Rows marked **proposed** or **open** are shown in the mock-up as undecided; they become tests only when decided.

Brief column: **exists** = an existing brief row covers it; **amend** = an existing brief needs a change; **new** = no brief yet. Owner is the seat writing that brief (file walls, 19:05).

The mock-up's sample data, simulated confirmations and "not kept" edits are not behaviours. Only the rules below are.

## 1. Signing in and "You" (#/me)

| # | Behaviour | Kind | Brief | Owner |
|---|---|---|---|---|
| 1.1 | A person signs in with one of several linked providers; each link is listed as a sign-in identity. | test | IDENTITY-001 R03 (exists) | Waffles |
| 1.2 | Sign-in identities are never lent to or held by an agent. | test | IDENTITY-001 (amend: state it) | Waffles |
| 1.3 | Service accounts a person may use are a separate list, each marked passable or use-only. | test | DIRECTORY (new row) | Chippy |
| 1.4 | "What you hold" shows each grant, its source, and whether it may be passed on. | test | DIRECTORY-001 R3 (amend) | Chippy |
| 1.5 | "You" shows the signed-in person's own agents, grants and personal secrets. This is personal scope, not global hiding: a directory administrator may lawfully see others through the directory screens. | test | DIRECTORY (amend) | Chippy |

## 2. Giving an agent access (delegation)

| # | Behaviour | Kind | Brief | Owner |
|---|---|---|---|---|
| 2.1 | A person can give an agent only a relation at or below one they hold. | test | DIRECTORY-001 R3 (amend) | Chippy |
| 2.2 | Only a grant marked may-pass-on can be delegated; may-pass-on is an affirmative fact on the grant, never "no prohibition found". | test | DIRECTORY (amend) | Chippy |
| 2.3 | The form shows the source grant, the actions it allows, may-pass-on, and that the new grant ends no later than its source. | test | DIRECTORY (amend) | Chippy |
| 2.4 | Everything the person cannot give is listed with its reason (use-only, above what you hold, lent to you, sign-in identity). | test | DIRECTORY (new row) | Chippy |
| 2.5 | Withdrawing a grant withdraws everything derived from it. | test | DIRECTORY-001 R3 (exists) | Chippy |
| 2.6 | Inherited expiry is enforced, not only displayed (leases, statement of 22 Sep). | test | DIRECTORY (new row) | Chippy |

## 3. People and agents (#/people, agent file)

| # | Behaviour | Kind | Brief | Owner |
|---|---|---|---|---|
| 3.1 | Every agent answers to a person; an agent whose person is retired is flagged as needing a new one. | test | LIFECYCLE-STATES (to brief: new) | Waffles |
| 3.2 | Identity states are registered, active, suspended, retired; state is authority only and says nothing about running. | test | LIFECYCLE-STATES (to brief: new) | Waffles |
| 3.3 | Suspending holds the agent's virtual credentials. Reinstating releases only those whose grant ancestry and lease still stand when rechecked; it never resurrects a revoked or expired grant. | test | SECRETS-002 R7 (amend) | Chippy |
| 3.4 | Emergency stop revokes tokens at once and asks for sessions to end; sessions show unconfirmed until reported. | test | LIFECYCLE (new) | Waffles |

## 4. Roles (#/roles)

| # | Behaviour | Kind | Brief | Owner |
|---|---|---|---|---|
| 4.1 | A role is a job (responsibilities, goals, practice), a profile, and grant templates. | proposed | ROLES (new) | Waffles |
| 4.2 | Editing a role makes a new version; no holder changes by itself. | test (once ADR lands) | ADR + ROLES (new) | Waffles |
| 4.3 | Moving a holder to a newer version is a deliberate act that shows what changes and is recorded with who did it. | test (once ADR lands) | ROLES (new) | Waffles |
| 4.4 | The policy "move at next renewal" is explicit: each holder shows the date it will move. | test (once ADR lands) | ROLES (new) | Waffles |
| 4.5 | A provisional holding has an end date on its grant; it lapses and is never renewed quietly; a version change never extends it. | test (once ADR lands) | ROLES + DIRECTORY | Waffles, Chippy |
| 4.6 | Which policy is the default. | open | ADR | Waffles |
| 4.7 | Role as live group or template copied at grant time. | open | ADR | Waffles |

## 5. Starting an agent (agent file, Start)

| # | Behaviour | Kind | Brief | Owner |
|---|---|---|---|---|
| 5.1 | This product does not run agents. It checks, gives a command, and records. | test (once ADR lands) | ADR + START (new) | Waffles |
| 5.2 | Checks before a command is given: agent active; profile version reviewed (from a record); machine allowed for the role; valid virtual credentials; machine may reach what the profile needs (from its egress list). A failed check names itself and no command is given. | test | START (new) | Waffles |
| 5.3 | No credential value is on the command line or the clipboard (settled). How the command proves itself, such as a one-time code for one machine for ten minutes, is the mock-up's illustration and stays proposed until the launch design chooses and enforces it. | test / proposed | START (new), SECRETS-002 R5 (amend) | Waffles, Chippy |
| 5.4 | A launch record is kept (machine, executable, working directory, profile version, credential ids), so a start can be given again without inspecting a running process. | test | START (new); fixes Cambium defect "Hand-started workers have no launch record" | Waffles |
| 5.5 | The agent is shown as running only when its signed report names that launch record; copying the command is not a start. | test | START (new) | Waffles |
| 5.6 | No report means unconfirmed, never "not started"; the request stands and asking elsewhere is warned against. | test | START (new) | Waffles |
| 5.7 | Terminal view, sandbox, VM or container runners. | proposed, separate products | none | — |

## 6. Certificate (agent file, Certificate)

| # | Behaviour | Kind | Brief | Owner |
|---|---|---|---|---|
| 6.1 | Each active agent has an X.509 capability certificate over a key it proved it holds (PKCS#10), issued for its person. | test | IDENTITY (new row, on lys-core capability certificates) | Archie |
| 6.2 | The certificate's claims are what held at issuance: id, role and version, person, grants and profile version. They are never presented as live grants; the current answer comes from enforcement. | test | same | Archie |
| 6.3 | When a certificate is withdrawn, it is withdrawn in the log, never edited (test). Whether every grant or role-version change reissues one is an undecided protocol choice. | test / open | same | Archie |
| 6.4 | Issuance is entered in a transparency log; anyone verifies the certificate and its inclusion offline with standard tools. | test | same, on lys-log-store | Archie |
| 6.5 | Start, what the agent was given, and role moves are signed statements checked with `lys verify --attestation`. | test | CONTEXT (amend: step 4 record) | Archie |
| 6.6 | Production keys and receipts outside tests stay Tom's acts. | constraint | all | — |

## 7. Secrets (#/secrets)

| # | Behaviour | Kind | Brief | Owner |
|---|---|---|---|---|
| 7.1 | A credential value goes in once and is never shown again, to anyone. This concerns credential values, not sealed memories, whose authorised disclosure is allowed. | test | SECRETS-002 R1 (exists) | Chippy |
| 7.2 | Agents use secrets only through virtual credentials: one holder, one person acted for, limits, lease. | test | SECRETS-002 R1, R9 (exists) | Chippy |
| 7.3 | A person issues from a secret only with an affirmative may-lend: as its owner, or through a valid human-rooted delegation that permits re-lending. Everything else is listed with its reason. | test | SECRETS-002 (amend) | Chippy |
| 7.4 | Credentials go only to the issuer's own active agents. This is the mock-up's sample and a policy choice, not yet ruled. | open | SECRETS-002 (amend) | Chippy |
| 7.5 | Revoking a lease is two facts: issuing stops at once; the system behind confirms separately; until then it shows pending. | test | SECRETS-002 R7, R8 (exists) | Chippy |
| 7.6 | Who may revoke a lease: the person acted for may. Whether a secret's owner may revoke every credential derived from it is a policy choice, not conferred by ownership alone. | test / open | SECRETS-002 (amend) | Chippy |
| 7.7 | Refresh, rotation and renewal are shown and recorded separately; rotation changes nothing for holders. | test | SECRETS-002 R2, R4 (exists) | Chippy |
| 7.8 | Scope filter: organisation, team, mine. | test | SECRETS-002 (amend) | Chippy |

## 8. Access, graph and network

| # | Behaviour | Kind | Brief | Owner |
|---|---|---|---|---|
| 8.1 | "Can X do this?" answers yes with the path to a person, or no with the named reason and the model version used. | test | DIRECTORY-001 R3 (exists) | Chippy |
| 8.2 | "Who can reach this?" lists everyone with the path, from the same answers. | test | DIRECTORY (amend) | Chippy |
| 8.3 | The graph draws the same answers as Access; it never computes its own. | test | DIRECTORY (new row) | Archie |
| 8.4 | Every grant shows last used, its source and window; "not seen" is never shown as "never used". | test | DIRECTORY (amend) | Chippy |
| 8.5 | The network view: machines, runtime, reporting state, where each role may run, egress. | proposed | none yet | — |

## 9. Shell, help and assistant

| # | Behaviour | Kind | Brief | Owner |
|---|---|---|---|---|
| 9.1 | Aion shell conformance: thin expandable rail, dock left or right, keyboard navigation, deep links for every screen and tab. | test | SHELL (new) | Archie |
| 9.2 | Help overlay numbers what is on screen; dismissing it never presses what is underneath; Escape exits and focus returns. | test | SHELL (new) | Archie |
| 9.3 | Every clickable element is reachable and usable by keyboard. | test | SHELL (new) | Archie |
| 9.4 | The assistant sees the screen only when the person opts in, and acts only through its own grants and the same requests. | proposed | none yet | — |

## Build order (for the briefs)

1. IDENTITY-001 as it stands: sign-in, directory, two providers (1.1, 1.2).
2. Directory grants and delegation (1.3 to 2.6, 8.1, 8.2, 8.4).
3. Lifecycle and the start command with launch records (3.1 to 3.4, 5.1 to 5.6).
4. Certificate and signed statements (6.1 to 6.5).
5. Secrets broker amendments (7.1 to 7.8).
6. Roles and versions, after the ADR (4.x).
7. Shell, help, graph (8.3, 9.1 to 9.3).

Waffles' first end-to-end path is steps 1 to 3 plus revoke: sign in, register, grant, start command, running, revoke, refused.
