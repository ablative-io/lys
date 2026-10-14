/** DIRECTORY-051 R6: an agent's refused tool calls, newest first, with what the list does not yet cover. */
import { afterEach, describe, expect, it } from 'vitest';
import { $, $$, mount, text, unmountAll } from './harness';
import { SCRIBE, SERVICE, ok, refused } from './fixtures';
import type { RefusalRecord, RefusalsView } from '../src/features/file/policyContract';
import type { Machine } from '../src/features/network/contract';

const policy = '/agents/' + SCRIBE + '/policy';
const refusals = '/agents/' + SCRIBE + '/refusals';
const empty = { agent: SCRIBE, policy: null, digest: null, applies: 'applies on the agent\'s next launch' };

afterEach(() => { unmountAll(); document.body.innerHTML = ''; });

const record = (attempt: string, at: number, grantable: boolean): RefusalRecord => ({
  version: 1, source: 'runner-one', attempt, session: 'session-1', agent: SCRIBE, at, tool: 'Bash', target: 'rm', policy_version: 2,
  rule: grantable ? 'secrets-dir' : 'no-shell', check: 'policy_denied', grantable,
  permission: grantable ? { rule: 'secrets-dir', resource: { kind: 'secret', id: 'vault' }, action: 'read' } : null,
  grantor: null, words: grantable ? 'Rule secrets-dir denies this call' : 'Rule no-shell denies this call',
});

const computer = (id: string, name: string, state: Machine['state']): Machine => ({
  id, name, state, kind: 'desktop', runtime: null, slots: 1, may_run: [], may_reach: [], named_by: 'person-test', named_at: 0,
  retired_at: state === 'retired' ? 1 : null, last_report_at: null,
});

describe('Agent refused tool calls', () => {
  it('lists refusals in the order answered, newest first, and says a hard rule is not grantable', async () => {
    const view: RefusalsView = { agent: SCRIBE, refusals: [record('b', 1790000002000, true), record('a', 1790000001000, false)], read_from: ['runner-one'] };
    await mount('#/file/' + SCRIBE + '/policy', { ...SERVICE, [policy]: ok(empty), [refusals]: ok(view),
      '/network': ok({ machines: [computer('runner-one', 'Studio', 'in_use')], reports_served: true }) });
    const shown = $$('[data-refusal]').map((row) => row.textContent ?? '');
    expect(shown).toHaveLength(2);
    expect(shown[0]).toContain('secrets-dir');
    expect(shown[0]).toContain('grantable by a grant of read on secret vault');
    expect(shown[1]).toContain('not grantable');
    expect(text()).toContain('Refusals on any other runner are not read here');
    expect(text()).not.toMatch(/sandbox/i);
  });

  it('names the computers read and omits retired computers without removing their refusal records', async () => {
    const first = 'op-' + '1'.repeat(32), retired = 'op-' + '2'.repeat(32), last = 'op-' + '3'.repeat(32);
    const view: RefusalsView = { agent: SCRIBE, refusals: [{ ...record('b', 1790000002000, true), source: retired },
      { ...record('a', 1790000001000, false), source: first }], read_from: [first, retired, last, first] };
    await mount('#/file/' + SCRIBE + '/policy', { ...SERVICE, [policy]: ok(empty), [refusals]: ok(view),
      '/network': ok({ machines: [computer(first, 'Studio', 'in_use'), computer(retired, 'Old laptop', 'retired'), computer(last, 'Laptop', 'in_use')], reports_served: true }) });
    expect($('[data-coverage]')?.textContent).toBe('Read from runners on Studio, Laptop. Refusals on any other runner are not read here.');
    const shown = $$('[data-refusal]').map((row) => row.textContent ?? '');
    expect(shown).toHaveLength(2);
    expect(shown[0]).toContain('secrets-dir');
    expect(shown[1]).toContain('no-shell');
    expect($('[data-coverage]')?.textContent).not.toMatch(/op-|Old laptop/);
  });

  it('does not claim coverage of a current computer when only retired runners were read', async () => {
    const view: RefusalsView = { agent: SCRIBE, refusals: [], read_from: ['retired-runner'] };
    await mount('#/file/' + SCRIBE + '/policy', { ...SERVICE, [policy]: ok(empty), [refusals]: ok(view),
      '/network': ok({ machines: [computer('retired-runner', 'Old laptop', 'retired')], reports_served: true }) });
    expect($('[data-coverage]')?.textContent).toContain('does not confirm coverage of any current computer');
    expect($('[data-coverage]')?.textContent).not.toMatch(/Old laptop|retired-runner|unknown/);
  });

  it('names an unlisted runner as unknown coverage without showing its raw identifier', async () => {
    const view: RefusalsView = { agent: SCRIBE, refusals: [], read_from: ['runner-one', 'op-' + '4'.repeat(32)] };
    await mount('#/file/' + SCRIBE + '/policy', { ...SERVICE, [policy]: ok(empty), [refusals]: ok(view),
      '/network': ok({ machines: [computer('runner-one', 'Studio', 'in_use')], reports_served: true }) });
    expect($('[data-coverage]')?.textContent).toContain('Read from runners on Studio.');
    expect($('[data-coverage]')?.textContent).toContain('1 runner is not listed in the computer network, so its name and retirement state are unknown.');
    expect($('[data-coverage]')?.textContent).not.toContain('op-');
  });

  it('keeps the refusals visible and names the reason when computer names cannot be read', async () => {
    const view: RefusalsView = { agent: SCRIBE, refusals: [record('a', 1790000001000, false)], read_from: ['runner-one'] };
    await mount('#/file/' + SCRIBE + '/policy', { ...SERVICE, [policy]: ok(empty), [refusals]: ok(view),
      '/network': refused(503, 'NetworkUnavailable', 'Computer directory is offline.') });
    expect($$('[data-refusal]')).toHaveLength(1);
    expect($('[data-coverage]')?.textContent).toContain('Computer names could not be read');
    expect($('[data-coverage]')?.textContent).toContain('Computer directory is offline.');
    expect($('[data-coverage]')?.textContent).not.toContain('runner-one');
  });

  it('names an unreadable computer answer instead of silently omitting a runner', async () => {
    const view: RefusalsView = { agent: SCRIBE, refusals: [record('a', 1790000001000, false)], read_from: ['runner-one'] };
    await mount('#/file/' + SCRIBE + '/policy', { ...SERVICE, [policy]: ok(empty), [refusals]: ok(view),
      '/network': ok({ machines: [{ ...computer('runner-one', 'Studio', 'in_use'), state: 'unknown' }], reports_served: true }) });
    expect($$('[data-refusal]')).toHaveLength(1);
    expect($('[data-coverage]')?.textContent).toContain('Computer names could not be read');
    expect($('[data-coverage]')?.textContent).toContain('The service did not answer valid computer names and retirement states.');
    expect($('[data-coverage]')?.textContent).not.toContain('Read from runners on');
  });

  it('says the list may be incomplete while no runner has been read', async () => {
    await mount('#/file/' + SCRIBE + '/policy', { ...SERVICE, [policy]: ok(empty), [refusals]: ok({ agent: SCRIBE, refusals: [], read_from: [] }) });
    expect(text()).toContain('this list may be incomplete');
    expect(text()).toContain('No refused tool calls have been read.');
  });
});
