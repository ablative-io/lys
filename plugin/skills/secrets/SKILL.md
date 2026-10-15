---
name: secrets
description: "Manage Lys secret metadata and handle scope without revealing secret values."
---

# Lys secrets

Read the sibling lys skill before calling a tool.
This skill covers 13 declared operations. Use [the route reference](references/routes.md)
to find their exact methods and paths, then inspect the live OpenAPI operation.

Use only the secret routes required by the user's task. Read metadata, grants,
audit, revocation, settings or handles without exposing values. Scope/recipients
POST routes ask questions; add/replace/retire/drop/held-account routes change state.
Read the declared request schema and route semantics before selecting a tool.

Never print a secret, private key, token, handle credential or whole credential
response. Do not ask the user to paste one into chat. Supply values through the
product's supported protected input only when the user authorized that operation.
If this host cannot carry a value without exposing it, hand back the exact step.
Do not use secrets as a probe, create a fallback account or rotate a key to answer
an access refusal. Audit and revocation receipts are separate from downstream
proof that a credential can no longer be used.
