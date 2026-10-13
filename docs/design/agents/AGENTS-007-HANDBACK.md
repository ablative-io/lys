# AGENTS-007 handback

The source passes TypeScript. The tree, seat acts and both terminal transports are implemented. Delivery is blocked. C737 exceeds its 50 ms render bound. Today's canvas API rejects the operations choice required by C739. C738 is component-proven. The brief records implemented and blocked development legs; it is not claimed complete or built.

START: main at dab11395bb8caad2eea418129538fdd2fbe316d9, 2026-10-13T15:42:41+11:00. The checkout was clean. `git pull --ff-only` exited 0: Already up to date.

## What changed

NetworkTree reads the complete paged network and live seats through the existing change signal. It draws every non-retired machine, including empty machines, and exact seat states, transports and reasons. Names are resolved from the served agent records. Start takes a fresh operation id and immediately uses the answer. An uncertain operation is retained.

NetworkTreePane uses the existing GPU byte transport for PTY seats and follows the managed attach lines for managed seats. Selection aborts the earlier read. Attach answers must name this seat and session. Send clears only the submitted draft; a newer draft survives. Stop distinguishes a confirmed end from an unconfirmed request. Offline and unknown states never mount a terminal.

Plain operations visits open the tree. Explicit agent, layout and tool-panel destinations retain their canvas destination. Canvas, sessions and the existing proxy view remain reachable. CanvasSurface.tsx holds the existing canvas at 489 lines. Its arrangement behavior was moved without changing the canvas assertions. The static control-count fixture moves the existing five controls to that child module.

The proposed kept member is `arrangement.operations = { view, selected, pane_width }`. Views are tree, canvas and sessions. Selected is a seat name or null. Width is 280–1200. Absence opens the tree. Refusals are named. No browser copy is substituted for this choice.

No Rust, dependency, package-lock, service data or installation file was changed. No cargo, packaging, installation, identity stop or push command was run. The existing canvas test files were not edited.

## Definition of done

| Requirement | Evidence |
| --- | --- |
| TypeScript | Final `npm run typecheck`: exit 0. |
| New components | components-final: exit 1; 17 tests, 16 passed and 1 failed. The unchanged 50 ms assertion measured 57.60 ms. |
| Entire Vitest suite | suite-last: exit 1; 122 files, 108 passed and 14 failed; 1,082 tests, 1,018 passed and 64 failed. |
| Exact starting source | baseline-exact: exit 1; 120 files, 116 passed and 4 failed; 1,065 tests, 1,055 passed and 10 failed. Lane bytes were held in memory, the lane's new source files were absent for this run, and every lane file was restored in finally. |
| Failure split | The 10 baseline failures remain. The final source adds 53 failures in existing tests that assume the former default or exactly two views, plus the new rendering-clock failure at 228.71 ms. |
| C737 | Functional tree and start checks pass. The timing assertion does not. Unchecked. |
| C738 | Component tests prove PTY and managed cancellation, send, confirmed/unconfirmed stop, offline/unknown reasons, cursor/session binding and draft retention. Checked with test names. The byte-sink double does not prove GPU pixels. |
| C739 | Default, fixture restoration, one PUT per change, named refusals and deep-link compatibility are component-proven. Today's server cannot store the new fields. Unchecked. |
| Design gate | Final output is parsed for FAIL and rendered-markdown differences. A gate exit alone is not the proof. |
| Push | Prohibited by the fixed header; the full suite is also red. No remote readback is claimed. |

## RED

The first R1/R2 run collected no tests because their new components did not exist: two failed suites, exit 1. NetworkTree.test.tsx reported `Failed to resolve import ./NetworkTree`; NetworkTreePane.test.tsx reported `Failed to resolve import ./NetworkTreePane`. These are collection reds. The initial nine behavioral test bodies had not executed.

The first default-view run failed `expected null not to be null`: exit 1, 1 failed and 4 skipped. That fixture initially omitted SERVICE and was corrected. Its first failure is retained and is not claimed as a valid authenticated-screen behavioral red.

`C738 preserves a new draft written while the earlier user turn is being delivered`: red-draft exited 1, 1 failed and 5 skipped; `expected '' to be 'next turn'`. It passes after the implementation change in components-final and suite-last.

`C739 an explicit canvas layout or tool-panel address keeps its canvas destination`: red-layout-address exited 1, 1 failed and 8 skipped; `expected 'Network' to be 'Canvas'`. It passes after the implementation change in components-final and suite-last.

`C737 renders ten machines and two hundred seats within 50 ms of React render work`: its first clock failure in green-new-4 was `expected 374.58146199992734 to be less than 50`. DOM construction was reduced without reducing the 200-seat fixture or raising 50 ms. green-new-8 passed all 16 tests, including that assertion. The final assertion still fails; a single earlier pass is not proof under shared load.

Additional tests first passed when added. No retroactive red claim is made for them.

## FOUND

1. Server persistence is blocked at crates/lys-identity-server/src/canvas_store.rs:188–209. Arrangement uses `serde(deny_unknown_fields)`. The proposed exact member before its closing brace is:

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub operations: Option<OperationsChoice>,
```

The same module needs a typed choice and view enum with these exact wire members:

```rust
pub view: OperationsView,
pub selected: Option<String>,
pub pane_width: f64,
```

Accepted view strings are tree, canvas and sessions. The server owner must validate finite width 280–1200 and the optional selected-seat name, preserve marks and saved layouts, update the served schema, and test old-install reading/migration plus PUT/GET/restart preservation. No Rust line was edited. The refusal fixture is `RequestMalformed: unknown field operations`; this is fixture evidence, not a live-service observation.

2. The requirement to keep existing canvas tests unchanged conflicts with requiring their plain `#/canvas` visit to render a tree. Their canvas assertions can be preserved by explicitly choosing the canvas in their inputs; proposed exact lines are recorded below. The assertion in tests/proxy-view.test.tsx that exactly Canvas and Proxy exist also needs to cover Network, Canvas, Sessions and Proxy. None of those test files was edited.

3. The hard 50 ms wall-clock acceptance fails under shared load. The final full-suite measurement is 228.71 ms and the final focused measurement is 57.60 ms. No threshold was raised, no fixture reduced and no serial marker or wait added.

4. The exact baseline has 10 unrelated failures: five in agent-run, one in cannot-start, two in settings-form and two in bare-buttons. Corresponding implementations were not changed. The bare act buttons are in features/dashboard/Alarms.tsx and features/runtime/Terminal.tsx; these need the shared Act control. Changing the standing count to hide them would weaken the rule.

5. The fixed header prohibits git push, cargo, installations and surface packaging. The release lane requires those actions. The later AGENTS-007 job explicitly forbids cargo/package/install and says another seat installs. The surface job was followed. Push-when-green conflicts with the fixed no-push rule; no push was attempted.

6. `npm run check` is absent from package.json. It exited 1: Missing script: check. The existing `npm run typecheck` runs tsc and exited 0. No dependency or redundant script was added.

7. CanvasSurface.tsx, vite.config.ts and bare-buttons.standing.ts were not named in the brief's file list. The under-500 rule required splitting the existing 560-line source. Vitest's include excluded the expressly named src tests. The control-count fixture needed its existing path moved to the child module. These supporting surface edits preserve the existing assertions.

## NOT CONFIRMED

Service persistence, 50 ms rendering under shared load, a green complete suite, independent rendered-screen review, GPU/VT pixels, live seat acts, surface package/install, release binaries, service health, boot-stage timings and remote readback are not confirmed. No Rust or package leg ran. No formatter command was named for the surface and none was run. The repository's renderer was used for the owned design Markdown.

If this code were used in a hospital, the worst credible failure is a seat act reaching a replacement session while the pane still shows the earlier session context. The exposed API addresses a seat rather than an expected session for send and stop. The number of people affected cannot be estimated from this repository; it depends on the workload delegated to that seat.

## Every check and proposed test-input changes

The following ledger is generated from the complete tool logs. The log directory is /Users/tom/Developer/.scratch/waffles/night-20261012/logs/LYS-AGENTS-007/. commands.jsonl records every command and its own exit. Counts are the test tool's terminal summary, not an inference from the wrapper.

| Log | Command | Exit | Full reported counts |
| --- | --- | --- | --- |
| check-script | npm run check | 1 | Missing check script; 0 tests |
| red-tree-pane | npx vitest run src/features/runtime/NetworkTree.test.tsx src/features/runtime/NetworkTreePane.test.tsx | 1 | Test Files  2 failed (2); Tests  no tests |
| red-tree-pane | npx vitest run src/features/runtime/NetworkTree.test.tsx src/features/runtime/NetworkTreePane.test.tsx | 1 | Test Files  2 failed (2); Tests  no tests |
| typecheck-1 | npm run typecheck | 1 | No test count reported |
| green-tree-pane-1 | npx vitest run src/features/runtime/NetworkTree.test.tsx src/features/runtime/NetworkTreePane.test.tsx | 1 | Test Files  1 failed  /  1 passed (2); Tests  1 failed  /  8 passed (9) |
| red-default | npx vitest run src/features/runtime/NetworkTree.test.tsx -t C739 | 1 | Test Files  1 failed (1); Tests  1 failed  /  4 skipped (5) |
| typecheck-2 | npm run typecheck | 1 | No test count reported |
| green-new-2 | npx vitest run src/features/runtime/NetworkTree.test.tsx src/features/runtime/NetworkTreePane.test.tsx | 1 | Test Files  1 failed  /  1 passed (2); Tests  1 failed  /  9 passed (10) |
| default-debug | npx vitest run src/features/runtime/NetworkTree.test.tsx -t C739 | 1 | Test Files  1 failed (1); Tests  1 failed  /  4 skipped (5) |
| full-vitest-1 | npx vitest run | 1 | Test Files  14 failed  /  108 passed (122); Tests  66 failed  /  1009 passed (1075) |
| typecheck-3 | npm run typecheck | 0 | No test count reported |
| green-new-3 | npx vitest run src/features/runtime/NetworkTree.test.tsx src/features/runtime/NetworkTreePane.test.tsx | 0 | Test Files  2 passed (2); Tests  10 passed (10) |
| design-gate-before | scripts/design/gate.sh | 1 | No test count reported |
| baseline-vitest | npx vitest run (starting source restored afterward) | 1 | Test Files  4 failed  /  116 passed (120); Tests  10 failed  /  1055 passed (1065) |
| typecheck-4 | npm run typecheck | 0 | No test count reported |
| green-new-4 | npx vitest run src/features/runtime/NetworkTree.test.tsx src/features/runtime/NetworkTreePane.test.tsx tests/bare-buttons.test.tsx | 1 | Test Files  2 failed  /  1 passed (3); Tests  4 failed  /  12 passed (16) |
| red-draft | npx vitest run src/features/runtime/NetworkTreePane.test.tsx -t "preserves a new draft" | 1 | Test Files  1 failed (1); Tests  1 failed  /  5 skipped (6) |
| green-new-5 | npx vitest run src/features/runtime/NetworkTree.test.tsx src/features/runtime/NetworkTreePane.test.tsx | 1 | Test Files  2 failed (2); Tests  3 failed  /  11 passed (14) |
| green-new-6 | npx vitest run src/features/runtime/NetworkTree.test.tsx src/features/runtime/NetworkTreePane.test.tsx | 1 | Test Files  1 failed  /  1 passed (2); Tests  1 failed  /  13 passed (14) |
| green-new-7 | npx vitest run src/features/runtime/NetworkTree.test.tsx src/features/runtime/NetworkTreePane.test.tsx | 1 | Test Files  1 failed  /  1 passed (2); Tests  1 failed  /  13 passed (14) |
| typecheck-5 | npm run typecheck | 0 | No test count reported |
| green-new-8 | npx vitest run src/features/runtime/NetworkTree.test.tsx src/features/runtime/NetworkTreePane.test.tsx | 0 | Test Files  2 passed (2); Tests  16 passed (16) |
| baseline-exact | npx vitest run (starting source restored afterward) | 1 | Test Files  4 failed  /  116 passed (120); Tests  10 failed  /  1055 passed (1065) |
| design-gate-final | scripts/design/gate.sh | 0 | No test count reported |
| typecheck-final | npm run typecheck | 0 | No test count reported |
| full-vitest-final | npx vitest run | 1 | Test Files  14 failed  /  108 passed (122); Tests  66 failed  /  1015 passed (1081) |
| red-layout-address | npx vitest run src/features/runtime/NetworkTree.test.tsx -t "explicit canvas layout" | 1 | Test Files  1 failed (1); Tests  1 failed  /  8 skipped (9) |
| typecheck-last | npm run typecheck | 0 | No test count reported |
| components-final | npx vitest run src/features/runtime/NetworkTree.test.tsx src/features/runtime/NetworkTreePane.test.tsx | 1 | Test Files  1 failed  /  1 passed (2); Tests  1 failed  /  16 passed (17) |
| suite-last | npx vitest run | 1 | Test Files  14 failed  /  108 passed (122); Tests  64 failed  /  1018 passed (1082) |
| design-gate-last | scripts/design/gate.sh | 0 | No test count reported |

### Exact proposed inputs, not applied

- surface/identity/tests/canvas-cover.test.tsx:48: exact proposed line: await mount('#/canvas?view=canvas', routes);
- surface/identity/tests/canvas-cover.test.tsx:60: exact proposed line: await mount('#/canvas?view=canvas', routes);
- surface/identity/tests/canvas-marks.test.tsx:50: exact proposed line: await mount('#/canvas?view=canvas', routes);
- surface/identity/tests/canvas-marks.test.tsx:73: exact proposed line: await mount('#/canvas?view=canvas', routes);
- surface/identity/tests/canvas-marks.test.tsx:105: exact proposed line: await mount('#/canvas?view=canvas', routes);
- surface/identity/tests/canvas-marks.test.tsx:132: exact proposed line: await mount('#/canvas?view=canvas', routes);
- surface/identity/tests/canvas-marks.test.tsx:142: exact proposed line: await mount('#/canvas?view=canvas', { ...routes, '/runtime/live': ok({ sessions: [{ ...running, session: again }], unanswered: [] }) });
- surface/identity/tests/canvas-marks.test.tsx:150: exact proposed line: await mount('#/canvas?view=canvas', routes);
- surface/identity/tests/canvas-marks.test.tsx:176: exact proposed line: await mount('#/canvas?view=canvas', routes);
- surface/identity/tests/canvas-marks.test.tsx:181: exact proposed line: await mount('#/canvas?view=canvas', routes);
- surface/identity/tests/canvas-marks.test.tsx:193: exact proposed line: const { posted } = await mount('#/canvas?view=canvas', {
- surface/identity/tests/canvas-marks.test.tsx:237: exact proposed line: await mount('#/canvas?view=canvas', { ...routes, '/canvas': refused(503, 'CanvasUnavailable', 'the canvas store is not open') });
- surface/identity/tests/canvas-marks.test.tsx:248: exact proposed line: await mount('#/canvas?view=canvas', { ...routes, '/canvas': ok({ arrangement: null, layouts: [] }), 'PUT /canvas': refused(503, 'CanvasUnavailable', 'the canvas store is not open') });
- surface/identity/tests/canvas-marks.test.tsx:255: exact proposed line: await mount('#/canvas?view=canvas', routes);
- surface/identity/tests/canvas-teams.test.tsx:111: exact proposed line: await mount('#/canvas?view=canvas', routes);
- surface/identity/tests/canvas-teams.test.tsx:127: exact proposed line: await mount('#/canvas?view=canvas', { ...SERVICE, '/runtime/live': ok({ sessions: [running], unanswered: [] }) });
- surface/identity/tests/canvas-tidy.test.tsx:124: exact proposed line: await mount('#/canvas?view=canvas', routes);
- surface/identity/tests/canvas-tidy.test.tsx:150: exact proposed line: await mount('#/canvas?view=canvas', routes);
- surface/identity/tests/canvas-widgets.test.tsx:48: exact proposed line: await mount('#/canvas?view=canvas', routes);
- surface/identity/tests/canvas-widgets.test.tsx:79: exact proposed line: await mount('#/canvas?view=canvas', routes);
- surface/identity/tests/canvas-widgets.test.tsx:104: exact proposed line: await mount('#/canvas?view=canvas', routes);
- surface/identity/tests/canvas-widgets.test.tsx:142: exact proposed line: await mount('#/canvas?view=canvas', { ...routes, '/dashboard': ok(dashboard({ [SCRIBE]: { usage: usage(SCRIBE, figures, accounts) } })) });
- surface/identity/tests/canvas-widgets.test.tsx:235: exact proposed line: await mount('#/canvas?view=canvas', { ...routes, '/dashboard': ok(dashboard({ [SCRIBE]: { usage: usage(SCRIBE, [used('tokens', 'day', 120000)]) }, [COURIER]: { usage: usage(COURIER, [used('tokens', 'day', 30000)]) } })) });
- surface/identity/tests/canvas-widgets.test.tsx:263: exact proposed line: await mount('#/canvas?view=canvas', routes);
- surface/identity/tests/canvas-widgets.test.tsx:267: exact proposed line: await mount('#/canvas?view=canvas', { ...routes, '/dashboard': refused(503, 'DashboardUnavailable', 'the dashboard is closed') });
- surface/identity/tests/canvas-widgets.test.tsx:274: exact proposed line: await mount('#/canvas?view=canvas', routes);
- surface/identity/tests/canvas-widgets.test.tsx:299: exact proposed line: await mount('#/canvas?view=canvas', { ...routes, '/runtime/live': ok({ sessions: [running, other], unanswered: [] }), ...input(session), ...input(other.session) });
- surface/identity/tests/canvas-widgets.test.tsx:322: exact proposed line: await mount('#/canvas?view=canvas', { ...routes, '/runtime/live': ok({ sessions: [running, other], unanswered: [] }), ...input(session), ...input(other.session, refused(409, 'RuntimeSessionEnded', 'the session has ended')) });
- surface/identity/tests/canvas-widgets.test.tsx:339: exact proposed line: await mount('#/canvas?view=canvas', routes);
- surface/identity/tests/canvas-widgets.test.tsx:357: exact proposed line: await mount('#/canvas?view=canvas', routes);
- surface/identity/tests/canvas-widgets.test.tsx:381: exact proposed line: await mount('#/canvas?view=canvas', routes);
- surface/identity/tests/canvas-widgets.test.tsx:423: exact proposed line: await mount('#/canvas?view=canvas', routes);
- surface/identity/tests/canvas-widgets.test.tsx:443: exact proposed line: await mount('#/canvas?view=canvas', routes);
- surface/identity/tests/proxy-view.test.tsx:41: exact proposed line: await mount('#/canvas?view=canvas', routes);
- surface/identity/tests/runtime-terminal.test.tsx:44: exact proposed line: const { posted } = await mount('#/canvas?view=canvas', {
- surface/identity/tests/runtime-terminal.test.tsx:112: exact proposed line: const { requests } = await mount('#/canvas?view=canvas', { ...SERVICE, ['POST ' + base + '/resize']: ok({ receipt: { index: 0 } }), '/runtime/live': ok({ sessions: [running], unanswered: [] }) });
- surface/identity/tests/runtime-terminal.test.tsx:121: exact proposed line: await mount('#/canvas?view=canvas', { ...SERVICE, ['POST ' + base + '/resize']: ok({ receipt: { index: 0 } }), '/runtime/live': ok({ sessions: [{ ...running, shown: 'stopped' }], unanswered: [] }) });
- surface/identity/tests/runtime-terminal.test.tsx:127: exact proposed line: await mount('#/canvas?view=canvas', { ...SERVICE, ['POST ' + base + '/resize']: ok({ receipt: { index: 0 } }), '/runtime/live': ok({ sessions: [running], unanswered }) });
- surface/identity/tests/runtime-terminal.test.tsx:135: exact proposed line: await mount('#/canvas?view=canvas', { ...SERVICE, ['POST ' + base + '/resize']: ok({ receipt: { index: 0 } }), '/runtime/live': ok({ sessions: [running] }) });
- surface/identity/tests/runtime-terminal.test.tsx:140: exact proposed line: await mount('#/canvas?view=canvas', { ...SERVICE, ['POST ' + base + '/resize']: ok({ receipt: { index: 0 } }), '/runtime/live': refused(503, 'RuntimeUnavailable', 'No reports store') });
- surface/identity/tests/session-canvas.test.tsx:26: exact proposed line: const { posted } = await mount('#/canvas?view=canvas', routes);
- surface/identity/tests/session-canvas.test.tsx:38: exact proposed line: await mount('#/canvas?view=canvas', {
- surface/identity/tests/session-canvas.test.tsx:50: exact proposed line: await mount('#/canvas?view=canvas', { ...routes, '/grants': ok({ grants: GRANTS.map((grant) => grant.holder === SCRIBE ? { ...grant, standing: { stands: false, refusal: 'GrantRevoked', grant: grant.id, reason: 'Its source was revoked' } } : grant), revision: 8 }) });
- surface/identity/tests/session-canvas.test.tsx:56: exact proposed line: await mount('#/canvas?view=canvas', { ...routes, '/grants': refused(403, 'GrantReadRefused', 'These grants are not visible') });
- surface/identity/tests/session-canvas.test.tsx:64: exact proposed line: await mount('#/canvas?view=canvas', { ...routes, '/runtime/live': refused(503, 'RuntimeUnavailable', 'Reports could not be read') });
- surface/identity/tests/session-canvas.test.tsx:71: exact proposed line: await mount('#/canvas?view=canvas', { ...routes, '/runtime/live': refused(401, 'NotSignedIn', 'Sign in first') });
- surface/identity/tests/session-canvas.test.tsx:77: exact proposed line: await mount('#/canvas?view=canvas', { ...routes, '/runtime/live': ok({ sessions: [], unanswered: [] }) });
- surface/identity/tests/session-canvas.test.tsx:86: exact proposed line: await mount('#/canvas?view=canvas', { ...routes, '/runtime/live': ok({ sessions: [running], unanswered: [{ session, machine: 'machine-one', refusal: 'runner_unreachable', reason: 'Socket closed' }] }) });
- surface/identity/tests/session-canvas.test.tsx:97: exact proposed line: await mount('#/canvas?view=canvas', { ...routes, '/runtime/live': ok({ sessions: [running], unanswered: [{ session, machine: 'machine-one', refusal: 'runner_unreachable', reason: 'Socket closed' }] }),
- surface/identity/tests/session-canvas.test.tsx:109: exact proposed line: const { posted } = await mount('#/canvas?view=canvas', {
- surface/identity/tests/session-canvas.test.tsx:129: exact proposed line: await mount('#/canvas?view=canvas', routes);
- surface/identity/tests/session-canvas.test.tsx:158: exact proposed line: await mount('#/canvas?view=canvas', {
- surface/identity/tests/session-canvas.test.tsx:170: exact proposed line: await mount('#/canvas?view=canvas', routes);
- surface/identity/tests/session-canvas.test.tsx:194: exact proposed line: await mount('#/canvas?view=canvas', routes);
- surface/identity/tests/session-canvas.test.tsx:217: exact proposed line: await mount('#/canvas?view=canvas', routes);
- surface/identity/tests/session-canvas.test.tsx:225: exact proposed line: await mount('#/canvas?view=canvas', { ...routes, ['POST ' + base + '/resize']: ok({ receipt: { index: 1 } }),
- surface/identity/tests/session-canvas.test.tsx:239: exact proposed line: await mount('#/canvas?view=canvas', { ...routes, '/teams': refused(503, 'TeamsUnavailable', 'Teams could not be read') });
- surface/identity/tests/session-canvas.test.tsx:247: exact proposed line: await mount('#/canvas?view=canvas', routes);
- surface/identity/tests/usage-partial.test.tsx:38: exact proposed line: await mount('#/canvas?view=canvas', { ...routes, '/dashboard': ok(dashboard({ [SCRIBE]: { usage } })) });

Final design-gate-delivery: exit 0; FAIL lines 0; rendered-markdown differences 0.

The shared HEAD advanced to 5a51e0ea88eb0b8aeec9c7be89b1f6d568399755 during this lane. Its diff from the start has no surface/identity or docs/design/agents paths. Foreign Rust changes remain uncommitted and were not staged by this lane.
