/**
 * The Dashboard's "Waiting on you" and the rail's count: both read `GET /waiting`, one list of what the person may
 * decide, oldest first, with a part that could not be read standing as its own line. The number is the list's own.
 * A request and a draft are decided in their own row from the record their entry carries (Tom, 18 October 2026);
 * an app action and a check due open their pages; the word Decide is nowhere.
 */
import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';
import { Refused } from '../../api';
import type { Draft } from '../drafts/contract';
import type { AccessRequest } from '../requests/contract';
import { WaitingOnYou } from '../dashboard/WaitingOnYou';
import { railWaiting, waitingCount } from '../dashboard/waiting';
import type { WaitingEntry, WaitingLine, WaitingPartRefused } from '../dashboard/waiting';

const ME = 'person-me';
const entry = (kind: WaitingEntry['kind'], id: string, about: string, at: string, more: Partial<WaitingEntry> = {}): WaitingEntry =>
  ({ kind, id, asked_by: { id: 'person-sam', name: 'Sam' }, about, asked_at: at, can_decide: true, ...more });
const refusedPart = (kind: WaitingEntry['kind']): WaitingPartRefused => ({ kind, refusal: 'ReviewsUnavailable', reason: 'the reviews log is closed' });
/** The request as GET /requests answers it. */
const asked: AccessRequest = {
  id: 'req/1', asked_by: 'person-sam', asked_by_name: 'Sam', responsible: { id: 'person-sam', display_name: 'Sam', state: 'active' } as AccessRequest['responsible'],
  resource: { kind: 'project', id: 'logo' }, relation: 'editor', actions: ['write'], ends_at: null, why: 'to fix the colours', asked_at: 1_760_749_200,
  state: 'waiting', approvers: [], sources: ['grant-mine'], can_issue_root: false, can_decide: true, held_by: null, decision: null,
};
/** The draft as GET /drafts answers it. */
const prepared: Draft = {
  id: 'draft-1', agent: { id: 'agent-helper', display_name: 'Helper' }, responsible: { id: ME, display_name: 'Me', state: 'active' },
  target: { kind: 'project', id: 'logo', action: 'write' }, method: 'POST', path: '/projects/logo', body: '{}', note: 'Post the logo',
  created_at: 1_760_752_800, creation_hash: 'hash-1', state: 'waiting', execution: null,
};
const page = (waiting: WaitingLine[] | Refused) => {
  const host = document.createElement('div');
  host.innerHTML = renderToStaticMarkup(<WaitingOnYou waiting={waiting} me={ME} />);
  return host;
};
/** Each row as its words and its act: the link's word, or the decision acts by name. */
const rows = (host: Element) => [...host.querySelectorAll('tbody tr')].map((tr) => tr.querySelector('summary')?.textContent
  ?? (tr.querySelector('td')?.textContent ?? '') + (tr.querySelector('a.btn')?.textContent ?? [...tr.querySelectorAll('td.dash-decide button')].map((act) => act.getAttribute('aria-label')).join(' / ')));
const links = (host: Element) => [...host.querySelectorAll('tbody tr a.btn')].map((a) => a.getAttribute('href'));

/** As the service answers it: oldest first, the request and the draft each with its record. */
const ANSWER: WaitingLine[] = [
  entry('review', 'grant-1', 'Check the access Helper holds: viewer on project logo', '2026-10-18T01:00:00Z'),
  entry('request', 'req/1', 'Sam asks to edit project logo', '2026-10-18T02:00:00Z', { request: asked }),
  entry('agent_draft', 'draft-1', 'Helper prepared something for you to approve: Post the logo', '2026-10-18T03:00:00Z', { draft: prepared }),
  entry('product_draft', 'pd-1', 'Sam asks to write notes 1 through notes', '2026-10-18T04:00:00Z'),
];

describe('Waiting on you on the Dashboard', () => {
  it('lists the route’s answer in its own order, decides a request and a draft in their rows, opens the rest, and its number is the list’s length', () => {
    const host = page(ANSWER);
    expect(rows(host)).toEqual([
      'Check the access Helper holds: viewer on project logoOpen',
      'Sam asks to edit project logoto fix the coloursApprove access / Decline request',
      'Helper prepared something for you to approve: Post the logoApprove / Decline',
      'Sam asks to write notes 1 through notesOpen',
    ]);
    // Only the app action and the check due are opened elsewhere; the request and the draft are decided where they stand.
    expect(links(host)).toEqual(['#/reviews', '#/access/drafts']);
    expect(host.querySelectorAll('td.dash-decide')).toHaveLength(2);
    expect(host.querySelector('[data-waiting="count"]')?.textContent).toBe('4');
    expect(waitingCount(ANSWER)).toBe(4);
  });

  it('never says Decide: no row, no act and no link on the list reads it', () => {
    const host = page([...ANSWER, refusedPart('agent_draft')]);
    expect(host.textContent).not.toMatch(/Decide/);
    expect([...host.querySelectorAll('button, a')].map((each) => each.getAttribute('aria-label') ?? each.textContent).filter((words) => /Decide/.test(words ?? ''))).toEqual([]);
  });

  it('says in the row when an older service sent a request or a draft without its record, and offers Open to its page', () => {
    const host = page([
      entry('request', 'req/2', 'Kim asks to view project logo', '2026-10-18T02:00:00Z'),
      entry('agent_draft', 'draft-2', 'Helper prepared something for you to approve', '2026-10-18T03:00:00Z'),
    ]);
    expect(rows(host)).toEqual([
      'Kim asks to view project logoThis Lys did not send the request with it, so it is decided on its page.Open',
      'Helper prepared something for you to approveThis Lys did not send the draft with it, so it is decided on its page.Open',
    ]);
    expect(links(host)).toEqual(['#/requests/req%2F2', '#/access/drafts']);
    expect(host.querySelectorAll('td.dash-decide')).toHaveLength(0);
  });

  it('says once that nothing is waiting, and shows no number', () => {
    const host = page([]);
    expect(rows(host)).toEqual(['Nothing is waiting on you.']);
    expect(host.querySelector('[data-waiting="count"]')).toBeNull();
    expect(waitingCount([])).toBe(0);
  });

  it('shows a part that could not be read as that part’s own line, keeps the rest, and shows no number', () => {
    const host = page([refusedPart('review'), ANSWER[1]]);
    expect(rows(host)).toEqual(['Whether the access you gave is due a check could not be read', 'Sam asks to edit project logoto fix the coloursApprove access / Decline request']);
    // The unread part keeps its Open to the page it stands for.
    expect(links(host)).toEqual(['#/reviews']);
    expect(host.querySelector('[data-unread-part="ReviewsUnavailable"]')).not.toBeNull();
    expect(host.querySelector('[data-waiting="count"]')).toBeNull();
    expect(host.textContent).not.toContain('Nothing is waiting');
    expect(waitingCount([refusedPart('review'), ANSWER[1]])).toBeNull();
  });

  it('says the whole list could not be read on one line when the route itself is refused', () => {
    const host = page(new Refused(503, { refusal: 'DirectoryUnavailable', reason: 'closed' }));
    expect(rows(host)).toEqual(['What waits on you could not be read']);
    expect(host.querySelector('[data-waiting="count"]')).toBeNull();
  });
});

describe('The rail’s count of what waits', () => {
  it('is the number of lines the route answers', () => {
    expect(railWaiting(ANSWER)).toEqual({ count: 4, words: '4 things wait on you' });
    expect(railWaiting([ANSWER[0]])).toEqual({ count: 1, words: '1 thing waits on you' });
    expect(railWaiting([])).toEqual({ count: 0, words: 'Nothing is waiting on you' });
  });

  it('never counts a part it could not read as nothing: it says which part', () => {
    expect(railWaiting([refusedPart('review'), ANSWER[1]])).toEqual({
      problem: 'Whether the access you gave is due a check could not be read; 1 thing waits on you besides',
    });
  });
});
