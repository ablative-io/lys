# Parked work

DIRECTORY-053, including issue #126, and DIRECTORY-054 are parked because they are not needed for tonight's demonstration. Archie directed this pause on 30 September 2026.

Last regression head: `86c9ebcc00c3c4dc0a8b1e3d537db86edbb60f71`.
Tree: `0877dd3cb97b7be2a465b271d3d13e99571cd277`.
Branch: `pikelet/053`.

The corrected regression gate was cancelled in Aion namespace `default`:
- Workflow: `6ccdf91d-5cd4-426c-86df-269e5d7ea8bc`.
- Run: `4f9f6963-7156-4149-9fc3-8ecde795f51b`.
- Measured head: `86c9ebcc00c3c4dc0a8b1e3d537db86edbb60f71`.

The earlier, superseded regression also has an outstanding result:
- Workflow: `6848dc08-8235-4a17-99dc-54d1cfd9b06b`.
- Run: `5bff14fd-ed4e-4b27-9691-708667f14b6e`.
- Measured head: `2bc7704027d098c2a10649de55e8b0255d5c6467`.

Cancellation requested with `aion cancel 6ccdf91d-5cd4-426c-86df-269e5d7ea8bc --reason "053 parked 30 Sep 18:38 for the team-tree work"`; command exit 0, accepted true, and Aion readback status Cancelled. Cancellation is not a red or green result. The earlier superseded run remains separately recorded. Neither run establishes completion. The branch contains the build-script regression only; the exported-tree and dirty-tree behavior tests are retained unchanged. No implementation, stored-record writer, live installation, or surface was changed. DIRECTORY-053's migrations, shared stamp and stated-commit behavior remain open. DIRECTORY-054 has not started. Resume only on a new direction from Archie.
||||||| 14a072e

# Parked delivery work

## Build registry and provisioning migration

The lead narrowed the current delivery to the named MCP server seam needed by
the next provisioning brief. The reusable declared-build registry, its profile
selection and form check route, and the historical-writer fixtures are parked.
No stored-shape change or migration implementation has been made.

The accepted migration plan is:

- Keep immutable build versions in the provisioning store, selected by name
  and version. Each profile version retains the resolved immutable harness
  snapshot used by its existing launch path.
- On loading an old store, derive registry versions from complete inline
  descriptions without inventing capabilities. Profiles without a harness stay
  undeclared. Persist the new shape on the next authorized write.
- Preserve the existing refusal of writes while an upgrade intent is pending
  or unreadable, including the previous reader's unchanged profile bytes.
- Produce fixtures on Dean by compiling the archived provisioning store code
  under integration-test support and invoking that writer's own `open` and
  `set` functions. The output fixture is not handwritten.
- Use writer commit `1b568cd90578f5ed5d7d438e628b23724eef7f12` for the state
  without a harness. Its store source has 300 lines. Verify unchanged profiles,
  undeclared harnesses and the existing named start refusal.
- Use writer commit `d7aad153d4437962ec1511a216495ec84849e692` for the first
  complete inline capability description. Its store source has 441 lines.
  Verify one registry version per distinct complete description, pinned profile
  references, and a resolved snapshot equal to the pre-migration snapshot.
- The earlier R1 commit `2c65af1fcccacb9da7a267b49970036fe26ce16a` stored a
  harness kind rather than a complete description. It is not the description
  fixture. The description and declared-harness types at `d7aad153` match the
  base at `14a072e83342e156e2b9ebc0aed92d07501e3634`.
- Both migration cases must be red on the base before a stored-shape edit.
  Fixtures, migration tests and their gate results remain unwritten and unrun.

## Stale review regression

Commit `29c6b3333ad6c0109111d6ecbd9d1287f998e809`, tree
`1ae5efcb7c33c89b06f58f1cdf81fd57b4caed50`, adds one regression test that
records two profile versions and requires a review of the replaced version to
refuse `ProvisioningChanged`, naming the latest version. No fix is implemented.

Its `src_gate` workflow is `4612aebc-0f31-4401-9b9d-7dcf7e486829`, run
`826b47fd-dc14-47e9-a7d1-930066493552`, namespace `default`. The measured
commit remains `29c6b333`, even though the branch has since advanced.
The lead requested cancellation with the reason
`#120 parked 30 Sep 18:38 for the team-tree work`. The command
`aion cancel 4612aebc-0f31-4401-9b9d-7dcf7e486829 --reason '#120 parked 30 Sep 18:38 for the team-tree work'`
exited 0 and answered accepted. A subsequent `describe_run` confirmed
terminal `Cancelled`, ending at `2026-09-30T08:40:48.095571+00:00`.
No gate exit or test outcome is claimed from the cancellation.

The requested #120 cancellation and queued #119 cancellation identify this
same single workflow: it measured the #119 regression under the #120 job.
The full list of eight `src_gate` runs since `2026-09-30T08:00:00Z` held no
second distinct #119 run. Only this workflow was cancelled; no other seat's
gate was changed. This regression remains without red proof;
it must be resolved or explicitly parked outside active test discovery before
any later change on this branch can be considered ready to land.

## Shared start checks

The lead directed the isolated cherry-pick of
`14017ce43fa6c5dce377560ff191dc907e425fa8`, without its parent's regression.
The resulting commit on this branch is
`b462642e6db078225941be4cbcd0dcca5a74ea9e`, tree
`abe95a5a44186c2ef02bcad342dc64c0b190dbee`. It extracts the existing placement
and egress checks into `start_checks.rs`. It has been pushed and read back;
no green gate for that extraction is claimed.

The lead approved sharing the existing active/review and broker-listing checks
as well, through the launch owner. Readiness must claim only what the actual
start checks establish. Expiry, use counts and budget checks are outside this
delivery. No readiness route or surface change has been made.

## Current seam and next handover

The base already stores each MCP server's name in a version's
`settings.mcp_servers`, validates it through `mcp_record`, and returns it from
the provisioning route. Existing provisioning and launch MCP tests cover named
servers. This is source evidence; those tests have not been run in this sitting.
The next brief must establish whether this existing seam satisfies its needs.

The nested-teams job needs its published brief and a complete six-part handover
before implementation starts. No team files, routes or stored shape have been
changed in this sitting.
