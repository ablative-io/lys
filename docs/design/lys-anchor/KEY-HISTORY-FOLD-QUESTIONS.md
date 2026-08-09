# Questions the key-history fold must answer

**Written 2026-08-09, deliberately BEFORE reading any candidate design.**

Three fold designs are in flight (workflow `wf_491bb3ad-508`, lenses `minimal` /
`adversarial` / `interop`). This file exists so they are judged against the **rules** rather
than against each other, and so the judging criteria cannot be shaped by what the designers
happened to think of.

⭐ The law this applies: *an injection table is a list of what somebody thought of, so it can
never be audited against itself — audit from the rules inward.* A review rubric derived from
the submissions has exactly that defect. So the rubric is derived from `DELEGATION-V1.md`
§1.2, §2.2, §2.3, §5 and DP16/DP24/DP26, and is committed before the submissions arrive.

---

## Part 1 — Rules the fold MUST enforce (restated from the normative sections)

These are not open questions. A design that contradicts one is wrong, not novel.

| # | Rule | Source |
|---|---|---|
| R1 | The partition key is `(subject_kind, subject_value, role)`. The **kind** is in it deliberately — a domain and a seat with colliding identifier strings must not share a counter | §2.2, and the paragraph at §1.2 on the typed subject |
| R2 | Within a partition, `sequence` is the sort key and is **strictly increasing** | §2.2 |
| R3 | Same partition **and** same `sequence`, **byte-identical** → ignore the later one as a duplicate. **MUST NOT refuse** | §2.2 |
| R4 | Same partition and same `sequence`, **differing** → **refuse**. Root-key equivocation. Never pick | §2.2 |
| R5 | `not_before_unix_ms` orders **nothing** | §2.2 |
| R6 | Log position orders **nothing**. The log establishes existence and publication | §2.2, DP16 |
| R7 | A derived view may **refuse** on its own authority and may **never permit** on it | DP26 |
| R8 | Genesis is `(1, 2)` at `sequence = 0`, leaf 0 | §5 |
| R9 | `sequence == u64::MAX` is already refused at decode; the fold inherits it and must not re-permit | §1.2 |

⛔ **R3 is the one most likely to be got wrong, and getting it wrong is a keyless denial of
service.** A replay is byte-identical *by construction* — that is the property `sequence`
exists because of. A fold that refuses on *any* duplicate at a sequence hands an attacker
holding **no key material** a permanent refusal in place of a rollback. The draft rule in
§2.2 had this defect, in the very sentence written to fix the replay. **A remedy that
preserves the attacker's reach has moved the failure, not removed it** — so every design is
to be checked against the keyless attacker specifically, not against a key-holding one.

---

## Part 2 — Genuinely open questions. A design that is silent on one is incomplete

Each of these is reachable by someone holding no key material unless stated otherwise.

### Q1 — `not_before` in the future, at the highest sequence. Which key is current?

**§2.2 pins that `sequence` orders and `not_before` does not, and leaves their interaction
unstated.** Suppose a partition holds sequence `N` (effective) and `N+1` (`not_before` two
days out, signed and published).

- "Highest sequence wins" ⇒ there is a window where the current key is one that is not yet
  in effect.
- "Highest **effective** sequence wins" ⇒ R2 is not the whole rule, and the answer now
  depends on the verifier's **clock**, so two honest verifiers disagree during the window.

Both readings are defensible and they are not the same system. Whichever is chosen, say
**where the evaluation instant comes from** — DP26 makes freshness tolerance an *input* to
verify rather than a property of the artifact, and this must be consistent with that.

⚠️ Note the asymmetry that makes this security-relevant rather than merely fiddly: under
"highest effective wins", an operator rotating away from a compromised key **cannot make the
rotation take effect earlier than the `not_before` they already signed**, and the compromised
key stays current for the whole window. Under "highest wins", they can — but a mis-typed
future date silently disables the subject.

### Q2 — Byte-identical **compared over what exactly?**

R3/R4 turn on "byte-identical", and the noun is load-bearing. Candidates: the payload
bstr, the full COSE artifact, the canonical re-encoding of the artifact, or the log leaf.
They differ when the unprotected bucket is non-empty (§3.4 requires it empty — but a fold
that trusts that requirement rather than re-checking it is trusting a *different* function's
discipline). State the noun, and state it as bytes the fold itself derives, not as bytes it
was handed.

### Q3 — A leaf that is not a delegation

A log carries more than delegations. The fold must distinguish:

- a leaf that is **not a delegation at all** → skip;
- a leaf that **claims to be one** (content type `application/vnd.lys.delegation.v1+cbor`)
  and **fails to decode** → this is the interesting one.

**Refusing on the second is a keyless DoS if anyone may append; skipping it silently
re-opens suppression.** This is the same rollback-versus-denial structure as R3/R4 and it
must be answered with the same care rather than falling out of the code. Whichever is
chosen, the *reason* must be the admission policy's reach: if `AcceptAll` ships
(`admission/trivial.rs` — it does), then "anyone may append" is the shipped default.

### Q4 — Suppression, and the fact that nothing marks genesis

§1.2 records that `sequence = 0` is a **convention, not a claim**, so a fold reading from a
non-zero offset cannot tell whether it has seen the true minimum. The generalisation is
worse than the recorded case: **a fold shown a prefix of the log is on a stale key and has
no signal.** Withholding the newest delegation needs no key material — only control of what
the verifier is shown.

Does the design require contiguous sequences (`0,1,2,…`)? Contiguity detects suppression of
an interior entry but makes an offline-prepared-and-never-published delegation permanently
break the chain. Non-contiguity is safe against that and blind to suppression. **Say which,
and say what the fold is therefore NOT claiming** — a fold that cannot detect suppression
must not present its answer as "the current key" without qualification.

### ⛔ Amendment — DP26 already constrains Q5 and Q6 harder than I first wrote them

Found while sweeping stale blockers in `BUILD-PLAN.md`, *after* Parts 1–2 were written, and
recorded as an amendment rather than edited in silently — a rubric that quietly grows the
answers it was supposed to be testing against is no longer a second party.

DP26 (`DECISIONS.md:400`, settling F4) rules:

- **Freshness tolerance is an input to the verification call, never a judgement on its
  output.** No default tolerance ships. **The log size `N` is in the answer type from the
  first version.**
- **A derived view may refuse on its own authority and may never permit on it** — so a
  *permission* must **walk the tail** from the view's stamped position to the current tip,
  bounded by entries-since-snapshot and never by history.

So Q5 is not open on whether the output carries its position: **it must**, or it cannot be
tail-walked. And Q6's incremental option is not merely permitted but effectively *required*
in shape — a stamped snapshot plus a bounded tail walk is exactly what DP26 describes. What
stays open in Q6 is what re-derives the snapshot from scratch and how often.

⚠️ **This also sharpens Q4.** The tail walk is the suppression defence for *permissions*
specifically: staleness fails open for revocation and closed for grants. It does **not** rescue
a fold shown a truncated log, because a truncated log has a tip too. Any design claiming DP26
handles suppression must say which of the two it is handling.

### Q5 — What does the fold return, and can a stranger re-verify it?

The one rule that governs everything: *its entire value is that strangers can verify it.*
A fold output that a third party must re-run the fold to trust is a cache, not evidence.
State whether the output is re-derivable, what inputs pin it (log size, root hash,
evaluation instant), and whether it is a signed artifact — and if it is, note that **it
freezes a wire format** and therefore is Tom's decision, not the designer's.

### Q6 — Incremental or whole-log?

Whole-log is O(n) per query and always correct. Incremental needs a checkpoint of derived
state, which is itself a claim needing its own integrity story — and a derived checkpoint
that can go stale in lockstep with what it summarises is the failure this repo has already
been bitten by. If incremental, say what re-derives it from scratch and how often.

### Q7 — One refusal status per substantive refusal

DP24's rule for transport applies here for the same reason: a fold that collapses
equivocation (R4), malformed-delegation (Q3) and unknown-subject into one error teaches
callers to treat them alike, and one of the three is a **detected fault of the root key**.

---

## Part 3 — What I will NOT accept as an answer

- **"The implementation will decide."** Every wire format in this repo has had a bug found
  by implementation rather than review — including `lys/delegation/v1`. That argues for
  specifying first, not for deferring.
- **A design whose safety rests on the operator being careful.** R7 exists because a derived
  view's authority is asymmetric.
- **A test plan made only of refusals.** A suite of rejections cannot tell a working fold
  from one that refuses everything. Every refusal case needs the near-miss that must be
  *accepted* alongside it.
- **A claim of coverage stated in the same breath as a gap being closed.** State which cases
  are covered and let the remaining gaps be visible.

## Part 4 — Decisions that are the operator's, not mine and not a designer's

Flagged in advance so a synthesis cannot quietly absorb them:

- Q1's choice changes what a signed delegation **means**, so it is a format-semantics
  decision and goes to Tom.
- Any fold output that is **signed** freezes a new wire format (`lys/key-history/v1` or
  whatever it is called) and is subject to the same rule as every other: *a new version
  alongside, never a mutation of the shipped one.*
- Publishing `lys-core 0.3.0` — which this work unblocks — remains Tom's.
