/** A kept change this tab cannot read back pauses sending, and Check with Lys settles it: never "ask an administrator", never stuck for good. */
import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { $, click, mount, serve, settle, text, unmountAll } from './harness';
import { ADA, AGENT_MODEL, GRANTS, ME, ROOT_G, SERVICE, ok, refused } from './fixtures';
import { DecisionForm } from '../src/features/requests/DecisionForm';
import { ShellProvider } from '../src/shell/ShellContext';
import type { AccessRequest } from '../src/features/requests/contract';

beforeEach(() => sessionStorage.clear());
afterEach(() => unmountAll());
const OP = 'op-' + '7'.repeat(32);
const KEY = 'lys.pending.request.' + ADA;
const model = { action_sentences: { read: 'Read this resource' }, version: 1, relations: { reader: ['read'] }, kinds: {} };
const asked = (id: string, state: AccessRequest['state'] = 'waiting') => ({ id, asked_by: ADA, asked_by_name: ME.person.display_name, responsible: ME.person, resource: { kind: 'project', id: 'Lys' },
  relation: 'reader', actions: ['read'], why: 'x', ends_at: null, asked_at: 1790000000, state, approvers: [ME.person], sources: [], decision: null });
const routes = (requests: unknown[]) => ({ ...SERVICE, '/requests': ok({ requests }), '/grants/model': ok(model), '/grants': ok({ revision: 1, grants: [] }) });
const button = (label: string) => [...document.querySelectorAll('button')].find((each) => (each.getAttribute('aria-label') ?? each.textContent) === label) ?? null;
const askOpen = () => $('form[aria-label="Ask for access"] fieldset.ask-fields') as HTMLFieldSetElement | null;

describe('Check with Lys, on the Ask form', () => {
  it('finds a damaged ask Lys already has, says so, and opens the form on Carry on without sending anything', async () => {
    sessionStorage.setItem(KEY, '{"operation":"' + OP + '", "resource": broken');
    const m = await mount('#/requests', routes([asked(OP)]));
    expect(askOpen()?.disabled).toBe(true);
    expect(text()).not.toContain('administrator');
    await click(button('Check with Lys'));
    expect(text()).toContain('Lys has your request: it went through');
    await click(button('Carry on'));
    expect(sessionStorage.getItem(KEY)).toBeNull();
    expect(askOpen()?.disabled).toBe(false);
    expect(m.posted).toHaveLength(0);
  });

  it('says a legible ask Lys does not have most likely did not go through, and clears only on her press', async () => {
    sessionStorage.setItem(KEY, '{"operation":"' + OP + '", "resource": broken');
    await mount('#/requests', routes([]));
    await click(button('Check with Lys'));
    expect(text()).toContain('Lys has no record of it among what you can see');
    expect(sessionStorage.getItem(KEY)).not.toBeNull();
    await click(button('Clear it and carry on'));
    expect(sessionStorage.getItem(KEY)).toBeNull();
    expect(askOpen()?.disabled).toBe(false);
  });

  it('says when too little is left to ask about, and still offers the way out', async () => {
    sessionStorage.setItem(KEY, '{broken');
    await mount('#/requests', routes([]));
    await click(button('Check with Lys'));
    expect(text()).toContain('Too little of it is left to ask Lys about');
    expect(button('Clear it and carry on')).not.toBeNull();
  });

  it('clears nothing when Lys does not answer, and says to check again', async () => {
    sessionStorage.setItem(KEY, '{"operation":"' + OP + '", "resource": broken');
    let closed = false;
    await mount('#/requests', { ...routes([]), '/requests': () => (closed ? refused(503, 'RequestsUnavailable', 'the request log is closed') : ok({ requests: [] })) });
    closed = true;
    await click(button('Check with Lys'));
    expect(text()).toContain('Nothing was cleared');
    expect(sessionStorage.getItem(KEY)).not.toBeNull();
  });
});

describe('Check with Lys, on a decision', () => {
  it('settles a damaged decision by the request no longer waiting', async () => {
    const key = 'lys.pending.decision.' + ADA + '.' + OP;
    sessionStorage.setItem(key, '{broken');
    serve({ ...SERVICE, '/requests': ok({ requests: [asked(OP, 'approved')] }) });
    const container = document.createElement('div'); document.body.append(container);
    const root = createRoot(container);
    await act(async () => { root.render(<ShellProvider><DecisionForm entry={asked(OP) as unknown as AccessRequest} person={ADA} canIssueRoot={false} changed={() => undefined} describe={() => ''} /></ShellProvider>); });
    await click(button('Check with Lys'));
    await settle();
    expect(container.textContent).toContain('Lys has your decision: it went through');
    act(() => root.unmount());
  });
});

describe('A job change kept unreadably', () => {
  it('cannot be asked about from here, says to look at the list, and clears on her press', async () => {
    sessionStorage.setItem('lys.pending.role.' + ADA + '.new', '{broken');
    await mount('#/roles', { ...SERVICE, '/roles': ok({ roles: [] }) });
    await click(button('Create a job'));
    expect(text()).toContain('Lys cannot be asked about this one from here');
    expect(($('form[aria-label="Create job"] fieldset') as HTMLFieldSetElement | null)?.disabled).toBe(true);
    await click(button('Clear it and carry on'));
    expect(($('form[aria-label="Create job"] fieldset') as HTMLFieldSetElement | null)?.disabled).toBe(false);
  });
});

describe('Check with Lys, on a give', () => {
  it('finds a damaged give Lys already has among the grants, and opens the form on Carry on', async () => {
    const key = 'lys.pending.grant.' + ADA + '.' + ROOT_G;
    const given = GRANTS[0].operation;
    sessionStorage.setItem(key, '{"operation":"' + given + '", "recipient": broken');
    const m = await mount('#/me?tab=account', { ...SERVICE, '/grants/model': ok(AGENT_MODEL) });
    await click($(`[data-act="delegate"][data-g="${ROOT_G}"]`));
    expect($('[data-why-not]')?.textContent).toBe('Check what you gave earlier with Lys first (below).');
    await click(button('Check with Lys'));
    expect(text()).toContain('Lys has the access you gave: it went through');
    await click(button('Carry on'));
    expect(sessionStorage.getItem(key)).toBeNull();
    expect(button('Check with Lys')).toBeNull();
    expect(m.posted).toHaveLength(0);
  });
});
