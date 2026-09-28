# Holder keys for agent sessions

An options note for DIRECTORY-049 R7 and DIRECTORY-050 R3 and R5, for a ruling. It is not a brief.

## The gap

A broker handle is usable only with a presentation signed by its holder key. The broker checks each presentation against the public key bound at issue (crates/lys-secrets/src/broker/admit.rs line 77, stored at broker.rs line 358). Today a handle is issued only by the lys-secrets Issue command, which loads the holder key from a file (crates/lys-secrets/src/bin/lys-secrets/main.rs lines 136 to 146). No route issues a handle. The launch template names handle ids only (crates/lys-identity-server/src/launch_template.rs lines 71 and 171). The broker's on-behalf path admits people only (PeopleOnly, crates/lys-secrets/src/store/policy.rs line 17).

So an agent session holds handle ids and no holder key. Every handle 050 R3 launches an agent with is unusable, and so is every account handle 050 R5 rotates through. `lys mcp` in 049 R7 cannot present for its signing handle either.

Every option below needs two things that do not exist yet.

1. A broker route that issues a handle bound to a given public key, for a caller the broker already admits. That is a SECRETS card.
2. A way for a session to prove to the one that holds or asks for the key that it is that session.

## What proves the session

The runner (050 R1) is the only process that spawns an agent, so it knows each session's root process. There are two ways a local process can prove it belongs to a session.

- **Peer ancestry.** The runner reads the peer's pid from the socket and walks up to a session root. rustix's socket_peercred is Linux only (rustix 1.1 net/sockopt.rs, cfg linux_kernel). On macOS the peer pid needs LOCAL_PEERPID and the parent walk needs proc_pidinfo, and neither is in the standard library or rustix. Both would need a foreign call in an unsafe block, which the Lys boundaries refuse. A pid can also be reused after a process ends.
- **An inherited descriptor.** At spawn the runner hands each session one end of a socket pair as a named extra descriptor. Holding that descriptor is the proof, with no pid read and no race. The standard library passes only standard input, output and error to a child. So placing a fourth descriptor needs pre_exec, which is unsafe, or a new dependency that does the same inside it.
- **Standard input as the channel.** A harness keeps standard input for its terminal, so this does not fit the agents 050 runs.

Neither proof is free today. The ruling has to take one cost. Peer ancestry costs a macOS foreign call. An inherited descriptor costs pre_exec or a dependency.

## Option A. The runner holds one key per session

- **Who holds the seed.** The runner makes one Ed25519 key per session in memory when it starts the session. The seed never leaves the runner and is never written.
- **Issuance.** The server asks the runner to start a session (050 R3). The runner answers with the session's public key before it spawns. The server then asks the broker to issue the launch record's handles bound to that key, and the runner spawns only after that.
- **Use.** The harness and `lys mcp` ask the runner, over its socket, for a presentation for one request. The runner checks the session proof and signs a presentation for that session's handles only.
- **What a stolen socket gets you.** The socket is mode 0600, so only the same user can open it. A same-user process that fails the session proof gets nothing. A process that passes it gets presentations for that one session's handles, which the session already has. The seed cannot be read through the socket.
- **What changes.**
  - 050 R1: "accepts only the server's signed requests" gains session peers proved as ruled, for the one act presentation.
  - 050 R2: the protocol gains that act.
  - 050 R3: the start orders key, then issue, then spawn.
  - 050 R5: account values reach the environment at spawn through the runner's presentation.
  - 049 R7: `lys mcp` asks the runner for each presentation and holds no key.

## Option B. `lys mcp` holds its own key

- **Who holds the seed.** Each consumer makes its own key in memory: `lys mcp` for itself, and the harness wrapper for the launch handles.
- **Issuance.** Each one asks the runner to have its public key's handles issued. The runner checks the session proof and asks the server, which asks the broker.
- **What a stolen socket gets you.** A process that passes the session proof can have handles issued to a key of its own. That is more than option A gives, because it gets usable handles, not single presentations.
- **What changes.** The same lines as option A, plus an issue act in 050 R2 in place of the presentation act. Two keys per session, and a handle issued twice for one session.
- **Why it's weaker for 050.** The harness has no wrapper of its own today, so B needs one for the launch handles, and that wrapper holds a seed beside the agent.

## Option C. The server holds the key

- **Who holds the seed.** lys-identity-server keeps the session's key.
- **Use.** Presentations are asked for with an agent signature (lys-agent-signature, agent_signature.rs). That needs the agent's certificate key in the agent's process, which is what 049 R7 exists to avoid. It is refused for that reason.

## Recommendation

Option A with the inherited descriptor as the proof. It keeps one seed per session in the one process that already spawns, it gives a thief nothing the session did not have, and its proof has no pid race. The unsafe it needs is one pre_exec in lys-runner. That crosses the Lys boundary, so it needs Tom's word or a named dependency. If neither is given, use option A with peer ancestry on Linux, and on macOS refuse the presentation act with a named refusal until a safe peer pid exists.
