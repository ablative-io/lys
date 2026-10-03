/** A start is confirmed only by a receipt that names the kept request, its profile version and a runner that ran it; a command alone is not a running agent. */
import { beforeEach, describe, expect, it } from 'vitest';
import { serve } from './harness';
import { SCRIBE, ok, refused } from './fixtures';
import type { Route } from './fixtures';
import { pendingStartOf, startRequest } from '../src/features/runtime/start-requests';
import type { Pending } from '../src/features/runtime/start-requests';

const prefix = '/agents/' + SCRIBE;
const path = prefix + '/start-command';
const machine = 'machine-one';
const operation = 'op-' + 'c'.repeat(32);
const kept: Pending = { stage: 'start', path, body: { machine, operation }, machine, version: 2 };
const receipt = { agent: SCRIBE, machine, runtime: 'test-runtime', session: operation, provisioning_version: 2, harness: 'test', handles: [], template: '{}', template_sha256: 'abc', command: 'test-runtime --session recorded', left_out: [], executed: false };
const running = { session: operation, state: 'running', pid: 1, started_at: 1 };
function answering(route: Route) {
  const posted: { path: string; body: unknown }[] = [];
  serve({ ['POST ' + path]: route }, posted);
  return posted;
}
beforeEach(() => sessionStorage.clear());

describe('The start request', () => {
  it('sends the kept request as it is and returns the receipt of a runner that ran it', async () => {
    const posted = answering(ok({ ...receipt, runner: running }));
    const answer = await startRequest(SCRIBE, kept);
    expect(posted).toEqual([{ path, body: { machine, operation } }]);
    expect(answer.runner?.state).toBe('running');
  });
  it('does not call a command with no runner a start', async () => {
    answering(ok(receipt));
    await expect(startRequest(SCRIBE, kept)).rejects.toThrow('RunnerStartUnconfirmed');
  });
  it('refuses a receipt naming a different session', async () => {
    answering(ok({ ...receipt, session: 'wrong-session', runner: running }));
    await expect(startRequest(SCRIBE, kept)).rejects.toThrow('StartReceiptMismatch');
  });
  it('refuses a runner answer naming a different session', async () => {
    answering(ok({ ...receipt, runner: { ...running, session: 'wrong-session' } }));
    await expect(startRequest(SCRIBE, kept)).rejects.toThrow('StartReceiptMismatch');
  });
  it('refuses a receipt naming another profile version', async () => {
    answering(ok({ ...receipt, provisioning_version: 3, runner: running }));
    await expect(startRequest(SCRIBE, kept)).rejects.toThrow('StartVersionChanged');
  });
  it('passes the service\'s own refusal on by its name', async () => {
    answering(refused(409, 'AgentNotActive', 'This agent is suspended'));
    await expect(startRequest(SCRIBE, kept)).rejects.toMatchObject({ refusal: { refusal: 'AgentNotActive' } });
  });
});

describe('A kept request read back', () => {
  const key = 'lys.pending.agent-start.person.' + SCRIBE;
  it('is returned as it was kept', () => {
    expect(pendingStartOf(JSON.parse(JSON.stringify(kept)), key, prefix)).toEqual({ ...kept, legacyKey: undefined });
  });
  it('is refused by name when it is not a request', () => {
    expect(() => pendingStartOf(null, key, prefix)).toThrow('PendingStartUnreadable');
  });
  it('is refused when it names another agent\'s address', () => {
    expect(() => pendingStartOf({ ...kept, path: '/agents/other/start-command' }, key, prefix)).toThrow('PendingStartUnreadable');
  });
  it('is refused when its body and its record name two computers', () => {
    expect(() => pendingStartOf({ ...kept, machine: 'machine-two' }, key, prefix)).toThrow('PendingStartUnreadable');
  });
});
