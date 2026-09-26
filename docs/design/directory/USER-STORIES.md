# Directory — User Stories

## Responsible person — Signs in and provisions agents under their own authority

**S1.** As the responsible person, I want every grant my agent holds to trace back to me, so that withdrawing my authority stops everything derived from it.

**S4.** As the responsible person, I want to register an agent under my name before it ever runs, with every change to it signed, so that who created it and who answers for it is never reconstructed after the fact.

**S5.** As a person signing in, I want Google and GitHub to resolve to the one me, so that adding a provider never splits me into two people or merges me with someone who shares my email.

**S15.** As a person signing in, I want my sign-in, and a refusal of it, to leave a receipt signed by the service and never by me, so that a claim about me is never settable by me.

## Reviewer — Reviews a brief before any of its rows is dispatched

**S2.** As the reviewer, I want each open identity row as a design-system brief with numbered requirements and criteria, so that rows can be dispatched to the loop one at a time and reviewed against their criteria.

## Operator — Installs and runs the standalone identity product

**S3.** As the operator, I want to install the identity product's dependencies on one PostgreSQL database whose host I choose, and restart or restore it without losing anyone, so that the product stands alone without Cambium or Manifold.

**S6.** As the operator, I want the directory's screens to show every refusal, pending audit and outage as it is, so that I never act on a completed state that did not happen.

**S14.** As the operator of a development install, I want its receipts to be test receipts that verify only with the test key I hold and never as real, so that I can prove the whole path end to end without performing an act reserved for Tom.

## Verifier — Checks a recorded identity change without the operator's cooperation

**S7.** As a verifier, I want to check a recorded identity change against a checkpoint and key with standard tooling, so that the directory's history does not rest on the operator's word.

**S13.** As a verifier, I want to check a receipt with the lys CLI alone, given the log and the service's public key, and have a receipt whose payload commitment does not match the record it points at refused, so that the directory's history never rests on the running service's word.

## Grant holder and reviewer — Exercises or delegates current authority and verifies its exact origin

**S8.** As a responsible person, I want to give my agent a bounded part of my authority and revoke it, so the same action is allowed before revocation and refused after it.

**S9.** As a person or agent with use-only permission, I want the service to explain why I cannot pass it on; with explicit pass-on rights I want the same permitted operation available through UI and tools.

**S10.** As an auditor, I want grant changes and retries tied to one durable signed operation, so restart or an uncertain reply cannot invent, lose or duplicate authority.

**S11.** As a person granting temporary access, I want its end date and ancestor restrictions enforced, so role changes or reinstatement cannot silently extend it.

**S12.** As an authorised reviewer, I want to ask why this identity can act and who can reach a resource using the same current decision, without exposing records I may not inspect.
