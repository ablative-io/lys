/** DIRECTORY-051 R6: an agent's tool policy is read and kept as numbered versions that apply on its next launch. */
import { act } from 'react';
import { afterEach, describe, expect, it } from 'vitest';
import { $, choose, click, mount, text, unmountAll } from './harness';
import { SCRIBE, SERVICE, ok, refused } from './fixtures';
import type { Route } from './fixtures';
import type { Policy, PolicyView, RefusalsView } from '../src/features/file/policyContract';

const policy = '/agents/' + SCRIBE + '/policy';
const refusals = '/agents/' + SCRIBE + '/refusals';
const APPLIES = "applies on the agent's next launch; a session already running keeps the version it was launched with";
const none: RefusalsView = { agent: SCRIBE, refusals: [], read_from: [] };

afterEach(() => { unmountAll(); document.body.innerHTML = ''; });

/** A service that keeps each version it is sent, as the identity service does. */
function keeping(start: Policy | null): Record<string, Route> {
  let kept = start;
  return {
    ...SERVICE,
    [refusals]: ok(none),
    [policy]: () => ok({ agent: SCRIBE, policy: kept, digest: kept ? 'd'.repeat(64) : null, applies: APPLIES } satisfies PolicyView),
    ['POST ' + policy]: (body) => {
      const given = body as { version: number; rules: Policy['rules'] };
      if (given.version !== (kept?.version ?? 0)) return refused(409, 'PolicyVersionConflict', 'crossed');
      kept = { version: given.version + 1, agent: SCRIBE, rules: given.rules };
      return ok(kept);
    },
  };
}

async function type(selector: string, value: string): Promise<void> {
  const input = document.querySelector<HTMLInputElement>(selector);
  if (!input) throw new Error('no ' + selector);
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')?.set?.call(input, value);
    input.dispatchEvent(new Event('input', { bubbles: true }));
  });
}

const button = (label: string) => [...document.querySelectorAll('button')].find((b) => b.textContent === label) ?? null;

describe('Agent tool policy', () => {
  it('says when a version applies, and that no policy is set', async () => {
    await mount('#/file/' + SCRIBE + '/policy', keeping(null));
    expect(text()).toContain('No policy set');
    expect(text()).toContain('applies on the agent\'s next launch');
    expect(text()).not.toMatch(/sandbox/i);
  });

  it('keeps a hard rule and a grantable one as the next version', async () => {
    const { posted } = await mount('#/file/' + SCRIBE + '/policy', keeping(null));
    await type('input[name="id"]', 'no-shell');
    await type('input[name="tool"]', 'Bash');
    await click($('form[aria-label="Add a rule"] button[type="submit"]'));
    await type('input[name="id"]', 'secrets-dir');
    await type('input[name="tool"]', 'Read');
    await choose($('select[name="kind"]'), 'path_prefix');
    await type('input[name="target"]', '/srv/secrets');
    await click($('input[name="grantable"]'));
    await type('input[name="resource_kind"]', 'secret');
    await type('input[name="resource_id"]', 'vault');
    await type('input[name="action"]', 'read');
    await click($('form[aria-label="Add a rule"] button[type="submit"]'));
    await click(button('Keep as version 1'));
    expect(posted).toEqual([{ path: policy, body: { version: 0, rules: [
      { id: 'no-shell', tool: 'Bash', kind: 'tool', authority: 'hard' },
      { id: 'secrets-dir', tool: 'Read', kind: 'path_prefix', target: '/srv/secrets', authority: { permission: { resource: { kind: 'secret', id: 'vault' }, action: 'read' } } },
    ] } }]);
    expect(text()).toContain('Version 1');
    expect(text()).toContain('Nobody: no grant can lift it');
    expect(text()).toContain('A grant of read on secret vault');
  });

  it('names a crossed change instead of keeping it', async () => {
    const routes = keeping(null);
    await mount('#/file/' + SCRIBE + '/policy', { ...routes, ['POST ' + policy]: refused(409, 'PolicyVersionConflict', 'crossed') });
    await type('input[name="id"]', 'no-shell');
    await type('input[name="tool"]', 'Bash');
    await click($('form[aria-label="Add a rule"] button[type="submit"]'));
    await click(button('Keep as version 1'));
    expect($('[role="alert"]')?.textContent).toContain('Someone else changed this policy first');
    expect(text()).toContain('No policy set');
  });
});
