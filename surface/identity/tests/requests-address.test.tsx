/** A request has its own address, #/requests/<id>, so the dashboard's Decide and any shared link open that request itself. */
import { afterEach, describe, expect, it } from 'vitest';
import { $, $$, mount, unmountAll } from './harness';
import { ADA, BEA, ME, SERVICE, ok } from './fixtures';

afterEach(() => unmountAll());
const model = { action_sentences: { read: 'Read this resource' }, version: 1, relations: { reader: ['read'] }, kinds: {} };
const op = (digit: string) => 'op-' + digit.repeat(32);
const asked = (id: string, by: string, name: string, state = 'waiting') => ({ id, asked_by: by, asked_by_name: name, responsible: { id: by, display_name: name, state: 'active' },
  resource: { kind: 'project', id: 'Lys' }, relation: 'reader', actions: ['read'], why: 'Why ' + name, ends_at: null, asked_at: 1790000000, state,
  approvers: [ME.person], sources: [], decision: null });
const FIRST = asked(op('1'), ADA, 'Ada (test person)');
const SECOND = asked(op('2'), BEA, 'Bea (test person)');
const DONE = asked(op('3'), BEA, 'Bea (test person)', 'approved');
const routes = { ...SERVICE, '/requests': ok({ requests: [FIRST, SECOND, DONE] }), '/grants/model': ok(model), '/grants': ok({ revision: 1, grants: [] }) };
const opened = () => $('article[aria-label="Request"] h2')?.textContent;

describe('A request in the address', () => {
  it('opens the request the address names, not the first in the queue', async () => {
    await mount('#/requests/' + SECOND.id, routes);
    expect(opened()).toBe('Bea (test person) · reader on Lys');
    expect($('article[aria-label="Request"]')?.textContent).toContain('Why Bea (test person)');
  });

  it('opens a decided request too, whichever view of the queue is shown', async () => {
    await mount('#/requests/' + DONE.id, routes);
    expect($('article[aria-label="Request"]')?.textContent).toContain('Approved');
  });

  it('says when the address names no request this person may see, and opens the queue as usual', async () => {
    await mount('#/requests/' + op('9'), routes);
    expect($('[data-request-unknown]')?.textContent).toBe('Lys shows you no request at this address: it is not one you may see, or the link is wrong. The requests you may see are below.');
    expect(opened()).toBe('Ada (test person) · reader on Lys');
  });

  it('gives every row its own address', async () => {
    await mount('#/requests', routes);
    expect($$('tr[data-href]').map((row) => row.getAttribute('data-href'))).toContain('#/requests/' + FIRST.id);
  });
});
