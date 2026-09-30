# Message connections on the session canvas

Cambium is authoritative for message history and who can read it. Lys reads
Cambium as the same person, then further restricts identities to the person's
Lys view. Edges mean a direct message or explicit mention addressed an identity.
They do not mean an agent consumed it, a person read it, or a terminal received it.
Team membership, terminal input and matching display names are never evidence.
The canvas uses distinct identity and terminal-session nodes.

Both services must serve the new routes: Cambium's
`GET /conversation/message-edges` and Lys's `GET /runtime/message-edges`.
The browser must be signed into both on the same host. Lys forwards only the
`cambium_session` cookie, never its own cookie or a service credential. A
Cambium caller not explicitly bound to the signed-in Lys person is refused.
Different-host deployments need an authenticated bridge before this read works;
missing authentication is not bypassed.

Add `cambium_messages` to Lys's existing configuration. The value has `url`
(the trusted Cambium base URL) and `bindings` (an array of records with
`participant`, the actual Cambium registry id, and `identity`, the actual
enduring Lys person or agent id). Obtain ids from each service's own directory;
do not copy display names. Bind the signed-in person as well as the agents.
Bindings must be one-to-one. HTTPS is required except on loopback. URLs with
credentials, queries or fragments and duplicate identities are refused at boot.
Redirects are never followed. Configuration contains no bearer token.

The read pages caller-visible places, each stream's posts and each thread's
replies. Cambium's existing history page-size ceiling applies when none is
requested. The view represents the authoritative current history; it is not an
immutable historical export of superseded post revisions. No message body is
sent to Lys. Missing identity bindings are named and those edges are withheld.
The screen explicitly says when pages remain, and its button reads the next
page. Refresh starts a new read; this is not a snapshot or background watcher.

Verification targets written with this change: Cambium `message_edges`; Lys
server `message_edges` plus `message_edges_tests`; surface
`message-connections.test.tsx` and `session-canvas.test.tsx`. Their addition does
not claim execution. A final end-to-end check requires both installed routes,
actual bindings and a caller with both sessions. Scripted transport rendering
proof is separate from that authenticated installed-service proof.
