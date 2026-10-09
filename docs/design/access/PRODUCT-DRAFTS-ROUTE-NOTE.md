# Product drafts and the operator draft route: a decision for Monday

Written by Gaia, 9 October 2026 16:5x, on Waffles' order (room post 02b678a6). Nothing here is decided. It sets out
the two options with their costs, so that nobody has to work them out again.

## The clash

ACCESS-001 R3 says that a product holding a grant `by_draft` records its prepared act as a draft. Lys then answers
that draft's approval back to the product's connector. lys-pass (`crates/lys-pass/src/drafts.rs`) already speaks
this shape:

- `create_draft` posts `DraftRequest {operation, grant, target, request_digest, words}` to `POST /drafts` with the
  holder's bearer pass.
- `approved_drafts` reads `GET /drafts?app=&state=approved&after=` with the connector pass and expects
  `ApprovedPage {drafts: [ApprovedDraft {id, app, grant, target, request_digest, words}], next, total}`.
- `record_execution` posts to `POST /drafts/{id}/executed` with `{receipt_digest}`, or to
  `POST /drafts/{id}/refused-on-execution`. Today that second post carries a permission `Refusal`. The change
  agreed with Apollo is that it carries `{refusal, reason}`, any name the product gives. That change is drafted in
  `.scratch/gaia/r3/`, not in the tree.

Lys already serves `POST /drafts` and `GET /drafts` for a different thing, the operator draft
(`crates/lys-identity-server/src/drafts_api.rs`, `drafts_list.rs`):

- The body is `DraftBody {operation, target, method, path, body, note}`, with `deny_unknown_fields`.
- Decisions go to `/drafts/{id}/approve`, `/refuse` and `/correct`. Each names the creation by `creation_hash`,
  and an approval reserves an `application` operation.
- `GET /drafts` answers `DraftList {drafts: [DraftView], ...Totals}`. A view's `state` is `waiting`, `approved`,
  `refused` or `corrected`.
- Users today: the surface's Drafts screen, Access tabs, dashboard widgets and palette (`surface/identity/src/
  features/drafts`, `features/dashboard`, `features/access/AccessTabs.tsx`, `shell/Palette.tsx`), the MCP `change`
  tool (`drafts_mcp_tests.rs`), the dashboard API, the OpenAPI table and three test files.

So the server would refuse lys-pass's `create_draft` (unknown fields), and its approved page would not parse as
`ApprovedPage`. No product draft can reach Lys today.

## Option A: a separate route for product drafts

Serve the product side at its own prefix, for example `/product-drafts`, `/product-drafts/{id}/executed` and
`/product-drafts/{id}/refused-on-execution`. lys-pass's three calls move to that prefix. The operator draft stays as
it is.

- Lys server: a new handler module with its own event kinds, or new variants on `draft_event`, plus OpenAPI
  entries and a test file. No existing route or screen changes.
- lys-pass: three endpoint strings and their tests. This is a breaking change for anyone pinned to the current
  paths. Today that is no one, because no product draft can reach Lys.
- Approval: someone has to say whether a person approves a product draft on the existing Drafts screen (one
  queue, two kinds) or on its own screen. That is a surface cost either way.
- Risk: two things called "draft" with two approval paths. The log keeps them apart by event kind.

## Option B: one route, one body shape

Make `POST /drafts` take one body that carries both cases. For example, the operator fields become one variant and
`{grant, request_digest, words}` another. `GET /drafts` then answers one view with both shapes.

- Lys server: `DraftBody` becomes an untagged or tagged enum. `deny_unknown_fields` has to be re-earned per
  variant. `DraftView`, `DraftList`, the OpenAPI table and every operator test need touching. The connector-pass
  read of `state=approved` needs its own projection, because an operator approval reserves an `application`
  operation and a product approval does not.
- Surface: the Drafts screen, dashboard widgets and Access tabs read `DraftView`, so each needs the new shape or
  a filter on it.
- lys-pass: `ApprovedPage` must parse the shared view, or Lys answers a filtered shape for a connector pass. The
  second is option A again, under one path.
- Risk: every operator-draft user is touched for a feature none of them use.

## What either choice still needs

- The `{refusal, reason}` change to `RefusedOnExecution`. The test is drafted at
  `.scratch/gaia/r3/drafts_refused.rs` and the patch at `.scratch/gaia/r3/apply_refused.py`.
- An approval event for a product draft that records nothing beyond the decision. Lys never executes the act.
- The haem door adapter. Apollo, 16:49 (room post 1f8d46fd): haematite calls only lys-pass's
  `create_draft(pass, &DraftRequest, &CheckAnswer)` and `run_approved`, never the wire. So either option changes
  lys-pass's client and leaves haematite alone, as long as those two signatures hold.
- Under option B, a product draft needs `grant` and `request_digest` as required fields. The grant is checked
  against the approvers, and the digest makes the run happen once. The operator shape carries neither (Apollo, same
  post).
