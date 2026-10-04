/** Machine controls use server records and keep an uncertain registration under its original operation. */
import { act } from 'react';
import { beforeEach, describe, expect, it } from 'vitest';
import { $, $$, click, mount, settle, text, unmountAll } from './harness';
import { ADA, ME, OWN, REVIEWER, SCRIBE, SERVICE, ok, refused } from './fixtures';
import type { Route } from './fixtures';
import type { Machine, NameMachine } from '../src/features/network/contract';

beforeEach(() => sessionStorage.clear());
const machine: Machine = { id: 'op-' + 'a'.repeat(32), name: 'Workshop laptop', kind: 'laptop', runtime: null, slots: 0, may_run: [], may_reach: [], named_by: ADA, named_at: 1790000000, state: 'in_use', retired_at: null, last_report_at: null };
const routes = { ...SERVICE, '/network': ok({ machines: [machine], reports_served: false }), ['/network/machines/' + machine.id + '/runner']: ok({ machine: machine.id, runner: null, answers: null }) };
const button = (label: string) => [...document.querySelectorAll('button')].find((value) => value.textContent === label) ?? null;
function input(name: string, value: string) {
  const element = $('[name="' + name + '"]');
  if (!(element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement || element instanceof HTMLSelectElement)) throw new Error('Missing ' + name);
  element.value = value;
}
async function submit() {
  const form = $('form[aria-label="Add a computer"]');
  if (!form) throw new Error('Missing registration form');
  await act(async () => { form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); });
  await settle();
}
const recorded = (body: NameMachine): Machine => ({ ...machine, ...body, id: body.operation, may_run: body.may_run.map((id) => ({ id, display_name: 'Scribe', state: 'active' })) });

async function adding(extra: Record<string, Route> = {}) {
  for (const [path, route] of Object.entries(routes)) if (!(path in extra)) extra[path] = route;
  const mounted = await mount('#/network', extra);
  await click(button('+ Add a computer'));
  return mounted;
}
function naming(runner: (id: string) => Route = (id) => ok({ machine: id, runner: { kind: 'lys' } })): Record<string, Route> {
  const answers: Record<string, Route> = {};
  answers['POST /network/machines'] = (body) => {
    const draft = body as NameMachine;
    answers['POST /network/machines/' + draft.operation + '/runner'] = runner(draft.operation);
    answers['/network/machines/' + draft.operation + '/runner'] = ok({ machine: draft.operation, runner: { kind: 'lys' }, answers: true });
    return ok(recorded(draft));
  };
  return answers;
}

describe('Network', () => {
  it('lists each computer with its status and who may start there, and opens it beside the list', async () => {
    await mount('#/network', routes);
    expect($('#screen .page.fill .pane table')).not.toBeNull();
    const row = $$('#screen tbody tr[data-href]')[0];
    expect(row.textContent).toContain('Workshop laptop'); expect(row.textContent).toContain('Does not run agents'); expect(row.textContent).toContain('Lys does not start agents here.');
    expect($('section[aria-label="Workshop laptop"] h2')?.textContent).toBe('Workshop laptop');
    expect(button('Refresh network')).toBeNull();
  });
  it('is titled Network and counts one computer in the singular, once', async () => {
    await mount('#/network', routes);
    expect($('#screen .page.fill .head h1')?.textContent).toBe('Network');
    expect($('#screen .page.fill .head .sub')).toBeNull();
    expect($('.tools .count')?.textContent).toBe('1 computer');
    expect($('#screen input[type="search"]')?.getAttribute('placeholder')).toBe('Search 1 computer');
    expect($$('#screen tbody tr[data-href]')[0].textContent).not.toMatch(/\b1 (computers|agents)\b/);
    expect($$('.stat .l').map((l) => l.textContent)).toEqual(['up now', 'not answering', 'agents running']);
    expect(text()).not.toMatch(/\b1 computers\b/);
  });
  describe('says how each computer stands, by its runner\'s answer and never by a report\'s age', () => {
    const lys = { kind: 'lys' };
    const old = Math.floor(Date.now() / 1000) - 16 * 60;
    const at = (given: Partial<Machine>, runner: object) => ({ ...routes, '/network': ok({ machines: [{ ...machine, runtime: 'lys-runner', ...given }], reports_served: true }),
      ['/network/machines/' + machine.id + '/runner']: ok({ machine: machine.id, ...runner }) });
    const shown = () => $$('#screen tbody tr[data-href]')[0]?.children[1]?.textContent;
    it('is up when its runner answered, however long ago it last reported', async () => {
      await mount('#/network', at({ last_report_at: old }, { runner: lys, answers: true }));
      expect(shown()).toBe('Up');
      expect($$('.stat .n').map((n) => n.textContent)).toEqual(['1', '0', '0']);
      expect(text()).not.toContain('Last heard');
    });
    it('is not answering when its runner did not answer, with when it was last heard and the service\'s reason beside it', async () => {
      await mount('#/network', at({ last_report_at: old }, { runner: lys, answers: false, refusal: 'runner_unreachable', reason: '/run/lys/runner.sock: connection refused' }));
      expect(shown()).toBe('Runner not answering, last heard 16 min ago');
      expect($$('.stat .n').map((n) => n.textContent)).toEqual(['0', '1', '0']);
      expect($('section[aria-label="Workshop laptop"]')?.textContent).toContain('Its runner did not answer: /run/lys/runner.sock: connection refused');
    });
    it('is not answering without inventing a last report when there is none', async () => {
      await mount('#/network', at({ last_report_at: null }, { runner: lys, answers: false, refusal: 'runner_not_dialled_in', reason: 'no bridge of this machine is dialled in to this server' }));
      expect(shown()).toBe('Runner not answering');
    });
    it('names a computer with no runner, and one that does not run agents, and one retired', async () => {
      await mount('#/network', at({}, { runner: null, answers: null }));
      expect(shown()).toBe('No runner connected');
      unmountAll(); document.body.innerHTML = '';
      await mount('#/network', at({ runtime: null }, { runner: null, answers: null }));
      expect(shown()).toBe('Does not run agents');
      unmountAll(); document.body.innerHTML = '';
      await mount('#/network', at({ state: 'retired', retired_at: 1 }, { runner: lys, answers: true }));
      await click(button('Show retired (1)'));
      expect(shown()).toBe('Retired');
    });
    it('counts the agents running on it, in the singular for one', async () => {
      const session = { session: 'op-' + 'e'.repeat(32), agent: SCRIBE, machine: machine.id, machine_name: 'Workshop laptop', runtime: 'sh', shown: 'running', last_reported: 'running', first_report_at: 1, last_report_at: 1, what: '', stopped: null, reported_by: 'runner' };
      await mount('#/network', { ...at({}, { runner: lys, answers: true }), '/runtime/live': ok({ sessions: [session], unanswered: [] }) });
      expect(shown()).toBe('Up, 1 agent running');
    });
    it('draws every computer at once, and a runner that never answers holds only its own row', async () => {
      // The silent computer has an address of its own: a read still waiting is shared, and this one never ends.
      const silent: Machine = { ...machine, id: 'op-' + 'c'.repeat(32), runtime: 'lys-runner' };
      const other: Machine = { ...machine, id: 'op-' + 'b'.repeat(32), name: 'Ward desk', runtime: 'lys-runner' };
      await mount('#/network', { ...routes, '/network': ok({ machines: [silent, other], reports_served: true }),
        ['/network/machines/' + silent.id + '/runner']: (() => new Promise(() => undefined)) as unknown as Route,
        ['/network/machines/' + other.id + '/runner']: ok({ machine: other.id, runner: lys, answers: true }) });
      const rows = $$('#screen tbody tr[data-href]');
      expect(rows.map((row) => row.children[0]?.textContent)).toEqual(['Workshop laptop', 'Ward desk']);
      expect(rows.map((row) => row.children[1]?.textContent)).toEqual(['Asking its runner…', 'Up']);
      expect($$('.stat .n').map((n) => n.textContent)).toEqual(['1', '0', '0']);
      expect($('section[aria-label="Workshop laptop"]')?.textContent).toContain('Asking its runner…');
      expect($('section[aria-label="Workshop laptop"]')?.textContent).not.toContain('No runner is connected');
    });
    it('ends an ask still waiting on its runner when the person leaves the page', async () => {
      const silent: Machine = { ...machine, id: 'op-' + 'd'.repeat(32), runtime: 'lys-runner' };
      await mount('#/me', { ...routes, '/network': ok({ machines: [silent], reports_served: true }),
        ['/network/machines/' + silent.id + '/runner']: (() => new Promise(() => undefined)) as unknown as Route });
      // The service's stand-in is watched for the signal each runner ask carries.
      const served = globalThis.fetch;
      const asks: AbortSignal[] = [];
      globalThis.fetch = ((input: string, init?: RequestInit) => {
        if (String(input).endsWith('/runner') && init?.signal) asks.push(init.signal);
        return served(input, init);
      }) as typeof fetch;
      try {
        await act(async () => { location.hash = '#/network'; });
        await settle();
        expect($$('#screen tbody tr[data-href]').map((row) => row.children[1]?.textContent)).toEqual(['Asking its runner…']);
        expect(asks).toHaveLength(1);
        expect(asks[0].aborted).toBe(false);
        await act(async () => { location.hash = '#/me'; });
        await settle();
        expect(asks[0].aborted).toBe(true);
      } finally { globalThis.fetch = served; }
    });
    it('refuses an answer that does not say whether the runner answers', async () => {
      await mount('#/network', at({}, { runner: lys }));
      await settle();
      expect(shown()).toBe('Lys could not ask its runner');
      expect($('section[aria-label="Workshop laptop"]')?.textContent).toContain('did not say whether the computer');
      expect($$('.stat .n').map((n) => n.textContent)).toEqual(['0', '0', '0']);
    });
  });

  it('adds a computer in the list\'s own last row, under its columns, with no side form', async () => {
    await mount('#/network', routes);
    const row = $('#screen .listing table > tfoot:last-child > tr:last-child');
    const heads = $$('#screen .listing thead th').map((th) => th.textContent);
    expect(heads).toEqual(['Computer', 'Status', 'Running now', 'May start here']);
    const cells = [...(row?.children ?? [])];
    expect(cells).toHaveLength(4);
    const form = cells[0].querySelector('form[aria-label="Add a computer"]');
    expect(form?.querySelector('input[name="name"]')).not.toBeNull();
    expect(cells[1].textContent).toBe('');
    expect(cells[2].textContent).toBe('');
    expect(cells[3].querySelector('button[type="submit"]')?.getAttribute('form')).toBe(form?.id);
    expect($$('form[aria-label="Add a computer"]')).toHaveLength(1);
    expect($('.recorded-form')).toBeNull();
    await click(button('+ Add a computer'));
    expect(document.activeElement).toBe(form?.querySelector('input[name="name"]'));
  });
  it('puts the keyboard in the add row when the address asks to add a computer', async () => {
    await mount('#/network?add=computer', routes);
    expect(document.activeElement?.getAttribute('name')).toBe('name');
    expect(document.activeElement?.closest('tfoot')).not.toBeNull();
  });
  it('offers no add row to a person who may not add a computer', async () => {
    await mount('#/network', { ...routes, '/directory/people': refused(403, 'NotAdmitted', 'not administrator'), '/people': ok(OWN) });
    expect($('form[aria-label="Add a computer"]')).toBeNull();
    expect($('#screen .listing tfoot')).toBeNull();
  });
  it('says an empty network is empty once, in the list', async () => {
    await mount('#/network', { ...routes, '/network': ok({ machines: [], reports_served: true }) });
    expect(text().split('No computer yet.')).toHaveLength(2);
    expect($('#screen tbody tr.empty td')?.textContent).toContain('No computer yet.');
    expect(text()).not.toContain('Nothing here yet');
  });
  it('adds this computer with a name and no website permissions', async () => {
    const { posted } = await adding(naming());
    expect($('input[name="kind"]')).toBeNull(); expect($('input[type="radio"]')).toBeNull();
    expect($('[name="hosts"]')).toBeNull();
    input('name', 'Lab'); await submit();
    expect(posted[0]).toEqual({ path: '/network/machines', body: { operation: expect.stringMatching(/^op-[0-9a-f]{32}$/), name: 'Lab', kind: 'Computer', runtime: 'lys-runner', slots: 0, may_run: [], may_run_roles: [], may_reach: [] } });
  });
  it('adds this computer with its own runner and grants no agent or role from the global page', async () => {
    const { posted } = await adding(naming());
    input('name', 'Lab'); await submit();
    expect($('input[aria-label="Add an agent"]')).toBeNull();
    expect(posted[0].body).toMatchObject({ runtime: 'lys-runner', slots: 0, may_run: [], may_run_roles: [] });
    const id = (posted[0].body as NameMachine).operation;
    expect(posted[1]).toEqual({ path: '/network/machines/' + id + '/runner', body: { runner: { kind: 'lys' } } });
  });
  it('keeps the runner phase unresolved when its answer is refused', async () => {
    const { posted } = await adding(naming(() => refused(503, 'RunnerUnavailable', 'The runner was not confirmed')));
    input('name', 'Lab'); await submit();
    expect(posted).toHaveLength(2); expect(text()).toContain('RunnerUnavailable');
    expect(text()).not.toContain('Lab was added.'); expect(sessionStorage.length).toBe(1);
    expect(button('Check whether it was added')).not.toBeNull();
  });
  it('keeps an unconfirmed addition across remount and sends only its original request', async () => {
    const first = await adding({ 'POST /network/machines': refused(503, 'NetworkUnavailable', 'write outcome unknown') });
    input('name', 'Lab'); await submit();
    const original = first.posted[0].body;
    unmountAll(); document.body.innerHTML = '';
    const second = await adding(naming());
    await click(button('Check whether it was added'));
    expect(second.posted[0]).toEqual({ path: '/network/machines', body: original });
    expect(text()).toContain('Lab was added.');
  });
  it('asks before retiring and says running agents keep running', async () => {
    const { posted } = await mount('#/network', { ...routes, ['POST /network/machines/' + machine.id + '/retire']: ok({ ...machine, state: 'retired' }) });
    expect(button('Retire this computer')?.closest('.section-h')).toBeNull();
    await click(button('Retire this computer')); expect(posted).toEqual([]);
    expect(text()).toContain('Agents already running on it keep running');
    await click(button('Confirm retirement'));
    expect(posted).toEqual([{ path: '/network/machines/' + machine.id + '/retire', body: {} }]);
  });
  it('does not offer changes to non-administrators or when the network is unavailable', async () => {
    await mount('#/network', { ...routes, '/directory/people': refused(403, 'NotAdmitted', 'not administrator'), '/people': ok(OWN) });
    expect(button('+ Add a computer')).toBeNull(); expect(button('Retire this computer')).toBeNull();
    unmountAll(); document.body.innerHTML = '';
    await mount('#/network', { ...routes, '/network': refused(503, 'NetworkUnavailable', 'not configured') });
    expect(text()).toContain('NetworkUnavailable'); expect(button('+ Add a computer')).toBeNull();
  });
  it('reads one saved format: an earlier saved addition is said to be unreadable and nothing is sent', async () => {
    const earlier: NameMachine = { operation: 'op-' + 'd'.repeat(32), name: 'Lab', kind: 'Computer', runtime: 'lys-runner', slots: 0, may_run: [], may_run_roles: [], may_reach: [] };
    sessionStorage.setItem('lys.pending.machine.' + ADA, JSON.stringify(earlier));
    const { posted } = await adding({});
    expect(text()).toContain('The retained computer addition cannot be read');
    expect(posted).toEqual([]); expect(sessionStorage.length).toBe(1);
  });

});

describe('Computers at the size of a business', () => {
  const team = (n: number, members: string[]) => ({ id: 'op-' + String(n).padStart(32, '0'), name: 'Team ' + n, owner: ADA, parent: null, lead: null, members, held: [], description: '', state: 'active', created_by: ME.signed_in, created_at: 1, retired_at: null });
  const now = Math.floor(Date.now() / 1000);
  const fleet: Machine[] = Array.from({ length: 60 }, (_, n) => ({ ...machine, id: 'op-' + String(n).padStart(32, 'a'), name: 'Builder ' + String(n).padStart(2, '0'), runtime: 'lys-runner', may_run: [{ id: n % 2 ? SCRIBE : REVIEWER, display_name: n % 2 ? 'Scribe' : 'Reviewer', state: 'active' }], last_report_at: n < 55 ? now : now - 86400 }));
  const big = { ...routes, '/teams': ok({ teams: [team(1, [SCRIBE]), team(2, [REVIEWER])] }), '/network': ok({ machines: fleet, reports_served: true }),
    ...Object.fromEntries(fleet.map((each, n) => ['/network/machines/' + each.id + '/runner', ok({ machine: each.id, runner: { kind: 'dialled', key: 'a'.repeat(64) },
      ...(n < 55 ? { answers: true } : { answers: false, refusal: 'runner_not_dialled_in', reason: 'no bridge of this machine is dialled in to this server' }) })])) };
  it('groups computers by the teams whose agents start there, and counts what is up', async () => {
    await mount('#/network', big);
    expect($$('#screen tr.group .group-name').map((name) => name.textContent)).toEqual(['Team 1', 'Team 2']);
    expect($$('.stat .n').map((n) => n.textContent)).toEqual(['55', '5', '0']);
    expect($('.tools .count')?.textContent).toBe('60 computers in 2 groups');
    expect(text().split('60 computers')).toHaveLength(2);
  });
  it('shows only the computers whose runner is not answering, in one click', async () => {
    await mount('#/network', big);
    await click(button('Not answering'));
    expect($$('#screen tbody tr[data-href]').map((row) => row.querySelector('td')?.textContent)).toEqual(['Builder 55', 'Builder 57', 'Builder 59', 'Builder 56', 'Builder 58']);
  });
});
