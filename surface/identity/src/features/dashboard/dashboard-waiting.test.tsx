/**
 * The Dashboard's "Waiting on you": one list of what the person may act on, its number counted from the list itself.
 */
import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';
import { Refused } from '../../api';
import type { Waiting } from '../dashboard/contract';
import { WaitingOnYou, asks, waitingCount } from '../dashboard/WaitingOnYou';
import type { Draft } from '../drafts/contract';
import type { AccessRequest } from '../requests/contract';

const ME = 'person-me';
const person = (id: string, name: string) => ({ id, display_name: name }) as AccessRequest['approvers'][number];
const ask = (id: string, at: number, approvers: string[], more: Partial<AccessRequest> = {}): AccessRequest => ({
  id, asked_by: 'person-sam', asked_by_name: 'Sam', responsible: person('person-sam', 'Sam'), resource: { kind: 'project', id: 'logo' },
  relation: 'editor', actions: ['read', 'write'], ends_at: null, why: 'to fix the colours', asked_at: at, state: 'waiting',
  approvers: approvers.map((each) => person(each, each)), sources: [], decision: null, ...more,
});
const draft = (id: string, note: string): Draft => ({ id, agent: { id: 'agent-helper', display_name: 'Helper' }, responsible: { id: ME, display_name: 'Me', state: 'active' },
  target: { kind: 'project', id: 'logo', action: 'write' }, method: 'POST', path: '/x', body: '{}', note, created_at: 1, creation_hash: 'h', state: 'waiting' });
const none: Waiting = { requests: 0, drafts: 0, reviews: 0, product_drafts: 0 };
const page = (requests: AccessRequest[] | Refused, drafts: Draft[] | Refused, waiting: Waiting = none) => {
  const host = document.createElement('div');
  host.innerHTML = renderToStaticMarkup(<WaitingOnYou requests={requests} drafts={drafts} waiting={waiting} me={ME} />);
  return host;
};
const rows = (host: Element) => [...host.querySelectorAll('tbody tr')].map((tr) => tr.querySelector('summary')?.textContent ?? tr.textContent);

describe('Waiting on you on the Dashboard', () => {
  it('lists only what the person may decide, oldest first, in words, and its number is the list’s own length', () => {
    const requests = [ask('r2', 20, [ME]), ask('r1', 10, [ME], { asked_by_name: 'Kim', relation: 'viewer' }), ask('r3', 5, ['person-other']),
      ask('r4', 1, [ME], { state: 'approved' }), ask('r5', 30, ['person-other'], { can_decide: true, relation: 'owner' })];
    const host = page(requests, [draft('d1', 'Post the logo')], { ...none, reviews: 1, product_drafts: 2 });
    expect(rows(host)).toEqual([
      'Kim asks to view project logoto fix the coloursDecide',
      'Sam asks to edit project logoto fix the coloursDecide',
      'Sam asks to own project logoto fix the coloursDecide',
      'Helper prepared something for you to approvePost the logoDecide',
      '2 app actions wait for your approvalOpen',
      'Check the access you gave: 1 is dueOpen',
      '1 more request waits on someone else',
    ]);
    // Three requests, one draft, two app drafts and one review: seven, the lines above it that can be acted on.
    expect(host.querySelector('[data-waiting="count"]')?.textContent).toBe('7');
    expect(host.querySelectorAll('tr[data-request], tr[data-draft]').length + 3).toBe(7);
    expect(waitingCount(requests, [draft('d1', '')], { ...none, reviews: 1, product_drafts: 2 }, ME)).toBe(7);
  });

  it('says once that nothing is waiting, shows no number, and still points to what waits on others', () => {
    const host = page([ask('r3', 5, ['person-other'])], []);
    expect(rows(host)).toEqual(['Nothing is waiting on you.', '1 more request waits on someone else']);
    expect(host.querySelector('[data-waiting="count"]')).toBeNull();
  });

  it('never shows a number or "nothing" when a list could not be read; it says which list, where it would stand', () => {
    const refused = new Refused(503, { refusal: 'RequestsUnreadable', reason: 'the requests store is closed' });
    const host = page(refused, [draft('d1', 'Post the logo')], { ...none, reviews: { refusal: 'ReviewsUnavailable', reason: 'closed' } });
    expect(rows(host)).toEqual(['Requests could not be read', 'Helper prepared something for you to approvePost the logoDecide', 'Whether the access you gave is due a check could not be read']);
    expect(host.querySelector('[data-waiting="count"]')).toBeNull();
    expect(host.textContent).not.toContain('Nothing is waiting');
    expect(waitingCount(refused, [], none, ME)).toBeNull();
  });

  it('names someone the directory cannot by saying so, and keeps a relation it has no word for', () => {
    expect(asks(ask('r', 1, [ME], { asked_by_name: null, relation: 'auditor' }))).toBe('Someone outside your view asks for auditor access to project logo');
  });
});
