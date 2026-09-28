/** Real routes expose session controls and authoritative review data without fictional decisions. */
import { describe, expect, it } from 'vitest';
import { $, click, mount, text, unreachable } from './harness';
import { ADA, ME, SERVICE, ok, refused } from './fixtures';

const current = { id: 'session-current', current: true, login: { issuer: 'https://issuer.test', subject: 'account-1' }, started_at: 1790000000, ends_at: 1790003600 };
const other = { ...current, id: 'session-other', current: false };
const sessions = { person: ADA, sessions: [current, other], cookie: 'never-render-this-cookie' };
const button = (label: string) => [...document.querySelectorAll('button')].find((value) => value.textContent === label) ?? null;

describe('Sessions', () => {
  it('reads sessions and requires confirmation before ending exactly the selected one', async () => {
    let ended = false;
    const { posted } = await mount('#/sessions', { ...SERVICE, '/sessions': () => ok(ended ? { ...sessions, sessions: [current] } : sessions),
      'POST /sessions/session-other/end': () => { ended = true; return ok({ ended: other.id }); },
    });
    expect(text()).toContain('This session');
    expect(text()).not.toContain('never-render-this-cookie');
    expect(unreachable()).toEqual([]);
    await click(button('End session'));
    expect(posted).toHaveLength(0);
    expect(text()).toContain('Your current session stays open');
    await click(button('Confirm end session'));
    expect(posted).toEqual([{ path: '/sessions/session-other/end', body: {} }]);
    expect(text()).not.toContain('Another signed-in session');
  });

  it('does not repeat an uncertain end until an authoritative refresh completes', async () => {
    const { posted } = await mount('#/sessions', { ...SERVICE, '/sessions': ok(sessions),
      'POST /sessions/session-other/end': refused(503, 'Unavailable', 'Outcome not confirmed'),
    });
    await click(button('End session'));
    await click(button('Confirm end session'));
    await click(button('Confirm end session'));
    expect(posted).toHaveLength(1);
    expect(text()).toContain('could not be confirmed');
    await click(button('Refresh sessions'));
    expect(button('End session')?.hasAttribute('disabled')).toBe(false);
    expect(posted).toHaveLength(1);
  });

  it('returns to sign-in after the current session is ended', async () => {
    await mount('#/sessions', { ...SERVICE, '/sessions': ok(sessions), 'POST /sessions/session-current/end': ok({ ended: current.id }) });
    await click(button('Sign out'));
    expect(text()).toContain('You will need to sign in again');
    await click(button('Confirm end session'));
    expect($('a[href="/api/login"]')).not.toBeNull();
  });

  it('uses the administrator route for an explicitly selected person', async () => {
    const { requests } = await mount('#/sessions?person=' + ADA, { ...SERVICE, ['/directory/people/' + ADA + '/sessions']: ok(sessions) });
    expect(requests).toContain('/directory/people/' + ADA + '/sessions');
    expect(requests).not.toContain('/sessions');
  });
});

describe('Reviews', () => {
  it('shows named agents, access, owners and a working access link without inventing Keep', async () => {
    const { posted, requests } = await mount('#/reviews', { ...SERVICE, '/reviews': ok({
      scope: 'personal', revision: 4, judged_at: 1790000000,
      due: [{ agent: { id: 'agent-example', display_name: 'Builder', state: 'active' }, reviewer: ME.person,
        grant: { id: 'grant-1', relation: 'editor', resource: { kind: 'project', id: 'Lys' }, actions: ['edit'] } }],
      unanswered: [{ agent: { id: 'agent-orphan', display_name: 'Unowned builder', state: 'active' }, person: { ...ME.person, state: 'suspended' } }],
    }) });
    expect(requests).toContain('/reviews');
    expect(posted).toHaveLength(0);
    expect(text()).toContain('Builder');
    expect(text()).toContain('editor on project Lys');
    expect(text()).toContain('These agents need an active owner');
    expect($('a[href="#/file/agent-example/access"]')).not.toBeNull();
    expect(button('Keep')).toBeNull();
    expect(text()).not.toContain('not built yet');
  });
});
