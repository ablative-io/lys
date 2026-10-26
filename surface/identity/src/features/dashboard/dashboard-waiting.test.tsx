/**
 * The Dashboard's "Waiting on you" and the rail's count: both read `GET /waiting`, one list of what the person may
 * decide, oldest first, with a part that could not be read standing as its own line. The number is the list's own.
 * A request and a draft are decided in their own row from the record their entry carries (Tom, 18 October 2026);
 * an app action is approved or refused in place and an agent's shown checks are kept together.
 */
import { renderToStaticMarkup } from 'react-dom/server';
import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { describe, expect, it, vi } from 'vitest';
import { Refused } from '../../api';
import type { Draft } from '../drafts/contract';
import type { AccessRequest } from '../requests/contract';
import { WaitingOnYou } from '../dashboard/WaitingOnYou';
import { railWaiting, waitingCount } from '../dashboard/waiting';
import type { WaitingEntry, WaitingLine, WaitingPartRefused } from '../dashboard/waiting';
import type { ProductDraft } from '../drafts/contract';

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
const product: ProductDraft = {
  id: 'pd-1', app: 'notes', grant: 'grant-held', target: { kind: 'notes.doc', id: '1', action: 'write' },
  request_digest: 'a'.repeat(64), words: '{"title":"For approval"}', state: 'waiting', mode: 'by_draft',
};
const checks: NonNullable<WaitingEntry['review']> = {
  agent: { id: 'agent-helper', display_name: 'Helper', state: 'active' }, count: 3,
  grants: [1, 2, 3].map((id) => ({ id: 'grant-' + id, relation: 'viewer', resource: 'project logo ' + id, last_check: 1_760_000_000 })),
};
const page = (waiting: WaitingLine[] | Refused) => {
  const host = document.createElement('div');
  host.innerHTML = renderToStaticMarkup(<WaitingOnYou waiting={waiting} me={ME} />);
  return host;
};
/** Each row as its words and its act: the link's word, or the decision acts by name. */
const rows = (host: Element) => [...host.querySelectorAll('tbody tr')].map((tr) => tr.getAttribute('data-waiting-entry') === 'product_draft'
  ? (tr.querySelector('td')?.firstChild?.textContent ?? '') + [...tr.querySelectorAll('td.dash-decide button')].map((act) => act.getAttribute('aria-label')).join(' / ')
  : tr.querySelector('summary')?.textContent
  ?? (tr.querySelector('td')?.textContent ?? '') + (tr.querySelector('a.btn')?.textContent ?? [...tr.querySelectorAll('td.dash-decide button')].map((act) => act.getAttribute('aria-label')).join(' / ')));
const links = (host: Element) => [...host.querySelectorAll('tbody tr a.btn')].map((a) => a.getAttribute('href'));

/** As the service answers it: oldest first, the request and the draft each with its record. */
const ANSWER: WaitingLine[] = [
  entry('review', 'agent-helper', 'Check the access Helper holds: 3 grants', '2026-10-18T01:00:00Z', { review: checks }),
  entry('request', 'req/1', 'Sam asks to edit project logo', '2026-10-18T02:00:00Z', { request: asked }),
  entry('agent_draft', 'draft-1', 'Helper prepared something for you to approve: Post the logo', '2026-10-18T03:00:00Z', { draft: prepared }),
  entry('product_draft', 'pd-1', 'Sam asks to write notes 1 through notes', '2026-10-18T04:00:00Z', { product_draft: product }),
];

describe('Waiting on you on the Dashboard', () => {
  it('lists the route’s answer in its own order, decides every complete row, and its number is the list’s length', () => {
    const host = page(ANSWER);
    expect(rows(host)).toEqual([
      'Check the access Helper holds: 3 grants',
      'Sam asks to edit project logoto fix the coloursApprove access / Decline request',
      'Helper prepared something for you to approve: Post the logoApprove / Decline',
      'Sam asks to write notes 1 through notesApprove / Decline',
    ]);
    // A check also opens the agent's access, where individual grants can be withdrawn.
    expect(links(host)).toEqual(['#/file/agent-helper/access']);
    expect(host.querySelectorAll('td.dash-decide')).toHaveLength(4);
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

describe('DIRECTORY-123 decisions where they stand', () => {
  async function drawn(line: WaitingEntry, decide: (host: HTMLElement) => Promise<void>) {
    const host = document.createElement('div');
    document.body.append(host);
    const root = createRoot(host);
    const posted: { path: string; body: Record<string, unknown> }[] = [];
    const changed = vi.fn();
    vi.stubGlobal('fetch', async (path: string, init: RequestInit) => {
      const body = JSON.parse(String(init.body)) as Record<string, unknown>;
      posted.push({ path, body });
      const answer = line.review
        ? { agent: line.review.agent.id, operation: body.operation, kept: line.review.grants.map((grant) => ({ grant: grant.id, kept_by: ME, operation: body.operation, note: '', at: 1_800_000_000, revision: 1 })) }
        : { draft: line.id, operation: body.operation };
      return new Response(JSON.stringify(answer), { headers: { 'content-type': 'application/json' } });
    });
    try {
      await act(async () => root.render(<WaitingOnYou waiting={[line]} me={ME} changed={changed} />));
      await decide(host);
      expect(changed).toHaveBeenCalledOnce();
      expect(host.querySelector('[data-waiting-entry]')).toBeNull();
      return posted;
    } finally {
      await act(async () => root.unmount());
      host.remove();
    }
  }

  it('shows the agent and count and keeps exactly the ids shown, removing the row', async () => {
    const posted = await drawn(ANSWER[0] as WaitingEntry, async (host) => {
      expect(host.textContent).toContain('Check the access Helper holds: 3 grants');
      await act(async () => host.querySelector<HTMLElement>('[aria-label="Keep all"]')?.click());
    });
    expect(posted).toHaveLength(1);
    expect(posted[0].path).toBe('/api/reviews/agents/agent-helper/keep');
    expect(posted[0].body.grants).toEqual(['grant-1', 'grant-2', 'grant-3']);
  });

  it('approves an app action in its row', async () => {
    const posted = await drawn(ANSWER[3] as WaitingEntry, async (host) => {
      await act(async () => host.querySelector<HTMLElement>('[data-act="product-approve"]')?.click());
    });
    expect(posted[0].path).toBe('/api/product-drafts/pd-1/approve');
    expect(posted[0].body.request_digest).toBe(product.request_digest);
  });

  it('refuses an app action in its row with the reason', async () => {
    const posted = await drawn(ANSWER[3] as WaitingEntry, async (host) => {
      await act(async () => host.querySelector<HTMLElement>('[data-act="product-refuse"]')?.click());
      const input = host.querySelector('input');
      await act(async () => {
        Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')?.set?.call(input, 'No longer needed');
        input?.dispatchEvent(new Event('input', { bubbles: true }));
      });
      await act(async () => host.querySelector('form')?.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })));
    });
    expect(posted[0].path).toBe('/api/product-drafts/pd-1/refuse');
    expect(posted[0].body.reason).toBe('No longer needed');
  });

  it('gives every complete row a decision act and lists a check’s grants on request', () => {
    const host = page(ANSWER);
    for (const row of host.querySelectorAll('[data-waiting-entry]')) expect(row.querySelector('button.act')).not.toBeNull();
    expect([...host.querySelectorAll('[data-waiting-entry="review"] details li')].map((grant) => grant.textContent)).toEqual(checks.grants.map((grant) => expect.stringContaining(grant.resource)));
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
