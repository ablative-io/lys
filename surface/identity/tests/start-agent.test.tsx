/** Launch requests retain their operation and never treat command generation as process execution. */
import { act } from 'react';
import { createRoot } from 'react-dom/client';
import type { Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { $, click, serve, settle, text } from './harness';
import { ADA, SCRIBE, SERVICE, ok, refused } from './fixtures';
import type { Route } from './fixtures';
import { StartAgent } from '../src/features/runtime/StartAgent';
import type { ProvisioningProfile } from '../src/features/provisioning/Provisioning';
const reviewed = { version: 2, harness: { name: 'Claude Code' }, reviewed_by: ADA } as unknown as ProvisioningProfile;
const path = '/agents/' + SCRIBE + '/start-command';
const machine = { id: 'machine-one', name: 'Test machine', runtime: 'test-runtime', state: 'in_use', may_run: [{ id: SCRIBE }] };
let root: Root | null = null;
async function mountStart(post: Route, extra: Record<string, Route> = {}) {
  const posted: { path: string; body: unknown }[] = [];
  serve({ ...SERVICE, '/roles': ok({ roles: [] }), '/network': ok({ machines: [machine], reports_served: true }), ['POST ' + path]: post, ...extra }, posted);
  const element = document.createElement('div'); document.body.appendChild(element); root = createRoot(element);
  await act(async () => { root?.render(<StartAgent agent={SCRIBE} profile={reviewed} />); }); await settle(); return posted;
}
function close() { act(() => root?.unmount()); root = null; document.body.innerHTML = ''; }
const button = (label: string) => [...document.querySelectorAll('button')].find((entry) => entry.textContent === label) ?? null;
async function submit() { await click(button('Start')); }
const receipt = (body: unknown) => { const request = body as Record<string, unknown>; return ok({ agent: SCRIBE, machine: request.machine, runtime: 'test-runtime', session: request.operation, provisioning_version: 2, harness: 'test', handles: [], template: '{}', template_sha256: 'abc', command: 'test-runtime --session recorded', left_out: [], executed: false }); };
beforeEach(() => sessionStorage.clear()); afterEach(close);
describe('Prepare start', () => {
  it('records only on request and shows unconfirmed with the exact operation', async () => {
    const posted = await mountStart(receipt); expect(posted).toEqual([]); await submit();
    expect(posted).toMatchObject([{ path, body: { machine: machine.id, operation: expect.stringMatching(/^op-/) } }]);
    expect(text()).toContain('run this command there yourself'); expect(text()).not.toContain('Reported running');
  });
  it('chooses the only permitted computer and says what pressing Start does', async () => {
    await mountStart(receipt); expect(($('select') as HTMLSelectElement).value).toBe(machine.id);
    expect(text()).toContain('Pressing Start makes Lys ask the runner on Test machine to start this agent');
  });
  it('after a runner start offers the terminal and no command to run again', async () => {
    await mountStart((body) => { const answer = receipt(body); return ok({ ...(answer.body as Record<string, unknown>), runner: { session: 's', state: 'running', pid: 1, started_at: 1 } }); }); await submit();
    expect(text()).toContain('Its Lys runner has it running'); expect(button('Copy command')).toBeNull(); expect($('textarea')).toBeNull();
  });
  it('retains and resends the original operation after an uncertain answer', async () => {
    const first = await mountStart(refused(503, 'RuntimeUnavailable', 'Unknown outcome')); await submit(); close();
    const second = await mountStart(receipt); await click(button('Check original change')); expect(second).toEqual(first); expect(text()).toContain('run this command there yourself');
  });
  it('refuses a receipt naming a different session', async () => {
    await mountStart((body) => { const answer = receipt(body); return ok({ ...(answer.body as Record<string, unknown>), session: 'wrong-session' }); }); await submit();
    expect(text()).toContain('original request is retained'); expect(text()).not.toContain('run this command there yourself');
  });
  it('keeps a named inactive refusal visible without a command', async () => {
    await mountStart(refused(409, 'AgentNotActive', 'This agent is suspended')); await submit();
    expect(text()).toContain('AgentNotActive'); expect($('textarea')).toBeNull(); expect(sessionStorage.getItem('lys.pending.start.' + ADA + '.' + SCRIBE)).toBeNull();
  });
  it('offers a machine through an active held role and excludes an ended role', async () => {
    const role = { id: 'role-build', holders: [{ holder: SCRIBE, state: 'holding' }] };
    const extra = { '/network': ok({ machines: [{ ...machine, may_run: [], may_run_roles: [role.id] }], reports_served: true }), '/roles': ok({ roles: [role] }) };
    const posted = await mountStart(receipt, extra); await submit(); expect(posted).toHaveLength(1); close();
    await mountStart(receipt, { ...extra, '/roles': ok({ roles: [{ ...role, holders: [{ holder: SCRIBE, state: 'ended' }] }] }) }); expect(text()).toContain('Let it use a computer'); expect(text()).toContain('Test machine: this agent is not allowed on it');
  });

});
