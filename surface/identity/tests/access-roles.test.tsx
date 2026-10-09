/**
 * ACCESS-004 on the screens. R1: a schema's roles are listed under its matrix,
 * a change an app proposed names each widening of a role in words before the
 * administrator approves it, and the issue form offers the chosen kind's roles
 * beside the mode, sending the role as the relation. R2: the reach view shows a
 * restricted place apart.
 */
import { act } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { beforeEach, describe, expect, it } from 'vitest';
import { $, $$, choose, mount, pick, settle } from './harness';
import { ADA, RECEIPTS, SERVICE, ok } from './fixtures';
import type { Route } from './fixtures';
import { SchemaMatrix } from '../src/features/apps/SchemaMatrix';
import { roleChanges } from '../src/features/apps/roleChanges';

beforeEach(() => sessionStorage.clear());

const APP = 'cambium';
const KIND = 'cambium.workspace';
const schema = (administrator: string[]) => ({ kinds: { [KIND]: {
  actions: ['read', 'post', 'seat_add', 'seat_retire'],
  relations: { member: ['read', 'post'] },
  parents: [],
  roles: { administrator, observer: ['read'] },
} } });

describe('A role change in words', () => {
  it('names a widening, a narrowing, a role added and a role removed', () => {
    const before = schema(['read', 'seat_add']);
    const after = { kinds: { [KIND]: { ...schema(['read', 'seat_retire']).kinds[KIND], roles: { administrator: ['read', 'seat_retire'], auditor: ['read'] } } } };
    expect(roleChanges(before, after)).toEqual([
      'adds seat_retire to administrator',
      'removes seat_add from administrator',
      'adds the role auditor to cambium.workspace, carrying read',
      'removes the role observer from cambium.workspace',
    ]);
    expect(roleChanges(before, before)).toEqual([]);
    expect(roleChanges({ kinds: { [KIND]: { actions: ['read'] } } }, { kinds: { [KIND]: { actions: ['read'] } } })).toEqual([]);
  });

  it('lists a kind\'s roles under its matrix', () => {
    const page = new DOMParser().parseFromString(renderToStaticMarkup(<SchemaMatrix label="Its schema" kinds={[[KIND, schema(['read', 'seat_add']).kinds[KIND]]]} />), 'text/html');
    const roles = [...page.querySelectorAll('tfoot tr.role td')].map((cell) => cell.textContent);
    expect(roles).toEqual(['The role administrator carries read, seat_add.', 'The role observer carries read.']);
  });
});

describe('An app\'s widening of a role waits on the Apps screen', () => {
  const app = {
    id: APP, name: 'Cambium', state: 'approved', redirects: [], sign_in: null, schema: schema(['read', 'seat_add']), version: 2, versions: [1, 2],
    pending: { operation: 'op-widen', replaces: 2, schema: schema(['read', 'seat_add', 'seat_retire']), by: { kind: 'service_account', id: 'cambium' }, at: 3 },
    client_id: APP, service_account: null, client_credentials: [], registered_by: { kind: 'start' }, registered_at: 1,
  };

  it('shows "adds seat_retire to administrator" beside Approve and Decline', async () => {
    await mount('#/apps', { ...SERVICE, '/apps': () => ok({ apps: [app] }) });
    const waiting = $('[aria-label="Change waiting for ' + APP + '"]');
    expect(waiting).not.toBeNull();
    const words = [...(waiting?.querySelectorAll('[aria-label="Role changes waiting for ' + APP + '"] li') ?? [])].map((item) => item.textContent);
    expect(words).toEqual(['adds seat_retire to administrator']);
    expect(waiting?.textContent).toContain('Approve');
  });
});

describe('The issue form offers the kind\'s roles beside the mode', () => {
  const routes: Record<string, Route> = {
    ...SERVICE,
    '/apps': ok({ apps: [{ id: APP, name: 'Cambium', state: 'approved' }] }),
    ['/apps/' + APP + '/schema']: ok({ app: APP, version: 2, schema: schema(['read', 'seat_add']) }),
    'POST /grants/roots': (body: unknown) => {
      const operation = (body as { operation: string }).operation;
      const receipt = { ...RECEIPTS[4].receipt, operation, identity: ADA };
      return ok({ operation, grant: 'grant-recorded', index: receipt.log.index, receipt: { ...receipt, caller: ADA, revision: 1 } });
    },
  };

  it('sends the chosen role as the relation', async () => {
    const { posted } = await mount('#/access/issue', routes);
    const page = $('form[aria-label="Issue root grant"]');
    if (!page) throw new Error('No Issue root grant form');
    expect($('select[name="role"]')).toBeNull();
    await pick(page, 'Find a person', 'Ada', 'Ada (test person)');
    await choose(page.querySelector('select[name="kind"]'), KIND);
    expect($$('select[name="role"] option').map((option) => option.getAttribute('value'))).toEqual(['', 'administrator', 'observer']);
    await choose(page.querySelector('select[name="role"]'), 'administrator');
    expect(page.querySelector<HTMLSelectElement>('select[name="relation"]')?.disabled).toBe(true);
    const input = page.querySelector<HTMLInputElement>('input[name="resource"]');
    if (!input) throw new Error('No resource input');
    input.value = 'ward';
    const checks = page.querySelectorAll<HTMLInputElement>('input[type="checkbox"]');
    await act(async () => { checks[1]?.click(); });
    await act(async () => { page.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); });
    await settle();
    expect(posted.at(-1)).toMatchObject({ path: '/grants/roots', body: { resource: { kind: KIND, id: 'ward' }, relation: 'administrator' } });
  });
});

describe('The reach view shows a restricted place apart', () => {
  it('lists it under Restricted places, not with the rest', async () => {
    const reach = (body: unknown) => {
      const asked = (body as { resources: { kind: string; id: string; actions: string[] }[] }).resources;
      return ok({ revision: 7, resources: asked.map(({ kind, id, actions }, n) => ({
        kind, id, restricted: n === 0,
        holders: [{ holder: ADA, actions, modes: actions.map(() => 'outright') }],
      })) });
    };
    await mount('#/access/reach/' + ADA, { ...SERVICE, 'POST /grants/reach': reach });
    const apart = $('section[aria-label="Restricted places"]');
    expect(apart).not.toBeNull();
    expect(apart?.querySelectorAll('tr')).toHaveLength(1);
    const all = $$('.check .card tr').length;
    expect(all).toBeGreaterThan(1);
  });
});
