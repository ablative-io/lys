# ACCESS-013 (DRAFT, not yet through the design gate): an agent decides and credentials one person's drafts and grants for one team, and passes a grant on only to a resource Lys has placed under it

**Status: draft.** Written by hand by Apollo on 2 November 2026, on Waffles' word (Lys is his to decide), and stopped
part-written when Tom asked to talk. It is not a brief yet: it isn't in `briefs/`, has no JSON, no checklist items and
no stories, and hasn't been through `scripts/design/gate.sh`. Nothing in it is approved, and Tom approves nothing
under it until it's live. Read at Lys main `75daf161` (31 October), with clx `6206e7e` and liminal's
`docs/design/access/schema.json` from the main checkout.

## Why

The estate plan (prospekt-mockups `docs/briefs/every-seat-registered-2026-11-02.md`) needs about 100 grant drafts
approved and 25 grant credentials written. Waffles' decision: one named agent, not the drafter, does both, and Tom's
only act is approving that agent's grants. At `75daf161` there is no narrow way to give it:

- **A grant reaches one id, or the whole organisation.** A grant names one `kind:id` (`crates/lys-identity/src/grants/types.rs:163-168`).
  Approve is scoped `draft.decide` on `draft:{id}` (`crates/lys-identity-server/src/openapi_table.rs:152`), and the
  credential route `grant.delegate` on `grant:{id}` (`openapi_table.rs:180`). The id comes from the path, and `all` is
  used only by routes with no path parameter (`openapi_table/admission.rs:43-60`). The only other way in is the agent's
  organisation standing (`organisation_standing.rs:41-63`, `route_actions.rs:84-95`): `organisation.administer` behind
  `draft.decide` and `organisation.grant` behind `grant.delegate` (`grants/shipped.rs:33-51`). That reaches every draft
  and grant in the organisation.
- **Approving alone registers nothing.** Lys carries out an approved draft as its approver
  (`drafts_apply.rs:105-111`). clx new drafts each seat's grant as `POST /grants` from the administrator's standing
  (clx `bin/src/network/drafts.rs:98-110`), and an agent gives from standing only through its organisation standing
  (`grants/administrator.rs:1-39`). A pass-on must name the source grant's own resource, or it is refused
  `ResourceOutside` (`grants/delegation.rs:110-117`). So a grant on the network can't give grants on the subjects in
  it.

## Decided (Waffles, 2 November)

1. `draft.decide` and `grant.delegate` can each be held on `person:<id>`, optionally narrowed to a team.
   - `draft.decide` on `person:P`, team T, reaches a draft whose reviewer is P and whose drafting agent is in T.
   - `grant.delegate` on `person:P`, team T, reaches a grant P is responsible for and that an agent in T holds.
   - A person-only grant (no team) is a valid form of the same grant, not an alternative to it.
2. The `own_draft` refusal stays. New: `own_grant`. An agent never writes a credential for a grant it holds itself.
3. `organisation.administer` is never the route for this. Nobody "fixes" a narrow refusal later by handing out the
   organisation's administration or standing.
4. A grant may be passed on to a resource Lys has placed under its own resource (narrowing only). liminal places
   `liminal.subject` and `liminal.agent` under `liminal.network`. Tom gives the named agent one grant on
   `liminal.network:<the estate's network>`, which it may pass on to agents in team T. clx new then drafts each seat's
   grants from that grant, not from the administrator's standing. That's clx's change, after this lands, with the
   grant's id in `station.json`.
5. So Tom's act, once this is live: approving three grants, `draft.decide` and `grant.delegate` on `person:Tom` with
   team T, and the network grant.

## Shape so far (to be written as requirements)

- **The team mark.** An optional grant member, key 17, the team's id. It's written only when present, so every grant
  written before it keeps its bytes, as keys 13 to 16 did (`grants/codec.rs:1-18`). Passing on keeps the mark: it is
  never dropped or changed. It's valid only on a `draft.decide` or `grant.delegate` grant on `person:<id>`.
- **The door's question.** For `draft:{id}`, the door also asks `draft.decide` on `person:<the draft's reviewer>`. For
  `grant:{id}`, it asks `grant.delegate` on `person:<the grant's responsible person>`. Where the admitting grant carries
  team T, the drafting agent (or the grant's holder) must be a confirmed member of T, or of a team nested under T, as
  `GET /drafts`' team filter reads it (`list_page.rs:146-162`), with T unretired. That is judged at each request, so an
  agent leaving T is refused from that moment, naming the team. `GET /drafts` lists exactly the drafts the grant may
  decide.
- **The credential route** (`grant_tokens.rs:240-262`) stops skipping the responsible-person check for an admitted
  agent unless the person-scoped grant above admits it.

## Reds so far

- A draft for another reviewer is refused.
- Another person's grant is refused for a credential.
- A draft made by an agent outside team T is refused, naming T.
- A grant held on `person:<id>` and team T stops reaching a draft once its drafting agent leaves T, refused from that
  moment and naming T.
- The deciding agent's own draft is still refused `own_draft`. A credential for its own grant is refused `own_grant`.

## Still to write, after Tom's talk (Waffles: four requirements, each with its red)

1. **Actions:** what's passed on to a child carries only actions the held grant reaches on that child. Never one it
   lacks.
2. **Time:** it ends no later than the held grant does.
3. **Revocation:** revoking or expiring the held grant ends everything passed on from it at once, and the refusal
   names the parent grant.
4. **Placement** (matters most): "placed under its resource" means a placement Lys has recorded, never one the schema
   merely allows, and the passing agent can't record that placement itself. Red: the agent places a foreign subject
   under its own network, passes a grant on it, and is refused.
   - **Not yet read:** who may record a placement under `liminal.network`. The route is `POST /apps/{app}/placements`
     (`apps_schema_api.rs:129`, `place`).

Also still to do: the checklist items and the story, the brief's JSON and its rendered markdown in `briefs/`, the files
list checked against `design.json`'s structure, and the design gate read for its FAIL lines.
