# Directory — User Stories

## Responsible person — Signs in and provisions agents under their own authority

**S1.** As the responsible person, I want every grant my agent holds to trace back to me, so that withdrawing my authority stops everything derived from it.

**S4.** As the responsible person, I want to register an agent under my name before it ever runs, with every change to it signed, so that who created it and who answers for it is never reconstructed after the fact.

**S5.** As a person signing in, I want Google and GitHub to resolve to the one me, so that adding a provider never splits me into two people or merges me with someone who shares my email.

**S84.** As the responsible person, I want my agent's start command given only after its checks pass, so that I never start an agent that is not active, reviewed, allowed on the machine, credentialed and able to reach what it needs.

**S85.** As the responsible person, I want each failed check named in words, so that I know what to fix before a command can be given.

**S86.** As the responsible person, I want my agent shown as running only when its own signed report names the command I was given, so that copying a command is never mistaken for a start.

**S87.** As the responsible person, I want an unanswered start shown as unconfirmed with its request standing, so that I do not ask elsewhere and start the agent twice.

**S88.** As the responsible person, I want to withdraw a start request I no longer want, so that the record says the request no longer stands without claiming the agent did not start.

**S93.** As the responsible person, I want the command I am given to run the executable in the working directory that were reviewed, so that no start request can change what runs.

**S94.** As the responsible person, I want starting my agent again while it runs to become a second session of the same agent, so that starting never makes another agent and never changes the one I registered.

## Reviewer — Reviews a brief before any of its rows is dispatched

**S2.** As the reviewer, I want each open identity row as a design-system brief with numbered requirements and criteria, so that rows can be dispatched to the loop one at a time and reviewed against their criteria.

**S90.** As a reviewer, I want a test that reads the command line and the clipboard and finds no credential value, so that giving a command never discloses a secret.

**S91.** As a reviewer, I want a test that fails if any start path spawns a process, so that lys stays a product that checks, gives and records and never runs an agent.

## Operator — Installs and runs the standalone identity product

**S3.** As the operator, I want to install the identity product's dependencies on one PostgreSQL database whose host I choose, and restart or restore it without losing anyone, so that the product stands alone without Cambium or Manifold.

**S6.** As the operator, I want the directory's screens to show every refusal, pending audit and outage as it is, so that I never act on a completed state that did not happen.

**S89.** As the operator, I want a kept launch record for every command given, so that I can give the same start again without looking inside a running process.

## Verifier — Checks a recorded identity change without the operator's cooperation

**S7.** As a verifier, I want to check a recorded identity change against a checkpoint and key with standard tooling, so that the directory's history does not rest on the operator's word.

## Grant holder and reviewer — Exercises or delegates current authority and verifies its exact origin

**S8.** As a responsible person, I want to give my agent a bounded part of my authority and revoke it, so the same action is allowed before revocation and refused after it.

**S9.** As a person or agent with use-only permission, I want the service to explain why I cannot pass it on; with explicit pass-on rights I want the same permitted operation available through UI and tools.

**S10.** As an auditor, I want grant changes and retries tied to one durable signed operation, so restart or an uncertain reply cannot invent, lose or duplicate authority.

**S11.** As a person granting temporary access, I want its end date and ancestor restrictions enforced, so role changes or reinstatement cannot silently extend it.

**S12.** As an authorised reviewer, I want to ask why this identity can act and who can reach a resource using the same current decision, without exposing records I may not inspect.

## Person without the right to start — Opens an agent file that is not theirs to start

**S92.** As a person who is neither the agent's responsible person nor a directory administrator, I want a refusal naming the agent and the right I lack, so that I know why there is no working Start for me.
