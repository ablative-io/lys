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
    ['/agents/' + SCRIBE + '/provisioning']: ok({ agent: SCRIBE, profile: null, versions: [], enforced: false }),
    '/harnesses': ok({ programs: [] }), '/network': ok({ machines: [], reports_served: true }),
    [refusals]: ok(none),
    [policy]: () => ok({ agent: SCRIBE, policy: kept, digest: kept ? 'd'.repeat(64) : null, applies: APPLIES } satisfies PolicyView),
    ['POST ' + policy]: (body) => {
      const given = body as { version: number; rules: Policy['rules'] };
      if (given.version !== (kept?.version ?? 0)) return refused(409, 'PolicyVersionConflict', 'crossed');
      kept = { version: given.version + 1, agent: SCRIBE, rules: given.rules };
      return ok({ agent: SCRIBE, policy: kept, digest: 'd'.repeat(64), applies: APPLIES } satisfies PolicyView);
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

const choosePolicy = () => choose($('tr[aria-label="Add a rule"] select[name="rule-enforcer"]'), 'policy');
const policyRows = () => [...document.querySelectorAll<HTMLTableRowElement>('table[aria-label="Rules"] tr[data-policy]')];

describe('Agent tool policy', () => {
  it('is part of the Settings tab, in the one Rules table: it says when a version applies, and that no policy is set', async () => {
    await mount('#/file/' + SCRIBE + '/policy', keeping(null));
    expect($('.tabs a.on')?.textContent).toBe('Settings');
    expect(document.querySelectorAll('table[aria-label="Rules"]')).toHaveLength(1);
    expect($('form[aria-label="Add a rule"]')).toBeNull();
    expect(text()).toContain('No policy set');
    expect(text()).toContain('applies on the agent\'s next launch');
    expect(text()).not.toMatch(/sandbox/i);
  });

  it('keeps a hard rule and a grantable one as the next version, with one read of the policy', async () => {
    const { posted, requests } = await mount('#/file/' + SCRIBE + '/policy', keeping(null));
    await choosePolicy();
    await type('input[name="id"]', 'no-shell');
    await type('input[name="tool"]', 'Bash');
    await click(button('Add rule'));
    await type('input[name="id"]', 'secrets-dir');
    await type('input[name="tool"]', 'Read');
    await choose($('select[name="kind"]'), 'path_prefix');
    await type('input[name="target"]', '/srv/secrets');
    await click($('input[name="grantable"]'));
    await type('input[name="resource_kind"]', 'secret');
    await type('input[name="resource_id"]', 'vault');
    await type('input[name="action"]', 'read');
    await click(button('Add rule'));
    expect(policyRows().map((row) => row.dataset.policy)).toEqual(['no-shell', 'secrets-dir']);
    expect(posted.filter((entry) => entry.path === policy)).toEqual([]);
    expect(text()).toContain('changed and not yet kept');
    await click(button('Save policy rules for the next start'));
    expect(posted.filter((entry) => entry.path === policy)).toEqual([{ path: policy, body: { version: 0, rules: [
      { id: 'no-shell', tool: 'Bash', kind: 'tool', authority: 'hard' },
      { id: 'secrets-dir', tool: 'Read', kind: 'path_prefix', target: '/srv/secrets', authority: { permission: { resource: { kind: 'secret', id: 'vault' }, action: 'read' } } },
    ] } }]);
    expect(requests.filter((entry) => entry === policy)).toHaveLength(1);
    expect(text()).toContain('Policy: Version 1');
    expect(text()).not.toContain('d'.repeat(64));
    expect(text()).toContain('Nobody: no grant can lift it');
    expect(text()).toContain('A grant of read on secret vault');
    expect(policyRows().every((row) => row.children[2]?.textContent === 'Refused')).toBe(true);
  });

  it('removes a policy rule in its own row and keeps the rest as the next version', async () => {
    const start: Policy = { version: 3, agent: SCRIBE, rules: [{ id: 'no-shell', tool: 'Bash', kind: 'tool', authority: 'hard' }, { id: 'no-web', tool: 'WebFetch', kind: 'host', target: 'example.org', authority: 'hard' }] };
    const { posted } = await mount('#/file/' + SCRIBE + '/policy', keeping(start));
    await click($('button[aria-label="Remove policy rule no-shell"]'));
    await click(button('Save policy rules for the next start'));
    expect(posted.filter((entry) => entry.path === policy)).toEqual([{ path: policy, body: { version: 3, rules: [start.rules[1]] } }]);
    expect(policyRows().map((row) => row.dataset.policy)).toEqual(['no-web']);
    expect(text()).toContain('Policy: Version 4');
  });

  it('names a crossed change instead of keeping it', async () => {
    const routes = keeping(null);
    await mount('#/file/' + SCRIBE + '/policy', { ...routes, ['POST ' + policy]: refused(409, 'PolicyVersionConflict', 'crossed') });
    await choosePolicy();
    await type('input[name="id"]', 'no-shell');
    await type('input[name="tool"]', 'Bash');
    await click(button('Add rule'));
    await click(button('Save policy rules for the next start'));
    expect($('[role="alert"]')?.textContent).toContain('Someone changed these rules first');
    expect(text()).toContain('No policy set');
  });
});
