---
type: brief
id: SECRETS-006
cluster: secrets
title: The broker signs with a key it holds and returns the signature only; the key never leaves it
---

# SECRETS-006: The broker signs with a key it holds and returns the signature only; the key never leaves it

> **Cluster:** secrets
> **Depends on:** SECRETS-005
> **Checklist:**
> - C411 — A signing key is stored as a secret with one named purpose; no route that carries a value serves it, and listing shows its public key and never its seed (SECRETS-006 R1).
> - C412 — The signing route admits the use as any other, builds the bytes to sign from typed members, and returns the signature only; it signs no raw digest (SECRETS-006 R2).
> - C413 — Every signing use and every refused one is on the audit record, and a revoked handle signs nothing (SECRETS-006 R3).
> **Stories:**
> - S165 (AI Agent, Uses a handle for its outbound calls and reads its sealed records) — As an agent, I want the broker to sign my requests with the key it holds for me, so that I can prove who I am without ever holding the key.

## Purpose

The broker admits a use of a secret and never gives its value out. A signing key is the one secret that cannot be used that way today: to sign, a caller would have to be given the key. DIRECTORY-049 R7 first had `lys mcp` read an agent's certificate key out of the broker at each call, which the broker's own rule forbids. This card gives the broker a signing use, so a caller that holds a handle to a key gets signatures and never the key.

## Task

Let a secret be stored as a signing key with the one purpose it signs for. Add a signing route that takes a handle, its signed presentation and the typed members of the thing to be signed, builds the bytes to sign itself, and returns the signature under the broker's ordinary admission and audit. The broker never signs a digest or bytes a caller composed.

## Requirements

### R1: A signing key is a secret with one named purpose

Behavioural. A secret may be stored with kind signing_key: an Ed25519 seed of 32 bytes, entered once like any secret, with the one purpose it signs for. Purposes are a closed list in code, each with its own domain label; the first is agent_request, the signed request of lys-identity-server (agent_signature.rs). A stored signing key has no value-bearing use: the proxy, the environment hand-over and every other route that carries a value refuse it not_a_value_secret. Listing shows its handle, scope, holder, purpose and public key, never the seed.

**Acceptance:**
- A seed of any length but 32 bytes is refused signing_key_invalid naming the length given.
- A purpose outside the list is refused signing_purpose_unknown naming it.
- Every value-bearing route refuses a signing key not_a_value_secret, with one test per route.
- The listing of a signing key holds its public key and purpose and no byte of its seed, checked by a test that searches the answer for the seed.

**Files:**
- create: crates/lys-secrets/src/broker/signing.rs
- create: crates/lys-secrets/src/broker/signing_tests.rs
- modify: crates/lys-secrets/src/broker.rs
- modify: crates/lys-secrets/src/lib.rs
- modify: crates/lys-secrets/src/bin/lys-secrets/view.rs

**Checklist:**
- C411 — A signing key is stored as a secret with one named purpose; no route that carries a value serves it, and listing shows its public key and never its seed (SECRETS-006 R1).

**Stories:**
- S165 (AI Agent, Uses a handle for its outbound calls and reads its sealed records) — As an agent, I want the broker to sign my requests with the key it holds for me, so that I can prove who I am without ever holding the key.

### R2: The broker builds what it signs and returns the signature only

Behavioural. POST /_lys/sign takes the handle, its signed presentation, the purpose and the purpose's typed members; for agent_request these are the method, the path, the digest of the body, the signing instant and the nonce. The broker admits the use exactly as it admits any other (admit_use_for_checked: the presentation is signed, the handle is live, the lease covers the named key, the caller is permitted), checks that the purpose asked is the key's own, builds the bytes to sign from the members under the purpose's domain label, signs, and answers the signature and the key's public key. It accepts no digest to sign and no bytes composed by the caller. The key that signs a presentation stays with the presenter as it does today; this route does not move it into the broker.

**Acceptance:**
- A signature the broker returns for agent_request verifies in lys-identity-server's own check of the same request, in a test that drives both.
- A call naming a purpose other than the key's own is refused signing_purpose_mismatch naming both.
- A call carrying a raw digest or bytes in place of typed members is refused by the request's shape.
- An unsigned presentation, an ended handle, a lease that does not cover the key and a caller not permitted are each refused by the names the other uses already give.
- No answer, log line or audit line holds a byte of the seed, checked by a test that searches each for it.

**Files:**
- create: crates/lys-secrets/src/bin/lys-secrets/sign.rs
- create: crates/lys-secrets/tests/signing.rs
- modify: crates/lys-secrets/src/bin/lys-secrets/serve.rs
- modify: crates/lys-secrets/src/bin/lys-secrets/main.rs

**Checklist:**
- C412 — The signing route admits the use as any other, builds the bytes to sign from typed members, and returns the signature only; it signs no raw digest (SECRETS-006 R2).

**Stories:**
- S165 (AI Agent, Uses a handle for its outbound calls and reads its sealed records) — As an agent, I want the broker to sign my requests with the key it holds for me, so that I can prove who I am without ever holding the key.

### R3: Every signature is on the audit record

Behavioural. Each signing use appends one audit line through the broker's existing audit path: the handle, the caller, the key's public key, the purpose, the digest of the bytes signed and the instant. A refused use appends its refusal by name. Revoking the handle, ending its lease or dropping the key refuses the next signing use by the names those acts already give.

**Acceptance:**
- A signing use and a refused one each appear once at GET /_lys/audit with the members above.
- After the handle is revoked the next call is refused and no signature is returned.
- The audit line holds the digest of what was signed, never the members' secret parts or the seed.

**Files:**
- modify: crates/lys-secrets/src/broker/using.rs
- modify: crates/lys-secrets/src/broker/admit.rs

**Checklist:**
- C413 — Every signing use and every refused one is on the audit record, and a revoked handle signs nothing (SECRETS-006 R3).

**Stories:**
- S165 (AI Agent, Uses a handle for its outbound calls and reads its sealed records) — As an agent, I want the broker to sign my requests with the key it holds for me, so that I can prove who I am without ever holding the key.

## Boundaries

- SHALL NOT return, log or write a signing key's seed on any path, or accept a request that would make the broker return it.
- SHALL NOT sign a digest or bytes a caller composed; the broker builds what it signs from typed members under a purpose's domain label.
- SHALL NOT add a second admission path: a signing use is admitted, leased, audited and revoked as every other use.
- SHALL NOT add a timeout, deadline, sleep, poll interval, #[allow], #[ignore] or any bypass.
- SHALL NOT add a silent fallback: every failure is a named refusal.

## Verification

- The full Lys gate and ast-grep scan exit 0 at the card's head, measured by the card round.
- On a scratch install: store a signing key, sign an agent request through the broker, and have lys-identity-server accept the request; then revoke the handle and see the next signing refused.
