# Directory — User Stories

## Responsible person — Signs in and provisions agents under their own authority

**S1.** As the responsible person, I want every grant my agent holds to trace back to me, so that withdrawing my authority stops everything derived from it.

**S4.** As the responsible person, I want to register an agent under my name before it ever runs, with every change to it signed, so that who created it and who answers for it is never reconstructed after the fact.

**S5.** As a person signing in, I want Google and GitHub to resolve to the one me, so that adding a provider never splits me into two people or merges me with someone who shares my email.

## Reviewer — Reviews a brief before any of its rows is dispatched

**S2.** As the reviewer, I want each open identity row as a design-system brief with numbered requirements and criteria, so that rows can be dispatched to the loop one at a time and reviewed against their criteria.

## Operator — Installs and runs the standalone identity product

**S3.** As the operator, I want to install the identity product's dependencies on one PostgreSQL database whose host I choose, and restart or restore it without losing anyone, so that the product stands alone without Cambium or Manifold.

**S6.** As the operator, I want the directory's screens to show every refusal, pending audit and outage as it is, so that I never act on a completed state that did not happen.

## Verifier — Checks a recorded identity change without the operator's cooperation

**S7.** As a verifier, I want to check a recorded identity change against a checkpoint and key with standard tooling, so that the directory's history does not rest on the operator's word.

## Grant holder and reviewer — Exercises or delegates current authority and verifies its exact origin

**S8.** As a responsible person, I want to give my agent a bounded part of my authority and revoke it, so the same action is allowed before revocation and refused after it.

**S9.** As a person or agent with use-only permission, I want the service to explain why I cannot pass it on; with explicit pass-on rights I want the same permitted operation available through UI and tools.

**S10.** As an auditor, I want grant changes and retries tied to one durable signed operation, so restart or an uncertain reply cannot invent, lose or duplicate authority.

**S11.** As a person granting temporary access, I want its end date and ancestor restrictions enforced, so role changes or reinstatement cannot silently extend it.

**S12.** As an authorised reviewer, I want to ask why this identity can act and who can reach a resource using the same current decision, without exposing records I may not inspect.

## Person a role holder answers to — Keeps each holding of a role on a version they chose

**S71.** As the person an agent answers to, I want an edit to its role to leave the agent on the version it holds so that its permissions never change without an act I can see.

**S73.** As the person a holder answers to or an owner of its role's project, I want to see the date each holding will move and who can stop it, and to change one holding's policy, so that no move surprises me.

**S74.** As the person a provisional holder answers to or an owner of its role's project, I want renewing its holding to be a recorded act of one of us so that it is never renewed quietly and lapses when nobody renews it.

## Mover of a holder — Moves a holder to a newer version of its role

**S72.** As the person a holder answers to or an owner of the project its role is defined in, I want to see what a move adds and removes, and choose when it takes effect, before I take it so that I move a holder knowing what it gains and loses.

## Reviewer of a role's history — Checks how each holder came to its version

**S75.** As a reviewer, I want every role act recorded with who made it and in which capacity so that I can verify how a holder came to the version it is on.

## Owner of a project — Defines the roles of a project and assigns them

**S80.** As an owner of a project, I want to make and edit its roles and assign them to agents without any existing holder changing so that a role can improve without silently changing what its holders may do.
