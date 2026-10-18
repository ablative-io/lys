/**
 * The Dashboard's "Waiting on you" and the rail's count: both read `GET /waiting`, one list of what the person may
 * decide, oldest first, with a part that could not be read standing as its own line. The number is the list's own.
 */
import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';
import { Refused } from '../../api';
import { WaitingOnYou } from '../dashboard/WaitingOnYou';
import { railWaiting, waitingCount } from '../dashboard/waiting';
import type { WaitingEntry, WaitingLine, WaitingPartRefused } from '../dashboard/waiting';

const entry = (kind: WaitingEntry['kind'], id: string, about: string, at: string): WaitingEntry =>
  ({ kind, id, asked_by: { id: 'person-sam', name: 'Sam' }, about, asked_at: at, can_decide: true });
const refusedPart = (kind: WaitingEntry['kind']): WaitingPartRefused => ({ kind, refusal: 'ReviewsUnavailable', reason: 'the reviews log is closed' });
const page = (waiting: WaitingLine[] | Refused) => {
  const host = document.createElement('div');
  host.innerHTML = renderToStaticMarkup(<WaitingOnYou waiting={waiting} />);
  return host;
};
const rows = (host: Element) => [...host.querySelectorAll('tbody tr')].map((tr) => tr.querySelector('summary')?.textContent ?? tr.textContent);
const links = (host: Element) => [...host.querySelectorAll('tbody tr a.btn')].map((a) => a.getAttribute('href'));

/** As the service answers it: oldest first. */
const ANSWER: WaitingLine[] = [
  entry('review', 'grant-1', 'Check the access Helper holds: viewer on project logo', '2026-10-18T01:00:00Z'),
  entry('request', 'req/1', 'Sam asks to edit project logo', '2026-10-18T02:00:00Z'),
  entry('agent_draft', 'draft-1', 'Helper prepared something for you to approve: Post the logo', '2026-10-18T03:00:00Z'),
  entry('product_draft', 'pd-1', 'Sam asks to write notes 1 through notes', '2026-10-18T04:00:00Z'),
];

describe('Waiting on you on the Dashboard', () => {
  it('lists the route’s answer in its own order, each line with its way to decide, and its number is the list’s length', () => {
    const host = page(ANSWER);
    expect(rows(host)).toEqual([
      'Check the access Helper holds: viewer on project logoOpen',
      'Sam asks to edit project logoDecide',
      'Helper prepared something for you to approve: Post the logoDecide',
      'Sam asks to write notes 1 through notesDecide',
    ]);
    expect(links(host)).toEqual(['#/reviews', '#/requests/req%2F1', '#/access/drafts', '#/access/drafts']);
    expect(host.querySelector('[data-waiting="count"]')?.textContent).toBe('4');
    expect(waitingCount(ANSWER)).toBe(4);
  });

  it('says once that nothing is waiting, and shows no number', () => {
    const host = page([]);
    expect(rows(host)).toEqual(['Nothing is waiting on you.']);
    expect(host.querySelector('[data-waiting="count"]')).toBeNull();
    expect(waitingCount([])).toBe(0);
  });

  it('shows a part that could not be read as that part’s own line, keeps the rest, and shows no number', () => {
    const host = page([refusedPart('review'), ANSWER[1]]);
    expect(rows(host)).toEqual(['Whether the access you gave is due a check could not be read', 'Sam asks to edit project logoDecide']);
    // The unread part keeps its Open to the page it stands for.
    expect(links(host)).toEqual(['#/reviews', '#/requests/req%2F1']);
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
