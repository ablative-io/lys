/**
 * ACCESS-005 R2: the Network screen shows each computer's machines, the identities its joins made, with the key it
 * joined with, the person answering for it, the grants it holds and what each is on, and lets the administrator give
 * it part of a grant they hold through the same grant request every other giving sends.
 */
import { beforeEach, describe, expect, it } from 'vitest';
import { $, $$, choose, click, mount, text, unmountAll } from './harness';
import { ADA, GRANTS, MODEL, SERVICE, ok, refused } from './fixtures';
import type { Route } from './fixtures';
import type { Machine, MachineIdentity } from '../src/features/network/contract';
import type { DelegateBody, Grant } from '../src/generated/grants';

beforeEach(() => { sessionStorage.clear(); unmountAll(); });

const computer: Machine = { id: 'op-' + 'c'.repeat(32), name: 'Liminal node', kind: 'computer', runtime: 'lys-runner', slots: 0, may_run: [], may_reach: [], named_by: ADA, named_at: 1790000000, state: 'in_use', retired_at: null, last_report_at: null };
const KEY = 'ab'.repeat(32);
const NOW = 'machine-' + '5a'.repeat(16);
const BEFORE = 'machine-' + '4b'.repeat(16);
const LINK = { kind: 'link', id: 'hermes-to-liminal' };
const held = { grant: 'grant-' + '7'.repeat(32), resource: LINK, relation: 'viewer', actions: ['view'], window: { starts_at: 1790000000, ends_at: null }, mode: 'outright' as const, admitted: true };
const machine = (given: Partial<MachineIdentity> = {}): MachineIdentity => ({
  identity: NOW, machine: computer.id, key: KEY, responsible: ADA, responsible_name: 'Ada (test person)', joined_at: 1790000000, state: 'active', replaced: false, grants: [held], ...given,
});
/** A grant Ada holds on the link that she may pass on to machines. */
const toMachines: Grant = { ...GRANTS[0], id: 'grant-' + '8'.repeat(32), resource: LINK, pass_on: { kind: 'to', actions: ['edit', 'view'], recipients: ['machine'] } };

function routes(identities: MachineIdentity[], extra: Record<string, Route> = {}): Record<string, Route> {
  return {
    ...SERVICE,
    '/network': ok({ machines: [computer], reports_served: true }),
    ['/network/machines/' + computer.id + '/runner']: ok({ machine: computer.id, runner: { kind: 'dialled', key: KEY, runner: 'r' }, answers: true }),
    '/network/machine-identities': ok({ machines: identities, revision: 7 }),
    '/grants': ok({ grants: [...GRANTS, toMachines], revision: 7 }),
    '/grants/model': ok(MODEL),
    ...extra,
  };
}
const card = (identity: string) => $('[data-machine="' + identity + '"]');
const button = (label: string) => [...document.querySelectorAll('button')].find((value) => (value.getAttribute('aria-label') ?? value.textContent) === label) ?? null;

describe('Machines on the Network screen', () => {
  it('shows the computer\'s machine with its key id, the person answering for it and each grant with what it is on', async () => {
    await mount('#/network', routes([machine()]));
    const shown = card(NOW);
    expect(shown).not.toBeNull();
    expect(shown?.textContent).toContain(NOW);
    expect(shown?.querySelector('[data-key]')?.textContent).toBe(KEY);
    const answers = [...(shown?.querySelectorAll('a') ?? [])].find((a) => a.getAttribute('href') === '#/file/' + ADA);
    expect(answers?.textContent).toBe('Ada (test person)');
    const row = shown?.querySelector('tr[data-grant="' + held.grant + '"]');
    expect(row?.textContent).toContain('link hermes-to-liminal');
    expect(row?.querySelector('a')?.getAttribute('href')).toBe('#/access/who/' + encodeURIComponent('link:hermes-to-liminal'));
    expect(row?.textContent).toContain('view');
    expect(row?.textContent).toContain('At once');
  });

  it('shows the machine a later join replaced after the one the computer answers as now, saying its grants no longer count', async () => {
    await mount('#/network', routes([machine({ identity: BEFORE, replaced: true, state: 'retired', grants: [] }), machine()]));
    const cards = $$('[data-machine]').map((each) => each.getAttribute('data-machine'));
    expect(cards).toEqual([NOW, BEFORE]);
    expect(card(BEFORE)?.textContent).toContain('Replaced by a later join');
    expect(card(BEFORE)?.textContent).toContain('It holds no grant.');
    expect(button('Give ' + BEFORE + ' a grant')).toBeNull();
    expect(button('Give ' + NOW + ' a grant')).not.toBeNull();
  });

  it('says a computer that never joined is no machine yet', async () => {
    await mount('#/network', routes([]));
    expect(text()).toContain('This computer has not joined with a connection code');
  });

  it('gives the machine part of a grant Ada holds through POST /grants, answering to the machine\'s own person', async () => {
    const mounted = await mount('#/network', routes([machine({ grants: [] })]));
    await click(button('Give ' + NOW + ' a grant'));
    const form = $('form[aria-label="Give ' + NOW + ' a grant"]');
    expect(form).not.toBeNull();
    const sources = [...(form?.querySelectorAll('select[name="source"] option') ?? [])].map((option) => option.getAttribute('value'));
    expect(sources).toEqual([toMachines.id]);
    await choose(form?.querySelector('select[name="relation"]') ?? null, 'viewer');
    await click(button('Give'));
    const sent = mounted.posted.find((each) => each.path === '/grants')?.body as DelegateBody | undefined;
    expect(sent?.recipient).toBe(NOW);
    expect(sent?.responsible).toBe(ADA);
    expect(sent?.source).toBe(toMachines.id);
    expect(sent?.resource).toEqual(LINK);
    expect(sent?.relation).toBe('viewer');
    expect(sent?.pass_on).toEqual({ kind: 'use_only' });
    expect(text()).toContain('Given. ' + NOW + ' now holds viewer on link hermes-to-liminal');
  });

  it('shows the service\'s refusal by name when the machine may not be given it', async () => {
    await mount('#/network', routes([machine({ grants: [] })], {
      'POST /grants': refused(403, 'MachineRefused', 'MachineRefused: ' + NOW + ': a machine never holds grant.delegate, which a person keeps'),
    }));
    await click(button('Give ' + NOW + ' a grant'));
    await click(button('Give'));
    expect($('[role="alert"]')?.textContent).toContain('MachineRefused');
  });

  it('offers nothing to give when Ada holds no grant that may be passed on to machines', async () => {
    await mount('#/network', routes([machine({ grants: [] })], { '/grants': ok({ grants: GRANTS, revision: 7 }) }));
    await click(button('Give ' + NOW + ' a grant'));
    expect(text()).toContain('You hold no grant that may be passed on to a machine.');
  });

  it('says so when the machines cannot be read, and still lists the computer', async () => {
    await mount('#/network', routes([], { '/network/machine-identities': refused(503, 'NetworkUnavailable', 'the codes cannot be read') }));
    expect(text()).toContain('The machines cannot be read');
    expect($$('#screen tbody tr[data-href]')[0]?.textContent).toContain('Liminal node');
  });
});
