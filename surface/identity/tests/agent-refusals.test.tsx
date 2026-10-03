/** DIRECTORY-051 R6: an agent's refused tool calls, newest first, with what the list does not yet cover. */
import { afterEach, describe, expect, it } from 'vitest';
import { $$, mount, text, unmountAll } from './harness';
import { SCRIBE, SERVICE, ok } from './fixtures';
import type { RefusalRecord, RefusalsView } from '../src/features/file/policyContract';

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

describe('Agent refused tool calls', () => {
  it('lists refusals in the order answered, newest first, and says a hard rule is not grantable', async () => {
    const view: RefusalsView = { agent: SCRIBE, refusals: [record('b', 1790000002000, true), record('a', 1790000001000, false)], read_from: ['runner-one'] };
    await mount('#/file/' + SCRIBE + '/policy', { ...SERVICE, [policy]: ok(empty), [refusals]: ok(view) });
    const shown = $$('[data-refusal]').map((row) => row.textContent ?? '');
    expect(shown).toHaveLength(2);
    expect(shown[0]).toContain('secrets-dir');
    expect(shown[0]).toContain('grantable by a grant of read on secret vault');
    expect(shown[1]).toContain('not grantable');
    expect(text()).toContain('Refusals on any other runner are not read here');
    expect(text()).not.toMatch(/sandbox/i);
  });

  it('says the list may be incomplete while no runner has been read', async () => {
    await mount('#/file/' + SCRIBE + '/policy', { ...SERVICE, [policy]: ok(empty), [refusals]: ok({ agent: SCRIBE, refusals: [], read_from: [] }) });
    expect(text()).toContain('this list may be incomplete');
    expect(text()).toContain('No refused tool calls have been read.');
  });
});
