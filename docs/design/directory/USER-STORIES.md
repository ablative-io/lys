# Directory — User Stories

## Responsible person — Relies on an agent's certificate staying ended once it is revoked

**S1.** As the responsible person, I want every grant my agent holds to trace back to me, so that withdrawing my authority stops everything derived from it.

**S4.** As the responsible person, I want to register an agent under my name before it ever runs, with every change to it signed, so that who created it and who answers for it is never reconstructed after the fact.

**S5.** As a person signing in, I want Google and GitHub to resolve to the one me, so that adding a provider never splits me into two people or merges me with someone who shares my email.

**S33.** As a responsible person, I want my agent's certificate revocable only by its issuing authority's key, so that nobody else can end it and nothing can quietly bring it back.

**S22.** As the responsible person, I want every session my agent runs to present the one enduring agent with a credential of its own, so that starting another session never leaves me a second agent to govern.

**S23.** As the responsible person, I want an agent I registered before it ever ran to show no session credential, so that I can tell an agent that has never run from one that has.

**S77.** As the responsible person, I want an agent whose permission I withdraw in the middle of a task to be refused on its next call, so that the withdrawal takes effect at once and not when the task ends.

**S81.** As a responsible person, I want each session of my agent kept as its own record under that one agent, so that starting another session never makes another agent.

**S109.** As the responsible person, I want my agent's start command given only after its checks pass, so that I never start an agent that is not active, reviewed, allowed on the machine, credentialed and able to reach what it needs.

**S99.** As the responsible person, I want each failed check named in words, so that I know what to fix before a command can be given.

**S100.** As the responsible person, I want my agent shown as running only when its own signed report names the command I was given, so that copying a command is never mistaken for a start.

**S101.** As the responsible person, I want an unanswered start shown as unconfirmed with its request standing, so that I do not ask elsewhere and start the agent twice.

**S102.** As the responsible person, I want to withdraw a start request I no longer want, so that the record says the request no longer stands without claiming the agent did not start.

**S107.** As the responsible person, I want the command I am given to run the executable in the working directory that were reviewed, so that no start request can change what runs.

**S108.** As the responsible person, I want starting my agent again while it runs to become a second session of the same agent, so that starting never makes another agent and never changes the one I registered.

**S133.** As a responsible person, I want every screen and tab to have its own address, so that a link I send opens exactly the view I was looking at.

## Reviewer — Reviews a brief before any of its rows is dispatched

**S2.** As the reviewer, I want each open identity row as a design-system brief with numbered requirements and criteria, so that rows can be dispatched to the loop one at a time and reviewed against their criteria.

**S104.** As a reviewer, I want a test that reads the command line and the clipboard and finds no credential value, so that giving a command never discloses a secret.

**S105.** As a reviewer, I want a test that fails if any start path spawns a process, so that lys stays a product that checks, gives and records and never runs an agent.

## Operator — Installs and runs the standalone identity product

**S3.** As the operator, I want to install the identity product's dependencies on one PostgreSQL database whose host I choose, and restart or restore it without losing anyone, so that the product stands alone without Cambium or Manifold.

**S6.** As the operator, I want the directory's screens to show every refusal, pending audit and outage as it is, so that I never act on a completed state that did not happen.

**S52.** As an operator, I want every agent whose person is retired to show that it needs a new person, so that no agent is left answering to a retired person without anyone seeing it.

**S79.** As the operator, I want a permission check the permission projection has not yet caught up with to be refused by name, naming the grant, while unrelated checks keep being answered, so that a lagging projection never admits a call and never stops unrelated work.

**S82.** As the operator, I want to stop a session on the directory, so that its credential is refused even when the agent crashed and never reported its end.

**S83.** As the operator, I want to list an agent's sessions with their states, so that I can see which of its session credentials are still accepted.

**S103.** As the operator, I want a kept launch record for every command given, so that I can give the same start again without looking inside a running process.

**S111.** As the operator, I want the exact release installed on a node I name and shown to me as a staging install, so that I can accept the product standalone before anything is cut over.

**S165.** As an operator, I want Lys to start agents with their credentials, tell apps where it is and sign people out of every app, with no app needing to run, so that Lys and each app work on their own and together.

## Verifier — Checks a recorded identity change without the operator's cooperation

**S7.** As a verifier, I want to check a recorded identity change against a checkpoint and key with standard tooling, so that the directory's history does not rest on the operator's word.

**S53.** As a verifier, I want an identity's state to move only along the lifecycle table and to say nothing about running, so that a state is read as authority and never mistaken for whether a process is alive.

**S113.** As a verifier, I want to read from an agent's certificate what it was granted at issuance and have a holder, a grant or an instant outside that refused, so that a signed claim is never taken as checked when nothing checked it.

**S114.** As a stranger verifying lys artifacts, I want the capability claim format specified, attacked and ratified before anything is signed under it, with every shipped format left byte-identical, so that no historical verification breaks.

**S162.** As a reader of a receipt, I want the checkpoint it is proved against to be signed by the service, so that a proof of inclusion tells me the act is in the service's log and not in a tree somebody built around it.

## Grant holder and reviewer — Exercises or delegates current authority and verifies its exact origin

**S8.** As a responsible person, I want to give my agent a bounded part of my authority and revoke it, so the same action is allowed before revocation and refused after it.

**S9.** As a person or agent with use-only permission, I want the service to explain why I cannot pass it on; with explicit pass-on rights I want the same permitted operation available through UI and tools.

**S10.** As an auditor, I want grant changes and retries tied to one durable signed operation, so restart or an uncertain reply cannot invent, lose or duplicate authority.

**S11.** As a person granting temporary access, I want its end date and ancestor restrictions enforced, so role changes or reinstatement cannot silently extend it.

**S12.** As an authorised reviewer, I want to ask why this identity can act and who can reach a resource using the same current decision, without exposing records I may not inspect.

**S76.** As a person giving part of my access, I want the form to list everything I cannot give to the recipient I chose, each with the one reason that stops it, so that I know what and why before I try, from the server's answer rather than my browser's guess.

**S78.** As a grant holder or reviewer looking at an identity, I want the screen to answer why it can or cannot do a thing, with the path to a responsible person or the named reason, so that I can trust or correct its access from the server's own decision.

**S115.** As a responsible person, I want my agent issued one certificate listing the grants it holds and naming its holder, against a key enrolled for it, so that it carries proof of those grants from its first spawn and a lost key or an expired certificate can be replaced.

## Administrator — Suspends, reinstates and retires identities, and reads why a check refused

**S18.** As the administrator, I want the screen to show an identity's state and the record that put it there, for a retired identity exactly as for a live one, so that I can answer why a check refused from the record and not from memory.

**S148.** As someone using the identity screens and API all day, I want every read to do its work once, so that the service stays light while it runs beside everything else.

## Certificate verifier — Checks an agent's certificate and its record against the certificate log without the issuer's cooperation

**S31.** As a certificate verifier, I want a revoked certificate to fail verification against the log with its revocation leaf named, so that revocation rests on the log and not on the issuer's word.

**S32.** As a certificate verifier, I want a revoked certificate's past record to still verify, so that revoking a certificate does not erase what it legitimately did before its revocation.

**S34.** As a certificate verifier using the command that takes no log, I want its help to say it does not check revocation, so that I never take its pass as proof a certificate is live.

## Person who signs in — Keeps their sign-in identities to themselves

**S15.** As the responsible person, I want my agent's own machine account accepted as its binding, so that the agent can have its own service account without holding anyone's sign-in.

**S16.** As a person who signs in with a provider account, I want refusals shown to other callers never to reveal my provider or subject, so that my sign-in account is not disclosed through someone else's refused request.

**S136.** As a person who signs in with a provider account, I want the directory to refuse every act that would give that account to an agent, so that no agent can ever hold my sign-in.

**S137.** As a person delegating to an agent or another person, I want my sign-in identities listed as things I cannot give with the reason 'sign-in identity', so that I know why they are never offered.

**S138.** As an agent's responsible person, I want my agent's own machine account accepted as its binding and kept as the agent's, so that the agent has its own service account without holding anyone's sign-in.

**S139.** As a person who signs in with a provider account, I want refusals shown to other callers never to reveal my provider or subject, so that my sign-in account is not disclosed through someone else's refused request.

## Directory administrator — Resolves a refused act

**S17.** As a directory administrator, I want a sign-in identity refusal to show me the provider and subject involved, so that I can tell which account a refused act touched.

**S140.** As a directory administrator, I want a refusal to show me the provider and subject involved, and which agent holds a binding a person tried to link, so that I can tell which account and which agent a refused act touched.

## Release reviewer — Decides from the record whether the release proof is complete

**S110.** As the release reviewer, I want every command, result, ref and hash recorded with every unmet requirement named, so that completion rests on recorded evidence and never on a health check.

## Started agent — Reports back to the directory and presents its session credential

**S80.** As a started agent, I want to present my session credential to the directory, so that it confirms I am my enduring agent in this session.

**S168.** As an agent started by Lys, I want every handle I am launched with to work, without ever holding the key that presents for it.

## Stranger — Checks an issued certificate and its log entry offline, holding nothing from lys

**S84.** As a stranger holding the issuer's certificate, an issued certificate, its leaf and its inclusion artifact, I want to check the certificate and its entry in the log offline with openssl and a standard-library script, so that I rely on nothing from lys and on no one's word that the certificate was logged.

## Issuer — Issues an agent's certificate under the CA key it holds

**S85.** As the issuer, I want every certificate I issue entered in the log before it is written, holding only my CA key, so that no certificate of mine exists outside the log.

## Log operator — Keeps the log and signs its checkpoints with the log's key

**S86.** As the log's operator, I want to be the only holder of the log's key and to make the inclusion artifact for an issued certificate's leaf myself, so that issuing a certificate never needs the log's key.

## Person a role holder answers to — Keeps each holding of a role on a version they chose

**S71.** As the person an agent answers to, I want an edit to its role to leave the agent on the version it holds so that its permissions never change without an act I can see.

**S73.** As the person a holder answers to or an owner of its role's project, I want to see the date each holding will move and who can stop it, and to change one holding's policy, so that no move surprises me.

**S74.** As the person a provisional holder answers to or an owner of its role's project, I want renewing its holding to be a recorded act of one of us so that it is never renewed quietly and lapses when nobody renews it.

## Mover of a holder — Moves a holder to a newer version of its role

**S72.** As the person a holder answers to or an owner of the project its role is defined in, I want to see what a move adds and removes, and choose when it takes effect, before I take it so that I move a holder knowing what it gains and loses.

## Reviewer of a role's history — Checks how each holder came to its version

**S75.** As a reviewer, I want every role act recorded with who made it and in which capacity so that I can verify how a holder came to the version it is on.

## Owner of a project — Defines the roles of a project and assigns them

**S98.** As an owner of a project, I want to make and edit its roles and assign them to agents without any existing holder changing so that a role can improve without silently changing what its holders may do.

## Person without the right to start — Opens an agent file that is not theirs to start

**S106.** As a person who is neither the agent's responsible person nor a directory administrator, I want a refusal naming the agent and the right I lack, so that I know why there is no working Start for me.

## Graph viewer — Looks at who can reach what on the access graph

**S121.** As a person signed in to the directory, I want to see what I may see as a graph, with my own reach drawn from Access's answers and the recorded containment and responsible people, so that what the graph shows is what the directory would decide.

**S122.** As a directory administrator holding the visibility permission, I want to ask the graph who can reach a resource and see Access's answer with its model version and the time it was drawn, so that I can review reach without the screen inventing any of it.

**S123.** As a reviewer of conformance row 8.3, I want tests that compare every drawn edge with the real evaluator and prove the graph module holds no rule, so that the graph cannot drift from Access unnoticed.

## Grant conformance reader — Reads the grant screens and the conformance table and verifies each row against a test

**S141.** As a signed-in person, I want You to show each grant I hold with its source and whether I may pass it on, and only my own agents and grants, while an administrator's People screen shows others, so what I see is what I hold.

**S142.** As a person giving an agent part of a grant, I want the form to show the source grant, the actions it allows, whether I may pass it on and the end the new grant can last no later than, as the service judged them, so I give only what my chain allows.

**S143.** As a stranger checking the conformance table, I want each grant row to name the test that passes it and the command that runs it, and each test to fail when its row's fact is taken away, so the row can be verified without trusting the author.

## Keyboard user — Working the identity screens without a mouse

**S132.** As a person working by keyboard, I want to reach and press every control on every screen without a mouse, so that nothing the screens offer is closed to me.

## Newcomer to the screens — Learning what a screen shows

**S134.** As a newcomer, I want the help overlay to number what is on screen and put me back where I was when I leave it, so that asking for help never costs me my place.

## Identity line lead — Keeping the mock-up and the build in step

**S135.** As the identity line lead, I want the mock-up and the built shell to change together and the proof to run in the gate, so that the mock-up stays the definition the build is held to.

## Brief reader — Reads a rendered brief before it is dispatched or built

**S13.** As a reader of a rendered brief, I want each blocker and each other prose entry on its own line in the order the brief gives them, so that I can tell where one entry ends and the next begins.

## Design gate maintainer — Keeps the repository's rendered documents what its renderer makes of their JSON

**S14.** As the maintainer of the design gate, I want the renderer in this repository to stay the method's copy and every rendered brief to be what it makes of its JSON, so that no .md disagrees with the script that made it and a card finished through the chain never turns the gate red.

## Estate operator — Runs Lys behind every agent and session

**S144.** As the operator of an estate where Lys runs behind every agent, I want each request, append and open to do its work once and scale with what it touches, so that Lys costs nothing it does not need to as history grows.

## Estate operator — Runs Lys behind every agent and session

**S145.** As the operator of an estate where Lys runs behind every agent, I want each request, append and open to do its work once and scale with what it touches, so that Lys costs nothing it does not need to as history grows.

## Installer of the identity stack — Runs lys identity install and stop on a machine

**S146.** As the person installing the identity stack, I want the install to go on the moment each service is ready and to name at once the one that died, so that nothing is cut off by a clock and nothing waits longer than the service does.

**S147.** As the person running the identity product, I want to see which build is running and take a newer one with one command that puts the old one back if the new one fails, so that each landing reaches me and a bad build never leaves me without sign-in.

**S159.** As the person who updated Lys, I want to go back to the build I had before with one command when the new one misbehaves after it started fine, so that a bad update never leaves me stuck.

**S161.** As the person who installed Lys, I want a service that has ended to be seen as ended at once, whatever else Lys was starting at that moment, so that a stop, a restart and an upgrade never wait on or misread a process that is gone.

## Person setting up Lys for the first time — Installs Lys on their own machine with no terminal knowledge and signs in

**S149.** As an ordinary person setting up Lys, I want one installer, then a Lys page that asks my name, email and password and lets me connect Google, GitHub or Microsoft, so that I am signed in without a terminal, a password file or any page that is not Lys.

**S150.** As someone using any of our products, I want to sign in with Lys everywhere, so that there is one account and one sign-in page, and it is always Lys's.

**S160.** As someone who has never used a terminal, I want to download Lys, open it and be guided in my browser until I am signed in, so that I can set it up for my team myself.

## Developer of an app that signs in with Lys — Builds a product that uses Lys for sign-in and permissions without Lys knowing about it

**S151.** As a developer of an app, I want to register my app and its permission schema through a documented API, so that my app's resources and actions are checked by Lys without anyone changing Lys.

**S152.** As an administrator, I want to approve each app and see its schema on a Lys screen before it takes effect, so that no app gives itself power I have not seen.

## Person or agent using Lys through an AI assistant — Reads and changes Lys through an MCP client, with only their own rights

**S153.** As a person or an agent using an AI assistant, I want the assistant to reach Lys with my own identity and nothing more, so that it can never see a secret or do what I could not.

## Person running a team of agents — Starts, watches, talks to and stops agents from Lys

**S154.** As a person running agents, I want to start an agent from Lys and have it running in the background, so that I don't need a terminal or another tool.

**S155.** As a person running agents, I want to see what an agent is doing, type to it, and stop it from one screen, so that every agent is in one place I control.

**S156.** As a person running agents, I want each agent held to a budget for context, tokens and time, compacted or stopped when it reaches it, so that no agent burns what I cannot afford.

**S157.** As a person running agents, I want to give an agent goals with deadlines and have it reminded, so that work is paced and I can see where each goal stands.

**S158.** As a person running agents, I want to ask for a team for a piece of work and have it provisioned with the right memories, opening conversation, budget, goals and checker, so that I can send it off knowing it will be done and checked within what I can afford.

## Installer and operator — Provision and upgrade the internal audit connection

**S163.** As a person installing Lys, I want its internal audit connection provisioned automatically so that I never handle credentials or configure the identity provider.

**S164.** As an operator, I want upgrade and interrupted setup to preserve the sender identity so that pending audit work remains verifiable.

**S172.** As the person running a Lys install, I want a missing API to say it is missing and one route that says the service is serving, so that a web page is never mistaken for an answer.

## Person waiting on a permission check — Uses a Lys screen or route that checks a grant

**S169.** As a person whose request is waiting on the permission service, I want my request to end when I leave it, and nobody else's request held behind mine.

## Person running an agent under a policy — Operates and inspects a runner-owned session

**S170.** As the person responsible for an agent, I want its operating-system sandbox to enforce the same policy as Lys, so a shell or child process cannot bypass my limits.

**S171.** As a person watching an agent, I want to see its actual sandbox and OS-backed refusals, so I can distinguish enforced restrictions from missing coverage.

## Person setting agent context and reminders — Controls agents without a terminal

**S173.** As the person responsible for an agent, I want Lys to compact or stop it at my context limit without terminal typing, so the next turn obeys the limit.

**S174.** As the person setting a goal, I want my saved words delivered at a turn boundary with honest receipts, so a reminder never interrupts a tool or silently arrives twice.

## Person upgrading a live runner — Installs and changes builds while agents work

**S177.** As the person upgrading Lys, I want the runner binary placed and both its installed and running builds stated honestly, without losing live sessions.

**S178.** As the person responsible for live sessions, I want a pending runner restart shown clearly and performed only through my explicit controlled action.
